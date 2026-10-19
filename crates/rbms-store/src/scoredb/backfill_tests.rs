use super::*;
use crate::{SCORE_LN_MODE_FROM_CHART, SCORE_RULE_VERSION};

const BODY: &str = "{\"api_version\":1}";
const TRACE: &str = "{\"format\":\"rbms-trace\"}";
const LATER_TRACE: &str = "{\"format\":\"rbms-trace\",\"later\":true}";
const BATCH: usize = 8;

fn fresh() -> ScoreDb {
    let mut db = ScoreDb::open_in_memory().expect("an in-memory database opens");
    db.migrate().expect("the schema applies");
    db
}

fn play(date: i64, replay_file: Option<&str>) -> PlayLog {
    PlayLog {
        chart_key: "aa".into(),
        mode: mode_id("BEAT_7K"),
        ln_mode: SCORE_LN_MODE_FROM_CHART.into(),
        md5: "aa".into(),
        judge: JudgeCounts::from_early_late([10, 0, 0, 0, 0, 0], [0; JUDGE_TIER_COUNT]),
        notes: 10,
        date,
        rule_version: SCORE_RULE_VERSION,
        replay_file: replay_file.map(str::to_owned),
        ..PlayLog::default()
    }
}

fn ids(plays: &[UntracedPlay]) -> Vec<i64> {
    plays.iter().map(|play| play.log.id).collect()
}

#[test]
fn the_untraced_plays_are_those_with_a_replay_and_no_trace_inside_the_bounds_oldest_first() {
    let mut db = fresh();
    let with_body = db.record_play_kept(&play(1, Some("a.ron")), true, Some(KeptSubmission { body: BODY, played_as: None })).expect("stored");
    let no_replay = db.record_play(&play(2, None), true).expect("stored");
    let traced = db.record_play(&play(3, Some("c.ron")), true).expect("stored");
    db.keep_trace(traced, TRACE).expect("the trace is kept");
    let without_body = db.record_play(&play(4, Some("d.ron")), true).expect("stored");
    let past_the_bound = db.record_play(&play(5, Some("e.ron")), true).expect("stored");

    let plays = db.untraced_plays(TRACE_BACKFILL_NOT_STARTED, without_body, BATCH).expect("read");
    assert_eq!(ids(&plays), vec![with_body, without_body], "no replay, a trace, and past the bound are left out ({no_replay}, {traced}, {past_the_bound})");
    assert_eq!(plays[0].body.as_deref(), Some(BODY));
    assert_eq!(plays[1].body, None, "a play stored before bodies were kept has none");
    assert_eq!(plays[0].log.replay_file.as_deref(), Some("a.ron"));
    assert_eq!(ids(&db.untraced_plays(with_body, without_body, BATCH).expect("read")), vec![without_body], "after a play is after it");
    assert_eq!(ids(&db.untraced_plays(TRACE_BACKFILL_NOT_STARTED, without_body, 1).expect("read")), vec![with_body], "the limit holds");
}

#[test]
fn kept_traces_and_the_mark_go_in_together_and_a_trace_a_play_has_stands() {
    let mut db = fresh();
    let first = db.record_play(&play(1, Some("a.ron")), true).expect("stored");
    let second = db.record_play(&play(2, Some("b.ron")), true).expect("stored");
    db.keep_trace(second, LATER_TRACE).expect("the trace is kept");
    assert_eq!(db.trace_backfill_through().expect("read"), TRACE_BACKFILL_NOT_STARTED);

    let batch = BackfillBatch {
        traces: vec![(first, TRACE.to_owned()), (second, TRACE.to_owned())],
        from: TRACE_BACKFILL_NOT_STARTED,
        through: second,
        ..BackfillBatch::default()
    };
    let written = db.keep_backfilled_traces(&batch).expect("written");
    assert_eq!(written, BackfillWritten { kept: 1, marked: true }, "the play that has a trace keeps it");
    assert_eq!(db.trace_body(first).expect("read").as_deref(), Some(TRACE));
    assert_eq!(db.trace_body(second).expect("read").as_deref(), Some(LATER_TRACE));
    assert_eq!(db.trace_backfill_through().expect("read"), second);
    assert!(db.untraced_plays(TRACE_BACKFILL_NOT_STARTED, second, BATCH).expect("read").is_empty());
}

/// A play found never to get a trace is not handed out again, past the mark or behind it, and is
/// forgotten once the mark has passed it.
#[test]
fn a_refused_play_is_not_looked_at_again() {
    let mut db = fresh();
    let not_now = db.record_play(&play(1, Some("a.ron")), true).expect("stored");
    let refused = db.record_play(&play(2, Some("b.ron")), true).expect("stored");
    let batch = BackfillBatch { refused: vec![refused], from: TRACE_BACKFILL_NOT_STARTED, through: not_now - 1, ..BackfillBatch::default() };
    assert!(db.keep_backfilled_traces(&batch).expect("written").marked);
    assert_eq!(
        ids(&db.untraced_plays(db.trace_backfill_through().expect("read"), refused, BATCH).expect("read")),
        vec![not_now],
        "the refused play is left out"
    );
    let passed = BackfillBatch { from: not_now - 1, through: refused, ..BackfillBatch::default() };
    assert!(db.keep_backfilled_traces(&passed).expect("written").marked);
    let left: i64 = db.conn.query_row("SELECT COUNT(*) FROM trace_backfill_refused", [], |row| row.get(0)).expect("counted");
    assert_eq!(left, 0, "a refusal behind the mark is not kept");
}

/// A batch made on a mark the database does not hold, because the batch before it was not written,
/// keeps what it made and leaves the mark where it stands.
#[test]
fn a_batch_made_on_another_mark_keeps_its_traces_and_moves_no_mark() {
    let mut db = fresh();
    let first = db.record_play(&play(1, Some("a.ron")), true).expect("stored");
    let second = db.record_play(&play(2, Some("b.ron")), true).expect("stored");
    let after_an_unwritten_batch = BackfillBatch { traces: vec![(second, TRACE.to_owned())], from: first, through: second, ..BackfillBatch::default() };
    assert_eq!(db.keep_backfilled_traces(&after_an_unwritten_batch).expect("written"), BackfillWritten { kept: 1, marked: false });
    assert_eq!(db.trace_backfill_through().expect("read"), TRACE_BACKFILL_NOT_STARTED);
    assert_eq!(
        ids(&db.untraced_plays(TRACE_BACKFILL_NOT_STARTED, second, BATCH).expect("read")),
        vec![first],
        "the play of the batch not written is looked at again"
    );
}

/// What stood last in the history at a second, newest first.
#[test]
fn the_plays_ended_by_a_second_are_the_newest_stored_by_then() {
    let mut db = fresh();
    let early = db.record_play(&play(10, Some("a.ron")), true).expect("stored");
    let at = db.record_play(&play(20, Some("b.ron")), true).expect("stored");
    let _later = db.record_play(&play(30, Some("c.ron")), true).expect("stored");
    let logs = db.plays_ended_by(20, BATCH).expect("read");
    assert_eq!(logs.iter().map(|log| log.id).collect::<Vec<_>>(), vec![at, early]);
    assert_eq!(db.plays_ended_by(20, 1).expect("read").len(), 1, "the limit holds");
    db.put_course("course", 20_500, BODY, None).expect("kept");
    let courses = db.kept_courses().expect("read");
    assert_eq!(courses.iter().map(|course| (course.played_at_ms, course.body.as_str())).collect::<Vec<_>>(), vec![(20_500, BODY)]);
}

#[test]
fn the_mark_is_kept_across_openings() {
    let dir = std::env::temp_dir().join(format!("rbms_scoredb_backfill_mark_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("scoredb.sqlite");
    let looked_at = {
        let mut db = ScoreDb::open(&path).expect("opened");
        db.migrate().expect("migrated");
        let id = db.record_play(&play(1, Some("a.ron")), true).expect("stored");
        db.keep_backfilled_traces(&BackfillBatch { through: id, ..BackfillBatch::default() }).expect("written");
        id
    };
    let db = ScoreDb::open(&path).expect("reopened");
    assert_eq!(db.trace_backfill_through().expect("read"), looked_at);
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

use super::schema::SCHEMA_SQL;
use super::sync::ReplayAnswer;
use super::*;
use crate::{SCORE_LN_MODE_FROM_CHART, SCORE_RULE_VERSION, SCRATCH_AUTO_ASSIST, SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION};

/// The score server the fixtures sync to, and a second one.
const SERVER: &str = "https://bms.example";
const OTHER_SERVER: &str = "https://other.example";

/// The account the fixtures sync under, and a second one.
const ACCOUNT: &str = "dj";
const OTHER_ACCOUNT: &str = "someone-else";

/// The first statement the sync tables are created by: everything in the DDL before it is the
/// schema of a database the first generation of this build wrote, before the tables existed.
const FIRST_SYNC_TABLE: &str = "CREATE TABLE IF NOT EXISTS play_submission";

/// An epoch millisecond to stamp a send with.
const NOW_MS: i64 = 1_700_000_000_000;

fn fresh() -> ScoreDb {
    let mut db = ScoreDb::open_in_memory().expect("an in-memory database opens");
    db.migrate().expect("the schema applies");
    db
}

fn play(md5: &str, date: i64) -> PlayLog {
    PlayLog {
        chart_key: md5.into(),
        mode: mode_id("BEAT_7K"),
        ln_mode: SCORE_LN_MODE_FROM_CHART.into(),
        md5: md5.into(),
        judge: JudgeCounts::from_early_late([10, 0, 0, 0, 0, 0], [0; JUDGE_TIER_COUNT]),
        notes: 10,
        date,
        rule_version: SCORE_RULE_VERSION,
        ..PlayLog::default()
    }
}

fn stored(db: &mut ScoreDb, log: &PlayLog) -> i64 {
    db.record_play(log, true).expect("the play is stored")
}

/// A walk of every play to `server` under [`ACCOUNT`] that sends every refusal again.
fn every(server: &str) -> SyncWalk<'_> {
    SyncWalk { server, login_id: ACCOUNT, ..SyncWalk::default() }
}

/// The builds a fixture sync sends under, and a later client's.
const BUILDS: &str = "rbms/0.1.4|server-c1";
const NEXT_CLIENT_BUILDS: &str = "rbms/0.1.5|server-c1";

fn ids(candidates: &[SyncCandidate]) -> Vec<i64> {
    candidates.iter().map(|candidate| candidate.log.id).collect()
}

fn answered(db: &mut ScoreDb, log_id: i64, server: &str, state: SyncState) {
    let row = IrSyncRow { score_id: (state == SyncState::Accepted).then(|| format!("sc_{log_id}")), ..IrSyncRow::new(log_id, server, ACCOUNT, state) };
    db.put_sync_state(&row).expect("the answer is written");
}

fn table_names(db: &ScoreDb) -> Vec<String> {
    let mut stmt = db.conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").expect("prepares");
    stmt.query_map([], |row| row.get::<_, String>(0)).expect("runs").map(|name| name.expect("a name")).collect()
}

#[test]
fn a_generation_one_database_gains_the_sync_tables_and_keeps_its_rows() {
    let dir = std::env::temp_dir().join(format!("rbms_scoredb_sync_generation_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("scoredb.sqlite");
    {
        let db = ScoreDb::open(&path).expect("opened");
        let old_schema = SCHEMA_SQL.split(FIRST_SYNC_TABLE).next().expect("the DDL starts before the sync tables");
        db.conn.execute_batch(old_schema).expect("the first generation's schema applies");
        db.conn.execute("INSERT INTO meta(key, value) VALUES('schema_version', '1')", []).expect("stamped");
        assert!(!table_names(&db).iter().any(|name| name == "ir_sync"), "the fixture predates the sync tables");
    }
    let mut db = ScoreDb::open(&path).expect("reopened");
    let id = stored(&mut db, &play("aa", 10));
    drop(db);

    let mut db = ScoreDb::open(&path).expect("reopened by this build");
    assert_eq!(db.migrate().expect("the schema applies"), SCORE_DB_SCHEMA_VERSION);
    assert_eq!(SCORE_DB_SCHEMA_VERSION, 1, "adding the sync tables leaves the generation where an older build still opens it");
    let tables = table_names(&db);
    assert!(tables.iter().any(|name| name == "ir_sync") && tables.iter().any(|name| name == "play_submission"), "{tables:?}");
    assert_eq!(db.all_plays().expect("the history reads").iter().map(|log| log.id).collect::<Vec<_>>(), [id]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_old_ir_submitted_flag_is_not_read_as_accepted() {
    let mut db = fresh();
    let id = stored(&mut db, &PlayLog { ir_submitted: true, ..play("aa", 10) });
    assert!(db.sync_states(SERVER).expect("reads").is_empty(), "a play handed over before is not known to have been taken");
    let candidates = db.sync_candidates(&every(SERVER)).expect("reads");
    assert_eq!(ids(&candidates), [id]);
    assert_eq!(candidates[0].state, None, "the play is unconfirmed");
}

#[test]
fn sync_candidates_leave_out_accepted_rows_and_old_scratch_auto_runs() {
    let mut db = fresh();
    let taken = stored(&mut db, &play("aa", 10));
    let unsent = stored(&mut db, &play("bb", 20));
    let old_rule = PlayLog { assist: SCRATCH_AUTO_ASSIST, rule_version: SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1, ..play("cc", 5) };
    stored(&mut db, &old_rule);
    let new_rule = stored(&mut db, &PlayLog { assist: SCRATCH_AUTO_ASSIST, ..play("dd", 30) });
    let imported_old = PlayLog { assist: ASSIST_UNRECORDED, rule_version: SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1, ..play("ee", 6) };
    stored(&mut db, &imported_old);
    let imported_current = stored(&mut db, &PlayLog { assist: ASSIST_UNRECORDED, ..play("ff", 40) });
    answered(&mut db, taken, SERVER, SyncState::Accepted);
    assert_eq!(
        ids(&db.sync_candidates(&every(SERVER)).expect("reads")),
        [unsent, new_rule, imported_current],
        "oldest first, only what the server lacks, and an imported assisted play of the old rule may have been a SCRATCH AUTO one"
    );
    assert_eq!(
        ids(&db.sync_candidates(&SyncWalk { only: Some(&[new_rule]), ..every(SERVER) }).expect("reads")),
        [new_rule],
        "the walk narrows to the named plays"
    );
    assert!(db.sync_candidates(&SyncWalk { only: Some(&[taken]), ..every(SERVER) }).expect("reads").is_empty());
}

#[test]
fn sync_candidates_leave_out_runs_played_as_another_account() {
    let mut db = fresh();
    let mine = stored(&mut db, &play("aa", 10));
    let theirs = stored(&mut db, &play("bb", 20));
    let guest = stored(&mut db, &play("cc", 30));
    let before = stored(&mut db, &play("dd", 40));
    db.put_submission(mine, "{}", Some(ACCOUNT)).expect("kept");
    db.put_submission(theirs, "{}", Some(OTHER_ACCOUNT)).expect("kept");
    db.put_submission(guest, "{}", None).expect("kept");
    let candidates = db.sync_candidates(&every(SERVER)).expect("reads");
    assert_eq!(ids(&candidates), [mine, guest, before], "a guest's play and one made before the account was kept go to whoever syncs");
    assert_eq!(candidates[0].played_as.as_deref(), Some(ACCOUNT));
    assert_eq!(candidates[1].played_as, None);
}

#[test]
fn rejected_retry_and_skipped_rows_are_candidates_again_when_every_refusal_is_sent() {
    let mut db = fresh();
    let rejected = stored(&mut db, &play("aa", 10));
    let skipped = stored(&mut db, &play("cc", 30));
    let retry = stored(&mut db, &play("bb", 20));
    answered(&mut db, rejected, SERVER, SyncState::Rejected);
    answered(&mut db, retry, SERVER, SyncState::Retry);
    answered(&mut db, skipped, SERVER, SyncState::Skipped);
    let candidates = db.sync_candidates(&every(SERVER)).expect("reads");
    assert_eq!(ids(&candidates), [rejected, skipped, retry], "the play the server failed to answer goes behind the others");
    assert_eq!(
        candidates.iter().map(|candidate| candidate.state).collect::<Vec<_>>(),
        [Some(SyncState::Rejected), Some(SyncState::Skipped), Some(SyncState::Retry)]
    );
}

/// A refusal written down under the builds a walk sends under would come back the same, so the walk
/// leaves it out; another client build, another server build, a refusal written before the builds
/// were, or the player asking for everything sends it again.
#[test]
fn a_refusal_under_the_same_builds_is_left_out_of_a_walk_under_them() {
    let mut db = fresh();
    let refused = stored(&mut db, &play("aa", 10));
    let unsent = stored(&mut db, &play("bb", 20));
    let refused_before_builds = stored(&mut db, &play("cc", 30));
    db.put_sync_state(&IrSyncRow { refused_under: Some(BUILDS.into()), ..IrSyncRow::new(refused, SERVER, ACCOUNT, SyncState::Rejected) }).expect("written");
    answered(&mut db, refused_before_builds, SERVER, SyncState::Rejected);

    let same = SyncWalk { refused_under: Some(BUILDS), ..every(SERVER) };
    assert_eq!(ids(&db.sync_candidates(&same).expect("reads")), [unsent, refused_before_builds]);
    let next_client = SyncWalk { refused_under: Some(NEXT_CLIENT_BUILDS), ..every(SERVER) };
    assert_eq!(ids(&db.sync_candidates(&next_client).expect("reads")), [refused, unsent, refused_before_builds]);
    assert_eq!(ids(&db.sync_candidates(&every(SERVER)).expect("reads")), [refused, unsent, refused_before_builds], "by hand every refusal goes");
}

/// The plays a server keeps failing to answer go behind the ones it has not been asked about, the
/// one that failed most last, so three such plays at the front of the history cannot stop every
/// sync before the plays after them.
#[test]
fn plays_the_server_keeps_failing_go_behind_the_rest() {
    let mut db = fresh();
    let failing: Vec<i64> = (1..=3).map(|date| stored(&mut db, &play("aa", date))).collect();
    let fresh_play = stored(&mut db, &play("bb", 100));
    for (times, id) in failing.iter().enumerate() {
        for _ in 0..=times {
            answered(&mut db, *id, SERVER, SyncState::Retry);
        }
    }
    assert_eq!(ids(&db.sync_candidates(&every(SERVER)).expect("reads")), [fresh_play, failing[0], failing[1], failing[2]]);
}

/// A walk reaches no further than the newest play when it began: a play stored meanwhile is sent by
/// whoever stored it.
#[test]
fn a_walk_reaches_no_further_than_the_play_it_was_bounded_by() {
    let mut db = fresh();
    let first = stored(&mut db, &play("aa", 10));
    let bound = db.newest_play_id().expect("reads");
    assert_eq!(bound, Some(first));
    let later = stored(&mut db, &play("bb", 20));
    assert_eq!(ids(&db.sync_candidates(&SyncWalk { up_to: bound, ..every(SERVER) }).expect("reads")), [first]);
    assert_eq!(ids(&db.sync_candidates(&every(SERVER)).expect("reads")), [first, later]);
}

#[test]
fn a_replay_refused_under_the_same_builds_is_not_uploaded_again() {
    let mut db = fresh();
    let id = stored(&mut db, &PlayLog { replay_file: Some("aa-10.ron".into()), ..play("aa", 10) });
    db.put_sync_state(&IrSyncRow {
        score_id: Some("sc".into()),
        replay_state: ReplayState::Pending,
        ..IrSyncRow::new(id, SERVER, ACCOUNT, SyncState::Accepted)
    })
    .expect("written");
    let refused = ReplayAnswer { state: ReplayState::Rejected, replay_id: None, error: Some("too large"), refused_under: Some(BUILDS) };
    db.put_replay_state(id, SERVER, &refused).expect("written");
    assert!(db.replay_candidates(&SyncWalk { refused_under: Some(BUILDS), ..every(SERVER) }).expect("reads").is_empty());
    assert_eq!(ids(&db.replay_candidates(&SyncWalk { refused_under: Some(NEXT_CLIENT_BUILDS), ..every(SERVER) }).expect("reads")), [id]);
    assert_eq!(ids(&db.replay_candidates(&every(SERVER)).expect("reads")), [id]);
    assert_eq!(db.sync_states(SERVER).expect("reads")[&id].replay_refused_under.as_deref(), Some(BUILDS));
}

/// A play and the body it is to be sent as are written in one transaction: when the body cannot be
/// kept, the play is not in the history either, and nothing it would have changed has moved.
#[test]
fn a_play_and_its_submission_body_are_written_together() {
    let mut db = fresh();
    let before = stored(&mut db, &play("aa", 10));
    db.conn
        .execute_batch("CREATE TRIGGER refuse_bodies BEFORE INSERT ON play_submission BEGIN SELECT RAISE(ABORT, 'no room for a body'); END")
        .expect("the trigger is made");
    let kept = KeptSubmission { body: "{}", played_as: Some(ACCOUNT) };
    assert!(db.record_play_kept(&play("aa", 20), true, Some(kept)).is_err(), "the body could not be kept");
    assert_eq!(db.all_plays().expect("reads").iter().map(|log| log.id).collect::<Vec<_>>(), [before], "the play went with its body");
    assert_eq!(db.best_in_ln_mode("aa", mode_id("BEAT_7K"), SCORE_LN_MODE_FROM_CHART).expect("reads").expect("a best").playcount, 1);
    assert_eq!(db.day_stats(0, i64::MAX).expect("reads").iter().map(|day| day.playcount).sum::<u32>(), 1);

    db.conn.execute_batch("DROP TRIGGER refuse_bodies").expect("dropped");
    let id = db.record_play_kept(&play("aa", 30), true, Some(kept)).expect("stored with its body");
    let candidate = db.sync_candidates(&SyncWalk { only: Some(&[id]), ..every(SERVER) }).expect("reads").remove(0);
    assert_eq!((candidate.body.as_deref(), candidate.played_as.as_deref()), (Some("{}"), Some(ACCOUNT)));
}

#[test]
fn an_ir_sync_table_that_predates_the_refusal_columns_gains_them() {
    let dir = std::env::temp_dir().join(format!("rbms_scoredb_refusal_columns_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("scoredb.sqlite");
    {
        let db = ScoreDb::open(&path).expect("opened");
        let without = SCHEMA_SQL.replace("    refused_under   TEXT,\n    replay_refused_under TEXT,\n", "");
        assert_ne!(without, SCHEMA_SQL, "the fixture leaves the columns out");
        db.conn.execute_batch(&without).expect("the older schema applies");
        db.conn.execute("INSERT INTO scorelog(chart_key, mode, date) VALUES('aa', 7, 10)", []).expect("a play");
        db.conn.execute("INSERT INTO ir_sync(log_id, server, state) VALUES(1, ?1, 'rejected')", [SERVER]).expect("a refusal");
    }
    let mut db = ScoreDb::open(&path).expect("reopened");
    db.migrate().expect("the columns are added");
    db.migrate().expect("a second migration finds them there");
    let row = db.sync_states(SERVER).expect("reads").remove(&1).expect("the row");
    assert_eq!((row.state, row.refused_under), (SyncState::Rejected, None));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sync_state_upsert_replaces_the_row_and_counts_attempts() {
    let mut db = fresh();
    let id = stored(&mut db, &play("aa", 10));
    let refused = IrSyncRow {
        http_status: Some(400),
        error_code: Some("VALIDATION_ERROR".into()),
        error_field: Some("judge".into()),
        error_reason: Some("judge count exceeds total_notes".into()),
        last_attempt_at_ms: NOW_MS,
        attempts: 99,
        ..IrSyncRow::new(id, SERVER, ACCOUNT, SyncState::Rejected)
    };
    db.put_sync_state(&refused).expect("written");
    let first = db.sync_states(SERVER).expect("reads").remove(&id).expect("a row");
    assert_eq!(first, IrSyncRow { attempts: 1, ..refused.clone() }, "the count is the database's, not the caller's");

    let taken = IrSyncRow {
        score_id: Some("sc_1".into()),
        last_attempt_at_ms: NOW_MS + 1,
        replay_state: ReplayState::Pending,
        ..IrSyncRow::new(id, SERVER, OTHER_ACCOUNT, SyncState::Accepted)
    };
    db.put_sync_state(&taken).expect("written");
    let second = db.sync_states(SERVER).expect("reads").remove(&id).expect("a row");
    assert_eq!(second, IrSyncRow { attempts: 2, ..taken }, "the refusal's columns are gone with it");

    let uploaded = ReplayAnswer { state: ReplayState::Uploaded, replay_id: Some("rp_1"), error: None, refused_under: None };
    db.put_replay_state(id, SERVER, &uploaded).expect("written");
    let third = db.sync_states(SERVER).expect("reads").remove(&id).expect("a row");
    assert_eq!((third.state, third.score_id.as_deref(), third.attempts), (SyncState::Accepted, Some("sc_1"), 2), "a replay leaves the score alone");
    assert_eq!((third.replay_state, third.replay_id.as_deref()), (ReplayState::Uploaded, Some("rp_1")));
}

#[test]
fn a_server_change_keeps_its_own_states() {
    let mut db = fresh();
    let id = stored(&mut db, &play("aa", 10));
    answered(&mut db, id, SERVER, SyncState::Accepted);
    assert!(db.sync_candidates(&every(SERVER)).expect("reads").is_empty());
    assert_eq!(ids(&db.sync_candidates(&every(OTHER_SERVER)).expect("reads")), [id], "the other server has not taken it");
    assert!(db.sync_states(OTHER_SERVER).expect("reads").is_empty());
    assert_eq!(db.sync_states(SERVER).expect("reads").len(), 1);
}

#[test]
fn replay_candidates_are_accepted_rows_with_a_replay_still_to_send() {
    let mut db = fresh();
    let with_replay = |md5: &str, date| PlayLog { replay_file: Some(format!("{md5}-{date}.ron")), ..play(md5, date) };
    let pending = stored(&mut db, &with_replay("aa", 10));
    let failed = stored(&mut db, &with_replay("bb", 20));
    let uploaded = stored(&mut db, &with_replay("cc", 30));
    let no_file = stored(&mut db, &play("dd", 40));
    let refused_score = stored(&mut db, &with_replay("ee", 50));
    for (id, state) in [(pending, ReplayState::Pending), (failed, ReplayState::Failed), (uploaded, ReplayState::Uploaded), (no_file, ReplayState::Pending)] {
        db.put_sync_state(&IrSyncRow { score_id: Some(format!("sc_{id}")), replay_state: state, ..IrSyncRow::new(id, SERVER, ACCOUNT, SyncState::Accepted) })
            .expect("written");
    }
    answered(&mut db, refused_score, SERVER, SyncState::Rejected);
    let candidates = db.replay_candidates(&every(SERVER)).expect("reads");
    assert_eq!(ids(&candidates), [pending, failed]);
    assert_eq!(candidates[0].score_id.as_deref(), Some(format!("sc_{pending}").as_str()));
    assert_eq!(candidates[1].replay_state, Some(ReplayState::Failed));
    assert!(
        db.replay_candidates(&SyncWalk { login_id: OTHER_ACCOUNT, ..every(SERVER) }).expect("reads").is_empty(),
        "a score another account sent is not this one's to link"
    );
}

#[test]
fn the_submission_body_round_trips() {
    let mut db = fresh();
    let id = stored(&mut db, &play("aa", 10));
    db.put_submission(id, "{\"first\":1}", None).expect("kept");
    db.put_submission(id, "{\"second\":2}", Some(ACCOUNT)).expect("replaced");
    let candidate = db.sync_candidates(&every(SERVER)).expect("reads").remove(0);
    assert_eq!((candidate.body.as_deref(), candidate.played_as.as_deref()), (Some("{\"second\":2}"), Some(ACCOUNT)));
    assert_eq!(candidate.log, PlayLog { id, ..play("aa", 10) }, "the play reads back whole through the join");
}

#[test]
fn history_reads_back_the_log_id() {
    let mut db = fresh();
    let first = stored(&mut db, &PlayLog { id: 77, ..play("aa", 10) });
    let second = stored(&mut db, &play("aa", 20));
    assert_ne!(first, 77, "the id a play is given is the database's");
    let history = db.history("aa", 2).expect("reads");
    assert_eq!(history.iter().map(|log| log.id).collect::<Vec<_>>(), [second, first]);
    assert_eq!(db.all_plays().expect("reads").iter().map(|log| log.id).collect::<Vec<_>>(), [first, second]);
}

#[test]
fn backfilling_a_sha256_fills_only_an_empty_one() {
    let mut db = fresh();
    let empty = stored(&mut db, &play("aa", 10));
    let known = stored(&mut db, &PlayLog { sha256: "kept".into(), ..play("bb", 20) });
    db.backfill_sha256(empty, "filled").expect("written");
    db.backfill_sha256(known, "overwritten").expect("written");
    let sha = |md5: &str| db.history(md5, 1).expect("reads")[0].sha256.clone();
    assert_eq!((sha("aa"), sha("bb")), ("filled".to_string(), "kept".to_string()));
}

/// The plays a sync never sends for their SCRATCH AUTO rule read as skipped for it, without a row
/// being written: picking what to send writes nothing.
#[test]
fn old_scratch_auto_rows_read_as_skipped_without_being_written() {
    let mut db = fresh();
    let old = stored(&mut db, &PlayLog { assist: SCRATCH_AUTO_ASSIST, rule_version: SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1, ..play("aa", 10) });
    stored(&mut db, &PlayLog { assist: SCRATCH_AUTO_ASSIST, ..play("bb", 20) });
    stored(&mut db, &play("cc", 30));
    let imported_old =
        stored(&mut db, &PlayLog { assist: ASSIST_UNRECORDED, rule_version: SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1, ..play("dd", 40) });
    stored(&mut db, &PlayLog { assist: ASSIST_UNRECORDED, ..play("ee", 50) });
    let states = db.sync_states(SERVER).expect("reads");
    for log_id in [old, imported_old] {
        let row = &states[&log_id];
        assert_eq!((row.state, row.error_code.as_deref(), row.attempts), (SyncState::Skipped, Some(SKIPPED_OLD_SCRATCH_AUTO_RULE), 0));
    }
    assert_eq!(states.len(), 2);
    let written: i64 = db.conn.query_row("SELECT COUNT(*) FROM ir_sync", [], |row| row.get(0)).expect("counts");
    assert_eq!(written, 0, "nothing was written to say so");
    answered(&mut db, old, SERVER, SyncState::Accepted);
    assert_eq!(db.sync_states(SERVER).expect("reads")[&old].state, SyncState::Accepted, "a row of its own is what it reads as");
}

/// A course is kept by the body it was sent with, and walked by the rules a play is.
#[test]
fn a_kept_course_is_sent_until_the_server_holds_it() {
    let mut db = fresh();
    let mine = db.put_course("course-a", NOW_MS, "{\"a\":1}", Some(ACCOUNT)).expect("kept");
    let guest = db.put_course("course-b", NOW_MS + 1, "{\"b\":1}", None).expect("kept");
    let theirs = db.put_course("course-c", NOW_MS + 2, "{\"c\":1}", Some(OTHER_ACCOUNT)).expect("kept");
    let bound = db.newest_course_id().expect("reads");
    let later = db.put_course("course-d", NOW_MS + 3, "{\"d\":1}", None).expect("kept");
    let walk = SyncWalk { courses_up_to: bound, refused_under: Some(BUILDS), ..every(SERVER) };
    let course_ids = |db: &ScoreDb, walk: &SyncWalk<'_>| db.course_candidates(walk).expect("reads").iter().map(|course| course.id).collect::<Vec<_>>();
    assert_eq!(course_ids(&db, &walk), [mine, guest], "another account's course is not this one's, and the walk is bounded");
    assert_eq!(db.course_candidates(&walk).expect("reads")[0].body, "{\"a\":1}");
    assert!(course_ids(&db, &SyncWalk { only: Some(&[mine]), ..walk }).is_empty(), "a walk of named plays sends no course");

    db.put_course_sync_state(&CourseSyncRow { score_id: Some("cs_1".into()), ..CourseSyncRow::new(mine, SERVER, ACCOUNT, SyncState::Accepted) })
        .expect("written");
    db.put_course_sync_state(&CourseSyncRow { refused_under: Some(BUILDS.into()), ..CourseSyncRow::new(guest, SERVER, ACCOUNT, SyncState::Rejected) })
        .expect("written");
    assert!(course_ids(&db, &walk).is_empty(), "taken, and refused under the same builds");
    assert_eq!(course_ids(&db, &every(SERVER)), [guest, later], "by hand the refusal goes again");
    assert_eq!(db.course_sync_states(SERVER).expect("reads")[&mine].attempts, 1);
    let _ = theirs;
}

#[test]
fn every_state_reads_back_from_its_token_and_an_unknown_one_is_sent_again() {
    for state in SyncState::ALL {
        assert_eq!(SyncState::from_token(state.as_token()), state);
    }
    for state in ReplayState::ALL {
        assert_eq!(ReplayState::from_token(state.as_token()), state);
    }
    assert_eq!(SyncState::from_token("future"), SyncState::Retry);
    assert_eq!(ReplayState::from_token("future"), ReplayState::Pending);
}

use super::schema::SCHEMA_SQL;
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
    let candidates = db.sync_candidates(SERVER, ACCOUNT, None).expect("reads");
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
        ids(&db.sync_candidates(SERVER, ACCOUNT, None).expect("reads")),
        [unsent, new_rule, imported_current],
        "oldest first, only what the server lacks, and an imported assisted play of the old rule may have been a SCRATCH AUTO one"
    );
    assert_eq!(ids(&db.sync_candidates(SERVER, ACCOUNT, Some(&[new_rule])).expect("reads")), [new_rule], "the walk narrows to the named plays");
    assert!(db.sync_candidates(SERVER, ACCOUNT, Some(&[taken])).expect("reads").is_empty());
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
    let candidates = db.sync_candidates(SERVER, ACCOUNT, None).expect("reads");
    assert_eq!(ids(&candidates), [mine, guest, before], "a guest's play and one made before the account was kept go to whoever syncs");
    assert_eq!(candidates[0].played_as.as_deref(), Some(ACCOUNT));
    assert_eq!(candidates[1].played_as, None);
}

#[test]
fn rejected_retry_and_skipped_rows_are_candidates_again() {
    let mut db = fresh();
    let rejected = stored(&mut db, &play("aa", 10));
    let retry = stored(&mut db, &play("bb", 20));
    let skipped = stored(&mut db, &play("cc", 30));
    answered(&mut db, rejected, SERVER, SyncState::Rejected);
    answered(&mut db, retry, SERVER, SyncState::Retry);
    answered(&mut db, skipped, SERVER, SyncState::Skipped);
    let candidates = db.sync_candidates(SERVER, ACCOUNT, None).expect("reads");
    assert_eq!(ids(&candidates), [rejected, retry, skipped]);
    assert_eq!(
        candidates.iter().map(|candidate| candidate.state).collect::<Vec<_>>(),
        [Some(SyncState::Rejected), Some(SyncState::Retry), Some(SyncState::Skipped)]
    );
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

    db.put_replay_state(id, SERVER, ReplayState::Uploaded, Some("rp_1"), None).expect("written");
    let third = db.sync_states(SERVER).expect("reads").remove(&id).expect("a row");
    assert_eq!((third.state, third.score_id.as_deref(), third.attempts), (SyncState::Accepted, Some("sc_1"), 2), "a replay leaves the score alone");
    assert_eq!((third.replay_state, third.replay_id.as_deref()), (ReplayState::Uploaded, Some("rp_1")));
}

#[test]
fn a_server_change_keeps_its_own_states() {
    let mut db = fresh();
    let id = stored(&mut db, &play("aa", 10));
    answered(&mut db, id, SERVER, SyncState::Accepted);
    assert!(db.sync_candidates(SERVER, ACCOUNT, None).expect("reads").is_empty());
    assert_eq!(ids(&db.sync_candidates(OTHER_SERVER, ACCOUNT, None).expect("reads")), [id], "the other server has not taken it");
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
    let candidates = db.replay_candidates(SERVER, ACCOUNT).expect("reads");
    assert_eq!(ids(&candidates), [pending, failed]);
    assert_eq!(candidates[0].score_id.as_deref(), Some(format!("sc_{pending}").as_str()));
    assert_eq!(candidates[1].replay_state, Some(ReplayState::Failed));
    assert!(db.replay_candidates(SERVER, OTHER_ACCOUNT).expect("reads").is_empty(), "a score another account sent is not this one's to link");
}

#[test]
fn the_submission_body_round_trips() {
    let mut db = fresh();
    let id = stored(&mut db, &play("aa", 10));
    db.put_submission(id, "{\"first\":1}", None).expect("kept");
    db.put_submission(id, "{\"second\":2}", Some(ACCOUNT)).expect("replaced");
    let candidate = db.sync_candidates(SERVER, ACCOUNT, None).expect("reads").remove(0);
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

#[test]
fn old_scratch_auto_rows_are_marked_skipped_once() {
    let mut db = fresh();
    let old = stored(&mut db, &PlayLog { assist: SCRATCH_AUTO_ASSIST, rule_version: SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1, ..play("aa", 10) });
    stored(&mut db, &PlayLog { assist: SCRATCH_AUTO_ASSIST, ..play("bb", 20) });
    stored(&mut db, &play("cc", 30));
    let imported_old =
        stored(&mut db, &PlayLog { assist: ASSIST_UNRECORDED, rule_version: SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1, ..play("dd", 40) });
    stored(&mut db, &PlayLog { assist: ASSIST_UNRECORDED, ..play("ee", 50) });
    assert_eq!(db.mark_old_scratch_auto_skipped(SERVER, NOW_MS).expect("marked"), 2);
    assert_eq!(db.mark_old_scratch_auto_skipped(SERVER, NOW_MS + 1).expect("marked"), 0, "a play already marked is left as it is");
    let states = db.sync_states(SERVER).expect("reads");
    for log_id in [old, imported_old] {
        let row = &states[&log_id];
        assert_eq!(
            (row.state, row.error_code.as_deref(), row.attempts, row.last_attempt_at_ms),
            (SyncState::Skipped, Some(SKIPPED_OLD_SCRATCH_AUTO_RULE), 0, NOW_MS)
        );
    }
    assert_eq!(states.len(), 2);
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

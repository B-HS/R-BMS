//! Turning a finished run into the result screen.
//!
//! The tally itself is short; what surrounds it is not. One ended run writes a local record, may
//! save a replay, may move the judge offset, and may submit to the score server — each with its own
//! rule about which runs count. Keeping that here leaves the play screen to the chart it is
//! drawing, and gives the result screen's own work one file to grow into.
#![allow(clippy::wildcard_imports)]

use rbms_render::result::{ResultExtras, TargetView};

use crate::ir_ranking::RankingState;
use crate::ir_session::submission_player_id;
use crate::keyconfig::mode_config_key;
use crate::stage::{PlayState, ResultState, Stage, Transition};
use crate::target::{self, TargetContext};
use crate::*;

/// How much of the chart title the result screen shows before it is cut.
const RESULT_TITLE_CHARS: usize = 48;

/// Leading md5 characters a saved replay's filename is stemmed to.
const REPLAY_STEM_MD5_CHARS: usize = 8;

/// The combo a course stage ended on, which the run summary does not carry: `rbms-play` reports the
/// maximum but not the standing one, and its crate is not part of this phase. Nothing reads it while
/// the gauge and combo carry is unapplied, and the divergence is recorded.
const COMBO_AT_END_UNKNOWN: u32 = 0;

/// Which of the three JUDGE WIDTH tiers the score submission reports, the score server's `judge_rate`
/// being one number rather than six. PGREAT is the tier the setting is read by.
const SUBMITTED_JUDGE_WIDTH_TIER: usize = 0;

/// The gauge a finished run is recorded and submitted under: the one it actually finished on, which
/// GAUGE AUTO SHIFT may have moved away from the gauge the player chose
/// (`BMSPlayer.java:639-650`).
///
/// The reference stores `-1` for a run whose gauge moved (`BMSPlayer.java:884`); neither the score
/// record nor the submission here has such a token, so the run reports the gauge that decided its
/// clear rather than one its own lamp would contradict. `configured` stands in for a course gauge,
/// which no setting selects.
pub(crate) fn reported_gauge(summary: &rbms_play::PlaySummary, configured: rbms_judge::GaugeKind) -> rbms_judge::GaugeKind {
    summary.finished_gauge.unwrap_or(configured)
}

/// What the result screen needs about the chart that was just played, copied out of the session so
/// the score record, the submission and the saved replay can all be built without holding it.
struct ChartRun {
    md5: String,
    sha256: String,
    title: String,
    init_bpm: f64,
    seed: u64,
}

/// Turn the finished run into the result screen: the on-screen summary, the local score record,
/// the saved replay, the auto-calibration step and the IR submission.
///
/// The deltas on the summary compare against the records that existed *before* this play — this
/// run's own record is pushed further down — so "previous" is the newest stored play and "best"
/// is the highest stored EX. The local record is written for every real interactive play, so
/// history and replays survive without a score server; autoplay and replay runs are excluded.
///
/// The TARGET the screen reports is settled against that same standing record rather than the run
/// that has just ended, so the target a run was paced against is the one it is scored against.
pub(crate) fn enter_result(state: &mut PlayState, shared: &mut AppShared) -> Transition {
    let play = &state.session;
    let summary = play.summary();
    let chart = ChartRun {
        md5: play.model().md5.clone(),
        sha256: play.model().sha256.clone(),
        title: play.model().meta.title.clone(),
        init_bpm: play.model().init_bpm,
        seed: play.seed(),
    };
    let judge_setup = play.judge_setup();
    let custom_judge = is_custom_judge(&judge_setup);
    let assist = assist_level(shared.config.play.scratch_auto, custom_judge);
    let practice = state.practice.is_some();
    let last_time_us = play.last_time_us();
    let save_replay = !practice
        &&
        shared.config.play.auto_replay && shared.replay.is_none() && !shared.config.play.autoplay && saves_replay(assist) && !play.recorded_events().is_empty();
    let recorded_events = save_replay.then(|| play.recorded_events().to_vec());
    let calibration_mean_us = play.calibration_mean_us();
    let calibration_samples = play.calibration_samples();
    let instrumentation = play.instrumentation();

    let lamp = assisted_lamp(summary.clear_lamp, assist);
    let (label, color) = clear_label_color(lamp);
    println!(
        "RESULT [{label}]  EX {}/{}  combo {}/{}  gauge {:.1}%  PG/GR/GD/BD/POOR/MISS {:?}  empty-poor {}",
        summary.ex_score, summary.max_ex_score, summary.max_combo, summary.total_notes, summary.gauge_value, summary.counts, summary.empty_poor,
    );
    let history = shared.scores.for_md5(&chart.md5);
    let prev_ex = history.first().map(|r| r.ex_score);
    let prev_best_ex = history.iter().map(|r| r.ex_score).max();
    let view = ResultView {
        title: chart.title.chars().take(RESULT_TITLE_CHARS).collect(),
        mode_label: mode_config_key(shared.mode),
        counts: summary.counts,
        ex_score: summary.ex_score,
        max_score: summary.max_ex_score,
        max_combo: summary.max_combo,
        total_notes: summary.total_notes,
        fast: state.fast(),
        slow: state.slow(),
        gauge: summary.gauge_value,
        clear_label: label,
        clear_color: color,
        prev_best_ex,
        prev_ex,
        show_graph: shared.config.display.score_graph,
        show_result_graphs: shared.config.display.result_graphs,
        gauge_series: instrumentation.gauge_series().to_vec(),
        timing_hist: instrumentation.timing_hist().to_vec().into_boxed_slice(),
        judge_dist: *instrumentation.judge_dist(),
    };

    let assist_tags = assist_flags(shared.config.play.scratch_auto, custom_judge);
    let finished_gauge = reported_gauge(&summary, shared.config.play.gauge);
    let c = summary.counts;
    let played_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let sub = ScoreSubmission {
        api_version: API_VERSION,
        chart: ChartId { md5: chart.md5.clone(), sha256: chart.sha256.clone() },
        player: PlayerId { id: submission_player_id(&shared.session, &shared.config.network.player_id) },
        mode: shared.mode.name.to_string(),
        clear: ir_clear(lamp),
        ex_score: summary.ex_score,
        max_ex_score: summary.max_ex_score,
        judge: JudgeBreakdown {
            pgreat: c[0],
            great: c[1],
            good: c[2],
            bad: c[3],
            poor: c[4],
            miss: c[5],
            fast: summary.fast,
            slow: summary.slow,
            combobreak: combo_breaks(&shared.mode, c),
            epg: summary.early[0],
            lpg: summary.late[0],
            egr: summary.early[1],
            lgr: summary.late[1],
            egd: summary.early[2],
            lgd: summary.late[2],
            ebd: summary.early[3],
            lbd: summary.late[3],
            epr: summary.early[4],
            lpr: summary.late[4],
            ems: summary.early[5],
            lms: summary.late[5],
            avgjudge: summary.avg_judge_us,
            empty_poor: summary.empty_poor,
        },
        max_combo: summary.max_combo,
        total_notes: summary.total_notes,
        passnotes: summary.total_judged,
        minbp: summary.min_bp,
        gauge_value: summary.gauge_value,
        options: PlayOptions {
            gauge: ir_gauge(finished_gauge),
            random: ir_random(shared.config.play.random),
            random_p2: None,
            scratch_auto: shared.config.play.scratch_auto,
            lntype: state.lntype,
            input_device: "keyboard".into(),
            assist: assist_tags,
            option: 0,
            judge_rate: judge_setup.judge_rate_key[SUBMITTED_JUDGE_WIDTH_TIER],
            offset_ms: shared.config.judge.offset_ms,
            constant: shared.config.play.constant_speed,
            hispeed: shared.config.play.hispeed,
            lift: shared.config.play.lift,
            lane_cover: shared.config.play.cover,
            total_override: shared.config.play.total_override,
            autoplay: shared.config.play.autoplay,
            auto_offset: shared.config.judge.auto_offset,
            scratch_left: shared.config.play.scratch_left,
            green_number: green_number_for(shared.config.play.constant_speed, chart.init_bpm, shared.config.play.hispeed, 1.0, shared.config.play.cover),
        },
        played_at,
        client: concat!("rbms/", env!("CARGO_PKG_VERSION")).into(),
        replay_id: None,
        seed: chart.seed,
        judge_algorithm: play.judge().algorithm().name().into(),
        rule: String::new(),
        skin: shared.config.display.skin.clone(),
        client_build_sha256: shared.build_sha256.clone(),
        client_platform: Some(client_platform()),
        extra: Default::default(),
    };
    let scores_count = updates_score(shared.config.play.autoplay, shared.replay.is_some(), custom_judge, shared.config.play.scratch_auto, practice);
    let block_reason = ir_submission_block_reason(shared.config.play.autoplay, shared.replay.is_some(), custom_judge, shared.config.play.scratch_auto, practice);

    let played_ms = played_at;
    let mut replay_file: Option<String> = None;
    let mut recorded_replay: Option<Replay> = None;
    if let Some(events) = recorded_events {
        let stem: String = chart.md5.chars().take(REPLAY_STEM_MD5_CHARS).collect();
        let dir = shared.replay_dir.clone();
        let name = format!("{stem}-{played_ms}.ron");
        let rp = Replay {
            chart_path: shared.chart_path.clone(),
            md5: chart.md5.clone(),
            mode: shared.mode.name.to_string(),
            random: shared.config.play.random.label().to_string(),
            seed: chart.seed,
            offset_ms: shared.config.judge.offset_ms,
            scratch_auto: shared.config.play.scratch_auto,
            gauge: gauge_token(shared.config.play.gauge).to_string(),
            judge: ReplayJudge {
                algorithm: algorithm_token(judge_setup.algorithm).to_string(),
                judge_rate_key: judge_setup.judge_rate_key,
                judge_rate_scratch: judge_setup.judge_rate_scratch,
                longnote_margin_rate: judge_setup.longnote_margin_rate,
                ln_mode: ln_mode_token(judge_setup.ln_mode).to_string(),
                gauge_set: gauge_set_token(judge_setup.gauge_set).to_string(),
                gauge_auto_shift: gauge_auto_shift_token(judge_setup.gauge_auto_shift).to_string(),
                bottom_shiftable_gauge: gauge_token(judge_setup.bottom_shiftable_gauge).to_string(),
            },
            events,
        };
        rp.save(&dir.join(&name));
        replay_file = Some(name);
        recorded_replay = Some(rp);
    }

    if shared.replay.is_none() && !shared.config.play.autoplay && !practice {
        let finished = crate::scoredb_store::FinishedPlay {
            md5: &chart.md5,
            sha256: &chart.sha256,
            title: &chart.title,
            mode: shared.mode,
            ln_mode: &state.ln_mode_key,
            lamp,
            summary: &summary,
            gauge: finished_gauge,
            random: shared.config.play.random,
            seed: chart.seed,
            assist,
            played_at_ms: played_ms,
            playtime_ms: crate::scoredb_store::playtime_ms(last_time_us),
            ir_submitted: block_reason.is_none() && shared.config.network.server_url.is_some(),
            replay_file: replay_file.clone(),
        };
        let log = crate::scoredb_store::play_log(&finished);
        if let Some(db) = shared.scoredb.as_mut() {
            crate::scoredb_store::record_finished_play(db, &log, scores_count);
        }
        shared.scores.push(crate::scoredb_store::record_of(&log));
    }

    if let (true, true, true, Some(mean_us)) = (shared.config.judge.auto_offset, !shared.config.play.autoplay, shared.replay.is_none(), calibration_mean_us) {
        let new_offset = calibrated_offset(shared.config.judge.offset_ms, mean_us);
        println!(
            "auto-cal: avg {:+} ms over {} hits → judge offset {} ms (was {})",
            mean_us / 1000,
            calibration_samples,
            new_offset,
            shared.config.judge.offset_ms
        );
        shared.config.judge.offset_ms = new_offset;
    }

    match block_reason {
        Some(reason) => {
            println!("score not submitted: {reason}");
            shared.submit_rx = None;
            shared.ir_status = IrStatus::Skipped(reason);
        }
        None if shared.config.network.server_url.is_none() => {
            shared.submit_rx = None;
            shared.ir_status = IrStatus::Off;
        }
        None => {
            let chart = sub.chart.clone();
            let replay = recorded_replay.as_ref().and_then(|rp| shared.replay_upload_payload(rp, &chart, state.lntype));
            shared.spawn_profile_submits(&sub);
            shared.spawn_score_submit(sub, replay);
        }
    }

    shared.save_settings();
    shared.dump_timing_csv();
    if practice {
        return Transition::Back;
    }
    if shared.course_run.is_some() {
        return advance_course(shared, &summary, block_reason, lamp);
    }
    let paced_by = run_target(shared, &chart.md5, summary.total_notes, prev_best_ex);
    let extras = ResultExtras { target: Some(TargetView { name: paced_by.name, ex: paced_by.ex }), run_again: offers_retry(shared) };
    Transition::To(Stage::Result(ResultState::new(view).paced_by(extras).cleared(lamp.is_cleared())))
}

/// The target a run on this chart is paced against: the TARGET row settled against the records that
/// stand right now.
///
/// The play HUD and the result screen ask the same question — the one while the run is going, the
/// other once it has ended — so both settle it here rather than each building its own context.
pub(crate) fn run_target(shared: &AppShared, md5: &str, total_notes: u32, local_best_ex: Option<u32>) -> target::ResolvedTarget {
    let (ir_best_ex, rival) = ranking_targets(shared, md5);
    target::resolve(shared.config.judge.target, &TargetContext { total_notes, local_best_ex, ir_best_ex, rival })
}

/// The best EX the score server holds for this account on this chart, and the strongest rival score
/// on it, as far as the browser's ranking cache has been told.
///
/// Nothing is asked for here: the panel's own fetch fills that cache while the chart is focused, so
/// a run started without opening the panel simply has no answer and the target falls back.
fn ranking_targets(shared: &AppShared, md5: &str) -> (Option<u32>, Option<(String, u32)>) {
    let Some(RankingState::Ready(board)) = shared.ranking_cache.peek(md5) else {
        return (None, None);
    };
    let you = board.you.as_ref().map(|row| row.ex_score);
    let rival = board.rivals.iter().filter_map(|(who, row)| row.as_ref().map(|row| (who.clone(), row.ex_score))).max_by_key(|(_, ex)| *ex);
    (you, rival)
}

/// The library index of the chart this run was played on, or `None` for a chart that is not in the
/// library at all — one launched straight from the command line.
fn played_song_index(shared: &AppShared) -> Option<usize> {
    let played = Path::new(&shared.chart_path);
    shared.library.songs().iter().position(|entry| entry.path.as_path() == played)
}

/// The song after this one in the order the browser is currently showing, as a position in that
/// list and an index into the library.
///
/// The search starts from where the played chart sits in the list, so the run follows the sort and
/// the filters that were in force when it was started; a chart the current filters have since
/// dropped falls back to the cursor.
fn following_song(shared: &AppShared) -> Option<(usize, usize)> {
    let played = played_song_index(shared);
    let from = shared.select_items.iter().position(|item| matches!(item, SelectItem::Song(i) if Some(*i) == played)).unwrap_or(shared.sel);
    shared.select_items.iter().enumerate().skip(from + 1).find_map(|(at, item)| match item {
        SelectItem::Song(index) => Some((at, *index)),
        SelectItem::Folder { .. } => None,
    })
}

/// Fold the stage that just ended into the course and go wherever it says: the next stage, or the
/// course result screen.
fn advance_course(shared: &mut AppShared, summary: &rbms_play::PlaySummary, block_reason: Option<&'static str>, lamp: ClearType) -> Transition {
    shared.course_stage_reasons.push(block_reason.map(str::to_string));
    let stage = rbms_course::StageResult {
        ex_score: summary.ex_score,
        max_ex_score: summary.max_ex_score,
        notes: summary.total_notes,
        counts: summary.counts,
        empty_poor: summary.empty_poor,
        fast: summary.fast,
        slow: summary.slow,
        combo_breaks: combo_breaks(&shared.mode, summary.counts),
        max_combo: summary.max_combo,
        combo_at_end: COMBO_AT_END_UNKNOWN,
        gauge_value: summary.gauge_value,
        clear: clear_type_id(lamp),
        survived: !summary.failed,
    };
    let Some(run) = shared.course_run.as_mut() else {
        return Transition::Back;
    };
    match run.advance(&stage) {
        rbms_course::CourseStep::Next => crate::load_course_stage(shared),
        rbms_course::CourseStep::Cleared | rbms_course::CourseStep::Failed => finish_course(shared),
    }
}

/// The course is over: submit it if every stage was submittable, then show the course result.
fn finish_course(shared: &mut AppShared) -> Transition {
    let Some(run) = shared.course_run.as_ref() else {
        return Transition::Back;
    };
    let mut reasons = shared.course_stage_reasons.clone();
    if !run.course.release {
        reasons.push(Some(UNRELEASED_COURSE_REASON.to_string()));
    }
    match crate::course_ir::course_block_reason(&reasons) {
        Some(reason) => println!("course not submitted: {reason}"),
        None if shared.multi_ir.primary_server().is_none() => {}
        None => {
            let sub = build_course_submission(run, &submission_player_id(&shared.session, &shared.config.network.player_id));
            let server = shared.course_ir_server();
            std::thread::spawn(move || {
                let _ = crate::course_ir::submit(server.as_ref(), &sub);
            });
        }
    }
    let state = CourseResultState::of(run);
    Transition::To(Stage::CourseResult(Box::new(state)))
}

/// Start a chart from the browser's list without going back through the browser, which is what both
/// run-again keys do. The LOADING screen replaces the result, so the browser stays suspended under
/// it and the run still returns there when it ends.
fn start_song(shared: &mut AppShared, index: usize) -> Transition {
    shared.release_play_audio();
    Transition::To(Stage::Loading(LoadingState::song(index)))
}

/// Restart the chart that has just been played, on the options it was played on.
///
/// Nothing about the run is carried over — the chart is loaded again from the library — so the
/// options it starts on are whatever [`AppShared::config`] holds, which is what the player last set.
pub(crate) fn retry(shared: &mut AppShared) -> Transition {
    let Some(index) = played_song_index(shared) else {
        return Transition::Stay;
    };
    start_song(shared, index)
}

/// Start the chart after this one in the browser's current order. At the end of the list there is
/// nothing to start, so the key does nothing rather than wrapping around to the top.
pub(crate) fn next_song(shared: &mut AppShared) -> Transition {
    let Some((at, index)) = following_song(shared) else {
        return Transition::Stay;
    };
    shared.sel = at;
    start_song(shared, index)
}

/// Whether a finished run may be restarted or followed on. A run the player did not play — an
/// autoplay demonstration or a replay being watched — has nothing to retry, and neither has a chart
/// that is not in the library to be started again from.
pub(crate) fn offers_retry(shared: &AppShared) -> bool {
    !shared.config.play.autoplay && shared.replay.is_none() && played_song_index(shared).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_ranking::{RankingBoard, RankingRow};

    /// Charts the library fixture holds, in the order a scan found them.
    const FIXTURE_PATHS: [&str; 3] = ["/songs/first.bms", "/songs/second.bms", "/songs/third.bms"];

    fn fresh_shared() -> AppShared {
        let dir = std::env::temp_dir().join(format!("rbms-app-result-tests-{}", std::process::id()));
        crate::App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron")).shared
    }

    fn song(at: usize) -> rbms_library::SongEntry {
        rbms_library::SongEntry {
            path: PathBuf::from(FIXTURE_PATHS[at]),
            title: format!("chart {at}"),
            subtitle: String::new(),
            artist: String::new(),
            genre: String::new(),
            maker: String::new(),
            level: at.to_string(),
            difficulty: 0,
            init_bpm: 120.0,
            rank: 2,
            total: 100.0,
            mode: Mode::BEAT_7K,
            md5: format!("md5-{at}"),
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
        }
    }

    /// A browser holding the three fixture charts in scan order, with the run that just ended having
    /// been played on the one at `played`.
    fn browsing(played: usize) -> AppShared {
        let mut shared = fresh_shared();
        shared.library = Library::from_songs((0..FIXTURE_PATHS.len()).map(song).collect());
        shared.select_items = (0..FIXTURE_PATHS.len()).map(SelectItem::Song).collect();
        shared.sel = played;
        shared.chart_path = FIXTURE_PATHS[played].to_string();
        shared
    }

    fn started_chart(transition: &Transition) -> bool {
        matches!(transition, Transition::To(Stage::Loading(_)))
    }

    #[test]
    fn a_run_the_player_played_may_be_restarted_or_followed_on() {
        let mut shared = browsing(0);
        shared.config.play.autoplay = false;
        shared.replay = None;
        assert!(offers_retry(&shared));
    }

    #[test]
    fn a_demonstration_or_a_watched_replay_offers_neither() {
        let mut shared = browsing(0);
        shared.config.play.autoplay = true;
        assert!(!offers_retry(&shared), "an autoplay run has nothing to retry");

        let mut shared = browsing(0);
        shared.config.play.autoplay = false;
        shared.replay = Some(Replay {
            chart_path: String::new(),
            md5: String::new(),
            mode: String::new(),
            random: String::new(),
            seed: 0,
            offset_ms: 0,
            scratch_auto: false,
            gauge: String::new(),
            judge: Default::default(),
            events: Vec::new(),
        });
        assert!(!offers_retry(&shared), "a replay being watched has nothing to retry either");
    }

    /// A chart launched straight from the command line is not in the library, so there is no entry
    /// to start again from and the keys stand down rather than doing nothing on a promise.
    #[test]
    fn a_chart_that_is_not_in_the_library_offers_neither() {
        let mut shared = browsing(0);
        shared.chart_path = "/elsewhere/loose.bms".to_string();
        assert!(!offers_retry(&shared));
        assert!(matches!(retry(&mut shared), Transition::Stay));
        let mut shared = fresh_shared();
        assert!(!offers_retry(&shared), "an empty library has nothing to start");
        assert!(matches!(retry(&mut shared), Transition::Stay));
    }

    /// The chart a retry starts is the one the run was played on, found in the library by the path
    /// the run was loaded from. The load itself happens on the LOADING screen a frame later, so what
    /// is checked here is the entry that screen is handed.
    #[test]
    fn retrying_starts_the_chart_that_was_just_played_again() {
        let mut shared = browsing(1);
        assert_eq!(played_song_index(&shared), Some(1));
        assert!(started_chart(&retry(&mut shared)), "retry did not start a chart");
        assert_eq!(shared.sel, 1, "retry moved the browser cursor");
    }

    /// The options a retry starts on are the ones the player is holding, which is what makes the key
    /// worth having: change a setting on the result screen's own terms and run it again.
    #[test]
    fn retrying_keeps_the_options_the_run_was_played_on() {
        let mut shared = browsing(1);
        shared.config.play.hispeed = 3.5;
        shared.config.play.random = rbms_chart::shuffle::NoteOption::Mirror;
        retry(&mut shared);
        assert_eq!(shared.config.play.hispeed, 3.5);
        assert_eq!(shared.config.play.random, rbms_chart::shuffle::NoteOption::Mirror);
    }

    #[test]
    fn the_next_song_is_the_one_after_it_in_the_browsers_order() {
        let mut shared = browsing(0);
        assert!(started_chart(&next_song(&mut shared)), "the next song did not start");
        assert_eq!(shared.sel, 1, "the browser cursor did not follow");
    }

    /// The list the key walks is the one the browser is showing, so a reordered or filtered list is
    /// followed rather than the library's own order: after the chart at library index 2 comes
    /// whatever the list puts next, not library index 3.
    #[test]
    fn the_next_song_follows_the_order_the_list_is_in() {
        let mut shared = browsing(2);
        shared.select_items = vec![SelectItem::Song(2), SelectItem::Song(0), SelectItem::Song(1)];
        shared.sel = 0;
        assert_eq!(following_song(&shared), Some((1, 0)), "the walk did not follow the list");
        assert!(started_chart(&next_song(&mut shared)));
        assert_eq!(shared.sel, 1, "the browser cursor did not follow");
    }

    /// Folders sit in the same list as songs; the key has to step over them rather than stopping.
    #[test]
    fn the_next_song_steps_over_a_folder_row() {
        let mut shared = browsing(0);
        shared.select_items = vec![SelectItem::Song(0), SelectItem::Folder { label: "MORE".into(), target: SelectView::Root }, SelectItem::Song(2)];
        assert_eq!(following_song(&shared), Some((2, 2)), "the folder row was taken for a song");
        assert!(started_chart(&next_song(&mut shared)));
        assert_eq!(shared.sel, 2);
    }

    #[test]
    fn at_the_end_of_the_list_the_next_song_key_does_nothing() {
        let mut shared = browsing(2);
        assert!(matches!(next_song(&mut shared), Transition::Stay), "the list wrapped around instead of stopping");
        assert_eq!(shared.sel, 2, "the cursor moved with nowhere to go");
        assert_eq!(shared.chart_path, FIXTURE_PATHS[2]);
    }

    /// A chart the filters have dropped since it was started is no longer in the list, so the walk
    /// carries on from the cursor rather than giving up.
    #[test]
    fn a_chart_the_list_no_longer_holds_is_followed_on_from_the_cursor() {
        let mut shared = browsing(0);
        shared.select_items = vec![SelectItem::Song(1), SelectItem::Song(2)];
        shared.sel = 0;
        assert_eq!(following_song(&shared), Some((1, 2)));
    }

    fn ranking_row(player: &str, ex: u32) -> RankingRow {
        RankingRow { rank: None, player: player.to_string(), ex_score: ex, lamp: "CLEAR", lamp_color: Color::GREEN, fast_slow: None, replay_id: None }
    }

    #[test]
    fn the_ranking_cache_supplies_the_account_and_rival_targets() {
        let mut shared = fresh_shared();
        let board = RankingBoard {
            rows: Vec::new(),
            you: Some(ranking_row("me", 1500)),
            rivals: vec![("quiet".into(), None), ("friend".into(), Some(ranking_row("friend", 1400))), ("rival".into(), Some(ranking_row("rival", 1600)))],
        };
        shared.ranking_cache.insert("md5-0".into(), RankingState::Ready(Box::new(board)));
        let (you, rival) = ranking_targets(&shared, "md5-0");
        assert_eq!(you, Some(1500));
        assert_eq!(rival, Some(("rival".to_string(), 1600)), "the rival to beat is the strongest one who has played it");
    }

    /// Without a server answer the cache holds nothing settled for the chart, and both targets come
    /// back empty so the TARGET row falls back instead of showing a number from another chart.
    #[test]
    fn a_chart_the_ranking_cache_knows_nothing_about_supplies_no_targets() {
        let mut shared = fresh_shared();
        assert_eq!(ranking_targets(&shared, "md5-0"), (None, None));
        shared.ranking_cache.insert("md5-0".into(), RankingState::Loading);
        assert_eq!(ranking_targets(&shared, "md5-0"), (None, None), "a fetch still in flight is not an answer");
        shared.ranking_cache.insert("md5-1".into(), RankingState::Failed("offline".into()));
        assert_eq!(ranking_targets(&shared, "md5-1"), (None, None));
    }

    /// The whole point of the target block is that the screen can pace the run against it, so the
    /// row the player chose has to survive the trip from the settings tab to the result screen.
    #[test]
    fn the_target_row_settles_into_something_the_screen_can_show() {
        let mut shared = browsing(0);
        shared.config.judge.target = rbms_config::ScoreTarget::RateAaa;
        let context = TargetContext { total_notes: 812, local_best_ex: Some(1400), ir_best_ex: None, rival: None };
        let settled = target::resolve(shared.config.judge.target, &context);
        assert_eq!(settled.ex, target::band_target_ex(812, 24));
        assert_eq!(settled.name, rbms_config::ScoreTarget::RateAaa.label());
    }
}

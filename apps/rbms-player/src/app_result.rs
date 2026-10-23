//! Turning a finished run into the result screen.
//!
//! The tally itself is short; what surrounds it is not. One ended run writes a local record, may
//! save a replay, may move the judge offset, and may submit to the score server — each with its own
//! rule about which runs count. Keeping that here leaves the play screen to the chart it is
//! drawing, and gives the result screen's own work one file to grow into.
#![allow(clippy::wildcard_imports)]

use rbms_render::result::{ResultExtras, TargetView};

use rbms_model::TimeLine;
use rbms_store::StagedWrite;
use rbms_store::scoredb::{KeptSubmission, PENDING_RUN_SUFFIX, PendingRun, ReplayState};

use crate::ir_ranking::RankingState;
use crate::ir_replay::wire_carries_judge;
use crate::ir_session::submission_player_id;
use crate::keyconfig::mode_config_key;
use crate::skin_host::result::snapshot::PreviousScore;
use crate::stage::result::{ResultRun, RunStanding};
use crate::stage::{PlayState, ResultState, Stage, Transition};
use crate::target::{self, TargetContext};
use crate::*;

/// How much of the chart title the result screen shows before it is cut.
const RESULT_TITLE_CHARS: usize = 48;

/// Leading md5 characters a saved replay's filename is stemmed to, and a run kept on the side.
const REPLAY_STEM_MD5_CHARS: usize = 8;

/// What a saved replay states of how far from its inputs the run was judged, in milliseconds:
/// nothing. An input is judged at the instant it arrived, and the judge offset the run was played
/// at moved only where its notes were drawn, which playback leaves to whoever watches it.
const REPLAY_JUDGED_WHERE_PLAYED_MS: i32 = 0;

/// How many tempo changes the result screen carries into a document's BPM graph. A chart that keeps
/// changing tempo says as much as the graph can show long before this, and the screen holds the list
/// for as long as it is on.
const BPM_POINT_CAPACITY: usize = 512;

/// Which of the three JUDGE WIDTH tiers the score submission reports, the score server's `judge_rate`
/// being one number rather than six. PGREAT is the tier the setting is read by.
const SUBMITTED_JUDGE_WIDTH_TIER: usize = 0;

/// What the result screen needs about the chart that was just played, copied out of the session so
/// the score record, the submission and the saved replay can all be built without holding it.
struct ChartRun {
    md5: String,
    sha256: String,
    title: String,
    init_bpm: f64,
    seed: u64,
}

/// How the run whose result is being shown came to its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunEnd {
    /// It has nothing left to play: it was played to its end, an empty gauge ended it, or it was
    /// closing when it was left. Its notes are counted as the run counted them.
    Reached,
    /// The player left it with notes still to come. Every note it never reached counts as a late
    /// POOR let pass, and it is FAILED ([`rbms_play::PlaySummary::left_unfinished`]).
    LeftPartWay,
}

/// Turn the run that has nothing left to play into its result screen ([`show_result`]).
pub(crate) fn enter_result(state: &mut PlayState, shared: &mut AppShared) -> Transition {
    show_result(state, shared, RunEnd::Reached)
}

/// Turn a run the player left with notes still to come into its result screen ([`show_result`]),
/// as a FAILED run whose notes not reached were let pass, which is what Escape does to a run played
/// by hand (the player's decision of 2026-10-23). A course stage left so fails the course, which
/// ends on its result.
pub(crate) fn enter_result_of_a_left_run(state: &mut PlayState, shared: &mut AppShared) -> Transition {
    show_result(state, shared, RunEnd::LeftPartWay)
}

/// Turn the run that ended as `end` says into the result screen: the on-screen summary, the local
/// score record, the saved replay and the IR submission. The judge offset a run adjusted is already in the settings
/// by now: it is kept as each input moves it ([`PlayState::keep_adjusted_offset`]).
///
/// The deltas on the summary compare against the records that existed *before* this play — this
/// run's own record is pushed further down — so "previous" is the newest stored play and "best"
/// is the highest stored EX. The local record is written for every real interactive play, so
/// history and replays survive without a score server; autoplay and replay runs are excluded.
///
/// The TARGET the screen reports is settled against the best the play screen paced the run against:
/// the best that stood before this run among the runs that may set one, in the long-note flavour
/// this run was played in ([`ScoreBook::best_ex_for_md5_in_ln_mode`]). An assisted run in the
/// history is no best, so it is no LOCAL BEST here either, and the target a run was paced against is
/// the one it is scored against. The best a skin's result screen compares the run with is read at
/// the same point, before the run joins the book ([`PreviousScore::of_book`]).
///
/// The screen, the history row, the submission and the record a skin's result screen reads all hold
/// the same judgement counts: a run left part-way is counted with its notes not reached as POORs in
/// each of them. Its record for the screen has its gauge history fall to zero where it was left
/// ([`rbms_play::PlaySession::left_record`]); the trace kept with the run is laid out from the history
/// as it was sampled, as it always was, since the trace says by its missing failure instant and its
/// unreached notes that the run was left. This is the one way such a run is sent: there is no
/// records sync of it besides.
fn show_result(state: &mut PlayState, shared: &mut AppShared, end: RunEnd) -> Transition {
    let play = &state.session;
    let (summary, record_shown) = match end {
        RunEnd::Reached => (play.summary(), None),
        RunEnd::LeftPartWay => (play.summary().left_unfinished(), Some(play.left_record())),
    };
    let standing_combo = play.standing_combo();
    let standing_max_combo = play.standing_max_combo();
    let md5 = play.model().md5.clone();
    let title = play.model().meta.title.clone();
    let artist = play.model().meta.artist.clone();
    let practice = state.practice.is_some();
    let instrumentation = play.instrumentation();
    let record = play.record();
    let tempo = tempo_points(&play.model().timelines);

    let scoring_history: Vec<_> = shared.scores.for_md5(&md5).into_iter().filter(|record| !record.assisted).collect();
    let prev_ex = scoring_history.first().map(|r| r.ex_score);
    let prev_best_ex = scoring_history.iter().map(|r| r.ex_score).max();
    let previous = PreviousScore::of_book(&shared.scores, &md5, &state.ln_mode_key);
    let standing_best_ex = shared.scores.best_ex_for_md5_in_ln_mode(&md5, &state.ln_mode_key);

    let recorded = record_run(state, shared, &summary, &record);
    let lamp = recorded.lamp;
    let (label, color) = crate::format::lamp_label_color(lamp, recorded.assist_grade);
    println!(
        "RESULT [{label}]  EX {}/{}  combo {}/{}  gauge {:.1}%  PG/GR/GD/BD/POOR/MISS {:?}  empty-poor {}",
        summary.ex_score, summary.max_ex_score, summary.max_combo, summary.total_notes, summary.gauge_value, summary.counts, summary.empty_poor,
    );
    let view = ResultView {
        title: title.chars().take(RESULT_TITLE_CHARS).collect(),
        artist,
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

    let block_reason = recorded.block_reason;
    match block_reason {
        Some(reason) => {
            println!("score not submitted: {reason}");
            shared.submit_rx = None;
            shared.submit_log = None;
            shared.ir_status = IrStatus::Skipped(reason);
        }
        None if !shared.has_primary_ir_server() => {
            shared.submit_rx = None;
            shared.submit_log = None;
            shared.ir_status = IrStatus::Off;
        }
        None => {
            let chart = recorded.sub.chart.id.clone();
            let replay = recorded.recorded_replay.as_ref().and_then(|rp| shared.replay_upload_payload(rp, &chart, state.lntype));
            shared.spawn_profile_submits(&recorded.sub);
            if shared.multi_ir.legacy_index().is_some() {
                let played =
                    recorded.log_id.map(|log_id| crate::app_ranking::PlayedRun { log_id, replay_plan: recorded.replay_plan, trace_kept: recorded.trace_kept });
                shared.spawn_score_submit(recorded.sub, replay, played);
            } else {
                shared.ir_status = IrStatus::Sending;
            }
        }
    }

    shared.save_settings();
    shared.dump_timing_csv();
    if practice {
        return Transition::Back;
    }
    keep_run_record(shared, record_shown.unwrap_or(record));
    if shared.course_run.is_some() {
        shared.course_assist = shared.course_assist.with_stage(recorded.scratch_auto, recorded.custom_judge);
        let carried = CarriedCombo { standing: standing_combo, longest: standing_max_combo };
        let course_lamp = course_lamp(shared, &state.session);
        return match fold_course_stage(shared, &summary, carried, block_reason, course_lamp) {
            Some(rbms_course::CourseStep::Next) => crate::load_course_stage(shared),
            Some(rbms_course::CourseStep::Cleared | rbms_course::CourseStep::Failed) => finish_course(shared),
            None => Transition::Back,
        };
    }
    let paced_by = run_target(shared, &md5, summary.total_notes, standing_best_ex);
    let target = TargetView { name: paced_by.name, ex: paced_by.ex };
    let standing = RunStanding { replay_saved: recorded.replay_file.is_some(), lamp, previous, target: Some(target.clone()) };
    let extras = ResultExtras { target: Some(target), run_again: offers_retry(shared) };
    let run = ResultRun::of(&state.session, shared, standing);
    Transition::To(Stage::Result(ResultState::new(view).paced_by(extras).cleared(lamp.is_cleared()).tempo(tempo).played(run)))
}

/// What a run that has just ended left behind once it was kept: the lamp it was awarded, the
/// submission built for it, the history row and the replay it was stored as, and why it is not to
/// be sent, if it is not.
pub(crate) struct RecordedRun {
    lamp: ClearType,
    /// The grade inside that lamp of a SCRATCH AUTO clear, by the gauge the run finished on
    /// ([`rbms_judge::assist_grade`]).
    assist_grade: rbms_judge::AssistGrade,
    sub: ScoreSubmission,
    /// The `scorelog` row the run was stored as; `None` for a run that is no record (autoplay, a
    /// replay being watched, practice) or that could not be stored.
    log_id: Option<i64>,
    /// Whether that row was stored with the run's trace ([`crate::play_trace`]).
    trace_kept: bool,
    replay_file: Option<String>,
    recorded_replay: Option<Replay>,
    /// What is to become of the replay once the server takes the run ([`crate::app_ranking::LiveSubmit`]).
    replay_plan: ReplayState,
    block_reason: Option<&'static str>,
    scratch_auto: bool,
    custom_judge: bool,
}

/// Keep a run that has just ended, as `summary` says it went: build its submission, save its replay,
/// and store it in the history with the submission it is to be sent as and its play trace
/// ([`crate::play_trace`]), which goes up once the server holds the score.
///
/// The local record is written for every real interactive run, so history and replays survive
/// without a score server; autoplay, a replay being watched and practice are excluded. The book the
/// screens read gains the run too, so it must be read for "what stood before" first.
///
/// What is recorded is the reference's record of the run (`BMSPlayer.java:863-930`): `avgjudge` is
/// `avg_duration_us`, the mean distance of every note from its input with a note never hit counted
/// at a second ([`rbms_play::PlayRecord::avg_duration_us`]), and the bad-poor count takes in every
/// note the run never reached ([`rbms_play::PlaySummary::min_bp_with_unreached`]). How far the hits
/// leaned, early or late, travels beside it in the submission's `extra`, with the rule generation
/// the figures are of ([`ScoreSubmission::state_mean_offset`], [`ScoreSubmission::state_rule_version`]).
///
/// A run played on a TOTAL of the player's own is eased the way a widened judge eases it
/// ([`crate::judged_wider`]): its lamp is an assist lamp and it sets no best, and its replay records
/// the TOTAL so playback moves the gauge as the run did.
///
/// The run is recorded and submitted under the gauge it finished on
/// ([`rbms_play::PlayRecord::finished_gauge`]), the one its lamp and its assist grade are of: the
/// gauge GAUGE AUTO SHIFT moved it to (`BMSPlayer.java:639-650`), and for a course's stage the course
/// gauge it ran on, which no setting selects. The reference stores `-1` for a run whose gauge moved
/// (`BMSPlayer.java:884`); neither the score record nor the submission here has such a token, so the
/// run reports the gauge that decided its clear rather than one its own lamp would contradict. Its
/// replay names the gauge the settings chose instead, which is what playback starts the run on.
fn record_run(state: &PlayState, shared: &mut AppShared, summary: &rbms_play::PlaySummary, record: &rbms_play::PlayRecord) -> RecordedRun {
    let play = &state.session;
    let avg_duration_us = record.avg_duration_us;
    let chart = ChartRun {
        md5: play.model().md5.clone(),
        sha256: play.model().sha256.clone(),
        title: play.model().meta.title.clone(),
        init_bpm: play.model().init_bpm,
        seed: play.seed(),
    };
    let judge_setup = play.judge_setup();
    let total_override = crate::run_total_override(&shared.config, shared.replay.as_ref());
    let custom_judge = crate::judged_wider(&judge_setup, total_override);
    let scratch_auto = scratch_auto_played(play);
    let assist = assist_level(scratch_auto, custom_judge);
    let practice = state.practice.is_some();
    let last_time_us = play.last_time_us();
    let save_replay =
        !practice && shared.config.play.auto_replay && shared.replay.is_none() && !shared.run_plays_itself() && !play.recorded_events().is_empty();
    let recorded_events = save_replay.then(|| play.recorded_events().to_vec());
    let lamp = assisted_lamp(summary.clear_lamp, assist);
    let finished_gauge = record.finished_gauge;
    let assist_grade = rbms_judge::assist_grade(lamp, Some(finished_gauge));

    let assist_tags = assist_flags(scratch_auto, custom_judge);
    let c = summary.counts;
    let played_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let mut sub = ScoreSubmission {
        api_version: API_VERSION,
        chart: ir_map::submitted_chart(play.model()),
        player: PlayerId { id: submission_player_id(&shared.session) },
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
            avgjudge: wire_avgjudge(avg_duration_us),
            empty_poor: summary.empty_poor,
        },
        max_combo: summary.max_combo,
        total_notes: summary.total_notes,
        passnotes: summary.total_judged,
        minbp: summary.min_bp_with_unreached(),
        gauge_value: summary.gauge_value,
        options: PlayOptions {
            gauge: rbms_ir::mapping::ir_gauge_of_index(finished_gauge),
            random: ir_random(shared.config.play.random),
            random_p2: None,
            scratch_auto,
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
            total_override,
            autoplay: shared.run_plays_itself(),
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
    sub.state_mean_offset(summary.avg_judge_us);
    sub.state_rule_version(rbms_store::SCORE_RULE_VERSION);
    let scores_count = updates_score(shared.run_plays_itself(), shared.replay.is_some(), custom_judge, practice);
    let block_reason = ir_submission_block_reason(shared.run_plays_itself(), shared.replay.is_some(), practice);

    let played_ms = played_at;
    let stem: String = chart.md5.chars().take(REPLAY_STEM_MD5_CHARS).collect();
    let mut replay_file: Option<String> = None;
    let mut recorded_replay: Option<Replay> = None;
    let mut staged_replay: Option<StagedWrite> = None;
    if let Some(events) = recorded_events {
        let name = format!("{stem}-{played_ms}.ron");
        let rp = Replay {
            chart_path: shared.chart_path.clone(),
            md5: chart.md5.clone(),
            mode: shared.mode.name.to_string(),
            random: shared.config.play.random.label().to_string(),
            seed: chart.seed,
            offset_ms: REPLAY_JUDGED_WHERE_PLAYED_MS,
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
                offset_steps: Vec::new(),
                clock: rbms_store::REPLAY_EVENT_CLOCK.to_string(),
                total_override,
            },
            events,
        };
        let path = shared.replay_dir.join(&name);
        match StagedWrite::create(&path) {
            Ok(staged) => {
                staged_replay = Some(staged);
                replay_file = Some(name);
            }
            Err(e) => notify(Level::Error, format!("replay not saved: {e} ({})", path.display())),
        }
        recorded_replay = Some(rp);
    }
    let mut log_id = None;
    let mut trace_kept = false;
    if shared.replay.is_none() && !shared.run_plays_itself() && !practice {
        let finished = crate::scoredb_store::FinishedPlay {
            md5: &chart.md5,
            sha256: &chart.sha256,
            title: &chart.title,
            mode: shared.mode,
            ln_mode: &state.ln_mode_key,
            lamp,
            summary,
            gauge: finished_gauge,
            random: shared.config.play.random,
            seed: chart.seed,
            assist,
            avg_duration_us,
            played_at_ms: played_ms,
            playtime_ms: crate::scoredb_store::playtime_ms(last_time_us),
            ir_submitted: block_reason.is_none() && shared.sends_to_primary_ir_server(),
            replay_file: replay_file.clone(),
        };
        let mut log = crate::scoredb_store::play_log(&finished);
        let body = submission_body(&sub);
        let trace = trace_source(state, shared, record, sub.options.random);
        let played_as = shared.session.is_logged_in().then(|| shared.session.login_id()).flatten().map(str::to_string);
        let kept = body.as_deref().map(|body| KeptSubmission { body, played_as: played_as.as_deref() });
        log_id = shared.scoredb.as_mut().and_then(|db| crate::scoredb_store::record_finished_play(db, &log, scores_count, kept));
        let side_name = format!("{played_ms}-{stem}{PENDING_RUN_SUFFIX}");
        let config_dir = shared.settings_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        match log_id {
            Some(log_id) => {
                trace_kept = true;
                let run = PendingRun { log: log.clone(), updates_best: scores_count, body: None, played_as: None, trace: None };
                let (db_path, _, _) = crate::scoredb_store::score_paths(&config_dir);
                let on_the_side = crate::scoredb_store::pending_runs_dir(&config_dir).join(side_name);
                shared.saves.keep_trace(trace, crate::save_worker::StoredRun { log_id, db_path, on_the_side, run });
            }
            None => {
                let run = PendingRun { log: log.clone(), updates_best: scores_count, body, played_as, trace: None };
                if !keep_on_the_side(shared, run, &side_name, trace) {
                    staged_replay = None;
                    replay_file = None;
                    log.replay_file = None;
                }
            }
        }
        log.id = log_id.unwrap_or_default();
        shared.scores.push(crate::scoredb_store::record_of(&log));
    }
    let replay_plan = match (&recorded_replay, &replay_file) {
        (None, _) | (_, None) => ReplayState::None,
        (Some(rp), Some(_)) if !wire_carries_judge(&rp.judge) => ReplayState::Skipped,
        (Some(_), Some(_)) => ReplayState::Pending,
    };
    if let (Some(staged), Some(file), Some(rp)) = (staged_replay, &replay_file, &recorded_replay) {
        shared.saves.save_replay(file.clone(), rp.clone(), staged);
    }

    RecordedRun { lamp, assist_grade, sub, log_id, trace_kept, replay_file, recorded_replay, replay_plan, block_reason, scratch_auto, custom_judge }
}

/// What the trace of the run `state` holds is made of, whose record is `record`, with the lane
/// option named as its submission names it: copied off the run here, laid out and written on the
/// save thread ([`crate::play_trace::TraceSource`]).
fn trace_source(state: &PlayState, shared: &AppShared, record: &rbms_play::PlayRecord, random: rbms_ir::RandomOption) -> crate::play_trace::TraceSource {
    let play = &state.session;
    let speed_end = crate::play_trace::speed_now(shared, play.model().init_bpm);
    let run = crate::play_trace::TraceRun { mode: shared.mode.name, lntype: state.lntype, random, speed: &state.speed_log, speed_end };
    crate::play_trace::TraceSource::of(play, record, run)
}

/// The submission a stored run is to be sent as, kept beside it so a later records sync sends
/// exactly what the result screen would have. A body that does not serialise is reported and the
/// run kept without one: a sync rebuilds a run that has none from its row.
fn submission_body(sub: &ScoreSubmission) -> Option<String> {
    let mut kept = sub.clone();
    kept.clamp_to_server_bounds();
    let body = kept.to_json();
    if body.is_none() {
        notify(Level::Warn, "submission of the run not kept: it does not serialise");
    }
    body
}

/// What the message bus says of a run the score database could not take, kept on the side.
pub(crate) const RUN_KEPT_ON_THE_SIDE: &str = "score database not open: this run is kept on the side and goes into it when it opens";

/// Keep a run the score database could not take in a file of its own, read in the next time the
/// database opens ([`crate::scoredb_store::import_pending_runs`]), and say so: a run is never lost
/// with the session. Its trace is laid out and added to the file on the save thread. Returns
/// whether the run was handed to the save thread; when it was not, the caller writes no replay for
/// it and names none on its record, since no row will point at one.
fn keep_on_the_side(shared: &AppShared, run: PendingRun, name: &str, trace: crate::play_trace::TraceSource) -> bool {
    let dir = crate::scoredb_store::pending_runs_dir(shared.settings_path.parent().unwrap_or_else(|| Path::new(".")));
    match run.to_ron() {
        Ok(text) => {
            shared.saves.keep_run_on_the_side(dir.join(name), run, text, trace);
            notify(Level::Error, RUN_KEPT_ON_THE_SIDE);
            true
        }
        Err(e) => {
            notify(Level::Error, format!("run not kept: {e}"));
            false
        }
    }
}

/// The `avgjudge` a submission carries for a run whose mean note distance is `avg_duration_us`. A
/// chart with no judged object has none ([`rbms_play::AVG_DURATION_ABSENT_US`]), which the wire
/// states as nothing rather than as a number past every server column.
fn wire_avgjudge(avg_duration_us: i64) -> i64 {
    if avg_duration_us == rbms_play::AVG_DURATION_ABSENT_US { 0 } else { avg_duration_us }
}

/// When what is kept with no screen to report on is sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sending {
    /// Now, as a records sync of the run and a course submission of its own.
    Now,
    /// By the records sync of the next start, the app being about to quit: a request started now
    /// would end with the process, and what it left unanswered would be sent twice.
    NextStart,
}

/// What the message bus says of a course ended before its last stage, once it is kept.
pub(crate) const ABANDONED_COURSE_KEPT: &str = "course ended before its last stage kept as FAILED";

/// End the course being played between two stages, keeping what was played of it.
///
/// The course stands on a stage not yet played: the LOADING or decide screen of it was left with
/// Escape, its chart could not be read or is not in the library, or the app is quitting. A course
/// with a stage played by hand behind it is kept and sent as a FAILED course of the stages it
/// played, as a course left part-way with Escape ends on its result; the stage it stood on is not
/// played and adds nothing. A course with nothing played yet, one the engine plays or one watched as
/// a replay keeps nothing, and a course already over has been kept already. Either way the course
/// ends here, so the next chart played is played as a chart of its own.
pub(crate) fn abandon_course(shared: &mut AppShared, sending: Sending) -> Transition {
    let played_by_hand = !shared.run_plays_itself() && shared.replay.is_none();
    let failed_here = shared.course_run.as_mut().filter(|run| played_by_hand && run.index > 0 && !run.is_finished()).map(|run| {
        run.failed_at = Some(run.index);
        run.clear = rbms_course::CLEAR_FAILED;
    });
    if failed_here.is_some() {
        keep_finished_course(shared, sending);
        notify(Level::Info, ABANDONED_COURSE_KEPT);
    }
    crate::end_course(shared)
}

/// What the message bus says of the run on screen when the app quit, once it is kept.
pub(crate) const QUIT_RUN_KEPT: &str = "run cut short by quitting kept; it is sent on the next start";

/// Keep the run on screen as the app quits, as Escape would have kept it ([`PlayState::has_ended`]
/// says which way, `ended`), but with nothing sent: the records sync of the next start sends it.
///
/// A run with nothing left to play is kept as the result screen keeps it; one left part-way is a
/// FAILED run whose every note not reached was let pass. A course stage folds into the course,
/// which is kept: as it stands when that was its last stage or the stage failed, and as a FAILED
/// course otherwise, the stages after it not being played. A run left part-way that judged nothing
/// at all is not kept, as Escape does not keep it (the player's decision of 2026-10-19), and the
/// course it stood in is kept as it stood before the stage ([`abandon_course`]). Nothing is kept of
/// a run the engine played, a replay being watched or practice.
pub(crate) fn keep_run_cut_short_by_quitting(state: &PlayState, ended: bool, shared: &mut AppShared) {
    if state.practice.is_some() || !crate::stage::play::keeps_a_left_run(shared) {
        if shared.course_run.is_some() {
            crate::end_course(shared);
        }
        return;
    }
    if !ended && state.has_judged_nothing() {
        if shared.course_run.is_some() {
            abandon_course(shared, Sending::NextStart);
        }
        return;
    }
    let summary = if ended { state.session.summary() } else { state.session.summary().left_unfinished() };
    let recorded = record_run(state, shared, &summary, &state.session.record());
    notify(Level::Info, QUIT_RUN_KEPT);
    if shared.course_run.is_none() {
        return;
    }
    shared.course_assist = shared.course_assist.with_stage(recorded.scratch_auto, recorded.custom_judge);
    let carried = CarriedCombo { standing: state.session.standing_combo(), longest: state.session.standing_max_combo() };
    let course_lamp = course_lamp(shared, &state.session);
    match fold_course_stage(shared, &summary, carried, recorded.block_reason, course_lamp) {
        Some(rbms_course::CourseStep::Next) => {
            abandon_course(shared, Sending::NextStart);
        }
        Some(rbms_course::CourseStep::Cleared | rbms_course::CourseStep::Failed) => {
            keep_finished_course(shared, Sending::NextStart);
            crate::end_course(shared);
        }
        None => {}
    }
}

/// What the message bus says of a failed run that START or SELECT started again, once it is kept.
pub(crate) const RETRIED_RUN_KEPT: &str = "failed run kept before starting again";

/// Keep a run that failed and is being started again at once with START or SELECT
/// (`BMSPlayer.java:696-710`), as the result screen would have kept it, and send it.
///
/// The run never reaches its result screen, so it is kept here on the way out, judged as it stood
/// when it failed, and sent by a records sync of this one run, there being no screen to send it. Every run
/// the player played by hand is kept and sent (the player's decision of 2026-10-14); a run the
/// engine played, a replay being watched and practice keep nothing.
pub(crate) fn record_retried_run(state: &PlayState, shared: &mut AppShared) {
    if state.practice.is_some() || shared.run_plays_itself() || shared.replay.is_some() {
        return;
    }
    let summary = state.session.summary();
    let recorded = record_run(state, shared, &summary, &state.session.record());
    send_without_a_screen(shared, &recorded);
    notify(Level::Info, RETRIED_RUN_KEPT);
}

/// Send a run that was kept with no result screen to report on: to the extra profiles at once, and
/// to the main server by a records sync of this one run.
fn send_without_a_screen(shared: &mut AppShared, recorded: &RecordedRun) {
    if recorded.block_reason.is_none() && shared.has_primary_ir_server() {
        shared.spawn_profile_submits(&recorded.sub);
    }
    if let Some(log_id) = recorded.log_id {
        shared.start_record_sync(crate::record_sync::SyncScope::Logs(vec![log_id]));
    }
}

/// Whether SCRATCH AUTO played any of a run: it took a note off a lane it plays, as the run had the
/// chart laid out. The reference raises the assist only where the modifier moved a note off a lane
/// (`AutoplayModifier.modify`), so a chart with no turntable notes, and a mode with no turntable at
/// all (9 keys), is played wholly by hand whatever the switch says, and such a run is no assist.
///
/// A run it did play is recorded and sent like any other and sets bests like any other (the
/// player's decision of 2026-10-14); the assist is what demotes its lamp to LIGHT ASSIST, graded by the gauge, and
/// what the history, the replay and the submission say of it.
pub(crate) fn scratch_auto_played(session: &PlaySession) -> bool {
    session.has_auto_played_notes()
}

/// Keep what the run that just ended left for a skin's result screen.
///
/// A single chart replaces whatever the last run left. A course keeps one record per stage, in the
/// order they were played, because its result screen draws the stages' gauge histories end to end
/// (`SkinGaugeGraphObject.java:126-131`); the list starts over on the course's first stage. Called
/// before the course advances, so the stage index is still the one that was just played.
fn keep_run_record(shared: &mut AppShared, record: rbms_play::PlayRecord) {
    let continues_a_course = shared.course_run.as_ref().is_some_and(|run| run.index > 0);
    if !continues_a_course {
        shared.run_records.clear();
    }
    shared.run_records.push(record);
}

/// The chart's tempo as `(progress through the chart, bpm)`, which is what a document's BPM graph
/// plots.
///
/// One point per tempo the chart holds rather than one per line: a chart states its tempo on every
/// timeline, and the graph is drawn from where that tempo changed. Progress is measured against the
/// last line rather than against how far the run got, so the graph shows the whole chart.
pub(crate) fn tempo_points(timelines: &[TimeLine]) -> Vec<(f32, f64)> {
    let Some(span) = timelines.last().map(|line| line.time_us).filter(|last| *last > 0) else {
        return Vec::new();
    };
    let mut points: Vec<(f32, f64)> = Vec::new();
    for line in timelines {
        if points.len() == BPM_POINT_CAPACITY {
            break;
        }
        if points.last().is_some_and(|(_, bpm)| *bpm == line.bpm) {
            continue;
        }
        points.push(((line.time_us as f64 / span as f64) as f32, line.bpm));
    }
    points
}

/// The target a run on this chart is paced against: the TARGET row settled against the records that
/// stand right now.
///
/// The play HUD and the result screen ask the same question — the one while the run is going, the
/// other once it has ended — so both settle it here rather than each building its own context.
pub(crate) fn run_target(shared: &AppShared, md5: &str, total_notes: u32, local_best_ex: Option<u32>) -> target::ResolvedTarget {
    target::resolve(shared.config.judge.target, shared.config.judge.target_fallback, &target_context(shared, md5, total_notes, local_best_ex))
}

/// What a target of the chart `md5` can be worked out from right now: the note count and the local
/// best that are handed in, and the scores the ranking cache holds ([`ranking_targets`]).
pub(crate) fn target_context(shared: &AppShared, md5: &str, total_notes: u32, local_best_ex: Option<u32>) -> TargetContext {
    let (ir_best_ex, rival) = ranking_targets(shared, md5);
    TargetContext { total_notes, custom_rate_percent: shared.config.judge.target_rate_percent, local_best_ex, ir_best_ex, rival }
}

/// The local best a run of the chart `md5` would be paced against, worked out before the chart is
/// read: the best of the records made under the LN MODE now set and of those made as the chart
/// states its long notes.
///
/// The run counts only the records of its own long-note kind ([`ScoreRecord::ln_mode`]), which is
/// the LN MODE it is played under when the chart leaves its long notes unstated and the chart's own
/// otherwise, and which of the two a chart is is known only once it is read. Every record of one
/// chart is of the same one of the two, so the best of both is the best the run will count.
///
/// [`ScoreRecord::ln_mode`]: rbms_store::ScoreRecord
pub(crate) fn local_best_before_reading(shared: &AppShared, md5: &str) -> Option<u32> {
    let set = ln_mode_token(run_judge_setup(&shared.config, None).ln_mode);
    [set, SCORE_LN_MODE_FROM_CHART].into_iter().filter_map(|ln_mode| shared.scores.best_ex_for_md5_in_ln_mode(md5, ln_mode)).max()
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
pub(crate) fn following_song(shared: &AppShared) -> Option<(usize, usize)> {
    let played = played_song_index(shared);
    let from = shared.select_items.iter().position(|item| matches!(item, SelectItem::Song(i) if Some(*i) == played)).unwrap_or(shared.sel);
    shared.select_items.iter().enumerate().skip(from + 1).find_map(|(at, item)| match item {
        SelectItem::Song(index) => Some((at, *index)),
        SelectItem::Folder { .. } => None,
    })
}

/// The course's streak where a stage ended, which is not the chart's own: the combo the next stage
/// starts from and the longest one of the course so far ([`PlaySession::standing_combo`],
/// [`PlaySession::standing_max_combo`]).
#[derive(Clone, Copy)]
struct CarriedCombo {
    standing: u32,
    longest: u32,
}

/// The lamp the course stands at with the stage just played folded in, demoted by the assists its
/// stages had so far as a single chart's is ([`PlaySession::course_lamp`], `MusicResult.java:402-428`).
/// The stage's own lamp is the chart's record and says nothing of the course: a stage can be a MAX
/// of its own after a stage that broke the course's streak.
fn course_lamp(shared: &AppShared, session: &PlaySession) -> ClearType {
    let (notes_before, counts_before) = shared.course_run.as_ref().map_or((0, [0; 6]), |run| (run.totals.notes, run.totals.counts));
    let assist = assist_level(shared.course_assist.scratch_auto, shared.course_assist.custom_judge);
    assisted_lamp(session.course_lamp(notes_before, counts_before), assist)
}

/// Fold the stage that just ended into the course, and answer where the course goes from here: the
/// next stage, or its end. `None` when no course is being played.
///
/// The course's own combo is the streak carried from stage to stage (`JudgeManager.coursecombo`);
/// the chart's record keeps the stage's own ([`rbms_play::PlaySummary::max_combo`]). `course_lamp`
/// is the lamp the course stands at with this stage in ([`course_lamp`]).
fn fold_course_stage(
    shared: &mut AppShared,
    summary: &rbms_play::PlaySummary,
    carried: CarriedCombo,
    block_reason: Option<&'static str>,
    course_lamp: ClearType,
) -> Option<rbms_course::CourseStep> {
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
        max_combo: carried.longest,
        combo_at_end: carried.standing,
        gauge_value: summary.gauge_value,
        clear: clear_type_id(course_lamp),
        survived: !summary.failed,
    };
    shared.course_run.as_mut().map(|run| run.advance(&stage))
}

/// The course is over: keep and send it if every stage was submittable, then show the course
/// result.
///
/// A stage played with an assist does not keep the course back: the submission states the assists
/// the stages had ([`crate::course_ir::CourseAssist`]), as a single chart's does, and the course
/// result reads from the same fold whether the course may set a best.
///
/// A course that may be sent is kept in the score database whether or not anybody is signed in, and
/// sent until the server holds it ([`AppShared::keep_and_send_course`]), as a played chart is.
fn finish_course(shared: &mut AppShared) -> Transition {
    let Some(run) = shared.course_run.as_ref() else {
        return Transition::Back;
    };
    let state = CourseResultState::of(run, shared);
    keep_finished_course(shared, Sending::Now);
    Transition::To(Stage::CourseResult(Box::new(state)))
}

/// Keep and send the course that is over, cleared or failed, when every stage was submittable and
/// the course is marked for release ([`AppShared::keep_and_send_course`]); sent when `sending` says.
fn keep_finished_course(shared: &mut AppShared, sending: Sending) {
    let Some(run) = shared.course_run.as_ref() else {
        return;
    };
    let mut reasons = shared.course_stage_reasons.clone();
    if !run.course.release {
        reasons.push(Some(UNRELEASED_COURSE_REASON.to_string()));
    }
    let submission = match crate::course_ir::course_block_reason(&reasons) {
        Some(reason) => {
            println!("course not submitted: {reason}");
            None
        }
        None => Some(build_course_submission(run, &submission_player_id(&shared.session), shared.course_assist)),
    };
    match (submission, sending) {
        (Some(sub), Sending::Now) => shared.keep_and_send_course(sub),
        (Some(sub), Sending::NextStart) => {
            shared.keep_course(&sub);
        }
        (None, _) => {}
    }
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
    !shared.run_plays_itself() && shared.replay.is_none() && played_song_index(shared).is_some()
}

#[cfg(test)]
pub(crate) mod record_tests;
#[cfg(test)]
mod target_tests;

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

    /// Four notes on one key lane, which is all a run needs to leave a record behind.
    const RECORD_CHART: &[u8] = b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00111:01010101\n";

    /// How far the fixture run is driven, and at what step: past the first two notes.
    const RECORD_RUN_US: i64 = 3_000_000;
    const RECORD_FRAME_US: i64 = 10_000;

    /// A played run on [`RECORD_CHART`] that has been ticked from the start, so its gauges have a
    /// history, and an app that will take its result.
    fn finished_run() -> (PlayState, AppShared) {
        let src = rbms_parser::parse_with(RECORD_CHART, Default::default());
        let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "record.bms"));
        let mut session = PlaySession::new(model, SessionOptions::default());
        for frame in 0..=RECORD_RUN_US / RECORD_FRAME_US {
            session.tick(SessionClock::at(frame * RECORD_FRAME_US), &mut NullSink);
        }
        let state = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());
        let dir = std::env::temp_dir().join(format!("rbms-app-result-record-tests-{}-{:?}", std::process::id(), std::thread::current().id()));
        let mut config = Config::default();
        config.play.autoplay = false;
        let mut shared = crate::App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron")).shared;
        shared.replay = None;
        (state, shared)
    }

    fn two_stage_course() -> rbms_course::CourseRun {
        let chart = rbms_course::CourseChart { md5: "not-in-the-library".to_string(), ..Default::default() };
        rbms_course::CourseRun::new(rbms_course::Course { charts: vec![chart.clone(), chart], ..Default::default() }, 100.0)
    }

    /// A skin's result screen reads the run from the record the play session took, so the record
    /// has to be the one the session reports and has to replace the last run's.
    #[test]
    fn a_finished_run_leaves_its_record_for_the_result_screen() {
        let (mut state, mut shared) = finished_run();
        let expected = state.session.record();
        assert!(!expected.gauge_log.is_empty(), "the fixture run was ticked, so every gauge has a history");
        assert!(matches!(enter_result(&mut state, &mut shared), Transition::To(Stage::Result(_))));
        assert_eq!(shared.run_records.as_slice(), std::slice::from_ref(&expected));

        let (mut again, _) = finished_run();
        enter_result(&mut again, &mut shared);
        assert_eq!(shared.run_records, [expected], "a second single run replaces the first rather than joining it");
    }

    /// A course result draws its stages' gauge histories end to end, so each stage's record is
    /// kept in order and the list starts over only when a course does.
    #[test]
    fn a_course_keeps_one_record_per_stage_in_the_order_they_were_played() {
        let (mut first, mut shared) = finished_run();
        shared.run_records.push(first.session.record());
        shared.course_run = Some(two_stage_course());
        enter_result(&mut first, &mut shared);
        assert_eq!(shared.run_records.len(), 1, "the first stage of a course starts the list over");

        let (mut second, _) = finished_run();
        let mut run = two_stage_course();
        run.index = 1;
        shared.course_run = Some(run);
        enter_result(&mut second, &mut shared);
        assert_eq!(shared.run_records.len(), 2, "a later stage joins the ones before it");
    }

    fn ranking_row(player: &str, ex: u32) -> RankingRow {
        RankingRow {
            rank: None,
            player: player.to_string(),
            ex_score: ex,
            record_lamp: rbms_store::RecordLamp::default(),
            lamp: "CLEAR",
            lamp_color: Color::GREEN,
            fast_slow: None,
            replay_id: None,
            played_at: 0,
        }
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
        let context = TargetContext { total_notes: 812, local_best_ex: Some(1400), ..TargetContext::default() };
        let settled = target::resolve(shared.config.judge.target, shared.config.judge.target_fallback, &context);
        assert_eq!(settled.ex, target::band_target_ex(812, 24));
        assert_eq!(settled.name, rbms_config::ScoreTarget::RateAaa.label());
    }
}

//! The PLAY screen: the running session, the song clock that drives it, the replay-analysis
//! controls, and the result the run is turned into when it ends.
#![allow(clippy::wildcard_imports)]

use crate::app_play::schedule_position_us;
use crate::ir_session::submission_player_id;
use crate::stage::{Canvas, FrameCtx, KeyInput, ResultState, Stage, StageHandler, Transition};
use crate::*;

/// How many of the most recent judged inputs the analysis overlay shows.
const ANALYSIS_MARKS_SHOWN: usize = 14;

/// Height of the analysis overlay strip along the bottom of the screen.
const ANALYSIS_PANEL_H: f32 = 74.0;

/// Horizontal inset of the analysis playback bar.
const ANALYSIS_BAR_INSET: f32 = 40.0;

/// How much of the chart title the result screen shows before it is cut.
const RESULT_TITLE_CHARS: usize = 48;

/// Leading md5 characters a saved replay's filename is stemmed to.
const REPLAY_STEM_MD5_CHARS: usize = 8;

/// What the result screen needs about the chart that was just played, copied out of the session so
/// the score record, the submission and the saved replay can all be built without holding it.
struct ChartRun {
    md5: String,
    sha256: String,
    title: String,
    init_bpm: f64,
    seed: u64,
}

/// The run in progress: the session (judge state, replay, recording, analysis clock and BGA
/// timeline), the chart's decoded BGA images, and the LN mode the score submission reports.
pub(crate) struct PlayState {
    session: PlaySession,
    bga: std::collections::HashMap<i32, Vec<u8>>,
    /// IR `lntype` of the loaded chart (0=LN, 1=CN, 2=HCN), derived from `#LNMODE` at load time so
    /// score submissions report the actual LN mode instead of a hardcoded value.
    lntype: i32,
    /// Song position this frame, taken once in `update` and reused by `draw`.
    song_us: i64,
}

impl PlayState {
    pub(crate) fn new(session: PlaySession, bga: std::collections::HashMap<i32, Vec<u8>>, lntype: i32) -> PlayState {
        PlayState { session, bga, lntype, song_us: 0 }
    }

    /// Hand the running session the judge settings as they stand right now, so a JUDGE OFFSET or
    /// AUTO CAL changed mid-song takes effect on the very next input rather than on the next chart.
    fn sync_judge_settings(&mut self, shared: &AppShared) {
        self.session.set_judge_offset_us(shared.offset_us());
        self.session.set_auto_calibration(shared.config.judge.auto_offset);
    }

    /// Handle an analysis-mode playback key (only while a replay analysis is active): pause/resume,
    /// playback rate, and one seek step either way. Returns whether the key was an analysis control.
    fn analysis_key(&mut self, shared: &AppShared, code: KeyCode) -> bool {
        self.sync_judge_settings(shared);
        if !self.session.analysis_enabled() {
            return false;
        }
        let play = &mut self.session;
        match code {
            KeyCode::Space => play.toggle_analysis_pause(),
            KeyCode::Equal => play.adjust_analysis_rate(1),
            KeyCode::Minus => play.adjust_analysis_rate(-1),
            KeyCode::PageUp => play.seek(play.analysis_position_us() + ANALYSIS_SEEK_STEP_US),
            KeyCode::PageDown => play.seek(play.analysis_position_us() - ANALYSIS_SEEK_STEP_US),
            _ => return false,
        }
        true
    }

    /// Turn the finished run into the result screen: the on-screen summary, the local score record,
    /// the saved replay, the auto-calibration step and the IR submission.
    ///
    /// The deltas on the summary compare against the records that existed *before* this play — this
    /// run's own record is pushed further down — so "previous" is the newest stored play and "best"
    /// is the highest stored EX. The local record is written for every real interactive play, so
    /// history and replays survive without a score server; autoplay and replay runs are excluded.
    fn enter_result(&mut self, shared: &mut AppShared) -> Transition {
        let play = &self.session;
        let summary = play.summary();
        let chart = ChartRun {
            md5: play.model().md5.clone(),
            sha256: play.model().sha256.clone(),
            title: play.model().meta.title.clone(),
            init_bpm: play.model().init_bpm,
            seed: play.seed(),
        };
        let save_replay = shared.config.play.auto_replay && shared.replay.is_none() && !shared.config.play.autoplay && !play.recorded_events().is_empty();
        let recorded_events = save_replay.then(|| play.recorded_events().to_vec());
        let calibration_mean_us = play.calibration_mean_us();
        let calibration_samples = play.calibration_samples();

        let lamp = summary.clear_lamp;
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
            counts: summary.counts,
            ex_score: summary.ex_score,
            max_score: summary.max_ex_score,
            max_combo: summary.max_combo,
            total_notes: summary.total_notes,
            fast: summary.fast,
            slow: summary.slow,
            gauge: summary.gauge_value,
            clear_label: label,
            clear_color: color,
            prev_best_ex,
            prev_ex,
            show_graph: shared.config.display.score_graph,
        };

        let assist = assist_flags(shared.config.play.scratch_auto, shared.config.judge.judge_rate);
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
                gauge: ir_gauge(shared.config.play.gauge),
                random: ir_random(shared.config.play.random),
                random_p2: None,
                scratch_auto: shared.config.play.scratch_auto,
                lntype: self.lntype,
                input_device: "keyboard".into(),
                assist,
                option: 0,
                judge_rate: shared.config.judge.judge_rate,
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
        let scores_count = updates_score(shared.config.play.autoplay, shared.replay.is_some(), shared.config.judge.judge_rate, shared.config.play.scratch_auto);
        let block_reason =
            ir_submission_block_reason(shared.config.play.autoplay, shared.replay.is_some(), shared.config.judge.judge_rate, shared.config.play.scratch_auto);

        let played_ms = played_at;
        let mut replay_file: Option<String> = None;
        let mut recorded_replay: Option<Replay> = None;
        if let Some(events) = recorded_events {
            let stem: String = chart.md5.chars().take(REPLAY_STEM_MD5_CHARS).collect();
            let dir = shared.settings_path.parent().map(|d| d.join("replays")).unwrap_or_else(|| PathBuf::from("replays"));
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
                events,
            };
            rp.save(&dir.join(&name));
            replay_file = Some(name);
            recorded_replay = Some(rp);
        }

        if shared.replay.is_none() && !shared.config.play.autoplay {
            let record = ScoreRecord {
                md5: chart.md5.clone(),
                title: chart.title.clone(),
                mode: shared.mode.name.to_string(),
                clear: clear_type_id(lamp),
                ex_score: summary.ex_score,
                max_ex: summary.max_ex_score,
                counts: summary.counts,
                empty_poor: summary.empty_poor,
                max_combo: summary.max_combo,
                total_notes: summary.total_notes,
                gauge: gauge_token(shared.config.play.gauge).to_string(),
                gauge_value: summary.gauge_value,
                random: shared.config.play.random.label().to_string(),
                played_at: played_ms,
                replay_file,
                rule_version: SCORE_RULE_VERSION,
                assisted: !scores_count,
            };
            shared.scores.push(record);
            shared.scores.save(&shared.scores_path);
        }

        if let (true, true, true, Some(mean_us)) = (shared.config.judge.auto_offset, !shared.config.play.autoplay, shared.replay.is_none(), calibration_mean_us)
        {
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
                let replay = recorded_replay.as_ref().and_then(|rp| shared.replay_upload_payload(rp, &chart, self.lntype));
                shared.spawn_score_submit(sub, replay);
            }
        }

        shared.save_settings();
        shared.dump_timing_csv();
        Transition::To(Stage::Result(ResultState::new(view)))
    }

    /// The lane press/release path: judged against the live song clock, with the keysound played on
    /// the raw input time so a judge offset never moves the sound.
    ///
    /// Without an output stream the press still goes through the session, into a sink that drops
    /// it. The session is what records the replay, so skipping the call would leave a run with
    /// releases but no presses — a replay that cannot be played back.
    fn lane_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        if shared.config.play.autoplay || shared.replay.is_some() {
            return;
        }
        let Some(lane) = shared.lane_for(key.code) else {
            return;
        };
        let raw = shared.song_us();
        self.sync_judge_settings(shared);
        if key.pressed {
            let anchor = shared.anchor_us;
            let hit = match shared.audio.as_mut() {
                Some(audio) => self.session.press(lane, raw, &mut PlayAudioSink::new(audio, anchor)),
                None => self.session.press(lane, raw, &mut NullSink),
            };
            shared.push_timing_sample(raw, hit.map(|r| r.delta_us));
        } else if key.released {
            self.session.release(lane, raw);
        }
    }

    /// The replay-analysis overlay: a playback bar, the current rate/paused state and time, plus the
    /// recent per-note timing errors (ms early = cyan +, late = orange -).
    fn draw_analysis(&self, canvas: &mut Canvas<'_>, song: i64) {
        let play = &self.session;
        let total = play.last_time_us().max(1);
        let prog = (song as f32 / total as f32).clamp(0.0, 1.0);
        let bx = ANALYSIS_BAR_INSET;
        let bw = CW as f32 - ANALYSIS_BAR_INSET * 2.0;
        let by = CH as f32 - 14.0;
        canvas.fill_rect(Rect::new(0.0, CH as f32 - ANALYSIS_PANEL_H, CW as f32, ANALYSIS_PANEL_H), Color { r: 0, g: 0, b: 0, a: 170 });
        canvas.fill_rect(Rect::new(bx, by, bw, 6.0), Color::rgb(40, 40, 52));
        canvas.fill_rect(Rect::new(bx, by, bw * prog, 6.0), Color::rgb(90, 200, 230));
        canvas.fill_rect(Rect::new((bx + bw * prog - 1.5).max(bx), by - 3.0, 3.0, 12.0), Color::WHITE);
        let state = if play.analysis_paused() { "PAUSED".to_string() } else { format!("{:.2}x", play.analysis_rate()) };
        draw_text(canvas, bx, CH as f32 - 66.0, 1.4, Color::YELLOW, &format!("ANALYSIS  {state}   {:.1} / {:.1} S", song as f32 / 1e6, total as f32 / 1e6));
        draw_text_right(canvas, bx + bw, CH as f32 - 66.0, 1.1, Color::GRAY, "SPACE PAUSE  -/+ SPEED  PGUP/PGDN SEEK  ESC EXIT");
        let mut mx = bx;
        for mark in play.recent_marks(ANALYSIS_MARKS_SHOWN) {
            let col = if mark.delta_us > 0 {
                Color::rgb(90, 210, 230)
            } else if mark.delta_us < 0 {
                Color::ORANGE
            } else {
                Color::WHITE
            };
            let txt = format!("{:+}", mark.delta_us / 1000);
            draw_text(canvas, mx, CH as f32 - 44.0, 1.3, col, &txt);
            mx += text_width(&txt, 1.3) + 12.0;
        }
    }
}

impl StageHandler for PlayState {
    /// In manual analysis the displayed song time comes from the virtual clock (pausable,
    /// rate-scaled); otherwise it follows the real (audio) clock, and analysis mirrors it so a
    /// first manual control resumes from the live position. Manual analysis mutes keysounds.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let manual = self.session.analysis_manual();
        let (song, lookahead) = if manual {
            let frame_us = (ctx.dt as f64 * 1_000_000.0) as i64;
            (self.session.advance_analysis(frame_us), 0)
        } else {
            let (s, lookahead) = ctx.shared.song_and_lookahead_us();
            self.session.sync_analysis_position(s);
            (s, lookahead)
        };
        self.song_us = song;
        ctx.shared.push_clock_sample(song);
        self.sync_judge_settings(ctx.shared);
        let clock = SessionClock { audible_us: song, scheduled_us: schedule_position_us(song, lookahead, ctx.shared.schedule_poll_us, manual) };
        let anchor = ctx.shared.anchor_us;
        match (manual, ctx.shared.audio.as_mut()) {
            (false, Some(audio)) => self.session.tick(clock, &mut PlayAudioSink::new(audio, anchor)),
            _ => self.session.tick(clock, &mut NullSink),
        }
        if self.session.is_finished(song) {
            return self.enter_result(ctx.shared);
        }
        Transition::Stay
    }

    /// Escape with nothing left to hit (every note resolved) goes straight to the result screen
    /// rather than discarding the run; otherwise it quits out of the chart.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if key.code == KeyCode::Escape {
            if self.session.all_notes_resolved() {
                return self.enter_result(ctx.shared);
            }
            ctx.shared.save_settings();
            return ctx.shared.leave_play();
        }
        if key.pressed && self.analysis_key(ctx.shared, key.code) {
            return Transition::Stay;
        }
        if key.pressed
            && let Some(action) = ctx.shared.control_for(key.code)
        {
            ctx.shared.apply_control(action);
            return Transition::Stay;
        }
        self.lane_key(ctx.shared, &key);
        Transition::Stay
    }

    /// The green number on the HUD is the note travel time (ms) for the scroll speed shown right
    /// now: fixed in CONSTANT mode (2000/hi-speed at the calibration BPM), and in FLOATING mode
    /// tracking the BPM of the timeline segment under the current play time.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let song = self.song_us;
        let play = &self.session;
        match (ctx.shared.config.display.bga, ctx.shared.skin.bga, self.bga.get(&play.bga_frame())) {
            (true, Some(rect), Some(img)) => canvas.set_bga(img, rect),
            _ => canvas.clear_bga(),
        }
        render_playfield_view(
            canvas,
            &ctx.shared.skin,
            &PlayfieldView {
                timelines: &play.model().timelines,
                microtime: song,
                hispeed: ctx.shared.config.play.hispeed,
                beam_on: play.beam_on(),
                beam_off: play.beam_off(),
                constant: ctx.shared.config.play.constant_speed,
            },
        );
        render_lane_cover(canvas, &ctx.shared.skin, ctx.shared.config.play.cover);
        render_key_bomb(canvas, &ctx.shared.skin, play.bomb(), song);
        let j = play.judge();
        let tls = &play.model().timelines;
        let seg = tls.binary_search_by(|t| t.time_us.cmp(&song)).unwrap_or_else(|i| i.saturating_sub(1));
        let (bpm, scroll) = tls.get(seg).map(|t| (t.bpm, t.scroll)).unwrap_or((play.model().init_bpm, 1.0));
        let green = green_number_for(ctx.shared.config.play.constant_speed, bpm, ctx.shared.config.play.hispeed, scroll, ctx.shared.config.play.cover);
        let best_ex = ctx.shared.scores.best_ex_for_md5(&play.model().md5);
        let hud = HudView {
            combo: j.combo,
            last_judge: j.last_judge.map(|x| x as u8),
            last_fast: j.last_fast,
            fast: j.fast,
            slow: j.slow,
            counts: j.counts,
            ex_score: j.ex_score,
            gauge: j.gauge.value(),
            green_number: green,
            max_ex: j.total_notes() * 2,
            best_ex,
        };
        render_hud(canvas, &ctx.shared.skin, &hud);
        if play.analysis_enabled() {
            self.draw_analysis(canvas, song);
        }
    }

    fn debug_lines(&self, ctx: &FrameCtx<'_>) -> Vec<String> {
        let j = self.session.judge();
        vec![
            format!("TIME {:.2} / {:.2} S", self.song_us as f32 / 1e6, self.session.last_time_us() as f32 / 1e6),
            format!("NOTES {} / {}", j.total_judged(), j.total_notes()),
            format!("COMBO {}  MAX {}", j.combo, j.max_combo),
            format!("EX {}  GAUGE {:.1}%", j.ex_score, j.gauge.value()),
            format!("FAST {}  SLOW {}  EPOOR {}", j.fast, j.slow, j.empty_poor),
            format!("HISPEED {:.2}  OFFSET {:+}MS", ctx.shared.config.play.hispeed, ctx.shared.config.judge.offset_ms),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::KeyInput;
    use crate::{App, Config, LaunchOptions};
    use rbms_play::SessionOptions;

    /// A one-measure 7-key chart, enough for a session with real lanes.
    const CHART: &str = "#PLAYER 1\n#TITLE t\n#BPM 120\n#WAV01 a.wav\n#00111:0101\n";

    /// An app with no window, no audio and no server, set to interactive play so the lane path is
    /// live. `App::new` never opens an output stream, so `shared.audio` is the "no device" case.
    fn app() -> App {
        let dir = std::env::temp_dir().join(format!("rbms-play-stage-tests-{}", std::process::id()));
        let mut config = Config::default();
        config.play.autoplay = false;
        let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
        app.shared.replay = None;
        app
    }

    fn play_state() -> PlayState {
        let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
        let mode = rbms_chart::detect_mode(&src, "t.bms");
        let model = rbms_chart::to_model(&src, mode);
        PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0)
    }

    fn key(code: KeyCode, pressed: bool) -> KeyInput<'static> {
        KeyInput { code, pressed, released: !pressed, text: None }
    }

    #[test]
    fn a_press_is_recorded_for_the_replay_even_with_no_output_device() {
        let mut app = app();
        assert!(app.shared.audio.is_none(), "the fixture is the audio-unavailable case");
        let lane_key = app.shared.active_keys.first().map(|(code, _)| *code).expect("the default key config binds lane 0");

        let mut state = play_state();
        let now = std::time::Instant::now();
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key(lane_key, true));
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key(lane_key, false));

        let events = state.session.recorded_events();
        assert_eq!(events.len(), 2, "a run with no device still records what was played: {events:?}");
        assert!(events[0].press, "the press is recorded first");
        assert!(!events[1].press, "and the release after it — never a release on its own");
    }

    #[test]
    fn a_submission_reports_the_candidate_policy_the_run_actually_used() {
        let state = play_state();
        let reported = state.session.judge().algorithm();
        assert_eq!(reported.name(), rbms_judge::JudgeAlgorithm::default().name(), "a fresh run uses the engine default");
        assert_eq!(reported.name(), "Duration", "and the default is the |dt|-nearest candidate rule this engine judges with");
    }
}

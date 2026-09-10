//! The PLAY screen: the running session, the song clock that drives it, the replay-analysis
//! controls, and the result the run is turned into when it ends.
#![allow(clippy::wildcard_imports)]

#[cfg(test)]
mod tests;

use crate::app_input::{ControlContext, ControlEffect, FixedSpeed, green_for_hispeed};
use crate::app_play::schedule_position_us;
use crate::app_result::{enter_result, run_target};
use crate::keyconfig::mode_config_key;
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::target::ResolvedTarget;
use crate::*;
use rbms_config::FixHiSpeed;
use rbms_render::playfield::LaneShade;
use rbms_render::skin_render::state::PlayViewState;
use rbms_render::{HudPace, KEY_LANE_KIND, LANE_KIND_COUNT, SCRATCH_LANE_KIND, TextureId};
use rbms_skin::timer::timer_id;

/// Microseconds in one millisecond, which is the unit the play clock is kept in and the unit a
/// document reads the play head in.
const MICROS_PER_MILLI: i64 = 1_000;

/// How long Escape has to be held down to abandon a run under [`PlayEscape::Hold`].
const ESC_HOLD: Duration = Duration::from_millis(PLAY_ESCAPE_HOLD_MS);

/// How long a second Escape still counts as the other half of a pair under [`PlayEscape::Double`].
const ESC_DOUBLE: Duration = Duration::from_millis(PLAY_ESCAPE_DOUBLE_MS);

/// How many of the most recent judged inputs the analysis overlay shows.
const ANALYSIS_MARKS_SHOWN: usize = 14;

/// Height of the analysis overlay strip along the bottom of the screen.
const ANALYSIS_PANEL_H: f32 = 74.0;

/// Horizontal inset of the analysis playback bar.
const ANALYSIS_BAR_INSET: f32 = 40.0;

/// The tempos a chart is played at, so a green number can be pinned to one of them.
///
/// The four are the reference implementation's own (`LaneRenderer.java:127-144`): the tempo the
/// chart opens at, its slowest, its fastest, and the one the most notes are played at. Which of them
/// a run uses is the SPEED FIX row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BpmStats {
    start: f64,
    min: f64,
    max: f64,
    main: f64,
}

impl BpmStats {
    /// Read the four tempos off a chart.
    ///
    /// The main tempo is the one carrying the most notes. A chart that splits its notes evenly
    /// between two tempos keeps the first of them, where the reference leaves the choice to hash
    /// order; a run has to pick the same tempo every time it loads the chart, or its pinned speed
    /// would move between two launches of the same file.
    pub(crate) fn of(model: &rbms_model::Model) -> BpmStats {
        let mut stats = BpmStats { start: model.init_bpm, min: model.init_bpm, max: model.init_bpm, main: model.init_bpm };
        let mut by_bpm: Vec<(f64, u32)> = Vec::new();
        for tl in &model.timelines {
            stats.min = stats.min.min(tl.bpm);
            stats.max = stats.max.max(tl.bpm);
            let notes = tl.notes.iter().flatten().filter(|n| !matches!(n.kind, rbms_model::NoteKind::Mine { .. })).count() as u32;
            match by_bpm.iter_mut().find(|(bpm, _)| *bpm == tl.bpm) {
                Some((_, count)) => *count += notes,
                None => by_bpm.push((tl.bpm, notes)),
            }
        }
        let mut most = 0;
        for (bpm, count) in by_bpm {
            if count > most {
                most = count;
                stats.main = bpm;
            }
        }
        stats
    }

    /// The tempo the SPEED FIX row pins to, or `None` when it pins to nothing.
    pub(crate) fn target(&self, fix: FixHiSpeed) -> Option<f64> {
        match fix {
            FixHiSpeed::Off => None,
            FixHiSpeed::StartBpm => Some(self.start),
            FixHiSpeed::MinBpm => Some(self.min),
            FixHiSpeed::MaxBpm => Some(self.max),
            FixHiSpeed::MainBpm => Some(self.main),
        }
    }
}

/// What this run's scroll speed is being held to.
///
/// `base` is the speed the chart started at, which is what a pinned green number steps in multiples
/// of. `fixed` is the green number being held, and the tempo it is held at, for a run whose SPEED
/// FIX row names one.
#[derive(Clone, Copy, Debug)]
struct RunSpeed {
    base: f64,
    fixed: Option<FixedSpeed>,
}

/// The EX a target would hold this far into a run: its final EX scaled by how much of the chart has
/// been judged, so what the pacemaker compares is the run against the target's pace rather than
/// against its finished total.
fn pace_ex_at(target_ex: u32, judged: u32, total: u32) -> i64 {
    if total == 0 {
        return 0;
    }
    (u64::from(target_ex) * u64::from(judged) / u64::from(total)) as i64
}

/// The run in progress: the session (judge state, replay, recording, analysis clock and BGA
/// timeline), the chart's decoded BGA images, and the LN mode the score submission reports.
pub(crate) struct PlayState {
    pub(crate) session: PlaySession,
    bga: std::collections::HashMap<i32, crate::DecodedImage>,
    /// IR `lntype` of this run (0=LN, 1=CN, 2=HCN): the chart's own `#LNMODE` when it states one,
    /// and the LN MODE the run was judged under when it does not.
    pub(crate) lntype: i32,
    /// The long-note key this run's record is compared within, so a chart forced to charge notes —
    /// twice the judged objects, twice the EX ceiling — does not overwrite the best of the same
    /// chart played as plain long notes.
    pub(crate) ln_mode_key: String,
    /// Song position this frame, taken once in `update` and reused by `draw`.
    song_us: i64,
    /// When Escape went down and is still down, for the mode that asks for it to be held.
    esc_down_at: Option<Instant>,
    /// When Escape was last pressed, for the mode that asks for two presses.
    esc_pressed_at: Option<Instant>,
    /// The four tempos of this chart, read off it once.
    bpm: BpmStats,
    /// What the scroll speed is being held to, taken from the settings the first time this run
    /// needs it. The screen is built before it can see them, so it is filled in on first use rather
    /// than in the constructor.
    speed: Option<RunSpeed>,
    /// Whether a free modifier key is down, which is what asks a lane shade for its finer step.
    fine_held: bool,
    /// The target this run is paced against, settled the first time the HUD needs it. It is fixed
    /// for the run: the records it is settled against are the ones that stood when the run started,
    /// which is what the result screen scores it against too.
    pace_target: Option<ResolvedTarget>,
}

/// The frame `PlaySession::bga_frame` reports before the chart's first background event, which is
/// what a chart with no events of its own stays on.
#[cfg(test)]
pub(crate) const NO_BGA_FRAME: i32 = -1;

/// What a play document needs from the frame beyond the HUD snapshot: where the play head is, the
/// tempo and scroll speed shown beside it, and the background image this frame decoded.
struct DocumentFrame {
    song_us: i64,
    bpm: f64,
    hispeed: f64,
    background: Option<TextureId>,
}

impl PlayState {
    pub(crate) fn new(session: PlaySession, bga: std::collections::HashMap<i32, crate::DecodedImage>, lntype: i32, ln_mode_key: String) -> PlayState {
        let bpm = BpmStats::of(session.model());
        PlayState {
            session,
            bga,
            lntype,
            ln_mode_key,
            song_us: 0,
            esc_down_at: None,
            esc_pressed_at: None,
            bpm,
            speed: None,
            fine_held: false,
            pace_target: None,
        }
    }

    /// Early hits of this run, split into the keys and the turntable.
    ///
    /// The split is kept by the session rather than here because not every input comes through this
    /// screen: a replay reproduces its own and an auto-played lane is judged inside the session, and
    /// a tally kept here would report both as nothing at all.
    ///
    /// It counts inputs, so it is not the judge engine's own `fast`/`slow`: those include the notes
    /// an auto-played lane resolved, which nobody hit. The debug overlay reports both, side by side.
    pub(crate) fn fast(&self) -> [u32; LANE_KIND_COUNT] {
        PlayState::lane_kinds(self.session.instrumentation().fast())
    }

    /// Late hits of this run, split the same way as [`PlayState::fast`].
    pub(crate) fn slow(&self) -> [u32; LANE_KIND_COUNT] {
        PlayState::lane_kinds(self.session.instrumentation().slow())
    }

    /// Put the session's two columns where the screens expect to read them, naming both ends, so
    /// the two crates' orderings cannot quietly swap the keys and the turntable.
    fn lane_kinds(split: [u32; rbms_play::LANE_KIND_COUNT]) -> [u32; LANE_KIND_COUNT] {
        let mut out = [0; LANE_KIND_COUNT];
        out[KEY_LANE_KIND] = split[rbms_play::KEY_LANE_KIND];
        out[SCRATCH_LANE_KIND] = split[rbms_play::SCRATCH_LANE_KIND];
        out
    }

    /// What this run's scroll speed is being held to, filled in from the settings on first use.
    ///
    /// A run whose SPEED FIX row names a tempo is pinned to the travel time it starts with: the
    /// speed the settings hold, read at that tempo, becomes the number the run keeps
    /// (`LaneRenderer.java:155-157` takes the same reading for its own base speed).
    fn run_speed(&mut self, shared: &AppShared) -> RunSpeed {
        if let Some(speed) = self.speed {
            return speed;
        }
        let base = shared.config.play.hispeed;
        let fixed =
            self.bpm.target(shared.config.play.fix_hispeed).map(|bpm| FixedSpeed { bpm, green: green_for_hispeed(bpm, base, shared.effective_cover()) });
        let speed = RunSpeed { base, fixed };
        self.speed = Some(speed);
        speed
    }

    /// Settle the target this run is paced against, once, from the records that stand right now.
    ///
    /// It is fixed for the rest of the run: the records it is settled against are the ones the
    /// result screen scores the run against too, so the line the HUD paces to and the line the
    /// summary reports are the same one.
    fn ensure_pace_target(&mut self, shared: &AppShared) {
        if self.pace_target.is_some() {
            return;
        }
        let md5 = self.session.model().md5.clone();
        let total_notes = self.session.judge().total_notes();
        let best = shared.scores.best_ex_for_md5_in_ln_mode(&md5, &self.ln_mode_key);
        self.pace_target = Some(run_target(shared, &md5, total_notes, best));
    }

    /// Apply one in-play control and keep a pinned green number in step with what it moved.
    ///
    /// Moving the speed itself re-reads the green number the run is now at, so the next lane-cover
    /// change holds the number the player just chose. Moving the cover holds the number and moves
    /// the speed instead, which is what keeps a covered field reading the same as an uncovered one
    /// (`LaneRenderer.java:212-215`).
    fn in_play_control(&mut self, shared: &mut AppShared, action: ControlAction) {
        let speed = self.run_speed(shared);
        let ctx = ControlContext { fine: self.fine_held, base_hispeed: speed.base, fixed: speed.fixed };
        match shared.apply_control(action, &ctx) {
            ControlEffect::Speed => {
                let refreshed =
                    speed.fixed.map(|fixed| FixedSpeed { green: green_for_hispeed(fixed.bpm, shared.config.play.hispeed, shared.effective_cover()), ..fixed });
                self.speed = Some(RunSpeed { fixed: refreshed, ..speed });
            }
            ControlEffect::Shade => {
                if let Some(fixed) = speed.fixed {
                    shared.retarget_hispeed(fixed);
                }
            }
            ControlEffect::None => {}
        }
    }

    /// Whether a key is free to act as the fine-step modifier for this run.
    ///
    /// A shift key is the modifier the option asks for, but on the shipped seven-key layout the left
    /// one is the turntable. A key that plays a lane or runs a control is left to that job, so the
    /// modifier never eats an input the chart needs.
    fn is_fine_modifier(shared: &AppShared, code: KeyCode) -> bool {
        matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) && shared.lane_input_for(code).is_none() && shared.control_for(code).is_none()
    }

    /// Hand the running session the judge settings as they stand right now, so a JUDGE OFFSET or
    /// AUTO CAL changed mid-song takes effect on the very next input rather than on the next chart.
    fn sync_judge_settings(&mut self, shared: &AppShared) {
        self.session.set_judge_offset_us(shared.offset_us());
        self.session.set_auto_calibration(shared.config.judge.auto_offset);
    }

    /// Whether the run is over and the result is due: the song ended, or the gauge emptied under a
    /// shift mode that does not rescue it — the reference moves to `STATE_FAILED` and stops judging
    /// there (`BMSPlayer.java:653-661, 694`) rather than tallying a dead run to the end of the song.
    ///
    /// A replay being scrubbed never ends on its own, failed or not: the player is driving the
    /// clock and can scrub back out of it.
    fn run_is_over(&self, song_us: i64) -> bool {
        self.session.is_finished(song_us) || (!self.session.analysis_enabled() && self.session.is_failed())
    }

    /// Escape while a chart is still running.
    ///
    /// A run is abandoned outright, on a press held for [`ESC_HOLD`], or on a second press within
    /// [`ESC_DOUBLE`], depending on the ESCAPE setting. The two guarded modes are what decision 6
    /// asks for; the immediate one is what the key has always done and is what a fresh install
    /// holds, so the setting adds a guard rather than moving one.
    ///
    /// Every mode acts on the press, as every other screen in the app does. A release carried over
    /// from the screen before — the Escape that left it — would otherwise abandon the run the
    /// moment it started.
    fn escape_key(&mut self, ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> Transition {
        match ctx.shared.config.play.play_escape {
            PlayEscape::Immediate => {
                if !key.pressed {
                    return Transition::Stay;
                }
                self.leave_run(ctx)
            }
            PlayEscape::Hold => {
                if key.released {
                    self.esc_down_at = None;
                } else if key.pressed {
                    self.esc_down_at = Some(ctx.now);
                }
                Transition::Stay
            }
            PlayEscape::Double => {
                if !key.pressed {
                    return Transition::Stay;
                }
                let paired = self.esc_pressed_at.is_some_and(|first| ctx.now.duration_since(first) <= ESC_DOUBLE);
                if paired {
                    self.esc_pressed_at = None;
                    return self.leave_run(ctx);
                }
                self.esc_pressed_at = Some(ctx.now);
                Transition::Stay
            }
        }
    }

    /// Whether Escape has now been held long enough to abandon the run. Checked every frame, since
    /// a key that is simply held down produces no further events.
    fn escape_hold_elapsed(&self, now: Instant) -> bool {
        self.esc_down_at.is_some_and(|since| now.duration_since(since) >= ESC_HOLD)
    }

    /// Abandon the run: keep whatever the in-play controls changed, and go back to the browser.
    fn leave_run(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.esc_down_at = None;
        self.esc_pressed_at = None;
        ctx.shared.save_settings();
        ctx.shared.leave_play()
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
        let Some((lane, dir)) = shared.lane_input_for(key.code) else {
            return;
        };
        let raw = shared.song_us();
        self.sync_judge_settings(shared);
        if key.pressed {
            let anchor = shared.anchor_us;
            let hit = match shared.audio.as_mut() {
                Some(audio) => self.session.press_dir(lane, dir, raw, &mut PlayAudioSink::new(audio, anchor)),
                None => self.session.press_dir(lane, dir, raw, &mut NullSink),
            };
            shared.push_timing_sample(raw, hit.map(|r| r.delta_us));
        } else if key.released {
            self.session.release_dir(lane, dir, raw);
        }
    }

    /// The replay-analysis overlay: a playback bar, the current rate/paused state and time, plus the
    /// recent per-note timing errors (ms early = cyan +, late = orange -).
    /// Draw the document this run's layout is selected for, and report whether it drew.
    ///
    /// The mode decides which document: a five-key chart reaches for the five-key screen, and a mode
    /// no document type covers goes straight back to the built-in field. The document itself was
    /// read and compiled at the top of the frame, because whether one exists is also what decides
    /// where the chart's background image goes. The timers are switched from the same HUD snapshot
    /// the built-in screen is drawn from, so a document's judgement flash and the HUD's own counter
    /// can never disagree about what just happened.
    fn draw_document(&self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>, hud: &HudView<'_>, frame: DocumentFrame) -> bool {
        let DocumentFrame { song_us, bpm, hispeed, background } = frame;
        let Some(skin_type) = mode_skin_type(ctx.shared.mode).filter(|skin_type| ctx.shared.has_skin_document(*skin_type)) else {
            return false;
        };
        let now_ms = ctx.shared.skin_now_ms();
        let total_notes = self.session.judge().total_notes();
        if self.session.is_failed() && !ctx.shared.skin_timers.is_on(timer_id::FAILED) {
            ctx.shared.skin_play_timers.fail(&mut ctx.shared.skin_timers, now_ms);
        }
        ctx.shared.skin_play_timers.update(&mut ctx.shared.skin_timers, hud, total_notes, now_ms);
        let state = PlayViewState {
            hud,
            title: &self.session.model().meta.title,
            song_ms: song_us / MICROS_PER_MILLI,
            duration_ms: self.session.last_time_us() / MICROS_PER_MILLI,
            bpm,
            hispeed,
            autoplay: ctx.shared.config.play.autoplay && ctx.shared.replay.is_none(),
            now_ms,
            offsets: None,
        };
        ctx.shared.draw_play_skin(canvas, skin_type, &state, background)
    }

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
    /// A run starting is the moment the reference switches the play timer on and the ready timer
    /// off, which is what a document's opening animation is measured from.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        let now_ms = ctx.shared.skin_now_ms();
        ctx.shared.skin_play_timers.start(&mut ctx.shared.skin_timers, now_ms);
    }

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
        if self.run_is_over(song) {
            return enter_result(self, ctx.shared);
        }
        if self.escape_hold_elapsed(ctx.now) {
            return self.leave_run(ctx);
        }
        Transition::Stay
    }

    /// Escape with nothing left to hit (every note resolved) goes straight to the result screen
    /// rather than discarding the run; otherwise it abandons the chart, in the way the ESCAPE
    /// setting asks for.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if PlayState::is_fine_modifier(ctx.shared, key.code) {
            if key.pressed {
                self.fine_held = true;
            } else if key.released {
                self.fine_held = false;
            }
        }
        if key.code == KeyCode::Escape {
            if key.pressed && self.session.all_notes_resolved() {
                return enter_result(self, ctx.shared);
            }
            return self.escape_key(ctx, &key);
        }
        if key.pressed && self.analysis_key(ctx.shared, key.code) {
            return Transition::Stay;
        }
        if key.pressed
            && let Some(action) = ctx.shared.control_for(key.code)
        {
            self.in_play_control(ctx.shared, action);
            return Transition::Stay;
        }
        self.lane_key(ctx.shared, &key);
        Transition::Stay
    }

    /// The green number on the HUD is the note travel time (ms) for the scroll speed shown right
    /// now: fixed in CONSTANT mode (2000/hi-speed at the calibration BPM), read against the tempo
    /// the SPEED FIX row pins to when it names one, and otherwise tracking the BPM and SCROLL of the
    /// timeline segment under the play head. The white number beside it is what the rest of the
    /// travel time is spent under the lane cover.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.ensure_pace_target(ctx.shared);
        let song = self.song_us;
        let play = &self.session;
        let skin_type = mode_skin_type(ctx.shared.mode);
        if let Some(skin_type) = skin_type {
            ctx.shared.prepare_skin(canvas, skin_type);
        }
        let built_in_background = skin_type.is_none_or(|skin_type| !ctx.shared.has_skin_document(skin_type));
        let frame_image = ctx.shared.config.display.bga.then(|| self.bga.get(&play.bga_frame())).flatten();
        let mut document_background = None;
        match (built_in_background, ctx.shared.skin.bga, frame_image) {
            (true, Some(rect), Some(img)) => canvas.set_background(img.generation, &img.rgba, img.width, img.height, rect),
            (false, _, Some(img)) => {
                canvas.clear_bga();
                document_background = canvas.background_texture(img.generation, &img.rgba, img.width, img.height);
            }
            _ => canvas.clear_bga(),
        }
        let constant = ctx.shared.config.play.constant_speed;
        let hispeed = ctx.shared.config.play.hispeed;
        let cover = ctx.shared.effective_cover();
        let j = play.judge();
        let tls = &play.model().timelines;
        let seg = tls.binary_search_by(|t| t.time_us.cmp(&song)).unwrap_or_else(|i| i.saturating_sub(1));
        let (segment_bpm, scroll) = tls.get(seg).map(|t| (t.bpm, t.scroll)).unwrap_or((play.model().init_bpm, 1.0));
        let pinned = self.bpm.target(ctx.shared.config.play.fix_hispeed);
        let (bpm, shown_scroll) = match pinned {
            Some(bpm) => (bpm, 1.0),
            None => (segment_bpm, scroll),
        };
        let green = green_number_for(constant, bpm, hispeed, shown_scroll, cover);
        let white = match ctx.shared.config.display.show_white_number {
            true => (green_number_for(constant, bpm, hispeed, shown_scroll, 0.0) - green).max(0.0),
            false => 0.0,
        };
        let best_ex = ctx.shared.scores.best_ex_for_md5_in_ln_mode(&play.model().md5, &self.ln_mode_key);
        let hud = HudView {
            mode_label: mode_config_key(ctx.shared.mode),
            combo: j.combo,
            last_judge: j.last_judge.map(|x| x as u8),
            last_fast: j.last_fast,
            fast: self.fast(),
            slow: self.slow(),
            counts: j.counts,
            ex_score: j.ex_score,
            gauge: j.gauge.value(),
            green_number: green,
            white_number: white,
            judge_text_y: ctx.shared.config.display.judge_text_y,
            max_ex: j.total_notes() * 2,
            best_ex,
            pace: self
                .pace_target
                .as_ref()
                .map(|target| HudPace { name: &target.name, delta: j.ex_score as i64 - pace_ex_at(target.ex, j.total_judged(), j.total_notes()) }),
        };
        if self.draw_document(ctx, canvas, &hud, DocumentFrame { song_us: song, bpm, hispeed, background: document_background }) {
            return;
        }
        render_playfield_view(
            canvas,
            &ctx.shared.skin,
            &PlayfieldView {
                timelines: &self.session.model().timelines,
                microtime: song,
                hispeed,
                beam_on: self.session.beam_on(),
                beam_off: self.session.beam_off(),
                constant,
                legacy_note: ctx.shared.config.play.legacy_note,
            },
        );
        render_lane_cover(canvas, &ctx.shared.skin, LaneShade { cover, hidden: ctx.shared.effective_hidden() });
        render_key_bomb(canvas, &ctx.shared.skin, self.session.bomb(), song);
        render_hud(canvas, &ctx.shared.skin, &hud);
        if self.session.analysis_enabled() {
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
            format!("PLAYED FAST {:?}  SLOW {:?}", self.fast(), self.slow()),
            format!("HISPEED {:.2}  OFFSET {:+}MS", ctx.shared.config.play.hispeed, ctx.shared.config.judge.offset_ms),
        ]
    }
}

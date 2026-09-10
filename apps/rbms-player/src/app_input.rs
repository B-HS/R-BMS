//! `App` methods: lane/control input mapping, settings + skin/font round-trip, replay
//! playback & analysis, and the key-config editor. Split out of the crate root; `use crate::*`
//! pulls in the crate-root types/consts/helpers these methods reference.
//!
//! Assembling the browser's row list moved to [`crate::stage::select`], which is the screen that
//! owns it.
#![allow(clippy::wildcard_imports)]

use crate::*;

/// The tempo a pinned green number is held at, and the travel time it is being held at.
///
/// A run that pins its green number keeps this instead of a scroll speed: the speed is whatever
/// puts a note's travel time at `green` when the chart is at `bpm`, so the number the player reads
/// stays put while the chart's own tempo does not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FixedSpeed {
    pub(crate) bpm: f64,
    pub(crate) green: f64,
}

/// What the in-play controls need to know about the run they are adjusting.
///
/// None of it belongs to the configuration: the base speed and the pinned tempo are properties of
/// the chart that is up, and the modifier is the state of a key right now.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ControlContext {
    /// Whether the finer of the two lane-shade steps was asked for.
    pub(crate) fine: bool,
    /// The scroll speed the run started at. A pinned green number steps in multiples of it, so one
    /// press moves the speed by the same proportion whatever tempo the chart is at
    /// (`LaneRenderer.java:238-240`).
    pub(crate) base_hispeed: f64,
    /// The green number this run is pinned to, when it is pinned to one.
    pub(crate) fixed: Option<FixedSpeed>,
}

/// What an in-play control moved, so the run can keep a pinned green number in step with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ControlEffect {
    /// Nothing moved: the row was already at the end of its range.
    None,
    /// The scroll speed itself moved, so a pinned green number follows it to its new value.
    Speed,
    /// The lane cover moved, so a pinned green number holds and the scroll speed follows it
    /// (`LaneRenderer.java:212-215`).
    Shade,
}

/// The scroll speed that puts a note's travel time at `green` ms on a chart at `bpm`.
///
/// This is the inverse of the engine's own green number (`rbms_chart::scroll::green_number`) rather
/// than a constant copied from anywhere: that one divides 240000 by the tempo and the speed and
/// scales the result by the window the cover leaves, so solving it for the speed is the line below.
/// The reference implementation states the same relation in its own units
/// (`LaneRenderer.java:206-210`).
pub(crate) fn hispeed_for_green(bpm: f64, green: f64, cover: f32) -> f64 {
    if bpm <= 0.0 || green <= 0.0 {
        return rbms_config::DEFAULT_HISPEED;
    }
    (MS_PER_MEASURE_NUM / bpm / green) * f64::from(1.0 - cover)
}

/// Travel time in ms of a note at `bpm` and `hispeed`, with `cover` of the field hidden.
///
/// The chart's own SCROLL is left at one here: a pinned green number is pinned to a tempo the whole
/// chart is measured against, not to whichever timeline the play head happens to be in.
pub(crate) fn green_for_hispeed(bpm: f64, hispeed: f64, cover: f32) -> f64 {
    rbms_chart::scroll::green_number(bpm, hispeed, 1.0, f64::from(cover))
}

/// A measure's length in milliseconds at one beat per minute — the numerator the engine's own green
/// number divides by the tempo. Taken from the engine's constant rather than restated, so the
/// forward and inverse directions cannot drift apart.
const MS_PER_MEASURE_NUM: f64 = rbms_model::US_PER_MEASURE_NUM / US_PER_MS;

/// Microseconds in a millisecond.
const US_PER_MS: f64 = 1000.0;

/// One step in the increasing direction, for the controls that come in up/down pairs.
const STEP_UP: f32 = 1.0;

/// One step in the decreasing direction.
const STEP_DOWN: f32 = -1.0;

impl AppShared {
    pub(crate) fn lane_for(&self, code: KeyCode) -> Option<usize> {
        self.active_keys.iter().find(|(k, _)| *k == code).map(|(_, l)| *l)
    }

    /// Which lane a key plays and which way it spins it. A scratch lane has one key per direction
    /// and the judge engine needs to know which one arrived, because spinning the other way is what
    /// ends a charge note (`JudgeManager.java:358-372`). Every other binding spins forward.
    pub(crate) fn lane_input_for(&self, code: KeyCode) -> Option<(usize, ScratchDir)> {
        match self.lane_for(code) {
            Some(lane) => Some((lane, ScratchDir::Forward)),
            None => self.active_reverse_keys.iter().find(|(k, _)| *k == code).map(|(_, lane)| (*lane, ScratchDir::Backward)),
        }
    }

    /// Which configured in-play control (if any) a key triggers.
    pub(crate) fn control_for(&self, code: KeyCode) -> Option<ControlAction> {
        ControlAction::ALL.into_iter().find(|a| self.keyconfig.control_key(*a) == Some(code))
    }

    /// One frame of controller input, or nothing when there is no controller.
    ///
    /// The two fields are taken apart rather than reached through `self`, because the poll needs
    /// the controller mutably and its bindings immutably at the same time.
    pub(crate) fn poll_pad(&mut self) -> Vec<PadEvent> {
        let AppShared { pad, keyconfig, mode, .. } = self;
        let Some(pad) = pad.as_mut() else {
            return Vec::new();
        };
        pad.poll_now(&keyconfig.pad, *mode)
    }

    /// Whether the course in progress forbids the hi-speed and lane-shade controls
    /// (`BMSPlayer.java:419-423` disables the control outright rather than clamping it).
    pub(crate) fn course_locks_speed(&self) -> bool {
        self.course_overrides.as_ref().is_some_and(|overrides| !overrides.accepts_speed_input())
    }

    /// How much of the field the lane cover hides right now: the amount the row holds while the
    /// cover is switched on, and nothing while it is off.
    ///
    /// The amount and the switch are separate values, as they are in the reference implementation
    /// (`PlayConfig.java:62,66`), so switching a cover off and back on returns the height the player
    /// had rather than a default. The same shape holds for the lift and the hidden band, which is
    /// what lets SUD+ and HID+ be up at the same time.
    pub(crate) fn effective_cover(&self) -> f32 {
        if self.config.play.enable_cover { self.config.play.cover } else { LANE_SHADE_MIN }
    }

    /// How much of the field the hidden band hides right now.
    pub(crate) fn effective_hidden(&self) -> f32 {
        if self.config.play.enable_hidden { self.config.play.hidden } else { LANE_SHADE_MIN }
    }

    /// How far the lift raises the judgment line right now.
    pub(crate) fn effective_lift(&self) -> f32 {
        if self.config.play.enable_lift { self.config.play.lift } else { LANE_SHADE_MIN }
    }

    /// Apply an in-play control: hi-speed, lane cover (sudden), lift and the hidden band, clamped
    /// to the ranges their settings rows declare, and answer what moved.
    ///
    /// The hi-speed step is the one the HI-SPEED STEP row holds, so a player can trade reach for
    /// precision without leaving the chart, and the lane shades take the finer of their two steps
    /// while the modifier is down (`PlayConfig.java:87,91`).
    pub(crate) fn apply_control(&mut self, action: ControlAction, ctx: &ControlContext) -> ControlEffect {
        if self.course_locks_speed() {
            return ControlEffect::None;
        }
        let step = if ctx.fine { self.config.play.lanecover_step_fine } else { LANE_SHADE_STEP };
        match action {
            ControlAction::HiSpeedUp => self.step_hispeed(ctx, STEP_UP),
            ControlAction::HiSpeedDown => self.step_hispeed(ctx, STEP_DOWN),
            ControlAction::CoverUp => {
                self.config.play.cover = shaded(self.config.play.cover, step * STEP_UP);
                ControlEffect::Shade
            }
            ControlAction::CoverDown => {
                self.config.play.cover = shaded(self.config.play.cover, step * STEP_DOWN);
                ControlEffect::Shade
            }
            ControlAction::HiddenUp => {
                self.config.play.hidden = shaded(self.config.play.hidden, step * STEP_UP);
                ControlEffect::None
            }
            ControlAction::HiddenDown => {
                self.config.play.hidden = shaded(self.config.play.hidden, step * STEP_DOWN);
                ControlEffect::None
            }
            ControlAction::LiftUp => {
                self.config.play.lift = shaded(self.config.play.lift, step * STEP_UP);
                self.rebuild_skin();
                ControlEffect::None
            }
            ControlAction::LiftDown => {
                self.config.play.lift = shaded(self.config.play.lift, step * STEP_DOWN);
                self.rebuild_skin();
                ControlEffect::None
            }
        }
    }

    /// Move the scroll speed one step.
    ///
    /// A run with a pinned green number steps in multiples of the speed it started at, so one press
    /// is the same proportional change whatever the chart's tempo is. The bounds are tested
    /// exclusively, as they are in the reference implementation (`LaneRenderer.java:236-245`): a step
    /// that would land on or past either end is not taken at all rather than clamped onto it.
    fn step_hispeed(&mut self, ctx: &ControlContext, direction: f32) -> ControlEffect {
        let margin = self.config.play.hispeed_step;
        let step = if ctx.fixed.is_some() { ctx.base_hispeed * margin } else { margin };
        let Some(next) = rbms_config::stepped_hispeed(self.config.play.hispeed, step * f64::from(direction)) else {
            return ControlEffect::None;
        };
        self.config.play.hispeed = next;
        ControlEffect::Speed
    }

    /// Put the scroll speed back where a pinned green number wants it, which is what a changed lane
    /// cover asks for: the window the notes cross has moved, so the speed has to move with it for
    /// the number the player reads to stay put (`LaneRenderer.java:212-215`).
    ///
    /// A run pinned with the lane fully covered has no travel time to hold — the window the notes
    /// cross is nothing — and there is no speed that produces it. That run keeps the speed it has
    /// rather than being thrown back to the shipped one, which is what solving for a green number of
    /// zero would otherwise do the moment the cover was touched.
    pub(crate) fn retarget_hispeed(&mut self, fixed: FixedSpeed) {
        if fixed.green <= 0.0 {
            return;
        }
        self.config.play.hispeed = hispeed_for_green(fixed.bpm, fixed.green, self.effective_cover()).clamp(HISPEED_MIN, HISPEED_MAX);
    }

    /// Write the whole configuration out.
    ///
    /// The live [`AccountSession`] — not the configuration — owns the credential while the app
    /// runs, so it is folded back in here on the way to disk. Every other value the settings
    /// screen edits is already in `config`.
    pub(crate) fn save_settings(&mut self) {
        self.config.network.ir_token = self.session.token().map(str::to_string);
        self.config.network.ir_login_id = self.session.login_id().map(str::to_string);
        save_config(&self.config, &self.settings_path);
    }

    /// Load a chosen UI font (TTF/OTF/TTC) live as the preferred family, and persist the path so it
    /// is reapplied next launch.
    ///
    /// The picker that chose it runs beside the frame loop, so this is called from the settings
    /// screen's own update once an answer has come back rather than from the key that asked.
    pub(crate) fn apply_font(&mut self, path: &Path) {
        match std::fs::read(path) {
            Ok(bytes) => match rbms_render::load_font(bytes) {
                Some(family) => {
                    rbms_render::set_ui_family(&family);
                    self.config.display.font_path = Some(path.to_string_lossy().to_string());
                    self.save_settings();
                    println!("font: {} ({family})", path.display());
                }
                None => notify(Level::Warn, format!("font load failed (no usable face): {}", path.display())),
            },
            Err(e) => notify(Level::Error, format!("font read failed ({}): {e}", path.display())),
        }
    }

    /// Revert the UI font to the bundled default.
    pub(crate) fn reset_font(&mut self) {
        rbms_render::reset_ui_family();
        self.config.display.font_path = None;
        self.save_settings();
    }

    pub(crate) fn offset_us(&self) -> i64 {
        self.config.judge.offset_ms as i64 * 1000
    }

    /// Rebuild the resolved skin from the loaded base config plus the live scratch-side, lift and
    /// five-key layout.
    ///
    /// The lift is the effective one: a height the player has set but switched off must not move the
    /// judgment line, and switching it back on must return the height rather than a default.
    pub(crate) fn rebuild_skin(&mut self) {
        let mut cfg = self.skin_cfg.clone();
        cfg.scratch_left = self.config.play.scratch_left;
        cfg.lift = self.effective_lift();
        cfg.five_key_layout = self.config.display.five_key_layout;
        self.result_palette = ResultPalette::from_skin(&cfg);
        self.skin = Skin::build(&cfg, self.mode, CW as f32, CH as f32);
    }

    /// Whether binding `code` to `row` (for `mode`) would collide with another action and so
    /// silently break it (controls are resolved before lanes in play, so a shared key would
    /// shadow the lane). Rebinding a row to its own current key is not a collision.
    pub(crate) fn binding_collides(&self, mode: Mode, row: &KcRow, code: KeyCode) -> bool {
        let taken_by_a_lane = |except: Option<usize>| self.keyconfig.lane_keys(mode).iter().any(|(k, l)| *k == code && Some(*l) != except);
        let taken_by_a_reverse = |except: Option<usize>| self.keyconfig.scratch_reverse_keys(mode).iter().any(|(k, l)| *k == code && Some(*l) != except);
        match row {
            KcRow::Lane(lane) => self.control_for(code).is_some() || taken_by_a_lane(Some(*lane)) || taken_by_a_reverse(None),
            KcRow::ScratchReverse(lane) => self.control_for(code).is_some() || taken_by_a_lane(None) || taken_by_a_reverse(Some(*lane)),
            KcRow::Control(action) => {
                taken_by_a_lane(None)
                    || taken_by_a_reverse(None)
                    || ControlAction::ALL.into_iter().any(|a| a != *action && self.keyconfig.control_key(a) == Some(code))
            }
            KcRow::ModeSelect
            | KcRow::PadDevice
            | KcRow::PadAnalogMode
            | KcRow::PadControl(_)
            | KcRow::PadLane(_)
            | KcRow::PadScratchReverse(_) => false,
        }
    }

    /// md5 of the currently focused chart, if a song (not a folder) is focused.
    pub(crate) fn focused_md5(&self) -> Option<String> {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => self.library.songs().get(*si).map(|e| e.md5.clone()),
            _ => None,
        }
    }

    /// Index into `songs` of the focused select row, if it is a chart (not a folder).
    pub(crate) fn focused_song_index(&self) -> Option<usize> {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => Some(*si),
            _ => None,
        }
    }

    /// Map a local score record into the renderer's record-row view (owned data, no borrow escapes).
    /// `trend` carries the EX delta versus the next-older play when the score graph is enabled.
    pub(crate) fn record_row_view(&self, r: &ScoreRecord, older: Option<&ScoreRecord>) -> RecordRowView {
        let (label, color) = clear_label_color(clear_type_from_id(r.clear));
        let trend = older.filter(|_| self.config.display.score_graph).map(|o| ex_delta_label(r.ex_score as i64 - o.ex_score as i64));
        let when = format!("{}{}", fmt_datetime(r.played_at), rule_version_mark(r.rule_version));
        RecordRowView { when, lamp: color, lamp_label: label, ex: r.ex_score, max_ex: r.max_ex, bp: r.counts[3] + r.counts[4] + r.counts[5], trend }
    }
}

/// One step of a lane shade, held inside the range its settings rows declare.
fn shaded(value: f32, delta: f32) -> f32 {
    (value + delta).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, Config, LaunchOptions};

    /// The reference tempo the green-number arithmetic is checked at.
    const TEST_BPM: f64 = 150.0;

    /// How far two travel times may sit apart and still count as the same one. A thousandth of a
    /// millisecond is far below anything a player could read off the HUD, and leaves room for the
    /// lane shades being stored at single precision while the arithmetic runs at double.
    const GREEN_TOLERANCE_MS: f64 = 1e-3;

    fn app() -> App {
        let dir = std::env::temp_dir().join(format!("rbms-app-input-tests-{}", std::process::id()));
        App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron"))
    }

    fn free(base_hispeed: f64) -> ControlContext {
        ControlContext { fine: false, base_hispeed, fixed: None }
    }

    /// The two directions have to be exact inverses, or a pinned green number would drift every
    /// time it was recomputed.
    #[test]
    fn the_green_number_and_the_speed_that_produces_it_are_inverses() {
        for bpm in [60.0, TEST_BPM, 222.0] {
            for hispeed in [0.5, 1.0, 3.75, 12.0] {
                for cover in [0.0, 0.25, 0.8] {
                    let green = green_for_hispeed(bpm, hispeed, cover);
                    let back = hispeed_for_green(bpm, green, cover);
                    assert!((back - hispeed).abs() < 1e-9, "bpm {bpm} hispeed {hispeed} cover {cover}: {green} came back as {back}");
                }
            }
        }
    }

    /// A chart with no tempo, or a run asking for a travel time of nothing, has no speed to compute:
    /// answering with the shipped default is what keeps a division by zero off the screen.
    #[test]
    fn a_speed_that_cannot_be_computed_falls_back_to_the_shipped_one() {
        assert_eq!(hispeed_for_green(0.0, 300.0, 0.0), rbms_config::DEFAULT_HISPEED);
        assert_eq!(hispeed_for_green(TEST_BPM, 0.0, 0.0), rbms_config::DEFAULT_HISPEED);
    }

    /// A run with no pinned tempo steps by the flat amount the row holds, which is what the speed
    /// has always done.
    #[test]
    fn an_unpinned_run_steps_the_speed_by_the_row_it_holds() {
        let mut app = app();
        app.shared.config.play.hispeed = 2.0;
        app.shared.config.play.hispeed_step = 0.25;
        assert_eq!(app.shared.apply_control(ControlAction::HiSpeedUp, &free(2.0)), ControlEffect::Speed);
        assert!((app.shared.config.play.hispeed - 2.25).abs() < 1e-9);
        app.shared.apply_control(ControlAction::HiSpeedDown, &free(2.0));
        assert!((app.shared.config.play.hispeed - 2.0).abs() < 1e-9);
    }

    /// A pinned run steps in multiples of the speed it started at, so one press is the same
    /// proportional change whatever tempo the chart is at (`LaneRenderer.java:238-240`).
    #[test]
    fn a_pinned_run_steps_the_speed_in_multiples_of_the_one_it_started_at() {
        let mut app = app();
        app.shared.config.play.hispeed = 4.0;
        app.shared.config.play.hispeed_step = 0.25;
        let ctx = ControlContext { fine: false, base_hispeed: 4.0, fixed: Some(FixedSpeed { bpm: TEST_BPM, green: 300.0 }) };
        app.shared.apply_control(ControlAction::HiSpeedUp, &ctx);
        assert!((app.shared.config.play.hispeed - 5.0).abs() < 1e-9, "a quarter of 4.0 is 1.0, got {}", app.shared.config.play.hispeed);
    }

    /// The bounds are exclusive in the reference (`LaneRenderer.java:243`): a step that would land
    /// on or past either end is not taken, rather than clamped onto the end.
    #[test]
    fn a_step_that_would_leave_the_range_is_not_taken_at_all() {
        let mut app = app();
        app.shared.config.play.hispeed_step = 1.0;
        app.shared.config.play.hispeed = rbms_config::HISPEED_MAX - 0.5;
        assert_eq!(app.shared.apply_control(ControlAction::HiSpeedUp, &free(1.0)), ControlEffect::None);
        assert_eq!(app.shared.config.play.hispeed, rbms_config::HISPEED_MAX - 0.5, "the speed stayed where it was");

        app.shared.config.play.hispeed = 0.5;
        assert_eq!(app.shared.apply_control(ControlAction::HiSpeedDown, &free(1.0)), ControlEffect::None);
        assert_eq!(app.shared.config.play.hispeed, 0.5);
    }

    /// The settings screen and the in-play key edit the same field, so they have to agree on where
    /// its ends are. One clamping onto the ceiling while the other stops short of it left a speed
    /// the settings row could reach and the key could then only move downwards from.
    #[test]
    fn the_settings_row_and_the_in_play_key_stop_at_the_same_ceiling() {
        for at in [rbms_config::HISPEED_MAX - 0.5, rbms_config::HISPEED_MAX - 0.01, rbms_config::HISPEED_MIN] {
            for delta in [1, -1] {
                let mut app = app();
                app.shared.config.play.hispeed_step = 1.0;
                app.shared.config.play.hispeed = at;
                let action = if delta > 0 { ControlAction::HiSpeedUp } else { ControlAction::HiSpeedDown };
                app.shared.apply_control(action, &free(1.0));
                let by_key = app.shared.config.play.hispeed;

                let mut config = app.shared.config.clone();
                config.play.hispeed = at;
                rbms_config::adjust(&mut config, rbms_config::SettingId::HiSpeed, delta);
                assert!((config.play.hispeed - by_key).abs() < 1e-9, "at {at} stepping {delta}: row left {} and key left {by_key}", config.play.hispeed);
            }
        }
    }

    /// A run pinned with the lane fully covered has no travel time to hold: there is no speed that
    /// produces one of nothing. Touching the cover then has to leave the speed the player chose
    /// alone rather than solve for it and land on the shipped one.
    #[test]
    fn a_fully_covered_pinned_run_keeps_the_speed_it_has() {
        let mut app = app();
        app.shared.config.play.hispeed = 4.5;
        app.shared.config.play.enable_cover = true;
        app.shared.config.play.cover = rbms_config::LANE_SHADE_MAX;
        app.shared.retarget_hispeed(FixedSpeed { bpm: TEST_BPM, green: green_for_hispeed(TEST_BPM, 4.5, rbms_config::LANE_SHADE_MAX) });
        assert!((app.shared.config.play.hispeed - 4.5).abs() < 1e-9, "a covered lane threw the speed away: {}", app.shared.config.play.hispeed);
    }

    /// The lane shades take the finer of their two steps while the modifier is down
    /// (`PlayConfig.java:87,91`), which is what makes a cover placeable to the pixel.
    #[test]
    fn the_modifier_switches_a_lane_shade_to_its_finer_step() {
        let mut app = app();
        app.shared.config.play.cover = 0.5;
        app.shared.apply_control(ControlAction::CoverUp, &ControlContext { fine: false, base_hispeed: 1.0, fixed: None });
        let coarse = app.shared.config.play.cover - 0.5;
        app.shared.config.play.cover = 0.5;
        app.shared.apply_control(ControlAction::CoverUp, &ControlContext { fine: true, base_hispeed: 1.0, fixed: None });
        let fine = app.shared.config.play.cover - 0.5;
        assert!((coarse - rbms_config::LANE_SHADE_STEP).abs() < 1e-6, "the coarse step is the shipped one");
        assert!((fine - app.shared.config.play.lanecover_step_fine).abs() < 1e-6, "the fine step is the row's");
        assert!(fine < coarse, "the modifier has to make the step smaller");
    }

    /// Every shade is held inside the range its row declares, at both ends.
    #[test]
    fn a_lane_shade_stops_at_both_ends_of_its_range() {
        let mut app = app();
        for _ in 0..1000 {
            app.shared.apply_control(ControlAction::HiddenUp, &free(1.0));
        }
        assert_eq!(app.shared.config.play.hidden, rbms_config::LANE_SHADE_MAX);
        for _ in 0..1000 {
            app.shared.apply_control(ControlAction::HiddenDown, &free(1.0));
        }
        assert_eq!(app.shared.config.play.hidden, rbms_config::LANE_SHADE_MIN);
    }

    /// Switching a shade off leaves the amount alone, so switching it back on returns the height the
    /// player had rather than a default (`PlayConfig.java:62,66`).
    #[test]
    fn switching_a_shade_off_hides_it_without_forgetting_it() {
        let mut app = app();
        app.shared.config.play.cover = 0.4;
        app.shared.config.play.hidden = 0.3;
        app.shared.config.play.lift = 0.2;
        app.shared.config.play.enable_cover = true;
        app.shared.config.play.enable_hidden = true;
        app.shared.config.play.enable_lift = true;
        assert_eq!((app.shared.effective_cover(), app.shared.effective_hidden(), app.shared.effective_lift()), (0.4, 0.3, 0.2));
        app.shared.config.play.enable_cover = false;
        app.shared.config.play.enable_hidden = false;
        app.shared.config.play.enable_lift = false;
        assert_eq!((app.shared.effective_cover(), app.shared.effective_hidden(), app.shared.effective_lift()), (0.0, 0.0, 0.0));
        assert_eq!((app.shared.config.play.cover, app.shared.config.play.hidden, app.shared.config.play.lift), (0.4, 0.3, 0.2), "the amounts are kept");
    }

    /// Moving the cover on a pinned run moves the speed with it, so the travel time the player reads
    /// stays where they put it (`LaneRenderer.java:212-215`).
    #[test]
    fn a_pinned_run_keeps_its_travel_time_when_the_cover_moves() {
        let mut app = app();
        app.shared.config.play.hispeed = 3.0;
        app.shared.config.play.enable_cover = true;
        app.shared.config.play.cover = 0.2;
        let green = green_for_hispeed(TEST_BPM, 3.0, app.shared.effective_cover());
        let ctx = ControlContext { fine: false, base_hispeed: 3.0, fixed: Some(FixedSpeed { bpm: TEST_BPM, green }) };

        assert_eq!(app.shared.apply_control(ControlAction::CoverUp, &ctx), ControlEffect::Shade);
        let fixed = ctx.fixed.expect("the run is pinned");
        app.shared.retarget_hispeed(fixed);
        let after = green_for_hispeed(TEST_BPM, app.shared.config.play.hispeed, app.shared.effective_cover());
        assert!((after - green).abs() < GREEN_TOLERANCE_MS, "the travel time moved from {green} to {after}");
        assert!(app.shared.config.play.hispeed < 3.0, "a taller cover means a slower scroll");
    }

    /// A cover switched off must not enter the arithmetic, or a run would speed up the moment the
    /// row was turned off.
    #[test]
    fn a_cover_that_is_switched_off_does_not_reach_the_pinned_speed() {
        let mut app = app();
        app.shared.config.play.cover = 0.5;
        app.shared.config.play.enable_cover = false;
        app.shared.retarget_hispeed(FixedSpeed { bpm: TEST_BPM, green: 300.0 });
        let uncovered = app.shared.config.play.hispeed;
        app.shared.config.play.enable_cover = true;
        app.shared.retarget_hispeed(FixedSpeed { bpm: TEST_BPM, green: 300.0 });
        assert!(app.shared.config.play.hispeed < uncovered, "switching the cover on has to change the speed it asks for");
    }

    /// The lift moves the judgment line, so the resolved skin has to follow the switch as well as
    /// the amount.
    #[test]
    fn the_resolved_skin_follows_the_lift_switch() {
        let mut app = app();
        app.shared.config.play.lift = 0.5;
        app.shared.config.play.enable_lift = false;
        app.shared.rebuild_skin();
        let flat = app.shared.skin.judge_y;
        app.shared.config.play.enable_lift = true;
        app.shared.rebuild_skin();
        assert!(app.shared.skin.judge_y < flat, "a lift that is on raises the judgment line");
    }

    /// The five-key layout is a settings row, so the resolved skin has to be built with it.
    #[test]
    fn the_resolved_skin_follows_the_five_key_layout_row() {
        let mut app = app();
        app.shared.mode = rbms_model::Mode::BEAT_5K;
        app.shared.config.display.five_key_layout = false;
        app.shared.rebuild_skin();
        let stretched = app.shared.skin.w[0];
        app.shared.config.display.five_key_layout = true;
        app.shared.rebuild_skin();
        assert!(app.shared.skin.w[0] < stretched, "the row does not reach the resolved skin");
    }
}

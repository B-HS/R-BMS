//! How long a skinned scene lasts: when it starts taking input, when it starts to fade, and when it
//! is over.
//!
//! A skin's header gives a scene three times in milliseconds -- `input`, `scene` and `fadeout` --
//! and the reference's screens all read them the same way (`MusicDecide.render`,
//! `MusicResult.render`, `MusicSelector.render`): once the scene clock has passed `input` the
//! `STARTINPUT` timer goes on and the screen starts listening, once it has passed `scene` the
//! `FADEOUT` timer goes on, and once that timer has run for longer than `fadeout` the scene is over.
//! A key can switch `FADEOUT` on early; nothing switches it off.
//!
//! Every comparison is the reference's own: strictly greater, in whole milliseconds. So a scene
//! with no times at all -- a skin that states none, or a screen drawn without a skin -- is over a
//! couple of milliseconds after it began rather than at the instant it did, which is as close to at
//! once as the reference comes.
//!
//! The two timers are the scene's ordinary skin timers, so what a skin animates on them and what
//! the screen decides from them cannot drift apart.

use std::time::Instant;

use rbms_skin::loader::LoadedSkin;
use rbms_skin::timer::{MICROS_PER_MILLI, TimerState, timer_id};

use crate::AppShared;

/// The three times a skin's header gives a scene, in milliseconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SceneTimes {
    /// How long the scene runs before it takes input.
    pub(crate) input_ms: i64,
    /// How long the scene runs before it fades out on its own.
    pub(crate) scene_ms: i64,
    /// How long the fade lasts before the scene is over.
    pub(crate) fadeout_ms: i64,
}

impl SceneTimes {
    /// The times `skin` states, or none at all for a screen with no skin.
    pub(crate) fn of_skin(skin: Option<&LoadedSkin>) -> SceneTimes {
        skin.map_or_else(SceneTimes::default, |skin| SceneTimes {
            input_ms: i64::from(skin.def.input),
            scene_ms: i64::from(skin.def.scene),
            fadeout_ms: i64::from(skin.def.fadeout),
        })
    }
}

/// Where a scene stands after a frame of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScenePhase {
    /// Running, and not yet listening.
    Opening,
    /// Running and taking input.
    Listening,
    /// Fading out; the keys that end a scene no longer do anything.
    FadingOut,
    /// The fade has run its length: the screen leaves.
    Over,
}

/// Move a scene on to `now_us` on its own clock, switching `STARTINPUT` and `FADEOUT` on as the
/// times in `times` are passed, and answer where that leaves it.
pub(crate) fn advance(times: SceneTimes, timers: &mut TimerState, now_us: i64) -> ScenePhase {
    let now_ms = now_us / MICROS_PER_MILLI;
    if now_ms > times.input_ms {
        timers.switch(timer_id::STARTINPUT, true, now_us);
    }
    if timers.is_on(timer_id::FADEOUT) {
        let fading_ms = (now_us - timers.value_us(timer_id::FADEOUT)) / MICROS_PER_MILLI;
        return if fading_ms > times.fadeout_ms { ScenePhase::Over } else { ScenePhase::FadingOut };
    }
    if now_ms > times.scene_ms {
        timers.set_on(timer_id::FADEOUT, now_us);
        return ScenePhase::FadingOut;
    }
    if timers.is_on(timer_id::STARTINPUT) { ScenePhase::Listening } else { ScenePhase::Opening }
}

/// Whether the scene acts on a key right now: it has started listening and has not begun to fade.
pub(crate) fn takes_input(timers: &TimerState) -> bool {
    timers.is_on(timer_id::STARTINPUT) && !timers.is_on(timer_id::FADEOUT)
}

/// Start the fade at `now_us`, which is what a key that ends the scene does.
pub(crate) fn begin_fadeout(timers: &mut TimerState, now_us: i64) {
    timers.set_on(timer_id::FADEOUT, now_us);
}

impl AppShared {
    /// Begin the running scene's time over, at the moment its skin can first be drawn: every timer
    /// off and the scene clock at zero.
    ///
    /// The reference reads a screen's skin before the screen's scene begins, and stands still while
    /// it does. Here the skin's files are read off the frame loop, so the screen is up for a few
    /// frames before its skin is; calling this on the frame the skin arrives is what makes the
    /// skin's first frame the scene's first moment. The skin itself is left as it was read.
    pub(crate) fn start_skin_scene_clock(&mut self) {
        self.skin_timers.clear();
        self.scene_started = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The times of the scene the tests below run: half a second before input, three seconds before
    /// the fade, and a second of fade.
    const TIMES: SceneTimes = SceneTimes { input_ms: 500, scene_ms: 3000, fadeout_ms: 1000 };

    /// One microsecond short of the next whole millisecond.
    const ALMOST_A_MILLI_US: i64 = MICROS_PER_MILLI - 1;

    fn at_ms(millis: i64) -> i64 {
        millis * MICROS_PER_MILLI
    }

    #[test]
    fn a_scene_starts_listening_only_once_its_input_time_has_passed() {
        let mut timers = TimerState::new();
        assert_eq!(advance(TIMES, &mut timers, 0), ScenePhase::Opening);
        assert_eq!(advance(TIMES, &mut timers, at_ms(TIMES.input_ms) + ALMOST_A_MILLI_US), ScenePhase::Opening, "the input time itself is not past it");
        assert!(!takes_input(&timers));

        let listening_from = at_ms(TIMES.input_ms + 1);
        assert_eq!(advance(TIMES, &mut timers, listening_from), ScenePhase::Listening);
        assert_eq!(timers.value_us(timer_id::STARTINPUT), listening_from);
        assert!(takes_input(&timers));

        advance(TIMES, &mut timers, at_ms(TIMES.input_ms + 200));
        assert_eq!(timers.value_us(timer_id::STARTINPUT), listening_from, "a timer that is on was started again");
    }

    #[test]
    fn a_scene_fades_out_on_its_own_once_its_length_has_passed() {
        let mut timers = TimerState::new();
        assert_eq!(advance(TIMES, &mut timers, at_ms(TIMES.scene_ms) + ALMOST_A_MILLI_US), ScenePhase::Listening, "the scene time itself is not past it");
        assert!(!timers.is_on(timer_id::FADEOUT));

        let fading_from = at_ms(TIMES.scene_ms + 1);
        assert_eq!(advance(TIMES, &mut timers, fading_from), ScenePhase::FadingOut);
        assert_eq!(timers.value_us(timer_id::FADEOUT), fading_from);
        assert!(!takes_input(&timers), "a fading scene still takes the keys that end it");
    }

    #[test]
    fn a_scene_is_over_once_its_fade_has_run_longer_than_the_fade_time() {
        let mut timers = TimerState::new();
        let fading_from = at_ms(TIMES.scene_ms + 1);
        advance(TIMES, &mut timers, fading_from);

        assert_eq!(
            advance(TIMES, &mut timers, fading_from + at_ms(TIMES.fadeout_ms) + ALMOST_A_MILLI_US),
            ScenePhase::FadingOut,
            "the fade time itself is not past it"
        );
        assert_eq!(advance(TIMES, &mut timers, fading_from + at_ms(TIMES.fadeout_ms + 1)), ScenePhase::Over);
        assert_eq!(timers.value_us(timer_id::FADEOUT), fading_from, "the fade was started again while it ran");
    }

    #[test]
    fn a_fade_begun_early_is_measured_from_when_it_was_begun() {
        let mut timers = TimerState::new();
        let skipped_at = at_ms(TIMES.input_ms + 100);
        advance(TIMES, &mut timers, skipped_at);
        begin_fadeout(&mut timers, skipped_at);

        assert_eq!(advance(TIMES, &mut timers, skipped_at + at_ms(TIMES.fadeout_ms)), ScenePhase::FadingOut);
        assert_eq!(advance(TIMES, &mut timers, skipped_at + at_ms(TIMES.fadeout_ms + 1)), ScenePhase::Over, "the fade waited for the scene's own length");
    }

    /// A scene with no times is the one a screen without a skin runs: it listens and fades on the
    /// first millisecond its clock shows, and is over on the next.
    #[test]
    fn a_scene_with_no_times_is_over_as_soon_as_its_clock_moves() {
        let mut timers = TimerState::new();
        let none = SceneTimes::of_skin(None);
        assert_eq!(none, SceneTimes::default());

        assert_eq!(advance(none, &mut timers, 0), ScenePhase::Opening, "nothing is past zero at zero");
        assert_eq!(advance(none, &mut timers, at_ms(1)), ScenePhase::FadingOut);
        assert!(timers.is_on(timer_id::STARTINPUT) && timers.is_on(timer_id::FADEOUT));
        assert_eq!(advance(none, &mut timers, at_ms(2)), ScenePhase::Over);
    }

    #[test]
    fn restarting_the_scene_clock_switches_every_timer_off_and_puts_the_clock_at_zero() {
        const AGED: std::time::Duration = std::time::Duration::from_secs(5);
        let mut app = crate::stage::render_tests::app();
        app.shared.age_skin_scene(AGED);
        let aged_us = app.shared.skin_now_us();
        assert_eq!(advance(TIMES, &mut app.shared.skin_timers, aged_us), ScenePhase::FadingOut);
        assert!(app.shared.skin_timers.is_on(timer_id::STARTINPUT) && app.shared.skin_timers.is_on(timer_id::FADEOUT));

        app.shared.start_skin_scene_clock();
        assert!(!app.shared.skin_timers.is_on(timer_id::STARTINPUT) && !app.shared.skin_timers.is_on(timer_id::FADEOUT));
        let restarted_us = app.shared.skin_now_us();
        assert!(restarted_us < aged_us, "the clock was not put back: {restarted_us} us");
    }
}

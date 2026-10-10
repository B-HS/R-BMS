//! The states a play screen goes through when a skin draws it: loading, ready, playing, and one of
//! the two ways a run closes.
//!
//! This is the reference's `BMSPlayer.render` (`BMSPlayer.java:458-789`) with everything that is not
//! a state or a time taken out of it. A play skin states how long each state lasts -- `loadend`,
//! `playstart`, `close`, `finishmargin`, and the `input` and `fadeout` every skin has -- and the
//! screen moves on those times alone:
//!
//! | State | Leaves when | For |
//! | --- | --- | --- |
//! | `PRELOAD` | everything is loaded, `loadend` has passed, and START and SELECT have been up for a second | `READY` |
//! | `READY` | the `READY` timer has run for longer than `playstart` | `PLAY` |
//! | `PLAY` | the gauge empties | `FAILED` |
//! | `PLAY` | the chart's playing time is over | `FINISHED` |
//! | `FAILED` | the `FAILED` timer has run for longer than `close` | the result |
//! | `FINISHED` | `finishmargin` after the music ended the fade begins, and `fadeout` after that | the result |
//!
//! Every comparison is the reference's own: strictly greater, in whole milliseconds for a timer and
//! in microseconds for the scene clock. That includes what looks odd and is kept. The second START
//! and SELECT have to have been up for is counted from the scene's own beginning when neither was
//! ever touched, so no chart is ready sooner than a second after its screen came up. And a run that
//! fails and runs out on the same frame is finished rather than failed, with both timers on.
//!
//! The scene decides nothing about the run. Whether the files are in, whether the gauge has emptied
//! and where the chart stands are handed to it as [`SceneFacts`], and what it decides comes back as
//! a [`SceneStep`] for the screen to act on: the song clock, the sounds and the result are the
//! screen's. What the scene does itself is tell the timer driver what happened, so the timers a skin
//! animates on are switched at the moment the state changes and by nothing else.
//!
//! It holds no clock of its own. The time is handed in with the facts, so a frame of it is the same
//! whether the application or a test is the one counting.

use rbms_skin::loader::LoadedSkin;
use rbms_skin::timer::{MICROS_PER_MILLI, TimerId, TimerState, timer_id};

use crate::skin_host::play::PlayPhase;
use crate::skin_host::play_timers::{PlayTimerDriver, PlayingFrame, SceneEvent};

#[cfg(test)]
mod tests;

/// How long START and SELECT have to have been up before a loaded chart is ready, in microseconds
/// (`BMSPlayer.java:500`).
const START_RELEASE_WAIT_US: i64 = 1_000_000;

/// How long after its last note a chart is kept playing, in milliseconds (`BMSPlayer.TIME_MARGIN`).
pub(crate) const TIME_MARGIN_MS: i64 = rbms_play::PLAY_TIME_MARGIN_MS;

/// The times a skin gives a play screen, in milliseconds.
///
/// The four that are the play screen's own are read only from a skin that places its note field,
/// which is the reference's rule and is applied where the skin is loaded
/// ([`rbms_skin::loader::PlayTimings`]). A screen with no skin has none at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlayTimes {
    /// How long the scene runs before `STARTINPUT` goes on.
    pub(crate) input_ms: i64,
    /// How long loading takes at the least.
    pub(crate) loadend_ms: i64,
    /// How long the chart stands ready before it plays.
    pub(crate) playstart_ms: i64,
    /// How long a failed run stays on screen.
    pub(crate) close_ms: i64,
    /// How long after the music ended the fade begins.
    pub(crate) finishmargin_ms: i64,
    /// How long the fade lasts.
    pub(crate) fadeout_ms: i64,
}

impl PlayTimes {
    /// The times `skin` states, or none at all for a screen with no skin.
    pub(crate) fn of_skin(skin: Option<&LoadedSkin>) -> PlayTimes {
        skin.map_or_else(PlayTimes::default, |skin| PlayTimes {
            input_ms: i64::from(skin.def.input),
            loadend_ms: i64::from(skin.play.loadend),
            playstart_ms: i64::from(skin.play.playstart),
            close_ms: i64::from(skin.play.close),
            finishmargin_ms: i64::from(skin.play.finishmargin),
            fadeout_ms: i64::from(skin.def.fadeout),
        })
    }
}

/// Where a run stands against the time the reference keeps its chart playing for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RunEnd {
    /// The playing time is over (`playtime < ptime`).
    pub(crate) ended: bool,
    /// The last note has gone by (`playtime - TIME_MARGIN < ptime`).
    pub(crate) last_note_passed: bool,
}

impl RunEnd {
    /// Where a run that has played for `played_ms` stands against a playing time of `play_time_ms`
    /// (`BMSPlayer.java:679-690`).
    pub(crate) fn of(play_time_ms: i64, played_ms: i64) -> RunEnd {
        RunEnd { ended: play_time_ms < played_ms, last_note_passed: play_time_ms - TIME_MARGIN_MS < played_ms }
    }
}

/// What the screen knows of one frame, which is all the scene decides from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SceneFacts {
    /// The frame's time on the scene clock, in microseconds.
    pub(crate) now_us: i64,
    /// The times of the skin as it stands this frame.
    pub(crate) times: PlayTimes,
    /// Everything the chart named is in, and so is the skin (`BMSResource.mediaLoadFinished`; the
    /// reference has read its skin before the screen exists).
    pub(crate) loaded: bool,
    /// Whether START and SELECT are down.
    pub(crate) start: bool,
    pub(crate) select: bool,
    /// How far into the chart the run starts, in microseconds: the start of a practice slice.
    pub(crate) start_offset_us: i64,
    /// The frame of the run, for the frames the chart is playing on.
    pub(crate) playing: PlayingFrame,
    /// The gauge has emptied with nothing to rescue the run.
    pub(crate) failed: bool,
    /// Where the run stands against its playing time.
    pub(crate) end: RunEnd,
    /// Every note of the chart has been judged (`BMSPlayer.isNoteEnd`).
    pub(crate) notes_done: bool,
    /// A failed run may be started again at once: a chart played by hand, outside a course
    /// (`BMSPlayer.java:696-697`).
    pub(crate) may_retry: bool,
}

/// How a run that is over leaves the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leave {
    /// On to the result.
    Result,
    /// The same chart again, without waiting for the failure to close: laid out as it was
    /// (`same_layout`, SELECT) or afresh on the same options (START).
    Retry { same_layout: bool },
}

/// What one frame of the scene asks the screen to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SceneStep {
    /// Nothing changed.
    Stay,
    /// The chart is ready: the ready cue is due.
    Ready,
    /// The chart starts playing on this frame: the song clock starts here.
    Started,
    /// The run failed on this frame: what is sounding stops, and the stop cue is due.
    Failed,
    /// The chart ran out, or the player ended a run with nothing left to hit. `failed_too` is the
    /// frame the gauge emptied on as well, which the reference still closes as a finished run.
    Finished { failed_too: bool },
    /// The scene is over.
    Leave(Leave),
}

/// The play screen's scene: the state it is in, and when START or SELECT was last down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayScene {
    phase: PlayPhase,
    /// The scene time START or SELECT was last seen down at (`startpressedtime`), which starts at
    /// the scene's own beginning.
    start_pressed_us: i64,
}

impl PlayScene {
    /// The scene of a chart whose files may still be on their way: it begins loading.
    pub(crate) fn loading() -> PlayScene {
        PlayScene { phase: PlayPhase::Preload, start_pressed_us: 0 }
    }

    /// The scene of a run that has nothing to load and nothing to wait for: it begins ready. This is
    /// this player's own rule for a practice slice, which is cut from a chart that is already in and
    /// is started again every time the slice is.
    pub(crate) fn ready() -> PlayScene {
        PlayScene { phase: PlayPhase::Ready, start_pressed_us: 0 }
    }

    /// The state the scene is in.
    pub(crate) fn phase(&self) -> PlayPhase {
        self.phase
    }

    /// Whether the chart is playing: the song clock runs and the run takes input.
    pub(crate) fn is_playing(&self) -> bool {
        self.phase == PlayPhase::Play
    }

    /// Whether the run is closing, one way or the other.
    pub(crate) fn is_closing(&self) -> bool {
        matches!(self.phase, PlayPhase::Failed | PlayPhase::Finished)
    }

    /// End a run that has nothing left to hit, which is what the reference's `stopPlay` does with
    /// one (`BMSPlayer.java:953-960`): the run is finished and its fade begins at once, without the
    /// margin a chart that ran out waits for. `true` when this is what ended the run.
    pub(crate) fn stop_finished(&mut self, driver: &mut PlayTimerDriver, timers: &mut TimerState, now_us: i64) -> bool {
        if self.phase != PlayPhase::Play && self.phase != PlayPhase::Finished {
            return false;
        }
        let ended = self.phase == PlayPhase::Play;
        self.phase = PlayPhase::Finished;
        driver.apply(timers, now_us, SceneEvent::Fadeout);
        ended
    }

    /// One frame of the scene, in the order the reference runs its own.
    pub(crate) fn step(&mut self, driver: &mut PlayTimerDriver, timers: &mut TimerState, facts: &SceneFacts) -> SceneStep {
        let now_us = facts.now_us;
        let times = facts.times;
        if now_us > times.input_ms * MICROS_PER_MILLI {
            driver.apply(timers, now_us, SceneEvent::InputOpen);
        }
        if facts.start || facts.select {
            self.start_pressed_us = now_us;
        }
        match self.phase {
            PlayPhase::Preload => {
                driver.apply(timers, now_us, SceneEvent::Preview { pressed: facts.start || facts.select });
                let ready = facts.loaded && now_us > times.loadend_ms * MICROS_PER_MILLI && now_us - self.start_pressed_us > START_RELEASE_WAIT_US;
                let step = if ready { self.make_ready(driver, timers, now_us) } else { SceneStep::Stay };
                driver.apply(timers, now_us, SceneEvent::Standby);
                step
            }
            PlayPhase::Ready if !timers.is_on(timer_id::READY) => self.make_ready(driver, timers, now_us),
            PlayPhase::Ready => {
                if elapsed_ms(timers, timer_id::READY, now_us) <= times.playstart_ms {
                    return SceneStep::Stay;
                }
                self.phase = PlayPhase::Play;
                driver.apply(timers, now_us, SceneEvent::Started { start_offset_us: facts.start_offset_us });
                SceneStep::Started
            }
            PlayPhase::Play => self.play(driver, timers, facts),
            PlayPhase::Failed => {
                driver.apply(timers, now_us, SceneEvent::JudgeStopped);
                if facts.may_retry && (facts.start ^ facts.select) {
                    return SceneStep::Leave(Leave::Retry { same_layout: !facts.start });
                }
                if elapsed_ms(timers, timer_id::FAILED, now_us) > times.close_ms { SceneStep::Leave(Leave::Result) } else { SceneStep::Stay }
            }
            PlayPhase::Finished => {
                driver.apply(timers, now_us, SceneEvent::JudgeStopped);
                if elapsed_ms(timers, timer_id::MUSIC_END, now_us) > times.finishmargin_ms {
                    driver.apply(timers, now_us, SceneEvent::Fadeout);
                }
                if elapsed_ms(timers, timer_id::FADEOUT, now_us) > times.fadeout_ms { SceneStep::Leave(Leave::Result) } else { SceneStep::Stay }
            }
        }
    }

    /// The chart becomes ready (`BMSPlayer.java:501-516`).
    fn make_ready(&mut self, driver: &mut PlayTimerDriver, timers: &mut TimerState, now_us: i64) -> SceneStep {
        self.phase = PlayPhase::Ready;
        driver.apply(timers, now_us, SceneEvent::Ready);
        SceneStep::Ready
    }

    /// One frame of the chart playing (`BMSPlayer.java:620-691`): the run's own frame, then the
    /// failure, then the end of the playing time, which wins over a failure on the same frame. A run
    /// with nothing left to hit also ends when START or SELECT is pressed
    /// (`ControlInputProcessor.java:196-200`).
    fn play(&mut self, driver: &mut PlayTimerDriver, timers: &mut TimerState, facts: &SceneFacts) -> SceneStep {
        let now_us = facts.now_us;
        driver.apply(timers, now_us, SceneEvent::Playing(facts.playing));
        if facts.failed {
            self.phase = PlayPhase::Failed;
            driver.apply(timers, now_us, SceneEvent::Failed);
        }
        if facts.end.ended {
            self.phase = PlayPhase::Finished;
            driver.apply(timers, now_us, SceneEvent::MusicEnded);
            return SceneStep::Finished { failed_too: facts.failed };
        }
        if facts.end.last_note_passed {
            driver.apply(timers, now_us, SceneEvent::LastNotePassed);
        }
        if facts.failed {
            return SceneStep::Failed;
        }
        if facts.notes_done && (facts.start || facts.select) && self.stop_finished(driver, timers, now_us) {
            return SceneStep::Finished { failed_too: false };
        }
        SceneStep::Stay
    }
}

/// How long a timer has been on, in whole milliseconds, and nought for one that is off
/// (`TimerManager.getNowTime(id)`).
pub(crate) fn elapsed_ms(timers: &TimerState, timer: TimerId, now_us: i64) -> i64 {
    if timers.is_on(timer) { (now_us - timers.value_us(timer)) / MICROS_PER_MILLI } else { 0 }
}

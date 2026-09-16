//! The application's [`SoundSink`]: one run's keysounds into the shared output stream.
//!
//! `rbms-play` knows nothing about cpal, buses or sample ids — it says which keysound it wants and
//! whether the sound is booked ahead or wanted at once. This adapter is the only place that maps
//! those onto [`AudioEngine`]: [`PlaySource`] onto an output bus, chart `#WAVxx` indices onto the
//! play id namespace, and the session's song clock onto the engine's own axis.

use rbms_audio::{AudioEngine, Bus, IdNamespace};
use rbms_play::{PlaySource, SoundRequest, SoundSink, SoundTime};

use crate::IMMEDIATE_KEYSOUND_AT_US;
use crate::practice::PracticeClock;

/// Output bus a session's keysound belongs on: accompaniment and note sounds carry their own
/// levels so the player can balance them against each other.
pub(crate) fn bus_for(source: PlaySource) -> Bus {
    match source {
        PlaySource::Bgm => Bus::Bg,
        PlaySource::Key => Bus::Key,
    }
}

/// Sample id a chart keysound occupies in the shared bank. Charts live in their own namespace so
/// the song-select preview can keep its samples loaded across a chart load.
pub(crate) fn play_sample_id(wav: u32) -> u32 {
    IdNamespace::PLAY.base + wav
}

/// Routes one run's keysounds into the shared output stream, rebasing the session's song clock by
/// the anchor captured when play began.
pub(crate) struct PlayAudioSink<'a> {
    engine: &'a mut AudioEngine,
    anchor_us: i64,
    clock: PracticeClock,
}

impl<'a> PlayAudioSink<'a> {
    pub(crate) fn new(engine: &'a mut AudioEngine, anchor_us: i64, clock: PracticeClock) -> Self {
        PlayAudioSink { engine, anchor_us, clock }
    }
}

/// The engine time a request sounds at. A booked sound moves with the anchor; one wanted at once
/// carries the engine's own "next callback" marker instead, so the judge offset can never delay it.
pub(crate) fn sink_time_us(at: SoundTime, anchor_us: i64, clock: PracticeClock) -> i64 {
    match at {
        SoundTime::Immediate => IMMEDIATE_KEYSOUND_AT_US,
        SoundTime::Scheduled(song_us) => anchor_us.saturating_add(clock.engine_time_us(song_us)),
    }
}

impl SoundSink for PlayAudioSink<'_> {
    fn play(&mut self, sound: SoundRequest) {
        let at_us = sink_time_us(sound.at, self.anchor_us, self.clock);
        self.engine.play_on(bus_for(sound.source), play_sample_id(sound.wav), sound.gain, sound.pan, self.clock.pitch(sound.pitch), at_us);
    }

    fn stop_all(&mut self) {
        self.engine.clear_namespace(IdNamespace::PLAY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accompaniment_and_note_sounds_take_their_own_buses() {
        assert_eq!(bus_for(PlaySource::Bgm), Bus::Bg);
        assert_eq!(bus_for(PlaySource::Key), Bus::Key);
    }

    #[test]
    fn chart_keysounds_stay_inside_the_play_namespace() {
        for wav in [0u32, 1, 1295] {
            let id = play_sample_id(wav);
            assert!(IdNamespace::PLAY.contains(id), "wav {wav} must map into the play namespace");
            assert!(!IdNamespace::PREVIEW.contains(id), "wav {wav} must not collide with a preview sample");
        }
    }

    #[test]
    fn a_scheduled_sound_is_rebased_on_the_play_anchor() {
        assert_eq!(sink_time_us(SoundTime::Scheduled(1_500_000), 40_000_000, PracticeClock::normal()), 41_500_000);
        assert_eq!(sink_time_us(SoundTime::Scheduled(0), 40_000_000, PracticeClock::normal()), 40_000_000, "song start sounds at the anchor");
    }

    #[test]
    fn an_immediate_sound_ignores_the_anchor() {
        assert_eq!(sink_time_us(SoundTime::Immediate, 40_000_000, PracticeClock::normal()), IMMEDIATE_KEYSOUND_AT_US);
        assert_eq!(sink_time_us(SoundTime::Immediate, 0, PracticeClock::normal()), IMMEDIATE_KEYSOUND_AT_US);
    }

    #[test]
    fn a_practice_schedule_is_converted_back_to_engine_time() {
        let clock = PracticeClock::new(30_000_000, 200);
        assert_eq!(sink_time_us(SoundTime::Scheduled(30_000_000), 40_000_000, clock), 40_000_000);
        assert_eq!(sink_time_us(SoundTime::Scheduled(50_000_000), 40_000_000, clock), 50_000_000);
    }

    #[test]
    fn practice_pitch_scales_but_an_immediate_marker_stays_fixed() {
        let clock = PracticeClock::new(30_000_000, 50);
        assert_eq!(clock.pitch(1.5), 0.75);
        assert_eq!(sink_time_us(SoundTime::Immediate, 40_000_000, clock), IMMEDIATE_KEYSOUND_AT_US);
    }
}

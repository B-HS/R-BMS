//! System sounds: the sound set loaded from the configured folder and the events that play it.
//!
//! The reference implementation names twenty-two effect stems and looks each one up inside a sound
//! set directory (`SystemSoundManager.java`). rbms keeps the same stems and the same names so an
//! existing sound set drops in unchanged, but resolves them out of a single configured folder
//! rather than scanning a root for sets, and accepts the same container list the rest of the player
//! accepts rather than `.wav` alone.
//!
//! Nothing here is required: a slot whose file is absent stays silent, and a set with no folder at
//! all is silent throughout, which is exactly how the player behaved before this module existed.
//! Guide sounds are gated separately and default to off, mirroring the reference implementation's
//! `isGuideSE` switch (`BMSPlayer.java:398-410`).
//!
//! Playback goes through the System bus so the system-sound volume is the one that governs it. A cue
//! is played once ([`SystemSoundSet::play`]) or repeated until it is stopped
//! ([`SystemSoundSet::play_loop`]), and can be cut short ([`SystemSoundSet::stop`]), the way the
//! reference plays the result cue with `isLoopResultSound` and silences it when the screen closes
//! (`MainState.play(sound, loop)`, `MainState.stop(sound)`).
//!
//! # The select music
//!
//! The reference plays the `select` sound as the song browser's music, looped from the moment the
//! browser is up until it is left, and turns it down to nothing while a song preview sounds and back
//! up afterwards (`PreviewMusicProcessor.PreviewThread.run`: the music is played at the system volume
//! with `loop = true`, faded to 0 over eleven 15 ms steps when a preview takes over, and set back to
//! the system volume when the preview is stopped, with the loop having kept running underneath).
//! [`SelectBgm`] holds that, and [`AppShared::drive_select_bgm`] puts it on the mixer: the music is a
//! looped effect whose level is moved (`AudioEngine::set_effect_level`) instead of being stopped, so
//! it keeps its place under a preview.
//!
//! rbms has always played `select` once as the cue for starting a chart, and the set it ships
//! carries a half-second cue under that name. A sound that short is not music, so [`SelectBgm`] only
//! runs when the file is at least [`SELECT_BGM_MIN_DURATION_US`] long, and a set whose `select` is
//! that long is music rather than a cue: it is not also played once when a chart is started. A set
//! with a short `select` behaves exactly as before.
//!
//! Which set is read is the player's own setting (`audio.sound_folder`), not the skin's: the
//! reference chooses its sound set and BGM set from `soundpath` and `bgmpath`, separately from the
//! skin (`SystemSoundManager.shuffle`), so a skin pack's `Sound` folder is not read unless the player
//! points the setting at it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rbms_audio::{AudioEngine, Bus, DecodedAudio, IdNamespace};
use rbms_judge::Judge;

use crate::notify::{Level, notify};
use crate::{AppShared, resolve_file};

/// Sample ids reserved for system sounds, kept clear of chart keysounds ([`IdNamespace::PLAY`]) and
/// of song-select previews ([`IdNamespace::PREVIEW`]) so one engine can hold all three at once.
pub(crate) const SYSTEM_SOUND_NAMESPACE: IdNamespace = IdNamespace { base: 0x0090_0000, len: 0x0000_0100 };

/// How many distinct sounds a set holds.
pub(crate) const SYSTEM_SOUND_COUNT: usize = 22;

/// Container preference when a stem resolves to more than one file, matching the order the loader
/// uses for keysounds except that `.wav` leads: the reference sets ship `.wav`.
const SYSTEM_SOUND_EXTENSIONS: [&str; 4] = ["wav", "ogg", "flac", "mp3"];

/// System sounds are mono cues with no stereo placement and no pitch shift.
const SYSTEM_SOUND_PAN: f32 = 0.0;
const SYSTEM_SOUND_PITCH: f32 = 1.0;

/// Play at the next callback rather than at a booked position: these are responses to input.
const SYSTEM_SOUND_AT_US: i64 = 0;

const SYSTEM_SOUND_GAIN_MIN: f32 = 0.0;
const SYSTEM_SOUND_GAIN_MAX: f32 = 1.0;

/// The gain a cue is raised at. Loudness is the System bus's job — the settings screen already
/// pushes `audio.system` to it — so a cue is played at unity and left to the bus.
pub(crate) const SYSTEM_SOUND_GAIN: f32 = 1.0;

const MICROS_PER_SECOND: i64 = 1_000_000;
const MILLIS_PER_SECOND: f32 = 1000.0;

/// The shortest `select` sound that is played as music. The reference's is a song-length loop; a
/// cue for starting a chart is a fraction of a second.
pub(crate) const SELECT_BGM_MIN_DURATION_US: i64 = 3 * MICROS_PER_SECOND;

/// The level the select music is held at under a preview, and the one it plays at otherwise.
const SELECT_BGM_HELD_LEVEL: f32 = 0.0;
const SELECT_BGM_FULL_LEVEL: f32 = 1.0;

/// One slot of a sound set: either nothing was found for the stem, or a decoded clip is held ready
/// to be handed to a mixer bank. The set keeps its own copy so a stream that is reopened — which
/// empties the bank — can be filled again without touching the disk.
enum Slot {
    Missing,
    Decoded(DecodedAudio),
}

/// Every sound the player can raise, in the reference implementation's `SoundType` order
/// (`SystemSoundManager.java:129-151`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SystemSound {
    Scratch,
    FolderOpen,
    FolderClose,
    OptionChange,
    OptionOpen,
    OptionClose,
    PlayReady,
    PlayStop,
    ResultClear,
    ResultFail,
    ResultClose,
    CourseClear,
    CourseFail,
    CourseClose,
    GuidePg,
    GuideGr,
    GuideGd,
    GuideBd,
    GuidePr,
    GuideMs,
    Select,
    Decide,
}

impl SystemSound {
    /// Every sound in slot order, for callers that load or iterate a whole set.
    pub(crate) const ALL: [SystemSound; SYSTEM_SOUND_COUNT] = [
        SystemSound::Scratch,
        SystemSound::FolderOpen,
        SystemSound::FolderClose,
        SystemSound::OptionChange,
        SystemSound::OptionOpen,
        SystemSound::OptionClose,
        SystemSound::PlayReady,
        SystemSound::PlayStop,
        SystemSound::ResultClear,
        SystemSound::ResultFail,
        SystemSound::ResultClose,
        SystemSound::CourseClear,
        SystemSound::CourseFail,
        SystemSound::CourseClose,
        SystemSound::GuidePg,
        SystemSound::GuideGr,
        SystemSound::GuideGd,
        SystemSound::GuideBd,
        SystemSound::GuidePr,
        SystemSound::GuideMs,
        SystemSound::Select,
        SystemSound::Decide,
    ];

    /// The file name without its extension, exactly as the reference sound sets name it.
    pub(crate) fn file_stem(self) -> &'static str {
        match self {
            SystemSound::Scratch => "scratch",
            SystemSound::FolderOpen => "f-open",
            SystemSound::FolderClose => "f-close",
            SystemSound::OptionChange => "o-change",
            SystemSound::OptionOpen => "o-open",
            SystemSound::OptionClose => "o-close",
            SystemSound::PlayReady => "playready",
            SystemSound::PlayStop => "playstop",
            SystemSound::ResultClear => "clear",
            SystemSound::ResultFail => "fail",
            SystemSound::ResultClose => "resultclose",
            SystemSound::CourseClear => "course_clear",
            SystemSound::CourseFail => "course_fail",
            SystemSound::CourseClose => "course_close",
            SystemSound::GuidePg => "guide-pg",
            SystemSound::GuideGr => "guide-gr",
            SystemSound::GuideGd => "guide-gd",
            SystemSound::GuideBd => "guide-bd",
            SystemSound::GuidePr => "guide-pr",
            SystemSound::GuideMs => "guide-ms",
            SystemSound::Select => "select",
            SystemSound::Decide => "decide",
        }
    }

    /// Index into a set's slots, equal to this sound's position in [`SystemSound::ALL`].
    pub(crate) fn slot(self) -> usize {
        match self {
            SystemSound::Scratch => 0,
            SystemSound::FolderOpen => 1,
            SystemSound::FolderClose => 2,
            SystemSound::OptionChange => 3,
            SystemSound::OptionOpen => 4,
            SystemSound::OptionClose => 5,
            SystemSound::PlayReady => 6,
            SystemSound::PlayStop => 7,
            SystemSound::ResultClear => 8,
            SystemSound::ResultFail => 9,
            SystemSound::ResultClose => 10,
            SystemSound::CourseClear => 11,
            SystemSound::CourseFail => 12,
            SystemSound::CourseClose => 13,
            SystemSound::GuidePg => 14,
            SystemSound::GuideGr => 15,
            SystemSound::GuideGd => 16,
            SystemSound::GuideBd => 17,
            SystemSound::GuidePr => 18,
            SystemSound::GuideMs => 19,
            SystemSound::Select => 20,
            SystemSound::Decide => 21,
        }
    }

    /// Whether this is one of the six per-judgment guide cues, which the guide switch governs.
    pub(crate) fn is_guide(self) -> bool {
        matches!(self, SystemSound::GuidePg | SystemSound::GuideGr | SystemSound::GuideGd | SystemSound::GuideBd | SystemSound::GuidePr | SystemSound::GuideMs)
    }

    /// Whether the reference implementation treats this stem as part of the BGM set rather than the
    /// effect set (`SystemSoundManager.java:149-150`). rbms resolves both out of one folder; this
    /// stays so a future split can tell them apart without re-deriving the list.
    #[cfg(test)]
    pub(crate) fn is_bgm(self) -> bool {
        matches!(self, SystemSound::Select | SystemSound::Decide)
    }

    /// The mixer sample id this sound occupies once installed.
    pub(crate) fn sample_id(self) -> u32 {
        SYSTEM_SOUND_NAMESPACE.base + self.slot() as u32
    }
}

/// The guide cue for a judgment, in the reference implementation's judge-code order
/// (`BMSPlayer.java:398`, index 0..5 = PG, GR, GD, BD, PR, MS).
pub(crate) fn guide_for_judge(judge: Judge) -> SystemSound {
    match judge {
        Judge::PerfectGreat => SystemSound::GuidePg,
        Judge::Great => SystemSound::GuideGr,
        Judge::Good => SystemSound::GuideGd,
        Judge::Bad => SystemSound::GuideBd,
        Judge::Poor => SystemSound::GuidePr,
        Judge::Miss => SystemSound::GuideMs,
    }
}

/// Which cue a chart result raises.
pub(crate) fn result_sound(cleared: bool) -> SystemSound {
    if cleared { SystemSound::ResultClear } else { SystemSound::ResultFail }
}

/// Which cue a course result raises.
pub(crate) fn course_result_sound(cleared: bool) -> SystemSound {
    if cleared { SystemSound::CourseClear } else { SystemSound::CourseFail }
}

/// Where a stem resolves to inside `dir`, and the extension it resolved through. `None` when the
/// folder holds no file for it, which leaves that one cue silent.
pub(crate) fn resolve_sound(dir: &Path, sound: SystemSound) -> Option<(PathBuf, String)> {
    resolve_file(dir, sound.file_stem(), &SYSTEM_SOUND_EXTENSIONS)
}

/// What [`SystemSoundSet::play`] would ask the mixer for. Separating the decision from the call
/// keeps the gates — resolved, guide switch, usable gain — testable without an output device.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SystemSoundCue {
    pub(crate) id: u32,
    pub(crate) gain: f32,
}

/// What the select music needs done on the mixer this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BgmStep {
    Idle,
    /// Start the loop.
    Start,
    /// Stop the loop.
    Stop,
    /// Move the loop's level to this value, keeping it running.
    Level(f32),
}

/// Whether the select music is running, on which output stream, and whether it is turned down.
///
/// It is asked once a frame for what it wants ([`SelectBgm::step`]) and is told nothing else, so a
/// stream that was reopened (and so lost the loop) is noticed by its instance id and the loop is
/// started again on the new one.
///
/// A loop is never started while it is to be held: it would begin audibly and be turned down a
/// moment later. It starts on the first frame it is not held.
#[derive(Debug, Default)]
pub(crate) struct SelectBgm {
    started_on: Option<u64>,
    held: bool,
}

impl SelectBgm {
    /// What to do this frame. `wanted` is whether the browser is up and has music, `engine` the
    /// instance id of the open output stream (`None` when there is none), and `hold` whether a
    /// preview is sounding.
    pub(crate) fn step(&mut self, wanted: bool, engine: Option<u64>, hold: bool) -> BgmStep {
        let running = engine.is_some() && self.started_on == engine;
        if !wanted {
            *self = SelectBgm::default();
            return if running { BgmStep::Stop } else { BgmStep::Idle };
        }
        if !running {
            if hold || engine.is_none() {
                *self = SelectBgm::default();
                return BgmStep::Idle;
            }
            *self = SelectBgm { started_on: engine, held: false };
            return BgmStep::Start;
        }
        if hold == self.held {
            return BgmStep::Idle;
        }
        self.held = hold;
        BgmStep::Level(if hold { SELECT_BGM_HELD_LEVEL } else { SELECT_BGM_FULL_LEVEL })
    }
}

/// A loaded sound set: at most one sample per [`SystemSound`], plus the guide switch.
pub(crate) struct SystemSoundSet {
    slots: Vec<Slot>,
    guide_enabled: bool,
    bgm: SelectBgm,
}

impl Default for SystemSoundSet {
    fn default() -> Self {
        SystemSoundSet::silent()
    }
}

impl SystemSoundSet {
    /// A set that plays nothing, which is what an unconfigured folder yields.
    pub(crate) fn silent() -> SystemSoundSet {
        SystemSoundSet { slots: (0..SYSTEM_SOUND_COUNT).map(|_| Slot::Missing).collect(), guide_enabled: false, bgm: SelectBgm::default() }
    }

    /// Read and decode every stem found under `dir`. A stem with no file, or with a file that will
    /// not decode, leaves its slot silent and does not stop the rest of the set from loading.
    pub(crate) fn load(dir: &Path) -> SystemSoundSet {
        let mut set = SystemSoundSet::silent();
        for sound in SystemSound::ALL {
            let Some((path, ext)) = resolve_sound(dir, sound) else {
                continue;
            };
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(err) => {
                    notify(Level::Warn, format!("[syssound] read failed {}: {err}", path.display()));
                    continue;
                }
            };
            match rbms_audio::decode_bytes(bytes, Some(ext.as_str())) {
                Ok(decoded) => set.slots[sound.slot()] = Slot::Decoded(decoded),
                Err(err) => notify(Level::Warn, format!("[syssound] decode failed {}: {err}", path.display())),
            }
        }
        set
    }

    /// Load from a configured folder, or stay silent when none is set.
    pub(crate) fn load_optional(dir: Option<&Path>) -> SystemSoundSet {
        match dir {
            Some(dir) => SystemSoundSet::load(dir),
            None => SystemSoundSet::silent(),
        }
    }

    /// Turn the six per-judgment guide cues on or off. Off is the default.
    pub(crate) fn set_guide_enabled(&mut self, enabled: bool) {
        self.guide_enabled = enabled;
    }

    #[cfg(test)]
    pub(crate) fn guide_enabled(&self) -> bool {
        self.guide_enabled
    }

    /// Whether a file was found and decoded for this sound.
    pub(crate) fn is_resolved(&self, sound: SystemSound) -> bool {
        matches!(self.slots[sound.slot()], Slot::Decoded(_))
    }

    /// How long the sound in this slot lasts, or `None` when there is none.
    pub(crate) fn duration_us(&self, sound: SystemSound) -> Option<i64> {
        let Slot::Decoded(decoded) = &self.slots[sound.slot()] else {
            return None;
        };
        let frames = decoded.samples.len() as i64 / i64::from(decoded.channels.max(1));
        Some(frames * MICROS_PER_SECOND / i64::from(decoded.rate.max(1)))
    }

    /// Whether this set's `select` sound is music for the browser to loop rather than a cue.
    pub(crate) fn select_plays_as_bgm(&self) -> bool {
        self.duration_us(SystemSound::Select).is_some_and(|duration| duration >= SELECT_BGM_MIN_DURATION_US)
    }

    /// How many of the twenty-two slots hold a sample.
    #[cfg(test)]
    pub(crate) fn resolved_count(&self) -> usize {
        self.slots.iter().filter(|slot| matches!(slot, Slot::Decoded(_))).count()
    }

    /// Copy every decoded sample into the mixer bank. Run it whenever the shared stream is opened
    /// — including after a reopen, which empties the bank — before [`SystemSoundSet::play`] can be
    /// heard. Playing without it is silent rather than an error.
    pub(crate) fn install(&self, engine: &mut AudioEngine) {
        for sound in SystemSound::ALL {
            if let Slot::Decoded(decoded) = &self.slots[sound.slot()] {
                engine.insert_decoded(sound.sample_id(), DecodedAudio { samples: decoded.samples.clone(), channels: decoded.channels, rate: decoded.rate });
            }
        }
    }

    /// What playing `sound` once at `gain` would ask of the mixer, or `None` when this set stays
    /// silent for it: no file, a guide cue with guides off, a gain that is not a usable number, or the
    /// `select` sound of a set whose `select` is music (see the module documentation), which is looped
    /// by the browser and not played as the cue for starting a chart.
    pub(crate) fn cue(&self, sound: SystemSound, gain: f32) -> Option<SystemSoundCue> {
        if sound == SystemSound::Select && self.select_plays_as_bgm() {
            return None;
        }
        self.loop_cue(sound, gain)
    }

    /// What starting `sound` as a loop at `gain` would ask of the mixer: [`SystemSoundSet::cue`]
    /// without the rule that keeps music from being played as a cue.
    pub(crate) fn loop_cue(&self, sound: SystemSound, gain: f32) -> Option<SystemSoundCue> {
        if !self.is_resolved(sound) {
            return None;
        }
        if sound.is_guide() && !self.guide_enabled {
            return None;
        }
        if !gain.is_finite() {
            return None;
        }
        Some(SystemSoundCue { id: sound.sample_id(), gain: gain.clamp(SYSTEM_SOUND_GAIN_MIN, SYSTEM_SOUND_GAIN_MAX) })
    }

    /// Raise `sound` on the System bus. Silent, never fatal, when this set has nothing for it.
    pub(crate) fn play(&self, engine: &mut AudioEngine, sound: SystemSound, gain: f32) {
        let Some(cue) = self.cue(sound, gain) else {
            return;
        };
        engine.play_on(Bus::System, cue.id, cue.gain, SYSTEM_SOUND_PAN, SYSTEM_SOUND_PITCH, SYSTEM_SOUND_AT_US);
    }

    /// Start `sound` and repeat it until it is stopped ([`SystemSoundSet::stop`]). Silent, never
    /// fatal, when this set has nothing for it. A second call starts a second copy on top of the
    /// first, as the reference's does, so a caller asks once.
    pub(crate) fn play_loop(&self, engine: &mut AudioEngine, sound: SystemSound, gain: f32) {
        let Some(cue) = self.loop_cue(sound, gain) else {
            return;
        };
        engine.play_effect(Bus::System, cue.id, cue.gain, true);
    }

    /// Silence `sound`. Asked of a cue that is not sounding it does nothing, and it is asked even of
    /// one whose file has since been replaced: a copy of the old file that is still sounding is
    /// stopped by the sample id they share.
    pub(crate) fn stop(&self, engine: &mut AudioEngine, sound: SystemSound) {
        engine.stop(sound.sample_id());
    }
}

impl AppShared {
    /// Start a system sound and repeat it until [`AppShared::stop_system_sound`]. Silent when the
    /// stream is not open or the set has no file for it.
    pub(crate) fn play_system_sound_loop(&mut self, sound: SystemSound) {
        if let Some(engine) = self.audio.as_mut() {
            self.syssound.play_loop(engine, sound, SYSTEM_SOUND_GAIN);
        }
    }

    /// Play a result screen's clear or fail cue once, or repeated when `looped` (the player's
    /// `isLoopResultSound` / `isLoopCourseResultSound`).
    pub(crate) fn play_result_sound(&mut self, sound: SystemSound, looped: bool) {
        if looped {
            self.play_system_sound_loop(sound);
        } else {
            self.play_system_sound(sound);
        }
    }

    /// Bring the select music to where the browser wants it: running while `wanted`, turned down
    /// while `hold`, over `ramp`. Called every frame the browser is up and once with `wanted` false
    /// when it is left. Does nothing, and opens no stream, for a set without select music.
    pub(crate) fn drive_select_bgm(&mut self, wanted: bool, hold: bool, ramp: Duration) {
        let wanted = wanted && self.syssound.select_plays_as_bgm();
        if wanted {
            self.ensure_audio();
        }
        let engine_id = self.audio.as_ref().map(AudioEngine::instance_id);
        let step = self.syssound.bgm.step(wanted, engine_id, hold);
        let Some(engine) = self.audio.as_mut() else {
            return;
        };
        match step {
            BgmStep::Idle => {}
            BgmStep::Start => self.syssound.play_loop(engine, SystemSound::Select, SYSTEM_SOUND_GAIN),
            BgmStep::Stop => self.syssound.stop(engine, SystemSound::Select),
            BgmStep::Level(level) => engine.set_effect_level(SystemSound::Select.sample_id(), level, ramp.as_secs_f32() * MILLIS_PER_SECOND),
        }
    }

    /// Silence a system sound. Nothing happens when the stream is not open.
    pub(crate) fn stop_system_sound(&mut self, sound: SystemSound) {
        if let Some(engine) = self.audio.as_mut() {
            self.syssound.stop(engine, sound);
        }
    }
}

#[cfg(test)]
mod tests;

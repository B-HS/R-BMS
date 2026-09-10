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
//! Playback goes through the System bus so the system-sound volume is the one that governs it.

use std::path::{Path, PathBuf};

use rbms_audio::{AudioEngine, Bus, DecodedAudio, IdNamespace};
use rbms_judge::Judge;

use crate::notify::{Level, notify};
use crate::resolve_file;

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

/// A loaded sound set: at most one sample per [`SystemSound`], plus the guide switch.
pub(crate) struct SystemSoundSet {
    slots: Vec<Slot>,
    guide_enabled: bool,
}

impl Default for SystemSoundSet {
    fn default() -> Self {
        SystemSoundSet::silent()
    }
}

impl SystemSoundSet {
    /// A set that plays nothing, which is what an unconfigured folder yields.
    pub(crate) fn silent() -> SystemSoundSet {
        SystemSoundSet { slots: (0..SYSTEM_SOUND_COUNT).map(|_| Slot::Missing).collect(), guide_enabled: false }
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

    pub(crate) fn guide_enabled(&self) -> bool {
        self.guide_enabled
    }

    /// Whether a file was found and decoded for this sound.
    pub(crate) fn is_resolved(&self, sound: SystemSound) -> bool {
        matches!(self.slots[sound.slot()], Slot::Decoded(_))
    }

    /// How many of the twenty-two slots hold a sample.
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

    /// What playing `sound` at `gain` would ask of the mixer, or `None` when this set stays silent
    /// for it: no file, a guide cue with guides off, or a gain that is not a usable number.
    pub(crate) fn cue(&self, sound: SystemSound, gain: f32) -> Option<SystemSoundCue> {
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

    /// Stop a cue that is still sounding, for the long stems a screen change cuts short.
    pub(crate) fn stop(&self, engine: &mut AudioEngine, sound: SystemSound) {
        if self.is_resolved(sound) {
            engine.stop(sound.sample_id());
        }
    }
}

#[cfg(test)]
mod tests;

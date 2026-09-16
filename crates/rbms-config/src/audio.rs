use serde::{Deserialize, Serialize};

/// Master output gain a fresh install starts at. The per-bus defaults carry the loudness split, so
/// the master itself stays transparent.
pub const DEFAULT_MASTER_VOLUME: f32 = 1.0;

/// Per-bus (key / bgm / system) output gain a fresh install starts at.
pub const DEFAULT_BUS_VOLUME: f32 = 0.5;

/// Quietest gain a volume row can produce.
pub const AUDIO_VOLUME_MIN_GAIN: f32 = 0.0;

/// Loudest gain a volume row can produce.
pub const AUDIO_VOLUME_MAX_GAIN: f32 = 1.0;

/// Percent one left/right step moves a volume row.
pub const AUDIO_VOLUME_STEP_PERCENT: i32 = 5;

/// Loudest a volume row goes.
pub const AUDIO_VOLUME_MAX_PERCENT: i32 = 100;

/// Output buffer sizes in frames the BUFFER SIZE row offers after AUTO.
pub const AUDIO_BUFFER_FRAMES_CHOICES: [u32; 8] = [128, 192, 256, 384, 512, 768, 1024, 2048];

/// Output rates in Hz the SAMPLE RATE row offers after AUTO.
pub const AUDIO_SAMPLE_RATE_HZ_CHOICES: [u32; 4] = [44_100, 48_000, 88_200, 96_000];

/// Fewest simultaneous voices the POLYPHONY row allows.
pub const AUDIO_POLYPHONY_MIN_VOICES: usize = 64;

/// Most simultaneous voices the POLYPHONY row allows.
pub const AUDIO_POLYPHONY_MAX_VOICES: usize = 1024;

/// Voices one left/right step moves the POLYPHONY row.
pub const AUDIO_POLYPHONY_STEP_VOICES: usize = 64;

/// Voices a fresh install lets the mixer sound at once. Mirrors the audio engine's own default; the
/// player pins the two together with a test rather than depending on the engine from here.
pub const DEFAULT_POLYPHONY_VOICES: usize = 512;

/// The AUDIO tab's values: the four gains, which reach a running stream immediately, and the four
/// output parameters, which only take effect when the stream is (re)opened.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioOptions {
    /// Output device name, `None` for the system default.
    pub device: Option<String>,
    /// Requested buffer size in frames, `None` for the backend default.
    pub buffer_frames: Option<u32>,
    /// Requested output rate in Hz, `None` for the device default.
    pub sample_rate: Option<u32>,
    /// Voices the mixer may sound at once.
    pub polyphony: usize,
    /// Master gain applied after the buses are summed.
    pub master: f32,
    /// Gain of the key bus.
    pub key: f32,
    /// Gain of the bgm bus.
    pub bg: f32,
    /// Gain of the system bus.
    pub system: f32,
    /// Folder holding the system sound set. `None` leaves every system sound silent.
    pub sound_folder: Option<String>,
    /// Play the per-judgment guide cues while a chart runs.
    pub guide_se: bool,
    #[serde(skip)]
    pub(crate) reopen_pending: bool,
}

impl Default for AudioOptions {
    fn default() -> Self {
        AudioOptions {
            device: None,
            buffer_frames: None,
            sample_rate: None,
            polyphony: DEFAULT_POLYPHONY_VOICES,
            master: DEFAULT_MASTER_VOLUME,
            key: DEFAULT_BUS_VOLUME,
            bg: DEFAULT_BUS_VOLUME,
            system: DEFAULT_BUS_VOLUME,
            sound_folder: None,
            guide_se: false,
            reopen_pending: false,
        }
    }
}

/// Two option sets are equal when the documents they would be written as are equal.
///
/// `reopen_pending` is live UI state, not part of the document: it says a row has moved since the
/// stream was opened. Counting it would make "has the configuration changed?" answer yes for a
/// parameter that is merely waiting to be applied, and answer differently before and after
/// [`AudioOptions::sanitise`] clears it.
impl PartialEq for AudioOptions {
    fn eq(&self, other: &Self) -> bool {
        self.device == other.device
            && self.buffer_frames == other.buffer_frames
            && self.sample_rate == other.sample_rate
            && self.polyphony == other.polyphony
            && self.master == other.master
            && self.key == other.key
            && self.bg == other.bg
            && self.system == other.system
            && self.sound_folder == other.sound_folder
            && self.guide_se == other.guide_se
    }
}

impl AudioOptions {
    /// Pull every field back into the range its row can produce, so a hand-edited file cannot open
    /// the stream outside what the app supports. Applied on load, never on save.
    pub fn sanitise(&mut self) {
        self.device = self.device.take().filter(|d| !d.trim().is_empty());
        self.buffer_frames = self.buffer_frames.filter(|f| *f > 0);
        self.sample_rate = self.sample_rate.filter(|r| *r > 0);
        self.polyphony = self.polyphony.clamp(AUDIO_POLYPHONY_MIN_VOICES, AUDIO_POLYPHONY_MAX_VOICES);
        self.master = clamp_volume(self.master, DEFAULT_MASTER_VOLUME);
        self.key = clamp_volume(self.key, DEFAULT_BUS_VOLUME);
        self.bg = clamp_volume(self.bg, DEFAULT_BUS_VOLUME);
        self.system = clamp_volume(self.system, DEFAULT_BUS_VOLUME);
        self.sound_folder = self.sound_folder.take().filter(|d| !d.trim().is_empty());
        self.reopen_pending = false;
    }

    /// Whether an output parameter has moved since the stream was last opened.
    pub fn reopen_pending(&self) -> bool {
        self.reopen_pending
    }

    /// Record that the stream has to be reopened for the current parameters to be heard.
    pub fn mark_reopen_pending(&mut self) {
        self.reopen_pending = true;
    }

    /// Clear the pending reopen once the stream has been opened with the current parameters.
    pub fn clear_reopen_pending(&mut self) {
        self.reopen_pending = false;
    }
}

/// A stored gain clamped to the range the volume rows can produce, falling back to `fallback` when
/// a hand-edited file holds something that is not a number.
pub fn clamp_volume(gain: f32, fallback: f32) -> f32 {
    if gain.is_nan() { fallback } else { gain.clamp(AUDIO_VOLUME_MIN_GAIN, AUDIO_VOLUME_MAX_GAIN) }
}

/// A gain as the whole percent its row shows.
pub fn volume_percent(gain: f32) -> i32 {
    let max = AUDIO_VOLUME_MAX_PERCENT as f32;
    (gain * max).round().clamp(0.0, max) as i32
}

/// One left/right step on a volume row, as a gain. Stepping goes through whole percent so a row
/// held down never drifts off the values it displays.
pub fn step_volume(gain: f32, d: i32) -> f32 {
    let percent = (volume_percent(gain) + d * AUDIO_VOLUME_STEP_PERCENT).clamp(0, AUDIO_VOLUME_MAX_PERCENT);
    percent as f32 / AUDIO_VOLUME_MAX_PERCENT as f32
}

/// One step through `AUTO` (`None`) followed by `choices`. A stored value that is not in `choices`
/// (a hand-edited file) steps from the `AUTO` position.
pub fn cycle_optional_u32(current: Option<u32>, choices: &[u32], d: i32) -> Option<u32> {
    let len = choices.len() as i32 + 1;
    let at = current.and_then(|v| choices.iter().position(|c| *c == v)).map_or(0, |i| i as i32 + 1);
    let next = (at + d).rem_euclid(len);
    (next > 0).then(|| choices[(next - 1) as usize])
}

/// One step through the system default (`None`) followed by the enumerated device names. A stored
/// name the host no longer reports steps from the default position.
pub fn cycle_device(current: Option<&str>, names: &[String], d: i32) -> Option<String> {
    let len = names.len() as i32 + 1;
    let at = current.and_then(|v| names.iter().position(|n| n == v)).map_or(0, |i| i as i32 + 1);
    let next = (at + d).rem_euclid(len);
    (next > 0).then(|| names[(next - 1) as usize].clone())
}

/// One step on the POLYPHONY row, clamped to the voice range the mixer is allowed to run with.
pub fn step_polyphony(current: usize, d: i32) -> usize {
    let stepped = current as i64 + d as i64 * AUDIO_POLYPHONY_STEP_VOICES as i64;
    stepped.clamp(AUDIO_POLYPHONY_MIN_VOICES as i64, AUDIO_POLYPHONY_MAX_VOICES as i64) as usize
}

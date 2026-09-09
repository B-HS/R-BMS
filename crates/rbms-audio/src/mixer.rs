use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

/// User-facing master gain, applied once after the bus sum. Unity by default: the reference
/// implementation's 0.5 playback volume lives on the per-bus gains instead (see
/// [`DEFAULT_BUS_GAIN`]), so the rendered amplitude of a default configuration is unchanged.
pub(crate) const DEFAULT_MASTER_GAIN: f32 = 1.0;

/// Default gain of every output bus, matching the reference implementation's `systemvolume`,
/// `keyvolume` and `bgvolume` (`AudioConfig.java:43-54`), which are all 0.5.
pub(crate) const DEFAULT_BUS_GAIN: f32 = 0.5;

/// Default chart gain, used until a chart declares `#VOLWAV` (`AbstractAudioDriver.java:355-358`
/// falls back to 1.0 for a missing or out-of-range value).
pub(crate) const DEFAULT_CHART_GAIN: f32 = 1.0;

/// Amplitude below which the master bus is perfectly linear. Above it the soft limiter
/// (see [`soft_limit`]) compresses toward 1.0 instead of hard-clipping.
pub(crate) const SOFT_LIMIT_THRESHOLD: f32 = 0.8;

/// Channel keys per sample id, matching the reference implementation's `channel(id, pitch) = id * 256 + pitch + 128`
/// (`AbstractAudioDriver.java:507-509`).
pub(crate) const CHANNELS_PER_SAMPLE_ID: u32 = 256;

/// Bias added to the semitone offset so the whole semitone range maps into `0..256`.
pub(crate) const PITCH_CHANNEL_BIAS: i32 = 128;

const MIN_SEMITONE_OFFSET: i32 = -128;
const MAX_SEMITONE_OFFSET: i32 = 127;
const SEMITONES_PER_OCTAVE: f32 = 12.0;

/// Smallest pitch ratio a voice may be started at, so the read stride never collapses to zero and
/// leaves a voice reading one source frame forever.
const MIN_PITCH_RATIO: f32 = 0.0001;

/// Fade-in of every voice start. Short enough that the attack transient of even a few-millisecond
/// keysound survives, long enough to remove the step discontinuity at sample zero.
const ATTACK_MS: f32 = 1.0;

/// Fade-out shared by stop, retrigger and voice stealing. The reference implementation cuts a
/// channel instantly (`AbstractAudioDriver.java:507-527`); the ramp is a deliberate rbms divergence
/// that trades three milliseconds of tail for the absence of a click.
const RELEASE_MS: f32 = 3.0;

/// How long a master, bus or chart gain change takes to travel to its new value, whatever the size
/// of the change. Short enough that a settings row still feels immediate, long enough that the step
/// lands as a ramp instead of a click.
const GAIN_SLEW_MS: f32 = 3.0;

const MS_PER_SECOND: f32 = 1000.0;

/// Per-frame envelope increment of a ramp lasting `ms` at `out_rate`. A ramp is at least one frame
/// long, so a degenerate output rate yields an instant ramp instead of dividing by zero.
fn ramp_step_per_frame(out_rate: u32, ms: f32) -> f32 {
    1.0 / (ms * out_rate as f32 / MS_PER_SECOND).max(1.0)
}

/// Frames a gain change is spread over, so the travel time is [`GAIN_SLEW_MS`] whatever the size of
/// the change.
fn slew_frames(out_rate: u32) -> f32 {
    (GAIN_SLEW_MS * out_rate as f32 / MS_PER_SECOND).max(1.0)
}

/// Per-frame travel that moves `current` to `target` over `frames` frames.
fn slew_step(current: f32, target: f32, frames: f32) -> f32 {
    (target - current).abs() / frames
}

/// Move `current` toward `target` by at most `travel`, stopping exactly on the target.
fn approach(current: f32, target: f32, travel: f32) -> f32 {
    let delta = target - current;
    if delta > travel {
        return current + travel;
    }
    if delta < -travel {
        return current - travel;
    }
    target
}

/// Semitone offset of a pitch ratio, quantized the way the reference implementation stores it: it keeps the integer
/// `pitchShift` and derives the ratio as `2^(pitchShift/12)`, so the inverse is
/// `round(12 * log2(ratio))`. Non-positive ratios have no defined offset and map to 0.
pub(crate) fn semitone_offset(pitch: f32) -> i32 {
    if pitch <= 0.0 || !pitch.is_finite() {
        return 0;
    }
    (SEMITONES_PER_OCTAVE * pitch.log2()).round().clamp(MIN_SEMITONE_OFFSET as f32, MAX_SEMITONE_OFFSET as f32) as i32
}

/// Voice channel key for a sample id played at a pitch ratio. Two plays of the same id at
/// different semitones get different keys, so they layer instead of cutting each other; the same
/// id at the same semitone re-triggers (cuts) as before.
pub(crate) fn channel_key(id: u32, pitch: f32) -> u32 {
    let biased = (semitone_offset(pitch) + PITCH_CHANNEL_BIAS) as u32;
    id.wrapping_mul(CHANNELS_PER_SAMPLE_ID).wrapping_add(biased)
}

/// Sample id a channel key belongs to (inverse of [`channel_key`]).
pub(crate) fn channel_sample_id(key: u32) -> u32 {
    key / CHANNELS_PER_SAMPLE_ID
}

/// Master-bus soft limiter. Linear up to [`SOFT_LIMIT_THRESHOLD`], then a tanh knee that is
/// continuous in value and slope at the threshold, strictly increasing, and bounded by 1.0 — so a
/// growing sum of simultaneous voices keeps getting louder instead of flattening into clipping
/// distortion.
pub(crate) fn soft_limit(x: f32) -> f32 {
    let mag = x.abs();
    if mag <= SOFT_LIMIT_THRESHOLD {
        return x;
    }
    let knee = 1.0 - SOFT_LIMIT_THRESHOLD;
    let limited = SOFT_LIMIT_THRESHOLD + knee * ((mag - SOFT_LIMIT_THRESHOLD) / knee).tanh();
    if x < 0.0 { -limited } else { limited }
}

/// Decoded keysound at its native sample rate, interleaved f32. Resampling to the
/// device rate happens per-voice via a fractional read stride (see `Voice::stride`),
/// which also yields pitch shifting for free.
pub struct SampleData {
    pub pcm: Arc<[f32]>,
    pub channels: u16,
    pub rate: u32,
}

impl SampleData {
    pub fn frames(&self) -> usize {
        if self.channels == 0 { 0 } else { self.pcm.len() / self.channels as usize }
    }
}

/// Output bus a voice is routed to. The three gains mirror the reference implementation's
/// `systemvolume` / `keyvolume` / `bgvolume` (`AudioConfig.java:43-54`) and are applied per voice,
/// before the bus sum is scaled by the chart and master gains.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bus {
    System,
    Key,
    Bg,
}

impl Bus {
    /// Every bus in declaration order, for callers that iterate per-bus gains.
    pub const ALL: [Bus; 3] = [Bus::System, Bus::Key, Bus::Bg];

    fn index(self) -> usize {
        match self {
            Bus::System => 0,
            Bus::Key => 1,
            Bus::Bg => 2,
        }
    }
}

/// Mixer telemetry for the debug overlay: how many voices are sounding, how often the allocator had
/// to take a slot from a fading or an audible voice, and how many schedules collapsed onto the
/// current frame instead of a future one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MixStats {
    pub active_voices: u32,
    pub steals: u64,
    pub hard_steals: u64,
    pub late_schedules: u64,
}

/// Stage of a voice's amplitude envelope. `Attack` ramps in from silence, `Sustain` holds unity, and
/// `Release` ramps out and frees the slot when it reaches silence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum VoicePhase {
    Attack,
    Sustain,
    Release,
}

struct Voice {
    sample: Option<Arc<SampleData>>,
    pos: f64,
    stride: f64,
    gain: f32,
    lgain: f32,
    rgain: f32,
    delay: u64,
    key: u32,
    bus: Bus,
    env: f32,
    phase: VoicePhase,
    start_frame: u64,
    active: bool,
    /// A start queued behind this voice's fade-out. Set when the pool was full and this slot was the
    /// one taken, so the incoming sound waits out the victim's release instead of cutting it off
    /// mid-waveform. Started in place, sample accurately, the frame the release reaches silence.
    pending: Option<PlayRequest>,
}

impl Voice {
    fn idle() -> Self {
        Voice {
            sample: None,
            pos: 0.0,
            stride: 1.0,
            gain: 0.0,
            lgain: 0.0,
            rgain: 0.0,
            delay: 0,
            key: 0,
            bus: Bus::Key,
            env: 0.0,
            phase: VoicePhase::Attack,
            start_frame: 0,
            active: false,
            pending: None,
        }
    }
}

/// Where the audio callback hands a finished sample back to the game thread. Dropping the last
/// `Arc<SampleData>` frees several megabytes of PCM, which the real-time thread must not do, so the
/// callback moves it into a ring instead and the owner frees it. A full ring is the only case where
/// the callback still drops one itself; it is bounded and counted.
struct RetireSink {
    producer: Option<rtrb::Producer<Arc<SampleData>>>,
    overflows: u64,
}

impl RetireSink {
    fn new() -> Self {
        RetireSink { producer: None, overflows: 0 }
    }

    fn take(&mut self, sample: Option<Arc<SampleData>>) {
        let Some(sample) = sample else {
            return;
        };
        let Some(producer) = self.producer.as_mut() else {
            self.overflows += 1;
            return;
        };
        if producer.push(sample).is_err() {
            self.overflows += 1;
        }
    }
}

/// Start the fade-out of a voice. A voice that has not emitted a sample yet — still waiting out its
/// scheduling delay, or stopped before its first mix — is dropped outright instead: there is no
/// discontinuity to smooth, and the slot becomes free immediately. A voice already fading is left
/// alone so a repeated stop cannot restart its ramp.
fn release(v: &mut Voice, retire: &mut RetireSink) {
    if v.phase == VoicePhase::Release {
        return;
    }
    if v.env <= 0.0 {
        v.active = false;
        retire.take(v.sample.take());
        return;
    }
    v.phase = VoicePhase::Release;
}

/// Drop a start queued behind this voice before it ever sounded.
fn cancel_pending(v: &mut Voice, retire: &mut RetireSink) {
    let queued = v.pending.take();
    retire.take(queued.map(|req| req.sample));
}

/// Fill a slot with a queued start. `at` is the mixer frame the voice is being started on, which is
/// the buffer boundary for an immediate start and the frame a fade ended on for a queued one.
fn start_in_place(v: &mut Voice, req: PlayRequest, delay: u64, at: u64, out_rate: u32) {
    let angle = (req.pan.clamp(-1.0, 1.0) + 1.0) * 0.5 * FRAC_PI_2;
    let stride = (req.sample.rate as f64 / out_rate.max(1) as f64) * req.pitch.max(MIN_PITCH_RATIO) as f64;
    v.sample = Some(req.sample);
    v.pos = 0.0;
    v.stride = stride;
    v.gain = req.gain;
    v.lgain = angle.cos();
    v.rgain = angle.sin();
    v.delay = delay;
    v.key = req.key;
    v.bus = req.bus;
    v.env = 0.0;
    v.phase = VoicePhase::Attack;
    v.start_frame = at + delay;
    v.active = true;
    v.pending = None;
}

/// A mixer instruction queued from the game thread and applied on the audio callback.
///
/// `Play` carries the [`Bus`] the voice belongs to; `StopRange` stops a half-open span of channel
/// keys, which is how a whole sample-id namespace is cleared. `ChartGain` is `#VOLWAV` / 100 and is
/// applied to every bus, matching the reference implementation, which multiplies the same
/// chart-derived volume into note sounds (`AbstractAudioDriver.java:486-491`) and judge sounds
/// (`AbstractAudioDriver.java:502`) alike.
#[non_exhaustive]
pub enum Command {
    Play { sample: Arc<SampleData>, gain: f32, pan: f32, pitch: f32, key: u32, at_frame: u64, bus: Bus },
    Stop { key: u32 },
    StopId { id: u32 },
    StopRange { lo_key: u32, hi_key: u32 },
    MasterGain(f32),
    BusGain { bus: Bus, gain: f32 },
    ChartGain(f32),
}

/// Everything needed to start one voice, kept as a struct so the mixer's start path stays a single
/// argument wide as the voice model grows.
struct PlayRequest {
    sample: Arc<SampleData>,
    gain: f32,
    pan: f32,
    pitch: f32,
    key: u32,
    at_frame: u64,
    bus: Bus,
}

/// Which rung of the voice-stealing ladder produced a slot.
enum SlotChoice {
    Free(usize),
    Releasing(usize),
    Audible(usize),
    Exhausted,
}

/// Real-time-safe software mixer. `mix` performs no allocation and no locking.
pub struct Mixer {
    voices: Vec<Voice>,
    master_gain: f32,
    master_gain_target: f32,
    bus_gain: [f32; Bus::ALL.len()],
    bus_gain_target: [f32; Bus::ALL.len()],
    chart_gain: f32,
    chart_gain_target: f32,
    master_slew: f32,
    bus_slew: [f32; Bus::ALL.len()],
    chart_slew: f32,
    slew_frames: f32,
    out_rate: u32,
    out_channels: u16,
    clock: u64,
    attack_step: f32,
    release_step: f32,
    steals: u64,
    hard_steals: u64,
    late_schedules: u64,
    retire: RetireSink,
}

impl Mixer {
    pub fn new(out_rate: u32, out_channels: u16, max_voices: usize) -> Self {
        Mixer {
            voices: (0..max_voices).map(|_| Voice::idle()).collect(),
            master_gain: DEFAULT_MASTER_GAIN,
            master_gain_target: DEFAULT_MASTER_GAIN,
            bus_gain: [DEFAULT_BUS_GAIN; Bus::ALL.len()],
            bus_gain_target: [DEFAULT_BUS_GAIN; Bus::ALL.len()],
            chart_gain: DEFAULT_CHART_GAIN,
            chart_gain_target: DEFAULT_CHART_GAIN,
            master_slew: 0.0,
            bus_slew: [0.0; Bus::ALL.len()],
            chart_slew: 0.0,
            slew_frames: slew_frames(out_rate),
            out_rate,
            out_channels,
            clock: 0,
            attack_step: ramp_step_per_frame(out_rate, ATTACK_MS),
            release_step: ramp_step_per_frame(out_rate, RELEASE_MS),
            steals: 0,
            hard_steals: 0,
            late_schedules: 0,
            retire: RetireSink::new(),
        }
    }

    /// Hand the mixer the producer end of the retirement ring. The consumer stays with the owner,
    /// which drains it on its own thread; without one the callback frees finished samples itself,
    /// which is only acceptable in tests that load no real PCM.
    pub fn set_retire(&mut self, producer: rtrb::Producer<Arc<SampleData>>) {
        self.retire.producer = Some(producer);
    }

    /// Samples the callback had to free itself because the retirement ring was full. Expected to
    /// stay 0; a non-zero value means the owner is not draining often enough.
    pub fn retire_overflows(&self) -> u64 {
        self.retire.overflows
    }

    pub fn clock_frames(&self) -> u64 {
        self.clock
    }

    /// Mixer telemetry for the debug overlay. The voice count is taken live, so it stays correct
    /// between mix calls; the steal and late-schedule counters accumulate for the mixer's lifetime.
    pub fn stats(&self) -> MixStats {
        MixStats {
            active_voices: self.voices.iter().filter(|v| v.active).count() as u32,
            steals: self.steals,
            hard_steals: self.hard_steals,
            late_schedules: self.late_schedules,
        }
    }

    pub fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Play { sample, gain, pan, pitch, key, at_frame, bus } => self.start_voice(PlayRequest { sample, gain, pan, pitch, key, at_frame, bus }),
            Command::Stop { key } => self.stop(key),
            Command::StopId { id } => self.stop_id(id),
            Command::StopRange { lo_key, hi_key } => self.stop_range(lo_key, hi_key),
            Command::MasterGain(g) => {
                self.master_slew = slew_step(self.master_gain, g, self.slew_frames);
                self.master_gain_target = g;
            }
            Command::BusGain { bus, gain } => {
                self.bus_slew[bus.index()] = slew_step(self.bus_gain[bus.index()], gain, self.slew_frames);
                self.bus_gain_target[bus.index()] = gain;
            }
            Command::ChartGain(g) => {
                self.chart_slew = slew_step(self.chart_gain, g, self.slew_frames);
                self.chart_gain_target = g;
            }
        }
    }

    /// Route a queued play to a slot. A free slot starts it at once; otherwise the chosen victim is
    /// only faded out and the start is queued behind that fade, so no slot ever jumps from one
    /// waveform to another mid-sample. When every slot is busy and already has something queued the
    /// play is dropped, which is silent where a cut would have clicked.
    fn start_voice(&mut self, req: PlayRequest) {
        self.stop(req.key);
        match self.pick_slot() {
            SlotChoice::Free(slot) => self.begin(slot, req),
            SlotChoice::Releasing(slot) => {
                self.steals += 1;
                self.queue_behind(slot, req);
            }
            SlotChoice::Audible(slot) => {
                self.hard_steals += 1;
                self.queue_behind(slot, req);
            }
            SlotChoice::Exhausted => self.retire.take(Some(req.sample)),
        }
    }

    /// Fade the voice in `slot` out and queue `req` to start the frame that fade reaches silence.
    /// A victim that had not sounded yet frees its slot immediately, so the start is not delayed.
    fn queue_behind(&mut self, slot: usize, req: PlayRequest) {
        release(&mut self.voices[slot], &mut self.retire);
        if !self.voices[slot].active {
            self.begin(slot, req);
            return;
        }
        cancel_pending(&mut self.voices[slot], &mut self.retire);
        self.voices[slot].pending = Some(req);
    }

    /// Start `req` in `slot` right now, counting a schedule that collapsed onto the current frame.
    fn begin(&mut self, slot: usize, req: PlayRequest) {
        let delay = req.at_frame.saturating_sub(self.clock);
        if delay == 0 && req.at_frame != 0 {
            self.late_schedules += 1;
        }
        let clock = self.clock;
        let out_rate = self.out_rate;
        let retire = &mut self.retire;
        let voice = &mut self.voices[slot];
        retire.take(voice.sample.take());
        cancel_pending(voice, retire);
        start_in_place(voice, req, delay, clock, out_rate);
    }

    /// Start the fade-out of every voice on `key`, and drop anything queued to start on it.
    fn stop(&mut self, key: u32) {
        let retire = &mut self.retire;
        for v in &mut self.voices {
            if v.pending.as_ref().is_some_and(|req| req.key == key) {
                cancel_pending(v, retire);
            }
            if v.active && v.key == key {
                release(v, retire);
            }
        }
    }

    /// Start the fade-out of every voice of a sample id, whatever pitch it was started at.
    fn stop_id(&mut self, id: u32) {
        let retire = &mut self.retire;
        for v in &mut self.voices {
            if v.pending.as_ref().is_some_and(|req| channel_sample_id(req.key) == id) {
                cancel_pending(v, retire);
            }
            if v.active && channel_sample_id(v.key) == id {
                release(v, retire);
            }
        }
    }

    /// Start the fade-out of every voice whose channel key lies in the half-open range
    /// `[lo_key, hi_key)`.
    fn stop_range(&mut self, lo_key: u32, hi_key: u32) {
        let retire = &mut self.retire;
        for v in &mut self.voices {
            if v.pending.as_ref().is_some_and(|req| req.key >= lo_key && req.key < hi_key) {
                cancel_pending(v, retire);
            }
            if v.active && v.key >= lo_key && v.key < hi_key {
                release(v, retire);
            }
        }
    }

    /// Pick the slot a new voice should take, in the order: a free slot, else the quietest voice
    /// already fading out, else the quietest sounding voice (oldest first when two are equally
    /// quiet). A slot that already has a start queued behind its fade is never taken, so a queued
    /// sound cannot be displaced by a later one; when nothing is left the pool is exhausted.
    fn pick_slot(&self) -> SlotChoice {
        let mut releasing: Option<(usize, f32)> = None;
        let mut audible: Option<(usize, f32, u64)> = None;
        for (i, v) in self.voices.iter().enumerate() {
            if !v.active {
                return SlotChoice::Free(i);
            }
            if v.pending.is_some() {
                continue;
            }
            if v.phase == VoicePhase::Release {
                if releasing.is_none_or(|(_, quietest)| v.env < quietest) {
                    releasing = Some((i, v.env));
                }
                continue;
            }
            let amplitude = v.gain.abs() * v.env;
            if audible.is_none_or(|(_, quietest, oldest)| amplitude < quietest || (amplitude == quietest && v.start_frame < oldest)) {
                audible = Some((i, amplitude, v.start_frame));
            }
        }
        if let Some((i, _)) = releasing {
            return SlotChoice::Releasing(i);
        }
        match audible {
            Some((i, _, _)) => SlotChoice::Audible(i),
            None => SlotChoice::Exhausted,
        }
    }

    /// Render one output buffer. Every active voice advances its envelope, reads its sample with
    /// linear interpolation (the last source frame interpolates against itself, so a one-frame
    /// sample still sounds), and is summed at its bus gain; the sum is then scaled by the chart and
    /// master gains and passed through [`soft_limit`].
    ///
    /// A voice that ends part way through the buffer either frees its slot or, when a start was
    /// queued behind its fade, hands the rest of the buffer to that start on the exact frame it
    /// falls silent. Gains that were changed since the last buffer travel to their new value over
    /// [`GAIN_SLEW_MS`] rather than jumping, so a volume step is a ramp instead of a click.
    pub fn mix(&mut self, out: &mut [f32]) {
        for s in out.iter_mut() {
            *s = 0.0;
        }
        let oc = self.out_channels.max(1) as usize;
        let frames = out.len() / oc;
        let bus_gain = self.bus_gain;
        let bus_target = self.bus_gain_target;
        let bus_slew = self.bus_slew;
        let attack_step = self.attack_step;
        let release_step = self.release_step;
        let clock = self.clock;
        let out_rate = self.out_rate;
        let retire = &mut self.retire;
        let late_schedules = &mut self.late_schedules;

        for v in &mut self.voices {
            let mut f = 0usize;
            while f < frames && v.active {
                let Some(sd) = v.sample.clone() else {
                    break;
                };
                let src_ch = sd.channels as usize;
                let nframes = sd.frames();
                let pcm = &sd.pcm;
                let bi = v.bus.index();
                let bus_settled = bus_gain[bi] == bus_target[bi];
                let mut ended = false;

                while f < frames {
                    if v.delay > 0 {
                        v.delay -= 1;
                        f += 1;
                        continue;
                    }
                    match v.phase {
                        VoicePhase::Attack => {
                            v.env += attack_step;
                            if v.env >= 1.0 {
                                v.env = 1.0;
                                v.phase = VoicePhase::Sustain;
                            }
                        }
                        VoicePhase::Sustain => {}
                        VoicePhase::Release => {
                            v.env -= release_step;
                            if v.env <= 0.0 {
                                v.env = 0.0;
                                ended = true;
                                break;
                            }
                        }
                    }
                    let i = v.pos as usize;
                    if i >= nframes {
                        ended = true;
                        break;
                    }
                    let frac = (v.pos - i as f64) as f32;
                    let next = (i + 1).min(nframes - 1);
                    let (sl, sr) = if src_ch == 1 {
                        let a = pcm[i];
                        let b = pcm[next];
                        let s = a + (b - a) * frac;
                        (s, s)
                    } else {
                        let base = i * src_ch;
                        let nb = next * src_ch;
                        let l = pcm[base] + (pcm[nb] - pcm[base]) * frac;
                        let r = pcm[base + 1] + (pcm[nb + 1] - pcm[base + 1]) * frac;
                        (l, r)
                    };
                    let vbus = if bus_settled { bus_gain[bi] } else { approach(bus_gain[bi], bus_target[bi], bus_slew[bi] * f as f32) };
                    let g = v.gain * v.env * vbus;
                    let l = sl * g * v.lgain;
                    let r = sr * g * v.rgain;
                    let o = f * oc;
                    if oc == 1 {
                        out[o] += (l + r) * 0.5;
                    } else {
                        out[o] += l;
                        out[o + 1] += r;
                    }
                    v.pos += v.stride;
                    f += 1;
                }

                if !ended {
                    break;
                }
                drop(sd);
                retire.take(v.sample.take());
                match v.pending.take() {
                    Some(req) => {
                        let at = clock + f as u64;
                        let delay = req.at_frame.saturating_sub(at);
                        if delay == 0 && req.at_frame != 0 {
                            *late_schedules += 1;
                        }
                        start_in_place(v, req, delay, at, out_rate);
                    }
                    None => v.active = false,
                }
            }
        }

        for (f, frame) in out.chunks_mut(oc).enumerate() {
            let chart = approach(self.chart_gain, self.chart_gain_target, self.chart_slew * f as f32);
            let master = approach(self.master_gain, self.master_gain_target, self.master_slew * f as f32);
            let g = chart * master;
            for s in frame.iter_mut() {
                *s = soft_limit(*s * g);
            }
        }

        let spanned = frames as f32;
        self.master_gain = approach(self.master_gain, self.master_gain_target, self.master_slew * spanned);
        self.chart_gain = approach(self.chart_gain, self.chart_gain_target, self.chart_slew * spanned);
        for (i, current) in self.bus_gain.iter_mut().enumerate() {
            *current = approach(*current, bus_target[i], bus_slew[i] * spanned);
        }
        self.clock += frames as u64;
    }

    #[cfg(test)]
    fn play(&mut self, sample: Arc<SampleData>, gain: f32, pan: f32, pitch: f32, key: u32, at_frame: u64) {
        self.start_voice(PlayRequest { sample, gain, pan, pitch, key, at_frame, bus: Bus::Key });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(n: usize, rate: u32, channels: u16) -> Arc<SampleData> {
        let pcm: Vec<f32> = (0..n * channels as usize).map(|i| (i % 100) as f32 / 100.0).collect();
        Arc::new(SampleData { pcm: pcm.into(), channels, rate })
    }

    #[test]
    fn plays_and_advances_clock() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(10, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 4);
        assert!(out.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn stride_resamples_by_rate_ratio() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(100, 24000, 1), 1.0, 0.0, 1.0, 1, 0);
        assert!((m.voices[0].stride - 0.5).abs() < 1e-9);
    }

    #[test]
    fn delay_is_sample_accurate() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(ramp(10, 48000, 1), 1.0, 0.0, 1.0, 1, 3);
        let mut out = vec![0.0f32; 5];
        m.mix(&mut out);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], 0.0);
        assert_eq!(out[2], 0.0);
        assert!(out[3] != 0.0 || out[4] != 0.0);
    }

    #[test]
    fn retrigger_same_key_cuts_previous() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 7, 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 7, 0);
        let active = m.voices.iter().filter(|v| v.active).count();
        assert_eq!(active, 1);
    }

    #[test]
    fn voice_deactivates_at_end() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(4, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 64];
        m.mix(&mut out);
        assert_eq!(m.voices.iter().filter(|v| v.active).count(), 0);
    }

    /// A sample whose every PCM value is `val`, so interpolation is exact and easy to reason about.
    fn flat(frames: usize, channels: u16, rate: u32, val: f32) -> Arc<SampleData> {
        let pcm: Vec<f32> = vec![val; frames * channels as usize];
        Arc::new(SampleData { pcm: pcm.into(), channels, rate })
    }

    /// A stereo sample with distinct constant L and R values for channel-routing assertions.
    fn flat_lr(frames: usize, rate: u32, lval: f32, rval: f32) -> Arc<SampleData> {
        let mut pcm = Vec::with_capacity(frames * 2);
        for _ in 0..frames {
            pcm.push(lval);
            pcm.push(rval);
        }
        Arc::new(SampleData { pcm: pcm.into(), channels: 2, rate })
    }

    fn active_count(m: &Mixer) -> usize {
        m.voices.iter().filter(|v| v.active).count()
    }

    /// Voices that are still sounding, i.e. active and not fading out.
    fn sounding_count(m: &Mixer) -> usize {
        m.voices.iter().filter(|v| v.active && v.phase != VoicePhase::Release).count()
    }

    /// Frames one ramp of `ms` occupies at `rate`, mirroring [`ramp_step_per_frame`].
    fn ramp_frames(rate: u32, ms: f32) -> usize {
        (ms * rate as f32 / MS_PER_SECOND) as usize
    }

    /// Frames rendered before an amplitude is read, chosen past every ramp at the 48 kHz test rate —
    /// the attack envelope and the gain slew — so a hand-computed gain chain can be compared.
    const STEADY_STATE_FRAMES: usize = 192;

    /// Length of a fixture sample that has to outlast several steady-state reads in a row.
    const STEADY_SAMPLE_FRAMES: usize = 4096;

    /// Largest sample-to-sample change any of the mixer's ramps may produce at full scale. Every
    /// ramp here is at least a millisecond long, so a single frame moves the envelope by well under
    /// a percent; anything above this is a waveform discontinuity, i.e. a click.
    const MAX_RAMP_STEP: f32 = 0.01;

    /// The whole default gain chain a settled voice is multiplied by: bus, chart and master.
    const DEFAULT_CHAIN_GAIN: f32 = DEFAULT_BUS_GAIN * DEFAULT_CHART_GAIN * DEFAULT_MASTER_GAIN;

    /// Renders [`STEADY_STATE_FRAMES`] frames and returns the last frame's samples.
    fn steady_frame(m: &mut Mixer, out_channels: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; STEADY_STATE_FRAMES * out_channels];
        m.mix(&mut out);
        out[(STEADY_STATE_FRAMES - 1) * out_channels..].to_vec()
    }

    #[test]
    fn clock_starts_at_zero() {
        let m = Mixer::new(48000, 2, 16);
        assert_eq!(m.clock_frames(), 0);
    }

    #[test]
    fn clock_advances_by_frames_not_samples_stereo() {
        let mut m = Mixer::new(48000, 2, 16);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 4);
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 8);
    }

    #[test]
    fn clock_advances_by_frames_mono() {
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 8);
    }

    #[test]
    fn empty_buffer_advances_clock_by_zero() {
        let mut m = Mixer::new(48000, 2, 16);
        let mut out: Vec<f32> = vec![];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 0);
    }

    #[test]
    fn out_channels_zero_treated_as_one_frame_per_sample() {
        let mut m = Mixer::new(48000, 0, 16);
        let mut out = vec![0.0f32; 6];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 6);
    }

    #[test]
    fn mix_clears_buffer_each_call_when_silent() {
        let mut m = Mixer::new(48000, 2, 16);
        let mut out = vec![0.5f32; 8];
        m.mix(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn mix_is_deterministic_for_equal_state() {
        let mut a = Mixer::new(48000, 2, 16);
        let mut b = Mixer::new(48000, 2, 16);
        a.play(ramp(50, 44100, 1), 0.8, -0.3, 1.2, 1, 2);
        b.play(ramp(50, 44100, 1), 0.8, -0.3, 1.2, 1, 2);
        let mut oa = vec![0.0f32; 32];
        let mut ob = vec![0.0f32; 32];
        a.mix(&mut oa);
        b.mix(&mut ob);
        assert_eq!(oa, ob);
    }

    #[test]
    fn delay_silence_before_then_audio_at_frame() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(10, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 4);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], 0.0);
        assert_eq!(out[2], 0.0);
        assert_eq!(out[3], 0.0);
        assert!(out[4] != 0.0);
    }

    #[test]
    fn delay_at_frame_zero_plays_immediately() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(10, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        assert_eq!(m.voices[0].delay, 0);
        let mut out = vec![0.0f32; 4];
        m.mix(&mut out);
        assert!(out[0] != 0.0);
    }

    #[test]
    fn delay_in_the_past_clamps_to_zero() {
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 10);
        m.play(flat(10, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 3);
        assert_eq!(m.voices[0].delay, 0);
    }

    #[test]
    fn delay_relative_to_clock_offset() {
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        m.play(flat(10, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 13);
        assert_eq!(m.voices[0].delay, 3);
    }

    #[test]
    fn delay_spanning_multiple_mix_calls() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(20, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 6);
        let mut out = vec![0.0f32; 4];
        m.mix(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
        let mut out2 = vec![0.0f32; 4];
        m.mix(&mut out2);
        assert_eq!(out2[0], 0.0);
        assert_eq!(out2[1], 0.0);
        assert!(out2[2] != 0.0);
    }

    #[test]
    fn stride_upsample_when_source_higher_rate() {
        let mut m = Mixer::new(24000, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        assert!((m.voices[0].stride - 2.0).abs() < 1e-9);
    }

    #[test]
    fn stride_unity_when_rates_match() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        assert!((m.voices[0].stride - 1.0).abs() < 1e-9);
    }

    #[test]
    fn stride_scales_with_pitch() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, 2.0, 1, 0);
        assert!((m.voices[0].stride - 2.0).abs() < 1e-9);
    }

    #[test]
    fn stride_combines_rate_ratio_and_pitch() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(100, 24000, 1), 1.0, 0.0, 3.0, 1, 0);
        assert!((m.voices[0].stride - 1.5).abs() < 1e-9);
    }

    #[test]
    fn pitch_floor_prevents_zero_stride() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, 0.0, 1, 0);
        assert!(m.voices[0].stride > 0.0);
        assert!((m.voices[0].stride - 0.0001).abs() < 1e-9);
    }

    #[test]
    fn negative_pitch_clamped_to_floor() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, -5.0, 1, 0);
        assert!((m.voices[0].stride - 0.0001).abs() < 1e-9);
    }

    #[test]
    fn out_rate_zero_treated_as_one() {
        let mut m = Mixer::new(0, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        assert!((m.voices[0].stride - 48000.0).abs() < 1e-6);
    }

    #[test]
    fn pan_center_equal_power() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(10, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        let v = &m.voices[0];
        assert!((v.lgain - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((v.rgain - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn pan_full_left() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(10, 48000, 1), 1.0, -1.0, 1.0, 1, 0);
        let v = &m.voices[0];
        assert!((v.lgain - 1.0).abs() < 1e-6);
        assert!(v.rgain.abs() < 1e-6);
    }

    #[test]
    fn pan_full_right() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(10, 48000, 1), 1.0, 1.0, 1.0, 1, 0);
        let v = &m.voices[0];
        assert!(v.lgain.abs() < 1e-6);
        assert!((v.rgain - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pan_out_of_range_clamped() {
        let mut left = Mixer::new(48000, 2, 16);
        left.play(ramp(10, 48000, 1), 1.0, -3.0, 1.0, 1, 0);
        assert!((left.voices[0].lgain - 1.0).abs() < 1e-6);
        assert!(left.voices[0].rgain.abs() < 1e-6);
        let mut right = Mixer::new(48000, 2, 16);
        right.play(ramp(10, 48000, 1), 1.0, 5.0, 1.0, 1, 0);
        assert!(right.voices[0].lgain.abs() < 1e-6);
        assert!((right.voices[0].rgain - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pan_gains_constant_power_invariant() {
        for pan in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
            let mut m = Mixer::new(48000, 2, 16);
            m.play(ramp(10, 48000, 1), 1.0, pan, 1.0, 1, 0);
            let v = &m.voices[0];
            let power = v.lgain * v.lgain + v.rgain * v.rgain;
            assert!((power - 1.0).abs() < 1e-5, "pan {pan} power {power}");
        }
    }

    #[test]
    fn mono_source_duplicated_to_lr_full_left_isolates_left() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(10, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert!(out[0] != 0.0);
        assert_eq!(out[1], 0.0);
        assert!(out[2] != 0.0);
        assert_eq!(out[3], 0.0);
    }

    #[test]
    fn stereo_source_routes_channels_independently() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat_lr(256, 48000, 0.5, 0.25), 1.0, 0.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        let g = std::f32::consts::FRAC_1_SQRT_2 * DEFAULT_CHAIN_GAIN;
        assert!((frame[0] - 0.5 * g).abs() < 1e-5);
        assert!((frame[1] - 0.25 * g).abs() < 1e-5);
        assert!(frame[0] > frame[1], "the louder source channel stays louder");
    }

    #[test]
    fn mono_output_averages_l_and_r() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat_lr(256, 48000, 1.0, 0.0), 1.0, 0.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 1);
        let g = std::f32::consts::FRAC_1_SQRT_2;
        assert!((frame[0] - (1.0 * g) * 0.5 * DEFAULT_CHAIN_GAIN).abs() < 1e-5);
    }

    #[test]
    fn voice_gain_scales_amplitude() {
        let mut full = Mixer::new(48000, 2, 16);
        full.play(flat(10, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let mut of = vec![0.0f32; 2];
        full.mix(&mut of);

        let mut half = Mixer::new(48000, 2, 16);
        half.play(flat(10, 1, 48000, 0.5), 0.5, -1.0, 1.0, 1, 0);
        let mut oh = vec![0.0f32; 2];
        half.mix(&mut oh);
        assert!((of[0] * 0.5 - oh[0]).abs() < 1e-6);
    }

    #[test]
    fn master_defaults_to_unity_and_the_headroom_lives_on_the_buses() {
        assert_eq!(DEFAULT_MASTER_GAIN, 1.0);
        assert_eq!(DEFAULT_BUS_GAIN, 0.5);
        assert_eq!(DEFAULT_CHART_GAIN, 1.0);
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        assert!((frame[0] - 0.25).abs() < 1e-6, "got {}", frame[0]);
    }

    #[test]
    fn master_gain_scales_output() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(0.5));
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.4), 1.0, -1.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        assert!((frame[0] - 0.4 * DEFAULT_BUS_GAIN * 0.5).abs() < 1e-6, "got {}", frame[0]);
    }

    #[test]
    fn heavy_overdrive_saturates_at_exactly_full_scale() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(8.0));
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.9), 1.0, -1.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        assert_eq!(frame[0], 1.0);
    }

    #[test]
    fn heavy_overdrive_saturates_at_exactly_negative_full_scale() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(8.0));
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, -0.9), 1.0, -1.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        assert_eq!(frame[0], -1.0);
    }

    #[test]
    fn mild_overdrive_takes_the_soft_knee_instead_of_hard_clipping() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(4.0));
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        assert!((frame[0] - 0.952_318_8).abs() < 1e-6, "got {}", frame[0]);
    }

    #[test]
    fn mild_negative_overdrive_takes_the_soft_knee_instead_of_hard_clipping() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(4.0));
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, -0.5), 1.0, -1.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        assert!((frame[0] + 0.952_318_8).abs() < 1e-6, "got {}", frame[0]);
    }

    #[test]
    fn negative_master_gain_inverts_phase() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(-1.0));
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let frame = steady_frame(&mut m, 2);
        assert!((frame[0] + 0.5 * DEFAULT_BUS_GAIN).abs() < 1e-6, "got {}", frame[0]);
    }

    #[test]
    fn retrigger_keeps_only_one_active_and_uses_new_sample() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 7, 0);
        let first_slot = m.voices.iter().position(|v| v.active).unwrap();
        m.play(ramp(1000, 48000, 1), 0.3, 0.0, 1.0, 7, 0);
        assert_eq!(active_count(&m), 1);
        let active = m.voices.iter().find(|v| v.active).unwrap();
        assert!((active.gain - 0.3).abs() < 1e-6);
        let _ = first_slot;
    }

    #[test]
    fn different_keys_coexist() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 2, 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(active_count(&m), 3);
    }

    #[test]
    fn voice_stealing_when_pool_full() {
        let mut m = Mixer::new(48000, 2, 2);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 2, 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(active_count(&m), 2);
        assert_eq!(m.voices.len(), 2);
        assert_eq!(m.stats().hard_steals, 1);
    }

    #[test]
    fn allocation_takes_the_first_free_slot() {
        let mut m = Mixer::new(48000, 2, 3);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        assert_eq!(m.voices[0].key, 1);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 2, 0);
        assert_eq!(m.voices[1].key, 2);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(m.voices[2].key, 3);
    }

    #[test]
    fn stop_deactivates_matching_key() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 5, 0);
        assert_eq!(active_count(&m), 1);
        m.apply(Command::Stop { key: 5 });
        assert_eq!(active_count(&m), 0);
    }

    #[test]
    fn stop_unknown_key_is_noop() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 5, 0);
        m.apply(Command::Stop { key: 999 });
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn stop_then_play_same_key_reactivates() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 5, 0);
        m.apply(Command::Stop { key: 5 });
        assert_eq!(active_count(&m), 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 5, 0);
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn play_via_command_path() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::Play { sample: ramp(100, 48000, 1), gain: 1.0, pan: 0.0, pitch: 1.0, key: 9, at_frame: 0, bus: Bus::Bg });
        assert_eq!(active_count(&m), 1);
        assert_eq!(m.voices.iter().find(|v| v.active).unwrap().key, 9);
    }

    #[test]
    fn all_frames_play_with_no_tail_truncation() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(5, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 16];
        m.mix(&mut out);
        let nonzero = out.iter().filter(|&&s| s != 0.0).count();
        assert_eq!(nonzero, 5, "all 5 source frames play (no last-frame truncation)");
        assert_eq!(active_count(&m), 0);
    }

    #[test]
    fn single_frame_sample_emits_one_frame() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(1, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 16];
        m.mix(&mut out);
        assert_ne!(out[0], 0.0, "the single frame is audible");
        assert!(out[1..].iter().all(|&s| s == 0.0), "only one frame is emitted");
        assert_eq!(active_count(&m), 0);
    }

    #[test]
    fn voice_position_persists_across_mix_calls() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(100, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        let pos_after_first = m.voices[0].pos;
        assert!((pos_after_first - 10.0).abs() < 1e-9);
        m.mix(&mut out);
        assert!((m.voices[0].pos - 20.0).abs() < 1e-9);
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn higher_stride_consumes_sample_faster() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(20, 1, 48000, 0.5), 1.0, 0.0, 2.0, 1, 0);
        let mut out = vec![0.0f32; 64];
        m.mix(&mut out);
        let nonzero = out.iter().filter(|&&s| s != 0.0).count();
        assert_eq!(nonzero, 10);
    }

    #[test]
    fn sampledata_frames_zero_channels_is_zero() {
        let sd = SampleData { pcm: vec![0.1, 0.2, 0.3].into(), channels: 0, rate: 48000 };
        assert_eq!(sd.frames(), 0);
    }

    #[test]
    fn sampledata_frames_counts_correctly() {
        let mono = SampleData { pcm: vec![0.0; 10].into(), channels: 1, rate: 48000 };
        assert_eq!(mono.frames(), 10);
        let stereo = SampleData { pcm: vec![0.0; 10].into(), channels: 2, rate: 48000 };
        assert_eq!(stereo.frames(), 5);
    }

    #[test]
    fn no_active_voices_produces_silence() {
        let mut m = Mixer::new(48000, 2, 16);
        let mut out = vec![0.0f32; 32];
        m.mix(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn soft_limit_is_identity_below_threshold() {
        assert_eq!(soft_limit(0.0), 0.0);
        assert_eq!(soft_limit(0.25), 0.25);
        assert_eq!(soft_limit(-0.25), -0.25);
        assert_eq!(soft_limit(SOFT_LIMIT_THRESHOLD), SOFT_LIMIT_THRESHOLD);
    }

    #[test]
    fn soft_limit_knee_matches_hand_computed_tanh() {
        assert!((soft_limit(1.0) - 0.952_318_8).abs() < 1e-6, "got {}", soft_limit(1.0));
        assert!((soft_limit(1.2) - 0.992_805_5).abs() < 1e-6, "got {}", soft_limit(1.2));
    }

    #[test]
    fn soft_limit_is_odd_symmetric() {
        assert!((soft_limit(-1.0) + 0.952_318_8).abs() < 1e-6, "got {}", soft_limit(-1.0));
        for x in [0.3f32, 0.8, 1.0, 2.5, 40.0] {
            assert!((soft_limit(-x) + soft_limit(x)).abs() < 1e-6, "x {x}");
        }
    }

    #[test]
    fn soft_limit_never_exceeds_full_scale() {
        for x in [0.9f32, 1.0, 2.0, 10.0, 1000.0] {
            assert!(soft_limit(x) <= 1.0, "x {x} -> {}", soft_limit(x));
            assert!(soft_limit(-x) >= -1.0, "x {x} -> {}", soft_limit(-x));
        }
    }

    #[test]
    fn simultaneous_voices_increase_monotonically_without_clipping() {
        let mut previous = 0.0f32;
        for n in 1..=8u32 {
            let mut m = Mixer::new(48000, 2, 16);
            for key in 0..n {
                m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, -1.0, 1.0, key, 0);
            }
            let frame = steady_frame(&mut m, 2);
            assert!(frame[0] <= 1.0, "n {n} clipped at {}", frame[0]);
            assert!(frame[0] > previous, "n {n} did not increase: {} <= {}", frame[0], previous);
            previous = frame[0];
        }
    }

    #[test]
    fn extreme_simultaneous_voice_count_saturates_at_exactly_full_scale() {
        let mut m = Mixer::new(48000, 2, 64);
        for key in 0..64 {
            m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, -1.0, 1.0, key, 0);
        }
        let frame = steady_frame(&mut m, 2);
        assert_eq!(frame[0], 1.0);
    }

    #[test]
    fn four_simultaneous_voices_hit_hand_computed_knee_value() {
        let mut m = Mixer::new(48000, 2, 16);
        for key in 0..4 {
            m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, -1.0, 1.0, key, 0);
        }
        let frame = steady_frame(&mut m, 2);
        assert!((frame[0] - 0.952_318_8).abs() < 1e-6, "got {}", frame[0]);
    }

    #[test]
    fn three_simultaneous_voices_stay_in_linear_region() {
        let mut m = Mixer::new(48000, 2, 16);
        for key in 0..3 {
            m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, -1.0, 1.0, key, 0);
        }
        let frame = steady_frame(&mut m, 2);
        assert!((frame[0] - 0.75).abs() < 1e-6, "got {}", frame[0]);
    }

    #[test]
    fn semitone_offset_quantizes_pitch_ratio() {
        assert_eq!(semitone_offset(1.0), 0);
        assert_eq!(semitone_offset(2.0), 12);
        assert_eq!(semitone_offset(0.5), -12);
        assert_eq!(semitone_offset(4.0), 24);
        assert_eq!(semitone_offset(1.059_463_1), 1);
        assert_eq!(semitone_offset(0.840_896_4), -3);
    }

    #[test]
    fn semitone_offset_of_non_positive_pitch_is_zero() {
        assert_eq!(semitone_offset(0.0), 0);
        assert_eq!(semitone_offset(-2.0), 0);
        assert_eq!(semitone_offset(f32::NAN), 0);
        assert_eq!(semitone_offset(f32::INFINITY), 0);
    }

    #[test]
    fn semitone_offset_clamps_to_channel_range() {
        assert_eq!(semitone_offset(1e30), 127);
        assert_eq!(semitone_offset(1e-30), -128);
    }

    #[test]
    fn channel_key_matches_reference_id_times_256_plus_pitch_plus_128() {
        assert_eq!(channel_key(0, 1.0), 128);
        assert_eq!(channel_key(3, 1.0), 3 * 256 + 128);
        assert_eq!(channel_key(3, 2.0), 3 * 256 + 12 + 128);
        assert_eq!(channel_key(3, 0.5), 3 * 256 - 12 + 128);
        assert_eq!(channel_key(1, 1e30), 256 + 127 + 128);
        assert_eq!(channel_key(1, 1e-30), 256 - 128 + 128);
    }

    #[test]
    fn channel_sample_id_inverts_channel_key() {
        for id in [0u32, 1, 3, 1295, 65536] {
            for pitch in [0.5f32, 1.0, 2.0] {
                assert_eq!(channel_sample_id(channel_key(id, pitch)), id, "id {id} pitch {pitch}");
            }
        }
    }

    #[test]
    fn same_id_different_pitch_layers_instead_of_cutting() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 2.0, channel_key(5, 2.0), 0);
        assert_eq!(active_count(&m), 2);
    }

    #[test]
    fn same_id_same_pitch_still_retriggers() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn near_identical_pitches_share_one_channel() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.01, channel_key(5, 1.01), 0);
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn stop_id_stops_every_pitch_variant() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 2.0, channel_key(5, 2.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(6, 1.0), 0);
        assert_eq!(active_count(&m), 3);
        m.apply(Command::StopId { id: 5 });
        assert_eq!(active_count(&m), 1);
        assert_eq!(channel_sample_id(m.voices.iter().find(|v| v.active).unwrap().key), 6);
    }

    #[test]
    fn stop_channel_leaves_other_pitch_variants_playing() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 2.0, channel_key(5, 2.0), 0);
        m.apply(Command::Stop { key: channel_key(5, 1.0) });
        assert_eq!(active_count(&m), 1);
        assert_eq!(m.voices.iter().find(|v| v.active).unwrap().key, channel_key(5, 2.0));
    }

    #[test]
    fn stop_id_of_unrelated_id_is_noop() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        m.apply(Command::StopId { id: 9 });
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn multiple_voices_sum_additively() {
        let mut one = Mixer::new(48000, 2, 16);
        one.play(flat(10, 1, 48000, 0.3), 1.0, -1.0, 1.0, 1, 0);
        let mut o1 = vec![0.0f32; 2];
        one.mix(&mut o1);

        let mut two = Mixer::new(48000, 2, 16);
        two.play(flat(10, 1, 48000, 0.3), 1.0, -1.0, 1.0, 1, 0);
        two.play(flat(10, 1, 48000, 0.3), 1.0, -1.0, 1.0, 2, 0);
        let mut o2 = vec![0.0f32; 2];
        two.mix(&mut o2);
        assert!((o2[0] - o1[0] * 2.0).abs() < 1e-6);
    }

    fn render_on_bus(bus: Bus) -> Vec<f32> {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::Play { sample: flat(8, 1, 48000, 0.4), gain: 0.7, pan: -0.25, pitch: 1.0, key: channel_key(3, 1.0), at_frame: 0, bus });
        let mut out = vec![0.0f32; 16];
        m.mix(&mut out);
        out
    }

    /// Plays one voice of `sample_value` on `bus` and returns the settled left-channel amplitude.
    fn steady_left_on_bus(m: &mut Mixer, bus: Bus, key: u32, sample_value: f32) -> f32 {
        m.apply(Command::Play { sample: flat(STEADY_SAMPLE_FRAMES, 1, 48000, sample_value), gain: 1.0, pan: -1.0, pitch: 1.0, key, at_frame: 0, bus });
        steady_frame(m, 2)[0]
    }

    #[test]
    fn bus_all_lists_every_bus_exactly_once() {
        assert_eq!(Bus::ALL, [Bus::System, Bus::Key, Bus::Bg]);
        for bus in Bus::ALL {
            assert_eq!(Bus::ALL.iter().filter(|&&b| b == bus).count(), 1, "bus {bus:?} is not listed exactly once");
        }
    }

    #[test]
    fn bus_index_is_unique_and_within_the_gain_array() {
        let mut seen = [false; Bus::ALL.len()];
        for bus in Bus::ALL {
            assert!(!seen[bus.index()], "bus {bus:?} shares an index");
            seen[bus.index()] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn every_bus_starts_at_the_same_default_gain() {
        let reference = render_on_bus(Bus::Key);
        assert!(reference.iter().any(|&s| s != 0.0), "the reference render must be audible");
        for bus in Bus::ALL {
            assert_eq!(render_on_bus(bus), reference, "bus {bus:?} differs at the default gain");
        }
    }

    #[test]
    fn every_bus_defaults_to_the_reference_half_scale() {
        for bus in Bus::ALL {
            let mut m = Mixer::new(48000, 2, 16);
            let left = steady_left_on_bus(&mut m, bus, 1, 0.5);
            assert!((left - 0.5 * DEFAULT_BUS_GAIN).abs() < 1e-6, "bus {bus:?} -> {left}");
        }
    }

    #[test]
    fn bus_gain_scales_only_voices_on_that_bus() {
        for muted in Bus::ALL {
            let mut m = Mixer::new(48000, 2, 16);
            m.apply(Command::BusGain { bus: muted, gain: 0.0 });
            for (i, bus) in Bus::ALL.into_iter().enumerate() {
                m.apply(Command::Play { sample: flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), gain: 1.0, pan: -1.0, pitch: 1.0, key: i as u32, at_frame: 0, bus });
            }
            let left = steady_frame(&mut m, 2)[0];
            let expected = 2.0 * 0.5 * DEFAULT_BUS_GAIN;
            assert!((left - expected).abs() < 1e-6, "muting {muted:?} left {left} instead of {expected}");
        }
    }

    #[test]
    fn bus_gain_takes_effect_while_a_voice_is_already_sounding() {
        let mut m = Mixer::new(48000, 2, 16);
        let before = steady_left_on_bus(&mut m, Bus::Bg, 1, 0.5);
        m.apply(Command::BusGain { bus: Bus::Bg, gain: 1.0 });
        let after = steady_frame(&mut m, 2)[0];
        assert!((before - 0.5 * DEFAULT_BUS_GAIN).abs() < 1e-6, "got {before}");
        assert!((after - 0.5).abs() < 1e-6, "got {after}");
    }

    /// Every gain change travels over [`GAIN_SLEW_MS`] instead of landing in one frame, so a volume
    /// row moved during play ramps rather than clicking. The regression the ramp exists for is the
    /// size of the discontinuity at the moment the command is applied.
    #[test]
    fn a_gain_change_ramps_instead_of_stepping() {
        for command in [Command::MasterGain(0.4), Command::BusGain { bus: Bus::Key, gain: 1.0 }, Command::ChartGain(0.5)] {
            let mut m = Mixer::new(48000, 2, 16);
            m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.8), 1.0, -1.0, 1.0, 1, 0);
            let last_before = steady_frame(&mut m, 2)[0];
            m.apply(command);
            let mut after = vec![0.0f32; 2];
            m.mix(&mut after);
            assert!((after[0] - last_before).abs() < MAX_RAMP_STEP, "a gain change stepped by {}", (after[0] - last_before).abs());
        }
    }

    /// A whole slew is [`GAIN_SLEW_MS`] long whatever the size of the change, so a big jump is not
    /// stretched into a slow fade.
    #[test]
    fn a_gain_change_reaches_its_target_within_one_slew() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.1), 1.0, -1.0, 1.0, 1, 0);
        steady_frame(&mut m, 2);
        m.apply(Command::MasterGain(4.0));
        let frames = ramp_frames(48000, GAIN_SLEW_MS) + 1;
        let mut out = vec![0.0f32; frames * 2];
        m.mix(&mut out);
        let settled = out[(frames - 1) * 2];
        assert!((settled - 0.1 * DEFAULT_BUS_GAIN * 4.0).abs() < 1e-6, "got {settled}");
    }

    #[test]
    fn chart_gain_scales_every_bus() {
        for bus in Bus::ALL {
            let mut m = Mixer::new(48000, 2, 16);
            m.apply(Command::ChartGain(0.5));
            let left = steady_left_on_bus(&mut m, bus, 1, 0.5);
            assert!((left - 0.5 * DEFAULT_BUS_GAIN * 0.5).abs() < 1e-6, "bus {bus:?} -> {left}");
        }
    }

    #[test]
    fn chart_gain_multiplies_with_the_bus_and_master_gains() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::ChartGain(0.5));
        m.apply(Command::MasterGain(0.5));
        m.apply(Command::BusGain { bus: Bus::Key, gain: 0.25 });
        let left = steady_left_on_bus(&mut m, Bus::Key, 1, 0.8);
        assert!((left - 0.8 * 0.25 * 0.5 * 0.5).abs() < 1e-6, "got {left}");
    }

    #[test]
    fn a_half_master_with_unity_buses_matches_a_unity_master_with_half_buses() {
        let mut reallocated = Mixer::new(48000, 2, 16);
        let mut legacy = Mixer::new(48000, 2, 16);
        for bus in Bus::ALL {
            legacy.apply(Command::BusGain { bus, gain: 1.0 });
        }
        legacy.apply(Command::MasterGain(0.5));
        for m in [&mut reallocated, &mut legacy] {
            for (i, bus) in Bus::ALL.into_iter().enumerate() {
                m.apply(Command::Play {
                    sample: flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.4),
                    gain: 0.7,
                    pan: -0.25,
                    pitch: 1.0,
                    key: i as u32,
                    at_frame: 0,
                    bus,
                });
            }
        }
        let mut phase_b = vec![0.0f32; STEADY_SAMPLE_FRAMES];
        let mut pre_phase_b = vec![0.0f32; STEADY_SAMPLE_FRAMES];
        reallocated.mix(&mut phase_b);
        legacy.mix(&mut pre_phase_b);
        let settled = ramp_frames(48000, GAIN_SLEW_MS) * 2;
        let phase_b = &phase_b[settled..];
        let pre_phase_b = &pre_phase_b[settled..];
        assert!(phase_b.iter().any(|&s| s != 0.0), "the comparison must be audible");
        assert_eq!(phase_b, pre_phase_b, "moving the 0.5 from the master onto the buses changed the output");
    }

    #[test]
    fn the_default_chain_reproduces_the_pre_phase_b_output_scale() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.4), 0.7, -1.0, 1.0, 1, 0);
        let left = steady_frame(&mut m, 2)[0];
        assert!((left - 0.4 * 0.7 * 0.5).abs() < 1e-6, "got {left}");
    }

    #[test]
    fn stats_report_the_live_active_voice_count() {
        let mut m = Mixer::new(48000, 2, 16);
        assert_eq!(m.stats(), MixStats::default());
        m.play(flat(4096, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        m.play(flat(4096, 1, 48000, 0.5), 1.0, 0.0, 1.0, 2, 0);
        assert_eq!(m.stats().active_voices, 2);
        let mut out = vec![0.0f32; 32];
        m.mix(&mut out);
        assert_eq!(m.stats().active_voices, 2);
        m.apply(Command::Stop { key: 1 });
        assert_eq!(m.stats().active_voices, 2, "a fading voice is still sounding");
        let mut tail = vec![0.0f32; (ramp_frames(48000, RELEASE_MS) + 2) * 2];
        m.mix(&mut tail);
        assert_eq!(m.stats().active_voices, 1);
    }

    #[test]
    fn stats_counters_start_at_zero_and_only_the_voice_count_is_live() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1024, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let stats = m.stats();
        assert_eq!(stats.active_voices, 1);
        assert_eq!(stats.steals, 0);
        assert_eq!(stats.hard_steals, 0);
        assert_eq!(stats.late_schedules, 0);
    }

    #[test]
    fn late_schedules_count_a_schedule_that_collapsed_onto_the_current_frame() {
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 10);
        m.play(flat(16, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 3);
        assert_eq!(m.stats().late_schedules, 1);
        assert_eq!(m.voices[0].delay, 0);
    }

    #[test]
    fn late_schedules_count_a_schedule_on_the_current_frame() {
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        m.play(flat(16, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 10);
        assert_eq!(m.stats().late_schedules, 1);
    }

    #[test]
    fn late_schedules_ignore_the_immediate_play_convention_of_frame_zero() {
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        m.play(flat(16, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        assert_eq!(m.stats().late_schedules, 0, "at_frame 0 means no delay was intended");
        assert_eq!(m.voices[0].delay, 0);
    }

    #[test]
    fn late_schedules_ignore_a_future_schedule() {
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        m.play(flat(16, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 13);
        assert_eq!(m.stats().late_schedules, 0);
        assert_eq!(m.voices[0].delay, 3);
    }

    #[test]
    fn ramp_step_spans_the_requested_milliseconds() {
        assert!((ramp_step_per_frame(48000, ATTACK_MS) - 1.0 / 48.0).abs() < 1e-9);
        assert!((ramp_step_per_frame(48000, RELEASE_MS) - 1.0 / 144.0).abs() < 1e-9);
        assert!((ramp_step_per_frame(44100, ATTACK_MS) - 1.0 / 44.1).abs() < 1e-6);
    }

    #[test]
    fn ramp_step_of_a_degenerate_rate_is_a_single_frame() {
        assert_eq!(ramp_step_per_frame(0, ATTACK_MS), 1.0);
        assert_eq!(ramp_step_per_frame(0, RELEASE_MS), 1.0);
    }

    #[test]
    fn attack_ramps_in_monotonically_and_settles_at_unity() {
        let attack_frames = ramp_frames(48000, ATTACK_MS);
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(4096, 1, 48000, 1.0), 1.0, 0.0, 1.0, 1, 0);
        let mut values = Vec::new();
        for _ in 0..attack_frames * 2 {
            let mut out = vec![0.0f32; 1];
            m.mix(&mut out);
            values.push(out[0]);
        }
        let steady = *values.last().unwrap();
        assert!(values[0] > 0.0, "the first frame is audible, not silent");
        assert!(values[0] < steady, "the first frame is quieter than the settled level");
        assert!(values.windows(2).all(|w| w[1] >= w[0]), "the attack ramp is monotone");
        for (i, &v) in values.iter().enumerate().skip(attack_frames + 1) {
            assert_eq!(v, steady, "frame {i} moved after the ramp finished");
        }
        let expected = std::f32::consts::FRAC_1_SQRT_2 * DEFAULT_CHAIN_GAIN;
        assert!((steady - expected).abs() < 1e-6, "got {steady}");
    }

    #[test]
    fn the_envelope_reaches_full_scale_after_attack_ms() {
        let attack_frames = ramp_frames(48000, ATTACK_MS);
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(4096, 1, 48000, 1.0), 1.0, 0.0, 1.0, 1, 0);
        let mut before = vec![0.0f32; attack_frames - 1];
        m.mix(&mut before);
        assert!(m.voices[0].env < 1.0, "env {} settled early", m.voices[0].env);
        assert_eq!(m.voices[0].phase, VoicePhase::Attack);
        let mut after = vec![0.0f32; 2];
        m.mix(&mut after);
        assert_eq!(m.voices[0].env, 1.0);
        assert_eq!(m.voices[0].phase, VoicePhase::Sustain);
    }

    #[test]
    fn the_envelope_starts_only_after_the_scheduling_delay() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(1024, 1, 48000, 1.0), 1.0, 0.0, 1.0, 1, 4);
        let mut delayed = vec![0.0f32; 4];
        m.mix(&mut delayed);
        assert_eq!(m.voices[0].env, 0.0);
        assert!(delayed.iter().all(|&s| s == 0.0));
        let mut sounding = vec![0.0f32; 1];
        m.mix(&mut sounding);
        assert!(m.voices[0].env > 0.0);
        assert!(sounding[0] != 0.0);
    }

    #[test]
    fn release_ramps_out_monotonically_and_frees_the_slot() {
        let release_frames = ramp_frames(48000, RELEASE_MS);
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(8192, 1, 48000, 1.0), 1.0, 0.0, 1.0, 5, 0);
        let mut warm = vec![0.0f32; ramp_frames(48000, ATTACK_MS) * 2];
        m.mix(&mut warm);
        m.apply(Command::Stop { key: 5 });
        assert_eq!(m.voices[0].phase, VoicePhase::Release);
        let mut values = Vec::new();
        for _ in 0..release_frames + 2 {
            let mut out = vec![0.0f32; 1];
            m.mix(&mut out);
            values.push(out[0]);
        }
        assert!(values[0] > 0.0, "the fade-out starts from the sounding level");
        assert!(values.windows(2).all(|w| w[1] <= w[0]), "the release ramp is monotone");
        assert_eq!(*values.last().unwrap(), 0.0);
        assert_eq!(active_count(&m), 0);
    }

    #[test]
    fn the_release_lasts_release_ms_before_the_voice_ends() {
        let release_frames = ramp_frames(48000, RELEASE_MS);
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(8192, 1, 48000, 1.0), 1.0, 0.0, 1.0, 5, 0);
        let mut warm = vec![0.0f32; ramp_frames(48000, ATTACK_MS) * 2];
        m.mix(&mut warm);
        m.apply(Command::Stop { key: 5 });
        let mut most = vec![0.0f32; release_frames - 2];
        m.mix(&mut most);
        assert_eq!(active_count(&m), 1, "the fade-out is still running");
        let mut rest = vec![0.0f32; 4];
        m.mix(&mut rest);
        assert_eq!(active_count(&m), 0);
    }

    #[test]
    fn a_stop_before_the_first_mix_frees_the_slot_without_a_ramp() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(1024, 1, 48000, 0.5), 1.0, 0.0, 1.0, 5, 0);
        m.apply(Command::Stop { key: 5 });
        assert_eq!(active_count(&m), 0, "a voice that never sounded cannot click");
    }

    #[test]
    fn a_stop_during_the_scheduling_delay_frees_the_slot_without_a_ramp() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(1024, 1, 48000, 0.5), 1.0, 0.0, 1.0, 5, 100);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert_eq!(active_count(&m), 1);
        m.apply(Command::Stop { key: 5 });
        assert_eq!(active_count(&m), 0);
    }

    #[test]
    fn a_second_stop_does_not_restart_the_release_ramp() {
        let release_frames = ramp_frames(48000, RELEASE_MS);
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(8192, 1, 48000, 1.0), 1.0, 0.0, 1.0, 5, 0);
        let mut warm = vec![0.0f32; ramp_frames(48000, ATTACK_MS) * 2];
        m.mix(&mut warm);
        m.apply(Command::Stop { key: 5 });
        let mut half = vec![0.0f32; release_frames / 2];
        m.mix(&mut half);
        let env = m.voices[0].env;
        assert!(env > 0.0 && env < 1.0, "env {env} is mid-fade");
        m.apply(Command::Stop { key: 5 });
        assert_eq!(m.voices[0].env, env);
        assert_eq!(m.voices[0].phase, VoicePhase::Release);
    }

    #[test]
    fn a_retrigger_lets_the_previous_voice_fade_while_the_new_one_starts() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 7, 0);
        let mut warm = vec![0.0f32; 8];
        m.mix(&mut warm);
        m.play(flat(8192, 1, 48000, 0.5), 0.3, 0.0, 1.0, 7, 0);
        assert_eq!(active_count(&m), 2, "the fading tail overlaps the new voice");
        assert_eq!(sounding_count(&m), 1);
        assert_eq!(m.voices[0].phase, VoicePhase::Release);
        assert_eq!(m.voices[1].phase, VoicePhase::Attack);
        assert!((m.voices[1].gain - 0.3).abs() < 1e-6);
        let mut tail = vec![0.0f32; ramp_frames(48000, RELEASE_MS) + 2];
        m.mix(&mut tail);
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn stop_range_fades_out_sounding_voices_inside_the_span() {
        let mut m = Mixer::new(48000, 1, 16);
        for key in [10u32, 11, 12] {
            m.play(flat(8192, 1, 48000, 0.2), 1.0, 0.0, 1.0, key, 0);
        }
        let mut warm = vec![0.0f32; 8];
        m.mix(&mut warm);
        m.apply(Command::StopRange { lo_key: 10, hi_key: 12 });
        assert_eq!(sounding_count(&m), 1, "only the key outside the span keeps sounding");
        assert_eq!(active_count(&m), 3, "the two inside the span are still fading");
        let mut tail = vec![0.0f32; ramp_frames(48000, RELEASE_MS) + 2];
        m.mix(&mut tail);
        assert_eq!(active_count(&m), 1);
        assert_eq!(m.voices.iter().find(|v| v.active).unwrap().key, 12);
    }

    #[test]
    fn stealing_prefers_a_free_slot() {
        let mut m = Mixer::new(48000, 1, 4);
        m.play(flat(1024, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        m.play(flat(1024, 1, 48000, 0.5), 1.0, 0.0, 1.0, 2, 0);
        assert_eq!(m.stats().steals, 0);
        assert_eq!(m.stats().hard_steals, 0);
        assert_eq!(m.voices[0].key, 1);
        assert_eq!(m.voices[1].key, 2);
    }

    #[test]
    fn stealing_takes_the_quietest_fading_voice_before_an_audible_one() {
        let mut m = Mixer::new(48000, 1, 2);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 2, 0);
        let mut warm = vec![0.0f32; 8];
        m.mix(&mut warm);
        m.apply(Command::Stop { key: 1 });
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(m.stats().steals, 1);
        assert_eq!(m.stats().hard_steals, 0);
        assert_eq!(m.voices[0].key, 1, "the fading voice keeps its slot until its ramp ends");
        assert_eq!(m.voices[0].pending.as_ref().expect("a queued start").key, 3, "the new voice waits behind that fade");
        assert_eq!(m.voices[1].key, 2, "the audible voice kept playing");

        let mut tail = vec![0.0f32; ramp_frames(48000, RELEASE_MS) + 2];
        m.mix(&mut tail);
        assert_eq!(m.voices[0].key, 3, "the queued voice took the slot once the fade finished");
        assert!(m.voices[0].pending.is_none());
    }

    #[test]
    fn stealing_takes_the_quietest_audible_voice_and_counts_a_hard_steal() {
        let mut m = Mixer::new(48000, 1, 2);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        m.play(flat(8192, 1, 48000, 0.5), 0.1, 0.0, 1.0, 2, 0);
        let mut warm = vec![0.0f32; ramp_frames(48000, ATTACK_MS) * 2];
        m.mix(&mut warm);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(m.stats().hard_steals, 1);
        assert_eq!(m.stats().steals, 0);
        assert_eq!(m.voices[0].key, 1, "the loud voice kept playing");
        assert_eq!(m.voices[1].key, 2, "the quietest voice fades out rather than being cut");
        assert_eq!(m.voices[1].phase, VoicePhase::Release);
        assert_eq!(m.voices[1].pending.as_ref().expect("a queued start").key, 3);

        let mut tail = vec![0.0f32; ramp_frames(48000, RELEASE_MS) + 2];
        m.mix(&mut tail);
        assert_eq!(m.voices[1].key, 3, "the queued voice took the slot once the fade finished");
    }

    #[test]
    fn stealing_breaks_an_amplitude_tie_by_taking_the_oldest_voice() {
        let mut m = Mixer::new(48000, 1, 2);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let mut gap = vec![0.0f32; 8];
        m.mix(&mut gap);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 2, 0);
        let mut settle = vec![0.0f32; ramp_frames(48000, ATTACK_MS) * 2];
        m.mix(&mut settle);
        assert_eq!(m.voices[0].env, 1.0);
        assert_eq!(m.voices[1].env, 1.0);
        assert!(m.voices[0].start_frame < m.voices[1].start_frame);
        m.play(flat(8192, 1, 48000, 0.5), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(m.stats().hard_steals, 1);
        assert_eq!(m.voices[0].pending.as_ref().expect("a queued start").key, 3, "the older of two equally loud voices was taken");
        assert!(m.voices[1].pending.is_none());
    }

    /// The regression the queued start exists for: taking a slot from a voice that is still at full
    /// amplitude used to overwrite it in place, so the output jumped from one waveform to another in
    /// a single frame. Both fixtures below produced a step of roughly a third of full scale.
    #[test]
    fn taking_a_slot_from_an_audible_voice_does_not_step_the_output() {
        let mut m = Mixer::new(48000, 1, 1);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.9), 1.0, 0.0, 1.0, 1, 0);
        let before = steady_frame(&mut m, 1)[0];
        assert!(before.abs() > MAX_RAMP_STEP, "the victim must be audible for the test to mean anything");

        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, -0.9), 1.0, 0.0, 1.0, 2, 0);
        assert_eq!(m.stats().hard_steals, 1);
        let mut after = vec![0.0f32; 1];
        m.mix(&mut after);
        assert!((after[0] - before).abs() < MAX_RAMP_STEP, "a hard steal stepped by {}", (after[0] - before).abs());
    }

    #[test]
    fn taking_a_slot_from_a_fading_voice_does_not_step_the_output() {
        let mut m = Mixer::new(48000, 1, 1);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.9), 1.0, 0.0, 1.0, 1, 0);
        let before = steady_frame(&mut m, 1)[0];
        m.apply(Command::Stop { key: 1 });

        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, -0.9), 1.0, 0.0, 1.0, 2, 0);
        assert_eq!(m.stats().steals, 1);
        let mut after = vec![0.0f32; 1];
        m.mix(&mut after);
        assert!((after[0] - before).abs() < MAX_RAMP_STEP, "a releasing steal stepped by {}", (after[0] - before).abs());
    }

    #[test]
    fn a_queued_start_still_sounds_after_the_victim_has_faded() {
        let mut m = Mixer::new(48000, 1, 1);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let mut warm = vec![0.0f32; 8];
        m.mix(&mut warm);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, 0.0, 1.0, 2, 0);

        let mut tail = vec![0.0f32; ramp_frames(48000, RELEASE_MS) + ramp_frames(48000, ATTACK_MS) + 4];
        m.mix(&mut tail);
        assert_eq!(m.voices[0].key, 2, "the queued sound must not be lost");
        assert!(m.voices[0].active);
        assert!(tail[tail.len() - 1].abs() > 0.0, "the queued sound must be audible");
    }

    #[test]
    fn a_slot_that_already_has_a_queued_start_is_not_taken_again() {
        let mut m = Mixer::new(48000, 1, 1);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let mut warm = vec![0.0f32; 8];
        m.mix(&mut warm);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, 0.0, 1.0, 2, 0);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(m.voices[0].pending.as_ref().expect("a queued start").key, 2, "the queued sound is not displaced by a later one");
        assert_eq!(m.stats().hard_steals, 1, "the third play found nothing left to take");
    }

    #[test]
    fn stopping_a_key_cancels_a_start_queued_on_it() {
        let mut m = Mixer::new(48000, 1, 1);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        let mut warm = vec![0.0f32; 8];
        m.mix(&mut warm);
        m.play(flat(STEADY_SAMPLE_FRAMES, 1, 48000, 0.5), 1.0, 0.0, 1.0, 2, 0);
        assert!(m.voices[0].pending.is_some());

        m.apply(Command::StopRange { lo_key: 2, hi_key: 3 });
        assert!(m.voices[0].pending.is_none(), "clearing a namespace must drop what it had queued");

        let mut tail = vec![0.0f32; ramp_frames(48000, RELEASE_MS) + 2];
        m.mix(&mut tail);
        assert!(!m.voices[0].active, "the slot frees instead of starting a cancelled sound");
    }

    #[test]
    fn an_empty_voice_pool_drops_the_play_instead_of_panicking() {
        let mut m = Mixer::new(48000, 2, 0);
        m.play(flat(16, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 0);
        assert_eq!(m.stats().active_voices, 0);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn stop_range_stops_every_key_inside_the_span() {
        let mut m = Mixer::new(48000, 2, 16);
        for key in [10u32, 11, 12] {
            m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, key, 0);
        }
        assert_eq!(active_count(&m), 3);
        m.apply(Command::StopRange { lo_key: 10, hi_key: 13 });
        assert_eq!(active_count(&m), 0);
    }

    #[test]
    fn stop_range_is_half_open_low_inclusive_high_exclusive() {
        let mut m = Mixer::new(48000, 2, 16);
        for key in [9u32, 10, 12] {
            m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, key, 0);
        }
        m.apply(Command::StopRange { lo_key: 10, hi_key: 12 });
        let live: Vec<u32> = m.voices.iter().filter(|v| v.active).map(|v| v.key).collect();
        assert_eq!(live, vec![9, 12]);
    }

    #[test]
    fn stop_range_with_an_empty_span_is_a_noop() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, 10, 0);
        m.apply(Command::StopRange { lo_key: 10, hi_key: 10 });
        assert_eq!(active_count(&m), 1);
    }

    #[test]
    fn stop_range_covers_one_whole_sample_id_channel_span() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(5, 1.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 2.0, channel_key(5, 2.0), 0);
        m.play(flat(1000, 1, 48000, 0.2), 1.0, 0.0, 1.0, channel_key(6, 1.0), 0);
        assert_eq!(active_count(&m), 3);
        m.apply(Command::StopRange { lo_key: 5 * CHANNELS_PER_SAMPLE_ID, hi_key: 6 * CHANNELS_PER_SAMPLE_ID });
        assert_eq!(active_count(&m), 1);
        assert_eq!(channel_sample_id(m.voices.iter().find(|v| v.active).unwrap().key), 6);
    }
}

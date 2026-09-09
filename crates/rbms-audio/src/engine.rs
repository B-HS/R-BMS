use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering, fence};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize, SupportedStreamConfigRange};

use crate::AudioError;
use crate::decode::{DecodedAudio, decode_bytes};
use crate::mixer::{Bus, CHANNELS_PER_SAMPLE_ID, Command, MixStats, Mixer, SampleData, channel_key};

/// Default polyphony. The reference implementation's `deviceSimultaneousSources` default is 256
/// (`AudioConfig.java:28`); rbms doubles that for slack, since a full pool still has to take a slot
/// from a sounding voice once every slot is busy.
pub const DEFAULT_MAX_VOICES: usize = 512;

/// A pool of zero voices would swallow every sound the mixer is asked to start, so a requested
/// polyphony is raised to at least this.
const MIN_MAX_VOICES: usize = 1;

/// Command ring capacity. One slot per queued play/stop; overflow is counted, never blocking.
const COMMAND_QUEUE_CAPACITY: usize = 8192;

/// Frames the callback scratch buffer is pre-sized for, so the steady state performs no
/// allocation inside the real-time callback. Device buffers are typically 128-2048 frames
/// (the reference implementation's `deviceBufferSize` default is 384, `AudioConfig.java:24`).
const SCRATCH_PREALLOC_FRAMES: usize = 4096;

/// Reader retries when sampling a seqlock-published record written by the audio callback.
const CLOCK_SNAPSHOT_ATTEMPTS: usize = 4;

/// Sequence step per callback write: the counter goes odd while the record is being written and
/// back to even once every field is stored, so a reader can tell a torn record apart.
const CLOCK_SEQ_STEP: u64 = 1;

/// Buffer size assumed until the first callback reports the real one. 512 frames is what CoreAudio
/// hands out for `BufferSize::Default` on the development machine; it only shapes the lookahead and
/// the extrapolation cap during the few milliseconds before audio starts flowing.
const ESTIMATED_DEFAULT_BUFFER_FRAMES: u32 = 512;

/// Extra frames added to the measured buffer size when deriving the scheduling lookahead: one so a
/// command landing exactly on a buffer boundary still schedules into the future rather than
/// collapsing onto the current frame, and one to absorb the frame/µs truncation that happens on the
/// way out of the engine and again on the way back into the mixer.
const LOOKAHEAD_EXTRA_FRAMES: u32 = 2;

/// How far past the moment this callback's data starts sounding the interpolated clock may run
/// before it stops advancing. Bounds the damage when the audio thread stalls: the clock freezes
/// instead of drifting away from the audio that is actually playing.
const MAX_EXTRAPOLATION_BUFFERS: u32 = 3;

/// Playback leads above this are treated as a backend reporting error rather than real latency,
/// and the buffer-derived estimate is used instead.
const MAX_PLAYBACK_AHEAD_MS: u64 = 200;

/// Callback-to-callback gap, relative to the period the previous buffer covered, above which a
/// dropout is assumed. cpal exposes no xrun signal, so this is an estimate.
const UNDERRUN_GAP_RATIO: f64 = 1.5;

/// Ladder rung: the requested device with the requested sample rate and buffer size.
const STEP_REQUESTED: u8 = 0;

/// Ladder rung: the requested device and sample rate, letting the backend choose the buffer size.
const STEP_DEFAULT_BUFFER: u8 = 1;

/// Ladder rung: the requested device with its own default configuration.
const STEP_DEVICE_DEFAULT_CONFIG: u8 = 2;

/// Ladder rung: the system default device with its own default configuration.
const STEP_DEFAULT_DEVICE: u8 = 3;

/// Ladder rung: every enumerated output device in turn, each with its own default configuration.
const STEP_ANY_DEVICE: u8 = 4;

/// Shown when a backend refuses to describe a device it just handed out.
const UNKNOWN_DEVICE_NAME: &str = "unknown device";

/// Reported when output devices exist but none of them advertised a configuration to try.
const NO_USABLE_CONFIG: &str = "no output device advertised a usable configuration";

/// How to open the output stream. Every field is a request, not a guarantee: `AudioEngine::open`
/// walks a fallback ladder and reports what it actually got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOptions {
    pub device_name: Option<String>,
    pub sample_rate: Option<u32>,
    pub buffer_frames: Option<u32>,
    pub max_voices: usize,
}

impl Default for AudioOptions {
    fn default() -> Self {
        AudioOptions { device_name: None, sample_rate: None, buffer_frames: None, max_voices: DEFAULT_MAX_VOICES }
    }
}

/// What the opened stream actually is, plus one line per downgrade taken to get there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOpenReport {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer: BufferSize,
    pub fallback_step: u8,
    pub notes: Vec<String>,
}

/// The clock state the audio callback last published, read atomically in one seqlock pass.
#[derive(Clone, Copy, Debug)]
pub struct ClockSnapshot {
    pub frames_at_callback_start: u64,
    pub frames_rendered: u64,
    pub callback_at: Instant,
    pub playback_ahead: Duration,
    pub buffer_frames: u32,
}

/// The three play positions a host needs, all derived from one [`ClockSnapshot`] so they agree with
/// each other: what is audible now, how far ahead a scheduler must book, and the gap between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioClocks {
    pub audible_us: i64,
    pub scheduled_us: i64,
    pub lookahead_us: i64,
}

/// A half-open span of sample ids, and therefore of mixer channel keys. Keeping chart keysounds and
/// song-select previews in separate spans lets one engine serve both without id collisions, the way
/// the reference implementation reserves `65536+` for judge sounds (`AbstractAudioDriver.java:493-503`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdNamespace {
    pub base: u32,
    pub len: u32,
}

impl IdNamespace {
    /// Chart keysound ids, i.e. `#WAVxx` indices.
    pub const PLAY: IdNamespace = IdNamespace { base: 0, len: 0x0001_0000 };

    /// Song-select preview ids: the file preview clip at `base`, autoplay preview keysounds above it.
    pub const PREVIEW: IdNamespace = IdNamespace { base: 0x0080_0000, len: 0x0001_0000 };

    /// Whether `id` belongs to this namespace.
    pub fn contains(&self, id: u32) -> bool {
        id.checked_sub(self.base).is_some_and(|offset| offset < self.len)
    }
}

/// Clamp an interpolated clock reading so it never steps backwards. The caller owns `previous`,
/// which keeps the engine side a pure function of the published callback state.
pub fn monotonic_us(previous: i64, current: i64) -> i64 {
    current.max(previous)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ClockState {
    frames_start: u64,
    frames_end: u64,
    callback_nanos: u64,
    playback_ahead_nanos: u64,
    buffer_frames: u32,
}

struct ClockCell {
    seq: AtomicU64,
    frames_start: AtomicU64,
    frames_end: AtomicU64,
    callback_nanos: AtomicU64,
    playback_ahead_nanos: AtomicU64,
    buffer_frames: AtomicU32,
}

impl ClockCell {
    fn new(state: ClockState) -> Self {
        ClockCell {
            seq: AtomicU64::new(0),
            frames_start: AtomicU64::new(state.frames_start),
            frames_end: AtomicU64::new(state.frames_end),
            callback_nanos: AtomicU64::new(state.callback_nanos),
            playback_ahead_nanos: AtomicU64::new(state.playback_ahead_nanos),
            buffer_frames: AtomicU32::new(state.buffer_frames),
        }
    }
}

struct StatsCell {
    seq: AtomicU64,
    active_voices: AtomicU32,
    steals: AtomicU64,
    hard_steals: AtomicU64,
    late_schedules: AtomicU64,
}

impl StatsCell {
    fn new() -> Self {
        StatsCell {
            seq: AtomicU64::new(0),
            active_voices: AtomicU32::new(0),
            steals: AtomicU64::new(0),
            hard_steals: AtomicU64::new(0),
            late_schedules: AtomicU64::new(0),
        }
    }
}

struct Telemetry {
    alive: AtomicBool,
    dropped_commands: AtomicU64,
    scratch_reallocations: AtomicU64,
    underruns: AtomicU64,
    timestamp_fallbacks: AtomicU64,
    retire_overflows: AtomicU64,
    clock: ClockCell,
    stats: StatsCell,
}

impl Telemetry {
    fn new(seed_buffer_frames: u32, out_rate: u32) -> Self {
        let seed = ClockState {
            frames_start: 0,
            frames_end: 0,
            callback_nanos: 0,
            playback_ahead_nanos: frames_to_duration(seed_buffer_frames, out_rate).as_nanos() as u64,
            buffer_frames: seed_buffer_frames,
        };
        Telemetry {
            alive: AtomicBool::new(true),
            dropped_commands: AtomicU64::new(0),
            scratch_reallocations: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            timestamp_fallbacks: AtomicU64::new(0),
            retire_overflows: AtomicU64::new(0),
            clock: ClockCell::new(seed),
            stats: StatsCell::new(),
        }
    }
}

/// Owns the cpal output stream, the RT-safe command queue, the sample-derived master clock, and the
/// loaded keysound bank. The play-position clock is `samples_played / rate` interpolated with the
/// wall clock, NOT the frame/vsync clock — this is the timing-source fix vs the reference implementation.
pub struct AudioEngine {
    _stream: cpal::Stream,
    retired: rtrb::Consumer<Arc<SampleData>>,
    producer: rtrb::Producer<Command>,
    telemetry: Arc<Telemetry>,
    clock_epoch: Instant,
    out_rate: u32,
    bank: HashMap<u32, Arc<SampleData>>,
    last_clock: Cell<ClockState>,
    last_stats: Cell<MixStats>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        Ok(Self::open(&AudioOptions::default())?.0)
    }

    pub fn with_max_voices(max_voices: usize) -> Result<Self, AudioError> {
        Ok(Self::open(&AudioOptions { max_voices, ..AudioOptions::default() })?.0)
    }

    /// Open an output stream, walking a fallback ladder when the request cannot be honoured: the
    /// requested device and format, then a backend-chosen buffer, then the device default, then the
    /// system default device, then any enumerated output device. The report says which rung won and
    /// why the earlier ones were skipped.
    pub fn open(opts: &AudioOptions) -> Result<(AudioEngine, AudioOpenReport), AudioError> {
        let host = cpal::default_host();
        let mut notes: Vec<String> = Vec::new();
        let mut saw_device = false;

        let named = match opts.device_name.as_deref() {
            Some(name) => {
                let found = find_output_device(&host, name);
                if found.is_none() {
                    notes.push(format!("audio device \"{name}\" is not available, falling back to the system default"));
                }
                found
            }
            None => None,
        };
        let asked_for_a_device = opts.device_name.is_some();
        let used_named = named.is_some();
        let floor = reported_step_floor(asked_for_a_device && !used_named);

        if let Some(device) = named.or_else(|| host.default_output_device()) {
            saw_device = true;
            if let Some((format, rungs)) = requested_rungs(&device, opts, &mut notes)
                && let Some(opened) = open_first_rung(&device, rungs, format, opts, floor, &mut notes)
            {
                return Ok(opened);
            }
        }

        if used_named && let Some(device) = host.default_output_device() {
            saw_device = true;
            if let Some((format, rungs)) = default_config_rung(&device, STEP_DEFAULT_DEVICE)
                && let Some(opened) = open_first_rung(&device, rungs, format, opts, STEP_DEFAULT_DEVICE, &mut notes)
            {
                return Ok(opened);
            }
        }

        if let Ok(devices) = host.output_devices() {
            for device in devices {
                saw_device = true;
                if let Some((format, rungs)) = default_config_rung(&device, STEP_ANY_DEVICE)
                    && let Some(opened) = open_first_rung(&device, rungs, format, opts, STEP_ANY_DEVICE, &mut notes)
                {
                    return Ok(opened);
                }
            }
        }

        if !saw_device {
            return Err(AudioError::NoDevice);
        }
        if notes.is_empty() {
            notes.push(NO_USABLE_CONFIG.to_string());
        }
        Err(AudioError::Stream(notes.join("; ")))
    }

    pub fn out_rate(&self) -> u32 {
        self.out_rate
    }

    pub fn clock_frames(&self) -> u64 {
        self.telemetry.clock.frames_end.load(Ordering::Acquire)
    }

    /// Quantised play position: whole frames rendered so far. Steps once per callback, so it is a
    /// debug and regression reference rather than the clock the game should follow.
    pub fn clock_us(&self) -> i64 {
        frames_to_us(self.clock_frames(), self.out_rate)
    }

    /// Everything the last callback published about the clock, read as one consistent record.
    pub fn snapshot(&self) -> ClockSnapshot {
        let state = read_clock_state(&self.telemetry.clock, &self.last_clock);
        ClockSnapshot {
            frames_at_callback_start: state.frames_start,
            frames_rendered: state.frames_end,
            callback_at: self.clock_epoch + Duration::from_nanos(state.callback_nanos),
            playback_ahead: Duration::from_nanos(state.playback_ahead_nanos),
            buffer_frames: state.buffer_frames,
        }
    }

    /// Position the DAC is sounding at `now`, in µs, interpolated between callbacks. This is the
    /// axis the player hears, so judgement, note rendering and replay recording all use it.
    pub fn audible_us(&self, now: Instant) -> i64 {
        audible_us_from(&self.snapshot(), now, self.out_rate)
    }

    /// Position a scheduler must target so its command is seen by a callback that has not started
    /// rendering yet: [`audible_us`](Self::audible_us) plus [`lookahead_us`](Self::lookahead_us).
    /// Sounds that must be immediate (a pressed keysound) use `audible_us` instead.
    ///
    /// Correct only for a host that books sounds continuously. One that books them once per display
    /// frame has to add its own polling interval, and should read [`clocks`](Self::clocks) so it can
    /// keep its own dead-stream fallback on the same snapshot.
    pub fn scheduled_us(&self, now: Instant) -> i64 {
        self.clocks(now).scheduled_us
    }

    /// The device lead a scheduled sound needs: how long this callback's data still has to travel
    /// before it is heard, plus the buffer the next callback will render, plus a frame so a command
    /// landing exactly on a boundary still schedules into the future. A host that only books sounds
    /// once per display frame has to add its own polling interval on top.
    pub fn lookahead_us(&self) -> i64 {
        lookahead_us_from(&self.snapshot(), self.out_rate)
    }

    /// The audible position, the scheduling position and the lead between them, all derived from a
    /// single snapshot. Reading them separately can mix two callbacks' records, which breaks the
    /// identity `scheduled - audible == lookahead` the caller relies on.
    pub fn clocks(&self, now: Instant) -> AudioClocks {
        let snapshot = self.snapshot();
        let audible_us = audible_us_from(&snapshot, now, self.out_rate);
        let lookahead_us = lookahead_us_from(&snapshot, self.out_rate);
        AudioClocks { audible_us, scheduled_us: audible_us + lookahead_us, lookahead_us }
    }

    /// `false` once the device reported a stream error (device unplugged, format change, driver
    /// reset). The audio clock stops advancing at that point, so a host should fall back to a
    /// wall-clock timebase rather than waiting on frames that will never arrive.
    pub fn is_alive(&self) -> bool {
        self.telemetry.alive.load(Ordering::Acquire)
    }

    /// Commands dropped because the RT command ring was full. Non-zero means keysounds were
    /// silently skipped; it is a load signal, not a fatal condition.
    pub fn dropped_commands(&self) -> u64 {
        self.telemetry.dropped_commands.load(Ordering::Relaxed)
    }

    /// Times the audio callback had to grow its scratch buffer, i.e. allocated on the real-time
    /// thread. Expected to stay 0 after the first callback thanks to the pre-allocation.
    pub fn scratch_reallocations(&self) -> u64 {
        self.telemetry.scratch_reallocations.load(Ordering::Relaxed)
    }

    /// Dropouts: every recoverable [`cpal::StreamError::BufferUnderrun`] the backend reported, plus
    /// the callbacks that arrived later than the previous buffer could cover. Backends that raise no
    /// underrun signal at all (CoreAudio) are only covered by the second, estimated half, which is
    /// why the overlay labels the figure as an estimate.
    pub fn underruns(&self) -> u64 {
        self.telemetry.underruns.load(Ordering::Relaxed)
    }

    /// Callbacks whose backend timestamp was missing or implausible, so the buffer-derived playback
    /// lead was used instead. A steady non-zero rate is normal on some backends.
    pub fn timestamp_fallbacks(&self) -> u64 {
        self.telemetry.timestamp_fallbacks.load(Ordering::Relaxed)
    }

    /// Samples the audio callback had to free itself because the retirement ring was full, i.e.
    /// allocator work on the real-time thread. Expected to stay 0; a non-zero value means the game
    /// thread is not draining the ring often enough.
    pub fn retire_overflows(&self) -> u64 {
        self.telemetry.retire_overflows.load(Ordering::Relaxed)
    }

    /// Mixer telemetry published by the last callback, read as one consistent record.
    pub fn mix_stats(&self) -> MixStats {
        read_mix_stats(&self.telemetry.stats, &self.last_stats)
    }

    pub fn load(&mut self, id: u32, bytes: Vec<u8>, ext: Option<&str>) -> Result<(), AudioError> {
        let dec = decode_bytes(bytes, ext)?;
        self.insert_decoded(id, dec);
        Ok(())
    }

    /// Insert an already-decoded sample into the keysound bank. Same effect as [`load`](Self::load)
    /// but skips the (CPU-heavy) symphonia decode, which the caller has already done — lets a host
    /// decode many keysounds in parallel off-thread, then insert the results here on one thread.
    pub fn insert_decoded(&mut self, id: u32, audio: DecodedAudio) {
        self.collect_retired();
        let sd = SampleData { pcm: audio.samples.into(), channels: audio.channels, rate: audio.rate };
        self.bank.insert(id, Arc::new(sd));
    }

    pub fn loaded(&self) -> usize {
        self.bank.len()
    }

    pub fn has_sample(&self, id: u32) -> bool {
        self.bank.contains_key(&id)
    }

    /// Duration of a loaded sample in µs (frames / rate), for scheduling a looped re-trigger. `None`
    /// if the id is not loaded.
    pub fn sample_duration_us(&self, id: u32) -> Option<i64> {
        self.bank.get(&id).map(|s| (s.frames() as i128 * 1_000_000 / s.rate.max(1) as i128) as i64)
    }

    pub fn play(&mut self, id: u32, gain: f32, pan: f32, pitch: f32, at_us: i64) {
        self.play_on(Bus::Key, id, gain, pan, pitch, at_us);
    }

    /// Schedule a sample on `bus`. `at_us` is an absolute position on the engine clock; `0` means
    /// "as soon as the next callback runs", which is what an immediate keysound wants.
    pub fn play_on(&mut self, bus: Bus, id: u32, gain: f32, pan: f32, pitch: f32, at_us: i64) {
        if let Some(sample) = self.bank.get(&id).cloned() {
            let at_frame = (at_us.max(0) as i128 * self.out_rate as i128 / 1_000_000) as u64;
            let key = channel_key(id, pitch);
            self.push(Command::Play { sample, gain, pan, pitch, key, at_frame, bus });
        }
    }

    /// Stop every voice of a sample id, whatever pitch it was started at.
    pub fn stop(&mut self, id: u32) {
        self.push(Command::StopId { id });
    }

    /// Stop only the voice of `id` started at `pitch`, leaving other pitch-shifted copies of the
    /// same sample playing — the reference implementation's per-channel `stop` (`AbstractAudioDriver.java:507-527`).
    pub fn stop_pitched(&mut self, id: u32, pitch: f32) {
        self.push(Command::Stop { key: channel_key(id, pitch) });
    }

    pub fn set_master_gain(&mut self, g: f32) {
        self.push(Command::MasterGain(g));
    }

    /// Per-bus gain, mirroring the reference implementation's `systemvolume` / `keyvolume` / `bgvolume`.
    pub fn set_bus_gain(&mut self, bus: Bus, g: f32) {
        self.push(Command::BusGain { bus, gain: g });
    }

    /// Chart-wide gain from `#VOLWAV` / 100, set once per chart and applied to every bus.
    pub fn set_chart_gain(&mut self, g: f32) {
        self.push(Command::ChartGain(g));
    }

    /// Drop every bank entry in `ns` and stop the voices playing them, so one long-lived engine can
    /// hand its slots from a chart to a preview and back without the bank growing without bound.
    pub fn clear_namespace(&mut self, ns: IdNamespace) {
        self.bank.retain(|id, _| !ns.contains(*id));
        let (lo_key, hi_key) = namespace_key_range(ns);
        self.push(Command::StopRange { lo_key, hi_key });
    }

    fn push(&mut self, cmd: Command) {
        self.collect_retired();
        push_or_count(&mut self.producer, &self.telemetry, cmd);
    }

    /// Drop the samples the mixer handed back after their voices ended. Freeing them here keeps the
    /// deallocation off the audio callback, which must not touch the allocator.
    pub fn collect_retired(&mut self) {
        while self.retired.pop().is_ok() {}
    }
}

/// Queue a command for the audio callback, counting it as dropped instead of blocking when the
/// ring is full. Never allocates and never waits, so it is safe to call from the game loop.
fn push_or_count(producer: &mut rtrb::Producer<Command>, telemetry: &Telemetry, cmd: Command) {
    if producer.push(cmd).is_err() {
        telemetry.dropped_commands.fetch_add(1, Ordering::Relaxed);
    }
}

/// Make the callback scratch buffer hold at least `needed` samples, counting every actual
/// allocation. Growth only: a smaller callback reuses the existing buffer instead of shrinking it,
/// which would otherwise reallocate on every alternating buffer size.
fn grow_scratch(scratch: &mut Vec<f32>, needed: usize, telemetry: &Telemetry) {
    if scratch.len() < needed {
        scratch.resize(needed, 0.0);
        telemetry.scratch_reallocations.fetch_add(1, Ordering::Relaxed);
    }
}

/// Open a seqlock write: the counter goes odd so a concurrent reader can tell the record is being
/// rewritten. Single writer only (the audio callback).
fn seq_write_begin(seq: &AtomicU64) -> u64 {
    let start = seq.load(Ordering::Relaxed);
    seq.store(start.wrapping_add(CLOCK_SEQ_STEP), Ordering::Relaxed);
    fence(Ordering::Release);
    start
}

/// Close a seqlock write opened by [`seq_write_begin`], publishing every field stored in between.
fn seq_write_end(seq: &AtomicU64, start: u64) {
    seq.store(start.wrapping_add(2 * CLOCK_SEQ_STEP), Ordering::Release);
}

/// Begin a seqlock read. `None` while a write is in flight.
fn seq_read_begin(seq: &AtomicU64) -> Option<u64> {
    let before = seq.load(Ordering::Acquire);
    if before.is_multiple_of(2) { Some(before) } else { None }
}

/// Whether the fields read since [`seq_read_begin`] belong to a single published record.
fn seq_read_valid(seq: &AtomicU64, before: u64) -> bool {
    fence(Ordering::Acquire);
    seq.load(Ordering::Relaxed) == before
}

/// Publish the whole clock record from the audio callback as one seqlock write.
fn write_clock_state(cell: &ClockCell, state: ClockState) {
    let start = seq_write_begin(&cell.seq);
    cell.frames_start.store(state.frames_start, Ordering::Relaxed);
    cell.frames_end.store(state.frames_end, Ordering::Relaxed);
    cell.callback_nanos.store(state.callback_nanos, Ordering::Relaxed);
    cell.playback_ahead_nanos.store(state.playback_ahead_nanos, Ordering::Relaxed);
    cell.buffer_frames.store(state.buffer_frames, Ordering::Relaxed);
    seq_write_end(&cell.seq, start);
}

/// One seqlock read attempt. `None` means the callback was mid-write or completed a write while the
/// fields were being read, i.e. the record would have been torn.
fn try_read_clock_state(cell: &ClockCell) -> Option<ClockState> {
    let before = seq_read_begin(&cell.seq)?;
    let state = ClockState {
        frames_start: cell.frames_start.load(Ordering::Relaxed),
        frames_end: cell.frames_end.load(Ordering::Relaxed),
        callback_nanos: cell.callback_nanos.load(Ordering::Relaxed),
        playback_ahead_nanos: cell.playback_ahead_nanos.load(Ordering::Relaxed),
        buffer_frames: cell.buffer_frames.load(Ordering::Relaxed),
    };
    if seq_read_valid(&cell.seq, before) { Some(state) } else { None }
}

/// Read back a record published by [`write_clock_state`]. Retries a torn read up to
/// [`CLOCK_SNAPSHOT_ATTEMPTS`] times, then returns the last record that was read whole. Reading the
/// fields individually instead would mix two callbacks — a new frame count next to an old callback
/// instant makes the interpolated position jump — and the caller must never block on an audio
/// thread that stalled mid-write.
fn read_clock_state(cell: &ClockCell, cache: &Cell<ClockState>) -> ClockState {
    for _ in 0..CLOCK_SNAPSHOT_ATTEMPTS {
        if let Some(state) = try_read_clock_state(cell) {
            cache.set(state);
            return state;
        }
        std::hint::spin_loop();
    }
    cache.get()
}

/// Publish the mixer's counters from the audio callback as one seqlock write, so the overlay never
/// shows a voice count from one callback next to a steal count from another.
fn write_mix_stats(cell: &StatsCell, stats: MixStats) {
    let start = seq_write_begin(&cell.seq);
    cell.active_voices.store(stats.active_voices, Ordering::Relaxed);
    cell.steals.store(stats.steals, Ordering::Relaxed);
    cell.hard_steals.store(stats.hard_steals, Ordering::Relaxed);
    cell.late_schedules.store(stats.late_schedules, Ordering::Relaxed);
    seq_write_end(&cell.seq, start);
}

/// One seqlock read attempt for the mixer counters. `None` on a torn read.
fn try_read_mix_stats(cell: &StatsCell) -> Option<MixStats> {
    let before = seq_read_begin(&cell.seq)?;
    let stats = MixStats {
        active_voices: cell.active_voices.load(Ordering::Relaxed),
        steals: cell.steals.load(Ordering::Relaxed),
        hard_steals: cell.hard_steals.load(Ordering::Relaxed),
        late_schedules: cell.late_schedules.load(Ordering::Relaxed),
    };
    if seq_read_valid(&cell.seq, before) { Some(stats) } else { None }
}

/// Read back the counters published by [`write_mix_stats`], with the same retry-then-last-whole-read
/// policy as the clock record.
fn read_mix_stats(cell: &StatsCell, cache: &Cell<MixStats>) -> MixStats {
    for _ in 0..CLOCK_SNAPSHOT_ATTEMPTS {
        if let Some(stats) = try_read_mix_stats(cell) {
            cache.set(stats);
            return stats;
        }
        std::hint::spin_loop();
    }
    cache.get()
}

fn frames_to_us(frames: u64, out_rate: u32) -> i64 {
    (frames as i128 * 1_000_000 / out_rate.max(1) as i128) as i64
}

fn frames_to_duration(frames: u32, out_rate: u32) -> Duration {
    Duration::from_nanos(frames as u64 * 1_000_000_000 / out_rate.max(1) as u64)
}

/// How far past its own callback instant the interpolated clock may run before it stops advancing.
fn max_extrapolation(buffer_frames: u32, out_rate: u32) -> Duration {
    frames_to_duration(buffer_frames.saturating_mul(MAX_EXTRAPOLATION_BUFFERS), out_rate)
}

/// Microseconds from `from` to `to`, negative when `to` is the earlier instant.
fn signed_micros_between(from: Instant, to: Instant) -> i64 {
    match to.checked_duration_since(from) {
        Some(elapsed) => elapsed.as_micros() as i64,
        None => -(from.saturating_duration_since(to).as_micros() as i64),
    }
}

/// Interpolated audible position in µs. The base is the frame count at the *start* of the published
/// callback, because `playback` is when that callback's first frame reaches the DAC; using the
/// post-render count would run one whole buffer ahead of what is being heard.
///
/// The offset from that base is a *signed* difference. Every backend reports a playback lead of at
/// least one buffer period, so clamping the difference at zero would hold the reading flat for the
/// whole callback period and collapse the clock back into the per-callback staircase this
/// interpolation exists to remove. Going negative is correct: the buffer just handed to the device
/// has not reached the DAC yet, so what is audible is still inside the previous one. The result is
/// continuous across callbacks, because the next record's base is exactly one buffer higher.
fn audible_us_from(snapshot: &ClockSnapshot, now: Instant, out_rate: u32) -> i64 {
    let base = frames_to_us(snapshot.frames_at_callback_start, out_rate);
    let frozen_at = snapshot.callback_at.checked_add(max_extrapolation(snapshot.buffer_frames, out_rate)).unwrap_or(snapshot.callback_at);
    let audible_at = snapshot.callback_at.checked_add(snapshot.playback_ahead).unwrap_or(snapshot.callback_at);
    base + signed_micros_between(audible_at, now.min(frozen_at))
}

/// Head start a scheduled sound needs, measured from the audible position: the lead this callback's
/// data still has before it sounds, plus the buffer the next callback renders (the first one that
/// can see the command), plus the frames that absorb the boundary case and the microsecond/frame
/// truncation. The worst case it has to cover is a command pushed the instant a callback returned:
/// it is not seen until the next one, which by then has already rendered through `frames_rendered`.
/// A host that only books sounds once per display frame adds its own polling interval on top.
fn lookahead_us_from(snapshot: &ClockSnapshot, out_rate: u32) -> i64 {
    if out_rate == 0 || snapshot.buffer_frames == 0 {
        return 0;
    }
    let device_lead_us = snapshot.playback_ahead.as_micros() as i64;
    device_lead_us + frames_to_us(snapshot.buffer_frames.saturating_add(LOOKAHEAD_EXTRA_FRAMES) as u64, out_rate)
}

/// Whether the backend's `playback - callback` lead can be trusted. Backends that report `playback`
/// before `callback`, or an implausibly long lead, fall back to the buffer-derived estimate.
fn ahead_is_usable(reported: Option<Duration>) -> bool {
    reported.is_some_and(|lead| lead <= Duration::from_millis(MAX_PLAYBACK_AHEAD_MS))
}

/// The playback lead to publish: the backend's own figure when it is usable, otherwise the time one
/// device buffer takes to play.
fn ahead_or_fallback(reported: Option<Duration>, buffer_frames: u32, out_rate: u32) -> Duration {
    match reported {
        Some(lead) if ahead_is_usable(Some(lead)) => lead,
        _ => frames_to_duration(buffer_frames, out_rate),
    }
}

/// Whether a callback gap is long enough to assume the device ran dry while waiting for it.
fn is_underrun_gap(gap: Duration, previous_buffer_frames: u32, out_rate: u32) -> bool {
    if previous_buffer_frames == 0 || out_rate == 0 {
        return false;
    }
    let expected = previous_buffer_frames as f64 / out_rate as f64;
    gap.as_secs_f64() > expected * UNDERRUN_GAP_RATIO
}

/// Retirement-ring slots per voice slot. A voice can only hand back one sample at a time, so this
/// many turnovers per voice can pile up between two game-thread drains before the callback has to
/// free one itself.
const RETIRE_SLOTS_PER_VOICE: usize = 4;

/// Floor on the retirement ring, so a tiny voice pool still absorbs a burst of stopped sounds.
const MIN_RETIRE_QUEUE_CAPACITY: usize = 1024;

fn retire_queue_capacity(max_voices: usize) -> usize {
    max_voices.saturating_mul(RETIRE_SLOTS_PER_VOICE).max(MIN_RETIRE_QUEUE_CAPACITY)
}

/// Whether a stream error means the stream is gone for good. cpal reports a recoverable dropout as
/// [`cpal::StreamError::BufferUnderrun`] — ALSA raises it after recovering from an xrun
/// (`host/alsa/mod.rs:807`, `:853`, `:1045`) and JACK on an overrun (`host/jack/stream.rs:468`) —
/// and the stream keeps producing audio afterwards. Treating that as death would strand the game on
/// the wall clock for the rest of the session while the device is still sounding.
fn stream_error_is_fatal(error: &cpal::StreamError) -> bool {
    !matches!(error, cpal::StreamError::BufferUnderrun)
}

/// Mixer channel keys covered by a namespace, as the half-open range `Command::StopRange` expects.
fn namespace_key_range(ns: IdNamespace) -> (u32, u32) {
    let lo = ns.base.wrapping_mul(CHANNELS_PER_SAMPLE_ID);
    let hi = ns.base.wrapping_add(ns.len).wrapping_mul(CHANNELS_PER_SAMPLE_ID);
    (lo, hi)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PickedConfig {
    sample_rate: u32,
    buffer: BufferSize,
    sample_rate_adjusted: bool,
    buffer_adjusted: bool,
}

/// Resolve the sample rate and buffer size to ask a device for, from what it advertises and what the
/// user asked for. Pure and device independent, so the ladder's first rung is unit testable:
/// `device_default_rate` stands in for the rate the backend would pick on its own when the request
/// says AUTO. `None` only when the device advertises no output configuration at all.
fn pick_config(supported: &[SupportedStreamConfigRange], req: &AudioOptions, device_default_rate: u32) -> Option<PickedConfig> {
    let wanted_rate = req.sample_rate.unwrap_or(device_default_rate);
    let range = supported
        .iter()
        .filter(|r| r.min_sample_rate() <= wanted_rate && wanted_rate <= r.max_sample_rate())
        .max_by(|a, b| a.cmp_default_heuristics(b))
        .or_else(|| supported.iter().max_by(|a, b| a.cmp_default_heuristics(b)))?;

    let sample_rate = wanted_rate.clamp(range.min_sample_rate(), range.max_sample_rate());
    let sample_rate_adjusted = req.sample_rate.is_some_and(|asked| asked != sample_rate);
    let (buffer, buffer_adjusted) = match (req.buffer_frames, range.buffer_size()) {
        (None, _) => (BufferSize::Default, false),
        (Some(asked), SupportedBufferSize::Range { min, max }) => {
            let clamped = asked.clamp(*min, *max);
            (BufferSize::Fixed(clamped), clamped != asked)
        }
        (Some(_), SupportedBufferSize::Unknown) => (BufferSize::Default, true),
    };
    Some(PickedConfig { sample_rate, buffer, sample_rate_adjusted, buffer_adjusted })
}

fn describe_buffer(buffer: BufferSize) -> String {
    match buffer {
        BufferSize::Fixed(frames) => format!("{frames} frames"),
        BufferSize::Default => "the device default buffer".to_string(),
    }
}

/// One user-facing line per part of the request the device could not honour verbatim.
fn pick_notes(req: &AudioOptions, pick: &PickedConfig) -> Vec<String> {
    let mut notes = Vec::new();
    if pick.sample_rate_adjusted
        && let Some(asked) = req.sample_rate
    {
        notes.push(format!("requested {asked} Hz -> device supports {} Hz", pick.sample_rate));
    }
    if pick.buffer_adjusted
        && let Some(asked) = req.buffer_frames
    {
        notes.push(format!("requested a {asked} frame buffer -> {}", describe_buffer(pick.buffer)));
    }
    notes
}

/// Frames the callback scratch buffer starts out sized for, and the buffer size assumed until the
/// first callback measures the real one.
fn seed_buffer_frames(config: &StreamConfig) -> u32 {
    match config.buffer_size {
        BufferSize::Fixed(frames) => frames.max(1),
        BufferSize::Default => ESTIMATED_DEFAULT_BUFFER_FRAMES,
    }
}

fn scratch_prealloc_samples(config: &StreamConfig) -> usize {
    let frames = match config.buffer_size {
        BufferSize::Fixed(n) => (n as usize).max(SCRATCH_PREALLOC_FRAMES),
        BufferSize::Default => SCRATCH_PREALLOC_FRAMES,
    };
    frames * config.channels.max(1) as usize
}

fn device_display_name(device: &cpal::Device) -> String {
    device.description().map(|d| d.name().to_string()).unwrap_or_else(|_| UNKNOWN_DEVICE_NAME.to_string())
}

fn find_output_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    host.output_devices().ok()?.find(|device| device_display_name(device) == name)
}

/// The rungs tried on the device the user asked for: the full request, then the request with a
/// backend-chosen buffer, then the device's own default configuration.
fn requested_rungs(device: &cpal::Device, opts: &AudioOptions, notes: &mut Vec<String>) -> Option<(SampleFormat, Vec<(u8, StreamConfig)>)> {
    let default_config = device.default_output_config().ok()?;
    let supported: Vec<SupportedStreamConfigRange> = device.supported_output_configs().map(|it| it.collect()).unwrap_or_default();
    let mut rungs: Vec<(u8, StreamConfig)> = Vec::new();
    if let Some(pick) = pick_config(&supported, opts, default_config.sample_rate()) {
        notes.extend(pick_notes(opts, &pick));
        let channels = default_config.channels();
        rungs.push((STEP_REQUESTED, StreamConfig { channels, sample_rate: pick.sample_rate, buffer_size: pick.buffer }));
        rungs.push((STEP_DEFAULT_BUFFER, StreamConfig { channels, sample_rate: pick.sample_rate, buffer_size: BufferSize::Default }));
    }
    rungs.push((STEP_DEVICE_DEFAULT_CONFIG, default_config.config()));
    rungs.dedup_by(|a, b| a.1 == b.1);
    Some((default_config.sample_format(), rungs))
}

/// Lowest ladder rung a report may claim. Losing the requested device is itself a downgrade, so a
/// stream opened on the system default after the named device went missing is never reported as the
/// request having been honoured, however well the rest of the format matched.
fn reported_step_floor(device_fell_back: bool) -> u8 {
    if device_fell_back { STEP_DEFAULT_DEVICE } else { STEP_REQUESTED }
}

/// The single rung tried on a fallback device: whatever the backend calls its default.
fn default_config_rung(device: &cpal::Device, step: u8) -> Option<(SampleFormat, Vec<(u8, StreamConfig)>)> {
    let config = device.default_output_config().ok()?;
    Some((config.sample_format(), vec![(step, config.config())]))
}

/// Try each rung in order, returning the first stream that opens together with the report describing
/// it. Failures are appended to `notes` so the user can see why a rung was skipped.
fn open_first_rung(
    device: &cpal::Device,
    rungs: Vec<(u8, StreamConfig)>,
    sample_format: SampleFormat,
    opts: &AudioOptions,
    step_floor: u8,
    notes: &mut Vec<String>,
) -> Option<(AudioEngine, AudioOpenReport)> {
    for (step, config) in rungs {
        match open_stream(device, &config, sample_format, opts.max_voices) {
            Ok(engine) => {
                let report = AudioOpenReport {
                    device_name: device_display_name(device),
                    sample_rate: config.sample_rate,
                    channels: config.channels,
                    buffer: config.buffer_size,
                    fallback_step: step.max(step_floor),
                    notes: std::mem::take(notes),
                };
                return Some((engine, report));
            }
            Err(err) => {
                notes.push(format!("{} at {} Hz with {} failed: {err}", device_display_name(device), config.sample_rate, describe_buffer(config.buffer_size)))
            }
        }
    }
    None
}

fn open_stream(device: &cpal::Device, config: &StreamConfig, sample_format: SampleFormat, max_voices: usize) -> Result<AudioEngine, AudioError> {
    let out_rate = config.sample_rate;
    let out_channels = config.channels;
    let (producer, consumer) = rtrb::RingBuffer::<Command>::new(COMMAND_QUEUE_CAPACITY);
    let voices = max_voices.max(MIN_MAX_VOICES);
    let (retire_tx, retired) = rtrb::RingBuffer::<Arc<SampleData>>::new(retire_queue_capacity(voices));
    let telemetry = Arc::new(Telemetry::new(seed_buffer_frames(config), out_rate));
    let seed_clock = read_clock_state(&telemetry.clock, &Cell::new(ClockState::default()));
    let clock_epoch = Instant::now();
    let mut mixer = Mixer::new(out_rate, out_channels, voices);
    mixer.set_retire(retire_tx);
    let scratch_samples = scratch_prealloc_samples(config);

    let ctx = StreamContext { mixer, consumer, telemetry: telemetry.clone(), clock_epoch, scratch_samples, out_channels, out_rate };
    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(device, config, ctx)?,
        SampleFormat::I16 => build_stream::<i16>(device, config, ctx)?,
        SampleFormat::U16 => build_stream::<u16>(device, config, ctx)?,
        other => return Err(AudioError::Unsupported(format!("sample format {other:?}"))),
    };
    stream.play().map_err(|e| AudioError::Stream(e.to_string()))?;

    Ok(AudioEngine {
        _stream: stream,
        retired,
        producer,
        telemetry,
        clock_epoch,
        out_rate,
        bank: HashMap::new(),
        last_clock: Cell::new(seed_clock),
        last_stats: Cell::new(MixStats::default()),
    })
}

struct StreamContext {
    mixer: Mixer,
    consumer: rtrb::Consumer<Command>,
    telemetry: Arc<Telemetry>,
    clock_epoch: Instant,
    scratch_samples: usize,
    out_channels: u16,
    out_rate: u32,
}

fn build_stream<T>(device: &cpal::Device, config: &StreamConfig, ctx: StreamContext) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample + FromSample<f32>,
{
    let StreamContext { mut mixer, mut consumer, telemetry, clock_epoch, scratch_samples, out_channels, out_rate } = ctx;
    let mut scratch: Vec<f32> = vec![0.0; scratch_samples];
    let mut previous_callback: Option<(Duration, u32)> = None;
    let error_telemetry = telemetry.clone();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                let entered_at = clock_epoch.elapsed();
                let buffer_frames = (data.len() / out_channels.max(1) as usize) as u32;
                if let Some((previous_at, previous_frames)) = previous_callback
                    && is_underrun_gap(entered_at.saturating_sub(previous_at), previous_frames, out_rate)
                {
                    telemetry.underruns.fetch_add(1, Ordering::Relaxed);
                }
                previous_callback = Some((entered_at, buffer_frames));

                while let Ok(cmd) = consumer.pop() {
                    mixer.apply(cmd);
                }
                let frames_start = mixer.clock_frames();
                grow_scratch(&mut scratch, data.len(), &telemetry);
                let buf = &mut scratch[..data.len()];
                mixer.mix(buf);
                for (o, s) in data.iter_mut().zip(buf.iter()) {
                    *o = T::from_sample(*s);
                }

                let timestamp = info.timestamp();
                let reported = timestamp.playback.duration_since(&timestamp.callback);
                if !ahead_is_usable(reported) {
                    telemetry.timestamp_fallbacks.fetch_add(1, Ordering::Relaxed);
                }
                let playback_ahead = ahead_or_fallback(reported, buffer_frames, out_rate);
                write_clock_state(
                    &telemetry.clock,
                    ClockState {
                        frames_start,
                        frames_end: mixer.clock_frames(),
                        callback_nanos: entered_at.as_nanos() as u64,
                        playback_ahead_nanos: playback_ahead.as_nanos() as u64,
                        buffer_frames,
                    },
                );
                write_mix_stats(&telemetry.stats, mixer.stats());
                telemetry.retire_overflows.store(mixer.retire_overflows(), Ordering::Relaxed);
            },
            move |e| {
                if stream_error_is_fatal(&e) {
                    error_telemetry.alive.store(false, Ordering::Release);
                } else {
                    error_telemetry.underruns.fetch_add(1, Ordering::Relaxed);
                }
            },
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(channels: u16, buffer_size: BufferSize) -> StreamConfig {
        StreamConfig { channels, sample_rate: 48000, buffer_size }
    }

    fn range(min_rate: u32, max_rate: u32, buffer: SupportedBufferSize) -> SupportedStreamConfigRange {
        SupportedStreamConfigRange::new(2, min_rate, max_rate, buffer, SampleFormat::F32)
    }

    fn bounded(min: u32, max: u32) -> SupportedBufferSize {
        SupportedBufferSize::Range { min, max }
    }

    fn snapshot(callback_at: Instant, frames_start: u64, playback_ahead: Duration, buffer_frames: u32) -> ClockSnapshot {
        ClockSnapshot {
            frames_at_callback_start: frames_start,
            frames_rendered: frames_start + buffer_frames as u64,
            callback_at,
            playback_ahead,
            buffer_frames,
        }
    }

    #[test]
    fn scratch_prealloc_covers_default_buffer_size() {
        assert_eq!(scratch_prealloc_samples(&config(2, BufferSize::Default)), 8192);
    }

    #[test]
    fn scratch_prealloc_never_shrinks_below_the_floor() {
        assert_eq!(scratch_prealloc_samples(&config(2, BufferSize::Fixed(384))), 8192);
    }

    #[test]
    fn scratch_prealloc_follows_large_fixed_buffer_size() {
        assert_eq!(scratch_prealloc_samples(&config(2, BufferSize::Fixed(8192))), 16384);
    }

    #[test]
    fn scratch_prealloc_scales_with_channel_count() {
        assert_eq!(scratch_prealloc_samples(&config(6, BufferSize::Default)), 24576);
    }

    #[test]
    fn scratch_prealloc_treats_zero_channels_as_one() {
        assert_eq!(scratch_prealloc_samples(&config(0, BufferSize::Default)), 4096);
    }

    #[test]
    fn seed_buffer_frames_follows_a_fixed_request() {
        assert_eq!(seed_buffer_frames(&config(2, BufferSize::Fixed(384))), 384);
    }

    #[test]
    fn seed_buffer_frames_estimates_a_backend_chosen_buffer() {
        assert_eq!(seed_buffer_frames(&config(2, BufferSize::Default)), ESTIMATED_DEFAULT_BUFFER_FRAMES);
    }

    #[test]
    fn seed_buffer_frames_never_reports_an_empty_buffer() {
        assert_eq!(seed_buffer_frames(&config(2, BufferSize::Fixed(0))), 1);
    }

    #[test]
    fn telemetry_starts_alive_with_zero_counters() {
        let t = Telemetry::new(512, 48000);
        assert!(t.alive.load(Ordering::Acquire));
        assert_eq!(t.dropped_commands.load(Ordering::Relaxed), 0);
        assert_eq!(t.scratch_reallocations.load(Ordering::Relaxed), 0);
        assert_eq!(t.underruns.load(Ordering::Relaxed), 0);
        assert_eq!(t.timestamp_fallbacks.load(Ordering::Relaxed), 0);
        assert_eq!(t.retire_overflows.load(Ordering::Relaxed), 0);
        assert_eq!(t.clock.frames_start.load(Ordering::Relaxed), 0);
        assert_eq!(t.clock.frames_end.load(Ordering::Relaxed), 0);
        assert_eq!(t.clock.callback_nanos.load(Ordering::Relaxed), 0);
        assert_eq!(read_mix_stats(&t.stats, &Cell::new(MixStats::default())), MixStats::default());
    }

    #[test]
    fn telemetry_seeds_the_buffer_size_before_the_first_callback() {
        let t = Telemetry::new(512, 48000);
        assert_eq!(t.clock.buffer_frames.load(Ordering::Relaxed), 512);
        assert_eq!(t.clock.playback_ahead_nanos.load(Ordering::Relaxed), 10_666_666);
    }

    #[test]
    fn full_command_ring_counts_drops_instead_of_blocking() {
        let telemetry = Telemetry::new(512, 48000);
        let (mut producer, mut consumer) = rtrb::RingBuffer::<Command>::new(2);
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.1));
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.2));
        assert_eq!(telemetry.dropped_commands.load(Ordering::Relaxed), 0);
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.3));
        assert_eq!(telemetry.dropped_commands.load(Ordering::Relaxed), 1);
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.4));
        assert_eq!(telemetry.dropped_commands.load(Ordering::Relaxed), 2);
        assert!(consumer.pop().is_ok());
    }

    #[test]
    fn drained_command_ring_accepts_pushes_again_without_counting_drops() {
        let telemetry = Telemetry::new(512, 48000);
        let (mut producer, mut consumer) = rtrb::RingBuffer::<Command>::new(2);
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.1));
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.2));
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.3));
        assert_eq!(telemetry.dropped_commands.load(Ordering::Relaxed), 1);
        assert!(consumer.pop().is_ok());
        push_or_count(&mut producer, &telemetry, Command::MasterGain(0.4));
        assert_eq!(telemetry.dropped_commands.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn scratch_growth_counts_one_reallocation_and_never_shrinks() {
        let telemetry = Telemetry::new(512, 48000);
        let mut scratch = vec![0.0f32; 4];
        grow_scratch(&mut scratch, 4, &telemetry);
        assert_eq!(scratch.len(), 4);
        assert_eq!(telemetry.scratch_reallocations.load(Ordering::Relaxed), 0);

        grow_scratch(&mut scratch, 10, &telemetry);
        assert_eq!(scratch.len(), 10);
        assert_eq!(telemetry.scratch_reallocations.load(Ordering::Relaxed), 1);

        grow_scratch(&mut scratch, 10, &telemetry);
        assert_eq!(telemetry.scratch_reallocations.load(Ordering::Relaxed), 1);

        grow_scratch(&mut scratch, 2, &telemetry);
        assert_eq!(scratch.len(), 10, "a smaller callback must reuse the buffer, not shrink it");
        assert_eq!(telemetry.scratch_reallocations.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn read_clock_state_returns_the_last_completed_record() {
        let cell = ClockCell::new(ClockState::default());
        let cache = Cell::new(ClockState::default());
        let first = ClockState { frames_start: 0, frames_end: 480, callback_nanos: 10_000_000, playback_ahead_nanos: 5_000_000, buffer_frames: 480 };
        write_clock_state(&cell, first);
        assert_eq!(read_clock_state(&cell, &cache), first);

        let second = ClockState { frames_start: 480, frames_end: 960, callback_nanos: 20_000_000, playback_ahead_nanos: 6_000_000, buffer_frames: 480 };
        write_clock_state(&cell, second);
        assert_eq!(read_clock_state(&cell, &cache), second);
    }

    #[test]
    fn try_read_clock_state_rejects_a_half_written_record() {
        let cell = ClockCell::new(ClockState::default());
        let published = ClockState { frames_start: 0, frames_end: 480, callback_nanos: 10_000_000, playback_ahead_nanos: 5_000_000, buffer_frames: 480 };
        write_clock_state(&cell, published);
        assert_eq!(try_read_clock_state(&cell), Some(published));

        cell.seq.store(1, Ordering::Relaxed);
        cell.frames_start.store(480, Ordering::Relaxed);
        assert_eq!(try_read_clock_state(&cell), None, "frames 480 paired with the previous instant and buffer size");

        cell.frames_end.store(960, Ordering::Relaxed);
        cell.callback_nanos.store(20_000_000, Ordering::Relaxed);
        cell.seq.store(2, Ordering::Release);
        let expected = ClockState { frames_start: 480, frames_end: 960, callback_nanos: 20_000_000, playback_ahead_nanos: 5_000_000, buffer_frames: 480 };
        assert_eq!(try_read_clock_state(&cell), Some(expected));
    }

    #[test]
    fn concurrent_clock_state_reads_never_observe_a_torn_record() {
        const WRITES: u64 = 100_000;
        const NANOS_PER_FRAME: u64 = 7;
        const AHEAD_NANOS_PER_FRAME: u64 = 3;
        const FRAMES_PER_STEP: u32 = 64;

        let cell = Arc::new(ClockCell::new(ClockState::default()));
        let done = Arc::new(AtomicBool::new(false));
        let writer_cell = cell.clone();
        let writer_done = done.clone();
        let writer = std::thread::spawn(move || {
            let mut frames_start = 0u64;
            for k in 1..=WRITES {
                let buffer_frames = (k % 3 + 1) as u32 * FRAMES_PER_STEP;
                write_clock_state(
                    &writer_cell,
                    ClockState {
                        frames_start,
                        frames_end: frames_start + buffer_frames as u64,
                        callback_nanos: frames_start * NANOS_PER_FRAME,
                        playback_ahead_nanos: buffer_frames as u64 * AHEAD_NANOS_PER_FRAME,
                        buffer_frames,
                    },
                );
                frames_start += buffer_frames as u64;
            }
            writer_done.store(true, Ordering::Release);
        });

        let mut verified = 0u64;
        let mut previous = 0u64;
        while !done.load(Ordering::Acquire) {
            if let Some(state) = try_read_clock_state(&cell) {
                assert_eq!(state.frames_end - state.frames_start, state.buffer_frames as u64, "torn record: {state:?}");
                assert_eq!(state.callback_nanos, state.frames_start * NANOS_PER_FRAME, "torn record: {state:?}");
                assert_eq!(state.playback_ahead_nanos, state.buffer_frames as u64 * AHEAD_NANOS_PER_FRAME, "torn record: {state:?}");
                assert!(state.frames_start >= previous, "frames went backwards: {} after {previous}", state.frames_start);
                previous = state.frames_start;
                verified += 1;
            }
        }
        writer.join().expect("writer thread");
        assert!(verified > 0, "the reader never completed a verified read");
    }

    #[test]
    fn read_mix_stats_returns_the_last_completed_record() {
        let cell = StatsCell::new();
        let cache = Cell::new(MixStats::default());
        let published = MixStats { active_voices: 37, steals: 4, hard_steals: 1, late_schedules: 9 };
        write_mix_stats(&cell, published);
        assert_eq!(read_mix_stats(&cell, &cache), published);
    }

    #[test]
    fn try_read_mix_stats_rejects_a_half_written_record() {
        let cell = StatsCell::new();
        write_mix_stats(&cell, MixStats { active_voices: 37, steals: 4, hard_steals: 1, late_schedules: 9 });
        cell.seq.store(1, Ordering::Relaxed);
        cell.active_voices.store(40, Ordering::Relaxed);
        assert_eq!(try_read_mix_stats(&cell), None);

        cell.seq.store(2, Ordering::Release);
        assert_eq!(try_read_mix_stats(&cell).map(|s| s.active_voices), Some(40));
    }

    /// The steady state every backend reports: the data handed to the device sounds exactly one
    /// buffer period later (CoreAudio derives `playback` that way, and the buffer-derived fallback
    /// does the same), so this is the condition the interpolation has to hold up under.
    fn steady_snapshot(callback_at: Instant, frames_start: u64, buffer_frames: u32, out_rate: u32) -> ClockSnapshot {
        snapshot(callback_at, frames_start, frames_to_duration(buffer_frames, out_rate), buffer_frames)
    }

    #[test]
    fn audible_position_advances_inside_one_callback_period() {
        let callback_at = Instant::now();
        let snap = steady_snapshot(callback_at, 48_000, 512, 48_000);
        let period = frames_to_duration(512, 48_000);
        let base = 1_000_000;
        let ahead_us = period.as_micros() as i64;

        let mut previous = i64::MIN;
        for step in 0..=4u32 {
            let now = callback_at + period * step / 4;
            let value = audible_us_from(&snap, now, 48_000);
            assert!(value > previous, "the clock stalled inside a callback period at step {step}: {value} after {previous}");
            previous = value;
        }
        assert_eq!(audible_us_from(&snap, callback_at, 48_000), base - ahead_us, "at the callback instant the previous buffer is still sounding");
        assert_eq!(audible_us_from(&snap, callback_at + period, 48_000), base, "one period later this buffer starts sounding");
    }

    /// Standard deviation of `(x, y)` around its least-squares line, which is how the soak gate
    /// measures whether the clock is genuinely interpolated.
    fn residual_sd(points: &[(f64, f64)]) -> f64 {
        let n = points.len() as f64;
        let mean_x = points.iter().map(|(x, _)| x).sum::<f64>() / n;
        let mean_y = points.iter().map(|(_, y)| y).sum::<f64>() / n;
        let sxx: f64 = points.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();
        let sxy: f64 = points.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum();
        let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
        let sse: f64 = points.iter().map(|(x, y)| (y - mean_y - slope * (x - mean_x)).powi(2)).sum();
        (sse / n).sqrt()
    }

    /// The Phase B acceptance figure: sampled densely across a real device timeline, the
    /// interpolated clock must stay on a straight line to well under a millisecond. A clock that
    /// only steps once per callback scatters by `buffer_period / sqrt(12)`, which is 3.08 ms at
    /// 512 frames and 48 kHz — three times the limit.
    #[test]
    fn the_interpolated_clock_stays_on_its_line_across_a_device_timeline() {
        const RATE: u32 = 48_000;
        const BUFFER: u32 = 512;
        const SAMPLE_INTERVAL_US: u64 = 250;
        const SPAN_S: u64 = 3;
        const RESIDUAL_LIMIT_US: f64 = 1_000.0;

        let epoch = Instant::now();
        let period = frames_to_duration(BUFFER, RATE);
        let period_us = period.as_micros() as u64;
        let mut points = Vec::new();
        for step in 0..(SPAN_S * 1_000_000 / SAMPLE_INTERVAL_US) {
            let elapsed_us = step * SAMPLE_INTERVAL_US;
            let callback = elapsed_us / period_us;
            let snap = steady_snapshot(epoch + period * callback as u32, callback * BUFFER as u64, BUFFER, RATE);
            let now = epoch + Duration::from_micros(elapsed_us);
            points.push((elapsed_us as f64, audible_us_from(&snap, now, RATE) as f64));
        }

        let scatter = residual_sd(&points);
        assert!(scatter < RESIDUAL_LIMIT_US, "the interpolated clock scattered by {scatter} us, which is a per-callback staircase");

        let quantised: Vec<(f64, f64)> = points.iter().map(|(x, _)| (*x, (*x as u64 / period_us * period_us) as f64)).collect();
        let quantisation_sd = period_us as f64 / 12.0f64.sqrt();
        assert!(
            residual_sd(&quantised) > RESIDUAL_LIMIT_US,
            "the fixture must be able to tell a staircase apart: a stepped clock scatters by about {quantisation_sd} us"
        );
    }

    #[test]
    fn audible_position_is_continuous_across_a_callback_boundary() {
        let callback_at = Instant::now();
        let period = frames_to_duration(512, 48_000);
        let old = steady_snapshot(callback_at, 48_000, 512, 48_000);
        let new = steady_snapshot(callback_at + period, 48_512, 512, 48_000);
        let boundary = callback_at + period;
        assert_eq!(
            audible_us_from(&old, boundary, 48_000),
            audible_us_from(&new, boundary, 48_000),
            "the reading must not jump when the callback publishes a new record"
        );
    }

    #[test]
    fn audible_position_tracks_the_wall_clock_one_to_one() {
        let callback_at = Instant::now();
        let snap = steady_snapshot(callback_at, 48_000, 512, 48_000);
        let at_start = audible_us_from(&snap, callback_at, 48_000);
        for elapsed_us in [1_000i64, 2_500, 5_000, 8_000] {
            let value = audible_us_from(&snap, callback_at + Duration::from_micros(elapsed_us as u64), 48_000);
            assert_eq!(value - at_start, elapsed_us, "the interpolated clock must run at wall-clock rate");
        }
    }

    #[test]
    fn audible_position_advances_with_the_wall_clock_once_the_buffer_sounds() {
        let callback_at = Instant::now();
        let snap = snapshot(callback_at, 48_000, Duration::from_millis(10), 512);
        assert_eq!(audible_us_from(&snap, callback_at + Duration::from_millis(15), 48_000), 1_005_000);
        assert_eq!(audible_us_from(&snap, callback_at + Duration::from_millis(20), 48_000), 1_010_000);
    }

    #[test]
    fn audible_position_stops_at_the_extrapolation_cap() {
        let callback_at = Instant::now();
        let snap = snapshot(callback_at, 48_000, Duration::from_millis(10), 512);
        let cap_us = (MAX_EXTRAPOLATION_BUFFERS as i64 * 512 * 1_000_000) / 48_000;
        let frozen = 1_000_000 + cap_us - 10_000;
        assert_eq!(audible_us_from(&snap, callback_at + Duration::from_secs(1), 48_000), frozen, "the cap is measured from the callback instant");
        assert_eq!(audible_us_from(&snap, callback_at + Duration::from_secs(60), 48_000), frozen);
    }

    #[test]
    fn audible_position_counts_from_the_frame_the_callback_started_at() {
        let callback_at = Instant::now();
        let snap = snapshot(callback_at, 0, Duration::from_millis(10), 512);
        assert_eq!(snap.frames_rendered, 512, "the fixture must render a whole buffer");
        assert_eq!(audible_us_from(&snap, callback_at + Duration::from_millis(10), 48_000), 0, "using frames_rendered would run one buffer ahead");
    }

    #[test]
    fn audible_position_survives_a_zero_output_rate() {
        let callback_at = Instant::now();
        let snap = snapshot(callback_at, 0, Duration::ZERO, 512);
        assert_eq!(audible_us_from(&snap, callback_at, 0), 0);
    }

    #[test]
    fn lookahead_covers_the_device_lead_the_next_buffer_and_a_frame() {
        let callback_at = Instant::now();
        let snap = steady_snapshot(callback_at, 0, 512, 48_000);
        let period_us = frames_to_duration(512, 48_000).as_micros() as i64;
        assert_eq!(lookahead_us_from(&snap, 48_000), period_us + 10_708);

        let reported = snapshot(callback_at, 0, Duration::from_millis(25), 384);
        assert_eq!(lookahead_us_from(&reported, 48_000), 25_000 + 8_041, "the backend's own lead is used verbatim");
    }

    #[test]
    fn lookahead_outruns_the_frame_a_command_would_otherwise_collapse_onto() {
        let callback_at = Instant::now();
        let out_rate = 48_000;
        let buffer_frames = 512u32;
        let snap = steady_snapshot(callback_at, 96_000, buffer_frames, out_rate);
        let lookahead = lookahead_us_from(&snap, out_rate);

        let booked_us = audible_us_from(&snap, callback_at, out_rate) + lookahead;
        let booked_frame = booked_us as i128 * out_rate as i128 / 1_000_000;
        assert!(booked_frame > snap.frames_rendered as i128, "a booked onset must land past the frame the next callback starts on");
    }

    /// The end-to-end reason the lookahead exists: a host that reads the clock once per display
    /// frame and books every sound that has come due must never hand the mixer a time the mixer has
    /// already passed, because that collapses the onset onto the next buffer boundary. This walks a
    /// whole song's worth of onsets through the real mixer on a simulated device timeline and holds
    /// `late_schedules` at zero.
    ///
    /// The loop below is one callback per iteration: it first drains what the host queued since the
    /// previous callback and renders, then lets every host frame falling inside that callback period
    /// book whatever has come due.
    #[test]
    fn a_frame_paced_host_never_books_an_onset_the_mixer_has_already_passed() {
        const RATE: u32 = 48_000;
        const BUFFER: u32 = 512;
        const OUT_CHANNELS: u16 = 1;
        const VOICES: usize = 64;
        const ONSET_SPACING_US: i64 = 7_000;
        const SONG_US: i64 = 5_000_000;
        /// Onsets start past the lead the device is already committed to at time zero. Nothing can
        /// place a sound inside that window: the frames covering it were handed to the DAC before
        /// the first command could be queued.
        const ONSET_START_US: i64 = 100_000;
        const SAMPLE_FRAMES: usize = 96;

        for frame_hz in [60u32, 120] {
            let period = frames_to_duration(BUFFER, RATE);
            let frame_period = Duration::from_micros(1_000_000 / frame_hz as u64);
            let poll_us = frame_period.as_micros() as i64;
            let epoch = Instant::now();

            let sample = Arc::new(SampleData { pcm: vec![0.25f32; SAMPLE_FRAMES].into(), channels: 1, rate: RATE });
            let mut mixer = Mixer::new(RATE, OUT_CHANNELS, VOICES);
            let mut out = vec![0.0f32; BUFFER as usize];

            let onsets: Vec<i64> = (0..).map(|i| ONSET_START_US + i * ONSET_SPACING_US).take_while(|at| *at < SONG_US).collect();
            let mut next_onset = 0usize;
            let mut queued: Vec<Command> = Vec::new();
            let mut callback = 0u64;
            let mut frame = 0u64;
            let mut booked = 0usize;

            while (callback * BUFFER as u64) < (SONG_US as u64 * RATE as u64 / 1_000_000) {
                let callback_at = epoch + period * callback as u32;
                for cmd in queued.drain(..) {
                    mixer.apply(cmd);
                }
                let snap = ClockSnapshot {
                    frames_at_callback_start: callback * BUFFER as u64,
                    frames_rendered: (callback + 1) * BUFFER as u64,
                    callback_at,
                    playback_ahead: period,
                    buffer_frames: BUFFER,
                };
                mixer.mix(&mut out);

                while epoch + frame_period * (frame as u32) < callback_at + period {
                    let now = epoch + frame_period * frame as u32;
                    let sched = audible_us_from(&snap, now, RATE) + lookahead_us_from(&snap, RATE) + poll_us;
                    while next_onset < onsets.len() && onsets[next_onset] <= sched {
                        let at_frame = (onsets[next_onset] as i128 * RATE as i128 / 1_000_000) as u64;
                        queued.push(Command::Play {
                            sample: Arc::clone(&sample),
                            gain: 1.0,
                            pan: 0.0,
                            pitch: 1.0,
                            key: next_onset as u32,
                            at_frame,
                            bus: Bus::Bg,
                        });
                        next_onset += 1;
                        booked += 1;
                    }
                    frame += 1;
                }
                callback += 1;
            }

            assert!(booked > 100, "the simulation must actually book onsets, booked {booked} at {frame_hz} Hz");
            assert_eq!(mixer.stats().late_schedules, 0, "at {frame_hz} Hz an onset was booked in the mixer's past");
        }
    }

    #[test]
    fn lookahead_is_zero_without_a_usable_stream_shape() {
        let callback_at = Instant::now();
        assert_eq!(lookahead_us_from(&snapshot(callback_at, 0, Duration::ZERO, 512), 0), 0);
        assert_eq!(lookahead_us_from(&snapshot(callback_at, 0, Duration::ZERO, 0), 48_000), 0);
    }

    #[test]
    fn a_recoverable_underrun_does_not_kill_the_stream() {
        assert!(!stream_error_is_fatal(&cpal::StreamError::BufferUnderrun));
        assert!(stream_error_is_fatal(&cpal::StreamError::DeviceNotAvailable));
        assert!(stream_error_is_fatal(&cpal::StreamError::StreamInvalidated));
        assert!(stream_error_is_fatal(&cpal::StreamError::BackendSpecific { err: cpal::BackendSpecificError { description: "driver reset".to_string() } }));
    }

    #[test]
    fn losing_the_requested_device_is_reported_as_a_device_fallback() {
        assert_eq!(reported_step_floor(false), STEP_REQUESTED);
        assert_eq!(reported_step_floor(true), STEP_DEFAULT_DEVICE);
        const { assert!(STEP_DEFAULT_DEVICE > STEP_DEVICE_DEFAULT_CONFIG, "a device fallback outranks any format downgrade") };
    }

    #[test]
    fn the_retirement_ring_holds_a_turnover_of_every_voice() {
        assert_eq!(retire_queue_capacity(0), MIN_RETIRE_QUEUE_CAPACITY);
        assert_eq!(retire_queue_capacity(512), 512 * RETIRE_SLOTS_PER_VOICE);
        assert!(retire_queue_capacity(DEFAULT_MAX_VOICES) >= DEFAULT_MAX_VOICES);
    }

    #[test]
    fn a_torn_read_falls_back_to_the_last_whole_record_instead_of_mixing_two() {
        let cell = ClockCell::new(ClockState::default());
        let cache = Cell::new(ClockState::default());
        let published = ClockState { frames_start: 0, frames_end: 480, callback_nanos: 10_000_000, playback_ahead_nanos: 5_000_000, buffer_frames: 480 };
        write_clock_state(&cell, published);
        assert_eq!(read_clock_state(&cell, &cache), published);

        cell.seq.store(1, Ordering::Relaxed);
        cell.frames_start.store(480, Ordering::Relaxed);
        cell.frames_end.store(960, Ordering::Relaxed);
        assert_eq!(read_clock_state(&cell, &cache), published, "a stalled writer must not yield a frame count from one record and an instant from another");
    }

    #[test]
    fn a_torn_stats_read_falls_back_to_the_last_whole_record() {
        let cell = StatsCell::new();
        let cache = Cell::new(MixStats::default());
        let published = MixStats { active_voices: 12, steals: 3, hard_steals: 0, late_schedules: 5 };
        write_mix_stats(&cell, published);
        assert_eq!(read_mix_stats(&cell, &cache), published);

        cell.seq.store(1, Ordering::Relaxed);
        cell.active_voices.store(99, Ordering::Relaxed);
        assert_eq!(read_mix_stats(&cell, &cache), published);
    }

    #[test]
    fn monotonic_clamp_never_steps_backwards() {
        assert_eq!(monotonic_us(1_000, 900), 1_000);
        assert_eq!(monotonic_us(1_000, 1_000), 1_000);
        assert_eq!(monotonic_us(1_000, 1_100), 1_100);
        assert_eq!(monotonic_us(-5, -10), -5);
    }

    #[test]
    fn playback_lead_uses_the_backend_figure_when_it_is_plausible() {
        let reported = Some(Duration::from_millis(5));
        assert!(ahead_is_usable(reported));
        assert_eq!(ahead_or_fallback(reported, 512, 48_000), Duration::from_millis(5));
    }

    #[test]
    fn playback_lead_falls_back_when_the_backend_reports_nothing() {
        assert!(!ahead_is_usable(None));
        assert_eq!(ahead_or_fallback(None, 512, 48_000), Duration::from_nanos(10_666_666));
    }

    #[test]
    fn playback_lead_falls_back_on_an_implausible_figure() {
        let absurd = Some(Duration::from_millis(MAX_PLAYBACK_AHEAD_MS + 1));
        assert!(!ahead_is_usable(absurd));
        assert_eq!(ahead_or_fallback(absurd, 512, 48_000), Duration::from_nanos(10_666_666));
        let at_limit = Some(Duration::from_millis(MAX_PLAYBACK_AHEAD_MS));
        assert!(ahead_is_usable(at_limit));
        assert_eq!(ahead_or_fallback(at_limit, 512, 48_000), Duration::from_millis(MAX_PLAYBACK_AHEAD_MS));
    }

    #[test]
    fn a_callback_arriving_on_time_is_not_an_underrun() {
        assert!(!is_underrun_gap(Duration::from_micros(10_666), 512, 48_000));
        assert!(!is_underrun_gap(Duration::from_micros(15_999), 512, 48_000));
    }

    #[test]
    fn a_callback_arriving_late_counts_as_an_underrun() {
        assert!(is_underrun_gap(Duration::from_micros(16_001), 512, 48_000));
        assert!(is_underrun_gap(Duration::from_millis(50), 512, 48_000));
    }

    #[test]
    fn underrun_detection_needs_a_previous_buffer_and_a_rate() {
        assert!(!is_underrun_gap(Duration::from_secs(1), 0, 48_000));
        assert!(!is_underrun_gap(Duration::from_secs(1), 512, 0));
    }

    #[test]
    fn namespaces_map_to_disjoint_channel_key_ranges() {
        assert_eq!(namespace_key_range(IdNamespace::PLAY), (0, 0x0100_0000));
        assert_eq!(namespace_key_range(IdNamespace::PREVIEW), (0x8000_0000, 0x8100_0000));
        let (play_lo, play_hi) = namespace_key_range(IdNamespace::PLAY);
        let (preview_lo, preview_hi) = namespace_key_range(IdNamespace::PREVIEW);
        assert!(play_lo < play_hi && play_hi <= preview_lo && preview_lo < preview_hi);
    }

    #[test]
    fn namespace_channel_keys_stay_inside_the_stop_range() {
        let (lo, hi) = namespace_key_range(IdNamespace::PREVIEW);
        let first = channel_key(IdNamespace::PREVIEW.base, 1.0);
        let last = channel_key(IdNamespace::PREVIEW.base + IdNamespace::PREVIEW.len - 1, 1.0);
        assert!(lo <= first && first < hi);
        assert!(lo <= last && last < hi, "the top of the preview namespace must not wrap past u32");
    }

    #[test]
    fn namespace_membership_is_half_open() {
        assert!(IdNamespace::PLAY.contains(0));
        assert!(IdNamespace::PLAY.contains(1295));
        assert!(!IdNamespace::PLAY.contains(IdNamespace::PLAY.len));
        assert!(!IdNamespace::PLAY.contains(IdNamespace::PREVIEW.base));
        assert!(IdNamespace::PREVIEW.contains(IdNamespace::PREVIEW.base));
        assert!(IdNamespace::PREVIEW.contains(IdNamespace::PREVIEW.base + 1295));
        assert!(!IdNamespace::PREVIEW.contains(IdNamespace::PREVIEW.base + IdNamespace::PREVIEW.len));
        assert!(!IdNamespace::PREVIEW.contains(IdNamespace::PREVIEW.base - 1));
    }

    #[test]
    fn audio_options_default_requests_nothing_in_particular() {
        let opts = AudioOptions::default();
        assert_eq!(opts.device_name, None);
        assert_eq!(opts.sample_rate, None);
        assert_eq!(opts.buffer_frames, None);
        assert_eq!(opts.max_voices, DEFAULT_MAX_VOICES);
    }

    #[test]
    fn config_pick_needs_at_least_one_advertised_range() {
        assert_eq!(pick_config(&[], &AudioOptions::default(), 48_000), None);
    }

    #[test]
    fn config_pick_leaves_an_auto_request_to_the_device() {
        let supported = [range(44_100, 96_000, bounded(128, 2048))];
        let pick = pick_config(&supported, &AudioOptions::default(), 48_000).expect("a range is advertised");
        assert_eq!(pick.sample_rate, 48_000);
        assert_eq!(pick.buffer, BufferSize::Default);
        assert!(!pick.sample_rate_adjusted);
        assert!(!pick.buffer_adjusted);
    }

    #[test]
    fn config_pick_honours_a_supported_request() {
        let supported = [range(44_100, 96_000, bounded(128, 2048))];
        let opts = AudioOptions { sample_rate: Some(44_100), buffer_frames: Some(384), ..AudioOptions::default() };
        let pick = pick_config(&supported, &opts, 48_000).expect("a range is advertised");
        assert_eq!(pick.sample_rate, 44_100);
        assert_eq!(pick.buffer, BufferSize::Fixed(384));
        assert!(!pick.sample_rate_adjusted);
        assert!(!pick.buffer_adjusted);
    }

    #[test]
    fn config_pick_clamps_a_sample_rate_the_device_cannot_reach() {
        let supported = [range(44_100, 48_000, bounded(128, 2048))];
        let high = AudioOptions { sample_rate: Some(96_000), ..AudioOptions::default() };
        let picked_high = pick_config(&supported, &high, 48_000).expect("a range is advertised");
        assert_eq!(picked_high.sample_rate, 48_000);
        assert!(picked_high.sample_rate_adjusted);

        let low = AudioOptions { sample_rate: Some(22_050), ..AudioOptions::default() };
        let picked_low = pick_config(&supported, &low, 48_000).expect("a range is advertised");
        assert_eq!(picked_low.sample_rate, 44_100);
        assert!(picked_low.sample_rate_adjusted);
    }

    #[test]
    fn config_pick_clamps_a_buffer_the_device_cannot_reach() {
        let supported = [range(44_100, 48_000, bounded(128, 2048))];
        let small = AudioOptions { buffer_frames: Some(64), ..AudioOptions::default() };
        let picked_small = pick_config(&supported, &small, 48_000).expect("a range is advertised");
        assert_eq!(picked_small.buffer, BufferSize::Fixed(128));
        assert!(picked_small.buffer_adjusted);

        let large = AudioOptions { buffer_frames: Some(8192), ..AudioOptions::default() };
        let picked_large = pick_config(&supported, &large, 48_000).expect("a range is advertised");
        assert_eq!(picked_large.buffer, BufferSize::Fixed(2048));
        assert!(picked_large.buffer_adjusted);
    }

    #[test]
    fn config_pick_gives_up_on_a_fixed_buffer_when_the_device_advertises_no_range() {
        let supported = [range(48_000, 48_000, SupportedBufferSize::Unknown)];
        let opts = AudioOptions { buffer_frames: Some(384), ..AudioOptions::default() };
        let pick = pick_config(&supported, &opts, 48_000).expect("a range is advertised");
        assert_eq!(pick.buffer, BufferSize::Default);
        assert!(pick.buffer_adjusted);
    }

    #[test]
    fn config_pick_prefers_the_range_that_covers_the_requested_rate() {
        let supported = [range(44_100, 44_100, bounded(128, 512)), range(48_000, 96_000, bounded(256, 4096))];
        let opts = AudioOptions { sample_rate: Some(96_000), buffer_frames: Some(4096), ..AudioOptions::default() };
        let pick = pick_config(&supported, &opts, 44_100).expect("a range is advertised");
        assert_eq!(pick.sample_rate, 96_000);
        assert_eq!(pick.buffer, BufferSize::Fixed(4096));
        assert!(!pick.sample_rate_adjusted);
        assert!(!pick.buffer_adjusted);
    }

    #[test]
    fn a_downgrade_produces_one_note_per_adjusted_field() {
        let opts = AudioOptions { sample_rate: Some(96_000), buffer_frames: Some(64), ..AudioOptions::default() };
        let pick = PickedConfig { sample_rate: 48_000, buffer: BufferSize::Fixed(128), sample_rate_adjusted: true, buffer_adjusted: true };
        assert_eq!(
            pick_notes(&opts, &pick),
            vec!["requested 96000 Hz -> device supports 48000 Hz".to_string(), "requested a 64 frame buffer -> 128 frames".to_string()]
        );
    }

    #[test]
    fn an_honoured_request_produces_no_notes() {
        let opts = AudioOptions { sample_rate: Some(48_000), buffer_frames: Some(512), ..AudioOptions::default() };
        let pick = PickedConfig { sample_rate: 48_000, buffer: BufferSize::Fixed(512), sample_rate_adjusted: false, buffer_adjusted: false };
        assert!(pick_notes(&opts, &pick).is_empty());
    }

    /// The §4.1 acceptance measurement on real hardware: poll the interpolated clock densely and
    /// check it stays on a straight line. A clock that only steps once per callback scatters by
    /// `buffer_period / sqrt(12)`, which is over 3 ms on a typical 512-frame device.
    #[test]
    #[ignore = "requires a real audio output device"]
    fn the_interpolated_clock_is_smooth_on_a_real_device() {
        const SAMPLE_INTERVAL: Duration = Duration::from_micros(250);
        const SAMPLES: usize = 8_000;
        const RESIDUAL_LIMIT_US: f64 = 1_000.0;

        let (engine, _) = AudioEngine::open(&AudioOptions::default()).expect("audio device");
        std::thread::sleep(Duration::from_millis(200));

        let start = Instant::now();
        let mut points = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let now = Instant::now();
            points.push((now.duration_since(start).as_micros() as f64, engine.audible_us(now) as f64));
            std::thread::sleep(SAMPLE_INTERVAL);
        }

        let scatter = residual_sd(&points);
        let period_us = frames_to_duration(engine.snapshot().buffer_frames, engine.out_rate()).as_micros() as f64;
        assert!(engine.is_alive(), "the stream died during the measurement");
        println!(
            "real device: buffer {period_us:.0} us, interpolated clock residual sd {scatter:.1} us, staircase would be {:.1} us",
            period_us / 12.0f64.sqrt()
        );
        assert!(scatter < RESIDUAL_LIMIT_US, "the clock scattered by {scatter} us; a {period_us} us staircase would give {}", period_us / 12.0f64.sqrt());
    }

    #[test]
    #[ignore = "requires a real audio output device"]
    fn engine_reports_alive_and_advancing_clock_on_a_real_device() {
        let (engine, report) = AudioEngine::open(&AudioOptions::default()).expect("audio device");
        assert!(engine.is_alive());
        assert_eq!(engine.dropped_commands(), 0);
        assert_eq!(report.fallback_step, STEP_REQUESTED);
        assert_eq!(report.sample_rate, engine.out_rate());
        std::thread::sleep(Duration::from_millis(200));

        let snap = engine.snapshot();
        assert!(snap.frames_rendered > 0, "audio clock did not advance");
        assert!(snap.frames_rendered >= snap.frames_at_callback_start);
        assert!(snap.buffer_frames > 0);
        assert!(snap.callback_at >= engine.clock_epoch);

        let clocks = engine.clocks(Instant::now());
        assert_eq!(clocks.scheduled_us - clocks.audible_us, clocks.lookahead_us, "the three clocks must come from one snapshot");
        assert!(clocks.lookahead_us > 0, "a live stream must ask the scheduler to look ahead");
        assert_eq!(engine.scratch_reallocations(), 0, "the callback allocated on the RT thread");
        assert_eq!(engine.underruns(), 0, "a silent engine should not report dropouts");
    }
}

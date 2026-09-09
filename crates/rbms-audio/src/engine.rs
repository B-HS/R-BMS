use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering, fence};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig};

use crate::AudioError;
use crate::decode::{DecodedAudio, decode_bytes};
use crate::mixer::{Command, Mixer, SampleData, channel_key};

/// Default polyphony. The reference implementation's `deviceSimultaneousSources` default is 256
/// (`AudioConfig.java:28`); rbms doubles that for slack, since a full pool still steals the voice
/// under the allocation cursor whether or not it is audible (`Mixer::alloc_slot`). Fading a stolen
/// voice out instead of cutting it is a separate, later change.
pub(crate) const DEFAULT_MAX_VOICES: usize = 512;

/// Command ring capacity. One slot per queued play/stop; overflow is counted, never blocking.
const COMMAND_QUEUE_CAPACITY: usize = 8192;

/// Frames the callback scratch buffer is pre-sized for, so the steady state performs no
/// allocation inside the real-time callback. Device buffers are typically 128-2048 frames
/// (the reference implementation's `deviceBufferSize` default is 384, `AudioConfig.java:23`).
const SCRATCH_PREALLOC_FRAMES: usize = 4096;

/// Reader retries when sampling the (frames, wall clock) pair written by the audio callback.
const CLOCK_SNAPSHOT_ATTEMPTS: usize = 4;

/// Sequence step per callback write: the counter goes odd while the (frames, nanos) pair is being
/// written and back to even once both halves are stored, so a reader can tell a torn pair apart.
const CLOCK_SEQ_STEP: u64 = 1;

struct Telemetry {
    alive: AtomicBool,
    dropped_commands: AtomicU64,
    scratch_reallocations: AtomicU64,
    callback_nanos: AtomicU64,
    clock_seq: AtomicU64,
}

impl Telemetry {
    fn new() -> Self {
        Telemetry {
            alive: AtomicBool::new(true),
            dropped_commands: AtomicU64::new(0),
            scratch_reallocations: AtomicU64::new(0),
            callback_nanos: AtomicU64::new(0),
            clock_seq: AtomicU64::new(0),
        }
    }
}

/// Owns the cpal output stream, the RT-safe command queue, the sample-derived master
/// clock, and the loaded keysound bank. The play-position clock is `samples_played /
/// rate`, NOT the frame/vsync clock — this is the timing-source fix vs the reference implementation.
pub struct AudioEngine {
    _stream: cpal::Stream,
    producer: rtrb::Producer<Command>,
    clock: Arc<AtomicU64>,
    telemetry: Arc<Telemetry>,
    clock_epoch: Instant,
    out_rate: u32,
    bank: HashMap<u32, Arc<SampleData>>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        Self::with_max_voices(DEFAULT_MAX_VOICES)
    }

    pub fn with_max_voices(max_voices: usize) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
        let supported = device.default_output_config().map_err(|e| AudioError::Stream(e.to_string()))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.config();
        let out_rate = config.sample_rate;
        let out_channels = config.channels;

        let (producer, consumer) = rtrb::RingBuffer::<Command>::new(COMMAND_QUEUE_CAPACITY);
        let clock = Arc::new(AtomicU64::new(0));
        let telemetry = Arc::new(Telemetry::new());
        let clock_epoch = Instant::now();
        let mixer = Mixer::new(out_rate, out_channels, max_voices);
        let scratch_samples = scratch_prealloc_samples(&config);

        let ctx = StreamContext { mixer, consumer, clock: clock.clone(), telemetry: telemetry.clone(), clock_epoch, scratch_samples };
        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(&device, &config, ctx)?,
            SampleFormat::I16 => build_stream::<i16>(&device, &config, ctx)?,
            SampleFormat::U16 => build_stream::<u16>(&device, &config, ctx)?,
            other => return Err(AudioError::Unsupported(format!("sample format {other:?}"))),
        };
        stream.play().map_err(|e| AudioError::Stream(e.to_string()))?;

        Ok(AudioEngine { _stream: stream, producer, clock, telemetry, clock_epoch, out_rate, bank: HashMap::new() })
    }

    pub fn out_rate(&self) -> u32 {
        self.out_rate
    }

    pub fn clock_frames(&self) -> u64 {
        self.clock.load(Ordering::Acquire)
    }

    pub fn clock_us(&self) -> i64 {
        (self.clock_frames() as i128 * 1_000_000 / self.out_rate.max(1) as i128) as i64
    }

    /// Frames rendered so far paired with the wall-clock instant the callback that rendered them
    /// ran. Lets a caller convert between the audio clock and `Instant`-based timing (lookahead
    /// scheduling, drift measurement) instead of assuming the two advance in lockstep.
    pub fn clock_snapshot(&self) -> (u64, Instant) {
        let (frames, nanos) = read_clock_pair(&self.clock, &self.telemetry.callback_nanos, &self.telemetry.clock_seq);
        (frames, self.clock_epoch + Duration::from_nanos(nanos))
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

    pub fn load(&mut self, id: u32, bytes: Vec<u8>, ext: Option<&str>) -> Result<(), AudioError> {
        let dec = decode_bytes(bytes, ext)?;
        self.insert_decoded(id, dec);
        Ok(())
    }

    /// Insert an already-decoded sample into the keysound bank. Same effect as [`load`](Self::load)
    /// but skips the (CPU-heavy) symphonia decode, which the caller has already done — lets a host
    /// decode many keysounds in parallel off-thread, then insert the results here on one thread.
    pub fn insert_decoded(&mut self, id: u32, audio: DecodedAudio) {
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
        if let Some(sample) = self.bank.get(&id).cloned() {
            let at_frame = (at_us.max(0) as i128 * self.out_rate as i128 / 1_000_000) as u64;
            let key = channel_key(id, pitch);
            self.push(Command::Play { sample, gain, pan, pitch, key, at_frame });
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

    fn push(&mut self, cmd: Command) {
        push_or_count(&mut self.producer, &self.telemetry, cmd);
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

/// Publish the (frames, wall-clock nanos) pair from the audio callback as one seqlock write: the
/// sequence counter is odd for the duration of the two stores, so a concurrent reader can detect
/// that it observed a half-updated pair. Single writer only (the callback).
fn write_clock_pair(clock: &AtomicU64, nanos: &AtomicU64, seq: &AtomicU64, frames: u64, elapsed_nanos: u64) {
    let start = seq.load(Ordering::Relaxed);
    seq.store(start.wrapping_add(CLOCK_SEQ_STEP), Ordering::Relaxed);
    fence(Ordering::Release);
    clock.store(frames, Ordering::Relaxed);
    nanos.store(elapsed_nanos, Ordering::Relaxed);
    seq.store(start.wrapping_add(2 * CLOCK_SEQ_STEP), Ordering::Release);
}

/// One seqlock read attempt. `None` means the callback was mid-write or completed a write while
/// the two halves were being read, i.e. the pair would have been torn.
fn try_read_clock_pair(clock: &AtomicU64, nanos: &AtomicU64, seq: &AtomicU64) -> Option<(u64, u64)> {
    let before = seq.load(Ordering::Acquire);
    if !before.is_multiple_of(2) {
        return None;
    }
    let frames = clock.load(Ordering::Relaxed);
    let elapsed_nanos = nanos.load(Ordering::Relaxed);
    fence(Ordering::Acquire);
    if seq.load(Ordering::Relaxed) == before { Some((frames, elapsed_nanos)) } else { None }
}

/// Read back a pair published by [`write_clock_pair`]. Retries a torn read up to
/// [`CLOCK_SNAPSHOT_ATTEMPTS`] times, then falls back to a plain read so a caller is never blocked
/// by an audio thread that stalled mid-write.
fn read_clock_pair(clock: &AtomicU64, nanos: &AtomicU64, seq: &AtomicU64) -> (u64, u64) {
    for _ in 0..CLOCK_SNAPSHOT_ATTEMPTS {
        if let Some(pair) = try_read_clock_pair(clock, nanos, seq) {
            return pair;
        }
        std::hint::spin_loop();
    }
    (clock.load(Ordering::Acquire), nanos.load(Ordering::Acquire))
}

fn scratch_prealloc_samples(config: &StreamConfig) -> usize {
    let frames = match config.buffer_size {
        BufferSize::Fixed(n) => (n as usize).max(SCRATCH_PREALLOC_FRAMES),
        BufferSize::Default => SCRATCH_PREALLOC_FRAMES,
    };
    frames * config.channels.max(1) as usize
}

struct StreamContext {
    mixer: Mixer,
    consumer: rtrb::Consumer<Command>,
    clock: Arc<AtomicU64>,
    telemetry: Arc<Telemetry>,
    clock_epoch: Instant,
    scratch_samples: usize,
}

fn build_stream<T>(device: &cpal::Device, config: &StreamConfig, ctx: StreamContext) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample + FromSample<f32>,
{
    let StreamContext { mut mixer, mut consumer, clock, telemetry, clock_epoch, scratch_samples } = ctx;
    let mut scratch: Vec<f32> = vec![0.0; scratch_samples];
    let error_telemetry = telemetry.clone();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                while let Ok(cmd) = consumer.pop() {
                    mixer.apply(cmd);
                }
                grow_scratch(&mut scratch, data.len(), &telemetry);
                let buf = &mut scratch[..data.len()];
                mixer.mix(buf);
                for (o, s) in data.iter_mut().zip(buf.iter()) {
                    *o = T::from_sample(*s);
                }
                write_clock_pair(&clock, &telemetry.callback_nanos, &telemetry.clock_seq, mixer.clock_frames(), clock_epoch.elapsed().as_nanos() as u64);
            },
            move |e| {
                error_telemetry.alive.store(false, Ordering::Release);
                eprintln!("audio stream error: {e}");
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
    fn telemetry_starts_alive_with_zero_counters() {
        let t = Telemetry::new();
        assert!(t.alive.load(Ordering::Acquire));
        assert_eq!(t.dropped_commands.load(Ordering::Relaxed), 0);
        assert_eq!(t.scratch_reallocations.load(Ordering::Relaxed), 0);
        assert_eq!(t.callback_nanos.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn full_command_ring_counts_drops_instead_of_blocking() {
        let telemetry = Telemetry::new();
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
        let telemetry = Telemetry::new();
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
        let telemetry = Telemetry::new();
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
    fn read_clock_pair_returns_the_last_completed_pair() {
        let clock = AtomicU64::new(0);
        let nanos = AtomicU64::new(0);
        let seq = AtomicU64::new(0);
        write_clock_pair(&clock, &nanos, &seq, 480, 10_000_000);
        assert_eq!(read_clock_pair(&clock, &nanos, &seq), (480, 10_000_000));
        write_clock_pair(&clock, &nanos, &seq, 960, 20_000_000);
        assert_eq!(read_clock_pair(&clock, &nanos, &seq), (960, 20_000_000));
    }

    #[test]
    fn try_read_clock_pair_rejects_a_half_written_pair() {
        let clock = AtomicU64::new(0);
        let nanos = AtomicU64::new(0);
        let seq = AtomicU64::new(0);
        write_clock_pair(&clock, &nanos, &seq, 480, 10_000_000);
        assert_eq!(try_read_clock_pair(&clock, &nanos, &seq), Some((480, 10_000_000)));

        seq.store(1, Ordering::Relaxed);
        clock.store(960, Ordering::Relaxed);
        assert_eq!(try_read_clock_pair(&clock, &nanos, &seq), None, "frames 960 paired with the old 10ms instant");

        nanos.store(20_000_000, Ordering::Relaxed);
        seq.store(2, Ordering::Release);
        assert_eq!(try_read_clock_pair(&clock, &nanos, &seq), Some((960, 20_000_000)));
    }

    #[test]
    fn concurrent_clock_pair_reads_never_observe_a_torn_pair() {
        const RATIO: u64 = 7;
        const WRITES: u64 = 200_000;
        let state = Arc::new((AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)));
        let writer_state = state.clone();
        let writer = std::thread::spawn(move || {
            let (clock, nanos, seq) = &*writer_state;
            for k in 1..=WRITES {
                write_clock_pair(clock, nanos, seq, k, k * RATIO);
            }
        });

        let (clock, nanos, seq) = &*state;
        let mut verified = 0u64;
        let mut previous = 0u64;
        while clock.load(Ordering::Acquire) < WRITES {
            if let Some((frames, ns)) = try_read_clock_pair(clock, nanos, seq) {
                assert_eq!(ns, frames * RATIO, "torn pair: frames {frames}, nanos {ns}");
                assert!(frames >= previous, "frames went backwards: {frames} after {previous}");
                previous = frames;
                verified += 1;
            }
        }
        writer.join().expect("writer thread");
        assert!(verified > 0, "the reader never completed a verified read");
    }

    #[test]
    #[ignore = "requires a real audio output device"]
    fn engine_reports_alive_and_advancing_clock_on_a_real_device() {
        let engine = AudioEngine::new().expect("audio device");
        assert!(engine.is_alive());
        assert_eq!(engine.dropped_commands(), 0);
        std::thread::sleep(Duration::from_millis(200));
        let (frames, at) = engine.clock_snapshot();
        assert!(frames > 0, "audio clock did not advance");
        assert!(at >= engine.clock_epoch);
        assert_eq!(engine.scratch_reallocations(), 0, "the callback allocated on the RT thread");
    }
}

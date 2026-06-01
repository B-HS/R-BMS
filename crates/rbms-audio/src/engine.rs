use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};

use crate::AudioError;
use crate::decode::{DecodedAudio, decode_bytes};
use crate::mixer::{Command, Mixer, SampleData};

/// Owns the cpal output stream, the RT-safe command queue, the sample-derived master
/// clock, and the loaded keysound bank. The play-position clock is `samples_played /
/// rate`, NOT the frame/vsync clock — this is the timing-source fix vs beatoraja.
pub struct AudioEngine {
    _stream: cpal::Stream,
    producer: rtrb::Producer<Command>,
    clock: Arc<AtomicU64>,
    out_rate: u32,
    bank: HashMap<u32, Arc<SampleData>>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        Self::with_max_voices(512)
    }

    pub fn with_max_voices(max_voices: usize) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
        let supported = device.default_output_config().map_err(|e| AudioError::Stream(e.to_string()))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.config();
        let out_rate = config.sample_rate;
        let out_channels = config.channels;

        let (producer, consumer) = rtrb::RingBuffer::<Command>::new(8192);
        let clock = Arc::new(AtomicU64::new(0));
        let mixer = Mixer::new(out_rate, out_channels, max_voices);

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(&device, &config, mixer, consumer, clock.clone())?,
            SampleFormat::I16 => build_stream::<i16>(&device, &config, mixer, consumer, clock.clone())?,
            SampleFormat::U16 => build_stream::<u16>(&device, &config, mixer, consumer, clock.clone())?,
            other => return Err(AudioError::Unsupported(format!("sample format {other:?}"))),
        };
        stream.play().map_err(|e| AudioError::Stream(e.to_string()))?;

        Ok(AudioEngine { _stream: stream, producer, clock, out_rate, bank: HashMap::new() })
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
            let _ = self.producer.push(Command::Play { sample, gain, pan, pitch, key: id, at_frame });
        }
    }

    pub fn stop(&mut self, key: u32) {
        let _ = self.producer.push(Command::Stop { key });
    }

    pub fn set_master_gain(&mut self, g: f32) {
        let _ = self.producer.push(Command::MasterGain(g));
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut mixer: Mixer,
    mut consumer: rtrb::Consumer<Command>,
    clock: Arc<AtomicU64>,
) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample + FromSample<f32>,
{
    let mut scratch: Vec<f32> = Vec::new();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                while let Ok(cmd) = consumer.pop() {
                    mixer.apply(cmd);
                }
                if scratch.len() != data.len() {
                    scratch.resize(data.len(), 0.0);
                }
                mixer.mix(&mut scratch);
                for (o, s) in data.iter_mut().zip(scratch.iter()) {
                    *o = T::from_sample(*s);
                }
                clock.store(mixer.clock_frames(), Ordering::Release);
            },
            |e| eprintln!("audio stream error: {e}"),
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))
}

use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

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
        if self.channels == 0 {
            0
        } else {
            self.pcm.len() / self.channels as usize
        }
    }
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
    active: bool,
}

impl Voice {
    fn idle() -> Self {
        Voice { sample: None, pos: 0.0, stride: 1.0, gain: 0.0, lgain: 0.0, rgain: 0.0, delay: 0, key: 0, active: false }
    }
}

pub enum Command {
    Play { sample: Arc<SampleData>, gain: f32, pan: f32, pitch: f32, key: u32, at_frame: u64 },
    Stop { key: u32 },
    MasterGain(f32),
}

/// Real-time-safe software mixer. `mix` performs no allocation and no locking.
pub struct Mixer {
    voices: Vec<Voice>,
    master_gain: f32,
    out_rate: u32,
    out_channels: u16,
    clock: u64,
    alloc_cursor: usize,
}

impl Mixer {
    pub fn new(out_rate: u32, out_channels: u16, max_voices: usize) -> Self {
        Mixer {
            voices: (0..max_voices).map(|_| Voice::idle()).collect(),
            master_gain: 1.0,
            out_rate,
            out_channels,
            clock: 0,
            alloc_cursor: 0,
        }
    }

    pub fn clock_frames(&self) -> u64 {
        self.clock
    }

    pub fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Play { sample, gain, pan, pitch, key, at_frame } => self.play(sample, gain, pan, pitch, key, at_frame),
            Command::Stop { key } => self.stop(key),
            Command::MasterGain(g) => self.master_gain = g,
        }
    }

    fn play(&mut self, sample: Arc<SampleData>, gain: f32, pan: f32, pitch: f32, key: u32, at_frame: u64) {
        self.stop(key);
        let slot = self.alloc_slot();
        let angle = (pan.clamp(-1.0, 1.0) + 1.0) * 0.5 * FRAC_PI_2;
        let stride = (sample.rate as f64 / self.out_rate.max(1) as f64) * pitch.max(0.0001) as f64;
        self.voices[slot] = Voice {
            sample: Some(sample),
            pos: 0.0,
            stride,
            gain,
            lgain: angle.cos(),
            rgain: angle.sin(),
            delay: at_frame.saturating_sub(self.clock),
            key,
            active: true,
        };
    }

    fn stop(&mut self, key: u32) {
        for v in &mut self.voices {
            if v.active && v.key == key {
                v.active = false;
                v.sample = None;
            }
        }
    }

    fn alloc_slot(&mut self) -> usize {
        let n = self.voices.len();
        for off in 0..n {
            let i = (self.alloc_cursor + off) % n;
            if !self.voices[i].active {
                self.alloc_cursor = (i + 1) % n;
                return i;
            }
        }
        let i = self.alloc_cursor;
        self.alloc_cursor = (i + 1) % n;
        i
    }

    pub fn mix(&mut self, out: &mut [f32]) {
        for s in out.iter_mut() {
            *s = 0.0;
        }
        let oc = self.out_channels.max(1) as usize;
        let frames = out.len() / oc;

        for v in &mut self.voices {
            if !v.active {
                continue;
            }
            let sd = v.sample.as_ref().unwrap();
            let src_ch = sd.channels as usize;
            let nframes = sd.frames();
            let pcm = &sd.pcm;

            for f in 0..frames {
                if v.delay > 0 {
                    v.delay -= 1;
                    continue;
                }
                let i = v.pos as usize;
                if i >= nframes {
                    v.active = false;
                    v.sample = None;
                    break;
                }
                let frac = (v.pos - i as f64) as f32;
                // At the tail there is no next frame to interpolate toward, so clamp the partner to
                // the last sample — the final source frame still plays (it used to be dropped, which
                // silenced 1-frame samples entirely).
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
                let l = sl * v.gain * v.lgain;
                let r = sr * v.gain * v.rgain;
                let o = f * oc;
                if oc == 1 {
                    out[o] += (l + r) * 0.5;
                } else {
                    out[o] += l;
                    out[o + 1] += r;
                }
                v.pos += v.stride;
            }
        }

        let g = self.master_gain;
        for s in out.iter_mut() {
            *s = (*s * g).clamp(-1.0, 1.0);
        }
        self.clock += frames as u64;
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

    // ---- helpers ----

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

    // ---- clock / frame accounting ----

    #[test]
    fn clock_starts_at_zero() {
        let m = Mixer::new(48000, 2, 16);
        assert_eq!(m.clock_frames(), 0);
    }

    #[test]
    fn clock_advances_by_frames_not_samples_stereo() {
        // stereo: 8 samples => 4 frames per mix
        let mut m = Mixer::new(48000, 2, 16);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 4);
        m.mix(&mut out);
        assert_eq!(m.clock_frames(), 8);
    }

    #[test]
    fn clock_advances_by_frames_mono() {
        // mono: 8 samples => 8 frames per mix
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
        // out_channels 0 -> oc = max(1) = 1, so frames == out.len()
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

    // ---- delay / at_frame ----

    #[test]
    fn delay_silence_before_then_audio_at_frame() {
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(10, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 4);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        // frames 0..3 silent, audio starts at frame 4
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
        // at_frame < clock => saturating_sub yields 0, play immediately
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out); // advance clock to 10
        assert_eq!(m.clock_frames(), 10);
        m.play(flat(10, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 3);
        assert_eq!(m.voices[0].delay, 0);
    }

    #[test]
    fn delay_relative_to_clock_offset() {
        // clock advanced to 10, schedule at frame 13 => delay 3
        let mut m = Mixer::new(48000, 1, 16);
        let mut out = vec![0.0f32; 10];
        m.mix(&mut out);
        m.play(flat(10, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 13);
        assert_eq!(m.voices[0].delay, 3);
    }

    #[test]
    fn delay_spanning_multiple_mix_calls() {
        // delay larger than one buffer's frames must carry over to next mix
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(20, 1, 48000, 0.5), 1.0, 0.0, 1.0, 1, 6);
        let mut out = vec![0.0f32; 4];
        m.mix(&mut out); // consumes 4 of delay -> remaining 2
        assert!(out.iter().all(|&s| s == 0.0));
        let mut out2 = vec![0.0f32; 4];
        m.mix(&mut out2); // frames 4,5 silent; frame 6,7 audio
        assert_eq!(out2[0], 0.0);
        assert_eq!(out2[1], 0.0);
        assert!(out2[2] != 0.0);
    }

    // ---- stride / resampling / pitch ----

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
        // (24000/48000)*3 = 1.5
        assert!((m.voices[0].stride - 1.5).abs() < 1e-9);
    }

    #[test]
    fn pitch_floor_prevents_zero_stride() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, 0.0, 1, 0);
        // pitch.max(0.0001) => stride == 0.0001f32 (not zero). The f32 literal widens to
        // ~9.9999997e-5 as f64, so compare with a tolerance.
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
        // out_rate.max(1) guards against divide-by-zero
        let mut m = Mixer::new(0, 2, 16);
        m.play(ramp(100, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        assert!((m.voices[0].stride - 48000.0).abs() < 1e-6);
    }

    // ---- panning (equal power) ----

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
        // lgain^2 + rgain^2 == 1 for equal-power panning across the range
        for pan in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
            let mut m = Mixer::new(48000, 2, 16);
            m.play(ramp(10, 48000, 1), 1.0, pan, 1.0, 1, 0);
            let v = &m.voices[0];
            let power = v.lgain * v.lgain + v.rgain * v.rgain;
            assert!((power - 1.0).abs() < 1e-5, "pan {pan} power {power}");
        }
    }

    // ---- channel routing ----

    #[test]
    fn mono_source_duplicated_to_lr_full_left_isolates_left() {
        // mono sample, pan full-left -> rgain 0, so right output channel stays silent
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(10, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 8]; // 4 frames stereo
        m.mix(&mut out);
        // left (even indices) non-zero, right (odd indices) zero
        assert!(out[0] != 0.0);
        assert_eq!(out[1], 0.0);
        assert!(out[2] != 0.0);
        assert_eq!(out[3], 0.0);
    }

    #[test]
    fn stereo_source_routes_channels_independently() {
        // L=0.5, R=0.25, center pan, unity gains scaled by 0.7071
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat_lr(10, 48000, 0.5, 0.25), 1.0, 0.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 8];
        m.mix(&mut out);
        let g = std::f32::consts::FRAC_1_SQRT_2;
        assert!((out[0] - 0.5 * g).abs() < 1e-5);
        assert!((out[1] - 0.25 * g).abs() < 1e-5);
        // left channel louder than right since L value larger
        assert!(out[0] > out[1]);
    }

    #[test]
    fn mono_output_averages_l_and_r() {
        // mono output (oc==1): out = (l + r) * 0.5
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat_lr(10, 48000, 1.0, 0.0), 1.0, 0.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 4];
        m.mix(&mut out);
        // l = 1.0*g, r = 0.0; mono out = (l + 0)*0.5
        let g = std::f32::consts::FRAC_1_SQRT_2;
        assert!((out[0] - (1.0 * g) * 0.5).abs() < 1e-5);
    }

    // ---- gain ----

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
    fn master_gain_default_is_unity() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(flat(10, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 2];
        m.mix(&mut out);
        assert!((out[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn master_gain_scales_output() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(0.5));
        m.play(flat(10, 1, 48000, 0.4), 1.0, -1.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 2];
        m.mix(&mut out);
        assert!((out[0] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn master_gain_clamps_output_to_one() {
        // value 0.9 * master 4.0 = 3.6 -> clamped to 1.0
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(4.0));
        m.play(flat(10, 1, 48000, 0.9), 1.0, -1.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 2];
        m.mix(&mut out);
        assert_eq!(out[0], 1.0);
    }

    #[test]
    fn master_gain_clamps_output_to_negative_one() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(4.0));
        m.play(flat(10, 1, 48000, -0.9), 1.0, -1.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 2];
        m.mix(&mut out);
        assert_eq!(out[0], -1.0);
    }

    #[test]
    fn negative_master_gain_inverts_phase() {
        let mut m = Mixer::new(48000, 2, 16);
        m.apply(Command::MasterGain(-1.0));
        m.play(flat(10, 1, 48000, 0.5), 1.0, -1.0, 1.0, 1, 0);
        let mut out = vec![0.0f32; 2];
        m.mix(&mut out);
        assert!((out[0] + 0.5).abs() < 1e-6);
    }

    // ---- voice lifecycle / stealing / retrigger ----

    #[test]
    fn retrigger_keeps_only_one_active_and_uses_new_sample() {
        let mut m = Mixer::new(48000, 2, 16);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 7, 0);
        let first_slot = m.voices.iter().position(|v| v.active).unwrap();
        m.play(ramp(1000, 48000, 1), 0.3, 0.0, 1.0, 7, 0);
        assert_eq!(active_count(&m), 1);
        // the active voice should carry the new gain
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
        // pool of 2 voices, schedule 3 distinct keys: never exceeds capacity
        let mut m = Mixer::new(48000, 2, 2);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 1, 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 2, 0);
        m.play(ramp(1000, 48000, 1), 1.0, 0.0, 1.0, 3, 0);
        assert_eq!(active_count(&m), 2);
        assert_eq!(m.voices.len(), 2);
    }

    #[test]
    fn alloc_cursor_round_robins_free_slots() {
        // With 3 slots, sequential plays fill slot 0,1,2 then wrap.
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
        m.apply(Command::Play { sample: ramp(100, 48000, 1), gain: 1.0, pan: 0.0, pitch: 1.0, key: 9, at_frame: 0 });
        assert_eq!(active_count(&m), 1);
        assert_eq!(m.voices.iter().find(|v| v.active).unwrap().key, 9);
    }

    // ---- end-of-sample truncation ----

    #[test]
    fn all_frames_play_with_no_tail_truncation() {
        // Every source frame is emitted, including the last (the tail clamps interpolation to the
        // final sample). An nframes-frame mono sample at unity stride produces nframes audio frames.
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
        // nframes == 1: the lone frame plays (tail clamps to itself), then the voice deactivates.
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
        // A long sample at unity stride continues from where it left off.
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
        // pitch 2.0 doubles stride -> sample exhausted in roughly half the frames.
        let mut m = Mixer::new(48000, 1, 16);
        m.play(flat(20, 1, 48000, 0.5), 1.0, 0.0, 2.0, 1, 0);
        let mut out = vec![0.0f32; 64];
        m.mix(&mut out);
        let nonzero = out.iter().filter(|&&s| s != 0.0).count();
        // stride 2.0 over 20 frames: positions 0,2,4,...; breaks when i+1>=20 i.e. i>=19 -> pos>=18
        // frames emitted: pos 0,2,...,18 = 10 frames
        assert_eq!(nonzero, 10);
    }

    // ---- channel-count edge cases ----

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
    fn multiple_voices_sum_additively() {
        // two identical mono full-left voices on different keys should sum to 2x one voice
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
}

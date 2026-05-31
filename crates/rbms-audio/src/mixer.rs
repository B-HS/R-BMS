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
                if i + 1 >= nframes {
                    v.active = false;
                    v.sample = None;
                    break;
                }
                let frac = (v.pos - i as f64) as f32;
                let (sl, sr) = if src_ch == 1 {
                    let a = pcm[i];
                    let b = pcm[i + 1];
                    let s = a + (b - a) * frac;
                    (s, s)
                } else {
                    let base = i * src_ch;
                    let nb = (i + 1) * src_ch;
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
}

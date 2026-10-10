//! Sound effects on the mixer: one-shot and looped playback, overlap, stop, bus gain and the way
//! they leave keysounds alone. Everything runs on a bare [`Mixer`], so no output device is needed.

use super::*;

const OUT_RATE: u32 = 48_000;
const OUT_CHANNELS: u16 = 2;
const VOICES: usize = 16;
const SAMPLE_LEVEL: f32 = 0.5;
const SHORT_SAMPLE_FRAMES: usize = 480;
const TINY_SAMPLE_FRAMES: usize = 10;
const LONG_RENDER_FRAMES: usize = SHORT_SAMPLE_FRAMES * 40;
const BUFFER_FRAMES: usize = 512;
const EFFECT_ID: u32 = 7;
const OTHER_EFFECT_ID: u32 = 8;
const KEYSOUND_KEY: u32 = 3;
const SLEW_AND_ATTACK_FRAMES: usize = 512;
const SYSTEM_VOLUME: f32 = 0.25;
const VOICE_GAIN: f32 = 2.0;
const INSTANCES: u8 = 4;
const SERIALS: usize = 256;
const HALF_RATE: u32 = OUT_RATE / 2;
const TOLERANCE: f32 = 1e-5;

fn flat(frames: usize, rate: u32, value: f32) -> Arc<SampleData> {
    Arc::new(SampleData { pcm: vec![value; frames].into(), channels: 1, rate })
}

fn effect(sample: Arc<SampleData>, key: u32, bus: Bus, gain: f32, looped: bool) -> Command {
    Command::PlayEffect { sample, gain, key, bus, looped }
}

fn mixer() -> Mixer {
    Mixer::new(OUT_RATE, OUT_CHANNELS, VOICES)
}

fn render(m: &mut Mixer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; frames * OUT_CHANNELS as usize];
    m.mix(&mut out);
    out
}

fn left(out: &[f32]) -> Vec<f32> {
    out.chunks(OUT_CHANNELS as usize).map(|frame| frame[0]).collect()
}

fn sounding(m: &Mixer) -> usize {
    m.voices.iter().filter(|v| v.active && v.phase != VoicePhase::Release).count()
}

fn centred(level: f32) -> f32 {
    level * std::f32::consts::FRAC_1_SQRT_2
}

#[test]
fn a_one_shot_effect_leaves_the_mixer_when_it_reaches_the_end_of_its_sample() {
    let mut m = mixer();
    m.apply(effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, 1.0, false));
    assert_eq!(m.stats().active_voices, 1);
    let heard = render(&mut m, SHORT_SAMPLE_FRAMES / 2);
    assert!(heard.iter().any(|&s| s != 0.0), "the effect must be audible while it plays");
    assert_eq!(m.stats().active_voices, 1);
    render(&mut m, SHORT_SAMPLE_FRAMES);
    assert_eq!(m.stats().active_voices, 0, "a finished one-shot must free its voice");
    assert!(render(&mut m, BUFFER_FRAMES).iter().all(|&s| s == 0.0));
}

#[test]
fn a_looped_effect_keeps_sounding_past_the_end_of_its_sample_without_a_gap() {
    let mut m = mixer();
    m.apply(effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, 1.0, true));
    let out = left(&render(&mut m, LONG_RENDER_FRAMES));
    assert_eq!(m.stats().active_voices, 1, "a loop must outlast many lengths of its sample");
    let steady = centred(SAMPLE_LEVEL * DEFAULT_BUS_GAIN);
    for (frame, value) in out.iter().enumerate().skip(SLEW_AND_ATTACK_FRAMES) {
        assert!((value - steady).abs() < TOLERANCE, "frame {frame} is {value}, expected {steady}: the loop left a gap at the wrap");
    }
}

#[test]
fn a_loop_of_a_sample_shorter_than_a_buffer_fills_the_whole_buffer() {
    let mut m = mixer();
    m.apply(effect(flat(TINY_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, 1.0, true));
    let out = left(&render(&mut m, BUFFER_FRAMES));
    let steady = centred(SAMPLE_LEVEL * DEFAULT_BUS_GAIN);
    for (frame, value) in out.iter().enumerate().skip(SLEW_AND_ATTACK_FRAMES / 4) {
        assert!((value - steady).abs() < TOLERANCE, "frame {frame} is {value}");
    }
}

#[test]
fn a_loop_of_a_sample_at_half_the_output_rate_wraps_too() {
    let mut m = mixer();
    m.apply(effect(flat(SHORT_SAMPLE_FRAMES, HALF_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, 1.0, true));
    render(&mut m, LONG_RENDER_FRAMES);
    assert_eq!(m.stats().active_voices, 1);
}

#[test]
fn an_empty_looped_sample_ends_instead_of_spinning() {
    let mut m = mixer();
    m.apply(effect(Arc::new(SampleData { pcm: Vec::new().into(), channels: 1, rate: OUT_RATE }), effect_key(EFFECT_ID, 0), Bus::System, 1.0, true));
    render(&mut m, BUFFER_FRAMES);
    assert_eq!(m.stats().active_voices, 0);
}

#[test]
fn stopping_a_looped_effect_fades_it_out_and_frees_the_voice() {
    let mut m = mixer();
    m.apply(effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, 1.0, true));
    render(&mut m, LONG_RENDER_FRAMES);
    m.apply(Command::StopId { id: EFFECT_ID });
    render(&mut m, BUFFER_FRAMES);
    assert_eq!(m.stats().active_voices, 0, "a stopped loop must leave the mixer");
    assert!(render(&mut m, BUFFER_FRAMES).iter().all(|&s| s == 0.0), "a stopped loop must be silent");
}

#[test]
fn stopping_a_sample_id_reaches_every_instance_of_it_and_no_other_id() {
    let mut m = mixer();
    for serial in 0..INSTANCES {
        m.apply(effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, serial), Bus::System, 1.0, true));
    }
    m.apply(effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(OTHER_EFFECT_ID, 0), Bus::System, 1.0, true));
    assert_eq!(sounding(&m), INSTANCES as usize + 1);
    m.apply(Command::StopId { id: EFFECT_ID });
    assert_eq!(sounding(&m), 1);
    render(&mut m, BUFFER_FRAMES);
    assert_eq!(m.stats().active_voices, 1);
}

#[test]
fn instances_of_one_sample_overlap_under_their_own_keys_and_cut_under_the_same_one() {
    let mut m = mixer();
    for serial in 0..INSTANCES {
        m.apply(effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, serial), Bus::System, 1.0, false));
    }
    assert_eq!(sounding(&m), INSTANCES as usize, "each instance gets a voice of its own");
    m.apply(effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, 1.0, false));
    assert_eq!(sounding(&m), INSTANCES as usize, "an instance under a key already sounding replaces it");
}

#[test]
fn effect_keys_stay_inside_the_channel_span_of_their_sample_id_and_differ_by_serial() {
    let mut seen = std::collections::HashSet::new();
    for serial in 0..=u8::MAX {
        let key = effect_key(EFFECT_ID, serial);
        assert_eq!(channel_sample_id(key), EFFECT_ID);
        assert!(seen.insert(key), "serial {serial} repeats a key");
    }
    assert_eq!(seen.len(), SERIALS);
    assert_ne!(channel_sample_id(effect_key(OTHER_EFFECT_ID, 0)), EFFECT_ID);
}

#[test]
fn an_effect_is_scaled_by_its_voice_gain_and_then_by_the_gain_of_its_bus() {
    let mut m = mixer();
    m.apply(Command::BusGain { bus: Bus::System, gain: SYSTEM_VOLUME });
    m.apply(effect(flat(LONG_RENDER_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, VOICE_GAIN, false));
    let out = left(&render(&mut m, SLEW_AND_ATTACK_FRAMES));
    let expected = centred(SAMPLE_LEVEL * VOICE_GAIN * SYSTEM_VOLUME);
    let last = *out.last().unwrap();
    assert!((last - expected).abs() < TOLERANCE, "got {last}, expected {expected}");
}

#[test]
fn an_effect_leaves_the_keysound_bus_and_its_voices_exactly_as_they_were() {
    let keysound = || Command::Play {
        sample: flat(LONG_RENDER_FRAMES, OUT_RATE, SAMPLE_LEVEL),
        gain: 1.0,
        pan: 0.0,
        pitch: 1.0,
        key: channel_key(KEYSOUND_KEY, 1.0),
        at_frame: 0,
        bus: Bus::Key,
    };
    let looped_effect = || effect(flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL), effect_key(EFFECT_ID, 0), Bus::System, 1.0, true);

    let mut alone = mixer();
    alone.apply(keysound());
    let keysound_only = left(&render(&mut alone, SLEW_AND_ATTACK_FRAMES));

    let mut effect_only = mixer();
    effect_only.apply(looped_effect());
    let effect_only = left(&render(&mut effect_only, SLEW_AND_ATTACK_FRAMES));

    let mut both = mixer();
    both.apply(keysound());
    both.apply(looped_effect());
    let together = left(&render(&mut both, SLEW_AND_ATTACK_FRAMES));
    for (frame, value) in together.iter().enumerate() {
        assert!((value - (keysound_only[frame] + effect_only[frame])).abs() < TOLERANCE, "frame {frame}: the effect changed the keysound");
    }

    let effect_span = (EFFECT_ID * CHANNELS_PER_SAMPLE_ID, (EFFECT_ID + 1) * CHANNELS_PER_SAMPLE_ID);
    both.apply(Command::StopRange { lo_key: effect_span.0, hi_key: effect_span.1 });
    render(&mut both, BUFFER_FRAMES);
    assert_eq!(both.stats().active_voices, 1, "clearing the effect span must not touch the keysound");
    assert_eq!(both.voices.iter().find(|v| v.active).map(|v| v.bus), Some(Bus::Key));
}

#[test]
fn a_plain_play_never_loops() {
    let mut m = mixer();
    m.apply(Command::Play {
        sample: flat(SHORT_SAMPLE_FRAMES, OUT_RATE, SAMPLE_LEVEL),
        gain: 1.0,
        pan: 0.0,
        pitch: 1.0,
        key: channel_key(KEYSOUND_KEY, 1.0),
        at_frame: 0,
        bus: Bus::Bg,
    });
    render(&mut m, LONG_RENDER_FRAMES);
    assert_eq!(m.stats().active_voices, 0);
}

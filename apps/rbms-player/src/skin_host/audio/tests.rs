//! The skin's sound bus, run against a bare mixer instead of an output device: a file really is read
//! and decoded by the worker, and what comes out is what the mixer renders.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rbms_audio::{Bus, Command, DecodedAudio, IdNamespace, Mixer, SampleData};

use super::{EffectOutput, PENDING_PLAYS_MAX, PRELOAD_VOLUME_CEILING, SKIN_SOUND_NAMESPACE, SkinSounds, is_inside_any, voice_gain};
use crate::notify::{self, Level};
use crate::skin_host::AudioRequest;
use crate::syssound::SYSTEM_SOUND_NAMESPACE;

const OUT_RATE: u32 = 48_000;
const OUT_CHANNELS: u16 = 2;
const VOICES: usize = 32;
const CHANNELS_PER_ID: u32 = 256;

const WAV_CHANNELS: u16 = 1;
const WAV_LEVEL: i16 = 16_384;
const SAMPLE_LEVEL: f32 = 0.5;
pub(super) const SHORT_FRAMES: usize = 480;
const LONG_FRAMES: usize = 48_000;
const SETTLE_FRAMES: usize = 512;
const PAST_THE_END_FRAMES: usize = SHORT_FRAMES * 4;
const MANY_LENGTHS_FRAMES: usize = SHORT_FRAMES * 30;

const SYSTEM_VOLUME: f32 = 0.25;
pub(super) const FULL_VOLUME: f32 = 1.0;
const TOLERANCE: f32 = 1e-5;

const WAIT_LIMIT: Duration = Duration::from_secs(10);
const WAIT_STEP: Duration = Duration::from_millis(2);
const QUIET_WAIT: Duration = Duration::from_millis(200);

pub(super) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms-skin-audio-tests-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A 16-bit mono PCM WAV of `frames` samples that all read `WAV_LEVEL`, at the mixer's own rate.
fn wav_bytes(frames: usize) -> Vec<u8> {
    let data_size = (frames * 2) as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_size).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&WAV_CHANNELS.to_le_bytes());
    out.extend_from_slice(&OUT_RATE.to_le_bytes());
    out.extend_from_slice(&(OUT_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
    for _ in 0..frames {
        out.extend_from_slice(&WAV_LEVEL.to_le_bytes());
    }
    out
}

pub(super) fn write_wav(dir: &Path, name: &str, frames: usize) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, wav_bytes(frames)).unwrap();
    path
}

/// A mixer standing in for the player's stream: the same bank, the same bus, the same commands.
pub(super) struct MixerOutput {
    mixer: Mixer,
    bank: HashMap<u32, Arc<SampleData>>,
    instance: u64,
    serial: u32,
}

impl MixerOutput {
    /// A stream whose system volume has already settled at `system_volume`.
    pub(super) fn new(system_volume: f32) -> MixerOutput {
        let mut out = MixerOutput { mixer: Mixer::new(OUT_RATE, OUT_CHANNELS, VOICES), bank: HashMap::new(), instance: 1, serial: 0 };
        out.mixer.apply(Command::BusGain { bus: Bus::System, gain: system_volume });
        out.render(SETTLE_FRAMES);
        out
    }

    fn render(&mut self, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; frames * OUT_CHANNELS as usize];
        self.mixer.mix(&mut out);
        out.chunks(OUT_CHANNELS as usize).map(|frame| frame[0]).collect()
    }

    fn voices(&self) -> usize {
        self.mixer.stats().active_voices as usize
    }

    /// The stream was opened again: another number, an empty bank and no voices.
    fn reopen(&mut self) {
        self.instance += 1;
        self.bank.clear();
        self.mixer = Mixer::new(OUT_RATE, OUT_CHANNELS, VOICES);
    }
}

impl EffectOutput for MixerOutput {
    fn instance_id(&self) -> u64 {
        self.instance
    }

    fn insert_decoded(&mut self, id: u32, audio: DecodedAudio) {
        self.bank.insert(id, Arc::new(SampleData { pcm: audio.samples.into(), channels: audio.channels, rate: audio.rate }));
    }

    fn play_effect(&mut self, id: u32, gain: f32, looped: bool) {
        if let Some(sample) = self.bank.get(&id).cloned() {
            let key = id * CHANNELS_PER_ID + self.serial % CHANNELS_PER_ID;
            self.serial += 1;
            self.mixer.apply(Command::PlayEffect { sample, gain, key, bus: Bus::System, looped });
        }
    }

    fn stop(&mut self, id: u32) {
        self.mixer.apply(Command::StopId { id });
    }

    fn clear_namespace(&mut self, namespace: IdNamespace) {
        self.bank.retain(|id, _| !namespace.contains(*id));
        self.mixer.apply(Command::StopRange { lo_key: namespace.base * CHANNELS_PER_ID, hi_key: (namespace.base + namespace.len) * CHANNELS_PER_ID });
    }
}

pub(super) fn play(path: &Path, volume: f32, looped: bool) -> AudioRequest {
    AudioRequest::Play { path: path.to_path_buf(), volume, looped }
}

fn stop(path: &Path) -> AudioRequest {
    AudioRequest::Stop { path: path.to_path_buf() }
}

/// Settles the sounds a frame at a time until `done` holds of the stream. Panics when it never does.
fn wait_until(sounds: &mut SkinSounds, out: &mut MixerOutput, what: &str, done: impl Fn(&MixerOutput) -> bool) {
    let started = Instant::now();
    loop {
        sounds.settle(out);
        if done(out) {
            return;
        }
        assert!(started.elapsed() < WAIT_LIMIT, "gave up waiting until {what}");
        std::thread::sleep(WAIT_STEP);
    }
}

/// Lets the worker have as long as it could want, so that a sound that was going to arrive has.
fn let_the_worker_finish(sounds: &mut SkinSounds, out: &mut MixerOutput) {
    let started = Instant::now();
    while started.elapsed() < QUIET_WAIT {
        sounds.settle(out);
        std::thread::sleep(WAIT_STEP);
    }
}

/// What the process-wide message queue holds about files under `dir`. The queue is shared with every
/// test that reports something, so only the messages that name this test's own folder are its own.
fn told_about(dir: &Path) -> Vec<(Level, String)> {
    let mut told = Vec::new();
    notify::drain(&mut told);
    told.retain(|(_, message)| message.contains(&dir.display().to_string()));
    told
}

fn centred(level: f32) -> f32 {
    level * std::f32::consts::FRAC_1_SQRT_2
}

fn sample_id_of(sounds: &SkinSounds, path: &Path) -> u32 {
    sounds.entries.get(path).expect("the sound has an entry").sample
}

#[test]
fn a_sound_is_decoded_off_the_calling_thread_and_played_once_it_arrives_and_then_leaves_the_bus() {
    let dir = temp_dir("play-once");
    let wav = write_wav(&dir, "hit.wav", SHORT_FRAMES);
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &[dir], play(&wav, FULL_VOLUME, false));
    assert_eq!(out.voices(), 0, "nothing can sound before the file is decoded");
    assert!(out.bank.is_empty(), "the frame that asked must not have decoded the file");

    wait_until(&mut sounds, &mut out, "the sound plays", |out| out.voices() == 1);
    let heard = out.render(SHORT_FRAMES / 2);
    assert!(heard.iter().any(|&sample| sample != 0.0), "the sound must be audible while it plays");
    out.render(PAST_THE_END_FRAMES);
    assert_eq!(out.voices(), 0, "a sound that has finished must be off the bus");
}

#[test]
fn a_sound_asked_for_again_plays_at_once_from_the_copy_in_the_bank_and_overlaps_itself() {
    let dir = temp_dir("overlap");
    let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    wait_until(&mut sounds, &mut out, "the sound plays", |out| out.voices() == 1);
    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    assert_eq!(out.voices(), 2, "the second request must start without waiting for anything");
    assert_eq!(sounds.known(), 1);
    assert_eq!(out.bank.len(), 1, "the file is decoded once however often it is played");
}

#[test]
fn a_looped_sound_goes_on_sounding_until_it_is_stopped_and_a_second_loop_adds_nothing() {
    let dir = temp_dir("loop");
    let wav = write_wav(&dir, "bgm.wav", SHORT_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, true));
    wait_until(&mut sounds, &mut out, "the loop starts", |out| out.voices() == 1);
    out.render(MANY_LENGTHS_FRAMES);
    assert_eq!(out.voices(), 1, "a loop must outlast many lengths of its file");

    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, true));
    assert_eq!(out.voices(), 1, "a loop asked of a sound that is already looping is not layered");

    sounds.request(&mut out, &roots, stop(&wav));
    out.render(SETTLE_FRAMES);
    assert_eq!(out.voices(), 0, "a stopped loop must leave the bus");
    assert!(out.render(SETTLE_FRAMES).iter().all(|&sample| sample == 0.0));

    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, true));
    assert_eq!(out.voices(), 1, "a loop that was stopped can be started again");
}

#[test]
fn a_stop_stops_every_copy_of_the_sound_including_the_one_shots() {
    let dir = temp_dir("stop-all");
    let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    wait_until(&mut sounds, &mut out, "the sound plays", |out| out.voices() == 1);
    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, true));
    assert_eq!(out.voices(), 3);

    sounds.request(&mut out, &roots, stop(&wav));
    out.render(SETTLE_FRAMES);
    assert_eq!(out.voices(), 0);
}

#[test]
fn a_stop_asked_before_the_file_has_arrived_drops_the_plays_waiting_for_it() {
    let dir = temp_dir("stop-early");
    let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, true));
    sounds.request(&mut out, &roots, stop(&wav));
    wait_until(&mut sounds, &mut out, "the file is in the bank", |out| out.bank.len() == 1);
    assert_eq!(out.voices(), 0, "what was stopped before it arrived must never sound");
    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    assert_eq!(out.voices(), 1, "the file is kept all the same");
}

#[test]
fn plays_asked_while_the_file_is_still_being_read_are_bounded_and_only_one_loop_is_kept() {
    let dir = temp_dir("pending");
    let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    for _ in 0..PENDING_PLAYS_MAX + 5 {
        sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    }
    sounds.request(&mut out, &roots, stop(&wav));
    for _ in 0..PENDING_PLAYS_MAX + 5 {
        sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
    }
    wait_until(&mut sounds, &mut out, "the file is in the bank", |out| out.bank.len() == 1);
    assert_eq!(out.voices(), PENDING_PLAYS_MAX);

    let looped = write_wav(&temp_dir("pending-loop"), "bgm.wav", LONG_FRAMES);
    let loop_roots = [looped.parent().unwrap().to_path_buf()];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));
    for _ in 0..PENDING_PLAYS_MAX {
        sounds.request(&mut out, &loop_roots, play(&looped, FULL_VOLUME, true));
    }
    wait_until(&mut sounds, &mut out, "the file is in the bank", |out| out.bank.len() == 1);
    assert_eq!(out.voices(), 1);
}

#[test]
fn a_volume_is_held_to_zero_through_two_and_then_multiplied_by_the_system_volume() {
    let cases = [(1.0, 1.0), (0.5, 0.5), (2.0, 2.0), (5.0, 2.0), (f32::INFINITY, 2.0)];
    for (case, (asked, voice)) in cases.into_iter().enumerate() {
        let dir = temp_dir(&format!("volume-{case}"));
        let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
        let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(SYSTEM_VOLUME));

        sounds.request(&mut out, &[dir], play(&wav, asked, false));
        wait_until(&mut sounds, &mut out, "the sound plays", |out| out.voices() == 1);
        let heard = *out.render(SETTLE_FRAMES).last().unwrap();
        let expected = centred(SAMPLE_LEVEL * voice * SYSTEM_VOLUME);
        assert!((heard - expected).abs() < TOLERANCE, "asked for {asked}: heard {heard}, expected {expected}");
    }
}

#[test]
fn the_system_volume_is_the_bus_and_follows_the_setting_while_a_sound_is_sounding() {
    let dir = temp_dir("volume-live");
    let wav = write_wav(&dir, "bgm.wav", SHORT_FRAMES);
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(SYSTEM_VOLUME));

    sounds.request(&mut out, &[dir], play(&wav, FULL_VOLUME, true));
    wait_until(&mut sounds, &mut out, "the loop starts", |out| out.voices() == 1);
    let before = *out.render(SETTLE_FRAMES).last().unwrap();
    out.mixer.apply(Command::BusGain { bus: Bus::System, gain: FULL_VOLUME });
    let after = *out.render(SETTLE_FRAMES).last().unwrap();
    assert!((before - centred(SAMPLE_LEVEL * SYSTEM_VOLUME)).abs() < TOLERANCE, "got {before}");
    assert!((after - centred(SAMPLE_LEVEL * FULL_VOLUME)).abs() < TOLERANCE, "got {after}");
}

#[test]
fn a_volume_too_quiet_to_be_a_play_decodes_and_keeps_the_file_without_a_sound() {
    let quiet = [0.0001, 0.0, PRELOAD_VOLUME_CEILING, -1.0, f32::NAN];
    for (case, volume) in quiet.into_iter().enumerate() {
        for looped in [false, true] {
            let dir = temp_dir(&format!("preload-{case}-{looped}"));
            let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
            let roots = [dir];
            let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

            sounds.request(&mut out, &roots, play(&wav, volume, looped));
            wait_until(&mut sounds, &mut out, "the file is in the bank", |out| out.bank.len() == 1);
            assert_eq!(out.voices(), 0, "a play of {volume} (looped: {looped}) must not start a voice");
            assert!(out.render(SETTLE_FRAMES).iter().all(|&sample| sample == 0.0), "a play of {volume} must be silent");

            sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
            assert_eq!(out.voices(), 1, "the preloaded file must play at once");
        }
    }
}

#[test]
fn a_preload_request_decodes_and_keeps_the_file_without_a_sound() {
    let dir = temp_dir("preload-request");
    let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &[dir], AudioRequest::Preload { path: wav.clone() });
    wait_until(&mut sounds, &mut out, "the file is in the bank", |out| out.bank.len() == 1);
    assert_eq!(out.voices(), 0);
    assert!(out.render(SETTLE_FRAMES).iter().all(|&sample| sample == 0.0));
}

#[test]
fn a_path_outside_the_skin_folder_is_ignored_whatever_it_is_spelled_like() {
    let dir = temp_dir("confine");
    let skin = dir.join("skin");
    let elsewhere = dir.join("elsewhere");
    std::fs::create_dir_all(&skin).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    let outside = write_wav(&elsewhere, "hit.wav", SHORT_FRAMES);
    let roots = [skin.clone()];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    let spellings = [outside.clone(), skin.join("..").join("elsewhere").join("hit.wav"), PathBuf::from("hit.wav")];
    for spelling in spellings {
        sounds.request(&mut out, &roots, play(&spelling, FULL_VOLUME, false));
        sounds.request(&mut out, &roots, play(&spelling, FULL_VOLUME, true));
        sounds.request(&mut out, &roots, AudioRequest::Preload { path: spelling });
    }
    let_the_worker_finish(&mut sounds, &mut out);
    assert_eq!(sounds.known(), 0, "a path outside the folder must not even be remembered");
    assert!(sounds.worker.is_none(), "a path outside the folder must not reach the decoder");
    assert_eq!(out.voices(), 0);
    assert!(out.bank.is_empty());
}

#[cfg(unix)]
#[test]
fn a_link_inside_the_skin_folder_that_leads_outside_it_is_refused_by_the_decoder() {
    notify::exclusive(|| {
        let dir = temp_dir("confine-link");
        let skin = dir.join("skin");
        std::fs::create_dir_all(&skin).unwrap();
        let outside = write_wav(&dir, "secret.wav", SHORT_FRAMES);
        let link = skin.join("hit.wav");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

        sounds.request(&mut out, &[skin], play(&link, FULL_VOLUME, false));
        let_the_worker_finish(&mut sounds, &mut out);
        assert_eq!(out.voices(), 0);
        assert!(out.bank.is_empty(), "a file outside the folder must never reach the bank");
        let told = told_about(&dir);
        assert!(told.iter().any(|(level, message)| *level == Level::Warn && message.contains("outside the skin folder")), "got {told:?}");
    });
}

#[test]
fn a_file_that_is_not_there_is_reported_once_and_not_asked_for_again_until_it_is_disposed_of() {
    notify::exclusive(|| {
        let dir = temp_dir("missing");
        let wav = dir.join("hit.wav");
        let roots = [dir.clone()];
        let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

        sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
        let started = Instant::now();
        let mut told = Vec::new();
        while told.is_empty() {
            sounds.settle(&mut out);
            told = told_about(&dir);
            assert!(started.elapsed() < WAIT_LIMIT, "the failure was never reported");
            std::thread::sleep(WAIT_STEP);
        }
        assert_eq!(told.len(), 1);
        assert!(told[0].1.contains("no such file"), "got {:?}", told[0]);

        sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
        let_the_worker_finish(&mut sounds, &mut out);
        let again = told_about(&dir);
        assert!(again.is_empty(), "a file that failed must not be tried on every request: {again:?}");

        write_wav(&dir, "hit.wav", SHORT_FRAMES);
        sounds.request(&mut out, &roots, AudioRequest::Dispose { path: wav.clone() });
        sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, false));
        wait_until(&mut sounds, &mut out, "the sound plays", |out| out.voices() == 1);
    });
}

#[test]
fn a_file_the_skin_names_in_one_container_is_found_in_another_the_way_the_reference_does() {
    let dir = temp_dir("fallback");
    write_wav(&dir, "hit.wav", SHORT_FRAMES);
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, std::slice::from_ref(&dir), play(&dir.join("hit.ogg"), FULL_VOLUME, false));
    wait_until(&mut sounds, &mut out, "the sound plays", |out| out.voices() == 1);
}

#[test]
fn leaving_the_screen_stops_everything_the_skin_played_and_lets_go_of_what_it_decoded() {
    let dir = temp_dir("leave");
    let looped = write_wav(&dir, "bgm.wav", SHORT_FRAMES);
    let once = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &roots, play(&looped, FULL_VOLUME, true));
    sounds.request(&mut out, &roots, play(&once, FULL_VOLUME, false));
    wait_until(&mut sounds, &mut out, "both sound", |out| out.voices() == 2);
    assert_eq!(out.bank.len(), 2);

    sounds.leave(Some(&mut out));
    out.render(SETTLE_FRAMES);
    assert_eq!(out.voices(), 0, "nothing the skin played may go on sounding");
    assert!(out.bank.is_empty(), "what the skin decoded must be let go of");
    assert_eq!(sounds.known(), 0);

    sounds.request(&mut out, &roots, play(&once, FULL_VOLUME, false));
    assert_eq!(out.voices(), 0, "the next screen decodes again instead of finding the old copy");
    wait_until(&mut sounds, &mut out, "the sound plays", |out| out.voices() == 1);
}

#[test]
fn a_file_still_being_read_when_the_screen_is_left_is_thrown_away_when_it_arrives() {
    let dir = temp_dir("leave-early");
    let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &[dir], play(&wav, FULL_VOLUME, true));
    sounds.leave(Some(&mut out));
    let_the_worker_finish(&mut sounds, &mut out);
    assert_eq!(out.voices(), 0, "a sound of the screen that was left must not start");
    assert!(out.bank.is_empty());
    assert_eq!(sounds.known(), 0);
}

#[test]
fn leaving_with_no_stream_forgets_everything_without_one() {
    let dir = temp_dir("leave-silent");
    let wav = write_wav(&dir, "hit.wav", SHORT_FRAMES);
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &[dir], play(&wav, FULL_VOLUME, false));
    assert_eq!(sounds.known(), 1);
    sounds.leave(None);
    assert_eq!(sounds.known(), 0);
}

#[test]
fn a_stream_that_was_opened_again_has_nothing_in_its_bank_so_the_skin_decodes_again() {
    let dir = temp_dir("reopen");
    let wav = write_wav(&dir, "hit.wav", LONG_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, true));
    wait_until(&mut sounds, &mut out, "the loop starts", |out| out.voices() == 1);

    out.reopen();
    sounds.request(&mut out, &roots, play(&wav, FULL_VOLUME, true));
    assert_eq!(out.voices(), 0, "the new stream has not got the sound yet");
    wait_until(&mut sounds, &mut out, "the loop starts on the new stream", |out| out.voices() == 1);
    assert_eq!(out.bank.len(), 1);
}

#[test]
fn disposing_of_a_sound_stops_it_takes_it_out_of_the_bank_and_gives_its_id_to_the_next() {
    let dir = temp_dir("dispose");
    let first = write_wav(&dir, "first.wav", SHORT_FRAMES);
    let second = write_wav(&dir, "second.wav", SHORT_FRAMES);
    let roots = [dir];
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &roots, play(&first, FULL_VOLUME, true));
    wait_until(&mut sounds, &mut out, "the loop starts", |out| out.voices() == 1);
    let id = sample_id_of(&sounds, &first);

    sounds.request(&mut out, &roots, AudioRequest::Dispose { path: first.clone() });
    out.render(SETTLE_FRAMES);
    assert_eq!(out.voices(), 0);
    assert!(out.bank.is_empty());
    assert_eq!(sounds.known(), 0);

    sounds.request(&mut out, &roots, play(&second, FULL_VOLUME, false));
    assert_eq!(sample_id_of(&sounds, &second), id, "the id comes back into use");
    wait_until(&mut sounds, &mut out, "the second sound plays", |out| out.voices() == 1);
}

#[test]
fn a_stop_or_a_dispose_of_a_sound_that_was_never_played_is_a_no_op() {
    let dir = temp_dir("unknown");
    let wav = dir.join("never.wav");
    let (mut sounds, mut out) = (SkinSounds::default(), MixerOutput::new(FULL_VOLUME));

    sounds.request(&mut out, &[dir], stop(&wav));
    sounds.request(&mut out, &[], AudioRequest::Dispose { path: wav });
    assert_eq!(sounds.known(), 0);
    assert!(sounds.worker.is_none());
}

#[test]
fn sample_ids_come_from_the_skin_namespace_without_repeating_and_run_out_at_its_end() {
    let mut sounds = SkinSounds::default();
    let mut taken = std::collections::HashSet::new();
    for _ in 0..SKIN_SOUND_NAMESPACE.len {
        let id = sounds.take_sample_id().expect("the namespace has room");
        assert!(SKIN_SOUND_NAMESPACE.contains(id), "{id:#x} is outside the namespace");
        assert!(taken.insert(id), "{id:#x} was handed out twice");
    }
    assert_eq!(sounds.take_sample_id(), None);
    sounds.free.push(SKIN_SOUND_NAMESPACE.base);
    assert_eq!(sounds.take_sample_id(), Some(SKIN_SOUND_NAMESPACE.base));
}

#[test]
fn the_skin_namespace_overlaps_no_other_namespace() {
    let last = SKIN_SOUND_NAMESPACE.base + SKIN_SOUND_NAMESPACE.len - 1;
    for other in [IdNamespace::PLAY, IdNamespace::PREVIEW, SYSTEM_SOUND_NAMESPACE] {
        assert!(!other.contains(SKIN_SOUND_NAMESPACE.base) && !other.contains(last), "{other:?} overlaps the skin namespace");
        assert!(!SKIN_SOUND_NAMESPACE.contains(other.base) && !SKIN_SOUND_NAMESPACE.contains(other.base + other.len - 1));
    }
    assert!(
        u64::from(last) * u64::from(CHANNELS_PER_ID) + u64::from(CHANNELS_PER_ID) <= u64::from(u32::MAX),
        "the channel keys of the namespace must fit a u32"
    );
}

#[test]
fn a_volume_becomes_a_voice_gain_between_zero_and_two_or_a_preload() {
    assert_eq!(voice_gain(1.0), Some(1.0));
    assert_eq!(voice_gain(0.5), Some(0.5));
    assert_eq!(voice_gain(2.0), Some(2.0));
    assert_eq!(voice_gain(7.5), Some(2.0));
    assert_eq!(voice_gain(f32::INFINITY), Some(2.0));
    assert_eq!(voice_gain(0.0001), None);
    assert_eq!(voice_gain(PRELOAD_VOLUME_CEILING), None);
    assert_eq!(voice_gain(PRELOAD_VOLUME_CEILING * 2.0), Some(PRELOAD_VOLUME_CEILING * 2.0));
    assert_eq!(voice_gain(0.0), None);
    assert_eq!(voice_gain(-3.0), None);
    assert_eq!(voice_gain(f32::NEG_INFINITY), None);
    assert_eq!(voice_gain(f32::NAN), None);
}

#[test]
fn a_path_is_inside_a_folder_when_it_is_under_it_once_dots_are_worked_out() {
    let root = temp_dir("inside");
    let roots = [root.clone()];
    assert!(is_inside_any(&root.join("a.ogg"), &roots));
    assert!(is_inside_any(&root.join("sounds").join("a.ogg"), &roots));
    assert!(is_inside_any(&root.join("sounds").join("..").join("a.ogg"), &roots));
    assert!(is_inside_any(&root.join(".").join("a.ogg"), &roots));
    assert!(!is_inside_any(&root.join("..").join("a.ogg"), &roots));
    assert!(!is_inside_any(&root.with_file_name("rbms-skin-audio-tests-sibling").join("a.ogg"), &roots));
    assert!(!is_inside_any(&root.join("a.ogg"), &[]));
    assert!(is_inside_any(&root.join("a.ogg"), &[PathBuf::from("/nowhere"), root.clone()]));
    assert!(!is_inside_any(Path::new("/"), &roots));
}

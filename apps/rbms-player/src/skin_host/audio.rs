//! What a skin asks of the sound system: `main_state.audio_play`, `audio_loop`, `audio_preload`,
//! `audio_stop` and `audio_dispose`.
//!
//! A skin makes these requests while a frame is being drawn, when nothing may change, so the host
//! records each one as it arrives and hands them over when the frame is done
//! ([`super::ScreenHost::take_calls`]). Carrying them out belongs here ([`carry_out`]).
//!
//! # The skin's own bus
//!
//! A skin's sounds are effects on the mixer's System bus, in an id namespace of their own
//! ([`SKIN_SOUND_NAMESPACE`]) that keysounds, the select preview and the system sounds never touch,
//! so a skin can neither cut them nor be cut by them, and stopping or clearing one side leaves the
//! other as it was. The reference plays a skin's sound at `systemvolume * clamp(volume, 0, 2)`
//! (`SkinAudioLuaApiExporter.play`); here the voice is played at the clamped volume and the System
//! bus, whose gain is the system volume the settings screen sets, supplies the other factor -- so
//! the volume follows the setting while a sound is still sounding, which the reference's does not.
//!
//! A request is for a file, and the reference keeps what it decoded under the path
//! (`AbstractAudioDriver.getSound`): the first request for a path decodes it, and every later one
//! plays the copy that is in the mixer's bank. Each request for a path that is sounding starts
//! another copy, so one sound can overlap itself; `audio_stop` stops every copy, and `audio_loop`
//! asked of a sound that is already looping adds nothing (the reference would stack a second loop on
//! the first, which only makes it louder and is bounded by nothing).
//!
//! # Off the frame
//!
//! Reading and decoding a file takes longer than a frame, so it is done on a worker thread
//! ([`SkinSounds::settle`] collects what it finished, once a frame). A request that arrives for a
//! sound still being decoded is remembered and played the moment the sound is in; one that is
//! stopped before that is forgotten. A file that is not where the skin says is looked for under the
//! same name with the other containers (`AudioDriver.getPaths`: wav, flac, ogg, mp3).
//!
//! # Preloading
//!
//! `audio_preload` is a play at volume zero in the reference: it decodes and caches, and sounds
//! nothing. A skin also preloads by playing at a volume of `0.0001`, which is inaudible there. Any
//! volume at or below [`PRELOAD_VOLUME_CEILING`] is taken for a preload here: the file is decoded
//! and kept, and no voice is started at all, so a preload of a sound that is looped does not leave a
//! silent loop running forever.
//!
//! # Confinement
//!
//! The skin's interpreter already refuses a path that leaves the skin's folder (`SkinPaths`). That is
//! checked again here, first by comparing the paths and then, by the worker, on the real file after
//! symbolic links are resolved ([`decode_skin_sound`]), because this is the code that opens the file.
//! A path outside is dropped without a word.
//!
//! # Leaving a screen
//!
//! The reference disposes of a screen's skin on every change of screen. [`AppShared::end_scene_sounds`]
//! is run when a scene begins: it stops everything the skin was playing, empties the bank of it and
//! forgets what was decoded, and a file still being decoded for the screen that was left is dropped
//! when it arrives. The system cues are left alone: each is played once, and the one raised on the way
//! out of a screen is still sounding as the next begins.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use rbms_audio::{AudioEngine, Bus, DecodedAudio, IdNamespace};

use super::AudioRequest;
use crate::AppShared;
use crate::notify::{Level, notify};
use crate::skin_select::skin_root;

/// Sample ids reserved for a skin's sounds, kept clear of chart keysounds ([`IdNamespace::PLAY`]),
/// song-select previews ([`IdNamespace::PREVIEW`]) and system sounds
/// ([`crate::syssound::SYSTEM_SOUND_NAMESPACE`]), so one engine holds all four at once.
pub(crate) const SKIN_SOUND_NAMESPACE: IdNamespace = IdNamespace { base: 0x00A0_0000, len: 0x0000_1000 };

/// The range a skin's volume is held to (`MathUtils.clamp(volume, 0.0f, 2.0f)`).
const SKIN_VOLUME_MIN: f32 = 0.0;
const SKIN_VOLUME_MAX: f32 = 2.0;

/// The volume at or below which a play is a preload. A skin writes `0.0001` for it.
pub(crate) const PRELOAD_VOLUME_CEILING: f32 = 0.001;

/// Plays remembered for a sound that is still being decoded. A skin that asks again every frame
/// while its file is read gets a handful of copies, not one per frame.
const PENDING_PLAYS_MAX: usize = 8;

/// The containers looked for when the file a skin names is not there, in the reference's order
/// (`AudioDriver.getPaths`).
const FALLBACK_EXTENSIONS: [&str; 4] = ["wav", "flac", "ogg", "mp3"];

/// What the worker thread that decodes a skin's sounds is called.
const DECODE_THREAD_NAME: &str = "rbms-skin-sound-decode";

/// Where a skin's sounds are played, and what is known about the stream they are played on.
///
/// The player's stream implements it; the tests implement it on a bare mixer, so the whole of this
/// module is exercised without an output device.
pub(crate) trait EffectOutput {
    /// Which stream this is. A stream that was opened again has an empty bank.
    fn instance_id(&self) -> u64;

    /// Put a decoded sound in the bank under `id`.
    fn insert_decoded(&mut self, id: u32, audio: DecodedAudio);

    /// Start one copy of the sound under `id`, repeating when `looped`, at the voice volume `gain`.
    fn play_effect(&mut self, id: u32, gain: f32, looped: bool);

    /// Stop every copy of the sound under `id`.
    fn stop(&mut self, id: u32);

    /// Stop everything in `namespace` and empty the bank of it.
    fn clear_namespace(&mut self, namespace: IdNamespace);
}

impl EffectOutput for AudioEngine {
    fn instance_id(&self) -> u64 {
        AudioEngine::instance_id(self)
    }

    fn insert_decoded(&mut self, id: u32, audio: DecodedAudio) {
        AudioEngine::insert_decoded(self, id, audio);
    }

    fn play_effect(&mut self, id: u32, gain: f32, looped: bool) {
        AudioEngine::play_effect(self, Bus::System, id, gain, looped);
    }

    fn stop(&mut self, id: u32) {
        AudioEngine::stop(self, id);
    }

    fn clear_namespace(&mut self, namespace: IdNamespace) {
        AudioEngine::clear_namespace(self, namespace);
    }
}

/// A play asked for while the sound was still being decoded.
struct Pending {
    gain: f32,
    looped: bool,
}

/// How far a sound has got.
enum Load {
    /// The worker has the file. Whatever was asked for meanwhile waits here.
    Decoding(Vec<Pending>),
    /// The sound is in the bank.
    Ready,
    /// The file could not be found, read or decoded. It is not tried again until the screen is left
    /// or the skin disposes of it.
    Failed,
}

/// One sound a skin has asked for, kept under its path.
struct Entry {
    /// The sample id the sound is in the bank under.
    sample: u32,
    load: Load,
    /// Whether a loop of it has been started and not stopped since.
    looping: bool,
}

/// A file for the worker to read and decode.
struct Job {
    epoch: u64,
    path: PathBuf,
    roots: Vec<PathBuf>,
}

/// What the worker made of a [`Job`].
struct Done {
    epoch: u64,
    path: PathBuf,
    result: Result<DecodedAudio, String>,
}

/// The thread that decodes, and the two ends the frame talks to it by.
struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

/// Every sound a skin has asked for since its screen began, and the worker that decodes them.
#[derive(Default)]
pub(crate) struct SkinSounds {
    entries: HashMap<PathBuf, Entry>,
    /// Sample ids given back by [`AudioRequest::Dispose`], handed out again before a new one is.
    free: Vec<u32>,
    /// How many ids of the namespace have been handed out.
    issued: u32,
    /// The stream the entries were installed in.
    instance: Option<u64>,
    /// Which generation of the skin's sounds is current. A file decoded for an older one is thrown
    /// away, and the worker does not start one.
    epoch: Arc<AtomicU64>,
    worker: Option<Worker>,
    /// Whether the namespace running out has been reported.
    reported_full: bool,
}

impl SkinSounds {
    /// Carries out one request a skin made during the frame that has just ended.
    ///
    /// `roots` are the folders a skin may play files from. Nothing here waits for a file: a sound that
    /// is not decoded yet is handed to the worker and played by [`SkinSounds::settle`] when it is in.
    pub(crate) fn request(&mut self, out: &mut dyn EffectOutput, roots: &[PathBuf], request: AudioRequest) {
        self.adopt(out);
        match request {
            AudioRequest::Play { path, volume, looped } => {
                if !is_inside_any(&path, roots) {
                    return;
                }
                match voice_gain(volume) {
                    Some(gain) => self.play(out, roots, path, gain, looped),
                    None => self.preload(roots, path),
                }
            }
            AudioRequest::Preload { path } => {
                if is_inside_any(&path, roots) {
                    self.preload(roots, path);
                }
            }
            AudioRequest::Stop { path } => self.stop(out, &path),
            AudioRequest::Dispose { path } => self.dispose(out, &path),
        }
    }

    /// Takes what the worker has finished, puts it in the bank and plays what was waiting for it.
    /// Run once a frame.
    pub(crate) fn settle(&mut self, out: &mut dyn EffectOutput) {
        self.adopt(out);
        let Some(worker) = &self.worker else {
            return;
        };
        let arrived: Vec<Done> = worker.done.try_iter().collect();
        let current = self.epoch.load(Ordering::Relaxed);
        for done in arrived.into_iter().filter(|done| done.epoch == current) {
            self.finish(out, done);
        }
    }

    /// Stops everything the skin was playing, empties the bank of it and forgets what was decoded.
    /// Run when a scene begins. With no stream there is nothing sounding to stop.
    pub(crate) fn leave(&mut self, out: Option<&mut dyn EffectOutput>) {
        if let Some(out) = out {
            out.clear_namespace(SKIN_SOUND_NAMESPACE);
            self.instance = Some(out.instance_id());
        }
        self.forget_everything();
    }

    /// How many sounds are known, decoded or not, for the tests.
    #[cfg(test)]
    fn known(&self) -> usize {
        self.entries.len()
    }

    /// Notes which stream the entries belong to, and starts over when it is another one: its bank is
    /// empty, and whatever was being decoded for the old one is of no use.
    fn adopt(&mut self, out: &dyn EffectOutput) {
        let instance = out.instance_id();
        if self.instance != Some(instance) {
            self.instance = Some(instance);
            self.forget_everything();
        }
    }

    fn forget_everything(&mut self) {
        self.entries.clear();
        self.free.clear();
        self.issued = 0;
        self.reported_full = false;
        self.epoch.fetch_add(1, Ordering::Relaxed);
    }

    /// `audio_play` and `audio_loop`.
    fn play(&mut self, out: &mut dyn EffectOutput, roots: &[PathBuf], path: PathBuf, gain: f32, looped: bool) {
        if !self.ensure_entry(roots, &path) {
            return;
        }
        let Some(entry) = self.entries.get_mut(&path) else {
            return;
        };
        match &mut entry.load {
            Load::Ready => start(out, entry, gain, looped),
            Load::Decoding(plays) => {
                let repeats_a_loop = looped && plays.iter().any(|play| play.looped);
                if plays.len() < PENDING_PLAYS_MAX && !repeats_a_loop {
                    plays.push(Pending { gain, looped });
                }
            }
            Load::Failed => {}
        }
    }

    /// `audio_preload`, and a play too quiet to be one.
    fn preload(&mut self, roots: &[PathBuf], path: PathBuf) {
        self.ensure_entry(roots, &path);
    }

    /// `audio_stop`: every copy of the sound stops, and a play still waiting for the file is dropped.
    fn stop(&mut self, out: &mut dyn EffectOutput, path: &Path) {
        let Some(entry) = self.entries.get_mut(path) else {
            return;
        };
        entry.looping = false;
        if let Load::Decoding(plays) = &mut entry.load {
            plays.clear();
        }
        out.stop(entry.sample);
    }

    /// `audio_dispose`: the sound stops and leaves the bank, and the next request decodes it again.
    fn dispose(&mut self, out: &mut dyn EffectOutput, path: &Path) {
        let Some(entry) = self.entries.remove(path) else {
            return;
        };
        out.clear_namespace(IdNamespace { base: entry.sample, len: 1 });
        self.free.push(entry.sample);
    }

    /// Makes sure the sound has an entry, handing the file to the worker when it is new. Whether it
    /// has one afterwards.
    fn ensure_entry(&mut self, roots: &[PathBuf], path: &Path) -> bool {
        if self.entries.contains_key(path) {
            return true;
        }
        let Some(sample) = self.take_sample_id() else {
            if !std::mem::replace(&mut self.reported_full, true) {
                notify(Level::Warn, format!("[skin audio] more than {} sounds on one screen, the rest are not played", SKIN_SOUND_NAMESPACE.len));
            }
            return false;
        };
        let load = if self.send_job(roots, path) {
            Load::Decoding(Vec::new())
        } else {
            notify(Level::Warn, format!("[skin audio] cannot start the decoder for {}", path.display()));
            Load::Failed
        };
        self.entries.insert(path.to_path_buf(), Entry { sample, load, looping: false });
        true
    }

    fn take_sample_id(&mut self) -> Option<u32> {
        if let Some(freed) = self.free.pop() {
            return Some(freed);
        }
        if self.issued >= SKIN_SOUND_NAMESPACE.len {
            return None;
        }
        self.issued += 1;
        Some(SKIN_SOUND_NAMESPACE.base + self.issued - 1)
    }

    fn send_job(&mut self, roots: &[PathBuf], path: &Path) -> bool {
        if self.worker.is_none() {
            self.worker = spawn_worker(Arc::clone(&self.epoch));
        }
        let Some(worker) = &self.worker else {
            return false;
        };
        worker.jobs.send(Job { epoch: self.epoch.load(Ordering::Relaxed), path: path.to_path_buf(), roots: roots.to_vec() }).is_ok()
    }

    /// Puts one decoded file in the bank and plays what asked for it while it was being read.
    fn finish(&mut self, out: &mut dyn EffectOutput, done: Done) {
        let Some(entry) = self.entries.get_mut(&done.path) else {
            return;
        };
        let Load::Decoding(plays) = &mut entry.load else {
            return;
        };
        let plays = std::mem::take(plays);
        match done.result {
            Ok(audio) => {
                out.insert_decoded(entry.sample, audio);
                entry.load = Load::Ready;
                for play in plays {
                    start(out, entry, play.gain, play.looped);
                }
            }
            Err(reason) => {
                entry.load = Load::Failed;
                notify(Level::Warn, format!("[skin audio] {}: {reason}", done.path.display()));
            }
        }
    }
}

/// Starts one copy of a sound that is in the bank. A loop is not started over one that is running.
fn start(out: &mut dyn EffectOutput, entry: &mut Entry, gain: f32, looped: bool) {
    if looped {
        if entry.looping {
            return;
        }
        entry.looping = true;
    }
    out.play_effect(entry.sample, gain, looped);
}

/// The volume a voice is played at for a volume a skin asked for, or `None` when the request is
/// too quiet to be a play ([`PRELOAD_VOLUME_CEILING`]) or is not a number.
pub(crate) fn voice_gain(volume: f32) -> Option<f32> {
    if volume.is_nan() {
        return None;
    }
    let gain = volume.clamp(SKIN_VOLUME_MIN, SKIN_VOLUME_MAX);
    (gain > PRELOAD_VOLUME_CEILING).then_some(gain)
}

/// The path with `.` and `..` worked out, made absolute, or `None` when `..` would leave the root.
fn lexically_normalized(path: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(path).ok()?;
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Some(normalized)
}

/// Whether the path, as written, lies under one of the folders.
pub(crate) fn is_inside_any(path: &Path, roots: &[PathBuf]) -> bool {
    let Some(path) = lexically_normalized(path) else {
        return false;
    };
    roots.iter().filter_map(|root| lexically_normalized(root)).any(|root| path.starts_with(root))
}

/// Whether the file, once every link on the way is followed, lies under one of the folders.
fn is_really_inside_any(file: &Path, roots: &[PathBuf]) -> bool {
    let Ok(real) = file.canonicalize() else {
        return false;
    };
    roots.iter().filter_map(|root| root.canonicalize().ok()).any(|root| real.starts_with(root))
}

/// The files a path can mean: itself when it is there, then the same name in the other containers.
fn candidate_files(path: &Path) -> Vec<PathBuf> {
    let own = path.extension().and_then(OsStr::to_str);
    let alternatives = FALLBACK_EXTENSIONS.into_iter().filter(|extension| own != Some(*extension)).map(|extension| path.with_extension(extension));
    std::iter::once(path.to_path_buf()).chain(alternatives).filter(|file| file.is_file()).collect()
}

/// Reads and decodes the first of the files `path` can mean that is inside a root and decodes.
fn decode_skin_sound(path: &Path, roots: &[PathBuf]) -> Result<DecodedAudio, String> {
    let mut failure = "no such file".to_owned();
    for file in candidate_files(path) {
        if !is_really_inside_any(&file, roots) {
            failure = format!("{} lies outside the skin folder", file.display());
            continue;
        }
        let bytes = match std::fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) => {
                failure = format!("cannot read {}: {error}", file.display());
                continue;
            }
        };
        match rbms_audio::decode_bytes(bytes, file.extension().and_then(OsStr::to_str)) {
            Ok(audio) => return Ok(audio),
            Err(error) => failure = format!("cannot decode {}: {error}", file.display()),
        }
    }
    Err(failure)
}

/// Starts the thread that decodes, which ends when the [`Worker`] is dropped. `None` when the
/// machine would not give one.
fn spawn_worker(epoch: Arc<AtomicU64>) -> Option<Worker> {
    let (jobs, queued) = channel::<Job>();
    let (finished, done) = channel::<Done>();
    std::thread::Builder::new()
        .name(DECODE_THREAD_NAME.to_owned())
        .spawn(move || {
            while let Ok(job) = queued.recv() {
                if epoch.load(Ordering::Relaxed) != job.epoch {
                    continue;
                }
                let result = decode_skin_sound(&job.path, &job.roots);
                if finished.send(Done { epoch: job.epoch, path: job.path, result }).is_err() {
                    break;
                }
            }
        })
        .ok()?;
    Some(Worker { jobs, done })
}

/// The folders a skin may play files from: the pack the run draws, and the skin folder.
fn skin_roots(shared: &AppShared) -> Vec<PathBuf> {
    let pack = shared.launch.skin_pack.as_deref().or_else(|| shared.config.skin.pack_folder().map(Path::new));
    pack.into_iter().map(Path::to_path_buf).chain(std::iter::once(skin_root(&shared.settings_path, &shared.config))).collect()
}

/// Carries out one sound request a skin made during the frame that has just ended.
///
/// A skin asks while its screen is drawn, which can be before the stream is open or after it
/// failed to; a request with no stream to play on is dropped.
pub(crate) fn carry_out(shared: &mut AppShared, request: AudioRequest) {
    let roots = skin_roots(shared);
    let AppShared { audio, skin_sounds, .. } = shared;
    if let Some(engine) = audio.as_mut() {
        skin_sounds.request(engine, &roots, request);
    }
}

impl AppShared {
    /// Plays what a skin asked for that has been decoded since the last frame. Run once a frame, after
    /// the frame's requests are carried out.
    pub(crate) fn settle_skin_sounds(&mut self) {
        let AppShared { audio, skin_sounds, .. } = self;
        if let Some(engine) = audio.as_mut() {
            skin_sounds.settle(engine);
        }
    }

    /// Silences what the scene that is ending was playing: the skin's sounds, all of them. A system
    /// cue is played once and left to finish: it was raised on the way out of a screen, and the screen
    /// that follows begins with it still sounding.
    pub(crate) fn end_scene_sounds(&mut self) {
        let AppShared { audio, skin_sounds, .. } = self;
        skin_sounds.leave(audio.as_mut().map(|engine| engine as &mut dyn EffectOutput));
    }
}

#[cfg(test)]
mod scene_tests;
#[cfg(test)]
mod tests;

//! The textures a compiled screen draws from: which of a document's image sources become one,
//! how large they may be, and how long they stay uploaded.
//!
//! A published skin declares a sheet for every customisation it offers and draws from a handful of
//! them, so a screen takes only the sources its assembled destinations name
//! ([`referenced_sources`]). The reference does the same by loading a source the first time an
//! object asks for it (`JSONSkinLoader.getSource`); here the answer is worked out once, ahead of the
//! decode, so a host can read exactly those files off its frame loop.
//!
//! Two things are refused on the way in, each with a line in the screen's warnings: an image longer
//! on an edge than the renderer can hold, and an image that would take the screen's textures past
//! its byte budget ([`TextureLimits`]). The objects that would have drawn from a refused source are
//! dropped one by one as they are built, exactly like the objects of a source that will not decode.
//!
//! A source whose file is a movie is not decoded here at all. The host says how large its frames
//! are when the screen is built -- which is what the movie is counted at against the budget, as one
//! frame -- and hands over the frame that is on show whenever it has a new one
//! ([`SkinScreen::show_movie_frame`](super::SkinScreen::show_movie_frame)). Each frame replaces the
//! last under one registry key, so a playing movie is one texture for as long as its screen is.
//! Until its first frame arrives it is none, and the image drawn from it draws nothing.
//!
//! A screen built on its own registers its textures under keys of its own and hands them back when
//! it is released. A screen built against a [`SkinTexturePool`] shares them by file instead: two
//! screens that draw from one file hold one texture, and a texture nobody holds any more stays
//! uploaded until the pool is swept, so the screen being entered can take over what the screen
//! being left had without decoding it again (the reference's `PixmapResourcePool` keeps a file for a
//! generation for the same reason).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use rbms_skin::loader::LoadedSkin;
use rbms_skin::model::{Destination, SourceKind};

use super::{SkinAssets, SkinImage};
use crate::{BYTES_PER_PIXEL, Renderer, TextureId, locked};

#[cfg(test)]
mod movie_tests;
#[cfg(test)]
mod tests;

/// Bytes in a mebibyte, which is what a texture total is said in.
const BYTES_PER_MIB: u64 = 1024 * 1024;

/// The most RGBA bytes the textures of one screen's skin may come to: one gibibyte.
///
/// The largest screen of the pack this was measured against takes about a sixth of it, so the
/// ceiling is there for a skin that is wrong rather than one that is large.
pub const SKIN_TEXTURE_BUDGET_BYTES: u64 = 1024 * BYTES_PER_MIB;

/// Tells one pooled texture's registry key from the next, across every pool there is.
static NEXT_POOLED_TEXTURE: AtomicU32 = AtomicU32::new(0);

/// Everything the draw list needs about the image sources that were registered, each as
/// `(document source id, texture, size in pixels)`.
pub(crate) type Source<'a> = &'a [(String, TextureId, (u32, u32))];

/// A texture and its size in pixels.
pub(crate) type SizedTexture = (TextureId, (u32, u32));

/// Looks an image source up by the id a document gave it.
pub(crate) fn source_of<'a>(sources: Source<'a>, id: &str) -> Option<&'a (String, TextureId, (u32, u32))> {
    sources.iter().find(|(source, _, _)| source == id)
}

/// Adds the object id of each nested destination to `named`.
fn name_all(named: &mut BTreeSet<String>, destinations: &[Destination]) {
    named.extend(destinations.iter().map(|destination| destination.id.clone()));
}

/// The image sources the skin's drawn objects read, by source id.
///
/// An object is drawn when one of the skin's assembled destinations names it -- which leaves out
/// every object preparing the skin removed, an option the player did not pick and a condition that
/// is settled once and came out false alike -- or when a repeating object such a destination names
/// lists it: a note set, a gauge, a judgement pop-up, a song wheel, an image set. Every definition
/// with a `src` that is named either way keeps its source. Everything else the skin declares is a
/// sheet for something this load does not draw, and is never decoded.
///
/// The answer errs towards keeping: an id two kinds of object share keeps the source of both.
pub fn referenced_sources(skin: &LoadedSkin) -> BTreeSet<String> {
    let def = &skin.def;
    let mut named: BTreeSet<String> = skin.destinations.iter().map(|track| track.id.clone()).collect();
    if let Some(note) = def.note.as_ref().filter(|note| named.contains(&note.id)) {
        let lists = [
            &note.note,
            &note.lnstart,
            &note.lnend,
            &note.lnbody,
            &note.lnbody_active,
            &note.lnactive,
            &note.hcnstart,
            &note.hcnend,
            &note.hcnbody,
            &note.hcnactive,
            &note.hcnbody_active,
            &note.hcndamage,
            &note.hcnbody_miss,
            &note.hcnreactive,
            &note.hcnbody_reactive,
            &note.mine,
            &note.hidden,
            &note.processed,
        ];
        named.extend(lists.into_iter().flatten().cloned());
        for nested in [&note.group, &note.bpm, &note.stop, &note.time] {
            name_all(&mut named, nested);
        }
    }
    if let Some(gauge) = def.gauge.as_ref().filter(|gauge| named.contains(&gauge.id)) {
        named.extend(gauge.nodes.iter().cloned());
    }
    for judge in def.judge.iter().filter(|judge| named.contains(&judge.id)).collect::<Vec<_>>() {
        name_all(&mut named, &judge.images);
        name_all(&mut named, &judge.numbers);
    }
    if let Some(list) = def.songlist.as_ref().filter(|list| named.contains(&list.id)) {
        for nested in [&list.listoff, &list.liston, &list.text, &list.level, &list.lamp, &list.playerlamp, &list.rivallamp, &list.trophy, &list.label] {
            name_all(&mut named, nested);
        }
        named.extend(list.graph.iter().map(|graph| graph.id.clone()));
    }
    let from_sets: Vec<String> = def.imageset.iter().filter(|set| named.contains(&set.id)).flat_map(|set| set.images.iter().cloned()).collect();
    named.extend(from_sets);

    let mut sources = BTreeSet::new();
    let mut keep = |id: &str, src: &str| {
        if named.contains(id) {
            sources.insert(src.to_owned());
        }
    };
    def.image.iter().for_each(|image| keep(&image.id, &image.src));
    def.value.iter().for_each(|value| keep(&value.id, &value.src));
    def.floatvalue.iter().for_each(|value| keep(&value.id, &value.src));
    def.slider.iter().for_each(|slider| keep(&slider.id, &slider.src));
    def.graph.iter().for_each(|graph| keep(&graph.id, &graph.src));
    def.hidden_cover.iter().for_each(|cover| keep(&cover.id, &cover.src));
    def.lift_cover.iter().for_each(|cover| keep(&cover.id, &cover.src));
    def.pmchara.iter().for_each(|chara| keep(&chara.id, &chara.src));
    sources
}

/// The files behind the sources of one kind among [`referenced_sources`], each once however many
/// source ids resolve to it.
fn referenced_files(skin: &LoadedSkin, kind: SourceKind) -> BTreeSet<&Path> {
    let referenced = referenced_sources(skin);
    let of_kind = |id: &str| referenced.contains(id) && skin.source_kind(id) == Some(kind);
    skin.sources.iter().filter(|(id, _)| of_kind(id)).map(|(_, path)| path.as_path()).collect()
}

/// The image files behind [`referenced_sources`]: every image a host has to have decoded before
/// the skin's screen is built, each once however many source ids resolve to it. A source that is a
/// movie is not among them ([`referenced_movie_files`]).
pub fn referenced_source_files(skin: &LoadedSkin) -> BTreeSet<&Path> {
    referenced_files(skin, SourceKind::Image)
}

/// The movie files behind [`referenced_sources`]: every movie a host has to be able to say the
/// frame size of when the skin's screen is built ([`SkinAssets::movie`]).
pub fn referenced_movie_files(skin: &LoadedSkin) -> BTreeSet<&Path> {
    referenced_files(skin, SourceKind::Movie)
}

/// One movie a screen draws from, as the host that plays it is told about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoviePlayback {
    /// Which of the screen's movies this is: what [`SkinScreen::show_movie_frame`](super::SkinScreen::show_movie_frame)
    /// is told a frame belongs to.
    pub index: usize,
    pub path: PathBuf,
    /// The size of the movie's frames in pixels, as the host said it when the screen was built.
    pub size: (u32, u32),
    /// What the frame clock read when the movie was started: the first time an object drawn from it
    /// was prepared (`SkinSourceMovie.getImage`, which starts playing on its first call). `None`
    /// until then, which is a movie that is not playing yet.
    pub started_us: Option<i64>,
    /// How many times the movie has been started. It goes up again when the frame clock is found
    /// behind where the movie was started -- the scene began again under the same screen -- and
    /// the movie is then to be played from its first frame.
    pub starts: u64,
}

/// What a movie source is doing.
#[derive(Debug, Default)]
struct MovieState {
    started_us: Option<i64>,
    starts: u64,
    /// The texture the movie's frames are uploaded into, once there has been one.
    texture: Option<TextureId>,
    /// Whether that texture holds a frame that is to be drawn.
    shown: bool,
}

/// One movie a screen draws from: where its frames go, and whether there is one on show.
///
/// Shared between the screen, which is handed the frames, and the objects drawn from the movie,
/// which read it through a shared borrow while a frame is prepared and drawn.
#[derive(Debug)]
pub(crate) struct MovieSlot {
    path: PathBuf,
    size: (u32, u32),
    /// The registry key every frame is registered under, each replacing the last.
    key: String,
    state: Mutex<MovieState>,
}

impl MovieSlot {
    /// Notes that an object drawn from the movie was prepared with the frame clock at `now_us`,
    /// which starts the movie the first time, and starts it over when the clock has gone back to
    /// before it was started.
    pub(crate) fn prepare(&self, now_us: i64) {
        let mut state = locked(&self.state);
        let starts_over = state.started_us.is_some_and(|started_us| now_us < started_us);
        if state.started_us.is_none() || starts_over {
            state.started_us = Some(now_us);
            state.starts += 1;
        }
        if starts_over {
            state.shown = false;
        }
    }

    /// The frame on show and its size, or `None` when there is nothing to draw: the movie has not
    /// had a frame decoded yet, or was given up on.
    pub(crate) fn frame(&self) -> Option<SizedTexture> {
        let state = locked(&self.state);
        state.texture.filter(|_| state.shown).map(|texture| (texture, self.size))
    }

    /// Puts `rgba` on show as the movie's frame, answering whether it was one: a buffer that is not
    /// a whole frame of the movie's size is refused.
    fn show<R: Renderer>(&self, r: &mut R, rgba: &[u8]) -> bool {
        if rgba_bytes(self.size) != rgba.len() as u64 {
            return false;
        }
        let texture = r.register_texture(&self.key, rgba, self.size.0, self.size.1);
        let mut state = locked(&self.state);
        state.texture = Some(texture);
        state.shown = true;
        true
    }

    /// Takes the frame off show, for a movie that stopped decoding.
    fn hide(&self) {
        locked(&self.state).shown = false;
    }

    /// Hands the movie's texture back, leaving nothing on show.
    fn release<R: Renderer>(&self, r: &mut R) {
        let mut state = locked(&self.state);
        state.shown = false;
        if let Some(texture) = state.texture.take() {
            r.release_texture(texture);
        }
    }

    fn playback(&self, index: usize) -> MoviePlayback {
        let state = locked(&self.state);
        MoviePlayback { index, path: self.path.clone(), size: self.size, started_us: state.started_us, starts: state.starts }
    }
}

/// Every movie source that was admitted, each as `(document source id, the movie it resolved to)`.
pub(crate) type MovieSources<'a> = &'a [(String, Arc<MovieSlot>)];

/// How many RGBA bytes an image of `size` pixels takes once it is uploaded.
pub fn rgba_bytes(size: (u32, u32)) -> u64 {
    u64::from(size.0) * u64::from(size.1) * BYTES_PER_PIXEL as u64
}

/// How many textures are held, and the RGBA bytes they come to between them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TextureStats {
    pub count: usize,
    pub bytes: u64,
}

impl TextureStats {
    /// The total in mebibytes, which is how a debug read-out says it.
    pub fn mebibytes(&self) -> f32 {
        self.bytes as f32 / BYTES_PER_MIB as f32
    }

    /// Counts one more texture of `size` pixels.
    fn add(&mut self, size: (u32, u32)) {
        self.count += 1;
        self.bytes += rgba_bytes(size);
    }
}

/// What one screen's textures are held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureLimits {
    /// The longest edge, in pixels, of an image that is uploaded at all.
    pub max_edge: u32,
    /// The most RGBA bytes the screen's textures may come to between them.
    pub budget_bytes: u64,
}

impl TextureLimits {
    /// The limits of a screen drawn on `r`: the renderer's own ceiling on a texture's edge, and
    /// [`SKIN_TEXTURE_BUDGET_BYTES`] for the lot.
    pub fn of<R: Renderer>(r: &R) -> TextureLimits {
        TextureLimits { max_edge: r.max_texture_size(), budget_bytes: SKIN_TEXTURE_BUDGET_BYTES }
    }
}

/// One file's texture in a [`SkinTexturePool`].
#[derive(Debug)]
struct PooledTexture {
    /// The registry key the texture was registered under, which is what replaces its pixels in
    /// place when the file is decoded again.
    key: String,
    tex: TextureId,
    size: (u32, u32),
    /// How many screens, and documents on their way to being one, draw from it.
    holders: usize,
}

/// The textures of every skin screen a host has compiled, one per image file.
///
/// A texture is taken with [`SkinTexturePool::hold`] (or by a screen built with
/// [`SkinScreen::build_shared`](super::SkinScreen::build_shared)) and given back with
/// [`SkinTexturePool::release`]. Giving the last hold back does not release the texture: it stays
/// uploaded until [`SkinTexturePool::sweep`], which is what lets a host let go of the screen it is
/// leaving first and decide what the screen it is entering shares with it afterwards. A host that
/// never sweeps never frees anything.
#[derive(Debug)]
pub struct SkinTexturePool {
    textures: BTreeMap<PathBuf, PooledTexture>,
    /// Files whose image is longer on an edge than the renderer can hold, each said once.
    refused: BTreeSet<PathBuf>,
    budget_bytes: u64,
}

impl Default for SkinTexturePool {
    fn default() -> SkinTexturePool {
        SkinTexturePool::with_budget(SKIN_TEXTURE_BUDGET_BYTES)
    }
}

impl SkinTexturePool {
    /// An empty pool whose screens are each held to [`SKIN_TEXTURE_BUDGET_BYTES`].
    pub fn new() -> SkinTexturePool {
        SkinTexturePool::default()
    }

    /// An empty pool whose screens are each held to `budget_bytes` of RGBA.
    pub fn with_budget(budget_bytes: u64) -> SkinTexturePool {
        SkinTexturePool { textures: BTreeMap::new(), refused: BTreeSet::new(), budget_bytes }
    }

    /// What a screen built against this pool on `r` is held to.
    pub fn limits<R: Renderer>(&self, r: &R) -> TextureLimits {
        TextureLimits { max_edge: r.max_texture_size(), budget_bytes: self.budget_bytes }
    }

    /// Whether the image at `path` is uploaded, held or not.
    pub fn contains(&self, path: &Path) -> bool {
        self.textures.contains_key(path)
    }

    /// Whether the image at `path` was found too large for the renderer, which is said once and
    /// not again however many screens name the file afterwards.
    pub fn is_refused(&self, path: &Path) -> bool {
        self.refused.contains(path)
    }

    /// How many holds there are on the texture of `path`; none for a file that is not uploaded.
    pub fn holders(&self, path: &Path) -> usize {
        self.textures.get(path).map_or(0, |texture| texture.holders)
    }

    /// Takes a hold on the texture of `path`, answering whether there was one to hold.
    ///
    /// A texture nobody held -- one waiting to be swept -- is held like any other, which is how a
    /// document about to be compiled keeps the files it shares with the screen that was just let go.
    pub fn hold(&mut self, path: &Path) -> bool {
        match self.textures.get_mut(path) {
            Some(texture) => {
                texture.holders += 1;
                true
            }
            None => false,
        }
    }

    /// Gives one hold on the texture of `path` back. The texture stays uploaded until the next
    /// [`SkinTexturePool::sweep`], held or not.
    pub fn release(&mut self, path: &Path) {
        if let Some(texture) = self.textures.get_mut(path) {
            texture.holders = texture.holders.saturating_sub(1);
        }
    }

    /// Hands every texture nobody holds back to `r`, answering how many that was.
    pub fn sweep<R: Renderer>(&mut self, r: &mut R) -> usize {
        let before = self.textures.len();
        self.textures.retain(|_, texture| {
            if texture.holders == 0 {
                r.release_texture(texture.tex);
            }
            texture.holders > 0
        });
        before - self.textures.len()
    }

    /// Every texture that is uploaded, held or waiting to be swept.
    pub fn stats(&self) -> TextureStats {
        let mut stats = TextureStats::default();
        self.textures.values().for_each(|texture| stats.add(texture.size));
        stats
    }

    /// How many uploaded textures nobody holds: what the next sweep frees.
    pub fn unheld(&self) -> usize {
        self.textures.values().filter(|texture| texture.holders == 0).count()
    }

    /// The texture and size of the file at `path`, when it is uploaded.
    fn lookup(&self, path: &Path) -> Option<SizedTexture> {
        self.textures.get(path).map(|texture| (texture.tex, texture.size))
    }

    /// Notes that the image at `path` is too large for the renderer, answering whether this is the
    /// first time it is noted -- which is the one time it is worth a warning.
    fn refuse(&mut self, path: &Path) -> bool {
        self.refused.insert(path.to_path_buf())
    }

    /// Uploads `image` as the texture of `path` and takes a hold on it.
    ///
    /// A file that is already uploaded has its pixels replaced under the handle it already had, so
    /// every screen drawing from it shows the file as it now is.
    fn put<R: Renderer>(&mut self, r: &mut R, path: &Path, image: &SkinImage) -> TextureId {
        self.refused.remove(path);
        let size = (image.width, image.height);
        if let Some(texture) = self.textures.get_mut(path) {
            texture.tex = r.register_texture(&texture.key, &image.rgba, image.width, image.height);
            texture.size = size;
            texture.holders += 1;
            return texture.tex;
        }
        let key = format!("rbms.skin.file.{}", NEXT_POOLED_TEXTURE.fetch_add(1, Ordering::Relaxed));
        let tex = r.register_texture(&key, &image.rgba, image.width, image.height);
        self.textures.insert(path.to_path_buf(), PooledTexture { key, tex, size, holders: 1 });
        tex
    }
}

/// The image sources one screen draws from.
#[derive(Debug, Default)]
pub(crate) struct SkinTextures {
    sources: Vec<(String, TextureId, (u32, u32))>,
    /// The textures registered under this screen's own keys, which it releases itself.
    owned: Vec<TextureId>,
    /// The files whose pooled texture this screen holds, which it gives back to the pool.
    pooled: Vec<PathBuf>,
    /// The textures held, each file counted once.
    stats: TextureStats,
    /// The registry namespace of the textures this screen registers for itself.
    serial: u32,
    /// The movies this screen draws from, each file once.
    movies: Vec<Arc<MovieSlot>>,
    /// Which movie each movie source id resolved to.
    movie_sources: Vec<(String, Arc<MovieSlot>)>,
}

/// Where one file's pixels are when a screen asks for them.
enum Pixels {
    /// The host decoded the file for this build.
    Decoded(SkinImage),
    /// The pool already holds the file's texture, of this size.
    Uploaded(TextureId, (u32, u32)),
}

impl Pixels {
    /// The image's size in pixels.
    fn size(&self) -> (u32, u32) {
        match self {
            Pixels::Decoded(image) => (image.width, image.height),
            Pixels::Uploaded(_, size) => *size,
        }
    }
}

/// One screen's sources on their way to being textures.
struct Registration<'a, R: Renderer> {
    r: &'a mut R,
    assets: &'a mut dyn SkinAssets,
    pool: Option<&'a mut SkinTexturePool>,
    limits: TextureLimits,
    /// The registry namespace of the textures this screen registers for itself.
    serial: u32,
    warnings: &'a mut Vec<String>,
    textures: SkinTextures,
}

impl<R: Renderer> Registration<'_, R> {
    /// The pixels of the file one source resolved to, or `None` with a warning when there are none.
    ///
    /// The host is asked for the image first, so a file it decoded again replaces whatever the pool
    /// held of it. A file the host has nothing for is drawn from the pool's texture when there is
    /// one, which is the whole point of the pool: the host need not decode it.
    fn pixels(&mut self, id: &str, path: &Path) -> Option<Pixels> {
        if let Some(image) = self.assets.image(path) {
            return Some(Pixels::Decoded(image));
        }
        let pool = self.pool.as_deref();
        if let Some((tex, size)) = pool.and_then(|pool| pool.lookup(path)) {
            return Some(Pixels::Uploaded(tex, size));
        }
        if !pool.is_some_and(|pool| pool.is_refused(path)) {
            self.warnings.push(format!("image source {id:?} could not be decoded from {}", path.display()));
        }
        None
    }

    /// Whether an image of `size` may be uploaded for this screen, leaving a warning when it may
    /// not: it is longer on an edge than the renderer can hold, or it does not fit what is left of
    /// the screen's budget.
    ///
    /// An image too large for the renderer is said once per pool, however many screens name it.
    fn fits(&mut self, id: &str, path: &Path, size: (u32, u32)) -> bool {
        if size.0.max(size.1) > self.limits.max_edge {
            if self.pool.as_deref_mut().is_none_or(|pool| pool.refuse(path)) {
                self.warnings.push(format!(
                    "image source {id:?} is {}x{}, past the {} pixels a texture can measure here, so it is left out ({})",
                    size.0,
                    size.1,
                    self.limits.max_edge,
                    path.display()
                ));
            }
            return false;
        }
        if self.textures.stats.bytes + rgba_bytes(size) > self.limits.budget_bytes {
            self.warnings.push(format!(
                "image source {id:?} would take this skin's textures past {} MiB, so it is left out ({})",
                self.limits.budget_bytes / BYTES_PER_MIB,
                path.display()
            ));
            return false;
        }
        true
    }

    /// Takes the movie one source resolved to, when the host can play it and one frame of it fits.
    ///
    /// The host is asked how large the movie's frames are, which is all a screen needs of a movie
    /// to be built: a movie the host cannot open is left out with a line in the warnings, and the
    /// image drawn from it is dropped as it is built. One frame of the movie is what it is counted
    /// at against the screen's budget, because one frame is what it ever has uploaded. Two source
    /// ids that resolve to one file share the movie.
    fn admit_movie(&mut self, id: &str, path: &Path) {
        let known = self.textures.movies.iter().find(|movie| movie.path == path).cloned();
        let movie = known.or_else(|| {
            let size = match self.assets.movie(path) {
                Ok(size) if size.0 > 0 && size.1 > 0 => size,
                Ok(size) => {
                    self.warnings.push(format!("movie source {id:?} has frames of {}x{}, so it is left out ({})", size.0, size.1, path.display()));
                    return None;
                }
                Err(reason) => {
                    self.warnings.push(format!("movie source {id:?} could not be opened from {}: {reason}", path.display()));
                    return None;
                }
            };
            if !self.fits(id, path, size) {
                return None;
            }
            self.textures.stats.add(size);
            let key = format!("rbms.skin.{}.movie.{}", self.serial, self.textures.movies.len());
            let movie = Arc::new(MovieSlot { path: path.to_path_buf(), size, key, state: Mutex::default() });
            self.textures.movies.push(Arc::clone(&movie));
            Some(movie)
        });
        if let Some(movie) = movie {
            self.textures.movie_sources.push((id.to_owned(), movie));
        }
    }

    /// The texture and size of the file one source resolved to, or `None` when the screen goes
    /// without it.
    fn admit(&mut self, id: &str, path: &Path) -> Option<SizedTexture> {
        let pixels = self.pixels(id, path)?;
        let size = pixels.size();
        if !self.fits(id, path, size) {
            return None;
        }
        let tex = match (pixels, self.pool.as_deref_mut()) {
            (Pixels::Decoded(image), Some(pool)) => {
                self.textures.pooled.push(path.to_path_buf());
                pool.put(self.r, path, &image)
            }
            (Pixels::Decoded(image), None) => {
                let key = format!("rbms.skin.{}.source.{id}", self.serial);
                let tex = self.r.register_texture(&key, &image.rgba, image.width, image.height);
                self.textures.owned.push(tex);
                tex
            }
            (Pixels::Uploaded(tex, _), pool) => {
                if pool.is_some_and(|pool| pool.hold(path)) {
                    self.textures.pooled.push(path.to_path_buf());
                }
                tex
            }
        };
        self.textures.stats.add(size);
        Some((tex, size))
    }
}

impl SkinTextures {
    /// Registers the image sources `skin` draws from ([`referenced_sources`]), and none of the ones
    /// it only declares.
    ///
    /// With a `pool` the textures are the pool's, shared by file with every other screen built
    /// against it. Without one they are registered under keys in the namespace `serial` claims and
    /// belong to this screen alone. Either way a file two source ids resolve to is asked for, and
    /// counted, once.
    ///
    /// The sources are taken in the order of their ids, which is what decides who goes without when
    /// the budget runs out: a source that does not fit what is left is skipped, and a smaller one
    /// after it may still fit. A source that is left out for any reason leaves a line in `warnings`
    /// and nothing else behind, so the objects that would have drawn from it are dropped one by one
    /// as they are built.
    pub(crate) fn register<R: Renderer>(
        r: &mut R,
        skin: &LoadedSkin,
        assets: &mut dyn SkinAssets,
        serial: u32,
        pool: Option<&mut SkinTexturePool>,
        warnings: &mut Vec<String>,
    ) -> SkinTextures {
        let limits = pool.as_deref().map_or_else(|| TextureLimits::of(r), |pool| pool.limits(r));
        let referenced = referenced_sources(skin);
        let mut registration = Registration { r, assets, pool, limits, serial, warnings, textures: SkinTextures { serial, ..SkinTextures::default() } };
        let mut files: BTreeMap<&Path, Option<SizedTexture>> = BTreeMap::new();
        for (id, path) in skin.sources.iter().filter(|(id, _)| referenced.contains(*id)) {
            if skin.source_kind(id) == Some(SourceKind::Movie) {
                registration.admit_movie(id, path);
                continue;
            }
            let admitted = match files.get(path.as_path()) {
                Some(admitted) => *admitted,
                None => {
                    let admitted = registration.admit(id, path);
                    files.insert(path.as_path(), admitted);
                    admitted
                }
            };
            if let Some((tex, size)) = admitted {
                registration.textures.sources.push((id.clone(), tex, size));
            }
        }
        registration.textures
    }

    /// Takes one more file for a screen that is already built, under the same limits and the same
    /// ownership as the sources it was built with: a page of a bitmap font, which is asked for only
    /// once a glyph on it is to be drawn.
    ///
    /// `assets` is asked for the file's pixels first and `pool` for a texture it already holds of
    /// it second, as for a source. `label` is what the file is called in a warning, and what tells
    /// its registry key from the sources' when the screen has no pool; it has to be the file's own.
    /// `None`, with a line in `warnings`, when the screen goes without the file.
    pub(crate) fn admit_late<R: Renderer>(
        &mut self,
        r: &mut R,
        label: &str,
        path: &Path,
        assets: &mut dyn SkinAssets,
        pool: Option<&mut SkinTexturePool>,
        warnings: &mut Vec<String>,
    ) -> Option<SizedTexture> {
        let limits = pool.as_deref().map_or_else(|| TextureLimits::of(r), |pool| pool.limits(r));
        let serial = self.serial;
        let mut registration = Registration { r, assets, pool, limits, serial, warnings, textures: std::mem::take(self) };
        let admitted = registration.admit(label, path);
        *self = registration.textures;
        admitted
    }

    /// The registered sources, as the object builders look them up.
    pub(crate) fn sources(&self) -> Source<'_> {
        &self.sources
    }

    /// How many textures are held, and the RGBA bytes they come to. A movie is one texture of one
    /// frame, from the moment the screen is built.
    pub(crate) fn stats(&self) -> TextureStats {
        self.stats
    }

    /// The movie sources that were admitted, as the object builders look them up.
    pub(crate) fn movie_sources(&self) -> MovieSources<'_> {
        &self.movie_sources
    }

    /// The movies this screen draws from and what each is doing.
    pub(crate) fn movies(&self) -> Vec<MoviePlayback> {
        self.movies.iter().enumerate().map(|(index, movie)| movie.playback(index)).collect()
    }

    /// Puts `rgba` on show as the frame of movie `index`, answering whether it was a whole frame
    /// of a movie this screen has.
    pub(crate) fn show_movie_frame<R: Renderer>(&self, r: &mut R, index: usize, rgba: &[u8]) -> bool {
        self.movies.get(index).is_some_and(|movie| movie.show(r, rgba))
    }

    /// Takes the frame of movie `index` off show.
    pub(crate) fn hide_movie(&self, index: usize) {
        if let Some(movie) = self.movies.get(index) {
            movie.hide();
        }
    }

    /// Lets go of every texture, leaving nothing held: the screen's own go back to `r`, and the
    /// pooled ones have their hold given back to `pool`, where they wait for its next sweep.
    ///
    /// A screen built against a pool has to be released with that pool. Without it the pooled
    /// textures keep this screen's hold and are never swept.
    pub(crate) fn release<R: Renderer>(&mut self, r: &mut R, pool: Option<&mut SkinTexturePool>) {
        self.sources.clear();
        self.stats = TextureStats::default();
        for tex in self.owned.drain(..) {
            r.release_texture(tex);
        }
        self.movie_sources.clear();
        for movie in self.movies.drain(..) {
            movie.release(r);
        }
        if let Some(pool) = pool {
            for path in self.pooled.drain(..) {
                pool.release(&path);
            }
        }
    }
}

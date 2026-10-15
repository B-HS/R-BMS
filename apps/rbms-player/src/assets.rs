//! Everything the player reads off disk or out of the binary before a screen can use it: the
//! built-in note-field layouts, the default skin pack shipped inside the binary ([`default_pack`]),
//! the UI theme file ([`theme_file`]), chart-relative file resolution, the keysound decode pool and
//! the BGA image decode.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use rbms_render::skin_render::frame::{BgaLook, BgaPicture, BgaPlayhead, BgaPrepared, RESIDENT_BYTES, texture_bytes_of, uploaded_ahead};
use rbms_render::{SkinConfig, SkinImage};
use rbms_video::{VideoDecoder, VideoError};
use sha2::{Digest, Sha256};

pub(crate) mod default_pack;
#[cfg(test)]
pub(crate) mod scripted_movie;
pub(crate) mod theme_file;

pub(crate) use default_pack::{default_sound_folder, install_default_pack, installed_default_pack};
pub(crate) use theme_file::load_theme;

/// A decode running on the worker pool: what has arrived, how far it has got, the cooperative
/// cancel, and how many jobs there are in total.
pub(crate) type DecodePool<I, T> = (Receiver<(I, T)>, Arc<AtomicUsize>, usize);

/// Most workers a decode is fanned across. Past this the disk, not the CPU, is the limit.
const DECODE_THREADS_MAX: usize = 8;

/// Workers used when the machine will not say how many cores it has.
const DECODE_THREADS_FALLBACK: usize = 4;

/// Bytes in a mebibyte, the unit [`PREPARED_BGA_BYTES`] is given in.
const MEBIBYTE: usize = 1024 * 1024;

/// Most bytes of pixels the decodes of one chart's background pictures paint ahead between them
/// ([`BgaPrepared`]), on top of the decoded pictures themselves. Past it a picture is painted the
/// frame it is shown in, as every picture once was.
pub(crate) const PREPARED_BGA_BYTES: usize = 256 * MEBIBYTE;

pub(crate) const SKIN_NORMAL: &str = include_str!("../../../assets/skins/normal.ron");
pub(crate) const SKIN_WIDE: &str = include_str!("../../../assets/skins/wide.ron");

/// The folder beside the settings file that holds everything skins write, one folder per skin pack.
pub(crate) const SKIN_DATA_FOLDER: &str = "skin-data";

/// What a pack whose folder has no usable name is called in its identifier.
const SKIN_PACK_FALLBACK_NAME: &str = "skin";

/// Most characters of a pack folder's own name that go into its identifier.
const SKIN_PACK_NAME_MAX_CHARS: usize = 32;

/// What a character of a pack folder's name that is unsafe in a file name is replaced with.
const SKIN_PACK_NAME_FILLER: char = '_';

/// Bytes of the pack path's digest that go into its identifier, which is what tells two packs with
/// the same folder name apart.
const SKIN_PACK_DIGEST_BYTES: usize = 8;

/// One of the built-in note-field layouts by name ("WIDE" or NORMAL). Used when no external
/// `--skin` is given.
pub(crate) fn bundled_skin(name: &str) -> SkinConfig {
    let s = if name.eq_ignore_ascii_case("WIDE") { SKIN_WIDE } else { SKIN_NORMAL };
    ron::from_str(s).unwrap_or_default()
}

/// The name one skin pack's writes are filed under: the pack folder's own name, made safe for a file
/// name, and a digest of the folder's whole path.
///
/// The path is resolved first when it can be, so the same folder reached by two spellings is one
/// pack, and the digest is of bytes rather than of anything the standard library may hash
/// differently next release, so a pack keeps its identifier from run to run.
pub(crate) fn skin_pack_identifier(pack: &Path) -> String {
    let resolved = pack.canonicalize().unwrap_or_else(|_| pack.to_path_buf());
    let name: String = resolved
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .take(SKIN_PACK_NAME_MAX_CHARS)
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { SKIN_PACK_NAME_FILLER })
        .collect();
    let name = if name.is_empty() { SKIN_PACK_FALLBACK_NAME.to_owned() } else { name };
    let digest = Sha256::digest(resolved.to_string_lossy().as_bytes());
    format!("{name}-{}", hex(&digest[..SKIN_PACK_DIGEST_BYTES]))
}

/// `bytes` in lowercase hexadecimal, two digits a byte.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Where the skin whose entry file is `document` writes: `skin-data/<pack identifier>` beside the
/// settings file, the pack being the folder the document is in.
///
/// A skin never writes into its own folder, which may be read-only and is not the player's to have
/// changed under them; whatever it writes lands here and is read back from here. The folder is not
/// created until a skin writes something.
pub(crate) fn skin_overlay_folder(settings_path: &Path, document: &Path) -> PathBuf {
    let pack = document.parent().unwrap_or(Path::new("."));
    settings_path.parent().unwrap_or(Path::new(".")).join(SKIN_DATA_FOLDER).join(skin_pack_identifier(pack))
}

pub(crate) fn resolve_file(dir: &Path, name: &str, exts: &[&str]) -> Option<(PathBuf, String)> {
    let name = name.replace('\\', "/");
    let direct = dir.join(&name);
    if direct.is_file() {
        let ext = direct.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        return Some((direct, ext));
    }
    let stem = Path::new(&name).with_extension("");
    for ext in exts {
        let p = dir.join(format!("{}.{ext}", stem.display()));
        if p.is_file() {
            return Some((p, ext.to_string()));
        }
    }
    None
}

pub(crate) fn resolve_keysound(dir: &Path, name: &str) -> Option<(PathBuf, String)> {
    resolve_file(dir, name, &["ogg", "wav", "flac", "mp3"])
}

/// Resolve every referenced keysound in a chart's `wavmap` to `(id, path, ext)` decode jobs (empty
/// names skipped, unresolvable files dropped). Shared by the Play loader and the autoplay preview.
pub(crate) fn keysound_jobs(wavmap: &[String], dir: &Path) -> Vec<(u32, PathBuf, String)> {
    wavmap
        .iter()
        .enumerate()
        .filter(|(_, name)| !name.is_empty())
        .filter_map(|(id, name)| resolve_keysound(dir, name).map(|(p, x)| (id as u32, p, x)))
        .collect()
}

/// Fan a decode out over a thread pool, streaming `(id, decoded)` back over a channel with a
/// progress counter.
///
/// Workers check `cancel` between jobs, so an abandoned load stops promptly instead of decoding to
/// the end. The counter counts jobs *attempted*, not results sent: a file that would not decode is
/// still one the screen no longer has to wait for, and a bar that stalled on a broken file would
/// never reach its end.
fn spawn_decode<I, J, T>(jobs: Vec<(I, J)>, cancel: Arc<AtomicBool>, decode: fn(&J) -> Option<T>) -> DecodePool<I, T>
where
    I: Send + 'static,
    J: Send + 'static,
    T: Send + 'static,
{
    let total = jobs.len();
    let (tx, rx) = std::sync::mpsc::channel();
    let progress = Arc::new(AtomicUsize::new(0));
    fan_out(jobs, cancel, decode, tx, progress.clone());
    (rx, progress, total)
}

/// How many workers a decode is fanned across on this machine.
fn decode_threads() -> usize {
    std::thread::available_parallelism().map(|c| c.get().min(DECODE_THREADS_MAX)).unwrap_or(DECODE_THREADS_FALLBACK).max(1)
}

/// [`spawn_decode`]'s workers, sending to `tx` and counting into `progress`.
fn fan_out<I, J, T>(jobs: Vec<(I, J)>, cancel: Arc<AtomicBool>, decode: fn(&J) -> Option<T>, tx: Sender<(I, T)>, progress: Arc<AtomicUsize>)
where
    I: Send + 'static,
    J: Send + 'static,
    T: Send + 'static,
{
    let total = jobs.len();
    if total == 0 {
        return;
    }
    let chunk = total.div_ceil(decode_threads()).max(1);
    let mut chunks: Vec<Vec<(I, J)>> = Vec::new();
    for job in jobs {
        match chunks.last_mut() {
            Some(last) if last.len() < chunk => last.push(job),
            _ => chunks.push(vec![job]),
        }
    }
    for jc in chunks {
        let tx = tx.clone();
        let progress = progress.clone();
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            for (id, job) in jc {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Some(decoded) = decode(&job) {
                    let _ = tx.send((id, decoded));
                }
                progress.fetch_add(1, Ordering::Relaxed);
            }
        });
    }
}

/// Decode one keysound file.
fn decode_keysound(job: &(PathBuf, String)) -> Option<rbms_audio::DecodedAudio> {
    let (path, ext) = job;
    let data = std::fs::read(path).ok()?;
    rbms_audio::decode_bytes(data, Some(ext.as_str())).ok()
}

/// Fan keysound decode out over the worker pool. Shared by the Play loader and the autoplay preview.
pub(crate) fn spawn_keysound_decode(jobs: Vec<(u32, PathBuf, String)>, cancel: Arc<AtomicBool>) -> DecodePool<u32, rbms_audio::DecodedAudio> {
    spawn_decode(jobs.into_iter().map(|(id, path, ext)| (id, (path, ext))).collect(), cancel, decode_keysound)
}

/// Distinguishes one decode from the next.
///
/// A background is handed to the target on every frame while the picture only changes every few
/// hundred milliseconds, so the target needs a cheap way to recognise the frame it already holds
/// and skip re-uploading megabytes of it. Identity, not content: two decodes of the same file are
/// two images here, which costs one upload and never shows the wrong picture.
static NEXT_IMAGE_GENERATION: AtomicU64 = AtomicU64::new(1);

/// One decoded image, at whatever size the file itself was.
///
/// Nothing resizes a background on the way in any more: the renderer takes a texture of any size and
/// stretches it onto the rectangle the screen asks for, so a chart's own resolution survives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecodedImage {
    pub(crate) rgba: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// This decode's own number, which a draw target uses to tell one frame from the next.
    pub(crate) generation: u64,
    /// The pixels a skin's `bga` object hands the renderer for each look the chart shows the image
    /// in, painted by the decode's worker so a frame does not paint them, and which of those looks
    /// the chart's plan uploads ahead of play instead ([`plan_uploads_ahead`]). Empty for an image
    /// that is not one of a chart's background pictures, for a look past [`PREPARED_BGA_BYTES`], for
    /// a look uploaded ahead, and for a look that needs no painting.
    pub(crate) prepared: BgaPrepared,
}

impl DecodedImage {
    /// One image built in a test, taking a generation of its own like any other decode.
    #[cfg(test)]
    pub(crate) fn for_test(rgba: Vec<u8>, width: u32, height: u32) -> DecodedImage {
        DecodedImage { rgba, width, height, generation: NEXT_IMAGE_GENERATION.fetch_add(1, Ordering::Relaxed), prepared: BgaPrepared::default() }
    }
}

/// Which of a skin document's files one decode job carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SkinAssetKind {
    Image,
    Font,
    /// A movie one of the document's images is drawn from.
    Movie,
}

/// One of a skin document's files, read and decoded off the frame loop.
#[derive(Debug)]
pub(crate) enum SkinAsset {
    Image(SkinImage),
    Font(Vec<u8>),
    /// A movie, opened and not yet decoded: its header has been read, which says how large its
    /// frames are, and its first frame is decoded when something starts playing it.
    Movie(Box<dyn VideoDecoder>),
    /// A movie that could not be opened, and why.
    Unplayable(String),
    /// The file is the one the caller already holds a texture of, so it was not decoded again.
    Unchanged,
}

/// What one skin decode job names.
pub(crate) type SkinAssetJob = (SkinAssetKind, PathBuf);

/// What tells one version of a file from the next without reading it: when it was last written and
/// how long it is.
pub(crate) type FileStamp = (SystemTime, u64);

/// One skin decode job as a worker is handed it: the file, and the stamp of the version of it the
/// caller already has uploaded, when it has one.
pub(crate) type SkinAssetRequest = (SkinAssetJob, Option<FileStamp>);

/// What a worker read of one of a document's files.
#[derive(Debug)]
pub(crate) struct SkinAssetRead {
    pub(crate) asset: SkinAsset,
    /// The file's stamp when the worker looked, taken before the file was read: a file written in
    /// between is then decoded once more than it had to be rather than once too few. `None` for a
    /// font, and for a file that cannot be stat'ed, which is never taken for unchanged.
    pub(crate) stamp: Option<FileStamp>,
}

/// The stamp of the file at `path` as it is now.
fn file_stamp(path: &Path) -> Option<FileStamp> {
    let file = std::fs::metadata(path).ok()?;
    Some((file.modified().ok()?, file.len()))
}

/// Decode one of a document's files.
///
/// An image the caller already has uploaded costs one stat when the file has not been written
/// since. That keeps the texture a screen shares with the one before it from being decoded twice,
/// and still shows a sheet its author just saved over the next time the skin is read.
fn decode_skin_asset(request: &SkinAssetRequest) -> Option<SkinAssetRead> {
    let ((kind, path), uploaded) = request;
    match kind {
        SkinAssetKind::Image => {
            let stamp = file_stamp(path);
            if stamp.is_some() && stamp == *uploaded {
                return Some(SkinAssetRead { asset: SkinAsset::Unchanged, stamp });
            }
            let decoded = image::open(path).ok()?.to_rgba8();
            let (width, height) = decoded.dimensions();
            SkinImage::new(width, height, decoded.into_raw()).map(|image| SkinAssetRead { asset: SkinAsset::Image(image), stamp })
        }
        SkinAssetKind::Font => std::fs::read(path).ok().map(|bytes| SkinAssetRead { asset: SkinAsset::Font(bytes), stamp: None }),
        SkinAssetKind::Movie => {
            let asset = match open_skin_movie(path) {
                Ok(decoder) => SkinAsset::Movie(decoder),
                Err(error) => SkinAsset::Unplayable(error.to_string()),
            };
            Some(SkinAssetRead { asset, stamp: None })
        }
    }
}

/// Opens the movie one of a skin's sources resolved to.
///
/// Opening reads the file's header and nothing else, but that is still a seek to wherever the
/// header is and a table for every frame, so it is done on the worker that would have decoded the
/// file had it been an image. A build with no video support opens nothing, and says so.
#[cfg(not(test))]
fn open_skin_movie(path: &Path) -> Result<Box<dyn VideoDecoder>, VideoError> {
    rbms_video::open(path)
}

/// Opens the movie one of a skin's sources resolved to, or the movie a test wrote out as a script
/// in its place ([`scripted_movie`]).
#[cfg(test)]
fn open_skin_movie(path: &Path) -> Result<Box<dyn VideoDecoder>, VideoError> {
    match scripted_movie::open(path) {
        Some(scripted) => scripted,
        None => rbms_video::open(path),
    }
}

/// Fan a skin document's images and fonts out over the worker pool.
///
/// A published skin draws from dozens of source images and some of them are several thousand pixels
/// square, which is seconds of decoding the frame loop cannot spend. Until the pool is done the
/// screen it belongs to draws its built-in layout.
pub(crate) fn spawn_skin_asset_decode(requests: Vec<SkinAssetRequest>, cancel: Arc<AtomicBool>) -> DecodePool<SkinAssetJob, SkinAssetRead> {
    spawn_decode(requests.into_iter().map(|request| (request.0.clone(), request)).collect(), cancel, decode_skin_asset)
}

/// One of a chart's background pictures as its decode is handed it: the file, the looks a skin's
/// `bga` object shows it in, which the worker paints ahead unless they are uploaded ahead of play,
/// and what is left of [`PREPARED_BGA_BYTES`] for the chart's decodes between them.
#[derive(Debug, Clone)]
pub(crate) struct BgaJob {
    pub(crate) path: PathBuf,
    pub(crate) looks: Vec<BgaLook>,
    pub(crate) room: Arc<AtomicUsize>,
    /// The looks the chart's plan uploads ahead of play ([`plan_uploads_ahead`]), which the worker
    /// leaves unpainted. `None` until a plan is made, and for a chart no plan is made for.
    pub(crate) ahead: Option<Vec<BgaLook>>,
}

/// Resolve every referenced background image in a chart's `bgamap` to decode jobs (empty names
/// skipped, unresolvable files dropped), each with the looks `shown` says a skin's `bga` object
/// shows it in. `shown` is `None` for a screen with no such object, which paints nothing ahead.
pub(crate) fn bga_jobs(bgamap: &[String], dir: &Path, shown: Option<&BgaPlayhead>) -> Vec<(i32, BgaJob)> {
    let mut looks: HashMap<i32, Vec<BgaLook>> = HashMap::new();
    for (number, look) in shown.into_iter().flat_map(BgaPlayhead::looks) {
        let of_number = looks.entry(number).or_default();
        if !of_number.contains(&look) {
            of_number.push(look);
        }
    }
    let room = Arc::new(AtomicUsize::new(PREPARED_BGA_BYTES));
    bgamap
        .iter()
        .enumerate()
        .filter(|(_, name)| !name.trim().is_empty())
        .filter_map(|(id, name)| {
            let path = resolve_bga_file(dir, name)?;
            let id = id as i32;
            Some((id, BgaJob { path, looks: looks.remove(&id).unwrap_or_default(), room: room.clone(), ahead: None }))
        })
        .collect()
}

/// Fan background-image decode out over the worker pool. A chart can reference hundreds of images
/// and each is decoded and painted for the looks it is shown in on the way in, which is seconds of
/// work the frame loop cannot spend.
///
/// `first_looks` is every picture and look a skin's `bga` object shows, in the order the chart
/// first shows them ([`BgaPlayhead::first_looks`]), or `None` for a screen with no such object.
/// With it, the decode first plans which looks are uploaded ahead of play and which are painted
/// ([`plan_uploads_ahead`]), off the frame thread like the decode itself.
pub(crate) fn spawn_bga_decode(jobs: Vec<(i32, BgaJob)>, first_looks: Option<Vec<(i32, BgaLook)>>, cancel: Arc<AtomicBool>) -> DecodePool<i32, DecodedImage> {
    let total = jobs.len();
    let (tx, rx) = std::sync::mpsc::channel();
    let progress = Arc::new(AtomicUsize::new(0));
    if total == 0 {
        return (rx, progress, 0);
    }
    let workers = progress.clone();
    std::thread::spawn(move || {
        let jobs = match first_looks {
            Some(first_looks) => plan_uploads_ahead(jobs, &first_looks, RESIDENT_BYTES, &cancel),
            None => jobs,
        };
        fan_out(jobs, cancel, decode_bga_job, tx, workers);
    });
    (rx, progress, total)
}

/// Decides, before any picture is decoded, which of the looks in `first_looks` are uploaded ahead
/// of play and so not painted ahead: the same looks the upload ahead holds under `allowance`
/// ([`uploaded_ahead`]), costed from each picture's size as its file gives it, read across the
/// workers. Every job is given the plan, as the looks of its own picture it names.
///
/// Knowing the plan before the decodes start is what keeps the two allowances apart. A worker that
/// painted whatever it reached first would paint looks the upload ahead then holds as textures too,
/// spending the room painting has on pixels that are held twice, and leaving the looks that are
/// not uploaded ahead, which are the ones it is for, unpainted.
fn plan_uploads_ahead(mut jobs: Vec<(i32, BgaJob)>, first_looks: &[(i32, BgaLook)], allowance: usize, cancel: &AtomicBool) -> Vec<(i32, BgaJob)> {
    let index: HashMap<i32, usize> = jobs.iter().enumerate().map(|(at, (number, _))| (*number, at)).collect();
    let shown: Vec<usize> = jobs.iter().enumerate().filter(|(_, (_, job))| !job.looks.is_empty()).map(|(at, _)| at).collect();
    let mut sizes: HashMap<usize, (u32, u32)> = HashMap::new();
    std::thread::scope(|scope| {
        let workers: Vec<_> = shown
            .chunks(shown.len().div_ceil(decode_threads()).max(1))
            .map(|part| {
                let jobs = &jobs;
                scope.spawn(move || {
                    part.iter()
                        .take_while(|_| !cancel.load(Ordering::Relaxed))
                        .filter_map(|at| bga_file_size(&jobs[*at].1.path).map(|size| (*at, size)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        sizes.extend(workers.into_iter().filter_map(|worker| worker.join().ok()).flatten());
    });
    let costs = first_looks.iter().map(|(number, _)| index.get(number).and_then(|at| sizes.get(at)).map(|(width, height)| texture_bytes_of(*width, *height)));
    let mut ahead: Vec<Vec<BgaLook>> = vec![Vec::new(); jobs.len()];
    for ((number, look), held) in first_looks.iter().zip(uploaded_ahead(costs, allowance)) {
        if let Some(at) = index.get(number).filter(|_| held) {
            ahead[*at].push(*look);
        }
    }
    for ((_, job), looks) in jobs.iter_mut().zip(ahead) {
        job.ahead = Some(looks);
    }
    jobs
}

/// The size of the picture in a background image file, read from its header alone: `None` for a
/// file that would not decode by its contents ([`decode_bga_file`]).
fn bga_file_size(path: &Path) -> Option<(u32, u32)> {
    let file = std::io::BufReader::new(std::fs::File::open(path).ok()?);
    image::ImageReader::new(file).with_guessed_format().ok()?.into_dimensions().ok()
}

/// Decode one of a chart's background pictures and paint it in each look it is shown in that is not
/// uploaded ahead of play, for as long as the chart's decodes have room for the pixels.
fn decode_bga_job(job: &BgaJob) -> Option<DecodedImage> {
    let mut image = decode_bga_file(&job.path)?;
    let mut prepared = BgaPrepared { ahead: job.ahead.clone(), ..BgaPrepared::default() };
    let picture = bga_picture(&image);
    let cost = picture.texture_bytes();
    for look in job.looks.iter().filter(|look| !job.ahead.as_ref().is_some_and(|ahead| ahead.contains(look))) {
        if job.room.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| left.checked_sub(cost)).is_err() {
            break;
        }
        let painted = BgaPrepared::paint(&picture, *look);
        if painted.is_none() {
            job.room.fetch_add(cost, Ordering::Relaxed);
        }
        match look {
            BgaLook::Plain => prepared.plain = painted,
            BgaLook::Keyed => prepared.keyed = painted,
        }
    }
    image.prepared = prepared;
    Some(image)
}

/// Where a chart's named background image is on disk.
fn resolve_bga_file(dir: &Path, name: &str) -> Option<PathBuf> {
    resolve_file(dir, name, &["png", "bmp", "jpg", "jpeg"]).map(|(path, _)| path)
}

/// Decode one background image at its own resolution.
fn decode_bga_file(path: &PathBuf) -> Option<DecodedImage> {
    let bytes = std::fs::read(path).ok()?;
    let rgba = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(DecodedImage {
        rgba: rgba.into_raw(),
        width,
        height,
        generation: NEXT_IMAGE_GENERATION.fetch_add(1, Ordering::Relaxed),
        prepared: BgaPrepared::default(),
    })
}

/// Decode one named background image, for the single cover the browser shows.
pub(crate) fn decode_bga_image(dir: &Path, name: &str) -> Option<DecodedImage> {
    decode_bga_file(&resolve_bga_file(dir, name)?)
}

/// Whether a play screen decodes a chart's background images at all.
///
/// A screen drawn from a skin document needs them when the player has backgrounds on and the
/// document places a `bga` object, which `document_has_bga` says (`None` while no document is
/// known). A screen with no document keeps the rule it always had: backgrounds on and a slot in the
/// built-in layout (`built_in_slot`) to put one in.
pub(crate) fn wants_bga_pictures(display_bga: bool, document_has_bga: Option<bool>, built_in_slot: bool) -> bool {
    display_bga && document_has_bga.unwrap_or(built_in_slot)
}

/// A decoded background image as the skin renderer's `bga` object takes it.
pub(crate) fn bga_picture(image: &DecodedImage) -> BgaPicture<'_> {
    BgaPicture { generation: image.generation, width: image.width, height: image.height, rgba: &image.rgba, prepared: Some(&image.prepared) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The decode job of a picture no skin shows, which paints nothing ahead.
    fn unshown(path: &Path) -> BgaJob {
        BgaJob { path: path.to_path_buf(), looks: Vec::new(), room: Arc::new(AtomicUsize::new(PREPARED_BGA_BYTES)), ahead: None }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rbms-assets-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the fixture directory");
        dir
    }

    /// A chart names its images by `#BMP` slot, and the slot number is what the session asks for
    /// while it plays — so a name that resolves to nothing has to drop out without shifting the
    /// ones after it.
    #[test]
    fn background_jobs_keep_each_images_own_slot_and_drop_the_ones_that_are_not_there() {
        let dir = temp_dir("bga-jobs");
        std::fs::write(dir.join("second.png"), b"not really a png").expect("write the fixture");
        let jobs = bga_jobs(&[String::new(), "second.png".into(), "missing.png".into(), "   ".into()], &dir, None);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].0, 1, "the surviving image keeps slot 1");
        let _ = std::fs::remove_file(dir.join("second.png"));
    }

    /// A background is handed on at the resolution the file itself was, which is what lets the
    /// renderer stretch it onto whatever rectangle the screen asks for rather than onto a square
    /// chosen by the decoder.
    #[test]
    fn a_background_keeps_the_size_the_file_was_drawn_at() {
        let dir = temp_dir("bga-size");
        let path = dir.join("wide.png");
        image::RgbaImage::from_pixel(6, 3, image::Rgba([1, 2, 3, 255])).save(&path).expect("write the fixture");

        let decoded = decode_bga_image(&dir, "wide.png").expect("the fixture decodes");
        assert_eq!((decoded.width, decoded.height), (6, 3), "the decoder resized what it was given");
        assert_eq!(decoded.rgba.len(), 6 * 3 * 4, "the buffer does not hold that many RGBA pixels");
        let _ = std::fs::remove_file(&path);
    }

    /// A skin document that places a `bga` object decides whether the chart's images are decoded, and
    /// a screen with no document decides as it always did.
    #[test]
    fn backgrounds_are_decoded_for_a_document_that_places_one_and_otherwise_for_the_built_in_slot() {
        assert!(wants_bga_pictures(true, Some(true), false), "a document with a bga object, over a layout with no slot");
        assert!(!wants_bga_pictures(true, Some(false), true), "a document with no bga object, over a layout with a slot");
        assert!(wants_bga_pictures(true, None, true), "no document, a built-in slot");
        assert!(!wants_bga_pictures(true, None, false), "no document, no slot");
        assert!(!wants_bga_pictures(false, Some(true), true), "the player turned backgrounds off");
    }

    /// A picture a skin shows is painted, on the decode's worker, in every look it is shown in -- as
    /// it is on its canvas when it is small, and keyed out as a layer -- so the frame that shows it
    /// paints nothing; a picture no skin shows is not painted at all.
    #[test]
    fn a_background_picture_is_painted_ahead_in_each_look_the_chart_shows_it_in() {
        use rbms_render::skin_render::frame::{BgaEvent, SMALL_PICTURE_EDGE};
        let dir = temp_dir("bga-prepared");
        image::RgbaImage::from_fn(4, 2, |x, _| if x < 2 { image::Rgba([0, 0, 0, 255]) } else { image::Rgba([9, 8, 7, 255]) })
            .save(dir.join("small.png"))
            .expect("write the fixture");
        let names = vec!["small.png".to_string(), "small.png".to_string(), "small.png".to_string()];
        let shown = BgaPlayhead::new(vec![BgaEvent { time_ms: 0, base: 0, layer: 1, miss: None }]);
        let jobs = bga_jobs(&names, &dir, Some(&shown));
        let decoded: Vec<DecodedImage> = jobs.iter().map(|(_, job)| decode_bga_job(job).expect("the fixture decodes")).collect();

        let canvas = (SMALL_PICTURE_EDGE * SMALL_PICTURE_EDGE * 4) as usize;
        let plain = decoded[0].prepared.plain.as_deref().expect("the chart's own picture is painted on its canvas");
        assert_eq!((plain.len(), decoded[0].prepared.keyed.as_ref()), (canvas, None), "the chart's own picture was keyed out too");
        let keyed = decoded[1].prepared.keyed.as_deref().expect("the layer is painted keyed out");
        assert_eq!((keyed.len(), decoded[1].prepared.plain.as_ref()), (canvas, None));
        let first_pixel = (SMALL_PICTURE_EDGE as usize - 4) / 2 * 4;
        assert_eq!(&plain[first_pixel..first_pixel + 4], &[0, 0, 0, 255], "the picture as it is keeps its black");
        assert_eq!(&keyed[first_pixel..first_pixel + 4], &[0, 0, 0, 0], "the layer's black was not keyed out");
        assert_eq!(decoded[2].prepared, BgaPrepared::default(), "a picture the chart never shows was painted");
        assert_eq!(bga_picture(&decoded[1]).prepared, Some(&decoded[1].prepared), "the renderer is not handed what was painted");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The decodes of a chart paint no more ahead than they have room for between them; what is
    /// past it is painted the frame it is shown in.
    #[test]
    fn painting_ahead_stops_at_the_room_the_charts_decodes_have() {
        let dir = temp_dir("bga-room");
        let path = dir.join("large.png");
        image::RgbaImage::from_pixel(300, 300, image::Rgba([1, 2, 3, 255])).save(&path).expect("write the fixture");
        let one = 300 * 300 * 4;
        let room = Arc::new(AtomicUsize::new(one + one / 2));
        let job = BgaJob { path, looks: vec![BgaLook::Keyed], room: room.clone(), ahead: None };
        assert!(decode_bga_job(&job).expect("the fixture decodes").prepared.keyed.is_some(), "the first layer had room");
        assert!(decode_bga_job(&job).expect("the fixture decodes").prepared.keyed.is_none(), "a layer past the room was painted");
        assert_eq!(room.load(Ordering::Relaxed), one / 2, "room was taken for a layer that was not painted");
        let plain = BgaJob { looks: vec![BgaLook::Plain], ..job };
        assert_eq!(decode_bga_job(&plain).expect("the fixture decodes").prepared, BgaPrepared::default(), "a large picture shown as it is was painted");
        assert_eq!(room.load(Ordering::Relaxed), one / 2, "room was kept for a picture that needed no painting");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A picture exactly as large as its 256 x 256 canvas, shown as it is, is that canvas already:
    /// nothing is painted for it and it takes none of the room, which stays for the pictures that
    /// need painting. Keyed out as a layer it is painted, and takes room for it.
    #[test]
    fn a_picture_as_large_as_its_canvas_is_not_painted_as_it_is_and_takes_no_room() {
        use rbms_render::skin_render::frame::SMALL_PICTURE_EDGE;
        let dir = temp_dir("bga-canvas-sized");
        let path = dir.join("frame.png");
        image::RgbaImage::from_pixel(SMALL_PICTURE_EDGE, SMALL_PICTURE_EDGE, image::Rgba([1, 2, 3, 255])).save(&path).expect("write the fixture");
        let one = (SMALL_PICTURE_EDGE * SMALL_PICTURE_EDGE * 4) as usize;
        let room = Arc::new(AtomicUsize::new(one * 2));
        let plain = BgaJob { path, looks: vec![BgaLook::Plain], room: room.clone(), ahead: None };
        for _ in 0..3 {
            assert!(decode_bga_job(&plain).expect("the fixture decodes").prepared == BgaPrepared::default(), "a picture as large as its canvas was painted");
        }
        assert_eq!(room.load(Ordering::Relaxed), one * 2, "room was taken for a picture that needed no painting");
        let keyed = BgaJob { looks: vec![BgaLook::Keyed], ..plain };
        assert!(decode_bga_job(&keyed).expect("the fixture decodes").prepared.keyed.is_some(), "the layer was not painted");
        assert_eq!(room.load(Ordering::Relaxed), one, "the layer took no room");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The looks a chart's background is held in as textures from before it plays are planned before
    /// any picture is decoded, from the pictures' sizes, in the order the chart first shows them; the
    /// decode leaves those unpainted and takes none of the room for them, which goes to the looks
    /// that are not uploaded ahead. A picture the chart does not show is in no plan.
    #[test]
    fn a_look_uploaded_ahead_of_play_is_not_painted_and_leaves_the_room_to_the_others() {
        use rbms_render::skin_render::frame::{BgaEvent, SMALL_PICTURE_EDGE, texture_bytes_of};
        const SMALL: u32 = 100;
        const LARGE: u32 = 300;
        let dir = temp_dir("bga-plan");
        image::RgbaImage::from_pixel(SMALL, SMALL, image::Rgba([1, 2, 3, 255])).save(dir.join("small.png")).expect("write the fixture");
        image::RgbaImage::from_pixel(LARGE, LARGE, image::Rgba([4, 5, 6, 255])).save(dir.join("large.png")).expect("write the fixture");
        let names = vec!["small.png".to_string(), "small.png".to_string(), "large.png".to_string(), "small.png".to_string()];
        let event = |time_ms: i64, base: i32, layer: i32| BgaEvent { time_ms, base, layer, miss: None };
        let shown = BgaPlayhead::new(vec![event(0, 0, 2), event(10, 1, -1), event(20, 2, -1), event(30, 0, -1)]);
        let canvas = texture_bytes_of(SMALL_PICTURE_EDGE, SMALL_PICTURE_EDGE);
        let large = texture_bytes_of(LARGE, LARGE);

        let jobs = plan_uploads_ahead(bga_jobs(&names, &dir, Some(&shown)), &shown.first_looks(), canvas + large, &AtomicBool::new(false));
        let ahead: Vec<Option<Vec<BgaLook>>> = jobs.iter().map(|(_, job)| job.ahead.clone()).collect();
        assert_eq!(
            ahead,
            vec![Some(vec![BgaLook::Plain]), Some(Vec::new()), Some(vec![BgaLook::Keyed]), Some(Vec::new())],
            "the plan is not the looks that fit in the order the chart first shows them"
        );
        let room = jobs[0].1.room.clone();
        let decoded: Vec<DecodedImage> = jobs.iter().map(|(_, job)| decode_bga_job(job).expect("the fixture decodes")).collect();
        assert_eq!(
            (decoded[0].prepared.plain.as_ref(), decoded[0].prepared.ahead.as_deref()),
            (None, Some(&[BgaLook::Plain][..])),
            "a look uploaded ahead was painted"
        );
        assert!(decoded[1].prepared.plain.is_some(), "a look past the plan was not painted");
        assert_eq!((decoded[2].prepared.keyed.as_ref(), decoded[2].prepared.plain.as_ref()), (None, None), "a layer uploaded ahead was painted");
        assert_eq!(decoded[3].prepared, BgaPrepared { ahead: Some(Vec::new()), ..BgaPrepared::default() }, "a picture the chart never shows was painted");
        assert_eq!(room.load(Ordering::Relaxed), PREPARED_BGA_BYTES - canvas, "the room went to looks uploaded ahead");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The decode a play screen starts makes the plan itself, before it decodes: a chart whose
    /// pictures all fit is held in textures from before it plays, and nothing is painted for it.
    #[test]
    fn the_decode_plans_the_uploads_ahead_before_it_paints() {
        use rbms_render::skin_render::frame::BgaEvent;
        const SMALL: u32 = 100;
        let dir = temp_dir("bga-plan-spawn");
        image::RgbaImage::from_pixel(SMALL, SMALL, image::Rgba([7, 8, 9, 255])).save(dir.join("small.png")).expect("write the fixture");
        let shown = BgaPlayhead::new(vec![BgaEvent { time_ms: 0, base: 0, layer: 1, miss: None }]);
        let jobs = bga_jobs(&["small.png".to_string(), "small.png".to_string()], &dir, Some(&shown));
        let (rx, _, total) = spawn_bga_decode(jobs, Some(shown.first_looks()), Arc::new(AtomicBool::new(false)));
        let mut decoded: Vec<(i32, DecodedImage)> = rx.iter().take(total).collect();
        decoded.sort_by_key(|(number, _)| *number);
        let prepared: Vec<BgaPrepared> = decoded.into_iter().map(|(_, image)| image.prepared).collect();
        let planned = |look| BgaPrepared { ahead: Some(vec![look]), ..BgaPrepared::default() };
        assert_eq!(prepared, vec![planned(BgaLook::Plain), planned(BgaLook::Keyed)], "a picture held as a texture ahead of play was painted too");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The renderer tells one decode from the next by its number, and reads the pixels as they are.
    #[test]
    fn a_decoded_background_is_handed_to_the_renderer_with_its_own_number_and_size() {
        let image = DecodedImage::for_test(vec![1, 2, 3, 4, 5, 6, 7, 8], 2, 1);
        let picture = bga_picture(&image);
        assert_eq!((picture.generation, picture.width, picture.height), (image.generation, 2, 1));
        assert_eq!(picture.rgba, image.rgba.as_slice());
        assert_eq!(picture.prepared, Some(&image.prepared));
    }

    /// Edge of the sheet the stamp test writes, and of the one it saves over it.
    const SHEET_EDGE: u32 = 2;
    const EDITED_SHEET_EDGE: u32 = 3;

    /// A sheet the caller already has uploaded costs a worker one stat for as long as its file is
    /// the one that was uploaded, and is decoded again once the file has been written since. A
    /// caller that holds nothing of the file has it decoded whatever its stamp.
    #[test]
    fn a_skin_image_is_decoded_again_only_once_its_file_was_written() {
        let dir = temp_dir("skin-stamp");
        let path = dir.join("sheet.png");
        image::RgbaImage::from_pixel(SHEET_EDGE, SHEET_EDGE, image::Rgba([1, 2, 3, 255])).save(&path).expect("write the fixture");
        let job = (SkinAssetKind::Image, path.clone());

        let first = decode_skin_asset(&(job.clone(), None)).expect("the fixture decodes");
        assert!(matches!(&first.asset, SkinAsset::Image(image) if image.width == SHEET_EDGE), "a sheet nobody holds was not decoded");
        let stamp = first.stamp.expect("a file that was just written has a stamp");

        let again = decode_skin_asset(&(job.clone(), Some(stamp))).expect("the fixture is still there");
        assert!(matches!(again.asset, SkinAsset::Unchanged), "a sheet that was not written since was decoded again");
        assert_eq!(again.stamp, Some(stamp));

        image::RgbaImage::from_pixel(EDITED_SHEET_EDGE, SHEET_EDGE, image::Rgba([1, 2, 3, 255])).save(&path).expect("save over the fixture");
        let edited = decode_skin_asset(&(job, Some(stamp))).expect("the edited fixture decodes");
        assert!(matches!(&edited.asset, SkinAsset::Image(image) if image.width == EDITED_SHEET_EDGE), "a sheet that was saved over was taken for unchanged");
        assert_ne!(edited.stamp, Some(stamp));

        let missing = (SkinAssetKind::Image, dir.join("missing.png"));
        assert!(decode_skin_asset(&(missing, None)).is_none(), "a file that is not there was handed over");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_chart_with_no_images_starts_no_workers_and_is_done_at_once() {
        let (_rx, progress, total) = spawn_bga_decode(Vec::new(), None, Arc::new(AtomicBool::new(false)));
        assert_eq!(total, 0);
        assert_eq!(progress.load(Ordering::Relaxed), 0);
    }

    /// A file that will not decode still counts as attempted: a bar that waited for a result that
    /// never comes would sit at not-quite-full for ever.
    #[test]
    fn a_file_that_will_not_decode_still_reports_itself_done() {
        let dir = temp_dir("bga-broken");
        let path = dir.join("broken.png");
        std::fs::write(&path, b"not really a png").expect("write the fixture");
        let (rx, progress, total) = spawn_bga_decode(vec![(0, unshown(&path))], None, Arc::new(AtomicBool::new(false)));
        assert_eq!(total, 1);
        while progress.load(Ordering::Relaxed) < total {
            std::hint::spin_loop();
        }
        assert!(rx.try_recv().is_err(), "a broken file has no image to hand over");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_cancelled_decode_stops_instead_of_working_through_the_rest() {
        let dir = temp_dir("bga-cancel");
        let path = dir.join("cancelled.png");
        std::fs::write(&path, b"not really a png").expect("write the fixture");
        let cancel = Arc::new(AtomicBool::new(true));
        let jobs: Vec<(i32, BgaJob)> = (0..64).map(|id| (id, unshown(&path))).collect();
        let (_rx, progress, total) = spawn_bga_decode(jobs, None, cancel);
        assert_eq!(total, 64);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(progress.load(Ordering::Relaxed) < total, "a flag set before the pool started should stop it early");
        let _ = std::fs::remove_file(&path);
    }

    /// A skin's writes are filed under the pack it belongs to, beside the settings file and never
    /// inside the pack: one folder per pack however its path is spelled, a different one for another
    /// pack of the same name, and a name that is safe whatever the pack folder is called.
    #[test]
    fn a_skin_writes_under_a_folder_named_for_its_pack_beside_the_settings_file() {
        let home = temp_dir("skin-overlay");
        let settings = home.join("settings.ron");
        let pack = home.join("packs").join("Modern Chic!");
        let twin = home.join("other").join("Modern Chic!");
        std::fs::create_dir_all(&pack).expect("the pack folder is creatable");
        std::fs::create_dir_all(&twin).expect("the second pack folder is creatable");

        let overlay = skin_overlay_folder(&settings, &pack.join("select.luaskin"));
        assert_eq!(overlay.parent(), Some(home.join(SKIN_DATA_FOLDER).as_path()), "the overlay is not beside the settings file");
        assert!(!overlay.starts_with(&pack), "a skin would write into its own folder");
        assert!(!overlay.exists(), "the overlay was created before a skin wrote anything");

        let identifier = skin_pack_identifier(&pack);
        assert!(identifier.starts_with("Modern_Chic_-"), "the folder name was not made safe: {identifier}");
        assert_eq!(identifier, skin_pack_identifier(&pack), "one pack was given two identifiers");
        assert_eq!(overlay, skin_overlay_folder(&settings, &pack.join("play7.luaskin")), "two documents of one pack write to different folders");
        assert_eq!(identifier, skin_pack_identifier(&pack.join("..").join("Modern Chic!")), "another spelling of the same folder is another pack");
        assert_ne!(identifier, skin_pack_identifier(&twin), "two packs with the same folder name share an overlay");
        let _ = std::fs::remove_dir_all(&home);
    }
}

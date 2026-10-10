//! Everything the player reads off disk or out of the binary before a screen can use it: the
//! built-in note-field layouts, the system sound set shipped inside the binary, the UI theme
//! template, chart-relative file resolution, the keysound decode pool and the BGA image decode.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

use rbms_config::DEFAULT_SKIN_FOLDER;
use rbms_render::skin_render::frame::BgaPicture;
use rbms_render::{SkinConfig, SkinImage};
use sha2::{Digest, Sha256};

/// A decode running on the worker pool: what has arrived, how far it has got, the cooperative
/// cancel, and how many jobs there are in total.
pub(crate) type DecodePool<I, T> = (Receiver<(I, T)>, Arc<AtomicUsize>, usize);

/// Most workers a decode is fanned across. Past this the disk, not the CPU, is the limit.
const DECODE_THREADS_MAX: usize = 8;

/// Workers used when the machine will not say how many cores it has.
const DECODE_THREADS_FALLBACK: usize = 4;

pub(crate) const SKIN_NORMAL: &str = include_str!("../../../assets/skins/normal.ron");
pub(crate) const SKIN_WIDE: &str = include_str!("../../../assets/skins/wide.ron");

/// The folder the player's own skin is installed into, below the skin folder beside the settings
/// file.
const DEFAULT_SKIN_DIRECTORY: &str = "rbms-default";

/// The folder inside [`DEFAULT_SKIN_DIRECTORY`] that holds the system sound set, read when the
/// player configured no sound folder of their own.
const DEFAULT_SOUND_DIRECTORY: &str = "sound";

/// The folder beside the settings file that holds everything skins write, one folder per skin pack.
const SKIN_DATA_FOLDER: &str = "skin-data";

/// What a pack whose folder has no usable name is called in its identifier.
const SKIN_PACK_FALLBACK_NAME: &str = "skin";

/// Most characters of a pack folder's own name that go into its identifier.
const SKIN_PACK_NAME_MAX_CHARS: usize = 32;

/// What a character of a pack folder's name that is unsafe in a file name is replaced with.
const SKIN_PACK_NAME_FILLER: char = '_';

/// Bytes of the pack path's digest that go into its identifier, which is what tells two packs with
/// the same folder name apart.
const SKIN_PACK_DIGEST_BYTES: usize = 8;

/// One file shipped inside the binary and written to disk on the first run.
struct EmbeddedFile {
    name: &'static str,
    bytes: &'static [u8],
}

/// The shipped system sound for each stem, read out of `assets/skins/rbms-default/sound` at build
/// time. Written as stems so the table cannot name a file under one spelling and embed another.
macro_rules! embedded_sounds {
    ($($stem:literal),* $(,)?) => {
        &[$(EmbeddedFile {
            name: concat!($stem, ".wav"),
            bytes: include_bytes!(concat!("../../../assets/skins/rbms-default/sound/", $stem, ".wav")),
        }),*]
    };
}

/// Every system sound the player ships, one per stem of [`crate::syssound::SystemSound`].
const DEFAULT_SOUNDS: &[EmbeddedFile] = embedded_sounds![
    "scratch",
    "f-open",
    "f-close",
    "o-change",
    "o-open",
    "o-close",
    "playready",
    "playstop",
    "clear",
    "fail",
    "resultclose",
    "course_clear",
    "course_fail",
    "course_close",
    "guide-pg",
    "guide-gr",
    "guide-gd",
    "guide-bd",
    "guide-pr",
    "guide-ms",
    "select",
    "decide",
];

/// One of the built-in note-field layouts by name ("WIDE" or NORMAL). Used when no external
/// `--skin` is given.
pub(crate) fn bundled_skin(name: &str) -> SkinConfig {
    let s = if name.eq_ignore_ascii_case("WIDE") { SKIN_WIDE } else { SKIN_NORMAL };
    ron::from_str(s).unwrap_or_default()
}

/// Where the shipped system sound set is installed: always below the default skin folder beside the
/// settings file, whichever folder the SKIN tab is looking for documents in.
fn default_sound_directory(settings_path: &Path) -> PathBuf {
    settings_path.parent().unwrap_or(Path::new(".")).join(DEFAULT_SKIN_FOLDER).join(DEFAULT_SKIN_DIRECTORY).join(DEFAULT_SOUND_DIRECTORY)
}

/// Write every shipped system sound that is not on disk yet, and report whether the whole set is
/// there afterwards.
///
/// A file that already exists is never written over, so a sound the player replaced stays replaced.
/// Every sound is tried even after one fails, so a single unwritable file costs that one cue rather
/// than the rest of the set. Nothing else under the skin folder is read, moved or removed.
pub(crate) fn install_default_sounds(settings_path: &Path) -> bool {
    let directory = default_sound_directory(settings_path);
    let failed = DEFAULT_SOUNDS.iter().filter(|sound| !install_embedded_file(&directory.join(sound.name), sound.bytes)).count();
    failed == 0
}

/// The installed system sound set, for a player who configured no folder of their own. `None` while
/// the folder is not there, which leaves the set silent rather than naming a directory nothing can
/// be read from.
pub(crate) fn default_sound_folder(settings_path: &Path) -> Option<PathBuf> {
    let directory = default_sound_directory(settings_path);
    directory.is_dir().then_some(directory)
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
    let digest: String = digest.iter().take(SKIN_PACK_DIGEST_BYTES).map(|byte| format!("{byte:02x}")).collect();
    format!("{name}-{digest}")
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

/// Write one shipped file unless something is already at its path, and report whether a regular
/// file is there afterwards.
fn install_embedded_file(path: &Path, bytes: &[u8]) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => return path.is_file(),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return false,
        Err(_) => {}
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return path.is_file(),
        Err(_) => return false,
    };
    if file.write_all(bytes).is_ok() {
        return true;
    }
    drop(file);
    let _ = std::fs::remove_file(path);
    false
}

/// A fully-populated, commented UI-theme template written to `~/.config/rbms/theme.ron` on first run
/// so the palette is discoverable and editable. Each colour is `Some((r,g,b))`; omit a line to keep
/// its default. The in-play note field is themed separately by the skin. See `docs/theme.md`.
pub(crate) const THEME_TEMPLATE: &str = "\
// rbms UI theme — colours for the song-select / result / menu chrome (0-255 per channel).
// Omit any line to keep its default. The in-play note field is themed by the skin, not here.
(
    bg:            Some((8, 8, 14)),      // window background
    topbar:        Some((18, 18, 30)),    // top header bar
    panel:         Some((14, 16, 26)),    // panel fill
    panel_hi:      Some((22, 24, 36)),    // raised panel / button
    divider:       Some((40, 42, 58)),    // dividers / outlines
    text:          Some((235, 235, 235)), // primary text
    text_dim:      Some((170, 170, 185)), // secondary text
    text_muted:    Some((120, 124, 140)), // muted hints
    accent:        Some((120, 205, 235)), // accents / counts / sort label
    focus:         Some((120, 200, 240)), // focus ring / selection rails
    title_focus:   Some((244, 236, 156)), // focused row title
    title_dim:     Some((190, 186, 150)), // unfocused row title
    row_even:      Some((22, 24, 34)),
    row_odd:       Some((27, 29, 42)),
    row_folder:    Some((40, 44, 30)),
    row_focus:     Some((50, 56, 82)),
    lamp_default:  Some((44, 44, 54)),     // clear-lamp with no record yet
    button:        Some((70, 80, 120)),    // selected tab / button fill
    button_active: Some((40, 56, 80)),     // active/toggled button, search box
    good:          Some((90, 200, 230)),   // progress / score-graph bars
    warn:          Some((230, 180, 60)),
)
";

/// Read the UI theme beside the settings file, writing the template there first when there is none
/// to read.
pub(crate) fn load_theme(settings_path: &Path) {
    let path = settings_path.parent().map(|directory| directory.join("theme.ron")).unwrap_or_else(|| PathBuf::from("theme.ron"));
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, THEME_TEMPLATE);
            THEME_TEMPLATE.to_string()
        }
    };
    rbms_render::set_theme(rbms_render::ThemeConfig::parse(&src).resolve());
    println!("theme: {}", path.display());
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
    if total == 0 {
        return (rx, progress, 0);
    }
    let nthreads = std::thread::available_parallelism().map(|c| c.get().min(DECODE_THREADS_MAX)).unwrap_or(DECODE_THREADS_FALLBACK).max(1);
    let chunk = total.div_ceil(nthreads).max(1);
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
    (rx, progress, total)
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
}

impl DecodedImage {
    /// One image built in a test, taking a generation of its own like any other decode.
    #[cfg(test)]
    pub(crate) fn for_test(rgba: Vec<u8>, width: u32, height: u32) -> DecodedImage {
        DecodedImage { rgba, width, height, generation: NEXT_IMAGE_GENERATION.fetch_add(1, Ordering::Relaxed) }
    }
}

/// Which half of a skin document's files one decode job carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SkinAssetKind {
    Image,
    Font,
}

/// One of a skin document's files, read and decoded off the frame loop.
#[derive(Debug)]
pub(crate) enum SkinAsset {
    Image(SkinImage),
    Font(Vec<u8>),
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

/// Resolve every referenced background image in a chart's `bgamap` to `(id, path)` decode jobs
/// (empty names skipped, unresolvable files dropped).
pub(crate) fn bga_jobs(bgamap: &[String], dir: &Path) -> Vec<(i32, PathBuf)> {
    bgamap
        .iter()
        .enumerate()
        .filter(|(_, name)| !name.trim().is_empty())
        .filter_map(|(id, name)| resolve_bga_file(dir, name).map(|path| (id as i32, path)))
        .collect()
}

/// Fan background-image decode out over the worker pool. A chart can reference hundreds of images
/// and each is resized on the way in, which is seconds of work the frame loop cannot spend.
pub(crate) fn spawn_bga_decode(jobs: Vec<(i32, PathBuf)>, cancel: Arc<AtomicBool>) -> DecodePool<i32, DecodedImage> {
    spawn_decode(jobs, cancel, decode_bga_file)
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
    Some(DecodedImage { rgba: rgba.into_raw(), width, height, generation: NEXT_IMAGE_GENERATION.fetch_add(1, Ordering::Relaxed) })
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
    BgaPicture { generation: image.generation, width: image.width, height: image.height, rgba: &image.rgba }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let jobs = bga_jobs(&[String::new(), "second.png".into(), "missing.png".into(), "   ".into()], &dir);
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

    /// The renderer tells one decode from the next by its number, and reads the pixels as they are.
    #[test]
    fn a_decoded_background_is_handed_to_the_renderer_with_its_own_number_and_size() {
        let image = DecodedImage::for_test(vec![1, 2, 3, 4, 5, 6, 7, 8], 2, 1);
        let picture = bga_picture(&image);
        assert_eq!((picture.generation, picture.width, picture.height), (image.generation, 2, 1));
        assert_eq!(picture.rgba, image.rgba.as_slice());
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
        let (_rx, progress, total) = spawn_bga_decode(Vec::new(), Arc::new(AtomicBool::new(false)));
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
        let (rx, progress, total) = spawn_bga_decode(vec![(0, path.clone())], Arc::new(AtomicBool::new(false)));
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
        let jobs: Vec<(i32, PathBuf)> = (0..64).map(|id| (id, path.clone())).collect();
        let (_rx, progress, total) = spawn_bga_decode(jobs, cancel);
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

    /// A first run on an empty configuration folder writes the whole shipped sound set below the
    /// default skin folder, one file per stem the player can raise, and that folder is then the one
    /// read for a player who configured none.
    #[test]
    fn a_first_run_installs_every_shipped_sound_below_the_default_skin_folder() {
        let home = temp_dir("sound-install");
        let settings = home.join(".config/rbms/settings.ron");
        assert_eq!(default_sound_folder(&settings), None, "a folder nothing was installed into was named as the sound set");

        assert!(install_default_sounds(&settings));
        let sounds = home.join(".config/rbms").join(DEFAULT_SKIN_FOLDER).join(DEFAULT_SKIN_DIRECTORY).join(DEFAULT_SOUND_DIRECTORY);
        assert_eq!(default_sound_folder(&settings), Some(sounds.clone()));
        assert_eq!(DEFAULT_SOUNDS.len(), crate::syssound::SYSTEM_SOUND_COUNT, "the shipped set and the set the player raises are different sizes");
        for sound in crate::syssound::SystemSound::ALL {
            let shipped = DEFAULT_SOUNDS.iter().find(|file| Path::new(file.name).file_stem().and_then(|stem| stem.to_str()) == Some(sound.file_stem()));
            let shipped = shipped.unwrap_or_else(|| panic!("no sound is shipped for {sound:?}"));
            let installed = std::fs::read(sounds.join(shipped.name)).unwrap_or_else(|error| panic!("{} was not installed: {error}", shipped.name));
            assert_eq!(installed, shipped.bytes, "{} is not the file that was shipped", shipped.name);
        }
        let written = std::fs::read_dir(&sounds).expect("the sound folder lists").count();
        assert_eq!(written, DEFAULT_SOUNDS.len(), "the install wrote something that is not a shipped sound");
        assert!(!settings.exists(), "installing the sound set wrote a settings file");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A later run finds the set already there and writes over none of it: a sound the player
    /// replaced stays replaced, one they deleted comes back, and everything else under the skin
    /// folder -- their own documents, a folder an earlier build installed -- is left exactly as it
    /// was.
    #[test]
    fn a_later_run_repairs_the_sound_set_without_writing_over_anything() {
        let home = temp_dir("sound-reinstall");
        let settings = home.join(".config/rbms/settings.ron");
        let skin_folder = home.join(".config/rbms").join(DEFAULT_SKIN_FOLDER);
        let user_document = skin_folder.join("mine/select.json5");
        let earlier_install = skin_folder.join("earlier-bundle/theme.ron");
        for (path, text) in [(&user_document, "user document"), (&earlier_install, "earlier install")] {
            std::fs::create_dir_all(path.parent().expect("the fixture has a folder")).expect("create the fixture folder");
            std::fs::write(path, text).expect("write the fixture");
        }
        assert!(install_default_sounds(&settings));

        let sounds = default_sound_folder(&settings).expect("the set was installed");
        let replaced = sounds.join(DEFAULT_SOUNDS[0].name);
        let deleted = sounds.join(DEFAULT_SOUNDS[1].name);
        std::fs::write(&replaced, "the player's own sound").expect("replace a shipped sound");
        std::fs::remove_file(&deleted).expect("delete a shipped sound");

        assert!(install_default_sounds(&settings));
        assert_eq!(std::fs::read_to_string(&replaced).expect("read the replaced sound"), "the player's own sound", "a replaced sound was written over");
        assert_eq!(std::fs::read(&deleted).expect("read the repaired sound"), DEFAULT_SOUNDS[1].bytes, "a deleted sound did not come back");
        assert_eq!(std::fs::read_to_string(&user_document).expect("read the user document"), "user document");
        assert_eq!(std::fs::read_to_string(&earlier_install).expect("read the earlier install"), "earlier install");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A skin folder that cannot be written into leaves the set silent rather than half installed,
    /// and the next run installs it once the folder can be made.
    #[test]
    fn sound_installation_retries_after_an_unwritable_skin_folder() {
        let home = temp_dir("sound-retry");
        let settings = home.join("settings.ron");
        let blocked = home.join(DEFAULT_SKIN_FOLDER);
        std::fs::write(&blocked, "not a folder").expect("write the blocking file");

        assert!(!install_default_sounds(&settings));
        assert_eq!(default_sound_folder(&settings), None, "a set that could not be installed was named anyway");
        assert_eq!(std::fs::read_to_string(&blocked).expect("read the blocking file"), "not a folder", "the install removed what was in its way");

        std::fs::remove_file(&blocked).expect("remove the blocking file");
        assert!(install_default_sounds(&settings));
        assert!(default_sound_folder(&settings).is_some());
        let _ = std::fs::remove_dir_all(&home);
    }
}

//! Everything the player reads off disk or out of the binary before a screen can use it: the
//! bundled skins, the UI theme template, chart-relative file resolution, the keysound decode pool
//! and the BGA image decode.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;

use rbms_config::{Config, DEFAULT_SKIN, STEEL_NEON_SKIN};
use rbms_model::Mode;
use rbms_render::{SkinConfig, SkinImage};

/// A decode running on the worker pool: what has arrived, how far it has got, the cooperative
/// cancel, and how many jobs there are in total.
pub(crate) type DecodePool<I, T> = (Receiver<(I, T)>, Arc<AtomicUsize>, usize);

/// Most workers a decode is fanned across. Past this the disk, not the CPU, is the limit.
const DECODE_THREADS_MAX: usize = 8;

/// Workers used when the machine will not say how many cores it has.
const DECODE_THREADS_FALLBACK: usize = 4;

pub(crate) const SKIN_NORMAL: &str = include_str!("../../../assets/skins/normal.ron");
pub(crate) const SKIN_WIDE: &str = include_str!("../../../assets/skins/wide.ron");

const STEEL_NEON_DIRECTORY: &str = "steel-neon";
const SKIN_TYPE_PLAY_5KEYS: i32 = 1;
const SKIN_TYPE_PLAY_14KEYS: i32 = 2;
const SKIN_TYPE_PLAY_10KEYS: i32 = 3;
const SKIN_TYPE_PLAY_9KEYS: i32 = 4;

struct BundledFile {
    path: &'static str,
    bytes: &'static [u8],
}

const STEEL_NEON_FILES: &[BundledFile] = &[
    BundledFile { path: "select.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/select.json5") },
    BundledFile { path: "decide.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/decide.json5") },
    BundledFile { path: "play-7k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/play-7k.json5") },
    BundledFile { path: "play-5k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/play-5k.json5") },
    BundledFile { path: "play-14k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/play-14k.json5") },
    BundledFile { path: "play-10k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/play-10k.json5") },
    BundledFile { path: "play-9k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/play-9k.json5") },
    BundledFile { path: "play-24k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/play-24k.json5") },
    BundledFile { path: "result.json5", bytes: include_bytes!("../../../assets/skins/steel-neon/result.json5") },
    BundledFile { path: "play.ron", bytes: include_bytes!("../../../assets/skins/steel-neon/play.ron") },
    BundledFile { path: "play-dual.ron", bytes: include_bytes!("../../../assets/skins/steel-neon/play-dual.ron") },
    BundledFile { path: "theme.ron", bytes: include_bytes!("../../../assets/skins/steel-neon/theme.ron") },
    BundledFile { path: "palette.json", bytes: include_bytes!("../../../assets/skins/steel-neon/palette.json") },
    BundledFile { path: "tools/generate-assets.py", bytes: include_bytes!("../../../assets/skins/steel-neon/tools/generate-assets.py") },
    BundledFile { path: "images/common-atlas.png", bytes: include_bytes!("../../../assets/skins/steel-neon/images/common-atlas.png") },
    BundledFile { path: "images/select-overlay.png", bytes: include_bytes!("../../../assets/skins/steel-neon/images/select-overlay.png") },
    BundledFile { path: "images/decide-backdrop.png", bytes: include_bytes!("../../../assets/skins/steel-neon/images/decide-backdrop.png") },
    BundledFile { path: "images/play-overlay.png", bytes: include_bytes!("../../../assets/skins/steel-neon/images/play-overlay.png") },
    BundledFile { path: "images/play-dual-overlay.png", bytes: include_bytes!("../../../assets/skins/steel-neon/images/play-dual-overlay.png") },
    BundledFile { path: "images/result-overlay.png", bytes: include_bytes!("../../../assets/skins/steel-neon/images/result-overlay.png") },
];

const STEEL_NEON_DOCUMENTS: &[(i32, &str)] = &[
    (rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT, "select.json5"),
    (rbms_skin::loader::SKIN_TYPE_DECIDE, "decide.json5"),
    (rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS, "play-7k.json5"),
    (SKIN_TYPE_PLAY_5KEYS, "play-5k.json5"),
    (SKIN_TYPE_PLAY_14KEYS, "play-14k.json5"),
    (SKIN_TYPE_PLAY_10KEYS, "play-10k.json5"),
    (SKIN_TYPE_PLAY_9KEYS, "play-9k.json5"),
    (rbms_skin::loader::SKIN_TYPE_RESULT, "result.json5"),
];

/// One of the bundled skins by name ("WIDE" or NORMAL). Used when no external `--skin` is given.
pub(crate) fn bundled_skin(name: &str) -> SkinConfig {
    let s = if name.eq_ignore_ascii_case("WIDE") { SKIN_WIDE } else { SKIN_NORMAL };
    ron::from_str(s).unwrap_or_default()
}

pub(crate) fn installed_play_skin_path(settings_path: &Path, config: &Config, mode: Mode) -> PathBuf {
    let filename = if matches!(mode, Mode::BEAT_10K | Mode::BEAT_14K) { "play-dual.ron" } else { "play.ron" };
    crate::skin_select::skin_root(settings_path, config).join(STEEL_NEON_DIRECTORY).join(filename)
}

fn active_theme_path(settings_path: &Path, config: &Config) -> PathBuf {
    if config.display.skin.eq_ignore_ascii_case(STEEL_NEON_SKIN) {
        return crate::skin_select::skin_root(settings_path, config).join(STEEL_NEON_DIRECTORY).join("theme.ron");
    }
    settings_path.parent().map(|directory| directory.join("theme.ron")).unwrap_or_else(|| PathBuf::from("theme.ron"))
}

pub(crate) fn install_default_skin(settings_path: &Path, config: &mut Config) -> bool {
    let root = crate::skin_select::skin_root(settings_path, config);
    let directory = root.join(STEEL_NEON_DIRECTORY);
    let mut skin_ready = true;
    for file in STEEL_NEON_FILES {
        skin_ready = install_bundled_file(&directory.join(file.path), file.bytes) && skin_ready;
    }
    let theme_path = settings_path.parent().unwrap_or(Path::new(".")).join("theme.ron");
    let theme_ready = install_bundled_file(&theme_path, THEME_TEMPLATE.as_bytes());

    if skin_ready {
        for (screen, file) in STEEL_NEON_DOCUMENTS {
            let path = directory.join(file);
            if config.skin.document(*screen).is_none() && path.is_file() {
                config.skin.select(*screen, Some(path.to_string_lossy().into_owned()));
            }
        }
        if config.display.skin.eq_ignore_ascii_case(DEFAULT_SKIN) && directory.join("play.ron").is_file() && directory.join("play-dual.ron").is_file() {
            config.display.skin = STEEL_NEON_SKIN.to_string();
        }
    }
    skin_ready && theme_ready
}

fn install_bundled_file(path: &Path, bytes: &[u8]) -> bool {
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

pub(crate) fn load_theme(settings_path: &Path, config: &Config) {
    let path = active_theme_path(settings_path, config);
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) if config.display.skin.eq_ignore_ascii_case(STEEL_NEON_SKIN) => String::new(),
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
}

/// What one skin decode job names.
pub(crate) type SkinAssetJob = (SkinAssetKind, PathBuf);

/// Decode one of a document's files.
fn decode_skin_asset(job: &SkinAssetJob) -> Option<SkinAsset> {
    let (kind, path) = job;
    match kind {
        SkinAssetKind::Image => {
            let decoded = image::open(path).ok()?.to_rgba8();
            let (width, height) = decoded.dimensions();
            SkinImage::new(width, height, decoded.into_raw()).map(SkinAsset::Image)
        }
        SkinAssetKind::Font => std::fs::read(path).ok().map(SkinAsset::Font),
    }
}

/// Fan a skin document's images and fonts out over the worker pool.
///
/// A published skin names dozens of source images and some of them are two thousand pixels square,
/// which is seconds of decoding the frame loop cannot spend. Until the pool is done the screen it
/// belongs to draws its built-in layout.
pub(crate) fn spawn_skin_asset_decode(jobs: Vec<SkinAssetJob>, cancel: Arc<AtomicBool>) -> DecodePool<SkinAssetJob, SkinAsset> {
    spawn_decode(jobs.into_iter().map(|job| (job.clone(), job)).collect(), cancel, decode_skin_asset)
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

    #[test]
    fn default_skin_installation_preserves_user_files_and_selections() {
        let dir = temp_dir("default-skin-install");
        let settings = dir.join("settings.ron");
        let root = dir.join("skin");
        let user_document = root.join("user-select.json5");
        std::fs::create_dir_all(&root).expect("create the skin folder");
        std::fs::write(&user_document, r#"{ type: 5, name: 'User', w: 1280, h: 720, destination: [] }"#).expect("write the user document");

        let mut config = Config::default();
        config.skin.select(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT, Some(user_document.to_string_lossy().into_owned()));
        assert!(install_default_skin(&settings, &mut config));
        assert_eq!(config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT), Some(user_document.to_string_lossy().as_ref()));
        assert_eq!(config.display.skin, STEEL_NEON_SKIN);
        assert!(settings.parent().expect("settings has a parent").join("theme.ron").is_file());

        let installed_single = installed_play_skin_path(&settings, &config, Mode::BEAT_7K);
        let installed_dual = installed_play_skin_path(&settings, &config, Mode::BEAT_14K);
        assert!(installed_single.is_file(), "single-field play skin is not installed");
        assert!(installed_dual.is_file(), "dual-field play skin is not installed");
        assert_ne!(installed_single, installed_dual, "single and dual fields share the same layout file");

        let single_config = SkinConfig::load(&installed_single).expect("single-field play skin parses");
        let single_skin = rbms_render::Skin::build(&single_config, Mode::BEAT_7K, 1280.0, 720.0);
        let single_field_right = single_skin.x.iter().zip(&single_skin.w).map(|(x, width)| x + width).fold(f32::MIN, f32::max);
        assert!(single_field_right < single_skin.bga.expect("single-field BGA").x, "single field reaches the central BGA");

        let dual_config = SkinConfig::load(&installed_dual).expect("dual-field play skin parses");
        for mode in [Mode::BEAT_10K, Mode::BEAT_14K] {
            let dual_skin = rbms_render::Skin::build(&dual_config, mode, 1280.0, 720.0);
            let dual_field_right = dual_skin.x.iter().zip(&dual_skin.w).map(|(x, width)| x + width).fold(f32::MIN, f32::max);
            assert_eq!(dual_skin.fields.len(), 2, "{mode:?} is not a two-field layout");
            assert!(dual_field_right < dual_skin.bga.expect("dual-field BGA").x, "{mode:?} reaches the BGA");
        }

        let edited = "user play skin";
        let installed_play = installed_single;
        std::fs::write(&installed_play, edited).expect("edit the installed play skin");
        let installed_theme = active_theme_path(&settings, &config);
        std::fs::write(&installed_theme, "user theme").expect("edit the installed theme");
        assert!(install_default_skin(&settings, &mut config));
        assert_eq!(std::fs::read_to_string(&installed_play).expect("read the edited play skin"), edited);
        assert_eq!(std::fs::read_to_string(&installed_theme).expect("read the edited theme"), "user theme");
        assert_eq!(std::fs::read_to_string(&user_document).expect("read the user document"), r#"{ type: 5, name: 'User', w: 1280, h: 720, destination: [] }"#);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_skin_installation_retries_after_an_unwritable_skin_root() {
        let dir = temp_dir("default-skin-retry");
        let settings = dir.join("settings.ron");
        let blocked_root = dir.join("blocked-root");
        std::fs::write(&blocked_root, "not a folder").expect("write the blocked root");

        let mut config = Config::default();
        config.skin.folder = Some(blocked_root.to_string_lossy().into_owned());
        assert!(!install_default_skin(&settings, &mut config));
        assert_eq!(config.skin.document(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS), None);
        assert_eq!(config.display.skin, DEFAULT_SKIN);

        std::fs::remove_file(&blocked_root).expect("remove the blocked root");
        assert!(install_default_skin(&settings, &mut config));
        assert!(config.skin.document(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS).is_some());
        let unsupported = rbms_skin::loader::mode_skin_type(rbms_model::Mode::KEYBOARD_24K).expect("the document type is known");
        assert_eq!(config.skin.document(unsupported), None);
        assert_eq!(config.display.skin, STEEL_NEON_SKIN);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selected_preset_loads_its_installed_theme_path() {
        let settings = PathBuf::from("/tmp/rbms/settings.ron");
        let mut config = Config::default();
        assert_eq!(active_theme_path(&settings, &config), PathBuf::from("/tmp/rbms/theme.ron"));

        config.display.skin = "WIDE".to_string();
        assert_eq!(active_theme_path(&settings, &config), PathBuf::from("/tmp/rbms/theme.ron"));

        config.display.skin = STEEL_NEON_SKIN.to_string();
        assert_eq!(active_theme_path(&settings, &config), PathBuf::from("/tmp/rbms/skin/steel-neon/theme.ron"));
    }
}

//! Everything the player reads off disk or out of the binary before a screen can use it: the
//! bundled skins, the UI theme template, chart-relative file resolution, the keysound decode pool
//! and the BGA image decode, plus the library scan the loading screen drives.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;

use rbms_library::SongEntry;
use rbms_render::SkinConfig;

use crate::gpu;

/// A decode running on the worker pool: what has arrived, how far it has got, the cooperative
/// cancel, and how many jobs there are in total.
pub(crate) type DecodePool<I, T> = (Receiver<(I, T)>, Arc<AtomicUsize>, usize);

/// Most workers a decode is fanned across. Past this the disk, not the CPU, is the limit.
const DECODE_THREADS_MAX: usize = 8;

/// Workers used when the machine will not say how many cores it has.
const DECODE_THREADS_FALLBACK: usize = 4;

pub(crate) const SKIN_NORMAL: &str = include_str!("../../../assets/skins/normal.ron");
pub(crate) const SKIN_WIDE: &str = include_str!("../../../assets/skins/wide.ron");

/// One of the bundled skins by name ("WIDE" or NORMAL). Used when no external `--skin` is given.
pub(crate) fn bundled_skin(name: &str) -> SkinConfig {
    let s = if name.eq_ignore_ascii_case("WIDE") { SKIN_WIDE } else { SKIN_NORMAL };
    ron::from_str(s).unwrap_or_default()
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

/// Load the UI theme from `~/.config/rbms/theme.ron`, writing the editable template there on first
/// run. A missing/broken file falls back to the built-in defaults.
pub(crate) fn load_theme(settings_path: &Path) {
    let path = settings_path.parent().map(|d| d.join("theme.ron")).unwrap_or_else(|| PathBuf::from("theme.ron"));
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
pub(crate) fn spawn_bga_decode(jobs: Vec<(i32, PathBuf)>, cancel: Arc<AtomicBool>) -> DecodePool<i32, Vec<u8>> {
    spawn_decode(jobs, cancel, decode_bga_file)
}

/// Where a chart's named background image is on disk.
fn resolve_bga_file(dir: &Path, name: &str) -> Option<PathBuf> {
    resolve_file(dir, name, &["png", "bmp", "jpg", "jpeg"]).map(|(path, _)| path)
}

/// Decode one background image to the square the GPU holds them in.
fn decode_bga_file(path: &PathBuf) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(img.resize_exact(gpu::BGA_DIM, gpu::BGA_DIM, image::imageops::FilterType::Triangle).to_rgba8().into_raw())
}

/// Decode one named background image, for the single cover the browser shows.
pub(crate) fn decode_bga_256(dir: &Path, name: &str) -> Option<Vec<u8>> {
    decode_bga_file(&resolve_bga_file(dir, name)?)
}

/// Scan every configured library folder into one song list, stopping when `cancel` is set so
/// leaving the loading screen does not leave a worker walking the disk behind it.
pub(crate) fn scan_folders(folders: &[String], count: &AtomicUsize, cancel: &AtomicBool) -> Vec<SongEntry> {
    rbms_library::scan_folders(folders, count, cancel)
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
}

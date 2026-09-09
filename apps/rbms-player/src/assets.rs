//! Everything the player reads off disk or out of the binary before a screen can use it: the
//! bundled skins, the UI theme template, chart-relative file resolution, the keysound decode pool
//! and the BGA image decode, plus the library scan the loading screen drives.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use rbms_library::SongEntry;
use rbms_render::SkinConfig;

use crate::gpu;

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

/// Fan keysound decode out over a thread pool, streaming `(id, decoded)` back over a channel with a
/// progress counter. Workers check `cancel` between jobs so an abandoned load (e.g. the preview
/// focus moved on) stops promptly instead of decoding to the end. Shared by the Play loader (which
/// passes a never-set flag) and the autoplay preview.
pub(crate) fn spawn_keysound_decode(
    jobs: Vec<(u32, PathBuf, String)>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> (std::sync::mpsc::Receiver<(u32, rbms_audio::DecodedAudio)>, std::sync::Arc<std::sync::atomic::AtomicUsize>, usize) {
    use std::sync::atomic::Ordering;
    let total = jobs.len();
    let (tx, rx) = std::sync::mpsc::channel();
    let progress = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    if total == 0 {
        return (rx, progress, 0);
    }
    let nthreads = std::thread::available_parallelism().map(|c| c.get().min(8)).unwrap_or(4).max(1);
    let chunk = total.div_ceil(nthreads).max(1);
    for jc in jobs.chunks(chunk).map(<[_]>::to_vec) {
        let tx = tx.clone();
        let progress = progress.clone();
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            for (id, path, ext) in jc {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(data) = std::fs::read(&path)
                    && let Ok(dec) = rbms_audio::decode_bytes(data, Some(ext.as_str()))
                {
                    let _ = tx.send((id, dec));
                }
                progress.fetch_add(1, Ordering::Relaxed);
            }
        });
    }
    (rx, progress, total)
}

pub(crate) fn decode_bga_256(dir: &Path, name: &str) -> Option<Vec<u8>> {
    let (path, _) = resolve_file(dir, name, &["png", "bmp", "jpg", "jpeg"])?;
    let bytes = std::fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(img.resize_exact(gpu::BGA_DIM, gpu::BGA_DIM, image::imageops::FilterType::Triangle).to_rgba8().into_raw())
}

/// Scan every configured library folder into one song list. Wraps the cancellable
/// `rbms_library` scan; the loading screen does not offer a cancel yet, so the flag is never set.
pub(crate) fn scan_folders(folders: &[String], count: &std::sync::atomic::AtomicUsize) -> Vec<SongEntry> {
    static SCAN_NEVER_CANCELLED: AtomicBool = AtomicBool::new(false);
    rbms_library::scan_folders(folders, count, &SCAN_NEVER_CANCELLED)
}

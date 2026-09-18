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

const STEEL_NEON_V1_DIRECTORY: &str = "steel-neon";
const STEEL_NEON_V2_DIRECTORY: &str = "steel-neon-v2";
const STEEL_NEON_V3_DIRECTORY: &str = "steel-neon-v3";
const SKIN_TYPE_PLAY_5KEYS: i32 = 1;
const SKIN_TYPE_PLAY_14KEYS: i32 = 2;
const SKIN_TYPE_PLAY_10KEYS: i32 = 3;
const SKIN_TYPE_PLAY_9KEYS: i32 = 4;

/// The folder a bundle keeps its system sound set in, read when the player configured none of their
/// own.
const BUNDLE_SOUND_DIRECTORY: &str = "sound";

struct BundledFile {
    path: &'static str,
    bytes: &'static [u8],
}

/// One shipped generation of the bundled skin: the directory it installs into, everything it is made
/// of, and the document each screen is drawn with.
///
/// Generations are listed oldest first and the last is the current one. An older generation is only
/// ever repaired -- never created -- so a player who edited one keeps it, while a fresh install gets
/// the current one alone.
struct BundleGeneration {
    directory: &'static str,
    files: &'static [BundledFile],
    documents: &'static [(i32, &'static str)],
}

const STEEL_NEON_V1_FILES: &[BundledFile] = &[
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

const STEEL_NEON_V2_FILES: &[BundledFile] = &[
    BundledFile { path: "select.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/select.json5") },
    BundledFile { path: "decide.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/decide.json5") },
    BundledFile { path: "play-7k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play-7k.json5") },
    BundledFile { path: "play-5k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play-5k.json5") },
    BundledFile { path: "play-14k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play-14k.json5") },
    BundledFile { path: "play-10k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play-10k.json5") },
    BundledFile { path: "play-9k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play-9k.json5") },
    BundledFile { path: "play-24k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play-24k.json5") },
    BundledFile { path: "result.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/result.json5") },
    BundledFile { path: "play.ron", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play.ron") },
    BundledFile { path: "play-dual.ron", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/play-dual.ron") },
    BundledFile { path: "theme.ron", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/theme.ron") },
    BundledFile { path: "palette.json", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/palette.json") },
    BundledFile { path: "tools/generate-assets.py", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/tools/generate-assets.py") },
    BundledFile { path: "images/common-atlas.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/common-atlas.png") },
    BundledFile { path: "images/decide-backdrop.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/decide-backdrop.png") },
    BundledFile { path: "images/select-background.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/select-background.png") },
    BundledFile { path: "images/select-foreground.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/select-foreground.png") },
    BundledFile { path: "images/play-background.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/play-background.png") },
    BundledFile { path: "images/play-foreground.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/play-foreground.png") },
    BundledFile { path: "images/play-dual-background.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/play-dual-background.png") },
    BundledFile { path: "images/play-dual-foreground.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/play-dual-foreground.png") },
    BundledFile { path: "images/result-background.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/result-background.png") },
    BundledFile { path: "images/result-foreground.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v2/images/result-foreground.png") },
];

const STEEL_NEON_V3_FILES: &[BundledFile] = &[
    BundledFile { path: "select.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/select.json5") },
    BundledFile { path: "decide.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/decide.json5") },
    BundledFile { path: "play-7k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play-7k.json5") },
    BundledFile { path: "play-5k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play-5k.json5") },
    BundledFile { path: "play-14k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play-14k.json5") },
    BundledFile { path: "play-10k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play-10k.json5") },
    BundledFile { path: "play-9k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play-9k.json5") },
    BundledFile { path: "play-24k.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play-24k.json5") },
    BundledFile { path: "result.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/result.json5") },
    BundledFile { path: "shared/judge-sp.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/shared/judge-sp.json5") },
    BundledFile { path: "shared/objects-play.json5", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/shared/objects-play.json5") },
    BundledFile { path: "play.ron", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play.ron") },
    BundledFile { path: "play-dual.ron", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/play-dual.ron") },
    BundledFile { path: "theme.ron", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/theme.ron") },
    BundledFile { path: "palette.json", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/palette.json") },
    BundledFile { path: "tools/generate-assets.py", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/tools/generate-assets.py") },
    BundledFile { path: "tools/generate-sounds.py", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/tools/generate-sounds.py") },
    BundledFile { path: "images/covers.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/covers.png") },
    BundledFile { path: "images/decide-bg.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/decide-bg.png") },
    BundledFile { path: "images/digits-f.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/digits-f.png") },
    BundledFile { path: "images/digits-l.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/digits-l.png") },
    BundledFile { path: "images/digits-m.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/digits-m.png") },
    BundledFile { path: "images/digits-s.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/digits-s.png") },
    BundledFile { path: "images/frame-dp.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/frame-dp.png") },
    BundledFile { path: "images/frame-result.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/frame-result.png") },
    BundledFile { path: "images/frame-select.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/frame-select.png") },
    BundledFile { path: "images/frame-sp-2p-near.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/frame-sp-2p-near.png") },
    BundledFile { path: "images/frame-sp-2p.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/frame-sp-2p.png") },
    BundledFile { path: "images/frame-sp-near.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/frame-sp-near.png") },
    BundledFile { path: "images/frame-sp.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/frame-sp.png") },
    BundledFile { path: "images/notes.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/notes.png") },
    BundledFile { path: "images/play-bg.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/play-bg.png") },
    BundledFile { path: "images/result-bg-a.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/result-bg-a.png") },
    BundledFile { path: "images/result-bg-aa.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/result-bg-aa.png") },
    BundledFile { path: "images/result-bg-aaa.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/result-bg-aaa.png") },
    BundledFile { path: "images/result-bg-clear.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/result-bg-clear.png") },
    BundledFile { path: "images/result-bg-failed.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/result-bg-failed.png") },
    BundledFile { path: "images/select-bg.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/select-bg.png") },
    BundledFile { path: "images/ui.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/ui.png") },
    BundledFile { path: "images/decide/Default.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/decide/Default.png") },
    BundledFile { path: "images/covers/Gradient.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/covers/Gradient.png") },
    BundledFile { path: "images/covers/Solid.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/covers/Solid.png") },
    BundledFile { path: "images/notes/Default.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/notes/Default.png") },
    BundledFile { path: "images/select/Default.png", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/images/select/Default.png") },
    BundledFile { path: "sound/clear.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/clear.wav") },
    BundledFile { path: "sound/course_clear.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/course_clear.wav") },
    BundledFile { path: "sound/course_close.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/course_close.wav") },
    BundledFile { path: "sound/course_fail.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/course_fail.wav") },
    BundledFile { path: "sound/decide.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/decide.wav") },
    BundledFile { path: "sound/f-close.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/f-close.wav") },
    BundledFile { path: "sound/f-open.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/f-open.wav") },
    BundledFile { path: "sound/fail.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/fail.wav") },
    BundledFile { path: "sound/guide-bd.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/guide-bd.wav") },
    BundledFile { path: "sound/guide-gd.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/guide-gd.wav") },
    BundledFile { path: "sound/guide-gr.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/guide-gr.wav") },
    BundledFile { path: "sound/guide-ms.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/guide-ms.wav") },
    BundledFile { path: "sound/guide-pg.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/guide-pg.wav") },
    BundledFile { path: "sound/guide-pr.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/guide-pr.wav") },
    BundledFile { path: "sound/o-change.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/o-change.wav") },
    BundledFile { path: "sound/o-close.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/o-close.wav") },
    BundledFile { path: "sound/o-open.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/o-open.wav") },
    BundledFile { path: "sound/playready.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/playready.wav") },
    BundledFile { path: "sound/playstop.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/playstop.wav") },
    BundledFile { path: "sound/resultclose.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/resultclose.wav") },
    BundledFile { path: "sound/scratch.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/scratch.wav") },
    BundledFile { path: "sound/select.wav", bytes: include_bytes!("../../../assets/skins/steel-neon-v3/sound/select.wav") },
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

const BUNDLE_GENERATIONS: &[BundleGeneration] = &[
    BundleGeneration { directory: STEEL_NEON_V1_DIRECTORY, files: STEEL_NEON_V1_FILES, documents: STEEL_NEON_DOCUMENTS },
    BundleGeneration { directory: STEEL_NEON_V2_DIRECTORY, files: STEEL_NEON_V2_FILES, documents: STEEL_NEON_DOCUMENTS },
    BundleGeneration { directory: STEEL_NEON_V3_DIRECTORY, files: STEEL_NEON_V3_FILES, documents: STEEL_NEON_DOCUMENTS },
];

/// The generation a fresh install is given, which is the last one listed.
fn current_bundle() -> &'static BundleGeneration {
    BUNDLE_GENERATIONS.last().expect("the bundled skin ships at least one generation")
}

/// Every generation the current one replaced, newest first, which is where a selection may still be
/// pointing.
fn superseded_bundles() -> impl Iterator<Item = &'static BundleGeneration> {
    BUNDLE_GENERATIONS.iter().rev().skip(1)
}

/// One of the bundled skins by name ("WIDE" or NORMAL). Used when no external `--skin` is given.
pub(crate) fn bundled_skin(name: &str) -> SkinConfig {
    let s = if name.eq_ignore_ascii_case("WIDE") { SKIN_WIDE } else { SKIN_NORMAL };
    ron::from_str(s).unwrap_or_default()
}

pub(crate) fn installed_play_skin_path(settings_path: &Path, config: &Config, mode: Mode) -> PathBuf {
    let filename = if matches!(mode, Mode::BEAT_10K | Mode::BEAT_14K) { "play-dual.ron" } else { "play.ron" };
    active_bundle_directory(&crate::skin_select::skin_root(settings_path, config)).join(filename)
}

fn active_theme_path(settings_path: &Path, config: &Config) -> PathBuf {
    if config.display.skin.eq_ignore_ascii_case(STEEL_NEON_SKIN) {
        return active_bundle_directory(&crate::skin_select::skin_root(settings_path, config)).join("theme.ron");
    }
    settings_path.parent().map(|directory| directory.join("theme.ron")).unwrap_or_else(|| PathBuf::from("theme.ron"))
}

pub(crate) fn needs_default_skin_install(settings_path: &Path, config: &Config) -> bool {
    if !config.skin.default_skin_installed {
        return true;
    }
    let current = current_bundle();
    !current.files.is_empty() && !bundle_is_complete(current, &crate::skin_select::skin_root(settings_path, config).join(current.directory))
}

pub(crate) fn install_default_skin(settings_path: &Path, config: &mut Config) -> bool {
    let root = crate::skin_select::skin_root(settings_path, config);
    let current = current_bundle();
    let current_directory = root.join(current.directory);
    let superseded_ready = superseded_bundles().fold(false, |ready, generation| {
        let directory = root.join(generation.directory);
        (directory.is_dir() && install_bundle(&directory, generation.files)) || ready
    });
    let current_ready = install_bundle(&current_directory, current.files);
    let theme_path = settings_path.parent().unwrap_or(Path::new(".")).join("theme.ron");
    let theme_ready = install_bundled_file(&theme_path, THEME_TEMPLATE.as_bytes());

    if superseded_ready || current_ready {
        let selects_default_documents = config.display.skin.eq_ignore_ascii_case(DEFAULT_SKIN);
        let can_use_current_bundle = bundle_can_draw(current, &current_directory);
        let migrates_superseded_wide =
            can_use_current_bundle && config.display.skin.eq_ignore_ascii_case("WIDE") && has_unmodified_superseded_selection(&root, config);
        if (selects_default_documents || migrates_superseded_wide)
            && active_bundle_directory(&root).join("play.ron").is_file()
            && active_bundle_directory(&root).join("play-dual.ron").is_file()
        {
            config.display.skin = STEEL_NEON_SKIN.to_string();
        }
        if selects_default_documents {
            select_default_bundle_documents(&root, config);
        }
        apply_skin_bundle_selection(settings_path, config);
    }
    (superseded_ready || current_ready) && theme_ready
}

pub(crate) fn apply_skin_bundle_selection(settings_path: &Path, config: &mut Config) {
    if !config.display.skin.eq_ignore_ascii_case(STEEL_NEON_SKIN) {
        return;
    }
    let root = crate::skin_select::skin_root(settings_path, config);
    let active = active_bundle(&root);
    let current_directory = root.join(active.directory);
    let mut moved_off: Vec<&'static str> = Vec::new();
    for (screen, file) in current_bundle().documents {
        let current_path = current_directory.join(file);
        let Some(selected) = config.skin.document(*screen).map(PathBuf::from) else {
            continue;
        };
        if selected == current_path || !current_path.is_file() {
            continue;
        }
        let shipped_as_selected = superseded_bundles()
            .find(|generation| selected == root.join(generation.directory).join(file) && matches_shipped_document(generation, &selected, file));
        if let Some(generation) = shipped_as_selected {
            move_document_selection(config, *screen, &selected, &current_path);
            if !moved_off.contains(&generation.directory) {
                moved_off.push(generation.directory);
            }
        }
    }
    for directory in moved_off {
        config.skin.move_shared(directory, active.directory);
    }
}

/// Move one screen onto the generation that replaced it, carrying the choices made in the document
/// it was pointing at so a customised screen looks the same after the move. A choice already stored
/// for the new path is the one kept. Choices made for the whole bundle are keyed by its directory
/// rather than by a document path, so they are carried once per generation by the caller.
fn move_document_selection(config: &mut Config, screen: i32, from: &Path, to: &Path) {
    let from = from.to_string_lossy().into_owned();
    let to = to.to_string_lossy().into_owned();
    if let Some(choices) = config.skin.custom.remove(&from) {
        config.skin.custom.entry(to.clone()).or_insert(choices);
    }
    config.skin.select(screen, Some(to));
}

pub(crate) fn skin_document_is_enabled(settings_path: &Path, config: &Config, screen: i32) -> bool {
    let Some(selected) = config.skin.document(screen) else {
        return false;
    };
    let Some((_, filename)) = current_bundle().documents.iter().find(|(document_screen, _)| *document_screen == screen) else {
        return true;
    };
    let root = crate::skin_select::skin_root(settings_path, config);
    let selected_path = Path::new(selected);
    if !BUNDLE_GENERATIONS.iter().any(|generation| selected_path == root.join(generation.directory).join(filename)) {
        return true;
    }
    config.display.skin.eq_ignore_ascii_case(STEEL_NEON_SKIN) && selected_path == active_bundle_directory(&root).join(filename)
}

/// The sound set the active bundle ships, for a player who configured no folder of their own.
pub(crate) fn bundled_sound_folder(settings_path: &Path, config: &Config) -> Option<PathBuf> {
    if !config.display.skin.eq_ignore_ascii_case(STEEL_NEON_SKIN) {
        return None;
    }
    let directory = active_bundle_directory(&crate::skin_select::skin_root(settings_path, config)).join(BUNDLE_SOUND_DIRECTORY);
    directory.is_dir().then_some(directory)
}

fn install_bundle(directory: &Path, files: &[BundledFile]) -> bool {
    let mut ready = !files.is_empty();
    for file in files {
        ready = install_bundled_file(&directory.join(file.path), file.bytes) && ready;
    }
    ready
}

/// The generation being drawn: the newest one that can draw, or the current one when none of them
/// can, which is the directory an install writes into.
fn active_bundle(root: &Path) -> &'static BundleGeneration {
    BUNDLE_GENERATIONS.iter().rev().find(|generation| bundle_can_draw(generation, &root.join(generation.directory))).unwrap_or_else(current_bundle)
}

fn active_bundle_directory(root: &Path) -> PathBuf {
    root.join(active_bundle(root).directory)
}

/// Whether a generation has everything a screen is drawn from: the theme, both note-field layouts
/// and every document.
///
/// This is deliberately narrower than [`bundle_is_complete`]. A bundle also ships the scripts that
/// generate its art, its sound set and images no document names yet, and losing one of those is a
/// reason to repair the install -- not a reason to fall back to a generation the player is not
/// using and, on a fresh machine, was never even written.
fn bundle_can_draw(generation: &BundleGeneration, directory: &Path) -> bool {
    ["theme.ron", "play.ron", "play-dual.ron"].into_iter().chain(generation.documents.iter().map(|(_, file)| *file)).all(|file| directory.join(file).is_file())
}

/// Whether every file a generation ships is on disk, which is what decides that an install has
/// nothing left to write.
fn bundle_is_complete(generation: &BundleGeneration, directory: &Path) -> bool {
    if generation.files.is_empty() {
        return bundle_can_draw(generation, directory);
    }
    generation.files.iter().all(|file| directory.join(file.path).is_file())
}

fn select_default_bundle_documents(root: &Path, config: &mut Config) {
    let generation = active_bundle(root);
    let directory = root.join(generation.directory);
    for (screen, file) in generation.documents {
        let path = directory.join(file);
        if config.skin.document(*screen).is_none() && path.is_file() {
            config.skin.select(*screen, Some(path.to_string_lossy().into_owned()));
        }
    }
}

/// Whether the file on disk is byte for byte the one that generation shipped, which is how an
/// untouched document is told apart from one the player edited.
fn matches_shipped_document(generation: &BundleGeneration, path: &Path, filename: &str) -> bool {
    let Some(file) = generation.files.iter().find(|file| file.path == filename) else {
        return false;
    };
    let Ok(actual) = std::fs::read(path) else {
        return false;
    };
    Sha256::digest(actual) == Sha256::digest(file.bytes)
}

fn has_unmodified_superseded_selection(root: &Path, config: &Config) -> bool {
    superseded_bundles().any(|generation| {
        let selections: Vec<(&str, PathBuf)> =
            generation.documents.iter().filter_map(|(screen, file)| config.skin.document(*screen).map(|path| (*file, PathBuf::from(path)))).collect();
        !selections.is_empty()
            && selections.iter().all(|(file, path)| path == &root.join(generation.directory).join(file) && matches_shipped_document(generation, path, file))
    })
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

    /// Lay a generation out on disk exactly as it shipped, which is what a selection into it has to
    /// look like for the move onto the current bundle to take it.
    fn write_bundle(directory: &Path, generation: &BundleGeneration) {
        for file in generation.files {
            let path = directory.join(file.path);
            std::fs::create_dir_all(path.parent().expect("a bundled file has a parent")).expect("create the bundle folder");
            std::fs::write(path, file.bytes).expect("write the bundled file");
        }
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
        for generation in superseded_bundles() {
            assert!(!root.join(generation.directory).exists(), "a fresh installation also created the superseded {} bundle", generation.directory);
        }

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
            let dual_field_left = dual_skin.x.iter().copied().fold(f32::MAX, f32::min);
            let dual_field_right = dual_skin.x.iter().zip(&dual_skin.w).map(|(x, width)| x + width).fold(f32::MIN, f32::max);
            assert_eq!(dual_skin.fields.len(), 2, "{mode:?} is not a two-field layout");
            let bga = dual_skin.bga.expect("dual-field BGA");
            assert!(dual_field_right < bga.x || dual_field_left > bga.x + bga.w, "{mode:?} reaches the BGA");
            assert!(dual_field_right - dual_field_left > 400.0, "{mode:?} fields collapsed beside the BGA");
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
    fn bundle_selection_preserves_documents_and_gates_the_legacy_bundle_for_wide() {
        let dir = temp_dir("bundle-selection-wide");
        let settings = dir.join("settings.ron");
        let root = dir.join("skin");
        let external = root.join("external.json5");
        std::fs::create_dir_all(&root).expect("create the skin folder");
        std::fs::write(&external, "external").expect("write the external document");

        let mut config = Config::default();
        assert!(install_default_skin(&settings, &mut config));
        let legacy = config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT).expect("the bundled select document is chosen").to_string();
        config.display.skin = "WIDE".to_string();
        apply_skin_bundle_selection(&settings, &mut config);
        assert_eq!(
            config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
            Some(legacy.as_str()),
            "the legacy document selection was erased with the WIDE preset"
        );

        config.skin.select(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT, Some(external.to_string_lossy().into_owned()));
        apply_skin_bundle_selection(&settings, &mut config);
        assert_eq!(
            config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
            Some(external.to_string_lossy().as_ref()),
            "the external document was cleared with the bundled one"
        );

        config.skin.select(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT, Some(legacy.clone()));
        assert!(
            !skin_document_is_enabled(&settings, &config, rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
            "the legacy overlay is enabled while the WIDE preset owns the layout"
        );
        config.display.skin = STEEL_NEON_SKIN.to_string();
        apply_skin_bundle_selection(&settings, &mut config);
        assert_eq!(
            config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
            Some(legacy.as_str()),
            "the document selection changed while returning to the bundled preset"
        );
        assert!(
            skin_document_is_enabled(&settings, &config, rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
            "the legacy overlay stays gated after its matching preset returns"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every generation the current bundle replaced is one a player may still be pointing at, so
    /// each of them has to move onto the current bundle -- and only while the document sitting there
    /// is still the one that generation shipped.
    #[test]
    fn unchanged_superseded_documents_move_to_a_complete_current_bundle() {
        for generation in superseded_bundles() {
            let dir = temp_dir(&format!("bundle-migration-{}", generation.directory));
            let settings = dir.join("settings.ron");
            let mut config = Config::default();
            let root = crate::skin_select::skin_root(&settings, &config);
            let superseded_directory = root.join(generation.directory);
            write_bundle(&superseded_directory, generation);
            let superseded = superseded_directory.join("select.json5");
            config.skin.select(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT, Some(superseded.to_string_lossy().into_owned()));
            config.skin.default_skin_installed = true;
            config.display.skin = "WIDE".to_string();
            assert!(install_default_skin(&settings, &mut config));

            let directory = generation.directory;
            let migrated = root.join(STEEL_NEON_V3_DIRECTORY).join("select.json5");
            assert_eq!(config.display.skin, STEEL_NEON_SKIN, "the unmodified {directory} bundle did not move off the incompatible WIDE preset");
            assert_eq!(
                config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
                Some(migrated.to_string_lossy().as_ref()),
                "the unchanged {directory} document was not migrated"
            );
            assert!(skin_document_is_enabled(&settings, &config, rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT), "the complete current bundle is gated off");

            std::fs::write(&superseded, "edited").expect("edit the superseded document");
            config.skin.select(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT, Some(superseded.to_string_lossy().into_owned()));
            apply_skin_bundle_selection(&settings, &mut config);
            assert_eq!(
                config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
                Some(superseded.to_string_lossy().as_ref()),
                "the edited {directory} document was migrated over the user's change"
            );
            assert!(
                !skin_document_is_enabled(&settings, &config, rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
                "the edited {directory} document stays enabled beside the current bundle"
            );
            assert_eq!(std::fs::read_to_string(&superseded).expect("read the edited superseded document"), "edited");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Moving a screen onto the generation that replaced it has to carry what the player chose
    /// inside it: the choices are stored under the document's path, so a move that leaves them
    /// behind is a screen that silently reverts to how its author shipped it.
    #[test]
    fn a_migrated_document_takes_its_customisation_to_the_new_generation() {
        for generation in superseded_bundles() {
            let dir = temp_dir(&format!("bundle-custom-migration-{}", generation.directory));
            let settings = dir.join("settings.ron");
            let mut config = Config::default();
            let root = crate::skin_select::skin_root(&settings, &config);
            let superseded_directory = root.join(generation.directory);
            write_bundle(&superseded_directory, generation);
            let superseded = superseded_directory.join("select.json5").to_string_lossy().into_owned();
            let kept = superseded_directory.join("result.json5").to_string_lossy().into_owned();
            config.skin.select(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT, Some(superseded.clone()));
            config.skin.select(rbms_skin::loader::SKIN_TYPE_RESULT, Some(kept.clone()));
            config.skin.customise(&superseded).properties.insert("LANE COVER".into(), 902);
            config.skin.customise(&kept).properties.insert("BACKGROUND".into(), 911);
            config.skin.shared_customise(generation.directory).properties.insert("PLAY SIDE".into(), 901);
            config.skin.default_skin_installed = true;
            config.display.skin = STEEL_NEON_SKIN.to_string();
            assert!(install_default_skin(&settings, &mut config));

            let directory = generation.directory;
            let current_directory = root.join(STEEL_NEON_V3_DIRECTORY);
            let migrated = current_directory.join("select.json5").to_string_lossy().into_owned();
            assert_eq!(
                config.skin.document(rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT),
                Some(migrated.as_str()),
                "the unchanged {directory} document was not migrated"
            );
            assert_eq!(config.skin.user_config(&migrated).properties.get("LANE COVER"), Some(&902), "the migrated document lost what was chosen in it");
            assert_eq!(config.skin.customisation(&superseded), None, "the choices were left behind under the old path as well");

            let migrated_result = current_directory.join("result.json5").to_string_lossy().into_owned();
            assert_eq!(config.skin.user_config(&migrated_result).properties.get("BACKGROUND"), Some(&911), "a second screen's choices did not follow it");
            assert_eq!(
                config.skin.user_config(&migrated).properties.get("PLAY SIDE"),
                Some(&901),
                "what was chosen for the whole {directory} bundle was left behind"
            );
            assert_eq!(config.skin.shared_customisation(directory), None, "the bundle's choices were kept under the old directory as well");

            let already_chosen_in = current_directory.join("decide.json5").to_string_lossy().into_owned();
            let superseded_decide = superseded_directory.join("decide.json5").to_string_lossy().into_owned();
            config.skin.select(rbms_skin::loader::SKIN_TYPE_DECIDE, Some(superseded_decide.clone()));
            config.skin.customise(&superseded_decide).properties.insert("DECIDE EFFECT".into(), 991);
            config.skin.customise(&already_chosen_in).properties.insert("DECIDE EFFECT".into(), 990);
            apply_skin_bundle_selection(&settings, &mut config);
            assert_eq!(config.skin.document(rbms_skin::loader::SKIN_TYPE_DECIDE), Some(already_chosen_in.as_str()));
            assert_eq!(
                config.skin.user_config(&already_chosen_in).properties.get("DECIDE EFFECT"),
                Some(&990),
                "choices already made for the new path were overwritten by the old ones"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Every document the current bundle ships has to load, and the result document has to keep
    /// declaring the native content it stands in for along with an object for each id that
    /// replacement names -- a document that drops either draws its own frame over native output it
    /// no longer covers.
    #[test]
    fn the_shipped_documents_load_and_the_result_screen_still_replaces_what_it_draws() {
        let dir = temp_dir("bundle-documents");
        let settings = dir.join("settings.ron");
        let mut config = Config::default();
        assert!(install_default_skin(&settings, &mut config));
        let root = crate::skin_select::skin_root(&settings, &config);
        let directory = active_bundle_directory(&root);

        for (screen, file) in current_bundle().documents {
            let path = directory.join(file);
            let user = config.skin.user_config(&path.to_string_lossy());
            let mode = rbms_skin::loader::skin_type_mode(*screen).unwrap_or(Mode::BEAT_7K);
            let loaded = rbms_skin::loader::load_skin(&path, rbms_skin::loader::SkinLoadOptions::new(&directory, &user, mode))
                .unwrap_or_else(|error| panic!("{file} did not load: {error}"));
            assert!(loaded.warnings.is_empty(), "{file} loaded with warnings: {:?}", loaded.warnings);
            if *screen != rbms_skin::loader::SKIN_TYPE_RESULT {
                continue;
            }
            let declared: Vec<&str> = loaded.def.text.iter().map(|text| text.id.as_str()).collect();
            for (unit, ids) in RESULT_REPLACEMENT_CONTRACT {
                assert!(loaded.replace_names().contains(*unit), "{file} stopped replacing the native {unit}");
                for id in *ids {
                    assert!(declared.contains(id), "{file} replaces the native {unit} without declaring {id}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The objects each result replacement unit needs, mirroring what the screen checks before it
    /// stops drawing the native content.
    const RESULT_REPLACEMENT_CONTRACT: &[(&str, &[&str])] = &[
        ("score", &["result-score-label", "result-score", "result-combo-label", "result-combo", "result-notes-label", "result-notes"]),
        ("clear", &["result-clear"]),
        ("judgment", &["result-judge-perfect", "result-judge-great", "result-judge-good", "result-judge-bad", "result-judge-poor", "result-judge-miss"]),
        ("target", &["result-target"]),
    ];

    /// A bundle is drawn from its theme, its two note-field layouts and its documents. Everything
    /// else it ships is art and tooling, and losing one of those has to leave the player on the
    /// generation they installed -- an install writes no other directory, so falling off this one
    /// points every path at a folder that is not there.
    #[test]
    fn a_bundle_still_draws_once_it_loses_a_file_no_screen_is_built_from() {
        let dir = temp_dir("bundle-readiness");
        let settings = dir.join("settings.ron");
        let mut config = Config::default();
        assert!(install_default_skin(&settings, &mut config));
        let root = crate::skin_select::skin_root(&settings, &config);
        let current = current_bundle();
        let directory = root.join(current.directory);

        let spare = current
            .files
            .iter()
            .map(|file| file.path)
            .find(|path| !["theme.ron", "play.ron", "play-dual.ron"].contains(path) && !current.documents.iter().any(|(_, file)| file == path))
            .expect("the bundle ships a file no screen is built from");
        std::fs::remove_file(directory.join(spare)).expect("remove the spare file");
        assert_eq!(active_bundle(&root).directory, current.directory, "losing {spare} moved the player off the generation they installed");
        assert!(needs_default_skin_install(&settings, &config), "losing {spare} left the install with nothing to repair");

        let document = current.documents.first().map(|(_, file)| *file).expect("the bundle ships a document");
        std::fs::remove_file(directory.join(document)).expect("remove the document");
        assert!(!bundle_can_draw(current, &directory), "a generation missing a screen's document still claimed to draw it");
        assert_eq!(active_bundle(&root).directory, current.directory, "a bundle nothing can draw pointed somewhere an install never writes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A player who configured no sound folder is given the set the bundle ships, and only while the
    /// bundled preset is the one drawing -- a folder that is not there leaves the set silent rather
    /// than naming a directory nothing can be read from.
    #[test]
    fn the_active_bundle_answers_for_the_sound_folder_only_once_it_ships_one() {
        let dir = temp_dir("bundle-sound-folder");
        let settings = dir.join("settings.ron");
        let mut config = Config::default();
        assert!(install_default_skin(&settings, &mut config));
        assert_eq!(config.display.skin, STEEL_NEON_SKIN);

        let sounds = active_bundle_directory(&crate::skin_select::skin_root(&settings, &config)).join(BUNDLE_SOUND_DIRECTORY);
        assert_eq!(bundled_sound_folder(&settings, &config), Some(sounds.clone()), "the installed bundle did not name the set it ships");

        config.display.skin = "WIDE".to_string();
        assert_eq!(bundled_sound_folder(&settings, &config), None, "a preset that is not the bundled one read the bundle's sounds");

        config.display.skin = STEEL_NEON_SKIN.to_string();
        std::fs::remove_dir_all(&sounds).expect("remove the bundled sound folder");
        let root = crate::skin_select::skin_root(&settings, &config);
        assert_eq!(
            active_bundle(&root).directory,
            STEEL_NEON_V3_DIRECTORY,
            "losing the sound set moved the player onto another generation, so the answer below is about the wrong bundle"
        );
        assert_eq!(bundled_sound_folder(&settings, &config), None, "a bundle with no sound folder still named one");
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
        assert_eq!(
            active_theme_path(&settings, &config),
            PathBuf::from("/tmp/rbms/skin").join(STEEL_NEON_V3_DIRECTORY).join("theme.ron"),
            "nothing installed named a generation an install would never write"
        );
    }
}

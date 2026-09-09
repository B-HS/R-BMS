use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rbms_audio::AudioEngine;
use rbms_chart::shuffle::NoteOption;
use rbms_chart::default_total;
use rbms_chart::to_model;
use rbms_ir::{API_VERSION, ChartId, JudgeBreakdown, NullScoreServer, PlayOptions, PlayerId, ScoreServer, ScoreSubmission};
use rbms_judge::GaugeKind;
use rbms_model::Mode;
use rbms_play::{PlayEvent, Player};
use rbms_render::{
    Color, CoverState, DensityView, DetailView, HudView, RANK_BANDS, Rect, RecordRowView, RecordsView, Renderer, ResultView, SelectDetail, SelectHot, SelectModal, SelectRow,
    ResultPalette, SelectView as SelectScene, Skin, SkinConfig, StatCell, cover_rect, dj_rank, draw_text, draw_text_centered, draw_text_right, ex_delta_label, render_hud, render_lane_cover,
    render_playfield, render_key_bomb, render_result_with_palette, render_select, text_width,
};

use sha2::{Digest, Sha256};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

mod app_input;
mod app_play;
mod app_select;
mod folders;
mod format;
mod gpu;
mod ir_map;
mod keyconfig;
mod replay;
mod scores;
mod settings;
mod tables;
mod tablesrc;
use folders::FolderList;
use format::{
    clear_label_color, clear_type_from_id, clear_type_id, difficulty_color, difficulty_name, fmt_datetime, fmt_duration, gauge_name, mode_color, mode_short, rank_label,
};
use gpu::Gpu;
use ir_map::{assist_flags, combo_breaks, gauge_from_name, gauge_token, ir_clear, ir_gauge, ir_lntype, ir_random};
use keyconfig::{ControlAction, KeyConfig, key_from_name, key_name};
use replay::{Replay, ReplayEvent};
use scores::{SCORE_RULE_VERSION, ScoreBook, ScoreRecord, rule_version_mark, rule_version_note};
use settings::PlaySettings;
use tables::{TableList, TableSource};
use tablesrc::{fetch_and_match, load_and_match};

const CW: u32 = 1280;
const CH: u32 = 720;
const MODE: Mode = Mode::BEAT_7K;

/// `#PREVIEW` hover-preview tuning: the reserved sample id, focus-settle debounce (frames), playback
/// gain, and the silent tail held after the last autoplay event before the loop restarts.
const PREVIEW_ID: u32 = 0;
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(333);
const PREVIEW_GAIN: f32 = 0.85;
const PREVIEW_LOOP_TAIL_US: i64 = 2_000_000;

/// How long the select focus must rest on a row before its heavy detail (full chart parse + timing
/// integration + cover decode) is computed. Frame-count debouncing tied the delay to the frame rate;
/// this keeps it constant.
const FOCUS_DETAIL_DEBOUNCE: Duration = Duration::from_millis(150);

/// How long a second Esc press at the select root still counts as confirming "quit".
const ROOT_ESC_CONFIRM: Duration = Duration::from_secs(1);

/// Suffix of the sibling temp file [`write_atomic`] writes before renaming it over the target.
const TEMP_WRITE_SUFFIX: &str = ".tmp";

/// JUDGE WIDTH percentage that leaves the windows exactly as the chart defines them. Anything above
/// it widens the windows and counts as an assist (no score submission).
const JUDGE_RATE_UNMODIFIED: i32 = 100;

/// Background → main-thread messages for an autoplay preview (a chart with no `#PREVIEW` file): the
/// extracted keysound timeline, then each decoded keysound. The channel disconnecting signals the
/// load is complete, at which point [`App::update_preview`] anchors the clock and begins playback.
enum PreviewMsg {
    Schedule { sched: Vec<(i64, u32)>, start_us: i64, end_us: i64 },
    Keysound(u32, rbms_audio::DecodedAudio),
}

struct PlayerConfig {
    keys_override: Option<Vec<(KeyCode, usize)>>,
    scratch_left: bool,
    scratch_auto: bool,
    lift: f32,
    cover: f32,
    hispeed: f64,
    gauge: GaugeKind,
    random: NoteOption,
    constant_speed: bool,
    offset_ms: i32,
    auto_offset: bool,
    judge_rate: i32,
    total_override: f64,
    bga: bool,
    skin_name: String,
    skin_path: Option<String>,
    server_url: Option<String>,
    player_id: String,
    table_url: Option<String>,
    keyconfig_path: Option<String>,
    replay_path: Option<String>,
    auto_replay: bool,
    debug: bool,
    font_path: Option<String>,
    score_graph: bool,
    replay_analysis: bool,
    preview: bool,
    songs_folder: Option<String>,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        PlayerConfig {
            keys_override: None,
            scratch_left: false,
            scratch_auto: false,
            lift: 0.0,
            cover: 0.0,
            hispeed: 2.0,
            gauge: GaugeKind::Normal,
            random: NoteOption::Off,
            constant_speed: false,
            offset_ms: 0,
            auto_offset: false,
            judge_rate: 100,
            total_override: 0.0,
            bga: true,
            skin_name: "NORMAL".into(),
            skin_path: None,
            server_url: None,
            player_id: "guest".into(),
            table_url: None,
            keyconfig_path: None,
            replay_path: None,
            auto_replay: true,
            debug: false,
            font_path: None,
            score_graph: true,
            replay_analysis: true,
            preview: true,
            songs_folder: None,
        }
    }
}


/// Write `contents` to `path` atomically: create the parent directory, write a sibling temp file,
/// flush it, then rename it over the target. A crash or a full disk mid-write can then never leave a
/// *truncated* config/score/replay file behind — the target is either the old contents or the new
/// ones. The rename itself is not durable (the parent directory is not fsynced), so a crash right
/// after it may still lose the update. The temp name carries the process id so two instances saving
/// the same file do not clobber each other's temp. The temp file is removed if anything fails.
pub(crate) fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name"));
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_file_name(format!("{name}.{}{TEMP_WRITE_SUFFIX}", std::process::id()));
    let written = std::fs::File::create(&tmp).and_then(|mut f| {
        f.write_all(contents.as_bytes())?;
        f.sync_all()
    });
    if let Err(e) = written.and_then(|()| std::fs::rename(&tmp, path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

/// Judgment time for an input taken at raw song time `raw_us`: the user's judge offset shifts when
/// the hit is *judged*. Keysound playback must NOT use this (see [`keysound_time_us`]) — beatoraja
/// plays the sound at the input instant and only offsets the judgment.
fn judge_time_us(raw_us: i64, offset_ms: i32) -> i64 {
    raw_us + offset_ms as i64 * 1000
}

/// Audio-clock time at which an input's keysound is scheduled: the raw input instant rebased onto the
/// output-stream clock. Independent of the judge offset, so a non-zero offset never delays the sound.
fn keysound_time_us(raw_us: i64, anchor_us: i64) -> i64 {
    raw_us + anchor_us
}

/// Green number (note travel time in ms) for the active scroll mode: CONSTANT is fixed by hi-speed
/// alone, FLOATING tracks the BPM/SCROLL of the segment under the play head. Both shrink with the
/// lane cover, which hides the top of the visible window.
fn green_number_for(constant: bool, bpm: f64, hispeed: f64, scroll: f64, cover: f32) -> f64 {
    if constant {
        rbms_chart::scroll::constant_green_number(hispeed, cover as f64)
    } else {
        rbms_chart::scroll::green_number(bpm, hispeed, scroll, cover as f64)
    }
}

/// Why this run's score must not be sent to the IR, or `None` when it may be submitted. Mirrors
/// beatoraja: only a real interactive PLAY reaches the IR (`MusicResult.java:82`), and any assist —
/// a judge window widened past 100% (`BMSPlayer.java:207-213`) or an auto-played lane
/// (`AutoplayModifier` sets `AssistLevel.ASSIST`, `BMSPlayer.java:233-234`) — clears the score flag.
fn ir_submission_block_reason(autoplay: bool, replay: bool, judge_rate: i32, scratch_auto: bool) -> Option<&'static str> {
    if autoplay {
        return Some("autoplay");
    }
    if replay {
        return Some("replay playback");
    }
    if judge_rate > JUDGE_RATE_UNMODIFIED {
        return Some("judge window widened");
    }
    if scratch_auto {
        return Some("scratch assist");
    }
    None
}

/// Whether this run may update the stored bests (EX / lamp / BP), i.e. it was an unassisted
/// interactive play. Same predicate as [`ir_submission_block_reason`], so the IR gate and the local
/// score book never disagree — beatoraja derives both from the one `score` flag
/// (`BMSPlayer.java:207-213` clears it, `:363` `resource.setUpdateScore(score)`,
/// `MusicResult.java:444-446` passes it to `PlayDataAccessor.writeScoreData`, and
/// `ScoreData.java:548,566,572,578` gate exscore/avgjudge/minbp/combo on it).
fn updates_score(autoplay: bool, replay: bool, judge_rate: i32, scratch_auto: bool) -> bool {
    ir_submission_block_reason(autoplay, replay, judge_rate, scratch_auto).is_none()
}

/// Whether an Esc at the select root confirms quitting: only when the previous Esc (`first`) landed
/// within [`ROOT_ESC_CONFIRM`]. A lone Esc arms the confirmation instead of exiting, so a stray press
/// can't drop the user out of the app.
fn esc_confirms_quit(first: Option<Instant>, now: Instant) -> bool {
    first.is_some_and(|t| now.duration_since(t) <= ROOT_ESC_CONFIRM)
}

/// Song time once the audio output stream has died: continue from `last_us` (the last position the
/// audio clock reported) plus the wall time since, so the clock neither freezes nor jumps.
fn resumed_clock_us(last_us: i64, elapsed_us: i64) -> i64 {
    last_us + elapsed_us
}

fn apply_settings(cfg: &mut PlayerConfig, s: &PlaySettings) {
    cfg.hispeed = s.hispeed.clamp(0.5, 10.0);
    cfg.gauge = gauge_from_name(&s.gauge);
    cfg.lift = s.lift.clamp(0.0, 0.9);
    cfg.cover = s.cover.clamp(0.0, 0.9);
    cfg.scratch_left = s.scratch_left;
    cfg.scratch_auto = s.scratch_auto;
    cfg.random = NoteOption::from_str(&s.random);
    cfg.constant_speed = s.constant_speed;
    cfg.offset_ms = s.offset_ms.clamp(-200, 200);
    cfg.auto_offset = s.auto_offset;
    cfg.judge_rate = s.judge_rate.clamp(50, 200);
    cfg.total_override = s.total_override.max(0.0);
    cfg.bga = s.bga;
    cfg.auto_replay = s.auto_replay;
    cfg.debug = s.debug;
    cfg.font_path = s.font_path.clone();
    cfg.score_graph = s.score_graph;
    cfg.replay_analysis = s.replay_analysis;
    cfg.preview = s.preview;
    cfg.songs_folder = s.songs_folder.clone();
    cfg.server_url = s.server_url.clone();
    cfg.player_id = s.player_id.clone();
    if !s.skin.trim().is_empty() {
        cfg.skin_name = s.skin.to_ascii_uppercase();
    }
}

/// Build the score server from config (`HttpScoreServer` if a URL is set, else offline
/// `NullScoreServer`) plus a connection flag a background thread keeps fresh via `health()`.
/// Used at startup and when the NETWORK settings tab changes the server URL.
fn build_server(config: &PlayerConfig) -> (Arc<dyn ScoreServer>, Arc<AtomicBool>) {
    let server: Arc<dyn ScoreServer> = match &config.server_url {
        Some(url) => match rbms_ir::HttpScoreServer::try_new(url.clone(), None) {
            Ok(s) => {
                println!("score server: {url}");
                Arc::new(s)
            }
            Err(e) => {
                eprintln!("score server unavailable ({e}) — playing offline");
                Arc::new(NullScoreServer)
            }
        },
        None => Arc::new(NullScoreServer),
    };
    let connected = Arc::new(AtomicBool::new(false));
    if config.server_url.is_some() {
        let server = server.clone();
        let connected = connected.clone();
        std::thread::spawn(move || {
            loop {
                connected.store(server.health().is_ok(), Ordering::Relaxed);
                std::thread::sleep(Duration::from_secs(5));
            }
        });
    }
    (server, connected)
}

/// SHA-256 of the running client binary, submitted as `client_build_sha256` for build-integrity /
/// ranked eligibility. Computed once at startup. `None` if the executable can't be read — the
/// server decides how to treat an unknown build (e.g. ranked=false).
fn compute_build_hash() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let bytes = std::fs::read(exe).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

/// Client platform tag (`OS-ARCH`, e.g. `macos-aarch64`) paired with the build hash.
fn client_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Auto-calibration: the new judge offset (ms) given the run's mean timing error. The offset
/// is held constant during a run (so judging is consistent and replays reproduce); the mean
/// error then recentres timing for the NEXT run, converging in ~one play. `mean_us` > 0 = the
/// player was early (FAST), so the offset increases to compensate.
fn calibrated_offset(current_ms: i32, mean_us: i64) -> i32 {
    (current_ms + (mean_us as f64 / 1000.0).round() as i32).clamp(-200, 200)
}

const SKIN_NORMAL: &str = include_str!("../../../assets/skins/normal.ron");
const SKIN_WIDE: &str = include_str!("../../../assets/skins/wide.ron");

/// One of the bundled skins by name ("WIDE" or NORMAL). Used when no external `--skin` is given.
fn bundled_skin(name: &str) -> SkinConfig {
    let s = if name.eq_ignore_ascii_case("WIDE") { SKIN_WIDE } else { SKIN_NORMAL };
    ron::from_str(s).unwrap_or_default()
}

/// A fully-populated, commented UI-theme template written to `~/.config/rbms/theme.ron` on first run
/// so the palette is discoverable and editable. Each colour is `Some((r,g,b))`; omit a line to keep
/// its default. The in-play note field is themed separately by the skin. See `docs/theme.md`.
const THEME_TEMPLATE: &str = "\
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
fn load_theme(settings_path: &Path) {
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


fn resolve_file(dir: &Path, name: &str, exts: &[&str]) -> Option<(PathBuf, String)> {
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

fn resolve_keysound(dir: &Path, name: &str) -> Option<(PathBuf, String)> {
    resolve_file(dir, name, &["ogg", "wav", "flac", "mp3"])
}

/// Resolve every referenced keysound in a chart's `wavmap` to `(id, path, ext)` decode jobs (empty
/// names skipped, unresolvable files dropped). Shared by the Play loader and the autoplay preview.
fn keysound_jobs(wavmap: &[String], dir: &Path) -> Vec<(u32, PathBuf, String)> {
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
fn spawn_keysound_decode(
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
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(dec) = rbms_audio::decode_bytes(data, Some(ext.as_str())) {
                        let _ = tx.send((id, dec));
                    }
                }
                progress.fetch_add(1, Ordering::Relaxed);
            }
        });
    }
    (rx, progress, total)
}

fn decode_bga_256(dir: &Path, name: &str) -> Option<Vec<u8>> {
    let (path, _) = resolve_file(dir, name, &["png", "bmp", "jpg", "jpeg"])?;
    let bytes = std::fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(img.resize_exact(gpu::BGA_DIM, gpu::BGA_DIM, image::imageops::FilterType::Triangle).to_rgba8().into_raw())
}

struct SongEntry {
    path: PathBuf,
    title: String,
    subtitle: String,
    artist: String,
    genre: String,
    maker: String,
    level: String,
    difficulty: i32,
    init_bpm: f64,
    rank: i32,
    total: f64,
    mode: Mode,
    md5: String,
    stagefile: String,
    banner: String,
    /// `#PREVIEW` audio path, played on a settled focus by `start_preview` (select-only audio engine).
    preview: String,
}

/// Per-chart details that need full timing integration (`to_model`), computed lazily for the focused
/// song only (not every chart in the library) and cached in `App::focused_detail`.
struct ChartDetail {
    notes: usize,
    long_notes: usize,
    duration_us: i64,
    bpm_min: f64,
    bpm_max: f64,
    density: Vec<u32>,
    peak_density: f64,
    avg_density: f64,
    end_density: f64,
}

fn is_chart(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("bms" | "bme" | "bml" | "pms"))
}

/// Scan every configured library folder and merge the results into one song list (the library is
/// the union of all folders). Missing/unreadable folders contribute nothing.
fn scan_folders(folders: &[String], count: &std::sync::atomic::AtomicUsize) -> Vec<SongEntry> {
    let mut out = Vec::new();
    for f in folders {
        out.extend(scan_folder(Path::new(f), count));
    }
    out
}

fn scan_folder(root: &Path, count: &std::sync::atomic::AtomicUsize) -> Vec<SongEntry> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_chart(&p) {
                if let Ok(bytes) = std::fs::read(&p) {
                    let src = rbms_parser::parse(&bytes);
                    let mode = rbms_chart::detect_mode(&src, p.to_str().unwrap_or(""));
                    let h = &src.headers;
                    let title = if h.title.is_empty() { p.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string() } else { h.title.clone() };
                    out.push(SongEntry {
                        path: p,
                        title,
                        subtitle: h.subtitle.clone(),
                        artist: h.artist.clone(),
                        genre: h.genre.clone(),
                        maker: h.maker.clone(),
                        level: h.play_level.clone(),
                        difficulty: h.difficulty,
                        init_bpm: h.init_bpm,
                        rank: h.rank,
                        total: h.total.unwrap_or(0.0),
                        mode,
                        md5: src.md5.clone(),
                        stagefile: h.stagefile.clone(),
                        banner: h.banner.clone(),
                        preview: h.preview.clone(),
                    });
                    count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

/// Parse + time-integrate a single chart to derive its playable-note count, long-note count, length
/// and BPM range for the select detail panel. `None` if the file can't be read.
fn compute_chart_detail(path: &Path, mode: Mode) -> Option<ChartDetail> {
    let bytes = std::fs::read(path).ok()?;
    let src = rbms_parser::parse(&bytes);
    let total_value = src.headers.total.unwrap_or(0.0);
    let model = to_model(&src, mode);
    let long_notes = model
        .timelines
        .iter()
        .flat_map(|tl| tl.notes.iter().flatten())
        .filter(|n| matches!(n.kind, rbms_model::NoteKind::LongStart { .. }))
        .count();
    let duration_us = model.timelines.last().map(|t| t.time_us).unwrap_or(0);
    let mut bpm_min = f64::MAX;
    let mut bpm_max = f64::MIN;
    for tl in &model.timelines {
        if tl.bpm > 0.0 {
            bpm_min = bpm_min.min(tl.bpm);
            bpm_max = bpm_max.max(tl.bpm);
        }
    }
    if bpm_min > bpm_max {
        bpm_min = model.init_bpm;
        bpm_max = model.init_bpm;
    }
    let dens = rbms_chart::note_density(&model, total_value);
    Some(ChartDetail { notes: rbms_chart::count_playable_notes(&model), long_notes, duration_us, bpm_min, bpm_max, density: dens.bins, peak_density: dens.peak, avg_density: dens.avg, end_density: dens.end })
}

#[derive(PartialEq, Debug)]
enum Stage {
    Select,
    Settings,
    KeyConfig,
    Tables,
    Folders,
    Loading,
    Play,
    Result,
}

/// What a `Stage::Loading` frame is waiting to do once the LOADING screen has been shown for one
/// frame. `Song` loads a chart and enters Play; `Folder` (re)scans a newly-picked song folder and
/// re-matches the difficulty tables before returning to Select. Both block, so the LOADING frame
/// is presented first (folder descent within the already-scanned library is instant and skips this).
enum Loading {
    /// Load a chart and enter Play.
    Song(usize),
    /// Rescan every library folder (after the folder list changed) and re-match tables, then Select.
    Scan,
}

/// Result of a background folder scan (run off-thread so the LOADING screen keeps animating instead
/// of freezing): the rescanned library and the per-table matched levels.
struct ScanOutcome {
    songs: Vec<SongEntry>,
    names: Vec<String>,
    levels: Vec<Vec<(String, Vec<usize>)>>,
}

/// One editable row in the key-config screen.
enum KcRow {
    ModeSelect,
    Control(ControlAction),
    Lane(usize),
}

fn kc_rows(edit_mode: Mode) -> Vec<KcRow> {
    let mut rows = vec![KcRow::ModeSelect];
    rows.extend(ControlAction::ALL.into_iter().map(KcRow::Control));
    rows.extend((0..edit_mode.key).map(KcRow::Lane));
    rows
}

/// Where the song-select browser currently is. Navigation is Root → (ALL SONGS | each table →
/// per-level folder) → charts, modelling beatoraja's table-as-custom-folder browsing. The
/// indices select a table and a level within `App::table_levels`.
#[derive(Clone, Copy, PartialEq)]
enum SelectView {
    Root,
    AllSongs,
    TableLevels(usize),
    TableLevel(usize, usize),
}

/// One row in the select list: either a folder to descend into, or a playable chart (index
/// into `App::songs`).
enum SelectItem {
    Song(usize),
    Folder { label: String, target: SelectView },
}

/// Song-list ordering, cycled with F3 in the select screen.
#[derive(Clone, Copy, PartialEq)]
enum SortMode {
    Default,
    Title,
    Artist,
    Level,
    Clear,
}

impl SortMode {
    const ALL: [SortMode; 5] = [SortMode::Default, SortMode::Title, SortMode::Artist, SortMode::Level, SortMode::Clear];
    fn label(self) -> &'static str {
        match self {
            SortMode::Default => "DEFAULT",
            SortMode::Title => "TITLE",
            SortMode::Artist => "ARTIST",
            SortMode::Level => "LEVEL",
            SortMode::Clear => "CLEAR",
        }
    }
    fn next(self) -> SortMode {
        let i = Self::ALL.iter().position(|&m| m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// Cache key for the assembled [`SelectScene`]: rebuild only when one of these changes, so the scene
/// is not re-allocated every frame of the continuous redraw loop. `select_gen` bumps on any list
/// rebuild (catches same-length folder swaps); `scores` length catches a freshly saved record.
type SelectKey = (u64, usize, Option<usize>, usize, bool, bool);

/// A clickable region recorded during rendering and hit-tested on a left-click. Immediate-mode:
/// `App::hot` is rebuilt every frame for the current stage, so the layout math lives in one place.
#[derive(Clone, Copy)]
enum Hot {
    SelectRow(usize),
    RecordRow(usize),
    SettingTab(usize),
    SettingRow(usize),
    ModalClose,
    ModalReplay,
    NavSearch,
    NavSort,
    NavFolders,
    NavTables,
    NavRecords,
    NavSettings,
}

const GAUGE_CYCLE: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];
const SETTING_KEYCONFIG: usize = 11;
const SETTING_FONT: usize = 18;
const SETTING_SERVER_URL: usize = 22;
const SETTING_PLAYER_ID: usize = 23;

/// Settings grouped into tabs by category. Each entry is `(tab name, [global setting indices])`
/// where the indices map to `setting_line`/`adjust_setting`.
const SETTING_TABS: &[(&str, &[usize])] = &[
    ("PLAY", &[0, 1, 2, 3, 16]),
    ("GAUGE", &[4, 13]),
    ("JUDGE", &[9, 12, 15]),
    ("DISPLAY", &[14, 18, 19, 20, 21, 5, 6, 10, 17]),
    ("INPUT", &[7, 8, 11]),
    ("NETWORK", &[22, 23]),
];


struct App {
    chart_path: String,
    autoplay: bool,
    config: PlayerConfig,
    mode: Mode,
    stage: Stage,
    songs: Vec<SongEntry>,
    active_keys: Vec<(KeyCode, usize)>,
    table_sources: Vec<TableSource>,
    table_names: Vec<String>,
    table_levels: Vec<Vec<(String, Vec<usize>)>>,
    tables_path: PathBuf,
    tables_sel: usize,
    /// Song-library folders (scanned + merged), managed in the Folders screen and persisted to
    /// `folders.ron`. The library is the union of all of these.
    folders: Vec<String>,
    folders_path: PathBuf,
    folders_sel: usize,
    text_input: Option<String>,
    select_view: SelectView,
    select_items: Vec<SelectItem>,
    /// Incremental song search: `searching` opens the box (`/`), `search` is the live query (filters
    /// the list by title/artist/subtitle). `sort` orders the filtered list (F3 cycles).
    search: String,
    searching: bool,
    sort: SortMode,
    sel: usize,
    set_tab: usize,
    set_sel: usize,
    pending: Option<Loading>,
    /// When set, a background folder scan is running; the LOADING screen polls it each frame.
    scan_rx: Option<std::sync::mpsc::Receiver<ScanOutcome>>,
    /// Live count of charts found by the in-flight scan, shown on the SCANNING screen.
    scan_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    loading_drawn: bool,
    /// When set, keysounds are decoding on background threads: `frame()` drains decoded samples into
    /// the audio bank and draws a progress bar, then `start_play()` once `ks_progress == ks_total`.
    ks_rx: Option<std::sync::mpsc::Receiver<(u32, rbms_audio::DecodedAudio)>>,
    ks_progress: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// Cooperative cancel for the in-flight keysound decode pool, flipped when the LOADING screen is
    /// abandoned so the workers stop instead of decoding the whole chart into a discarded bank.
    ks_cancel: Arc<AtomicBool>,
    ks_total: usize,
    keyconfig: KeyConfig,
    keyconfig_path: PathBuf,
    settings_path: PathBuf,
    seed: u64,
    /// IR `lntype` of the loaded chart (0=LN, 1=CN, 2=HCN), derived from `#LNMODE` at load time so
    /// score submissions report the actual LN mode instead of a hardcoded value.
    chart_lntype: i32,
    recording: Vec<ReplayEvent>,
    replay: Option<Replay>,
    replay_cursor: usize,
    cal_sum_us: i64,
    cal_count: u32,
    kc_edit_mode: Mode,
    kc_sel: usize,
    kc_capturing: bool,
    kc_warn: bool,
    result: Option<ResultView>,
    gpu: Option<Gpu>,
    audio: Option<AudioEngine>,
    player: Option<Player>,
    skin: Skin,
    skin_cfg: SkinConfig,
    /// Result-screen judge colours/labels resolved from the active skin, rebuilt with it.
    result_palette: ResultPalette,
    bga_images: std::collections::HashMap<i32, Vec<u8>>,
    bga_events: Vec<(i64, i32)>,
    bga_cursor: usize,
    cur_bga: i32,
    server: Arc<dyn ScoreServer>,
    server_connected: Arc<AtomicBool>,
    clock: Instant,
    anchor_us: i64,
    /// Set once the audio output stream is found dead: the song position the audio clock last
    /// reported and the instant that was noticed, so `song_us` continues on the wall clock.
    audio_dead_at: std::cell::Cell<Option<(i64, Instant)>>,
    scores: ScoreBook,
    scores_path: PathBuf,
    /// When the records modal is open, the index into the focused chart's record list (newest
    /// first) currently shown in detail.
    record_modal: Option<usize>,
    /// Lazily computed detail (notes/LN/length/BPM range) for the currently focused song, with the
    /// song index it was computed for — recomputed only when the focus moves to a different song.
    focused_detail: Option<ChartDetail>,
    focused_detail_si: Option<usize>,
    /// Focus-settle debounce for the heavy detail: the row the focus is currently on and when it
    /// arrived there. The detail is computed only once it has rested for [`FOCUS_DETAIL_DEBOUNCE`].
    focus_settle_si: Option<usize>,
    focus_settle_at: Instant,
    cover_rgba: Option<Vec<u8>>,
    cached_select: Option<SelectScene>,
    cached_select_key: Option<SelectKey>,
    select_gen: u64,
    preview_audio: Option<AudioEngine>,
    preview_si: Option<usize>,
    preview_target: Option<usize>,
    preview_target_at: Instant,
    preview_loop_us: i64,
    preview_next_us: i64,
    /// Autoplay-preview keysound timeline `(at_us, wav)` for the focused chart when it defines no
    /// `#PREVIEW` file — extracted once from a throwaway autoplay `Player`, then replayed against the
    /// preview engine clock and looped. Empty in file-preview mode.
    preview_sched: Vec<(i64, u32)>,
    preview_cursor: usize,
    preview_anchor: i64,
    preview_start_us: i64,
    preview_end_us: i64,
    /// Background autoplay-preview load channel: the throwaway-`Player` schedule, then decoded
    /// keysounds; `None` once the load finishes (channel disconnected) and playback has begun.
    preview_prep_rx: Option<std::sync::mpsc::Receiver<PreviewMsg>>,
    /// Cooperative cancel for the in-flight autoplay-preview load (parse + decode workers), flipped
    /// when the focus moves on so abandoned work stops instead of running to completion.
    preview_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    cursor: (f32, f32),
    hot: Vec<(Rect, Hot)>,
    last_frame: Instant,
    fps: f32,
    frame_count: u64,
    ram_mb: f32,
    /// SHA-256 of this binary, computed once at startup, submitted for build integrity (F8).
    build_sha256: Option<String>,
    /// When the first Esc at the select root was pressed: a second press within [`ROOT_ESC_CONFIRM`]
    /// quits, anything else cancels. Guards against losing a session to a stray Esc.
    esc_quit_at: Option<Instant>,
    // --- replay analysis mode (F4) ---
    /// Active when a replay is playing and REPLAY ANALYSIS is on: shows the analysis overlay and
    /// enables playback controls.
    analysis: bool,
    /// Once the user takes manual control (pause/seek/rate), playback runs off the virtual
    /// `analysis_us` clock and keysounds are muted (so scrubbing/slow-mo doesn't desync audio);
    /// until then it follows the real (audio) clock at 1× with sound.
    analysis_manual: bool,
    analysis_paused: bool,
    analysis_rate: f64,
    analysis_us: i64,
    /// Recent judged inputs `(lane, delta_us, judge)` for the per-note ms-off overlay (newest last).
    msoff: Vec<(usize, i64, u8)>,
}

impl App {
    fn new(input: String, autoplay: bool, config: PlayerConfig, settings_path: PathBuf) -> Self {
        let replay = config.replay_path.as_ref().and_then(|p| match Replay::load(Path::new(p)) {
            Ok(r) => {
                println!("replay: {} on {}", p, r.chart_path);
                Some(r)
            }
            Err(e) => {
                eprintln!("replay load failed: {e}");
                None
            }
        });
        let autoplay = if replay.is_some() { false } else { autoplay };

        // Song-library folders (multi-folder). Load the list, migrating a legacy single remembered
        // folder into it on first run.
        let folders_path = settings_path.parent().map(|d| d.join("folders.ron")).unwrap_or_else(|| PathBuf::from("folders.ron"));
        let mut folders = FolderList::load(&folders_path).folders;
        if folders.is_empty() {
            if let Some(f) = config.songs_folder.clone().filter(|s| !s.trim().is_empty()) {
                folders.push(f);
            }
        }

        let p = Path::new(&input);
        let is_dir = replay.is_none() && p.is_dir();
        // A folder launch joins the library list so it persists alongside any others.
        if is_dir && !folders.iter().any(|f| f == &input) {
            folders.push(input.clone());
            FolderList { folders: folders.clone() }.save(&folders_path);
        }

        let tables_path = settings_path.parent().map(|d| d.join("tables.ron")).unwrap_or_else(|| PathBuf::from("tables.ron"));
        let scores_path = settings_path.parent().map(|d| d.join("scores.ron")).unwrap_or_else(|| PathBuf::from("scores.ron"));
        let scores = ScoreBook::load(&scores_path);
        let mut table_sources = TableList::load(&tables_path).tables;
        if let Some(url) = &config.table_url {
            if !table_sources.iter().any(|t| &t.location == url) {
                table_sources.push(TableSource { name: String::new(), location: url.clone() });
            }
        }

        // Library load: a chart/replay launch goes straight to Play; an empty library (first run / a
        // bare .dmg double-click) lands on the Select onboarding screen; otherwise the scan runs on a
        // BACKGROUND thread so the window appears immediately (a big library took seconds and froze
        // startup before any UI). `frame()` polls `scan_rx` and `apply_scan` swaps in the result.
        let scan_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let songs: Vec<SongEntry> = Vec::new();
        let table_names: Vec<String> = Vec::new();
        let table_levels: Vec<Vec<(String, Vec<usize>)>> = Vec::new();
        let (stage, chart_path, scan_rx, pending) = if let Some(rp) = &replay {
            (Stage::Play, rp.chart_path.clone(), None, None)
        } else if p.is_file() {
            (Stage::Play, input, None, None)
        } else if folders.is_empty() {
            (Stage::Select, String::new(), None, None)
        } else {
            let dirs = folders.clone();
            let sources = table_sources.clone();
            let count = scan_count.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let songs = scan_folders(&dirs, &count);
                let (names, levels) = fetch_and_match(&sources, &songs);
                let _ = tx.send(ScanOutcome { songs, names, levels });
            });
            (Stage::Loading, String::new(), Some(rx), Some(Loading::Scan))
        };

        let (server, server_connected) = build_server(&config);

        let keyconfig_path = config.keyconfig_path.clone().map(PathBuf::from).unwrap_or_else(|| config_dir().join("keyconfig.ron"));
        let keyconfig = KeyConfig::load(&keyconfig_path);

        let mut app = App {
            chart_path,
            autoplay,
            config,
            mode: MODE,
            stage,
            songs,
            active_keys: keyconfig.lane_keys(MODE),
            table_sources,
            table_names,
            table_levels,
            tables_path,
            tables_sel: 0,
            folders,
            folders_path,
            folders_sel: 0,
            text_input: None,
            select_view: SelectView::Root,
            select_items: Vec::new(),
            search: String::new(),
            searching: false,
            sort: SortMode::Default,
            sel: 0,
            set_tab: 0,
            set_sel: 0,
            pending,
            scan_rx,
            scan_count,
            loading_drawn: false,
            ks_rx: None,
            ks_progress: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            ks_cancel: Arc::new(AtomicBool::new(false)),
            ks_total: 0,
            keyconfig,
            keyconfig_path,
            settings_path,
            seed: 1,
            chart_lntype: 0,
            recording: Vec::new(),
            replay,
            replay_cursor: 0,
            cal_sum_us: 0,
            cal_count: 0,
            kc_edit_mode: MODE,
            kc_sel: 0,
            kc_capturing: false,
            kc_warn: false,
            result: None,
            gpu: None,
            audio: None,
            player: None,
            skin: Skin::default_for(MODE, CW as f32, CH as f32),
            skin_cfg: SkinConfig::default(),
            result_palette: ResultPalette::from_skin(&SkinConfig::default()),
            bga_images: std::collections::HashMap::new(),
            bga_events: Vec::new(),
            bga_cursor: 0,
            cur_bga: -1,
            server,
            server_connected,
            clock: Instant::now(),
            anchor_us: 0,
            audio_dead_at: std::cell::Cell::new(None),
            scores,
            scores_path,
            record_modal: None,
            focused_detail: None,
            focused_detail_si: None,
            focus_settle_si: None,
            focus_settle_at: Instant::now(),
            cover_rgba: None,
            cached_select: None,
            cached_select_key: None,
            select_gen: 0,
            preview_audio: None,
            preview_si: None,
            preview_target: None,
            preview_target_at: Instant::now(),
            preview_loop_us: 0,
            preview_next_us: 0,
            preview_sched: Vec::new(),
            preview_cursor: 0,
            preview_anchor: 0,
            preview_start_us: 0,
            preview_end_us: 0,
            preview_prep_rx: None,
            preview_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cursor: (0.0, 0.0),
            hot: Vec::new(),
            last_frame: Instant::now(),
            fps: 0.0,
            frame_count: 0,
            ram_mb: 0.0,
            build_sha256: compute_build_hash(),
            esc_quit_at: None,
            analysis: false,
            analysis_manual: false,
            analysis_paused: false,
            analysis_rate: 1.0,
            analysis_us: 0,
            msoff: Vec::new(),
        };
        app.rebuild_select_items();
        app
    }

}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title("rbms").with_inner_size(winit::dpi::LogicalSize::new(CW, CH));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.gpu = Some(Gpu::new(window.clone()));
        if self.stage == Stage::Play {
            // Direct chart/replay launch: load, then show the keysound-loading bar (or start play if
            // there is nothing to decode). A failed load exits.
            if self.load() {
                self.after_load();
            } else {
                event_loop.exit();
                return;
            }
        }
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Map the physical cursor onto the fixed 1280×720 logical space the UI is laid out
                // in (the surface stretches that space across the whole window).
                if let Some(gpu) = self.gpu.as_ref() {
                    let sz = gpu.window.inner_size();
                    self.cursor = (position.x as f32 * CW as f32 / sz.width.max(1) as f32, position.y as f32 * CH as f32 / sz.height.max(1) as f32);
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.handle_click(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                let pressed = event.state == ElementState::Pressed && !event.repeat;
                match self.stage {
                    Stage::Select if self.record_modal.is_some() => {
                        if pressed {
                            match code {
                                KeyCode::Escape => self.record_modal = None,
                                KeyCode::ArrowUp => self.record_modal_nav(-1),
                                KeyCode::ArrowDown => self.record_modal_nav(1),
                                KeyCode::Enter | KeyCode::NumpadEnter => self.play_record_replay(),
                                _ => {}
                            }
                        }
                    }
                    Stage::Select if self.searching => {
                        if pressed {
                            match code {
                                KeyCode::Escape => self.exit_search(),
                                KeyCode::Backspace => {
                                    self.search.pop();
                                    self.apply_search();
                                }
                                KeyCode::F3 => self.cycle_sort(),
                                KeyCode::ArrowUp => {
                                    self.sel = self.sel.saturating_sub(1);
                                    self.print_selection();
                                }
                                KeyCode::ArrowDown => {
                                    if self.sel + 1 < self.select_items.len() {
                                        self.sel += 1;
                                    }
                                    self.print_selection();
                                }
                                KeyCode::Enter | KeyCode::NumpadEnter => self.select_enter(),
                                _ => {
                                    // Any other key types into the query (letters, digits, space, …).
                                    if let Some(t) = event.text.as_deref() {
                                        let add: String = t.chars().filter(|c| !c.is_control()).collect();
                                        if !add.is_empty() {
                                            self.search.push_str(&add);
                                            self.apply_search();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Stage::Select => {
                        if pressed {
                            if !matches!(code, KeyCode::Escape | KeyCode::ArrowLeft) {
                                self.esc_quit_at = None;
                            }
                            match code {
                                KeyCode::Escape | KeyCode::ArrowLeft => self.select_escape(event_loop),
                                KeyCode::Slash => self.start_search(),
                                KeyCode::F3 => self.cycle_sort(),
                                KeyCode::Tab => self.stage = Stage::Settings,
                                KeyCode::KeyO => self.open_folders(),
                                KeyCode::KeyT => self.open_tables(),
                                KeyCode::KeyR => self.open_record_modal(),
                                KeyCode::ArrowUp => {
                                    self.sel = self.sel.saturating_sub(1);
                                    self.print_selection();
                                }
                                KeyCode::ArrowDown => {
                                    if self.sel + 1 < self.select_items.len() {
                                        self.sel += 1;
                                    }
                                    self.print_selection();
                                }
                                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::ArrowRight => self.select_enter(),
                                _ => {}
                            }
                        }
                    }
                    Stage::Settings => {
                        if pressed {
                            if self.text_input.is_some() {
                                self.settings_text_input(code, event.text.as_deref());
                            } else {
                                let items = SETTING_TABS[self.set_tab].1;
                                self.set_sel = self.set_sel.min(items.len().saturating_sub(1));
                                let focused = items.get(self.set_sel).copied().unwrap_or(0);
                                let on_keyconfig = focused == SETTING_KEYCONFIG;
                                let on_font = focused == SETTING_FONT;
                                let on_net = focused == SETTING_SERVER_URL || focused == SETTING_PLAYER_ID;
                                match code {
                                    KeyCode::Escape => {
                                        self.save_settings();
                                        self.stage = Stage::Select;
                                    }
                                    KeyCode::Tab => {
                                        self.set_tab = (self.set_tab + 1) % SETTING_TABS.len();
                                        self.set_sel = 0;
                                    }
                                    KeyCode::Enter | KeyCode::NumpadEnter => {
                                        if on_keyconfig {
                                            self.enter_keyconfig();
                                        } else if on_font {
                                            self.pick_font();
                                        } else if on_net {
                                            self.begin_net_edit(focused);
                                        } else {
                                            self.save_settings();
                                            self.stage = Stage::Select;
                                        }
                                    }
                                    KeyCode::ArrowUp => self.set_sel = self.set_sel.saturating_sub(1),
                                    KeyCode::ArrowDown => self.set_sel = (self.set_sel + 1).min(items.len().saturating_sub(1)),
                                    KeyCode::ArrowLeft => self.adjust_setting(focused, -1),
                                    KeyCode::ArrowRight => {
                                        if on_keyconfig {
                                            self.enter_keyconfig();
                                        } else if on_net {
                                            self.begin_net_edit(focused);
                                        } else {
                                            self.adjust_setting(focused, 1);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Stage::KeyConfig => {
                        if pressed {
                            self.keyconfig_input(code);
                        }
                    }
                    Stage::Tables => {
                        if pressed {
                            self.tables_input(event_loop, code, event.text.as_deref());
                        }
                    }
                    Stage::Folders => {
                        if pressed {
                            self.folders_input(code);
                        }
                    }
                    Stage::Result => {
                        if pressed && matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter) {
                            self.to_select_or_exit(event_loop);
                        }
                    }
                    Stage::Loading => {
                        if pressed && code == KeyCode::Escape {
                            self.pending = None;
                            self.scan_rx = None;
                            self.ks_cancel.store(true, Ordering::Relaxed);
                            self.ks_rx = None; // abandon any in-flight keysound decode
                            self.stop_preview();
                            // Cancel back to Select rather than exit: during the initial background scan
                            // `songs` is still empty, and routing through `to_select_or_exit` would quit
                            // on the empty library — surprising mid-load. Drop any half-loaded play state.
                            self.audio = None;
                            self.player = None;
                            self.select_view = SelectView::Root;
                            self.sel = 0;
                            self.rebuild_select_items();
                            self.stage = Stage::Select;
                        }
                    }
                    Stage::Play => {
                        if code == KeyCode::Escape {
                            // If there is nothing left to hit (every note resolved), skip straight to
                            // the result screen instead of discarding the run; otherwise quit out.
                            let done = self.player.as_ref().is_some_and(|p| p.judge.total_notes() > 0 && p.judge.total_judged() >= p.judge.total_notes());
                            if done {
                                self.enter_result();
                            } else {
                                self.to_select_or_exit(event_loop);
                            }
                            return;
                        }
                        if pressed && self.analysis_key(code) {
                            return;
                        }
                        if pressed {
                            if let Some(action) = self.control_for(code) {
                                self.apply_control(action);
                                return;
                            }
                        }
                        if !self.autoplay && self.replay.is_none() {
                            if let Some(lane) = self.lane_for(code) {
                                let raw = self.song_us();
                                let judge_t = judge_time_us(raw, self.config.offset_ms);
                                let sound_t = keysound_time_us(raw, self.anchor_us);
                                match event.state {
                                    ElementState::Pressed if !event.repeat => {
                                        self.recording.push(ReplayEvent { t: raw, lane, press: true });
                                        let mut hit: Option<rbms_judge::JudgeResult> = None;
                                        if let (Some(player), Some(audio)) = (self.player.as_mut(), self.audio.as_mut()) {
                                            hit = player.press(lane, judge_t, |e: PlayEvent| audio.play(e.wav.max(0) as u32, 1.0, 0.0, 1.0, sound_t));
                                        }
                                        // auto-calibration: accumulate the timing error of accurate hits
                                        // (PG/GR/GD, ±150ms) — offset is recentred for the next run at
                                        // enter_result, so within-run judging stays consistent.
                                        if self.config.auto_offset {
                                            if let Some(r) = hit {
                                                if (r.judge as usize) <= 2 && r.delta_us.abs() <= 150_000 {
                                                    self.cal_sum_us += r.delta_us;
                                                    self.cal_count += 1;
                                                }
                                            }
                                        }
                                    }
                                    ElementState::Released => {
                                        self.recording.push(ReplayEvent { t: raw, lane, press: false });
                                        if let Some(player) = self.player.as_mut() {
                                            player.release(lane, judge_t);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.frame();
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// Resolve `~/.config/rbms`, the per-user config dir holding settings/keyconfig/scores/tables/
/// folders/theme/replays.
fn config_dir() -> PathBuf {
    config_dir_from(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

/// Pure config-dir resolution (testable without touching process env): prefer `HOME` (Unix/macOS),
/// then `USERPROFILE` (Windows, where `HOME` is usually unset), then the current dir. Without the
/// `USERPROFILE` fallback all config landed in the cwd on Windows instead of the user profile.
fn config_dir_from(home: Option<std::ffi::OsString>, userprofile: Option<std::ffi::OsString>) -> PathBuf {
    home.or(userprofile).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(".config/rbms")
}

fn main() {
    let settings_path = config_dir().join("settings.ron");
    let saved = PlaySettings::load(&settings_path);
    let mut cfg = PlayerConfig::default();
    apply_settings(&mut cfg, &saved);
    let mut autoplay = saved.autoplay;

    let mut chart: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--interactive" => autoplay = false,
            "--auto" => autoplay = true,
            "--sc-left" => cfg.scratch_left = true,
            "--sc-auto" => cfg.scratch_auto = true,
            "--lift" => {
                if let Some(v) = args.next().and_then(|v| v.parse::<f32>().ok()) {
                    cfg.lift = v.clamp(0.0, 0.9);
                }
            }
            "--hispeed" => {
                if let Some(v) = args.next().and_then(|v| v.parse::<f64>().ok()) {
                    cfg.hispeed = v.clamp(0.5, 10.0);
                }
            }
            "--gauge" => {
                if let Some(name) = args.next() {
                    cfg.gauge = gauge_from_name(&name);
                }
            }
            "--skin" => cfg.skin_path = args.next(),
            "--font" => cfg.font_path = args.next(),
            "--server" => cfg.server_url = args.next(),
            "--table" => cfg.table_url = args.next(),
            "--keyconfig" => cfg.keyconfig_path = args.next(),
            "--replay" => cfg.replay_path = args.next(),
            "--player" => {
                if let Some(id) = args.next() {
                    cfg.player_id = id;
                }
            }
            "--keys" => {
                if let Some(spec) = args.next() {
                    let parsed: Vec<(KeyCode, usize)> = spec.split(',').enumerate().filter_map(|(i, n)| key_from_name(n.trim()).map(|k| (k, i))).collect();
                    if !parsed.is_empty() {
                        cfg.keys_override = Some(parsed);
                    }
                }
            }
            other if chart.is_none() && !other.starts_with("--") => chart = Some(other.to_string()),
            _ => {}
        }
    }
    // Folder/chart precedence: explicit launch arg > the remembered song folder > $RBMS_SONGS.
    let remembered = saved.songs_folder.clone().filter(|s| !s.trim().is_empty()).or_else(|| std::env::var("RBMS_SONGS").ok().filter(|s| !s.trim().is_empty()));
    let chart = match chart {
        Some(c) => c,
        None if cfg.replay_path.is_some() => String::new(),
        // A bare launch (no chart/folder arg, nothing remembered) is the first-run / .dmg double-click
        // case: open the GUI on an empty library with an onboarding CTA instead of exiting to a
        // terminal that isn't there. `App::new` treats an empty path as a bare launch.
        None => remembered.unwrap_or_default(),
    };

    // Apply a user-chosen UI font (settings or --font) before any text is drawn.
    if let Some(fp) = cfg.font_path.clone() {
        match std::fs::read(&fp) {
            Ok(bytes) => match rbms_render::load_font(bytes) {
                Some(family) => {
                    rbms_render::set_ui_family(&family);
                    println!("font: {fp} ({family})");
                }
                None => eprintln!("font load failed (no usable face): {fp}"),
            },
            Err(e) => eprintln!("font not found: {fp} ({e})"),
        }
    }

    // Apply the UI theme (colours for the select/result/menu chrome) before any text is drawn.
    load_theme(&settings_path);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(chart, autoplay, cfg, settings_path);
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(test)]
mod tests {
    use super::{
        ROOT_ESC_CONFIRM, SortMode, THEME_TEMPLATE, bundled_skin, calibrated_offset, clear_type_from_id, clear_type_id, client_platform, compute_build_hash, config_dir_from, default_total,
        esc_confirms_quit, fmt_datetime, green_number_for, ir_submission_block_reason, judge_time_us, keysound_time_us, resumed_clock_us, updates_score, write_atomic,
    };
    use rbms_judge::ClearType;
    use std::time::{Duration, Instant};

    #[test]
    fn config_dir_prefers_home_then_userprofile() {
        use std::ffi::OsString;
        use std::path::PathBuf;
        let cd = |h: Option<&str>, u: Option<&str>| config_dir_from(h.map(OsString::from), u.map(OsString::from));
        assert_eq!(cd(Some("/home/u"), None), PathBuf::from("/home/u").join(".config/rbms"));
        assert_eq!(cd(None, Some("C:/Users/u")), PathBuf::from("C:/Users/u").join(".config/rbms"), "USERPROFILE used when HOME unset (Windows)");
        assert_eq!(cd(Some("/h"), Some("C:/x")), PathBuf::from("/h").join(".config/rbms"), "HOME wins over USERPROFILE");
        assert_eq!(cd(None, None), PathBuf::from(".").join(".config/rbms"));
    }

    #[test]
    fn theme_template_is_valid_ron_and_matches_defaults() {
        // The bundled template (written to ~/.config/rbms/theme.ron on first run) must be valid RON,
        // and its values are the defaults — so a fresh install looks identical to no theme file.
        let parsed: Result<rbms_render::ThemeConfig, _> = ron::from_str(THEME_TEMPLATE);
        assert!(parsed.is_ok(), "theme template must be valid RON: {parsed:?}");
        assert_eq!(parsed.unwrap().resolve(), rbms_render::Theme::default(), "template values equal the defaults");
    }

    #[test]
    fn datetime_formats_utc() {
        assert_eq!(fmt_datetime(0), "1970-01-01 00:00");
        assert_eq!(fmt_datetime(86_400_000), "1970-01-02 00:00");
        assert_eq!(fmt_datetime(1_700_000_000_000), "2023-11-14 22:13");
    }

    #[test]
    fn clear_lamp_id_roundtrips() {
        for c in [ClearType::NoPlay, ClearType::Failed, ClearType::AssistEasy, ClearType::Easy, ClearType::Normal, ClearType::Hard, ClearType::ExHard, ClearType::FullCombo, ClearType::Perfect, ClearType::Max] {
            assert_eq!(clear_type_from_id(clear_type_id(c)), c, "{c:?} lamp id must round-trip");
        }
    }

    #[test]
    fn build_hash_is_64_hex_chars() {
        let h = compute_build_hash().expect("the test binary should be readable");
        assert_eq!(h.len(), 64, "SHA-256 hex is 64 chars");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "hash is lowercase hex");
    }

    #[test]
    fn client_platform_is_os_arch() {
        let p = client_platform();
        assert!(p.contains('-'), "platform tag is OS-ARCH");
        assert_eq!(p, format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH));
    }

    #[test]
    fn calibrated_offset_recenters_from_mean() {
        assert_eq!(calibrated_offset(0, 60_000), 60, "60ms-early avg -> +60ms offset");
        assert_eq!(calibrated_offset(0, -40_000), -40);
        assert_eq!(calibrated_offset(60, 0), 60, "no error -> no change");
        assert_eq!(calibrated_offset(0, 0), 0);
        assert_eq!(calibrated_offset(190, 50_000), 200, "clamped to +200");
    }

    #[test]
    fn calibration_converges_in_one_run() {
        // a constant +60ms-early bias: the run's mean error is (bias - offset*1000);
        // recentring once should drive the next run's mean error to ~0.
        let bias_us: i64 = 60_000;
        let offset0 = 0;
        let mean_run1 = bias_us - offset0 as i64 * 1000;
        let offset1 = calibrated_offset(offset0, mean_run1);
        let mean_run2 = bias_us - offset1 as i64 * 1000;
        assert!(mean_run2.abs() <= 1_000, "after one calibration the residual error is ~0 (was {mean_run2}us)");
    }

    // --- calibrated_offset clamping & rounding edges ---

    #[test]
    fn calibrated_offset_clamps_both_ends() {
        assert_eq!(calibrated_offset(-190, -50_000), -200, "clamped to -200");
        assert_eq!(calibrated_offset(190, 50_000), 200, "clamped to +200");
        assert_eq!(calibrated_offset(200, 1_000_000), 200, "never exceeds +200");
        assert_eq!(calibrated_offset(-200, -1_000_000), -200, "never below -200");
    }

    #[test]
    fn calibrated_offset_rounds_to_nearest_ms() {
        assert_eq!(calibrated_offset(0, 1_400), 1, "1.4ms rounds to 1");
        assert_eq!(calibrated_offset(0, 1_600), 2, "1.6ms rounds to 2");
        assert_eq!(calibrated_offset(0, -1_600), -2, "-1.6ms rounds to -2");
        assert_eq!(calibrated_offset(0, 499), 0, "sub-half-ms rounds to 0");
    }

    // --- default_total ---

    #[test]
    fn default_total_has_a_floor_of_260() {
        // Sparse charts hit the 260.0 floor.
        assert!(default_total(0) >= 260.0);
        assert!(default_total(1) >= 260.0);
        assert!(default_total(50) >= 260.0);
    }

    #[test]
    fn default_total_zero_and_one_note_equal_due_to_floor() {
        // notes.max(1): 0 notes is treated like 1 note.
        assert_eq!(default_total(0), default_total(1));
    }

    #[test]
    fn default_total_is_non_decreasing_in_note_count() {
        // More notes never reduces the gauge TOTAL (denser charts gain at least as much).
        let mut prev = default_total(1);
        for n in [10, 100, 500, 1000, 5000, 20000] {
            let t = default_total(n);
            assert!(t >= prev - 1e-9, "TOTAL non-decreasing at {n} notes ({t} < {prev})");
            prev = t;
        }
    }

    #[test]
    fn default_total_large_charts_exceed_floor() {
        // A dense chart should produce a TOTAL well above the 260 floor.
        assert!(default_total(2000) > 260.0, "dense chart rises above the floor");
    }

    // --- SortMode cycle / labels ---

    #[test]
    fn sortmode_next_cycles_through_all_and_returns_to_start() {
        // SortMode derives PartialEq but not Debug, so compare with `==` (not assert_eq!).
        let mut seen = Vec::new();
        let mut m = SortMode::Default;
        for _ in 0..SortMode::ALL.len() {
            seen.push(m);
            m = m.next();
        }
        // visited every variant exactly once before wrapping
        assert_eq!(seen.len(), SortMode::ALL.len());
        for variant in SortMode::ALL {
            assert!(seen.iter().any(|&s| s == variant), "{:?} visited", variant.label());
        }
        assert!(m == SortMode::Default, "wraps back to the start after a full cycle");
    }

    #[test]
    fn sortmode_next_advances_by_one_each_step() {
        // Each next() advances to the following variant in ALL, wrapping at the end. Verified by
        // matching the label rather than the (Debug-less) variant.
        let chain = [SortMode::Default, SortMode::Title, SortMode::Artist, SortMode::Level, SortMode::Clear];
        for w in chain.windows(2) {
            assert_eq!(w[0].next().label(), w[1].label(), "{} -> {}", w[0].label(), w[1].label());
        }
        assert_eq!(SortMode::Clear.next().label(), SortMode::Default.label(), "last wraps to first");
    }

    #[test]
    fn sortmode_labels_distinct_and_nonempty() {
        let mut labels: Vec<&str> = SortMode::ALL.iter().map(|m| m.label()).collect();
        for l in &labels {
            assert!(!l.is_empty(), "label non-empty");
        }
        let n = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), n, "all labels distinct");
    }

    // --- bundled skins parse ---

    #[test]
    fn bundled_skins_parse_for_both_names() {
        // include_str! embeds the skin RON at build time; both bundled names must deserialize.
        // (Any name that is not "WIDE" resolves to the NORMAL skin.)
        let _wide = bundled_skin("WIDE");
        let _normal = bundled_skin("NORMAL");
        let _default = bundled_skin("anything-else");
        let _case = bundled_skin("wide"); // case-insensitive match for WIDE
    }

    #[test]
    fn judge_time_shifts_by_the_offset_in_milliseconds() {
        assert_eq!(judge_time_us(1_000_000, 0), 1_000_000);
        assert_eq!(judge_time_us(1_000_000, 30), 1_030_000, "+30ms offset judges 30ms later");
        assert_eq!(judge_time_us(1_000_000, -45), 955_000, "-45ms offset judges 45ms earlier");
    }

    #[test]
    fn keysound_schedule_uses_raw_input_time_not_the_judge_offset() {
        assert_eq!(keysound_time_us(1_000_000, 7_500_000), 8_500_000);
        assert_eq!(keysound_time_us(1_000_000, 0), 1_000_000, "no anchor => the raw instant");
        assert_eq!(keysound_time_us(0, -250_000), -250_000, "a negative anchor shifts it back");
    }

    /// Reproduce the press path of `main.rs`'s key handler: judge at `judge_time_us(raw, offset)`,
    /// schedule the keysound at `keysound_time_us(raw, anchor)`. Returns the times the keysound
    /// callback was scheduled at.
    fn press_schedule_times(offset_ms: i32, raw_us: i64, anchor_us: i64) -> Vec<i64> {
        let model = rbms_chart::to_model(&rbms_parser::parse(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n"), rbms_model::Mode::BEAT_7K);
        let mut player = rbms_play::Player::new(model, false);
        let judge_t = judge_time_us(raw_us, offset_ms);
        let sound_t = keysound_time_us(raw_us, anchor_us);
        let mut scheduled = Vec::new();
        player.press(0, judge_t, |_e: rbms_play::PlayEvent| scheduled.push(sound_t));
        scheduled
    }

    #[test]
    fn press_path_schedules_the_keysound_at_raw_plus_anchor_for_every_offset() {
        let (raw, anchor) = (1_000_000_i64, 7_500_000_i64);
        for offset_ms in [-200, -30, 0, 30, 200] {
            let scheduled = press_schedule_times(offset_ms, raw, anchor);
            assert_eq!(scheduled, vec![8_500_000], "offset {offset_ms}ms must not move the sound");
        }
    }

    #[test]
    fn press_path_moves_the_judgment_with_the_offset_while_the_sound_stays() {
        let (raw, anchor) = (1_000_000_i64, 7_500_000_i64);
        assert_eq!(judge_time_us(raw, 30), 1_030_000);
        assert_eq!(press_schedule_times(30, raw, anchor), vec![8_500_000]);
        assert_eq!(press_schedule_times(0, raw, anchor), vec![8_500_000]);
    }

    #[test]
    fn resumed_clock_continues_from_the_last_audio_position() {
        assert_eq!(resumed_clock_us(12_000_000, 2_500_000), 14_500_000);
        assert_eq!(resumed_clock_us(12_000_000, 0), 12_000_000, "no wall time yet => no movement");
    }

    #[test]
    fn green_number_constant_depends_only_on_hispeed_and_cover() {
        assert_eq!(green_number_for(true, 120.0, 2.0, 1.0, 0.0), 1000.0);
        assert_eq!(green_number_for(true, 300.0, 2.0, 0.5, 0.0), 1000.0, "BPM/SCROLL do not apply");
        assert!((green_number_for(true, 120.0, 2.0, 1.0, 0.25) - 750.0).abs() < 1e-3);
    }

    #[test]
    fn green_number_floating_tracks_bpm_scroll_and_cover() {
        assert_eq!(green_number_for(false, 120.0, 1.0, 1.0, 0.0), 2000.0);
        assert_eq!(green_number_for(false, 240.0, 1.0, 1.0, 0.0), 1000.0);
        assert!((green_number_for(false, 120.0, 1.0, 2.0, 0.0) - 1000.0).abs() < 1e-9, "SCROLL 2.0 halves the travel time");
        assert!((green_number_for(false, 120.0, 2.0, 1.0, 0.0) - 1000.0).abs() < 1e-9, "hi-speed 2.0 halves it too");
        assert!((green_number_for(false, 120.0, 1.0, 1.0, 0.4) - 1200.0).abs() < 1e-3);
    }

    #[test]
    fn ir_submission_allowed_only_for_an_unassisted_interactive_play() {
        assert_eq!(ir_submission_block_reason(false, false, 100, false), None);
        assert_eq!(ir_submission_block_reason(false, false, 50, false), None, "a NARROWED judge window is not an assist");
    }

    #[test]
    fn ir_submission_blocked_for_autoplay_replay_and_assists() {
        assert_eq!(ir_submission_block_reason(true, false, 100, false), Some("autoplay"));
        assert_eq!(ir_submission_block_reason(false, true, 100, false), Some("replay playback"));
        assert_eq!(ir_submission_block_reason(false, false, 105, false), Some("judge window widened"));
        assert_eq!(ir_submission_block_reason(false, false, 100, true), Some("scratch assist"));
    }

    #[test]
    fn updates_score_is_the_exact_complement_of_the_ir_block_reason() {
        for &autoplay in &[false, true] {
            for &replay in &[false, true] {
                for &judge_rate in &[50, 100, 105, 200] {
                    for &scratch_auto in &[false, true] {
                        let blocked = ir_submission_block_reason(autoplay, replay, judge_rate, scratch_auto).is_some();
                        assert_eq!(
                            updates_score(autoplay, replay, judge_rate, scratch_auto),
                            !blocked,
                            "autoplay={autoplay} replay={replay} judge_rate={judge_rate} scratch_auto={scratch_auto}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn updates_score_only_for_an_unassisted_interactive_play() {
        assert!(updates_score(false, false, 100, false));
        assert!(updates_score(false, false, 50, false), "a narrowed judge window still scores");
        assert!(!updates_score(false, false, 101, false), "a widened judge window does not");
        assert!(!updates_score(false, false, 100, true), "auto scratch does not");
        assert!(!updates_score(true, false, 100, false));
        assert!(!updates_score(false, true, 100, false));
    }

    #[test]
    fn first_escape_arms_the_quit_confirmation_instead_of_quitting() {
        assert!(!esc_confirms_quit(None, Instant::now()), "a lone Esc never quits");
    }

    #[test]
    fn second_escape_quits_only_within_the_confirm_window() {
        let now = Instant::now();
        assert!(esc_confirms_quit(Some(now), now), "an immediate second Esc quits");
        assert!(esc_confirms_quit(Some(now), now + ROOT_ESC_CONFIRM), "exactly at the window edge still quits");
        assert!(!esc_confirms_quit(Some(now), now + ROOT_ESC_CONFIRM + Duration::from_millis(1)), "a late second Esc does not quit");
    }

    #[test]
    fn write_atomic_writes_the_contents_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("rbms_atomic_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested/settings.ron");
        write_atomic(&path, "(hispeed: 1.0)").expect("atomic write creates the parent dir and the file");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "(hispeed: 1.0)");
        assert!(!path.with_file_name("settings.ron.tmp").exists(), "the temp file is renamed away, not left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_replaces_an_existing_file_wholesale() {
        let dir = std::env::temp_dir().join(format!("rbms_atomic_replace_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("scores.ron");
        write_atomic(&path, "aaaaaaaaaaaaaaaaaaaa").unwrap();
        write_atomic(&path, "bb").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "bb");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_rejects_a_path_without_a_file_name() {
        assert!(write_atomic(std::path::Path::new("/"), "x").is_err(), "a directory path is not a writable target");
    }
}

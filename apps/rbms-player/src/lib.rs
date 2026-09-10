//! The rbms player application: window, stages, audio wiring and the whole GUI.
//!
//! The crate root is the wiring file — [`run`] is the entry point the thin `main` binary calls, and
//! everything else lives in the modules below. Building the app as a library (with the binary a
//! three-line shim) is what lets the integration tests under `tests/` reach these types.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use rbms_audio::{AudioEngine, AudioOpenReport, AudioOptions, Bus, IdNamespace};
use rbms_chart::default_total_for_mode;
use rbms_chart::shuffle::NoteOption;
use rbms_chart::to_model;
use rbms_config::{
    Config, HISPEED_MAX, HISPEED_MIN, LANE_SHADE_MAX, LANE_SHADE_MIN, LANE_SHADE_STEP, PLAY_ESCAPE_DOUBLE_MS, PLAY_ESCAPE_HOLD_MS, PlayEscape, SortMode,
    TableSource, algorithm_token, gauge_auto_shift_token, gauge_from_name, gauge_set_token, gauge_token, ln_mode_token,
};
use rbms_ir::mapping::{CUSTOM_JUDGE_ASSIST, LIGHT_ASSIST, NO_ASSIST, assist_flags, assist_level, combo_breaks, ir_clear, ir_gauge, ir_random};
use rbms_ir::{
    API_VERSION, AuthResponse, ChartId, IrError, JudgeBreakdown, PlayOptions, PlayerId, PlayerProfile, ReplayData, ScoreServer, ScoreSubmission, SettingsBlob,
    SettingsPutResult, SubmitOutcome,
};
use rbms_judge::gauge::{AssistLevel, assist_downgrade};
use rbms_judge::{ClearType, clear_type_from_id, clear_type_id};
use rbms_library::{ChartDetail, Library, compute_chart_detail};
use rbms_model::Mode;
use rbms_play::{ANALYSIS_SEEK_STEP_US, NullSink, PlaySession, Player, ScratchDir, SessionClock, SessionOptions};
use rbms_render::{
    Color, CoverState, DensityView, DetailView, HudView, PlayfieldView, RANK_BANDS, RecordRowView, RecordsView, Rect, Renderer, ResultPalette, ResultView,
    SelectDetail, SelectHot, SelectModal, SelectRow, SelectView as SelectScene, Skin, SkinConfig, StatCell, cover_rect, dj_rank, draw_text, draw_text_centered,
    draw_text_right, ex_delta_label, render_hud, render_key_bomb, render_lane_cover, render_playfield_view, render_result_with_palette, render_select,
    text_width,
};
use rbms_store::{Replay, ReplayJudge, SCORE_LN_MODE_FROM_CHART, SCORE_RULE_VERSION, ScoreBook, ScoreRecord};

use sha2::{Digest, Sha256};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

mod app_input;
mod app_network;
mod app_options;
mod app_play;
mod app_ranking;
mod app_result;
mod assets;
mod dialog;
mod favorites;
mod format;
mod gpu;
mod ir_outcome;
mod ir_panel;
mod ir_ranking;
mod ir_ranking_view;
mod ir_replay;
mod ir_session;
mod ir_sync;
mod judge_setup;
mod keyconfig;
#[cfg(test)]
mod keyconfig_tests;
#[cfg(test)]
mod main_tests;
mod notify;
mod play_sink;
mod settings_ui;
mod settings_view;
mod stage;
mod tablesrc;
pub mod target;
mod textedit;
mod timing;
mod toast;
use app_network::build_server;
use app_play::schedule_poll_interval_us;
pub(crate) use assets::{bundled_skin, decode_bga_256, keysound_jobs, load_theme, resolve_file, scan_folders, spawn_keysound_decode};
use favorites::{Favorites, favorites_path};
use format::{
    clear_label_color, difficulty_color, difficulty_name, fmt_datetime, fmt_duration, gauge_name, mode_color, mode_short, rank_label, rule_version_mark,
    rule_version_note,
};
use gpu::Gpu;
use ir_outcome::{IR_RESULT_LINE_H, IR_RESULT_SCALE, IR_RESULT_X, IR_RESULT_Y, IrStatus, ir_line_color};
use ir_ranking::{RANKING_CACHE_CAPACITY, RankingCache, RankingFetch};
use ir_ranking_view::render_ranking_panel;
use ir_session::{AccountSession, AuthAction};
use ir_sync::SyncLock;
use judge_setup::{is_custom_judge, run_judge_setup, run_lntype};
use keyconfig::{ControlAction, KeyConfig, key_from_name, key_name};
use notify::{Level, notify};
use play_sink::PlayAudioSink;
use settings_view::{SettingsHot, render_settings};
use stage::{Canvas, FrameCtx, KeyInput, LoadingState, SelectState, Stage, StageId, Transition};
use tablesrc::{TableLevels, fetch_and_match};
use textedit::{TextEdit, edit_key};
use timing::{SoakLogger, SoakSnapshot, TIMING_CSV_ENV, TimingProbe, TimingSample, env_path, us_to_millis};
use toast::ToastQueue;

pub(crate) use rbms_ir::mapping as ir_map;
pub(crate) use rbms_store as replay;
pub(crate) use rbms_store::write_atomic;

const CW: u32 = 1280;
const CH: u32 = 720;
const MODE: Mode = Mode::BEAT_7K;

/// `#PREVIEW` hover-preview tuning: the reserved sample id (the base of the preview namespace, so
/// preview sounds never collide with the loaded chart's keysounds), focus-settle debounce, and the
/// silent tail held after the last autoplay event before the loop restarts. The gain is a setting,
/// so it is read off the config rather than fixed here.
const PREVIEW_ID: u32 = IdNamespace::PREVIEW.base;
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(333);
const PREVIEW_LOOP_TAIL_US: i64 = 2_000_000;

/// Shortest loop period the file preview will re-trigger on, so a zero-length clip cannot spin.
const PREVIEW_MIN_LOOP_US: i64 = 1;

/// `at_us` that marks a keysound as "sound it now": the mixer collapses a schedule at frame zero to
/// the next callback boundary without counting it as a late schedule. Interactive presses and
/// anything else that must not wait for the lookahead use this.
const IMMEDIATE_KEYSOUND_AT_US: i64 = 0;

/// How long an audio setting must rest before the shared output stream is reopened for it.
const AUDIO_REOPEN_DEBOUNCE: Duration = Duration::from_millis(400);

/// Neutral placement of a preview voice: centred and unpitched. A chart keysound carries the
/// equivalents from `rbms-play`, so the run itself decides how its own sounds are voiced.
const KEYSOUND_PAN: f32 = 0.0;
const KEYSOUND_PITCH: f32 = 1.0;

/// How long the select focus must rest on a row before its heavy detail (full chart parse + timing
/// integration + cover decode) is computed. Frame-count debouncing tied the delay to the frame rate;
/// this keeps it constant.
const FOCUS_DETAIL_DEBOUNCE: Duration = Duration::from_millis(150);

/// How long a second Esc press at the select root still counts as confirming "quit".
const ROOT_ESC_CONFIRM: Duration = Duration::from_secs(1);

/// Background → main-thread messages for a hover preview: a decoded `#PREVIEW` clip, or — for a
/// chart that names none — the extracted keysound timeline followed by each decoded keysound. The
/// channel disconnecting signals the load is complete, at which point the browser anchors the clock
/// and begins playback.
enum PreviewMsg {
    Clip(rbms_audio::DecodedAudio),
    Schedule { sched: Vec<(i64, u32)>, start_us: i64, end_us: i64 },
    Keysound(u32, rbms_audio::DecodedAudio),
}

/// Overrides that come from the command line and are never persisted: they describe how this one
/// run was started, not what the player remembers between runs. Everything the settings screen can
/// change lives in [`Config`] instead.
#[derive(Clone, Debug, Default)]
struct LaunchOptions {
    /// `--keys`: lane bindings for this run, bypassing the key config file.
    keys_override: Option<Vec<(KeyCode, usize)>>,
    /// `--skin`: a skin RON to load instead of the configured bundled skin.
    skin_path: Option<String>,
    /// `--table`: a difficulty table added to the library for this run.
    table_url: Option<String>,
    /// `--keyconfig`: a key config file to use instead of the one in the config directory.
    keyconfig_path: Option<String>,
    /// `--replay`: a saved replay to play back instead of an interactive run.
    replay_path: Option<String>,
    /// `--timing-csv <path>`: where the per-input timing ring is dumped when the result screen is
    /// entered. `None` falls back to the `RBMS_TIMING_CSV` environment variable.
    timing_csv: Option<String>,
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
/// the reference implementation: only a real interactive PLAY reaches the IR (`MusicResult.java:82`), and any assist —
/// a judge width or long-note margin widened past 100% (`BMSPlayer.java:208-214`) or an auto-played
/// lane (`BMSPlayer.java:248-252`) — clears the score flag.
fn ir_submission_block_reason(autoplay: bool, replay: bool, custom_judge: bool, scratch_auto: bool) -> Option<&'static str> {
    if autoplay {
        return Some("autoplay");
    }
    if replay {
        return Some("replay playback");
    }
    if custom_judge {
        return Some("judge window widened");
    }
    if scratch_auto {
        return Some("scratch assist");
    }
    None
}

/// Whether this run may update the stored bests (EX / lamp / BP), i.e. it was an unassisted
/// interactive play. Same predicate as [`ir_submission_block_reason`], so the IR gate and the local
/// score book never disagree — the reference implementation derives both from the one `score` flag
/// (`BMSPlayer.java:203-252` clears it wherever it raises `assist`, `:363`
/// `resource.setUpdateScore(score)`, `MusicResult.java:444-446` passes it to
/// `PlayDataAccessor.writeScoreData`, and `ScoreData.java:548,566,572,578` gate
/// exscore/avgjudge/minbp/combo on it).
fn updates_score(autoplay: bool, replay: bool, custom_judge: bool, scratch_auto: bool) -> bool {
    ir_submission_block_reason(autoplay, replay, custom_judge, scratch_auto).is_none()
}

/// Whether a run at this assist level may still leave a replay behind. A custom judge is the level
/// decision 12 blocks the recording at; an auto-played lane keeps its replay.
///
/// Keeping it is a deliberate divergence: the reference gates the save on the score flag
/// (`MusicResult.java:317-321`), which every assist clears, so it saves no replay at
/// [`LIGHT_ASSIST`] either. Recorded in `docs/acknowledge/reference-divergences.md`.
fn saves_replay(assist: u8) -> bool {
    assist < CUSTOM_JUDGE_ASSIST
}

/// The lamp a run at this assist level is awarded: the reference demotes any assisted clear to
/// `LightAssistEasy` or `AssistEasy` and so puts the full-combo lamps out of reach
/// (`BMSPlayer.java:866` reads `assist == 1 ? LightAssistEasy : AssistEasy`, so only the single
/// light assist takes the higher lamp and anything stronger takes the lower one).
fn assisted_lamp(lamp: ClearType, assist: u8) -> ClearType {
    let level = match assist {
        NO_ASSIST => AssistLevel::None,
        LIGHT_ASSIST => AssistLevel::Light,
        _ => AssistLevel::Full,
    };
    assist_downgrade(lamp, level)
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

/// One editable row in the key-config screen.
enum KcRow {
    ModeSelect,
    Control(ControlAction),
    Lane(usize),
    /// The second key a scratch lane may be spun backwards with.
    ScratchReverse(usize),
}

fn kc_rows(edit_mode: Mode) -> Vec<KcRow> {
    let mut rows = vec![KcRow::ModeSelect];
    rows.extend(ControlAction::ALL.into_iter().map(KcRow::Control));
    rows.extend((0..edit_mode.key).map(KcRow::Lane));
    rows.extend((0..edit_mode.key).filter(|&lane| edit_mode.is_scratch(lane)).map(KcRow::ScratchReverse));
    rows
}

/// Where the song-select browser currently is. Navigation is Root → (ALL SONGS | each table →
/// per-level folder) → charts, modelling the reference implementation's table-as-custom-folder browsing. The
/// indices select a table and a level within `AppShared::table_levels`.
#[derive(Clone, Copy, PartialEq)]
enum SelectView {
    Root,
    AllSongs,
    TableLevels(usize),
    TableLevel(usize, usize),
}

/// One row in the select list: either a folder to descend into, or a playable chart (index
/// into `AppShared::library`).
enum SelectItem {
    Song(usize),
    Folder { label: String, target: SelectView },
}

/// Cache key for the assembled [`SelectScene`]: rebuild only when one of these changes, so the scene
/// is not re-allocated every frame of the continuous redraw loop. `select_gen` bumps on any list
/// rebuild (catches same-length folder swaps); `scores` length catches a freshly saved record.
type SelectKey = (u64, usize, Option<usize>, usize, bool, bool);

/// A clickable region recorded during rendering and hit-tested on a left-click. Immediate-mode:
/// `AppShared::hot` is rebuilt every frame for the current stage, so the layout math lives in one place.
#[derive(Clone, Copy)]
enum Hot {
    SelectRow(usize),
    RecordRow(usize),
    SettingTab(usize),
    SettingRow(usize),
    RivalRow(usize),
    RankingRow(usize),
    ModalClose,
    ModalReplay,
    NavSearch,
    NavSort,
    NavFolders,
    NavTables,
    NavRecords,
    NavSettings,
}

/// The running application: the state that outlives a stage change, the screen that is up, and the
/// screens it was opened over.
struct App {
    shared: AppShared,
    stage: Stage,
    /// Screens suspended by one opened on top of them; [`Transition::Back`] resumes the newest.
    /// Only ever a screen the browser (or the settings screen) opened, so it stays one or two deep.
    suspended: Vec<Stage>,
    /// A chart or replay named on the command line, loaded once the window exists.
    launch_chart: bool,
    /// Why the app could not start, set when the window or the GPU backend could not be brought up.
    /// The event loop is asked to exit and [`run`] turns this into a failing exit code, so a machine
    /// with no usable adapter gets a message instead of a panic.
    startup_error: Option<String>,
}

/// Everything that survives a stage change: the configuration, the library and score book, the
/// window and audio, the network session and the frame bookkeeping.
///
/// Stage-local state belongs to the [`Stage`] variant that owns it. The browser's own list state
/// (which folder is open, the filtered rows, the cursor) is here rather than in
/// [`stage::SelectState`] because every other screen returns to it and the debug overlay reports it
/// from wherever it is.
struct AppShared {
    chart_path: String,
    /// Everything the player remembers between runs, in one document. The settings screen edits it
    /// in place and every save writes the whole thing.
    config: Config,
    /// This run's command-line overrides, which the configuration never sees.
    launch: LaunchOptions,
    mode: Mode,
    /// The scanned charts plus their md5 index, so every chart-keyed lookup (scores, difficulty
    /// tables, IR rankings) is a hash lookup rather than a scan of the whole library.
    library: Library,
    active_keys: Vec<(KeyCode, usize)>,
    /// The second key each scratch lane may be spun with, which ends a charge note the forward key
    /// is holding (`JudgeManager.java:358-372`). Empty for a mode with no scratch lane, and for a
    /// run whose lane bindings came from the command line.
    active_reverse_keys: Vec<(KeyCode, usize)>,
    table_names: Vec<String>,
    table_levels: Vec<TableLevels>,
    select_view: SelectView,
    select_items: Vec<SelectItem>,
    /// Incremental song search: `searching` opens the box (`/`), `search` is the live query (filters
    /// the list by title/artist/subtitle). What orders the filtered list is the configuration's own
    /// SORT row, which F3 and the settings screen both move, so the two cannot disagree.
    search: String,
    searching: bool,
    sel: usize,
    /// Bumped on any list rebuild, so the assembled select scene can be cached across frames.
    select_gen: u64,
    keyconfig: KeyConfig,
    keyconfig_path: PathBuf,
    settings_path: PathBuf,
    /// The mode the key-config editor is editing, remembered between visits to it.
    kc_edit_mode: Mode,
    /// The replay this run was started from, kept for the whole run so every gate that asks
    /// "is this a playback?" reads the same answer the load did.
    replay: Option<Replay>,
    gpu: Option<Gpu>,
    /// The one output stream for the whole app lifetime. Opened lazily on the first frame (or the
    /// first chart/preview load) and kept across Play, Result and Select; stage changes clear the
    /// affected id namespace instead of tearing the stream down.
    audio: Option<AudioEngine>,
    /// What the shared stream actually opened as, plus any downgrades taken to get there.
    audio_report: Option<AudioOpenReport>,
    /// Voice budget the shared stream was opened with, shown next to the live voice count.
    audio_max_voices: usize,
    /// The options the current stream actually opened with, retried first when a reopen fails.
    audio_opened_with: Option<AudioOptions>,
    /// Set once an open attempt failed, so the device is not probed again every chart and every
    /// preview. Cleared when a settings change asks for a reopen.
    audio_failed: bool,
    /// When the pending audio-setting change is due to be applied, armed by the settings screen and
    /// debounced by [`AUDIO_REOPEN_DEBOUNCE`].
    audio_reopen_at: Option<Instant>,
    /// `#VOLWAV` of the loaded chart as a gain. Kept so a stream reopened mid-session is handed the
    /// chart's own level again instead of the neutral default.
    chart_gain: f32,
    /// Peak-held frame period the sound scheduler books ahead by, in µs.
    schedule_poll_us: i64,
    skin: Skin,
    skin_cfg: SkinConfig,
    /// Result-screen judge colours/labels resolved from the active skin, rebuilt with it.
    result_palette: ResultPalette,
    server: Arc<dyn ScoreServer>,
    server_connected: Arc<AtomicBool>,
    /// Stops the connection probe of the previous server when the server is rebuilt.
    server_probe_stop: Arc<AtomicBool>,
    /// The signed-in IR identity, restored from the settings file and driven by the NETWORK tab.
    session: AccountSession,
    /// Password held only between the PASSWORD row and the LOGIN/REGISTER that consumes it. Never
    /// written to disk, never logged, never rendered.
    password: String,
    /// Last network result or error, shown under the NETWORK rows.
    net_status: String,
    /// Validate a restored token with `whoami` on the first frame.
    startup_whoami: bool,
    auth_rx: Option<(AuthAction, Receiver<Result<AuthResponse, IrError>>)>,
    whoami_rx: Option<Receiver<Result<String, IrError>>>,
    sync_upload_rx: Option<Receiver<Result<SettingsPutResult, IrError>>>,
    sync_download_rx: Option<Receiver<Result<SettingsBlob, IrError>>>,
    /// Read-back a `204` upload still forces, purely to learn the stamp such a server stored.
    sync_base_rx: Option<Receiver<Result<SettingsBlob, IrError>>>,
    /// The optimistic lock the next settings upload sends.
    sync_lock: SyncLock,
    rivals_rx: Option<Receiver<Result<Vec<PlayerProfile>, IrError>>>,
    /// IR ranking panel caches: the per-chart board, the generation that lets a stale fetch be
    /// dropped, and the fetch in flight. They outlive the panel so reopening it is instant.
    ranking_cache: RankingCache,
    ranking_generation: u64,
    ranking_requested: Option<String>,
    ranking_rx: Option<Receiver<Result<RankingFetch, IrError>>>,
    replay_download_rx: Option<Receiver<Result<ReplayData, IrError>>>,
    /// Chart path and md5 a downloading replay will be played against.
    replay_download_target: Option<(String, String)>,
    submit_rx: Option<Receiver<SubmitOutcome>>,
    /// What the result screen reports about this run's submission. Kept here because the request
    /// outlives the screen that shows it.
    ir_status: IrStatus,
    clock: Instant,
    anchor_us: i64,
    /// Set once the audio output stream is found dead: the song position the audio clock last
    /// reported and the instant that was noticed, so `song_us` continues on the wall clock.
    audio_dead_at: std::cell::Cell<Option<(i64, Instant)>>,
    /// Last value [`AppShared::song_us`] returned, so the interpolated clock can never step
    /// backwards between two readings inside one frame.
    song_us_last: std::cell::Cell<i64>,
    /// Ring of recent input timings (interpolated vs quantised clock, judgement error).
    timing: TimingProbe,
    /// Periodic stability log, active only when `RBMS_SOAK_LOG` names a path.
    soak: SoakLogger,
    /// Set once a soak row failed to write, so the failure is reported once rather than per row.
    soak_failed: bool,
    scores: ScoreBook,
    scores_path: PathBuf,
    /// The charts the player has starred, and where they are kept.
    favorites: Favorites,
    favorites_path: PathBuf,
    /// The messages waiting to be shown over whichever screen is up, refilled every frame from the
    /// process-wide [`notify`] bus.
    toasts: ToastQueue,
    /// The option panel the browser puts over itself, which is offered every key before the screen
    /// underneath sees it.
    options: app_options::OptionsOverlay,
    cursor: (f32, f32),
    hot: Vec<(Rect, Hot)>,
    last_frame: Instant,
    fps: f32,
    frame_count: u64,
    ram_mb: f32,
    /// SHA-256 of this binary, computed once at startup, submitted for build integrity (F8).
    build_sha256: Option<String>,
}

impl App {
    /// Build the app from the loaded configuration and this run's launch overrides.
    ///
    /// A replay launch forces autoplay off for the whole session, a folder launch joins the library
    /// list (and is persisted straight away, so the next launch finds it), and a `--table` source is
    /// added for this run only.
    fn new(input: String, mut config: Config, launch: LaunchOptions, settings_path: PathBuf) -> Self {
        let replay = launch.replay_path.as_ref().and_then(|p| match Replay::load(Path::new(p)) {
            Ok(r) => {
                println!("replay: {} on {}", p, r.chart_path);
                Some(r)
            }
            Err(e) => {
                notify(Level::Error, format!("replay load failed: {e}"));
                None
            }
        });
        if replay.is_some() {
            config.play.autoplay = false;
        }

        if config.library.folders.is_empty()
            && let Some(f) = config.library.songs_folder.clone().filter(|s| !s.trim().is_empty())
        {
            config.library.folders.push(f);
        }

        let p = Path::new(&input);
        let is_dir = replay.is_none() && p.is_dir();
        if is_dir && !config.library.folders.iter().any(|f| f == &input) {
            config.library.folders.push(input.clone());
            save_config(&config, &settings_path);
        }

        let scores_path = settings_path.parent().map(|d| d.join("scores.ron")).unwrap_or_else(|| PathBuf::from("scores.ron"));
        let scores = ScoreBook::load(&scores_path);
        let favorites_path = favorites_path(&settings_path);
        let favorites = Favorites::load(&favorites_path);
        if let Some(url) = &launch.table_url
            && !config.library.tables.iter().any(|t| &t.location == url)
        {
            config.library.tables.push(TableSource { name: String::new(), location: url.clone() });
        }

        let library = Library::default();
        let table_names: Vec<String> = Vec::new();
        let table_levels: Vec<TableLevels> = Vec::new();
        let (stage, chart_path, launch_chart) = if let Some(rp) = &replay {
            (Stage::Select(Box::new(SelectState::new())), rp.chart_path.clone(), true)
        } else if p.is_file() {
            (Stage::Select(Box::new(SelectState::new())), input, true)
        } else if config.library.folders.is_empty() {
            (Stage::Select(Box::new(SelectState::new())), String::new(), false)
        } else {
            let stage = Stage::Loading(LoadingState::scan(config.library.folders.clone(), config.library.tables.clone()));
            (stage, String::new(), false)
        };

        let session = AccountSession::restored(config.network.ir_token.clone(), config.network.ir_login_id.clone());
        let startup_whoami = session.is_logged_in();
        let built = build_server(&config, session.token().map(str::to_string));

        let keyconfig_path = launch.keyconfig_path.clone().map(PathBuf::from).unwrap_or_else(|| config_dir().join("keyconfig.ron"));
        let keyconfig = KeyConfig::load(&keyconfig_path);

        let mut app = App {
            stage,
            suspended: Vec::new(),
            launch_chart,
            startup_error: None,
            shared: AppShared {
                chart_path,
                config,
                launch,
                mode: MODE,
                library,
                active_keys: keyconfig.lane_keys(MODE),
                active_reverse_keys: keyconfig.scratch_reverse_keys(MODE),
                table_names,
                table_levels,
                select_view: SelectView::Root,
                select_items: Vec::new(),
                search: String::new(),
                searching: false,
                sel: 0,
                keyconfig,
                keyconfig_path,
                settings_path,
                replay,
                kc_edit_mode: MODE,
                gpu: None,
                audio: None,
                audio_report: None,
                audio_max_voices: rbms_audio::DEFAULT_MAX_VOICES,
                audio_opened_with: None,
                audio_failed: false,
                audio_reopen_at: None,
                chart_gain: rbms_chart::chart_gain(rbms_chart::VOLWAV_DEFAULT_PERCENT),
                schedule_poll_us: 0,
                skin: Skin::default_for(MODE, CW as f32, CH as f32),
                skin_cfg: SkinConfig::default(),
                result_palette: ResultPalette::from_skin(&SkinConfig::default()),
                server: built.server,
                server_connected: built.connected,
                server_probe_stop: built.probe_stop,
                session,
                password: String::new(),
                net_status: String::new(),
                startup_whoami,
                auth_rx: None,
                whoami_rx: None,
                sync_upload_rx: None,
                sync_download_rx: None,
                sync_base_rx: None,
                sync_lock: SyncLock::default(),
                rivals_rx: None,
                ranking_cache: RankingCache::new(RANKING_CACHE_CAPACITY),
                ranking_generation: 0,
                ranking_requested: None,
                ranking_rx: None,
                replay_download_rx: None,
                replay_download_target: None,
                submit_rx: None,
                ir_status: IrStatus::Off,
                clock: Instant::now(),
                anchor_us: 0,
                audio_dead_at: std::cell::Cell::new(None),
                song_us_last: std::cell::Cell::new(0),
                timing: TimingProbe::default(),
                soak: SoakLogger::from_env(),
                soak_failed: false,
                scores,
                scores_path,
                favorites,
                favorites_path,
                toasts: ToastQueue::default(),
                options: app_options::OptionsOverlay::default(),
                select_gen: 0,
                cursor: (0.0, 0.0),
                hot: Vec::new(),
                last_frame: Instant::now(),
                fps: 0.0,
                frame_count: 0,
                ram_mb: 0.0,
                build_sha256: compute_build_hash(),
            },
        };
        app.shared.rebuild_select_items();
        app
    }
}

/// How often the resident-set reading behind the debug overlay and the soak log is refreshed.
const RAM_SAMPLE_FRAMES: u64 = 15;

/// Width of the debug overlay panel, sized for its widest audio telemetry line.
const DEBUG_PANEL_W: f32 = 380.0;

/// Line height of the debug overlay.
const DEBUG_LINE_H: f32 = 16.0;

impl App {
    /// Apply a screen change: the leaving screen's `on_exit`, the swap, then the arriving screen's
    /// `on_enter`. Every stage change in the app goes through here, so the order of those three
    /// steps — which the audio engine depends on — is stated once.
    fn apply(&mut self, transition: Transition, event_loop: &ActiveEventLoop) {
        if matches!(transition, Transition::Quit) {
            event_loop.exit();
            return;
        }
        self.switch(transition);
    }

    /// Everything a transition does apart from closing the window, so the screen-history rules can
    /// be exercised without an event loop.
    fn switch(&mut self, transition: Transition) {
        let suspend = matches!(transition, Transition::Open(_));
        let next = match transition {
            Transition::Stay | Transition::Quit => return,
            Transition::Open(next) | Transition::To(next) => next,
            Transition::Back => self.suspended.pop().unwrap_or_else(|| Stage::Select(Box::new(SelectState::new()))),
        };
        let now = Instant::now();
        if !app_options::opens_over(next.id()) {
            app_options::close(&mut self.shared);
        }
        self.stage.on_exit(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 });
        let previous = std::mem::replace(&mut self.stage, next);
        if suspend {
            self.suspended.push(previous);
        }
        self.stage.on_enter(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 });
    }

    /// One frame: the shared bookkeeping every screen needs, the screen's own update, then the
    /// screen's own draw with the app-wide overlays on top.
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.shared.last_frame).as_secs_f32();
        self.shared.last_frame = now;
        if dt > 0.0 {
            let inst = 1.0 / dt;
            self.shared.fps = if self.shared.fps <= 0.0 { inst } else { self.shared.fps * 0.9 + inst * 0.1 };
        }
        self.shared.frame_count = self.shared.frame_count.wrapping_add(1);
        self.shared.soak.record_frame(dt);
        self.shared.toasts.pump(now);
        if let Some(audio) = self.shared.audio.as_mut() {
            audio.collect_retired();
        }
        self.shared.schedule_poll_us = schedule_poll_interval_us(self.shared.schedule_poll_us, (dt as f64 * 1_000_000.0) as i64);
        let stage = self.stage.id();
        let keysounds_decoding = matches!(&self.stage, Stage::Loading(loading) if loading.keysounds_pending());
        self.shared.poll_network();
        self.shared.poll_ir_jobs(stage);
        self.shared.poll_audio_reopen(now, stage, keysounds_decoding);
        if (self.shared.config.display.debug || self.shared.soak.enabled())
            && self.shared.frame_count.is_multiple_of(RAM_SAMPLE_FRAMES)
            && let Some(u) = memory_stats::memory_stats()
        {
            self.shared.ram_mb = u.physical_mem as f32 / (1024.0 * 1024.0);
        }
        if stage != StageId::Play {
            self.shared.poll_audio_clock();
        }

        let transition = self.stage.update(&mut FrameCtx { shared: &mut self.shared, now, dt });
        self.apply(transition, event_loop);

        if self.shared.soak.due(now) {
            self.shared.write_soak_row(now, self.stage.id());
        }
        let Some(mut gpu) = self.shared.gpu.take() else {
            return;
        };
        self.shared.hot.clear();
        {
            let mut canvas = Canvas::Window(&mut gpu);
            let mut ctx = FrameCtx { shared: &mut self.shared, now, dt };
            self.stage.draw(&mut ctx, &mut canvas);
            App::draw_overlays(&self.stage, &mut ctx, &mut canvas);
        }
        gpu.render();
        self.shared.gpu = Some(gpu);
    }

    /// The overlays drawn on top of every screen: the option panel, the transient messages, the
    /// server connection dot and the debug panel.
    ///
    /// The window's aspect is settled here rather than when a chart loads, so the DISPLAY tab's
    /// LETTERBOX row takes on the very next frame instead of waiting for the next load.
    fn draw_overlays(stage: &Stage, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        crate::gpu::apply_letterbox(canvas, ctx.shared.config.display.letterbox);
        app_options::draw(stage.id(), ctx, canvas);
        toast::draw(&ctx.shared.toasts, canvas);
        if ctx.shared.config.network.server_url.is_some() {
            let connected = ctx.shared.server_connected.load(Ordering::Relaxed);
            canvas.fill_rect(Rect::new(CW as f32 - 22.0, 10.0, 10.0, 10.0), if connected { Color::GREEN } else { Color::RED });
        }
        if !ctx.shared.config.display.debug {
            return;
        }
        let frame_ms = if ctx.shared.fps > 0.0 { 1000.0 / ctx.shared.fps } else { 0.0 };
        let mut lines = vec![
            "DEBUG".to_string(),
            format!("FPS {:.0}  ({:.1} MS)", ctx.shared.fps, frame_ms),
            format!("RAM {:.1} MB", ctx.shared.ram_mb),
            format!("QUADS {}  STAGE {:?}", canvas.quad_count(), stage),
        ];
        lines.extend(stage.debug_lines(ctx));
        lines.extend(ctx.shared.debug_audio_lines(ctx.shared.anchor_us, stage.id() == StageId::Play));
        let (layouts, runs) = rbms_render::cache_stats();
        lines.push(format!("FONT {}/{}  RUNS {}/{}", layouts, rbms_render::LAYOUT_CACHE_LIMIT, runs, rbms_render::RUN_CACHE_LIMIT));
        let ph = lines.len() as f32 * DEBUG_LINE_H + 12.0;
        canvas.fill_rect(Rect::new(6.0, 6.0, DEBUG_PANEL_W, ph), Color { r: 0, g: 0, b: 0, a: 180 });
        for (i, l) in lines.iter().enumerate() {
            let col = if i == 0 { Color::YELLOW } else { Color::rgb(120, 240, 140) };
            draw_text(canvas, 14.0, 12.0 + i as f32 * DEBUG_LINE_H, 1.2, col, l);
        }
    }
}

impl App {
    /// Record why the app cannot start and ask the event loop to wind down. [`run`] turns this into
    /// a message and a failing exit code, so a machine that cannot open a window or find a usable
    /// graphics adapter is told so rather than shown a panic.
    fn fail_startup(&mut self, event_loop: &ActiveEventLoop, reason: String) {
        self.startup_error = Some(reason);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    /// Bring up the window and the GPU backend, then load a chart or replay named on the command
    /// line: the keysound-loading bar is shown, or play starts at once when there is nothing to
    /// decode. A failed load exits.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shared.gpu.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title("rbms").with_inner_size(winit::dpi::LogicalSize::new(CW, CH));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(e) => return self.fail_startup(event_loop, format!("cannot open a window: {e}")),
        };
        match Gpu::new(window.clone()) {
            Ok(gpu) => self.shared.gpu = Some(gpu),
            Err(e) => return self.fail_startup(event_loop, e.to_string()),
        }
        self.shared.ensure_audio();
        if self.launch_chart {
            self.launch_chart = false;
            match self.shared.load() {
                Some(loaded) => {
                    let next = self.shared.enter_loaded_chart(loaded);
                    self.apply(Transition::To(next), event_loop);
                }
                None => {
                    event_loop.exit();
                    return;
                }
            }
        }
        window.request_redraw();
    }

    /// Translate one winit event into what the current screen understands. A moved cursor is mapped
    /// onto the fixed 1280x720 logical space the UI is laid out in, which the surface stretches
    /// across the whole window.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.shared.gpu.as_mut() {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(gpu) = self.shared.gpu.as_ref() {
                    self.shared.cursor = gpu.logical_from_physical(position.x as f32, position.y as f32);
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let now = Instant::now();
                let at = self.shared.cursor;
                let transition = self.stage.handle_mouse(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 }, at);
                self.apply(transition, event_loop);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let key = KeyInput {
                    code,
                    pressed: event.state == ElementState::Pressed && !event.repeat,
                    released: event.state == ElementState::Released,
                    text: event.text.as_deref(),
                };
                let now = Instant::now();
                let transition = self.stage.handle_key(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 }, key);
                self.apply(transition, event_loop);
            }
            WindowEvent::RedrawRequested => {
                self.frame(event_loop);
                if let Some(gpu) = self.shared.gpu.as_ref() {
                    gpu.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// The process exit code once the event loop has wound down: a run that never got a window or a
/// graphics adapter reports why and fails, so a launcher or a CI job can tell the two apart.
fn exit_code(startup_error: Option<&str>) -> ExitCode {
    match startup_error {
        Some(reason) => {
            notify(Level::Error, reason);
            ExitCode::FAILURE
        }
        None => ExitCode::SUCCESS,
    }
}

/// Write the configuration out, reporting a failed write rather than losing it silently. Every
/// save in the app goes through here, so the whole document is written as one file.
fn save_config(config: &Config, path: &Path) {
    if let Err(e) = rbms_config::save(config, path) {
        notify(Level::Error, format!("settings save failed ({}): {e}", path.display()));
    }
}

/// Resolve `~/.config/rbms`, the per-user config dir holding settings/keyconfig/scores/theme/
/// replays.
fn config_dir() -> PathBuf {
    config_dir_from(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

/// Pure config-dir resolution (testable without touching process env): prefer `HOME` (Unix/macOS),
/// then `USERPROFILE` (Windows, where `HOME` is usually unset), then the current dir. Without the
/// `USERPROFILE` fallback all config landed in the cwd on Windows instead of the user profile.
fn config_dir_from(home: Option<std::ffi::OsString>, userprofile: Option<std::ffi::OsString>) -> PathBuf {
    home.or(userprofile).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(".config/rbms")
}

/// Run the player: read the persisted configuration, fold the command line over it, apply the font
/// and theme, then hand the window over to winit. `args` is the raw process argument list, program
/// name included. A failure to start the event loop is reported to the user rather than panicking.
///
/// A configuration written by a newer build is left untouched: this run falls back to the defaults
/// rather than overwriting settings it cannot read.
pub fn run(args: impl Iterator<Item = String>) -> ExitCode {
    let settings_path = config_dir().join("settings.ron");
    let mut config = match rbms_config::load(&settings_path) {
        Ok(outcome) => {
            if let Some(from) = outcome.migrated_from {
                println!("settings migrated from schema version {from}");
                save_config(&outcome.config, &settings_path);
            }
            outcome.config
        }
        Err(e) => {
            notify(Level::Warn, format!("settings not loaded ({e}); running on defaults and leaving the file alone"));
            Config::default()
        }
    };

    let mut launch = LaunchOptions::default();
    let mut chart: Option<String> = None;
    let mut args = args.skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--interactive" => config.play.autoplay = false,
            "--auto" => config.play.autoplay = true,
            "--sc-left" => config.play.scratch_left = true,
            "--sc-auto" => config.play.scratch_auto = true,
            "--lift" => {
                if let Some(v) = args.next().and_then(|v| v.parse::<f32>().ok()) {
                    config.play.lift = v.clamp(rbms_config::LANE_SHADE_MIN, rbms_config::LANE_SHADE_MAX);
                }
            }
            "--hispeed" => {
                if let Some(v) = args.next().and_then(|v| v.parse::<f64>().ok()) {
                    config.play.hispeed = v.clamp(rbms_config::HISPEED_MIN, rbms_config::HISPEED_MAX);
                }
            }
            "--gauge" => {
                if let Some(name) = args.next() {
                    config.play.gauge = gauge_from_name(&name);
                }
            }
            "--skin" => launch.skin_path = args.next(),
            "--font" => config.display.font_path = args.next(),
            "--server" => config.network.server_url = args.next(),
            "--table" => launch.table_url = args.next(),
            "--keyconfig" => launch.keyconfig_path = args.next(),
            "--replay" => launch.replay_path = args.next(),
            "--timing-csv" => launch.timing_csv = args.next(),
            "--player" => {
                if let Some(id) = args.next() {
                    config.network.player_id = id;
                }
            }
            "--keys" => {
                if let Some(spec) = args.next() {
                    let parsed: Vec<(KeyCode, usize)> = spec.split(',').enumerate().filter_map(|(i, n)| key_from_name(n.trim()).map(|k| (k, i))).collect();
                    if !parsed.is_empty() {
                        launch.keys_override = Some(parsed);
                    }
                }
            }
            other if chart.is_none() && !other.starts_with("--") => chart = Some(other.to_string()),
            _ => {}
        }
    }
    let remembered =
        config.library.songs_folder.clone().filter(|s| !s.trim().is_empty()).or_else(|| std::env::var("RBMS_SONGS").ok().filter(|s| !s.trim().is_empty()));
    let chart = match chart {
        Some(c) => c,
        None if launch.replay_path.is_some() => String::new(),
        None => remembered.unwrap_or_default(),
    };

    if let Some(fp) = config.display.font_path.clone() {
        match std::fs::read(&fp) {
            Ok(bytes) => match rbms_render::load_font(bytes) {
                Some(family) => {
                    rbms_render::set_ui_family(&family);
                    println!("font: {fp} ({family})");
                }
                None => notify(Level::Warn, format!("font load failed (no usable face): {fp}")),
            },
            Err(e) => notify(Level::Warn, format!("font not found: {fp} ({e})")),
        }
    }

    load_theme(&settings_path);

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            notify(Level::Error, format!("cannot open a window: {e}"));
            return ExitCode::FAILURE;
        }
    };
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(chart, config, launch, settings_path);
    if let Err(e) = event_loop.run_app(&mut app) {
        notify(Level::Error, format!("the window event loop stopped: {e}"));
        return ExitCode::FAILURE;
    }
    exit_code(app.startup_error.as_deref())
}

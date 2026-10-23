//! The rbms player application: window, stages, audio wiring and the whole GUI.
//!
//! The crate root is the wiring file — [`run`] is the entry point the thin `main` binary calls, and
//! everything else lives in the modules below. Building the app as a library (with the binary a
//! three-line shim) is what lets the integration tests under `tests/` reach these types.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rbms_audio::{AudioEngine, AudioOpenReport, AudioOptions, Bus, IdNamespace};
use rbms_chart::default_total_for_mode;
use rbms_chart::shuffle::NoteOption;
use rbms_chart::to_model;
use rbms_config::{
    Config, HISPEED_MAX, HISPEED_MIN, LANE_SHADE_MAX, LANE_SHADE_MIN, LANE_SHADE_STEP, PLAY_ESCAPE_DOUBLE_MS, PLAY_ESCAPE_HOLD_MS, PlayEscape, SortMode,
    TableSource, algorithm_token, gauge_auto_shift_token, gauge_from_name, gauge_set_token, gauge_token, ln_mode_token,
};
#[cfg(test)]
use rbms_ir::ChartId;
use rbms_ir::mapping::{LIGHT_ASSIST, NO_ASSIST, assist_flags, assist_level, combo_breaks, ir_clear, ir_gauge, ir_random};
use rbms_ir::{
    API_VERSION, IrError, JudgeBreakdown, PlayOptions, PlayerId, PlayerProfile, ReplayData, ScoreServer, ScoreSubmission, SettingsBlob, SettingsPutResult,
    SubmitOutcome,
};
use rbms_judge::gauge::{AssistLevel, assist_downgrade};
use rbms_judge::{ClearType, clear_type_from_id, clear_type_id};
use rbms_library::{ChartDetail, Library, compute_chart_detail};
use rbms_model::Mode;
use rbms_play::{ANALYSIS_SEEK_STEP_US, NullSink, PlaySession, Player, ScratchDir, SessionClock, SessionOptions};
use rbms_render::skin_render::frame::{BarDistribution, BarKind, BarTrophy};
use rbms_render::{
    Color, CoverState, DensityView, DetailView, HudView, PlayfieldView, RANK_BANDS, RecordRowView, RecordsView, Rect, Renderer, ResultPalette, ResultView,
    SelectDetail, SelectHot, SelectModal, SelectRow, SelectView as SelectScene, Skin, SkinConfig, StatCell, cover_rect, dj_rank, draw_text, draw_text_centered,
    draw_text_right, ex_delta_label, render_key_bomb, render_lane_cover, render_playfield_view, render_result_with_palette, render_select, text_width, theme,
};
pub(crate) use rbms_skin::loader::{SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_RESULT, mode_skin_type};
use rbms_skin::timer::TimerState;
use rbms_store::{Replay, ReplayJudge, SCORE_LN_MODE_FROM_CHART, ScoreBook, ScoreRecord};

use sha2::{Digest, Sha256};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

mod app_input;
mod app_network;
mod app_options;
mod app_play;
mod app_ranking;
mod app_result;
mod assets;
mod command_line;
mod course_ir;
mod course_ui;
mod dialog;
mod favorites;
mod format;
mod gamepad;
mod ghost;
mod gpu;
mod input_debounce;
mod ir_ext;
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
mod library;
#[cfg(test)]
mod main_tests;
mod notify;
mod number_row;
mod play_sink;
mod play_trace;
mod pointer;
mod practice;
mod record_sync;
mod save_worker;
mod scene_clock;
mod scoredb_store;
mod select_tree;
mod settings_ui;
mod settings_view;
pub mod skin_host;
mod skin_preview;
mod skin_screen;
mod skin_select;
mod stage;
mod syssound;
mod tablesrc;
pub mod target;
mod textedit;
mod timing;
mod toast;
mod trace_backfill;
mod user_folders;
mod window_mode;
use app_network::build_server;
use app_play::schedule_poll_interval_us;
pub(crate) use assets::{DecodedImage, bundled_skin, decode_bga_image, install_default_pack, keysound_jobs, load_theme, resolve_file, spawn_keysound_decode};
use course_ir::{CourseAssist, UNRELEASED_COURSE_REASON, build_course_submission};
use course_ui::{CourseEntry, CourseList, CourseOverrides, SelectTab, courses_dir, library_index, stage_label};
use favorites::{Favorites, favorites_path};
use format::{
    difficulty_color, difficulty_name, fmt_datetime, fmt_duration, gauge_name, mode_color, mode_short, rank_label, rule_version_mark, rule_version_note,
};
use gamepad::{PadEvent, PadState};
use gpu::Gpu;
use ir_ext::{MultiIr, ProfileResult, spawn_submit_all, submit_summary};
use ir_outcome::{IR_RESULT_LINE_H, IR_RESULT_SCALE, IR_RESULT_X, IR_RESULT_Y, IrStatus, ir_line_color};
use ir_ranking::{RANKING_CACHE_CAPACITY, RankingCache, RankingFetch};
use ir_ranking_view::render_ranking_panel;
use ir_session::AccountSession;
use ir_sync::SyncLock;
use judge_setup::{is_custom_judge, run_judge_setup, run_lntype};
use keyconfig::{ControlAction, KeyConfig, key_name};
use notify::{Level, notify};
use play_sink::PlayAudioSink;
use pointer::{HeldButtons, PointerInput, is_tracked};
use settings_view::{SettingsHot, render_settings};
use skin_screen::{SkinScene, SkinScreens};
use skin_select::SkinLibrary;
use stage::{Canvas, CourseResultState, FrameCtx, KeyInput, LoadingState, SelectState, Stage, StageId, Transition};
use syssound::{SYSTEM_SOUND_GAIN, SystemSound, SystemSoundSet};
use tablesrc::TableLevels;
use textedit::{TextEdit, edit_key};
use timing::{SoakLogger, SoakSnapshot, TIMING_CSV_ENV, TimingProbe, TimingSample, env_path, us_to_millis};
use toast::ToastQueue;
use window_mode::{AppWindow, window_attributes};

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

/// The argument macOS can add when Finder opens the app bundle (`-psn_0_<number>`, the process
/// serial number). It names no chart, so the launch reads it as nothing.
const PROCESS_SERIAL_ARG_PREFIX: &str = "-psn_";

/// Whether a launch argument is one the system added rather than one the player typed.
fn is_system_launch_arg(arg: &str) -> bool {
    arg.starts_with(PROCESS_SERIAL_ARG_PREFIX)
}
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
    /// `--skin`: a skin RON to load instead of the configured bundled skin. It lays out the
    /// built-in note field, so a run started with it draws its play screens with that field
    /// whatever skin document is chosen for them.
    skin_path: Option<String>,
    /// `--table`: a difficulty table added to the library for this run. It is a source like any
    /// the player registered: the scan at startup reads it from the copy kept of it, and when it
    /// has none the song list fetches it once it is up. Nothing waits for its address.
    table_url: Option<String>,
    /// `--keyconfig`: a key config file to use instead of the one in the config directory.
    keyconfig_path: Option<String>,
    /// `--replay`: a saved replay to play back instead of an interactive run.
    replay_path: Option<String>,
    /// `--timing-csv <path>`: where the per-input timing ring is dumped when the result screen is
    /// entered. `None` falls back to the `RBMS_TIMING_CSV` environment variable.
    timing_csv: Option<String>,
    /// `RBMS_SKIN_PACK`: a skin pack folder drawn for this run, whatever the settings file names.
    skin_pack: Option<PathBuf>,
    /// Whether the platform's controller layer is opened. A run of the player opens it; nothing
    /// else has a device to read, and each layer that is opened keeps threads of its own alive.
    controllers: bool,
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

/// Where the song database lives, next to the settings document it is keyed off.
fn songdb_path(settings_path: &Path) -> PathBuf {
    settings_path.parent().map(|d| d.join(crate::library::SONGDB_FILE)).unwrap_or_else(|| PathBuf::from(crate::library::SONGDB_FILE))
}

/// Where system sounds are read from: the folder the player configured, with a blank setting read as
/// "not configured", and otherwise the set the first run installed beside the settings file.
fn sound_folder_path(settings_path: &Path, config: &Config) -> Option<PathBuf> {
    match config.audio.sound_folder.as_deref().map(str::trim).filter(|folder| !folder.is_empty()) {
        Some(folder) => Some(PathBuf::from(folder)),
        None => crate::assets::default_sound_folder(settings_path),
    }
}

/// Gauge a course stage starts on before anything has been played, read off the `start` row of the
/// gauge table `set` so the number the browser shows is the one the run will use.
fn course_start_gauge(set: rbms_judge::gauge_tables::GaugeSetId, start: rbms_judge::gauge::GaugeIndex) -> f32 {
    rbms_judge::builtin_gauge_tables().get(set.data_key()).map_or(0.0, |table| table.at(start).init)
}

/// Begin a course: rewrite the play settings the constraints narrow, remember the originals the
/// course puts back when it ends ([`end_course`]), and hand the first stage to LOADING.
///
/// A course still standing from before is ended first, as one left between two stages is
/// ([`crate::app_result::abandon_course`]): what was played of it by hand is kept and sent as a
/// FAILED course, and the settings it narrowed are put back before they are taken as the player's
/// own.
///
/// The course opens on the course gauge the GAUGE setting maps to, read off the table the COURSE
/// GAUGE setting picks for a seven-key chart: the first chart's mode is not known until it loads,
/// and every table's course gauges open at the same value.
fn start_course(shared: &mut AppShared, entry: &CourseEntry) -> Transition {
    if shared.course_run.is_some() || shared.course_settings_backup.is_some() {
        crate::app_result::abandon_course(shared, crate::app_result::Sending::Now);
    }
    let overrides = CourseOverrides::of(&entry.course);
    shared.course_settings_backup = Some(shared.config.play.clone());
    overrides.apply_to(&mut shared.config.play);
    shared.course_stage_reasons.clear();
    shared.course_assist = CourseAssist::NONE;
    let set = shared.config.judge.course_gauge.gauge_set(&Mode::BEAT_7K);
    let start = set.course_gauge_for(shared.config.play.gauge);
    shared.course_overrides = Some(overrides);
    shared.course_run = Some(rbms_course::CourseRun::new(entry.course.clone(), course_start_gauge(set, start)));
    load_course_stage(shared)
}

/// Hand the stage the run is standing on to LOADING, or give up when the library cannot supply it.
///
/// The first stage is a chart picked in the browser like any other, so it goes through the decide
/// scene when a skin draws one. The stages after it follow straight on from a result screen, as the
/// reference's do (`MusicResult.render`). A stage the library does not hold ends the course there,
/// keeping what was played of it ([`crate::app_result::abandon_course`]).
fn load_course_stage(shared: &mut AppShared) -> Transition {
    let Some(chart) = shared.course_run.as_ref().and_then(|run| run.current_chart()).cloned() else {
        return end_course(shared);
    };
    let Some(index) = library_index(&shared.library, &chart) else {
        notify(Level::Warn, format!("course stage not in the library: {}", chart.title));
        return crate::app_result::abandon_course(shared, crate::app_result::Sending::Now);
    };
    shared.release_play_audio();
    let opens_the_course = shared.course_run.as_ref().is_some_and(|run| run.index == 0);
    Transition::To(if opens_the_course { shared.decided_song_stage(index) } else { Stage::Loading(LoadingState::song(index)) })
}

/// Put back the player's own settings the course narrowed and forget the run. Called on the course
/// result screen's way out and on an abandoned course alike; what is kept of an abandoned course is
/// kept before it ([`crate::app_result::abandon_course`]).
///
/// Only what the constraints narrowed is put back ([`CourseOverrides::put_back_into`]): a hi-speed or
/// lane shade the player set during a course without NO SPEED stays theirs.
fn end_course(shared: &mut AppShared) -> Transition {
    if let (Some(own), Some(overrides)) = (shared.course_settings_backup.take(), shared.course_overrides.as_ref()) {
        overrides.put_back_into(&own, &mut shared.config.play);
    }
    shared.course_run = None;
    shared.course_overrides = None;
    shared.course_stage_reasons.clear();
    shared.course_assist = CourseAssist::NONE;
    Transition::Back
}

/// Why this run's score must not be sent to the IR, or `None` when it may be submitted: only a run
/// the player did not play by hand -- an autoplay, a replay being watched or a practice slice -- is
/// held back (`MusicResult.java:82` sends only a PLAY).
///
/// Every run the player played is sent, assisted or not (the player's decision of 2026-10-14: "no
/// record goes unsaved"). The reference sends none that cleared its `score` flag; here the assist
/// travels with the submission instead ([`assist_flags`], `options.scratch_auto`) and what the
/// server makes of it is the server's.
fn ir_submission_block_reason(autoplay: bool, replay: bool, practice: bool) -> Option<&'static str> {
    if autoplay {
        return Some("autoplay");
    }
    if replay {
        return Some("replay playback");
    }
    crate::practice::practice_block_reason(practice)
}

/// Whether this run may update the stored bests (EX / judgements / combo / BP): a run the player
/// played by hand, unless its judge window or long-note margin was widened past the chart's
/// (`BMSPlayer.java:208-214` clears the `score` flag for it, `ScoreData.java:548,566,572,578` gate
/// the bests on that flag). A SCRATCH AUTO run sets bests like any other: its turntable notes are
/// not in the chart it was judged on, so it is scored out of the notes it was played on.
fn updates_score(autoplay: bool, replay: bool, custom_judge: bool, practice: bool) -> bool {
    ir_submission_block_reason(autoplay, replay, practice).is_none() && !custom_judge
}

/// The TOTAL a run is judged on in place of the chart's: the one a replay being watched recorded, or
/// the one the player set ([`rbms_config::PlayConfig::total_override`]). [`rbms_store::REPLAY_CHART_TOTAL`]
/// or less overrides nothing. A replay recorded before the TOTAL was written down was played on the
/// chart's own, so a TOTAL set now never leaks into its playback.
fn run_total_override(config: &Config, replay: Option<&Replay>) -> f64 {
    replay.map_or(config.play.total_override, |replay| replay.judge.total_override)
}

/// Whether a TOTAL override moves the gauge at all.
fn overrides_total(total_override: f64) -> bool {
    total_override > rbms_store::REPLAY_CHART_TOTAL
}

/// Whether a run was eased the way that costs it its best: judged wider than its chart asks
/// ([`is_custom_judge`], `BMSPlayer.java:208-214`), or on a TOTAL of the player's own, which moves
/// the gauge as far as they like and so the lamp with it (the player's decision of 2026-10-15). The
/// reference has no TOTAL setting; the run is treated as it treats a widened judge: an AssistEasy
/// lamp at most, kept and sent, and never a best.
fn judged_wider(setup: &rbms_play::JudgeSetup, total_override: f64) -> bool {
    is_custom_judge(setup) || overrides_total(total_override)
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
/// ranked eligibility. `None` if the executable can't be read — the server decides how to treat an
/// unknown build (e.g. ranked=false).
fn compute_build_hash() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let bytes = std::fs::read(exe).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

/// [`compute_build_hash`], worked out the first time it is asked for and remembered for the life of
/// the process. The executable is the same file however many times an [`App`] is built from it, and
/// reading the whole of it through a digest is the most expensive thing building one does.
fn build_hash() -> Option<String> {
    static BUILD_HASH: OnceLock<Option<String>> = OnceLock::new();
    BUILD_HASH.get_or_init(compute_build_hash).clone()
}

/// Client platform tag (`OS-ARCH`, e.g. `macos-aarch64`) paired with the build hash.
fn client_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// One editable row in the key-config screen.
enum KcRow {
    ModeSelect,
    Control(ControlAction),
    Lane(usize),
    /// The second key a scratch lane may be spun backwards with.
    ScratchReverse(usize),
    /// The controller the pad rows bind against, and the turntable algorithm they read it with.
    PadDevice,
    PadAnalogMode,
    PadControl(ControlAction),
    PadLane(usize),
    PadScratchReverse(usize),
}

fn kc_rows(edit_mode: Mode) -> Vec<KcRow> {
    let mut rows = vec![KcRow::ModeSelect];
    rows.extend(ControlAction::ALL.into_iter().map(KcRow::Control));
    rows.extend((0..edit_mode.key).map(KcRow::Lane));
    rows.extend((0..edit_mode.key).filter(|&lane| edit_mode.is_scratch(lane)).map(KcRow::ScratchReverse));
    rows.push(KcRow::PadDevice);
    rows.push(KcRow::PadAnalogMode);
    rows.extend(ControlAction::ALL.into_iter().map(KcRow::PadControl));
    rows.extend((0..edit_mode.key).map(KcRow::PadLane));
    rows.extend((0..edit_mode.key).filter(|&lane| edit_mode.is_scratch(lane)).map(KcRow::PadScratchReverse));
    rows
}

/// Whether a key-config row binds a controller element rather than a keyboard key.
fn is_pad_row(row: &KcRow) -> bool {
    matches!(row, KcRow::PadDevice | KcRow::PadAnalogMode | KcRow::PadControl(_) | KcRow::PadLane(_) | KcRow::PadScratchReverse(_))
}

/// One row in the select list: either a folder to descend into ([`select_tree::SelectFolder`]), or
/// a playable chart (index into `AppShared::library`).
enum SelectItem {
    Song(usize),
    Folder { label: String, target: select_tree::SelectFolder },
}

/// The best score the player holds on a chart: the EX of the run that scored highest and the most
/// that run could have scored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BarScore {
    pub(crate) ex: u32,
    pub(crate) max_ex: u32,
}

/// The pictures a chart names for the screens that show it, each as the file it would be read from.
/// A picture the chart does not name is `None`; one it names may still not be on disk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BarImages {
    /// `#STAGEFILE`.
    pub(crate) stagefile: Option<PathBuf>,
    /// `#BANNER`.
    pub(crate) banner: Option<PathBuf>,
    /// `#BACKBMP`, which only the song database remembers.
    pub(crate) backbmp: Option<PathBuf>,
}

/// What a bar that is a chart knows about its chart beyond its title.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BarChart {
    /// Where the chart is in [`AppShared::library`].
    pub(crate) index: usize,
    /// `#SUBTITLE`, which the reference writes after the title on a bar (`SongData.getFullTitle`).
    pub(crate) subtitle: String,
    pub(crate) mode: Mode,
    /// `#PLAYLEVEL` as the chart wrote it, which is what the built-in row shows.
    pub(crate) level_text: String,
    /// `#PLAYLEVEL` as a number, zero when it is not one (`SongData.getLevel`).
    pub(crate) level: i32,
    /// `#DIFFICULTY` as the chart wrote it (`SongData.getDifficulty`).
    pub(crate) difficulty: i32,
    /// What the chart is made of, as the reference's `SongData.getFeature` bits. Zero when the
    /// song database has not been asked ([`stage::select`] asks it only for a skin).
    pub(crate) features: u32,
    /// When the chart first entered the song database, in seconds, or `None` when that is not
    /// known (`SongData.getAdddate`).
    pub(crate) added_at: Option<i64>,
    pub(crate) favorite: bool,
    /// The run that scored highest, when anything has been recorded on the chart.
    pub(crate) best: Option<BarScore>,
    pub(crate) images: BarImages,
}

/// What a bar that is a course knows about its course beyond its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BarCourse {
    /// How many stages the course has.
    pub(crate) stages: usize,
    /// The constraints the course plays under and whether it is released, as the built-in row
    /// labels them.
    pub(crate) badges: Vec<&'static str>,
    /// The features of every chart of the course or-ed together, which is what the reference
    /// labels a course with. Zero for a course the library cannot supply in full.
    pub(crate) features: u32,
}

/// One bar of the browser's list, with what every screen that draws the list reads off it: the
/// reference's `Bar` with the questions a wheel and the bar properties ask already answered.
///
/// `kind` says which of the reference's bar classes it stands for. What this build has no source
/// for is left at "nothing": no rival is ever picked, so `rival_lamp` is always `None`, and a
/// course's own clear and medal are not kept anywhere, so a course never has a `lamp` or a
/// `trophy`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectBar {
    pub(crate) kind: BarKind,
    /// What the bar is called. A chart's is its `#TITLE` alone; [`SelectBar::full_title`] is what
    /// the reference writes on the bar.
    pub(crate) title: String,
    /// The chart behind a bar that is one.
    pub(crate) chart: Option<BarChart>,
    /// The course behind a bar that is one.
    pub(crate) course: Option<BarCourse>,
    /// The best clear the player holds on the bar, as the reference numbers its `ClearType`s, or
    /// `None` when nothing was recorded on it. A folder's is the lowest clear any chart under it
    /// sits on (`DirectoryBar.getLamp`).
    pub(crate) lamp: Option<u8>,
    /// The assist grade inside a chart's `lamp` ([`rbms_store::RecordLamp::assist_grade`]), which
    /// the built-in row colours its lamp by; a skin is handed the lamp alone. Zero for no grade, and
    /// for every bar that is not a chart.
    pub(crate) assist_grade: u8,
    /// The same for the rival that is picked.
    pub(crate) rival_lamp: Option<u8>,
    /// The medal of a course (`GradeBar.getTrophy`).
    pub(crate) trophy: Option<BarTrophy>,
    /// How the charts under a bar that opens have been cleared and scored, for the bars the
    /// reference counts that for: a folder of charts and a level of a table, not a table itself.
    pub(crate) distribution: Option<BarDistribution>,
}

impl SelectBar {
    /// A bar of `kind` called `title` with nothing else known about it.
    pub(crate) fn new(kind: BarKind, title: impl Into<String>) -> SelectBar {
        SelectBar {
            kind,
            title: title.into(),
            chart: None,
            course: None,
            lamp: None,
            assist_grade: rbms_store::ASSIST_GRADE_NONE,
            rival_lamp: None,
            trophy: None,
            distribution: None,
        }
    }

    /// What the reference writes on the bar (`Bar.getTitle`): a chart's title with its subtitle
    /// after it when it has one (`SongData.getFullTitle`), and any other bar's own name.
    pub(crate) fn full_title(&self) -> String {
        match self.chart.as_ref().map(|chart| chart.subtitle.as_str()).filter(|subtitle| !subtitle.is_empty()) {
            Some(subtitle) => format!("{} {subtitle}", self.title),
            None => self.title.clone(),
        }
    }
}

/// Cache key for the assembled [`SelectScene`]: rebuild only when one of these changes, so the scene
/// is not re-allocated every frame of the continuous redraw loop. `select_gen` bumps on any list
/// rebuild (catches same-length folder swaps); `scores` length catches a freshly saved record; the
/// last flag is whether a skin draws the browser, which words some of its hints differently.
type SelectKey = (u64, usize, Option<usize>, usize, bool, bool, SelectTab, bool);

/// A clickable region recorded during rendering and hit-tested on a left-click. Immediate-mode:
/// `AppShared::hot` is rebuilt every frame for the current stage, so the layout math lives in one place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hot {
    SelectRow(usize),
    RecordRow(usize),
    SettingTab(usize),
    SettingRow(usize),
    RivalRow(usize),
    /// The button on the settings screen's tab strip that opens the play menu.
    PlayMenuLink,
    /// The button on the settings screen's tab strip that opens the skin menu.
    SkinMenuLink,
    /// A row of the skin menu's list, by its place in the whole list.
    SkinMenuRow(usize),
    /// A tab of the skin menu's strip of screens, by its place on the strip.
    SkinMenuKind(usize),
    /// A row of the play menu's list, by its place in the whole list.
    PlayMenuRow(usize),
    /// A tab of the play menu's strip of modes, by its place on the strip.
    PlayMenuMode(usize),
    /// A row of the practice panel, by its place in the panel.
    PracticeRow(usize),
    RankingRow(usize),
    ModalClose,
    ModalReplay,
    NavSearch,
    NavSort,
    NavFolders,
    NavTables,
    NavRecords,
    NavSettings,
    /// A region of a panel the application draws over a skin, which takes a press that lands on it
    /// without doing anything, so the skin beneath is not handed it.
    OverlayPanel,
    /// Everything on screen but the key guide the application draws over a skin: a press there
    /// closes it.
    GuideClose,
}

/// The running application: the state that outlives a stage change, the screen that is up, and the
/// screens it was opened over.
struct App {
    shared: AppShared,
    stage: Stage,
    /// Screens suspended by one opened on top of them; [`Transition::Back`] resumes the newest.
    /// Only ever a screen the browser (or the settings screen) opened, so it stays one or two deep.
    suspended: Vec<Suspended>,
    /// A chart or replay named on the command line, loaded once the window exists.
    launch_chart: bool,
    /// Why the app could not start, set when the window or the GPU backend could not be brought up.
    /// The event loop is asked to exit and [`run`] turns this into a failing exit code, so a machine
    /// with no usable adapter gets a message instead of a panic.
    startup_error: Option<String>,
    /// The open window and the DISPLAY rows it was last brought to; `None` until the window exists.
    window: Option<AppWindow>,
    /// What the GPU backend wakes the event loop with when the image the next frame is drawn into
    /// has come. Set by [`run`] before the loop starts; `None` in a test, which has no loop.
    frame_waker: Option<EventLoopProxy<FrameReady>>,
}

/// The event the event loop is woken with from the presenter's thread: the image the next frame is
/// drawn into has come ([`Gpu::frame_ready`]).
///
/// Frames are driven by it rather than by `RedrawRequested`, so that between frames the event loop
/// waits on nothing but the window and the presenter: a key reaches it while the display is still
/// being waited for, and is judged at the instant it came in.
#[derive(Debug, Clone, Copy)]
struct FrameReady;

/// A screen parked under the one opened over it, with the skin scene it was in, so that coming back
/// finds its timers and its clock where it left them.
struct Suspended {
    stage: Stage,
    scene: SkinScene,
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
    /// The song database the library is read back from. `None` when it could not be opened, in
    /// which case the run browses whatever a scan hands it and stores nothing.
    song_db: Option<rbms_library::songdb::SongDb>,
    active_keys: Vec<(KeyCode, usize)>,
    /// The second key each scratch lane may be spun with, which ends a charge note the forward key
    /// is holding (`JudgeManager.java:358-372`). Empty for a mode with no scratch lane, and for a
    /// run whose lane bindings came from the command line.
    active_reverse_keys: Vec<(KeyCode, usize)>,
    /// The controller, when this machine has one to open. `None` is the shipped state and the
    /// state of a machine with no gilrs backend: the keyboard path is untouched either way.
    pad: Option<PadState>,
    table_names: Vec<String>,
    table_levels: Vec<TableLevels>,
    /// What the browser keeps of each table beside its name and its levels, aligned with both.
    table_extras: Vec<tablesrc::TableExtras>,
    /// The difficulty tables being fetched beside the frame loop, which whichever screen is up
    /// takes as they land. A fetch outlives the screen that started it, so it is kept here.
    table_fetches: tablesrc::TableFetches,
    /// The signed-in player's own folders of charts and the fetch of them, which like a table's
    /// outlives the screen that started it.
    user_folders: user_folders::UserFolders,
    /// The folders the browser has opened, outermost first; empty at the top of the library.
    select_path: Vec<select_tree::OpenedFolder>,
    select_items: Vec<SelectItem>,
    /// The filter `select_items` was last built through, which a list rebuilt where it stood
    /// ([`select_tree::show_the_list_again`]) is built through again.
    select_filter: stage::select::SelectFilter,
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
    /// The course being played, when one is. Every stage loads, plays and finishes through the
    /// screens a single chart uses; this is what tells them they are inside a course.
    course_run: Option<rbms_course::CourseRun>,
    /// What the course's constraint set narrows, read back by the keys the course locks out.
    course_overrides: Option<CourseOverrides>,
    /// One IR block reason per finished stage, folded into the course verdict at the end.
    course_stage_reasons: Vec<Option<String>>,
    /// The assists the finished stages were played with, taken over the course so far: what the
    /// course submission states, and whether the course may set a best.
    course_assist: CourseAssist,
    /// What the run that just ended left for a result screen a skin draws: every gauge's history,
    /// the note timings and the per-second judge tables, to the reference's definitions. One record
    /// for a single chart; one per finished stage, in order, while a course is being played. Kept
    /// by [`app_result::enter_result`], which also decides when the list starts over.
    run_records: Vec<rbms_play::PlayRecord>,
    /// The seed the next chart load lays its lanes out with, in place of a fresh one: the seed of
    /// the run a skin's result screen was asked to run again as it was. The load that follows
    /// takes it, so it shapes that one run.
    retry_seed: Option<u64>,
    /// The play settings as they stood before the course rewrote them. The ones its constraints
    /// narrowed are put back from these when it ends and are what a save made during it writes.
    course_settings_backup: Option<rbms_config::PlayOptions>,
    /// The player's own SCRATCH AUTO, gauge and lane arrangement while a watched replay has the
    /// settings hold the ones it was recorded with: written to the settings file in their place and
    /// put back when the replay ends ([`AppShared::end_replay`]).
    own_play_options: Option<app_play::OwnPlayOptions>,
    gpu: Option<Gpu>,
    /// The one output stream for the whole app lifetime. Opened lazily on the first frame (or the
    /// first chart/preview load) and kept across Play, Result and Select; stage changes clear the
    /// affected id namespace instead of tearing the stream down.
    audio: Option<AudioEngine>,
    /// What the shared stream actually opened as, plus any downgrades taken to get there.
    audio_report: Option<AudioOpenReport>,
    /// Voice budget the shared stream was opened with, shown next to the live voice count.
    audio_max_voices: usize,
    /// The system sound set read from the configured folder. Silent throughout when no folder is
    /// set, which is what a fresh install holds.
    syssound: SystemSoundSet,
    /// The sounds the skin of the running scene asked for: what was decoded, what is sounding, and
    /// the worker that decodes. Emptied whenever a scene begins ([`AppShared::end_scene_sounds`]).
    skin_sounds: skin_host::audio::SkinSounds,
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
    /// The skin documents on disk and the one each screen is drawn with, which the skin menu edits.
    /// Held here rather than on the settings screen because the document outlives the screen that
    /// chose it: every other screen draws with it.
    skins: SkinLibrary,
    /// The documents that have been compiled into drawable screens, one per screen type, and the
    /// textures they draw from. Held alongside the library because a screen's textures are let go
    /// of by the frame after its stage is left rather than by the stage itself, which is what lets
    /// the screen that follows take over the files the two share.
    skin_screens: SkinScreens,
    /// The timer table every document animates against. Which of its timers are switched, and
    /// when, is each screen's own to decide.
    skin_timers: TimerState,
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
    /// The login or register in flight. Letting go of it revokes the token it was answered with,
    /// unless the answer was taken ([`app_network::AuthInFlight`]).
    auth_rx: Option<app_network::AuthInFlight>,
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
    /// The score servers this play is submitted to, and which one the ranking panel reads.
    multi_ir: MultiIr,
    /// One client per profile, index-aligned with `multi_ir.profiles`.
    profile_servers: Vec<Arc<dyn ScoreServer>>,
    /// The extra profiles' fan-out, in flight. `server_url` itself goes through `submit_rx`.
    profile_submit_rx: Option<Receiver<Vec<ProfileResult<rbms_ir::SubmitResponse>>>>,
    /// What the result screen reports about this run's submission. Kept here because the request
    /// outlives the screen that shows it.
    ir_status: IrStatus,
    /// The play the submission in `submit_rx` was made for, so its answer is written down against
    /// that play ([`app_ranking::LiveSubmit`]).
    submit_log: Option<app_ranking::LiveSubmit>,
    /// The records sync in flight ([`record_sync`]).
    record_sync: Option<record_sync::RecordSyncRun>,
    /// Plays to sync once the running sync ends: a run left mid-song while one was going.
    record_sync_pending: Vec<i64>,
    /// Where each play stands with the score server now set, keyed by `scorelog` id, for the record
    /// screens. Read from the database when the server changes and kept up as answers come in.
    record_sync_states: std::collections::HashMap<i64, rbms_store::scoredb::IrSyncRow>,
    /// How fast a records sync sends: under the server's rate limits in the game, at once in a test.
    record_sync_pacing: record_sync::SyncPacing,
    /// The build the score server last said it runs, which a refusal it gives is written down under
    /// ([`record_sync::refusal_key`]). `None` until a sync has asked.
    server_build: Option<String>,
    /// Finished courses on their way to the main server, whose answers are still to come
    /// ([`app_ranking::CourseSubmit`]).
    course_submits: Vec<app_ranking::CourseSubmit>,
    /// The thread file writes are handed to, so a save never stalls a frame on the disk
    /// ([`save_worker`]).
    saves: save_worker::SaveWorker,
    /// The traces being made for runs stored without one ([`trace_backfill`]).
    trace_backfill: trace_backfill::TraceBackfill,
    /// The wall clock the audio fallback and the timing probe read, restarted when a run begins. The
    /// skin does not read it: that is [`AppShared::scene_clock`].
    clock: Instant,
    /// When the application started, which is what the skin's boot-time numbers count from.
    booted: Instant,
    /// How long the running scene has run, which is what the skin clock reads.
    scene_clock: scene_clock::SceneClock,
    /// Where a test has stopped the machine's side of the skin clock
    /// ([`AppShared::stop_scene_clock`]).
    #[cfg(test)]
    scene_stopped_at: Option<Instant>,
    /// Where a test has stopped the machine's side of the song clock read without a sound device
    /// ([`AppShared::stop_clocks_at`]).
    #[cfg(test)]
    song_stopped_at: Option<Instant>,
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
    /// The durable store the book above is a read-through cache of. `None` when the database could
    /// not be opened, in which case a run is kept on the side and read in the next time it opens
    /// ([`scoredb_store::PENDING_RUNS_DIR`]).
    scoredb: Option<rbms_store::scoredb::ScoreDb>,
    /// Where saved replays live, and what the retention policy prunes.
    replay_dir: PathBuf,
    /// The per-chart practice ranges the panel remembers between visits.
    practice_book: crate::practice::PracticeBook,
    /// The next chart load is for the practice panel rather than for a run, set by the browser's
    /// practice key and cleared when the loaded chart opens the panel.
    practice_requested: bool,
    /// The run the browser started is to play itself whatever the AUTOPLAY setting says, set by the
    /// skin's autoplay button and its key and cleared when the browser is next arrived at
    /// ([`AppShared::run_plays_itself`]). The setting itself is never touched for it, so nothing
    /// that writes the settings out while the run is on can write it switched on.
    autoplay_once: bool,
    /// The chart the practice panel is set up on, kept while the panel is open so each slice is cut
    /// from the untrimmed model.
    practice_chart: Option<Box<app_play::PracticeChart>>,
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
    /// The mouse buttons that are down, which decide whether a cursor move is a drag.
    mouse_held: HeldButtons,
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
    ///
    /// A run that starts on the browser without scanning -- a first run with no song folder, or a
    /// chart or replay named on the command line -- reads its difficulty tables from their copies
    /// here, asking the network nothing, so a table every player is given is a folder from the first
    /// frame; the ones that have no copy are fetched once the browser is up. The signed-in player's
    /// own folders are shown from the copy kept of them ([`user_folders::follow_account`]).
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

        let config_dir = settings_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        let (scoredb_path, scores_path, replay_dir) = scoredb_store::score_paths(&config_dir);
        let mut scoredb = scoredb_store::open_score_db(&scoredb_path, &scores_path);
        if let Some(db) = scoredb.as_mut() {
            scoredb_store::import_pending_runs(db, &scoredb_store::pending_runs_dir(&config_dir));
        }
        let scores = match &scoredb {
            Some(db) => scoredb_store::book_from_db(db),
            None => ScoreBook::load(&scores_path),
        };
        if let Some(db) = scoredb.as_mut() {
            let hold = scoredb_store::upload_hold(&config.network);
            scoredb_store::run_replay_gc(db, &replay_dir, &rbms_store::scoredb::ReplayPolicy::default(), hold.as_ref());
        }
        let practice_book = practice::PracticeBook::load(&practice::practice_path(&settings_path));
        let favorites_path = favorites_path(&settings_path);
        let favorites = Favorites::load(&favorites_path);
        if let Some(url) = &launch.table_url
            && !config.library.tables.iter().any(|t| &t.location == url)
        {
            config.library.tables.push(TableSource { name: String::new(), location: url.clone(), group: String::new() });
        }

        let songdb_path = songdb_path(&settings_path);
        let song_db = crate::library::open_song_db(&songdb_path);
        let library = song_db.as_ref().map(crate::library::stored_library).unwrap_or_default();
        let full_rescan = song_db.as_ref().is_none_or(|db| db.needs_full_rescan().unwrap_or(true));
        if let Some(db) = &song_db {
            for md5 in favorites.md5s() {
                for row in db.by_md5(md5).unwrap_or_default() {
                    let _ = db.set_favorite(&row.path, 1);
                }
            }
        }
        let (stage, chart_path, launch_chart) = if let Some(rp) = &replay {
            (Stage::Select(Box::new(SelectState::new())), rp.chart_path.clone(), true)
        } else if p.is_file() {
            (Stage::Select(Box::new(SelectState::new())), input, true)
        } else if config.library.scan_roots().is_empty() {
            (Stage::Select(Box::new(SelectState::new())), String::new(), false)
        } else {
            let stage = Stage::Loading(LoadingState::scan(config.library.scan_roots(), config.library.tables.clone(), songdb_path.clone(), full_rescan));
            (stage, String::new(), false)
        };
        let mut table_fetches = tablesrc::TableFetches::default();
        let matched = if matches!(stage, Stage::Loading(_)) || config.library.tables.is_empty() {
            Vec::new()
        } else {
            table_fetches.first_fetches_are_due();
            let store = tablesrc::TableStore::beside(&settings_path);
            tablesrc::read_and_match_all(&store, &config.library.tables, &library, &std::sync::atomic::AtomicUsize::new(0))
        };
        let (table_names, table_levels, table_extras) = tablesrc::split_matched(matched);

        let session = AccountSession::restored(config.network.ir_token.clone(), config.network.ir_login_id.clone());
        let startup_whoami = session.is_logged_in();
        let built = build_server(&config, session.token().map(str::to_string));
        let multi_ir = MultiIr::from_network(&config.network);
        let profile_servers = multi_ir.build_servers_reusing(Some(built.server.clone()));
        let syssound = {
            let mut set = SystemSoundSet::load_optional(sound_folder_path(&settings_path, &config).as_deref());
            set.set_guide_enabled(config.audio.guide_se);
            set
        };

        let keyconfig_path = launch.keyconfig_path.clone().map(PathBuf::from).unwrap_or_else(|| config_dir.join("keyconfig.ron"));
        let keyconfig = KeyConfig::load(&keyconfig_path);
        let pad = if launch.controllers { PadState::new(&keyconfig.pad) } else { None };
        let skins = SkinLibrary::new(&settings_path, &config)
            .forcing_pack(launch.skin_pack.clone(), &config)
            .drawing_play_built_in(launch.skin_path.is_some(), &config);

        let mut app = App {
            stage,
            suspended: Vec::new(),
            launch_chart,
            startup_error: None,
            window: None,
            frame_waker: None,
            shared: AppShared {
                chart_path,
                config,
                launch,
                mode: MODE,
                library,
                song_db,
                active_keys: keyconfig.lane_keys(MODE),
                active_reverse_keys: keyconfig.scratch_reverse_keys(MODE),
                pad,
                table_names,
                table_levels,
                table_extras,
                table_fetches,
                user_folders: user_folders::UserFolders::default(),
                select_path: Vec::new(),
                select_items: Vec::new(),
                select_filter: stage::select::SelectFilter::default(),
                search: String::new(),
                searching: false,
                sel: 0,
                keyconfig,
                keyconfig_path,
                settings_path,
                replay,
                course_run: None,
                course_overrides: None,
                course_stage_reasons: Vec::new(),
                course_assist: CourseAssist::NONE,
                run_records: Vec::new(),
                retry_seed: None,
                course_settings_backup: None,
                own_play_options: None,
                kc_edit_mode: MODE,
                gpu: None,
                audio: None,
                audio_report: None,
                audio_max_voices: rbms_audio::DEFAULT_MAX_VOICES,
                syssound,
                skin_sounds: skin_host::audio::SkinSounds::default(),
                audio_opened_with: None,
                audio_failed: false,
                audio_reopen_at: None,
                chart_gain: rbms_chart::chart_gain(rbms_chart::VOLWAV_DEFAULT_PERCENT),
                schedule_poll_us: 0,
                skin: Skin::default_for(MODE, CW as f32, CH as f32),
                skin_cfg: SkinConfig::default(),
                skins,
                skin_screens: SkinScreens::new(),
                skin_timers: TimerState::default(),
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
                multi_ir,
                profile_servers,
                profile_submit_rx: None,
                ir_status: IrStatus::Off,
                submit_log: None,
                record_sync: None,
                record_sync_pending: Vec::new(),
                record_sync_states: std::collections::HashMap::new(),
                record_sync_pacing: record_sync::SyncPacing::GAME,
                server_build: None,
                course_submits: Vec::new(),
                saves: save_worker::SaveWorker::for_this_build(),
                trace_backfill: trace_backfill::TraceBackfill::default(),
                clock: Instant::now(),
                booted: Instant::now(),
                scene_clock: scene_clock::SceneClock::begun(Instant::now()),
                #[cfg(test)]
                scene_stopped_at: None,
                #[cfg(test)]
                song_stopped_at: None,
                anchor_us: 0,
                audio_dead_at: std::cell::Cell::new(None),
                song_us_last: std::cell::Cell::new(0),
                timing: TimingProbe::default(),
                soak: SoakLogger::from_env(),
                soak_failed: false,
                scores,
                scores_path,
                scoredb,
                replay_dir,
                practice_book,
                practice_requested: false,
                autoplay_once: false,
                practice_chart: None,
                favorites,
                favorites_path,
                toasts: ToastQueue::default(),
                options: app_options::OptionsOverlay::default(),
                select_gen: 0,
                cursor: (0.0, 0.0),
                mouse_held: HeldButtons::default(),
                hot: Vec::new(),
                last_frame: Instant::now(),
                fps: 0.0,
                frame_count: 0,
                ram_mb: 0.0,
                build_sha256: build_hash(),
            },
        };
        user_folders::follow_account(&mut app.shared);
        app.shared.reload_record_sync_states();
        app.shared.rebuild_select_items();
        app.shared.reload_skin();
        app
    }
}

/// How often the resident-set reading behind the debug overlay and the soak log is refreshed.
const RAM_SAMPLE_FRAMES: u64 = 15;

/// Width of the debug overlay panel, sized for its widest audio telemetry line.
const DEBUG_PANEL_W: f32 = 380.0;

/// Line height of the debug overlay.
const DEBUG_LINE_H: f32 = 16.0;

/// Where the connection dot and the debug panel stand, as the top left corner of each in the fixed
/// screen the application lays itself out for: the built-in screens' places, and the places they
/// move to over a browser a skin draws.
///
/// A skin fills the screen with its own, and the corners are where a frame puts what it has least to
/// say. ModernChic's top frame is 138 high with its help button at x 20, its skin name and version
/// down the right edge and its row of buttons under that, so over it the dot is in the left corner,
/// clear of the button and of the version, and the panel is under the frame instead of over the help
/// button and the title. The messages stay where they are: bottom right, over the skin's
/// least-read window.
const BUILT_IN_DOT_ORIGIN: (f32, f32) = (CW as f32 - 22.0, 10.0);
const SKIN_DOT_ORIGIN: (f32, f32) = (4.0, 8.0);
const BUILT_IN_DEBUG_ORIGIN: (f32, f32) = (6.0, 6.0);
const SKIN_DEBUG_ORIGIN: (f32, f32) = (6.0, 144.0);

/// The size of the connection dot.
const CONNECTION_DOT_SIZE: f32 = 10.0;

/// Where the debug panel's first line is from its corner.
const DEBUG_TEXT_INSET: (f32, f32) = (8.0, 6.0);

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
    ///
    /// The skin's scene changes with the screen: every timer goes off and the scene clock restarts
    /// at zero. A screen opened over another does not end that one's scene, though -- it is parked
    /// with the screen, the clock standing still while the other screen is up, and [`Transition::Back`]
    /// puts it back, so the browser's skin does not play its opening again after a visit to the
    /// settings.
    fn switch(&mut self, transition: Transition) {
        let suspend = matches!(transition, Transition::Open(_));
        let mut resumed = None;
        let next = match transition {
            Transition::Stay | Transition::Quit => return,
            Transition::Open(next) | Transition::To(next) => next,
            Transition::Back => match self.suspended.pop() {
                Some(Suspended { stage, scene }) => {
                    resumed = Some(scene);
                    stage
                }
                None => Stage::Select(Box::new(SelectState::new())),
            },
        };
        let now = Instant::now();
        if !app_options::opens_over(next.id()) {
            app_options::close(&mut self.shared);
        }
        self.stage.on_exit(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 });
        let previous = std::mem::replace(&mut self.stage, next);
        if suspend {
            let scene = self.shared.suspend_skin_scene();
            self.suspended.push(Suspended { stage: previous, scene });
        } else if let Some(scene) = resumed {
            self.shared.resume_skin_scene(scene);
        } else {
            self.shared.begin_skin_scene();
        }
        self.stage.on_enter(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 });
    }

    /// Bring the window to the DISPLAY tab's RESOLUTION and WINDOW MODE rows when either changed,
    /// whichever way it changed: the settings screen, an account download or a hand-edited file.
    fn sync_window(&mut self) {
        let Some(size) = self.window.as_mut().and_then(|window| window.sync(&self.shared.config.display)) else {
            return;
        };
        if let Some(gpu) = self.shared.gpu.as_mut() {
            gpu.resize(size.width, size.height);
        }
    }

    /// Let the window take text through an input method exactly while a line is being typed into --
    /// an editable text of a skin, or the browser's search word -- so Hangul and kana compose in it,
    /// and no longer: a chart being played reads keys, not text.
    fn sync_ime(&mut self) {
        let wanted = self.shared.skin_text_is_focused() || self.stage.takes_composed_text(&self.shared);
        if let Some(window) = self.window.as_mut() {
            window.sync_ime(wanted);
        }
    }

    /// One frame: the shared bookkeeping every screen needs, the screen's own update, then the
    /// screen's own draw with the app-wide overlays on top.
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        self.sync_window();
        self.sync_ime();
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

        for (event, at) in self.shared.poll_pad() {
            let transition = self.stage.handle_pad(&mut FrameCtx { shared: &mut self.shared, now: at, dt: 0.0 }, event);
            self.apply(transition, event_loop);
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
            ctx.shared.finish_skin_frame(&mut canvas);
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
        crate::gpu::apply_letterbox(canvas, ctx.shared.config.display.fits_screen_shape());
        app_options::draw(stage.id(), ctx, canvas);
        toast::draw(&ctx.shared.toasts, stage.id(), canvas);
        let over_skin = stage.id() == StageId::Select && ctx.shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT);
        if ctx.shared.config.network.server_url.is_some() {
            let connected = ctx.shared.server_connected.load(Ordering::Relaxed);
            let (dot_x, dot_y) = if over_skin { SKIN_DOT_ORIGIN } else { BUILT_IN_DOT_ORIGIN };
            canvas.fill_rect(Rect::new(dot_x, dot_y, CONNECTION_DOT_SIZE, CONNECTION_DOT_SIZE), if connected { Color::GREEN } else { Color::RED });
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
        lines.push(ctx.shared.debug_skin_texture_line());
        let ph = lines.len() as f32 * DEBUG_LINE_H + 12.0;
        let (panel_x, panel_y) = if over_skin { SKIN_DEBUG_ORIGIN } else { BUILT_IN_DEBUG_ORIGIN };
        canvas.fill_rect(Rect::new(panel_x, panel_y, DEBUG_PANEL_W, ph), Color { r: 0, g: 0, b: 0, a: 180 });
        let colors = theme();
        for (i, l) in lines.iter().enumerate() {
            let col = if i == 0 { colors.text } else { colors.text_dim };
            draw_text(canvas, panel_x + DEBUG_TEXT_INSET.0, panel_y + DEBUG_TEXT_INSET.1 + i as f32 * DEBUG_LINE_H, 1.2, col, l);
        }
    }
}

impl App {
    /// Hand one mouse event to the screen that is up, at the position the cursor was last seen.
    fn dispatch_pointer(&mut self, event_loop: &ActiveEventLoop, input: PointerInput) {
        let now = Instant::now();
        let at = self.shared.cursor;
        let transition = self.stage.handle_pointer(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 }, at, input);
        self.apply(transition, event_loop);
    }

    /// Run a frame if the image it is drawn into has come ([`Gpu::frame_ready`]). Called whenever the
    /// presenter's thread wakes the event loop, and on a `RedrawRequested` the system asks for.
    fn frame_if_ready(&mut self, event_loop: &ActiveEventLoop) {
        if self.shared.gpu.as_mut().is_some_and(Gpu::frame_ready) {
            self.frame(event_loop);
        }
    }

    /// Record why the app cannot start and ask the event loop to wind down. [`run`] turns this into
    /// a message and a failing exit code, so a machine that cannot open a window or find a usable
    /// graphics adapter is told so rather than shown a panic.
    fn fail_startup(&mut self, event_loop: &ActiveEventLoop, reason: String) {
        self.startup_error = Some(reason);
        event_loop.exit();
    }
}

impl ApplicationHandler<FrameReady> for App {
    /// Bring up the window and the GPU backend, then load a chart or replay named on the command
    /// line: the keysound-loading bar is shown, or play starts at once when there is nothing to
    /// decode. A failed load exits.
    ///
    /// The first frame is drawn once the backend has the image for it and wakes the loop
    /// ([`FrameReady`]); every frame after it is drawn the same way.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shared.gpu.is_some() {
            return;
        }
        let Some(waker) = self.frame_waker.clone() else {
            return self.fail_startup(event_loop, "the event loop has no way to be woken for a frame".to_string());
        };
        let window = match event_loop.create_window(window_attributes(&self.shared.config.display)) {
            Ok(window) => Arc::new(window),
            Err(e) => return self.fail_startup(event_loop, format!("cannot open a window: {e}")),
        };
        let wake = move || {
            waker.send_event(FrameReady).ok();
        };
        match Gpu::new(window.clone(), stage::canvas::UI_SIZE, self.shared.config.display.fits_screen_shape(), wake) {
            Ok(gpu) => self.shared.gpu = Some(gpu),
            Err(e) => return self.fail_startup(event_loop, e.to_string()),
        }
        self.window = Some(AppWindow::new(window.clone(), &self.shared.config.display));
        self.shared.ensure_audio();
        self.shared.begin_skin_scene();
        if self.launch_chart {
            self.launch_chart = false;
            match self.shared.load() {
                Some(loaded) => {
                    let next = self.shared.enter_loaded_chart(loaded);
                    self.apply(Transition::To(next), event_loop);
                }
                None => event_loop.exit(),
            }
        }
    }

    /// The image the next frame is drawn into has come.
    fn user_event(&mut self, event_loop: &ActiveEventLoop, _: FrameReady) {
        self.frame_if_ready(event_loop);
    }

    /// Before the loop waits again: ask for the next image if a window that cannot be shown held
    /// asking back until now, and otherwise wake the loop when it is due ([`Gpu::held_until`]), so
    /// frames go on while the window is hidden, one a hidden frame interval, rather than either
    /// stopping or running as fast as a hidden window refuses images.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let held_until = self.shared.gpu.as_mut().and_then(|gpu| {
            gpu.resume_if_due(Instant::now());
            gpu.held_until()
        });
        event_loop.set_control_flow(held_until.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }

    /// Translate one winit event into what the current screen understands. A moved cursor is mapped
    /// onto the fixed 1280x720 space the built-in screens are laid out in, which is drawn across
    /// the viewport the frame lands in. The left, right and middle buttons, a drag with any of them held
    /// and the wheel all reach the screen; a screen that has no use for one ignores it.
    ///
    /// A key is handed to the screen with the instant it is taken in, which is the instant it is
    /// judged at. Nothing this loop runs between frames waits for the display -- the image a frame
    /// is drawn into is waited for on the presenter's thread, which wakes the loop with
    /// [`FrameReady`] -- so a key comes in while the display is still being waited for, and the
    /// instant it is taken in is the instant the window had it, short of a frame the loop is busy
    /// drawing.
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
                    self.shared.cursor = gpu.position_in_space(position.x as f32, position.y as f32, stage::canvas::UI_SIZE);
                }
                if let Some(input) = self.shared.mouse_held.on_move() {
                    self.dispatch_pointer(event_loop, input);
                }
            }
            WindowEvent::MouseInput { state, button, .. } if is_tracked(button) => {
                let input = self.shared.mouse_held.on_button(button, state == ElementState::Pressed);
                self.dispatch_pointer(event_loop, input);
            }
            WindowEvent::MouseWheel { delta, .. } => self.dispatch_pointer(event_loop, PointerInput::from_wheel(delta)),
            WindowEvent::Focused(false) => {
                let released = self.stage.release_keys_lost_with_focus(&mut FrameCtx { shared: &mut self.shared, now: Instant::now(), dt: 0.0 });
                for transition in released {
                    self.apply(transition, event_loop);
                }
                self.shared.release_held_inputs();
            }
            WindowEvent::Ime(ime) => self.stage.handle_ime(&mut FrameCtx { shared: &mut self.shared, now: Instant::now(), dt: 0.0 }, &ime),
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
            WindowEvent::RedrawRequested => self.frame_if_ready(event_loop),
            WindowEvent::Occluded(false) => {
                if let Some(gpu) = self.shared.gpu.as_mut() {
                    gpu.resume();
                }
            }
            _ => {}
        }
    }

    /// Keep what the player was playing when the app was told to quit, then finish every write handed
    /// to the save thread. Quit in the macOS menu or the Dock ends the process from inside the event
    /// loop, right after this, so [`run`] never gets back from the loop to wait for them.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.keep_what_quitting_cuts_short();
        self.shared.finish_saves();
    }
}

impl App {
    /// Keep the run, or the course, the player was in when the app was told to quit: closing the
    /// window, Quit in the menu or the Dock. It is kept as Escape keeps it (the player's decision of
    /// 2026-10-15, "no record goes unsaved"): a run played by hand left part-way is a FAILED run, a
    /// course is a FAILED course with every stage it played. Nothing is sent from here: the process is
    /// about to end, and the records sync of the next start sends what was kept. Called before the
    /// save thread is waited for, so the replay and every file this keeps are on disk before the
    /// process ends.
    ///
    /// A run on the play screen also has the settings written first, whatever is kept of it, as
    /// Escape writes them: the hi-speed and lane shades its keys moved and the offset AUTO ADJUST
    /// moved are the ones the next start plays on. Nowhere else is anything written on the way out,
    /// so a settings file this build could not read stays as it is ([`run`]).
    fn keep_what_quitting_cuts_short(&mut self) {
        match &mut self.stage {
            Stage::Play(state) => {
                self.shared.save_settings();
                let ended = state.has_ended();
                crate::app_result::keep_run_cut_short_by_quitting(state, ended, &mut self.shared);
            }
            _ if self.shared.course_run.is_some() => {
                crate::app_result::abandon_course(&mut self.shared, crate::app_result::Sending::NextStart);
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
/// rather than overwriting settings it cannot read. A configuration that was read is given the
/// difficulty tables every player is given and has not had yet ([`tablesrc::seed_default_tables`]).
pub fn run(args: impl Iterator<Item = String>) -> ExitCode {
    run_in(args, &config_dir())
}

/// [`run`] with the settings kept in `config_dir`. The command line is read first: `--help`,
/// `--version` and a refused command line are answered there, before anything in `config_dir`, the
/// databases or the network is touched ([`command_line`]).
fn run_in(args: impl Iterator<Item = String>, config_dir: &Path) -> ExitCode {
    let command_line = match command_line::read(args) {
        Ok(command_line) => command_line,
        Err(answer) => return answer.give(),
    };
    let settings_path = config_dir.join("settings.ron");
    let mut config = match rbms_config::load(&settings_path) {
        Ok(outcome) => {
            let mut config = outcome.config;
            let given_tables = tablesrc::seed_default_tables(&mut config, &settings_path);
            if let Some(from) = outcome.migrated_from {
                println!("settings migrated from schema version {from}");
            }
            if outcome.migrated_from.is_some() || given_tables {
                save_config(&config, &settings_path);
            }
            config
        }
        Err(e) => {
            notify(Level::Warn, format!("settings not loaded ({e}); running on defaults and leaving the file alone"));
            Config::default()
        }
    };

    let installed = install_default_pack(&settings_path);
    if !installed.is_complete() {
        notify(
            Level::Warn,
            format!(
                "default skin not fully installed: {} files could not be written beside {}; a screen without its skin uses the built-in layout",
                installed.failed,
                settings_path.display()
            ),
        );
    }

    let mut launch = LaunchOptions {
        skin_pack: skin_select::pack_from_environment(std::env::var_os(skin_select::SKIN_PACK_ENV)),
        controllers: true,
        ..LaunchOptions::default()
    };
    let chart = command_line.apply(&mut config, &mut launch);
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

    let event_loop = match EventLoop::<FrameReady>::with_user_event().build() {
        Ok(el) => el,
        Err(e) => {
            notify(Level::Error, format!("cannot open a window: {e}"));
            return ExitCode::FAILURE;
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(chart, config, launch, settings_path);
    app.frame_waker = Some(event_loop.create_proxy());
    app.shared.start_trace_backfill(trace_backfill::BackfillPacing::GAME);
    let ran = event_loop.run_app(&mut app);
    if let Some(gpu) = app.shared.gpu.as_mut() {
        gpu.stop_presenting();
    }
    app.shared.finish_saves();
    if let Err(e) = ran {
        notify(Level::Error, format!("the window event loop stopped: {e}"));
        return ExitCode::FAILURE;
    }
    exit_code(app.startup_error.as_deref())
}

//! What the Discord profile says for each screen: the activity a screen shows, built from a small
//! view of the screen ([`PresenceView`]) with no I/O, so every line of it can be checked in a test.

use rbms_model::Mode;
use serde::Serialize;

use crate::format::{difficulty_name, mode_short};

/// The key the art asset R-BMS's picture is uploaded under on the Developer Portal.
pub(crate) const LARGE_IMAGE_KEY: &str = "rbms";

/// The shortest text the profile takes in a field.
pub(crate) const MIN_TEXT_CHARS: usize = 2;

/// The longest text a field is sent with, in UTF-8 bytes: the 128 characters the profile takes and
/// the 128 bytes `discord-rpc` cuts at, both.
pub(crate) const MAX_TEXT_BYTES: usize = 128;

/// What a text cut short ends with.
const ELLIPSIS: &str = "...";

/// What stands between the parts of one line.
const SEPARATOR: &str = " · ";

/// What a `#DIFFICULTY` the chart does not state, or a key count no layout has, is named as.
const UNNAMED_DIFFICULTY: &str = "—";
const UNNAMED_MODE: &str = "?";

/// One activity as the desktop app is told it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Activity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) state: Option<String>,
    pub(crate) timestamps: Timestamps,
    pub(crate) assets: Assets,
    pub(crate) instance: bool,
}

/// Since when the activity has been going, in Unix seconds: the profile counts the time up from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Timestamps {
    pub(crate) start: u64,
}

/// The picture and the text shown on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Assets {
    pub(crate) large_image: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) large_text: Option<String>,
}

/// The instants the elapsed time is counted from, in Unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Starts {
    /// When the application started.
    pub(crate) app: u64,
    /// When the run on the play screen started.
    pub(crate) run: Option<u64>,
    /// When the course being played started.
    pub(crate) course: Option<u64>,
}

/// What a chart is, as the profile names it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ChartLine<'a> {
    pub(crate) title: &'a str,
    pub(crate) subtitle: &'a str,
    pub(crate) mode: Option<Mode>,
    pub(crate) difficulty: i32,
    /// The `#PLAYLEVEL`, empty when the chart states none.
    pub(crate) level: String,
    /// The level of the difficulty table the chart was picked from, empty when it was not picked
    /// from one.
    pub(crate) table_level: String,
}

/// How the run on the play screen is being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunKind {
    ByHand,
    Autoplay,
    Replay,
    Practice,
}

/// Which screen is up, with what the profile says of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Screen<'a> {
    /// The song browser, its folders and its tables.
    Browsing,
    /// The settings, the key configuration, the skin and play menus.
    Menus,
    /// The song folders being scanned.
    Scanning,
    /// A chart picked and loading.
    Loading(ChartLine<'a>),
    /// A run being played.
    Playing { chart: ChartLine<'a>, kind: RunKind },
    /// The practice panel set up on a chart.
    PracticePanel(ChartLine<'a>),
    /// A run's result. `chart` is the chart the course stage played, known only in a course.
    Result { title: &'a str, clear_label: &'a str, mode_label: &'a str, chart: Option<ChartLine<'a>> },
    /// A finished course.
    CourseResult { name: &'a str, cleared: bool },
}

/// The stage of a course a screen is part of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CourseStage<'a> {
    pub(crate) name: &'a str,
    /// One-based.
    pub(crate) number: usize,
    pub(crate) count: usize,
}

/// A screen as the profile sees it: which screen, and the course stage it is part of, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresenceView<'a> {
    pub(crate) screen: Screen<'a>,
    pub(crate) course: Option<CourseStage<'a>>,
}

/// The text a field is sent with: trimmed, quoted when it is shorter than the profile takes, and cut
/// short on a character boundary when it is longer. `None` when nothing is left, so the field is
/// left out.
pub(crate) fn fit_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() < MIN_TEXT_CHARS {
        return Some(format!("\"{trimmed}\""));
    }
    if trimmed.len() <= MAX_TEXT_BYTES {
        return Some(trimmed.to_string());
    }
    let room = MAX_TEXT_BYTES - ELLIPSIS.len();
    let cut = trimmed.char_indices().map(|(at, c)| at + c.len_utf8()).take_while(|end| *end <= room).last().unwrap_or(0);
    Some(format!("{}{ELLIPSIS}", trimmed[..cut].trim_end()))
}

/// The title and the subtitle with a space between, or the title alone.
fn full_title(chart: &ChartLine<'_>) -> String {
    joined(&[chart.title.trim(), chart.subtitle.trim()], " ")
}

/// The parts that are not empty, with `between` between them.
fn joined(parts: &[&str], between: &str) -> String {
    parts.iter().filter(|part| !part.is_empty()).copied().collect::<Vec<_>>().join(between)
}

/// The mode, the difficulty and the level, each left out when the chart does not say, and the table
/// level after them.
fn chart_label(chart: &ChartLine<'_>) -> String {
    let mode = chart.mode.map(mode_short).filter(|label| *label != UNNAMED_MODE).unwrap_or_default();
    let difficulty = Some(difficulty_name(chart.difficulty)).filter(|name| *name != UNNAMED_DIFFICULTY).unwrap_or_default();
    let label = joined(&[mode, difficulty, chart.level.trim()], " ");
    joined(&[&label, chart.table_level.trim()], SEPARATOR)
}

/// `what` before the chart's label.
fn labelled(what: &str, chart: &ChartLine<'_>) -> String {
    joined(&[what, &chart_label(chart)], SEPARATOR)
}

/// The name the picture is hovered as when there is nothing more particular to say.
fn app_label() -> String {
    format!("R-BMS {}", env!("CARGO_PKG_VERSION"))
}

/// The activity a screen shows, its time counted from the start in `starts` the screen counts from.
pub(crate) fn activity_for(view: &PresenceView<'_>, starts: &Starts) -> Activity {
    let (details, state, large_text, start) = match (&view.screen, &view.course) {
        (Screen::Loading(chart) | Screen::Playing { chart, .. }, Some(course)) => {
            (course_line(course), full_title(chart), chart_label(chart), starts.course.unwrap_or(starts.app))
        }
        (Screen::Result { title, mode_label, chart, .. }, Some(course)) => {
            let label = chart.as_ref().map(chart_label).unwrap_or_else(|| mode_label.to_string());
            (course_line(course), title.to_string(), label, starts.course.unwrap_or(starts.app))
        }
        (Screen::Browsing, _) => ("Choosing a song".to_string(), String::new(), app_label(), starts.app),
        (Screen::Menus, _) => ("In the menus".to_string(), String::new(), app_label(), starts.app),
        (Screen::Scanning, _) => ("Scanning the song folders".to_string(), String::new(), app_label(), starts.app),
        (Screen::Loading(chart), None) => (full_title(chart), labelled("Loading", chart), app_label(), starts.app),
        (Screen::Playing { chart, kind }, None) => {
            let state = match kind {
                RunKind::ByHand => chart_label(chart),
                RunKind::Autoplay => labelled("Autoplay", chart),
                RunKind::Replay => labelled("Watching a replay", chart),
                RunKind::Practice => labelled("Practice", chart),
            };
            (full_title(chart), state, app_label(), starts.run.unwrap_or(starts.app))
        }
        (Screen::PracticePanel(chart), _) => (full_title(chart), labelled("Practice", chart), app_label(), starts.app),
        (Screen::Result { title, clear_label, mode_label, .. }, None) => {
            let hover = Some(mode_label.trim()).filter(|label| !label.is_empty()).map(str::to_string).unwrap_or_else(app_label);
            (title.to_string(), joined(&["Result", clear_label.trim()], SEPARATOR), hover, starts.app)
        }
        (Screen::CourseResult { name, cleared }, _) => {
            let outcome = if *cleared { "Course cleared" } else { "Course failed" };
            (name.to_string(), outcome.to_string(), app_label(), starts.app)
        }
    };
    Activity {
        details: fit_text(&details),
        state: fit_text(&state),
        timestamps: Timestamps { start },
        assets: Assets { large_image: LARGE_IMAGE_KEY, large_text: fit_text(&large_text) },
        instance: false,
    }
}

/// The course's name and how far into it the screen is.
fn course_line(course: &CourseStage<'_>) -> String {
    joined(&[&format!("{}/{}", course.number, course.count), course.name.trim()], SEPARATOR)
}

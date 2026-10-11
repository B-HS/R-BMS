//! The rows of the play menu: which there are, what each reads, what a step does to one, and what
//! puts one back to what it ships as.
//!
//! Three kinds of row are listed under the names of their groups. The tempo of the sample chart is
//! the menu's own and is stored nowhere. A row of the settings table (`rbms_config::SETTINGS`) is
//! one of the groups [`SettingTab::PLAY_MENU`] names, read and stepped through the calls the
//! settings screen has always used, so it lands in the same field of the same file. A row the
//! play skin of the previewed mode declares is that document's, stored where the skin menu stores
//! it: its options at the end of the lane rows with the offsets it declares to move a part of its
//! own after them, and the rest of its offsets in a group of their own at the end of the list.
//!
//! A play skin has more offsets than the list has room to show with everything else, so the rest
//! are kept in two folds ([`Fold`]): the ones that fade a part of the screen, and the ones the game
//! adds to every play skin, which move or resize the whole of it, its notes and its judgement. A
//! fold is one row that says how many rows it holds and is stepped to show or hide them. Which
//! folds are open is the menu's own and is stored nowhere. An offset a skin declares to move one
//! part -- where its judgement is written -- is what a player comes to this menu for, and is
//! listed with the skin's options where it is seen without opening anything.
//!
//! Two rows of the table change places with KEEP GREEN NUMBER. While a green number is held the
//! scroll speed is worked out from it, so HI-SPEED shows what it comes to and cannot be stepped;
//! while none is held the green number is what the stored speed comes to, and it is the one that
//! cannot. What either comes to is read from the one place the preview's own scroll is
//! ([`SpeedReadout`]).

use rbms_config::{AdjustOutcome, Config, HISPEED_DECIMALS, MILLISECOND_UNIT, PlayOptions, SettingId, SettingKind, SettingTab, adjust, descriptor, tab_rows};

use crate::AppShared;
use crate::skin_preview::target::SpeedReadout;
use crate::skin_select::{SkinRow, SkinRowKind};
use crate::stage::preview_menu::{ONE_STEP, RowLook, SHIFT_STEPS};
use crate::stage::skin_menu::skin_row_help;

/// The group the rows that are no setting are listed under.
pub(super) const GROUP_PREVIEW: &str = "PREVIEW";

/// The group the options of the previewed play skin are listed at the end of, with the offsets it
/// declares to move a part of its own: what they change is where the lanes are, how wide, and
/// where the judgement is written.
const SKIN_OPTIONS_GROUP: SettingTab = SettingTab::PlayLane;

/// The group the offsets of the previewed play skin are listed under, at the end of the list.
pub(super) const GROUP_SKIN: &str = "SKIN";

/// What a fold reads while its rows are on show and while they are not, before how many they are.
const FOLD_OPEN: &str = "SHOWN";
const FOLD_CLOSED: &str = "HIDDEN";

/// What is said of each fold.
const OPACITY_FOLD_HELP: &str = "How opaque each part of the play skin of this mode is drawn. ENTER shows or hides the rows";
const PLACE_FOLD_HELP: &str = "The offsets the game adds to every play skin: its screen, notes and judgement. ENTER shows or hides the rows";

/// The row of the tempo of the sample chart.
pub(super) const PREVIEW_BPM_LABEL: &str = "PREVIEW BPM";

/// What a tempo is written with after it.
pub(super) const BPM_UNIT: &str = "BPM";

/// What a scroll speed is written with after it.
const HISPEED_UNIT: &str = "x";

/// What is written after a scroll speed that stands at an end of its range because the speed that
/// would hold the green number is outside it.
const LIMIT_MARK: &str = " (LIMIT)";

/// What is written after the value of a row that is worked out from the others, so the list says
/// why the arrows leave it alone before the cursor is put on it.
const WORKED_OUT_MARK: &str = " AUTO";

/// What the lane cover's row names the number after its share of the lane.
const WHITE_NUMBER_NAME: &str = "WHITE";

/// What a share of the lane is multiplied by to read it in the white number's units, and how many
/// places after the point the fine step is written to in them: its smallest is a tenth of one.
const WHITE_NUMBER_PER_LANE: f32 = 1000.0;
const FINE_STEP_WHITE_DECIMALS: usize = 1;

/// What is said of the rows the settings table does not explain.
const PREVIEW_BPM_HELP: &str = "The tempo of the sample chart. It is not a setting: it shows what the speed options come to on another chart";
const SKIN_OPTION_HELP: &str = "An option the play skin of this mode declares. It is stored for that skin document";

/// What is said of the two rows that change places with KEEP GREEN NUMBER while each cannot be
/// stepped.
const HISPEED_WORKED_OUT_HELP: &str = "Worked out from GREEN NUMBER at the preview tempo. Turn KEEP GREEN NUMBER off to set it";
const GREEN_NUMBER_WORKED_OUT_HELP: &str = "What HI-SPEED comes to at the preview tempo. Turn KEEP GREEN NUMBER on to set it";

/// One of the two folds the offsets of the previewed play skin are kept in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fold {
    /// The offsets that only fade what names them, each one row read as an opacity.
    Opacity,
    /// The offsets the game adds to every play skin, which move or resize the whole screen, the
    /// notes and the judgement.
    Place,
}

impl Fold {
    /// Both folds, in the order they are listed.
    pub(super) const ALL: [Fold; 2] = [Fold::Opacity, Fold::Place];

    /// What the fold's row is called.
    pub(super) fn label(self) -> &'static str {
        match self {
            Fold::Opacity => "OPACITY",
            Fold::Place => "POSITION AND SIZE",
        }
    }

    /// Whether a row of a skin of `kind` is kept in this fold. `automatic` says the row is of an
    /// offset the game adds rather than one the document declares.
    fn holds(self, kind: SkinRowKind, automatic: bool) -> bool {
        match (self, kind) {
            (Fold::Opacity, SkinRowKind::Opacity) => true,
            (Fold::Place, SkinRowKind::Offset(_)) => automatic,
            (_, SkinRowKind::Property | SkinRowKind::File | SkinRowKind::Offset(_) | SkinRowKind::Opacity) => false,
        }
    }

    /// What is said of the fold's row.
    fn help(self) -> &'static str {
        match self {
            Fold::Opacity => OPACITY_FOLD_HELP,
            Fold::Place => PLACE_FOLD_HELP,
        }
    }
}

/// Which folds have their rows on show. Both open closed: the list is then as long as it is with
/// no skin at all, and a fold says what opening it would show.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Folds {
    opacity: bool,
    place: bool,
}

impl Folds {
    /// Whether one fold's rows are on show.
    pub(super) fn is_open(self, fold: Fold) -> bool {
        match fold {
            Fold::Opacity => self.opacity,
            Fold::Place => self.place,
        }
    }

    /// Show one fold's rows if they are hidden, and hide them if they are on show.
    pub(super) fn toggle(&mut self, fold: Fold) {
        match fold {
            Fold::Opacity => self.opacity = !self.opacity,
            Fold::Place => self.place = !self.place,
        }
    }
}

/// What one entry of the list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Row {
    /// The name of a group. The cursor never stands on one.
    Group(&'static str),
    /// The tempo of the sample chart.
    PreviewBpm,
    /// A row of the settings table.
    Setting(SettingId),
    /// A row the play skin of the previewed mode declares: an option, or one of its offsets.
    Skin(SkinRow),
    /// The row that shows or hides the offsets of one fold.
    Fold(Fold),
}

impl Row {
    /// Whether the entry is the name of a group rather than a row.
    pub(super) fn is_heading(self) -> bool {
        matches!(self, Row::Group(_))
    }
}

/// One entry of the list as it is written, in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Line {
    pub(super) label: String,
    pub(super) value: String,
    pub(super) look: RowLook,
}

/// The list, top to bottom, for a preview of `screen`: the tempo of the sample chart, then every
/// group of the play menu with the rows the settings table offers in it and, at the end of the
/// group they belong with, the options the play skin of that screen declares and the offsets it
/// declares to move a part of its own ([`declared`]); and last the rest of that skin's offsets,
/// fold by fold, each fold's rows under it while `open` has it open.
///
/// A fold the skin has nothing for is not listed, and neither is the group when it has none. A
/// document's file slots are not listed: they are the skin menu's.
pub(super) fn listed(shared: &AppShared, screen: i32, open: Folds) -> Vec<Row> {
    let mut rows = vec![Row::Group(GROUP_PREVIEW), Row::PreviewBpm];
    for tab in SettingTab::PLAY_MENU {
        rows.push(Row::Group(tab.label()));
        rows.extend(tab_rows(tab, &shared.config).into_iter().map(Row::Setting));
        if tab == SKIN_OPTIONS_GROUP {
            rows.extend(declared(shared, screen).into_iter().map(Row::Skin));
        }
    }
    let folds: Vec<(Fold, Vec<SkinRow>)> =
        Fold::ALL.into_iter().map(|fold| (fold, folded(shared, screen, fold))).filter(|(_, held)| !held.is_empty()).collect();
    if !folds.is_empty() {
        rows.push(Row::Group(GROUP_SKIN));
    }
    for (fold, held) in folds {
        rows.push(Row::Fold(fold));
        if open.is_open(fold) {
            rows.extend(held.into_iter().map(Row::Skin));
        }
    }
    rows
}

/// The rows of the play skin of `screen` that are listed with the lane rows, in the order the skin
/// menu lists them: its options, and each axis of the offsets it declares itself that move, resize
/// or turn what names them.
pub(super) fn declared(shared: &AppShared, screen: i32) -> Vec<SkinRow> {
    let skins = &shared.skins;
    let listed_here = |row: &SkinRow| match skins.kind_for(&shared.config, screen, *row) {
        Some(SkinRowKind::Property) => true,
        Some(SkinRowKind::Offset(_)) => !skins.is_automatic_for(&shared.config, screen, *row),
        Some(SkinRowKind::File | SkinRowKind::Opacity) | None => false,
    };
    skins.rows_for(&shared.config, screen).into_iter().filter(listed_here).collect()
}

/// The rows of the play skin of `screen` one fold holds, in the order the skin menu lists them.
pub(super) fn folded(shared: &AppShared, screen: i32, fold: Fold) -> Vec<SkinRow> {
    let skins = &shared.skins;
    let held = |row: &SkinRow| {
        let automatic = skins.is_automatic_for(&shared.config, screen, *row);
        skins.kind_for(&shared.config, screen, *row).is_some_and(|kind| fold.holds(kind, automatic))
    };
    skins.rows_for(&shared.config, screen).into_iter().filter(held).collect()
}

/// What a fold's row reads: whether its rows are on show, and how many they are.
pub(super) fn fold_value(open: bool, rows: usize) -> String {
    format!("{} ({rows})", if open { FOLD_OPEN } else { FOLD_CLOSED })
}

/// Whether a row shows a value worked out from the others, which stepping it does nothing to.
pub(super) fn is_read_only(play: &PlayOptions, row: Row) -> bool {
    match row {
        Row::Setting(SettingId::HiSpeed) => play.keep_green_number,
        Row::Setting(SettingId::GreenNumber) => !play.keep_green_number,
        _ => false,
    }
}

/// A tempo as it is written.
pub(super) fn bpm_value(bpm: u32) -> String {
    format!("{bpm} {BPM_UNIT}")
}

/// A scroll speed as the menu writes one, without its unit.
pub(super) fn hispeed_figure(hispeed: f64) -> String {
    format!("{:.*}", usize::from(HISPEED_DECIMALS), hispeed)
}

/// What a row of the settings table reads on this menu: what the settings table shows for it, with
/// the unit a row of this menu carries after it.
///
/// A speed is a multiple. The green number is written with the travel time it stands for, the lane
/// cover with the white number it reads as, and the fine step of the covers with how much of a
/// white number one press of it is. The two rows KEEP GREEN NUMBER switches between show what
/// `readout` came to while they are the one worked out, and say that they are.
pub(super) fn setting_value(shared: &AppShared, readout: &SpeedReadout, id: SettingId) -> String {
    let play = &shared.config.play;
    let stored = shared.setting_line(id).1;
    match id {
        SettingId::HiSpeed if play.keep_green_number => {
            format!("{}{HISPEED_UNIT}{}{WORKED_OUT_MARK}", hispeed_figure(readout.hispeed), if readout.limited { LIMIT_MARK } else { "" })
        }
        SettingId::HiSpeed | SettingId::HiSpeedStep => format!("{stored}{HISPEED_UNIT}"),
        SettingId::GreenNumber if play.keep_green_number => format!("{stored} ({}{MILLISECOND_UNIT})", play.duration_ms),
        SettingId::GreenNumber => format!("{} ({}{MILLISECOND_UNIT}){WORKED_OUT_MARK}", readout.green_number, readout.duration_ms),
        SettingId::LaneCover => format!("{stored} ({WHITE_NUMBER_NAME} {})", readout.white_number),
        SettingId::LaneCoverFineStep => {
            format!("{stored} ({WHITE_NUMBER_NAME} {:.*})", FINE_STEP_WHITE_DECIMALS, play.lanecover_step_fine * WHITE_NUMBER_PER_LANE)
        }
        _ => stored,
    }
}

/// What is said of one row under the preview. `kind` is the kind of row a row of a skin is, which
/// an offset is said by as the skin menu says it; an option is said as this menu's own.
pub(super) fn help(play: &PlayOptions, row: Row, kind: Option<SkinRowKind>) -> &'static str {
    match row {
        Row::Group(_) => "",
        Row::PreviewBpm => PREVIEW_BPM_HELP,
        Row::Setting(SettingId::HiSpeed) if play.keep_green_number => HISPEED_WORKED_OUT_HELP,
        Row::Setting(SettingId::GreenNumber) if !play.keep_green_number => GREEN_NUMBER_WORKED_OUT_HELP,
        Row::Setting(id) => descriptor(id).help,
        Row::Skin(_) => match kind {
            Some(kind @ (SkinRowKind::Offset(_) | SkinRowKind::Opacity)) => skin_row_help(kind),
            Some(SkinRowKind::Property | SkinRowKind::File) | None => SKIN_OPTION_HELP,
        },
        Row::Fold(fold) => fold.help(),
    }
}

/// Whether a row of the settings table holds a number, which is what Shift moves ten steps at a
/// time: a switch has two places and a list is walked one name at a time.
pub(super) fn holds_a_number(id: SettingId) -> bool {
    matches!(descriptor(id).kind, SettingKind::IntRange { .. } | SettingKind::FloatRange { .. } | SettingKind::Percent { .. })
}

/// Step one row of the settings table, and report whether it moved. `wide` asks for ten steps at
/// once, which a row that holds no number takes as one.
///
/// A wide step that would carry a row past the end of its range is refused whole by a row that does
/// not stop at its end -- the scroll speed -- so what single steps there is room for are taken
/// instead, and the row ends where holding the key down would have left it.
pub(super) fn step_setting(config: &mut Config, id: SettingId, delta: i32, wide: bool) -> bool {
    let steps = if wide && holds_a_number(id) { i32::from(SHIFT_STEPS) } else { i32::from(ONE_STEP) };
    if adjust(config, id, delta * steps) == AdjustOutcome::Changed {
        return true;
    }
    steps > 1 && (0..steps).filter(|_| adjust(config, id, delta) == AdjustOutcome::Changed).count() > 0
}

/// Put one row of the settings table back to what a fresh configuration holds for it, and report
/// whether it moved. A row this menu does not list is left alone.
pub(super) fn reset_setting(config: &mut Config, id: SettingId) -> bool {
    let shipped = Config::default();
    match id {
        SettingId::KeepGreenNumber => put(&mut config.play.keep_green_number, shipped.play.keep_green_number),
        SettingId::GreenNumber => put(&mut config.play.duration_ms, shipped.play.duration_ms),
        SettingId::HiSpeed => put(&mut config.play.hispeed, shipped.play.hispeed),
        SettingId::HiSpeedStep => put(&mut config.play.hispeed_step, shipped.play.hispeed_step),
        SettingId::FixHiSpeed => put(&mut config.play.fix_hispeed, shipped.play.fix_hispeed),
        SettingId::SpeedFix => put(&mut config.play.constant_speed, shipped.play.constant_speed),
        SettingId::EnableCover => put(&mut config.play.enable_cover, shipped.play.enable_cover),
        SettingId::LaneCover => put(&mut config.play.cover, shipped.play.cover),
        SettingId::EnableLift => put(&mut config.play.enable_lift, shipped.play.enable_lift),
        SettingId::Lift => put(&mut config.play.lift, shipped.play.lift),
        SettingId::EnableHidden => put(&mut config.play.enable_hidden, shipped.play.enable_hidden),
        SettingId::Hidden => put(&mut config.play.hidden, shipped.play.hidden),
        SettingId::LaneCoverFineStep => put(&mut config.play.lanecover_step_fine, shipped.play.lanecover_step_fine),
        SettingId::JudgeOffset => put(&mut config.judge.offset_ms, shipped.judge.offset_ms),
        SettingId::AutoCal => put(&mut config.judge.auto_offset, shipped.judge.auto_offset),
        SettingId::Random => put(&mut config.play.random, shipped.play.random),
        SettingId::LaneOption => put(&mut config.play.lane_option, shipped.play.lane_option),
        SettingId::Gauge => put(&mut config.play.gauge, shipped.play.gauge),
        SettingId::LegacyNote => put(&mut config.play.legacy_note, shipped.play.legacy_note),
        SettingId::Target => put(&mut config.judge.target, shipped.judge.target),
        SettingId::Autoplay => put(&mut config.play.autoplay, shipped.play.autoplay),
        SettingId::ScratchAuto => put(&mut config.play.scratch_auto, shipped.play.scratch_auto),
        SettingId::AutoReplay => put(&mut config.play.auto_replay, shipped.play.auto_replay),
        SettingId::PlayEscapeMode => put(&mut config.play.play_escape, shipped.play.play_escape),
        _ => false,
    }
}

/// Store `value` in `slot`, and report whether that changed what it held.
fn put<T: PartialEq>(slot: &mut T, value: T) -> bool {
    if *slot == value {
        return false;
    }
    *slot = value;
    true
}

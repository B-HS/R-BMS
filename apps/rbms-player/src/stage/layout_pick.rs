//! LAYOUT PICK: the chart under the browser's cursor laid out as the player writes it, and played so.
//!
//! The player writes, for each player side, which key lane's notes are played on each key lane from
//! the left: `3517246` puts the notes of lane 3 on the leftmost key, those of lane 5 on the next, and
//! so on, the way the reference writes a lane order ([`rbms_chart::shuffle::permutation_of_layout`]).
//! The turntable stays where it is. The screen draws the chart's notes laid out so, a few measures at
//! a time, and writes under each lane the lane its notes come from.
//!
//! The run is RANDOM's. A layout is played as RANDOM with the first seed that lays the chart out as
//! written ([`rbms_chart::shuffle::random_seed_for`]), so its record, its replay and what is sent to
//! the score server state RANDOM and that seed, as any other run on RANDOM does. OFF and MIRROR
//! written out here are played the same way. The run is started on a borrowed layout
//! ([`AppShared::borrow_layout`]), which lasts for the run and its retries on the same layout and
//! no further, and which a run that plays itself is refused.
//!
//! It is a screen of its own rather than a panel of the browser: it reads the lane keys as lanes to
//! write, and its preview is the built-in playfield, which clears the frame it is drawn on.
//!
//! # The preview
//!
//! The preview plays the chart laid out as the rows are, on the screen's own clock
//! ([`clock::PreviewClock`]), at the scroll speed a run of the chart would start at -- the HI-SPEED
//! row, or the speed that holds the green number when it is held -- under the player's lane cover,
//! lift and hidden band, and with no sound. Space stops and plays it, the page keys move it four
//! measures, Home goes back to the top, and a preview that runs off the end of its chart begins it
//! again. Writing the rows again lays the chart out again under the same moment.
//!
//! It is the built-in playfield rather than the play skin's run of the sample chart
//! ([`crate::stage::play::skin_preview::PlaySample`]): that run is the whole play screen, gauge and
//! all, drawn into a rectangle, so its lanes would be a quarter of the rectangle and too narrow to
//! number; it walks the skin's scene on the application's one table of skin timers, which has no way
//! to stand still or to be moved about in the chart; and it is played by a written hand whose
//! judgements are nothing to this screen. Making it any chart's, paused and moved about, reaches
//! into the play menu's and the skin menu's previews.
//!
//! # Captured layouts
//!
//! The layouts the player captured on a result screen ([`crate::captured_layouts`]) are listed
//! beside the field: the chart's own first, every chart's on the turn of a key. The cursor on a
//! capture of the chart shows it -- the rows are written as it and the preview laid out so -- and
//! START plays it on its own arrangement and seed, which need no looking for. A capture of another
//! chart is played on that chart: START goes there, when the library has it.
//!
//! The chart is read, and the seed looked for, on threads of their own: a large chart, or a
//! fourteen-key layout whose seed is hundreds of millions of draws away, never holds a frame up. A
//! layout written while a seed is still being looked for cancels that search.
//!
//! What was written last is kept for each mode while the app runs ([`PickedLayouts`]) and is what
//! the screen opens with next; it is written to no file.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use rbms_chart::shuffle::{
    NoteOption, SeedSearchError, apply_lane_permutation, is_layout_row, layout_row_text, permutation_of_layout, random_seed_for, random_seed_limit,
    side_key_lanes,
};
use rbms_config::{HeldTravel, PlayOptions, held_speed};
use rbms_model::translations::Catalog;
use rbms_model::{Language, LnKind, Mode, Model, TimeLine};
use rbms_play::ScratchDir;
use rbms_render::text::language;
use rbms_render::{Color, LaneShade, PlayfieldView, Renderer, Skin, draw_text, draw_text_right, render_lane_cover, render_playfield_view, text_width, theme};
use winit::keyboard::KeyCode;

use crate::app_play::{decode_chart, scratch_auto_color, scratch_auto_lanes};
use crate::borrowed_layout::{LayoutRefusal, LayoutRequest};
use crate::captured_layouts::CapturedLayout;
use crate::format::fmt_datetime;
use crate::gamepad::PadEvent;
use crate::keyconfig::{ControlAction, KeyBind, Shortcut};
use crate::stage::play::BpmStats;
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::text::menus::layout_pick::LayoutPickText;
use crate::{AppShared, CH, CW};

use captured::{CapturedList, ListView};
use clock::{PreviewClock, end_of};

/// Where the heading, the title and the lines over the field start from the left.
const TEXT_X: f32 = 40.0;

/// Where the lines over the field are, top to bottom, and how large each is drawn.
const HEADING_Y: f32 = 14.0;
const HEADING_SCALE: f32 = 2.2;
const TITLE_Y: f32 = 48.0;
const TITLE_SCALE: f32 = 1.3;
const LAYOUT_Y: f32 = 72.0;
const LAYOUT_SCALE: f32 = 1.8;
const STATUS_Y: f32 = 104.0;
const NOTICE_Y: f32 = 124.0;
const STATUS_SCALE: f32 = 1.2;
const HINT_Y: f32 = 146.0;
const HINT_SECOND_Y: f32 = 162.0;
const HINT_SCALE: f32 = 1.0;

/// How far apart the rows of the two sides are written.
const SIDE_GAP: f32 = 48.0;

/// The top of the preview's lanes and its judgement line, in the screen's own pixels.
const FIELD_TOP: f32 = 184.0;
const FIELD_JUDGE: f32 = 640.0;

/// How much of the screen's width the field takes, for one side and for two.
const FIELD_WIDTH: f32 = 0.4;
const DUAL_FIELD_WIDTH: f32 = 0.42;

/// How far under the judgement line the number of the lane a lane's notes come from is written, and
/// how large.
const LANE_NUMBER_DROP: f32 = 12.0;
const LANE_NUMBER_SCALE: f32 = 1.6;

/// Where the measure at the judgement line is written: against the right edge, under the field.
const MEASURE_RIGHT_INSET: f32 = 40.0;
const MEASURE_Y: f32 = 690.0;

/// How many measures Page Up and Page Down move the preview by.
const MEASURES_PER_PAGE: i64 = 4;

/// The places of a side's key lanes, counted from 0, that work the list of captured layouts while
/// it has the keys: up, down, the other scope, and removal.
const LIST_KEY_UP: usize = 0;
const LIST_KEY_DOWN: usize = 1;
const LIST_KEY_SCOPE: usize = 2;
const LIST_KEY_REMOVE: usize = 3;

/// How long a refused key or a refused start is said for.
const NOTICE_SHOWN_FOR: Duration = Duration::from_secs(2);

/// What the rows of the two sides of a double chart are called.
const SIDE_NAMES: [&str; 2] = ["1P", "2P"];

/// The layout last written on LAYOUT PICK for each mode, by the mode's name: a row a side.
#[derive(Default)]
pub(crate) struct PickedLayouts(HashMap<&'static str, Vec<Vec<u8>>>);

impl PickedLayouts {
    fn of(&self, mode: Mode) -> Option<&Vec<Vec<u8>>> {
        self.0.get(mode.name)
    }

    fn keep(&mut self, mode: Mode, rows: Vec<Vec<u8>>) {
        self.0.insert(mode.name, rows);
    }
}

/// One player side's row as it is being written: the lane numbers typed so far, left to right. The
/// places not typed yet take the side's other lanes in order, so the row is a whole layout after
/// every key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SideRow {
    typed: Vec<u8>,
    keys: u8,
}

impl SideRow {
    /// A row of `keys` key lanes, written as `kept` when that is a whole row of them.
    pub(crate) fn new(keys: u8, kept: Option<&Vec<u8>>) -> SideRow {
        let typed = kept.filter(|row| is_layout_row(row, usize::from(keys))).cloned().unwrap_or_default();
        SideRow { typed, keys }
    }

    /// The whole row: what was typed, then the lanes not typed yet in order.
    pub(crate) fn row(&self) -> Vec<u8> {
        let rest = (1..=self.keys).filter(|number| !self.typed.contains(number));
        self.typed.iter().copied().chain(rest).collect()
    }

    /// How many places of the row were typed rather than filled in.
    pub(crate) fn typed_len(&self) -> usize {
        self.typed.len()
    }

    /// Put lane `number` in the next place. A row typed to its end is begun again with it. A number
    /// that is not one of the side's lanes, or that the row already has, is refused.
    pub(crate) fn type_lane(&mut self, number: u8) -> bool {
        if !(1..=self.keys).contains(&number) {
            return false;
        }
        if self.typed.len() == usize::from(self.keys) {
            self.typed.clear();
        }
        if self.typed.contains(&number) {
            return false;
        }
        self.typed.push(number);
        true
    }

    /// Take back the last place typed.
    pub(crate) fn erase(&mut self) {
        self.typed.pop();
    }

    /// Take back every place typed.
    pub(crate) fn clear(&mut self) {
        self.typed.clear();
    }

    /// Type the row as the lanes are, OFF's order.
    pub(crate) fn in_order(&mut self) {
        self.typed = (1..=self.keys).collect();
    }
}

/// The chart as a run of it is laid out before its lanes are, read on the reading thread: its notes
/// with the long notes the LN MODE decides resolved, its hash and its mode.
struct ReadChart {
    model: Model,
    md5: String,
    mode: Mode,
}

/// Where the preview stands.
enum Preview {
    /// The chart is being read.
    Reading(Receiver<Option<ReadChart>>),
    /// The file is not a chart, or could not be read.
    Failed,
    /// The chart, laid out as the rows are now, where each of its measures starts, where the
    /// preview begins it again, and its tempos. The two charts are boxed so a preview still being
    /// read is not as large as one that has been.
    Read { chart: Box<ReadChart>, laid_out: Box<Model>, measures: Vec<i64>, end_us: i64, bpm: BpmStats },
}

/// Where the seed of the layout stands.
enum SeedLook {
    /// The chart's mode has no seed to look for.
    Unsupported,
    /// A thread is looking for the seed of `perm`.
    Looking { perm: Vec<usize>, cancel: Arc<AtomicBool>, reply: Receiver<Result<u64, SeedSearchError>> },
    /// The search for the seed of `perm` came to an end.
    Done { perm: Vec<usize>, answer: Result<u64, SeedSearchError> },
    /// The rows are a captured layout, which is played on its own arrangement and seed.
    Captured,
}

/// Where the keys go: to the rows, or to the list of captured layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Rows,
    Captured,
}

/// Something said for a while under the status line.
#[derive(Clone, Debug)]
enum Notice {
    /// A lane number refused to a row.
    DigitRefused(u8),
    /// A start refused, or the list worked, for a reason of this screen.
    Says(LayoutPickText),
    /// A start the borrowed layout refused.
    Refusal(LayoutRefusal),
}

impl Notice {
    fn text(&self, language: Language) -> String {
        match self {
            Notice::DigitRefused(number) => LayoutPickText::DigitRefused.fill(language, &[number]),
            Notice::Says(text) => text.get(language).to_string(),
            Notice::Refusal(refusal) => refusal.in_language(language),
        }
    }
}

/// The LAYOUT PICK screen's own state.
pub(crate) struct LayoutPickState {
    chart_index: usize,
    /// The chart as the library knows it: what the run is asked for, and what the read chart has to be.
    md5: String,
    mode: Mode,
    title: String,
    /// The key lanes of each side, left to right.
    lanes: Vec<Vec<usize>>,
    rows: Vec<SideRow>,
    /// The side a typed number goes to.
    side: usize,
    preview: Preview,
    seed: SeedLook,
    /// Where the preview stands in the chart, and whether it plays.
    clock: PreviewClock,
    /// Turns of the wheel not yet taken as a measure or a row of the list.
    wheel: f32,
    focus: Focus,
    list: CapturedList,
    /// The captured layout last shown on the rows; it is played on its own terms while the rows
    /// are still as it wrote them.
    loaded: Option<CapturedLayout>,
    notice: Option<(Notice, Instant)>,
    /// The keys of the chart's mode, which may not be the mode last played.
    lane_keys: Vec<(KeyCode, usize)>,
    reverse_keys: Vec<(KeyCode, usize)>,
    skin: Skin,
    auto_lanes: Vec<bool>,
    auto_note: Color,
    legacy_note: bool,
}

impl LayoutPickState {
    /// Open the screen on the library's chart `chart_index`, written as the layout last written for its
    /// mode, and start reading the chart.
    pub(crate) fn open(shared: &AppShared, chart_index: usize) -> LayoutPickState {
        let entry = shared.library.songs().get(chart_index);
        let mode = entry.map_or(Mode::BEAT_7K, |entry| entry.mode);
        let path = entry.map(|entry| entry.path.clone()).unwrap_or_default();
        let title = entry.map(|entry| format!("{} {}", entry.title, entry.subtitle).trim_end().to_string()).unwrap_or_default();
        let md5 = entry.map(|entry| entry.md5.clone()).unwrap_or_default();
        let lanes: Vec<Vec<usize>> = (0..usize::from(mode.player.max(1))).map(|side| side_key_lanes(mode, side)).collect();
        let kept = shared.picked_layouts.of(mode);
        let rows = lanes
            .iter()
            .enumerate()
            .map(|(side, keys)| SideRow::new(u8::try_from(keys.len()).unwrap_or(u8::MAX), kept.and_then(|rows| rows.get(side))))
            .collect();
        let ln = crate::judge_setup::run_judge_setup(&shared.config, None).ln_mode.resolve();
        let mut state = LayoutPickState {
            chart_index,
            md5,
            mode,
            title,
            lanes,
            rows,
            side: 0,
            preview: Preview::Reading(read_chart(path, ln)),
            seed: SeedLook::Unsupported,
            clock: PreviewClock::default(),
            wheel: 0.0,
            focus: Focus::Rows,
            list: CapturedList::default(),
            loaded: None,
            notice: None,
            lane_keys: shared.keyconfig.lane_keys(mode),
            reverse_keys: shared.keyconfig.scratch_reverse_keys(mode),
            skin: preview_skin(shared, mode),
            auto_lanes: scratch_auto_lanes(mode, shared.config.play.scratch_auto),
            auto_note: Color::from(scratch_auto_color(&shared.config.play)),
            legacy_note: shared.config.play.legacy_note,
        };
        state.relay_out();
        state
    }

    /// The rows as they stand, a whole layout a side.
    pub(crate) fn layout(&self) -> Vec<Vec<u8>> {
        self.rows.iter().map(SideRow::row).collect()
    }

    /// The rows written out, sides apart by a slash.
    fn layout_text(&self) -> String {
        self.layout().iter().map(|row| layout_row_text(row)).collect::<Vec<_>>().join(" / ")
    }

    /// Whether the chart's mode is one a layout can be written for.
    fn writable(&self) -> bool {
        random_seed_limit(self.mode).is_some()
    }

    /// The lane permutation the rows stand for.
    fn permutation(&self) -> Option<Vec<usize>> {
        permutation_of_layout(self.mode, &self.layout())
    }

    /// The captured layout the rows are written as, while they still are.
    fn captured_on_show(&self) -> Option<&CapturedLayout> {
        let layout = self.layout();
        self.loaded.as_ref().filter(|capture| capture.rows(self.mode).as_ref() == Some(&layout))
    }

    /// The seed of the layout as the rows stand, once it has been found.
    pub(crate) fn found_seed(&self) -> Option<u64> {
        match &self.seed {
            SeedLook::Done { perm, answer: Ok(seed) } if Some(perm) == self.permutation().as_ref() => Some(*seed),
            _ => None,
        }
    }

    /// Lay the preview out again and look for the seed again, after the rows changed, where the
    /// preview stands. A seed still being looked for is given up, and none is looked for while the
    /// rows are a captured layout, which brings its own.
    fn relay_out(&mut self) {
        if let SeedLook::Looking { cancel, .. } = &self.seed {
            cancel.store(true, Ordering::Relaxed);
        }
        let (Some(perm), Some(limit)) = (self.permutation(), random_seed_limit(self.mode)) else {
            self.seed = SeedLook::Unsupported;
            return;
        };
        if let Preview::Read { chart, laid_out, .. } = &mut self.preview {
            **laid_out = chart.model.clone();
            apply_lane_permutation(laid_out, &perm);
        }
        self.seed = match self.captured_on_show() {
            Some(_) => SeedLook::Captured,
            None => look_for_seed(self.mode, perm, limit),
        };
    }

    /// Change the row of `side` by `edit`, and lay the preview out again.
    fn edit(&mut self, side: usize, edit: impl FnOnce(&mut SideRow)) {
        if !self.writable() {
            return;
        }
        if let Some(row) = self.rows.get_mut(side) {
            edit(row);
            self.relay_out();
        }
    }

    /// Put lane `number` in the next place of the row of `side`, or say why it is not put there.
    fn type_lane(&mut self, side: usize, number: u8, now: Instant) {
        if !self.writable() {
            return;
        }
        let typed = self.rows.get_mut(side).is_some_and(|row| row.type_lane(number));
        match typed {
            true => self.relay_out(),
            false => self.notice = Some((Notice::DigitRefused(number), now)),
        }
    }

    /// Write the rows as `capture` and lay the preview out so, when it is a capture of this chart
    /// whose rows can be written.
    fn show_capture(&mut self, capture: CapturedLayout) {
        if !self.writable() || !capture.md5.eq_ignore_ascii_case(&self.md5) {
            return;
        }
        let Some(rows) = capture.rows(self.mode) else {
            return;
        };
        self.rows = self.lanes.iter().zip(&rows).map(|(keys, row)| SideRow::new(u8::try_from(keys.len()).unwrap_or(u8::MAX), Some(row))).collect();
        self.loaded = Some(capture);
        self.relay_out();
    }

    /// Show the capture under the list's cursor, while the list has the keys.
    fn show_selected(&mut self, shared: &AppShared) {
        if self.focus != Focus::Captured {
            return;
        }
        if let Some(capture) = self.list.selected(&shared.captured_layouts, &self.md5) {
            self.show_capture(capture);
        }
    }

    /// The side a typed number goes to next.
    #[cfg(test)]
    pub(crate) fn side(&self) -> usize {
        self.side
    }

    fn next_side(&mut self) {
        self.side = (self.side + 1) % self.rows.len().max(1);
    }

    /// SELECT: the next side's row, after the last side's the list of captured layouts, and after
    /// the list the first side's row again.
    fn next_focus(&mut self, shared: &AppShared) {
        match self.focus {
            Focus::Captured => {
                self.focus = Focus::Rows;
                self.side = 0;
            }
            Focus::Rows if self.side + 1 < self.rows.len() => self.side += 1,
            Focus::Rows => self.focus_captured(shared),
        }
    }

    /// Give the keys to the list of captured layouts, and show the capture under its cursor.
    fn focus_captured(&mut self, shared: &AppShared) {
        self.focus = Focus::Captured;
        self.show_selected(shared);
    }

    /// A lane of the chart's mode pressed: a key lane is typed into its side's row as that side's
    /// lane number, and the turntable moves the preview a measure, later turned forwards and earlier
    /// turned back. While the list of captured layouts has the keys they work the list instead
    /// ([`LayoutPickState::list_lane`]).
    fn press_lane(&mut self, shared: &mut AppShared, lane: usize, dir: ScratchDir, now: Instant) {
        let backward = matches!(dir, ScratchDir::Backward);
        let place = self.place_of(lane);
        if self.focus == Focus::Captured {
            self.list_lane(shared, place.map(|(_, at)| at), backward, now);
            return;
        }
        if self.mode.is_scratch(lane) {
            self.move_measures(if backward { -1 } else { 1 });
            return;
        }
        if let Some((side, at)) = place {
            self.side = side;
            self.type_lane(side, u8::try_from(at + 1).unwrap_or(u8::MAX), now);
        }
    }

    /// The side a key lane of the chart's mode is on, and where it is among that side's key lanes;
    /// `None` for the turntable.
    fn place_of(&self, lane: usize) -> Option<(usize, usize)> {
        self.lanes.iter().enumerate().find_map(|(side, keys)| keys.iter().position(|key| *key == lane).map(|at| (side, at)))
    }

    /// Whether `code` is a key that removes the capture under the list's cursor: Delete, or, while
    /// the list has the keys, the key of a side's fourth key lane. A removal waits for its second
    /// press through these and is taken back by any other key.
    fn removes_a_capture(&self, shared: &AppShared, code: KeyCode) -> bool {
        let removal_lane = |lane: usize| self.place_of(lane).is_some_and(|(_, at)| at == LIST_KEY_REMOVE);
        code == KeyCode::Delete || (self.focus == Focus::Captured && self.lane_of(shared, code).is_some_and(|(lane, _)| removal_lane(lane)))
    }

    /// The lane of the chart's mode `code` plays and the way it spins it, or `None` for a key that
    /// plays none. The press that made two keys held together plays none either: it is theirs, as
    /// it is on every screen but a run ([`crate::keyconfig::HeldKeys::press_made_a_chord`]).
    fn lane_of(&self, shared: &AppShared, code: KeyCode) -> Option<(usize, ScratchDir)> {
        if shared.keyconfig.held.press_made_a_chord(code) {
            return None;
        }
        let lane = self.lane_keys.iter().find(|(key, _)| *key == code).map(|(_, lane)| (*lane, ScratchDir::Forward));
        lane.or_else(|| self.reverse_keys.iter().find(|(key, _)| *key == code).map(|(_, lane)| (*lane, ScratchDir::Backward)))
    }

    /// The menu button `code` is the key of alone, START or SELECT, or `None` for any other key and
    /// for the press that made two keys held together.
    fn menu_button_of(shared: &AppShared, code: KeyCode) -> Option<ControlAction> {
        if shared.keyconfig.held.press_made_a_chord(code) {
            return None;
        }
        [ControlAction::Start, ControlAction::Select].into_iter().find(|action| shared.keyconfig.control_key(*action) == Some(code))
    }

    /// A lane pressed while the list of captured layouts has the keys, `place` being where it is
    /// among its side's key lanes and `None` for the turntable: the turntable and the first two
    /// keys move the cursor, the third turns to the other scope, and the fourth removes the capture
    /// on a second press.
    fn list_lane(&mut self, shared: &mut AppShared, place: Option<usize>, backward: bool, now: Instant) {
        if place == Some(LIST_KEY_REMOVE) {
            self.press_remove(shared, now);
            return;
        }
        self.list.disarm();
        match place {
            None if backward => self.step_list(shared, -1),
            None => self.step_list(shared, 1),
            Some(LIST_KEY_UP) => self.step_list(shared, -1),
            Some(LIST_KEY_DOWN) => self.step_list(shared, 1),
            Some(LIST_KEY_SCOPE) => self.turn_scope(shared),
            Some(_) => {}
        }
    }

    /// Move the list's cursor `by` rows and show the capture it comes to.
    fn step_list(&mut self, shared: &AppShared, by: i64) {
        let len = self.list.entries(&shared.captured_layouts, &self.md5).len();
        self.list.step(by, len);
        self.show_selected(shared);
    }

    /// Turn the list to the chart's captures or to every chart's, and show the capture under the
    /// cursor.
    fn turn_scope(&mut self, shared: &AppShared) {
        self.list.turn_scope(&shared.captured_layouts, &self.md5);
        self.show_selected(shared);
    }

    /// The removal key on the list: the first press says a second one removes the capture under
    /// the cursor, and the second removes it from the file.
    fn press_remove(&mut self, shared: &mut AppShared, now: Instant) {
        let Some(selected) = self.list.selected(&shared.captured_layouts, &self.md5) else {
            return;
        };
        let Some(capture) = self.list.press_remove(selected) else {
            self.notice = Some((Notice::Says(LayoutPickText::RemoveArmed), now));
            return;
        };
        shared.captured_layouts.remove(&capture);
        self.notice = Some((Notice::Says(LayoutPickText::Removed), now));
        if self.loaded.as_ref().is_some_and(|loaded| loaded.same_layout(&capture)) {
            self.loaded = None;
            self.relay_out();
        }
        self.step_list(shared, 0);
    }

    /// The capture under the list's cursor while the list has the keys, when it is of another chart.
    fn selected_elsewhere(&self, shared: &AppShared) -> Option<CapturedLayout> {
        if self.focus != Focus::Captured {
            return None;
        }
        let selected = self.list.selected(&shared.captured_layouts, &self.md5)?;
        (!selected.md5.eq_ignore_ascii_case(&self.md5)).then_some(selected)
    }

    /// Enter and START: on a capture of another chart in the list, go to that chart; otherwise play
    /// the rows ([`LayoutPickState::start`]).
    fn confirm(&mut self, shared: &mut AppShared, now: Instant) -> Transition {
        match self.selected_elsewhere(shared) {
            Some(capture) => {
                self.go_to_chart(shared, &capture, now);
                Transition::Stay
            }
            None => self.start(shared, now),
        }
    }

    /// Open the screen on the chart of `capture`, with the list on it and the rows written as it,
    /// or say that the library has no such chart.
    fn go_to_chart(&mut self, shared: &mut AppShared, capture: &CapturedLayout, now: Instant) {
        let Some(chart_index) = shared.library.songs().iter().position(|entry| entry.md5.eq_ignore_ascii_case(&capture.md5)) else {
            self.notice = Some((Notice::Says(LayoutPickText::CapturedChartMissing), now));
            return;
        };
        self.leave(shared);
        let mut there = LayoutPickState::open(shared, chart_index);
        there.focus = Focus::Captured;
        there.list.scope = self.list.scope;
        there.list.point_at(&shared.captured_layouts, &there.md5, capture);
        there.show_capture(capture.clone());
        *self = there;
    }

    /// Give up a seed still being looked for, and keep what the rows hold for the mode.
    fn leave(&mut self, shared: &mut AppShared) {
        if let SeedLook::Looking { cancel, .. } = &self.seed {
            cancel.store(true, Ordering::Relaxed);
        }
        if self.writable() {
            shared.picked_layouts.keep(self.mode, self.layout());
        }
    }

    /// Where each measure of the read chart starts, or none before it is read.
    fn measures(&self) -> &[i64] {
        match &self.preview {
            Preview::Read { measures, .. } => measures,
            _ => &[],
        }
    }

    /// The measure at the judgement line of the preview.
    pub(crate) fn measure(&self) -> usize {
        self.clock.measure(self.measures())
    }

    /// Where the preview stands, for the tests that check its clock.
    #[cfg(test)]
    pub(crate) fn clock(&self) -> PreviewClock {
        self.clock
    }

    /// The chart as the preview has it laid out, once it is read, for the tests that hold it
    /// against a run.
    #[cfg(test)]
    pub(crate) fn previewed(&self) -> Option<&Model> {
        match &self.preview {
            Preview::Read { laid_out, .. } => Some(laid_out),
            _ => None,
        }
    }

    fn move_measures(&mut self, by: i64) {
        if let Preview::Read { measures, .. } = &self.preview {
            self.clock.move_measures(measures, by);
        }
    }

    /// The scroll speed the preview is drawn at, which is the one a run of this chart would start at:
    /// the one the settings hold, or, while a green number is held from chart to chart, the one that
    /// holds it on this chart under the lane cover as it stands.
    fn shown_hispeed(&self, play: &PlayOptions) -> f64 {
        let Preview::Read { bpm, .. } = &self.preview else {
            return play.hispeed;
        };
        let held = bpm.held(play).and_then(|bpm| held_speed(&HeldTravel::of(play, bpm)));
        held.map_or(play.hispeed, |held| held.hispeed)
    }

    /// Take what the reading thread and the seed search have finished since the last frame.
    fn poll(&mut self) {
        if let Preview::Reading(reply) = &self.preview {
            match reply.try_recv() {
                Ok(Some(chart)) => {
                    let measures = measure_starts(&chart.model.timelines);
                    let end_us = end_of(chart.model.timelines.last().map_or(0, |line| line.time_us));
                    let bpm = BpmStats::of(&chart.model);
                    let laid_out = Box::new(chart.model.clone());
                    self.preview = Preview::Read { chart: Box::new(chart), laid_out, measures, end_us, bpm };
                    self.relay_out();
                }
                Ok(None) | Err(TryRecvError::Disconnected) => self.preview = Preview::Failed,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let SeedLook::Looking { perm, reply, .. } = &self.seed {
            let answer = match reply.try_recv() {
                Ok(answer) => answer,
                Err(TryRecvError::Disconnected) => Err(SeedSearchError::Cancelled),
                Err(TryRecvError::Empty) => return,
            };
            self.seed = SeedLook::Done { perm: perm.clone(), answer };
        }
    }

    /// Whether the read chart is the one the library scanned: the run reads the file again, and a
    /// file that changed since would be played on the player's own arrangement instead.
    fn chart_changed(&self) -> bool {
        match &self.preview {
            Preview::Read { chart, .. } => !chart.md5.eq_ignore_ascii_case(&self.md5) || chart.mode != self.mode,
            _ => false,
        }
    }

    /// Why the layout cannot be played yet, or `None` when it can be.
    fn not_playable(&self, shared: &AppShared) -> Option<LayoutPickText> {
        if shared.run_plays_itself() {
            return Some(LayoutPickText::AutoplayOn);
        }
        match (&self.preview, &self.seed) {
            (Preview::Failed, _) => Some(LayoutPickText::ReadFailed),
            (Preview::Reading(_), _) => Some(LayoutPickText::NotReady),
            _ if self.chart_changed() => Some(LayoutPickText::ChartChanged),
            (_, SeedLook::Unsupported) => Some(LayoutPickText::UnsupportedMode),
            (_, SeedLook::Done { answer: Err(SeedSearchError::Unreachable), .. }) => Some(LayoutPickText::Unreachable),
            (_, SeedLook::Captured) if self.captured_on_show().is_some() => None,
            _ if self.found_seed().is_none() => Some(LayoutPickText::NotReady),
            _ => None,
        }
    }

    /// What the run is asked to borrow: a captured layout on show is played on its own arrangement
    /// and seed, and said to be of the moment it was captured; any other layout is played as RANDOM
    /// with the seed found for it.
    fn request(&self) -> Option<LayoutRequest> {
        if let (SeedLook::Captured, Some(capture)) = (&self.seed, self.captured_on_show()) {
            let source = fmt_datetime(capture.captured_at);
            return Some(LayoutRequest { md5: capture.md5.clone(), random_label: capture.random_label.clone(), seed: capture.seed, source });
        }
        let seed = self.found_seed()?;
        let source = LayoutPickText::Source.fill(language(), &[&self.layout_text()]);
        Some(LayoutRequest { md5: self.md5.clone(), random_label: NoteOption::Random.label().to_string(), seed, source })
    }

    /// Play the chart laid out as the rows are, on a layout borrowed for the run ([`LayoutPickState::request`]).
    /// The run takes this screen's place over the browser. A layout that cannot be played yet stays,
    /// and the screen says why.
    fn start(&mut self, shared: &mut AppShared, now: Instant) -> Transition {
        if let Some(why) = self.not_playable(shared) {
            self.notice = Some((Notice::Says(why), now));
            return Transition::Stay;
        }
        let Some(request) = self.request() else {
            return Transition::Stay;
        };
        match shared.borrow_layout(request, self.chart_index) {
            Ok(Transition::Open(stage)) => Transition::To(stage),
            Ok(other) => other,
            Err(refusal) => {
                self.notice = Some((Notice::Refusal(refusal), now));
                Transition::Stay
            }
        }
    }

    /// A key that is neither a fixed key of the screen nor one that moves the preview: a lane of the
    /// chart's mode, a number, START or SELECT.
    fn other_key(&mut self, shared: &mut AppShared, code: KeyCode, now: Instant) -> Transition {
        if let Some((lane, dir)) = self.lane_of(shared, code) {
            self.press_lane(shared, lane, dir, now);
            return Transition::Stay;
        }
        match number_of(code) {
            Some(_) if self.focus == Focus::Captured => {}
            Some(0) => self.edit(self.side, SideRow::in_order),
            Some(number) => self.type_lane(self.side, number, now),
            None => match LayoutPickState::menu_button_of(shared, code) {
                Some(ControlAction::Start) => return self.confirm(shared, now),
                Some(ControlAction::Select) => self.next_focus(shared),
                _ => {}
            },
        }
        Transition::Stay
    }

    /// A key that works the list of captured layouts while it has the keys, or else whatever the
    /// key does on the rows ([`LayoutPickState::other_key`]).
    fn list_key(&mut self, shared: &mut AppShared, code: KeyCode, now: Instant) -> Transition {
        match code {
            KeyCode::ArrowUp => self.step_list(shared, -1),
            KeyCode::ArrowDown => self.step_list(shared, 1),
            KeyCode::Tab => self.turn_scope(shared),
            KeyCode::Delete => self.press_remove(shared, now),
            code => return self.other_key(shared, code, now),
        }
        Transition::Stay
    }

    /// The status line: where the chart and the seed stand.
    fn status(&self, language: Language) -> String {
        match (&self.preview, &self.seed) {
            (Preview::Reading(_), _) => LayoutPickText::Reading.get(language).to_string(),
            (Preview::Failed, _) => LayoutPickText::ReadFailed.get(language).to_string(),
            _ if self.chart_changed() => LayoutPickText::ChartChanged.get(language).to_string(),
            (_, SeedLook::Unsupported) => LayoutPickText::UnsupportedMode.fill(language, &[&self.mode.name]),
            (_, SeedLook::Looking { .. }) => LayoutPickText::Searching.get(language).to_string(),
            (_, SeedLook::Done { answer: Ok(seed), .. }) => LayoutPickText::SeedFound.fill(language, &[seed]),
            (_, SeedLook::Done { answer: Err(SeedSearchError::Unreachable), .. }) => LayoutPickText::Unreachable.get(language).to_string(),
            (_, SeedLook::Captured) => match self.captured_on_show() {
                Some(capture) => LayoutPickText::CapturedSeed.fill(language, &[&capture.random_label, &capture.seed]),
                None => LayoutPickText::NotReady.get(language).to_string(),
            },
            (_, SeedLook::Done { .. }) => LayoutPickText::NotReady.get(language).to_string(),
        }
    }

    /// The rows over the field, each digit typed in the colour of its side and each filled in dimmed.
    fn draw_rows(&self, canvas: &mut Canvas<'_>) {
        let th = theme();
        let mut x = TEXT_X;
        for (side, row) in self.rows.iter().enumerate() {
            if self.rows.len() > 1 {
                let name = SIDE_NAMES.get(side).copied().unwrap_or_default();
                let colour = if side == self.side && self.focus == Focus::Rows { th.accent } else { th.text_dim };
                draw_text(canvas, x, LAYOUT_Y, LAYOUT_SCALE, colour, name);
                x += text_width(&format!("{name} "), LAYOUT_SCALE);
            }
            for (place, number) in row.row().iter().enumerate() {
                let digit = number.to_string();
                draw_text(canvas, x, LAYOUT_Y, LAYOUT_SCALE, self.digit_colour(side, place), &digit);
                x += text_width(&digit, LAYOUT_SCALE);
            }
            x += SIDE_GAP;
        }
    }

    fn digit_colour(&self, side: usize, place: usize) -> Color {
        let th = theme();
        let typed = self.rows.get(side).is_some_and(|row| place < row.typed_len());
        match (typed, side == self.side && self.focus == Focus::Rows) {
            (false, _) => th.text_muted,
            (true, true) => th.accent,
            (true, false) => th.text,
        }
    }

    /// Under each key lane of the field, the number of the lane its notes come from.
    fn draw_lane_numbers(&self, canvas: &mut Canvas<'_>) {
        for (side, (keys, row)) in self.lanes.iter().zip(&self.rows).enumerate() {
            for (place, (lane, number)) in keys.iter().zip(row.row()).enumerate() {
                let (Some(x), Some(w)) = (self.skin.x.get(*lane), self.skin.w.get(*lane)) else {
                    continue;
                };
                let digit = number.to_string();
                let left = x + (w - text_width(&digit, LANE_NUMBER_SCALE)) * 0.5;
                draw_text(canvas, left, self.skin.judge_y + LANE_NUMBER_DROP, LANE_NUMBER_SCALE, self.digit_colour(side, place), &digit);
            }
        }
    }

    /// The preview: the chart laid out as the rows are, where the clock stands, under the lane
    /// shades the player has on.
    fn draw_preview(&self, shared: &AppShared, canvas: &mut Canvas<'_>) {
        let Preview::Read { laid_out, .. } = &self.preview else {
            canvas.clear(theme().bg);
            return;
        };
        let play = &shared.config.play;
        let view = PlayfieldView {
            timelines: &laid_out.timelines,
            microtime: self.clock.at_us(),
            hispeed: self.shown_hispeed(play),
            beam_on: &[],
            beam_off: &[],
            constant: play.constant_speed,
            legacy_note: self.legacy_note,
            auto_lanes: &self.auto_lanes,
            auto_note: self.auto_note,
        };
        render_playfield_view(canvas, &self.skin, &view);
        render_lane_cover(canvas, &self.skin, LaneShade { cover: shared.effective_cover(), hidden: shared.effective_hidden() });
    }

    /// The key or the two keys a result screen captures a layout on, as the list names them, or
    /// `None` while the capture has no key.
    fn capture_key(shared: &AppShared) -> Option<String> {
        shared.keyconfig.shortcut_bind(Shortcut::CaptureLayout).map(KeyBind::label)
    }

    /// The list of captured layouts, beside the field.
    fn draw_list(&self, shared: &AppShared, canvas: &mut Canvas<'_>, language: Language) {
        let capture_key = LayoutPickState::capture_key(shared);
        let view = ListView { md5: &self.md5, focused: self.focus == Focus::Captured, capture_key: capture_key.as_deref(), language };
        self.list.draw(canvas, &shared.captured_layouts, &view);
    }
}

impl StageHandler for LayoutPickState {
    /// Take what the threads finished, and move a playing preview on by the frame's time.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.poll();
        if let Preview::Read { end_us, .. } = &self.preview {
            self.clock.advance(ctx.dt, *end_us);
        }
        Transition::Stay
    }

    /// Escape goes back to the browser and Enter plays the layout. Space plays and stops the
    /// preview, Page Up and Page Down move it four measures, Home and End to the chart's first and
    /// last. Right gives the keys to the list of captured layouts and Left back to the rows.
    ///
    /// On the rows Tab turns to the other side's row, a number or a lane key puts that lane in the
    /// next place, Backspace takes the last place back, Delete every place, and 0 types the row in
    /// order; up and down move the preview a measure, and so does the turntable. On the list up and
    /// down move the cursor, Tab turns to the other scope and Delete pressed twice removes the
    /// capture. START plays the layout as Enter does, and SELECT goes round the rows and the list.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if !key.pressed {
            return Transition::Stay;
        }
        if !self.removes_a_capture(ctx.shared, key.code) {
            self.list.disarm();
        }
        match key.code {
            KeyCode::Escape => return Transition::Back,
            KeyCode::Enter | KeyCode::NumpadEnter => return self.confirm(ctx.shared, ctx.now),
            KeyCode::Space => self.clock.toggle(),
            KeyCode::PageUp => self.move_measures(MEASURES_PER_PAGE),
            KeyCode::PageDown => self.move_measures(-MEASURES_PER_PAGE),
            KeyCode::Home => self.clock.seek(0),
            KeyCode::End => self.move_measures(i64::MAX),
            KeyCode::ArrowRight => self.focus_captured(ctx.shared),
            KeyCode::ArrowLeft => self.focus = Focus::Rows,
            code if self.focus == Focus::Captured => return self.list_key(ctx.shared, code, ctx.now),
            KeyCode::Tab => self.next_side(),
            KeyCode::Backspace => self.edit(self.side, SideRow::erase),
            KeyCode::Delete => self.edit(self.side, SideRow::clear),
            KeyCode::ArrowUp => self.move_measures(1),
            KeyCode::ArrowDown => self.move_measures(-1),
            code => return self.other_key(ctx.shared, code, ctx.now),
        }
        Transition::Stay
    }

    /// The wheel moves the preview a measure a line, rolled towards the player earlier, or the
    /// list's cursor a row while the list has the keys.
    fn handle_scroll(&mut self, ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        self.wheel += lines;
        while self.wheel.abs() >= 1.0 {
            let step = self.wheel.signum();
            self.wheel -= step;
            let by = if step > 0.0 { -1 } else { 1 };
            match self.focus {
                Focus::Rows => self.move_measures(by),
                Focus::Captured => self.step_list(ctx.shared, by),
            }
        }
        Transition::Stay
    }

    /// A controller's lane does what its key does, when the controller is read in the chart's mode;
    /// START plays the layout and SELECT goes round the rows and the list.
    fn handle_pad(&mut self, ctx: &mut FrameCtx<'_>, event: PadEvent) -> Transition {
        match event {
            PadEvent::Lane { lane, dir, press: true } if ctx.shared.mode == self.mode => self.press_lane(ctx.shared, lane, dir, ctx.now),
            PadEvent::Control(ControlAction::Start) => {
                self.list.disarm();
                return self.confirm(ctx.shared, ctx.now);
            }
            PadEvent::Control(ControlAction::Select) => {
                self.list.disarm();
                self.next_focus(ctx.shared);
            }
            _ => {}
        }
        Transition::Stay
    }

    /// The layout written is kept for the mode, and a seed still being looked for is given up.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.leave(ctx.shared);
    }

    /// The screen reads every key itself, so the option overlay does not open over it.
    fn holds_keys(&self, _ctx: &FrameCtx<'_>) -> bool {
        true
    }

    /// The preview first, since the playfield clears the frame; then the lines over it, the lane
    /// numbers under it and the list beside it.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = theme();
        let language = language();
        canvas.clear_bga();
        self.draw_preview(ctx.shared, canvas);
        draw_text(canvas, TEXT_X, HEADING_Y, HEADING_SCALE, th.text, LayoutPickText::ShortcutLabel.get(language));
        draw_text(canvas, TEXT_X, TITLE_Y, TITLE_SCALE, th.text_dim, &self.title);
        if self.writable() {
            self.draw_rows(canvas);
            self.draw_lane_numbers(canvas);
        }
        draw_text(canvas, TEXT_X, STATUS_Y, STATUS_SCALE, th.text, &self.status(language));
        let notice = self.notice.as_ref().filter(|(_, at)| ctx.now.saturating_duration_since(*at) < NOTICE_SHOWN_FOR).map(|(notice, _)| notice.text(language));
        let notice = notice.or_else(|| ctx.shared.run_plays_itself().then(|| LayoutPickText::AutoplayOn.get(language).to_string()));
        if let Some(notice) = notice {
            draw_text(canvas, TEXT_X, NOTICE_Y, STATUS_SCALE, th.accent, &notice);
        }
        let keys = if self.focus == Focus::Captured { LayoutPickText::HintCaptured } else { LayoutPickText::HintTyping };
        draw_text(canvas, TEXT_X, HINT_Y, HINT_SCALE, th.text_muted, keys.get(language));
        draw_text(canvas, TEXT_X, HINT_SECOND_Y, HINT_SCALE, th.text_muted, LayoutPickText::HintMoving.get(language));
        self.draw_list(ctx.shared, canvas, language);
        if !self.measures().is_empty() {
            let measure = LayoutPickText::Measure.fill(language, &[&(self.measure() + 1), &self.measures().len()]);
            let shown = match self.clock.is_playing() {
                true => measure,
                false => format!("{}   {measure}", LayoutPickText::Stopped.get(language)),
            };
            draw_text_right(canvas, CW as f32 - MEASURE_RIGHT_INSET, MEASURE_Y, STATUS_SCALE, th.text_dim, &shown);
        }
    }
}

/// The digit a number key stands for, on the row of numbers or on the keypad.
fn number_of(code: KeyCode) -> Option<u8> {
    let number = match code {
        KeyCode::Digit0 | KeyCode::Numpad0 => 0,
        KeyCode::Digit1 | KeyCode::Numpad1 => 1,
        KeyCode::Digit2 | KeyCode::Numpad2 => 2,
        KeyCode::Digit3 | KeyCode::Numpad3 => 3,
        KeyCode::Digit4 | KeyCode::Numpad4 => 4,
        KeyCode::Digit5 | KeyCode::Numpad5 => 5,
        KeyCode::Digit6 | KeyCode::Numpad6 => 6,
        KeyCode::Digit7 | KeyCode::Numpad7 => 7,
        KeyCode::Digit8 | KeyCode::Numpad8 => 8,
        KeyCode::Digit9 | KeyCode::Numpad9 => 9,
        _ => return None,
    };
    Some(number)
}

/// Where each measure of a chart starts: its start, and every bar line after it.
fn measure_starts(timelines: &[TimeLine]) -> Vec<i64> {
    let mut starts: Vec<i64> = std::iter::once(0).chain(timelines.iter().filter(|line| line.section_line).map(|line| line.time_us)).collect();
    starts.dedup();
    starts
}

/// The playfield the preview is drawn on: the built-in one the player has, with the turntable on the
/// side they play it on, the lift they have on, a double chart's sides apart and no picture beside
/// it, laid out across the middle of the screen under the lines over it.
fn preview_skin(shared: &AppShared, mode: Mode) -> Skin {
    let mut cfg = shared.skin_cfg.clone();
    let width = if mode.player > 1 { DUAL_FIELD_WIDTH } else { FIELD_WIDTH };
    cfg.scratch_left = shared.config.play.scratch_left;
    cfg.five_key_layout = shared.config.display.five_key_layout;
    cfg.lift = shared.effective_lift();
    cfg.bga = None;
    cfg.dual_field = true;
    cfg.top_y = FIELD_TOP;
    cfg.judge_y = FIELD_JUDGE;
    cfg.field_width = width;
    cfg.field_x = (1.0 - width) * 0.5;
    Skin::build(&cfg, mode, CW as f32, CH as f32)
}

/// Read the chart at `path` on a thread of its own, as a run of it is read, its undefined long notes
/// resolved as `ln`. The answer is `None` when the file is not a chart this build reads.
fn read_chart(path: PathBuf, ln: LnKind) -> Receiver<Option<ReadChart>> {
    let (answer, reply) = mpsc::channel();
    let read = move || {
        let decoded = std::fs::read(&path).ok().and_then(|bytes| decode_chart(&bytes, &path.to_string_lossy()));
        let chart = decoded.map(|decoded| {
            let mut model = decoded.model;
            rbms_chart::resolve_long_note_flavour(&mut model, ln);
            ReadChart { model, md5: decoded.md5, mode: decoded.mode }
        });
        answer.send(chart).ok();
    };
    std::thread::Builder::new().name(READ_THREAD_NAME.to_string()).spawn(read).ok();
    reply
}

/// Look for the seed of `perm` on a thread of its own, through `limit` seeds.
fn look_for_seed(mode: Mode, perm: Vec<usize>, limit: u64) -> SeedLook {
    let cancel = Arc::new(AtomicBool::new(false));
    let (answer, reply) = mpsc::channel();
    let (raised, wanted) = (Arc::clone(&cancel), perm.clone());
    let look = move || {
        answer.send(random_seed_for(mode, &wanted, limit, &raised)).ok();
    };
    std::thread::Builder::new().name(SEARCH_THREAD_NAME.to_string()).spawn(look).ok();
    SeedLook::Looking { perm, cancel, reply }
}

/// The names the two threads go by, for a debugger's list of threads.
const READ_THREAD_NAME: &str = "rbms-layout-pick-read";
const SEARCH_THREAD_NAME: &str = "rbms-layout-pick-seed";

mod captured;
mod clock;
#[cfg(test)]
mod tests;

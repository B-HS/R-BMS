//! The modal a chart a difficulty table lists and the library does not hold opens on.
//!
//! The reference hands the two links of such a chart straight to the browser
//! (`EventFactory.open_download_site`). This player says first what each link is: the DL of a table
//! is the chart file alone, a difference chart, and the song it is a chart of -- its sounds and its
//! pictures -- has to be got from the song's own page and put beside it. Then it offers, in one row
//! of buttons: getting the chart itself when its link is a zip archive the player can fetch, opening
//! the song's page, opening the chart's page, and closing the modal.
//!
//! The row is one ordered list ([`DownloadModal::buttons`]), and the drawing, the keys and the mouse
//! all go by its length and by which of its buttons are on. A button that is off is drawn dimmed
//! and the cursor steps over it. While the chart is being got, getting it gives its place to
//! cancelling it, and a line under the note says how far the job is ([`crate::download`]); while
//! another chart is being got, getting this one is off.
//!
//! Opening the modal reads nothing from the disk or the network: the table's entry is in memory. A
//! link is handed to the browser only once [`rbms_download::browsable_url`] has let it through
//! ([`crate::open_url`]), and getting the chart asks for the download folder first when none has
//! been chosen ([`crate::dialog::pick_download_folder`]).
//!
//! The modal takes every key, the controller's keys and the mouse before anything under it does,
//! the way the record modal does, whether a skin draws the browser or not.

use rbms_download::{LinkKind, browsable_url, classify_link};
use rbms_model::translations::Catalog;
use rbms_render::text::language;
use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_centered, fit_text, theme};
use rbms_table::TableEntry;

use super::keys::{KeyLayout, SelectKey};
use super::list::missing_chart_title;
use super::skinned::skin_status;
use super::{BUILT_IN_KEY_LAYOUT, SelectState};
use crate::dialog::{DialogHandle, DialogState};
use crate::download::{JobStatus, Phase, cancel_download, start_download};
use crate::keyconfig::{SCRATCH_BACKWARD_INDEX, SCRATCH_FORWARD_INDEX, key_index_of};
use crate::notify::{Level, notify, notify_text};
use crate::select_tree::{level_title, table_name};
use crate::stage::result::SkinStatus;
use crate::text::select::download_modal::DownloadModalText;
use crate::{AppShared, CH, CW, Hot, KeyCode, MissingRef, SelectItem};

/// What the modal says of the two links of a table's chart.
const NOTE_LINES: [DownloadModalText; 2] = [DownloadModalText::NoteChartFile, DownloadModalText::NoteSongUrl];

/// How dark the screen under the modal is made.
const BACKDROP_ALPHA: u8 = 180;

/// The panel: its size, the room left at its sides, and the lines of text in it, each as how far
/// down the panel it stands and how large it is set.
const PANEL_W: f32 = 1000.0;
const PANEL_H: f32 = 360.0;
const PANEL_PAD_X: f32 = 32.0;
const HAIRLINE: f32 = 1.0;
const HEADING_Y: f32 = 22.0;
const HEADING_SCALE: f32 = 1.2;
const TITLE_Y: f32 = 46.0;
const TITLE_SCALE: f32 = 2.0;
const ARTIST_Y: f32 = 88.0;
const ARTIST_SCALE: f32 = 1.3;
const PLACE_Y: f32 = 116.0;
const PLACE_SCALE: f32 = 1.1;
const NOTE_Y: f32 = 156.0;
const NOTE_GAP: f32 = 28.0;
const NOTE_SCALE: f32 = 1.3;
const STATUS_Y: f32 = 222.0;
const STATUS_SCALE: f32 = 1.1;

/// The row of buttons: how far above the panel's foot it stands, how tall a button is, the gap
/// between two, and how large a label is set. The buttons share the row's width equally, however
/// many there are.
const BUTTONS_FROM_FOOT: f32 = 96.0;
const BUTTON_H: f32 = 40.0;
const BUTTON_GAP: f32 = 16.0;
const BUTTON_SCALE: f32 = 1.3;
const FOCUS_OUTLINE: f32 = 1.5;

/// The key hint: how far above the panel's foot it stands and how large it is set.
const HINT_FROM_FOOT: f32 = 36.0;
const HINT_SCALE: f32 = 1.0;

/// The height of a line of text at scale one, which is what centres a label in its button.
const TEXT_LINE: f32 = 14.0;

/// A button of the modal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModalButton {
    /// Fetch the chart's zip archive and unpack it into the download folder.
    GetChart,
    /// Open the song's own page, where its sounds and pictures are got.
    OpenSongUrl,
    /// Open the page the chart's link leads to.
    OpenChartUrl,
    /// Close the modal.
    Close,
    /// Stop getting the chart, in the place of [`ModalButton::GetChart`] while it is being got.
    Cancel,
}

impl ModalButton {
    /// What the button says, in the language the screens are written in.
    fn label(self) -> &'static str {
        match self {
            ModalButton::GetChart => DownloadModalText::GetChart,
            ModalButton::OpenSongUrl => DownloadModalText::OpenSongUrl,
            ModalButton::OpenChartUrl => DownloadModalText::OpenChartUrl,
            ModalButton::Close => DownloadModalText::Close,
            ModalButton::Cancel => DownloadModalText::Cancel,
        }
        .get(language())
    }

    /// Whether the two buttons stand in one place of the row, which getting the chart and
    /// cancelling it do: the cursor on the one is on the other once it has taken its place.
    fn shares_place_with(self, other: ModalButton) -> bool {
        matches!((self, other), (ModalButton::GetChart | ModalButton::Cancel, ModalButton::GetChart | ModalButton::Cancel)) || self == other
    }
}

/// The modal over a chart the library does not hold: the table's entry of it, where the table lists
/// it, what its two links are, and the button the cursor is on.
pub(crate) struct DownloadModal {
    pub(crate) entry: TableEntry,
    pub(crate) table_title: String,
    pub(crate) level_title: String,
    /// What the chart's own link (`url_diff`) leads to.
    pub(crate) chart_link: LinkKind,
    /// The song's link (`url`) and the chart's, each only when it may be handed to the browser.
    pub(crate) song_url: Option<String>,
    pub(crate) chart_url: Option<String>,
    pub(crate) focus: ModalButton,
    /// The picker of the download folder getting the chart opened, while it is up.
    folder_picker: Option<DialogHandle>,
    /// How the download job stood when the browser last looked ([`crate::download::status`]).
    job: Option<JobStatus>,
}

impl DownloadModal {
    /// The modal over the chart `missing` names, or `None` when the table no longer lists one there.
    /// The cursor starts on the first button that is on.
    pub(crate) fn of(shared: &AppShared, missing: &MissingRef) -> Option<DownloadModal> {
        let entry = shared.missing_chart(missing)?.clone();
        let level =
            shared.table_levels.get(missing.table).and_then(|levels| levels.get(missing.level)).map_or(entry.level.as_str(), |(label, _)| label.as_str());
        let level_title = level_title(shared, missing.table, level);
        let mut modal = DownloadModal {
            table_title: table_name(shared, missing.table),
            level_title,
            chart_link: classify_link(&entry.url_diff),
            song_url: browsable_url(&entry.url),
            chart_url: browsable_url(&entry.url_diff),
            entry,
            focus: ModalButton::Close,
            folder_picker: None,
            job: crate::download::status(shared),
        };
        modal.focus = modal.buttons().into_iter().find(|(_, on)| *on).map_or(ModalButton::Close, |(button, _)| button);
        Some(modal)
    }

    /// The row of buttons in order, each with whether it is on. Getting the chart is on only for a
    /// link to a zip archive, while no job is getting a chart and the chart has not just been got;
    /// while a job is getting this chart, cancelling it stands in its place, on until the archive is
    /// unpacked. Opening a link is on only for one the browser may be handed.
    pub(crate) fn buttons(&self) -> Vec<(ModalButton, bool)> {
        let first = match self.own_job() {
            Some(job) if job.is_running() => (ModalButton::Cancel, job.can_cancel()),
            Some(JobStatus { phase: Phase::Done { found: true, .. }, .. }) => (ModalButton::GetChart, false),
            _ => (ModalButton::GetChart, self.chart_link == LinkKind::DirectZip && !self.job.as_ref().is_some_and(JobStatus::is_running)),
        };
        vec![first, (ModalButton::OpenSongUrl, self.song_url.is_some()), (ModalButton::OpenChartUrl, self.chart_url.is_some()), (ModalButton::Close, true)]
    }

    /// The download job, when it is getting this chart or last got it.
    fn own_job(&self) -> Option<&JobStatus> {
        self.job.as_ref().filter(|job| job.md5.eq_ignore_ascii_case(self.entry.md5.trim()))
    }

    /// The line under the note: the folder picker being up, how far the job getting this chart is
    /// or how it ended, or another chart being got.
    fn status_line(&self) -> Option<String> {
        if self.folder_picker.is_some() {
            return Some(DownloadModalText::PickingTheFolder.get(language()).to_owned());
        }
        if let Some(job) = self.own_job() {
            return Some(job.line());
        }
        self.job.as_ref().filter(|job| job.is_running()).map(|_| DownloadModalText::AnotherDownloadIsRunning.get(language()).to_owned())
    }

    /// The button the cursor is on as the row stands now, when it is on.
    fn focused_button(&self) -> Option<ModalButton> {
        self.buttons().into_iter().find(|(button, on)| *on && button.shares_place_with(self.focus)).map(|(button, _)| button)
    }

    /// Move the cursor to the next button that is on (`forward`) or the one before, round the ends.
    fn step_focus(&mut self, forward: bool) {
        let buttons = self.buttons();
        let count = buttons.len();
        let at = buttons.iter().position(|(button, _)| button.shares_place_with(self.focus)).unwrap_or_default();
        let next = (1..count).map(|step| if forward { (at + step) % count } else { (at + count - step) % count }).find(|index| buttons[*index].1);
        if let Some(index) = next {
            self.focus = buttons[index].0;
        }
    }
}

/// What the modal reaches outside the player with: the system's browser and the folder picker.
/// They are held apart so that a test can hand in its own and see what would have been opened.
pub(super) struct DownloadHands {
    pub(super) open_url: Box<dyn FnMut(&str)>,
    pub(super) pick_folder: Box<dyn FnMut() -> DialogHandle>,
}

/// The system's browser and its folder picker.
#[cfg(not(test))]
impl Default for DownloadHands {
    fn default() -> DownloadHands {
        DownloadHands { open_url: Box::new(crate::open_url::open_in_browser), pick_folder: Box::new(crate::dialog::pick_download_folder) }
    }
}

/// A test is never handed the system's browser or its folder picker: one that has not put hands of
/// its own in fails the moment the modal reaches for either.
#[cfg(test)]
impl Default for DownloadHands {
    fn default() -> DownloadHands {
        DownloadHands {
            open_url: Box::new(|url| panic!("a test reached for the system's browser with {url}")),
            pick_folder: Box::new(|| panic!("a test reached for the system's folder picker")),
        }
    }
}

/// Start getting the chart, and say why when it cannot be.
fn begin_download(shared: &mut AppShared, entry: &TableEntry) {
    if let Err(why) = start_download(shared, entry) {
        notify(Level::Warn, why);
    }
}

/// Where a line of text `scale` large goes in a box `height` tall from `top` to be in its middle.
fn centred_text_y(top: f32, height: f32, scale: f32) -> f32 {
    top + (height - scale * TEXT_LINE) * 0.5
}

/// A line round a rectangle.
fn outline<R: Renderer>(r: &mut R, rect: Rect, thickness: f32, color: Color) {
    r.fill_rect(Rect::new(rect.x, rect.y, rect.w, thickness), color);
    r.fill_rect(Rect::new(rect.x, rect.y + rect.h - thickness, rect.w, thickness), color);
    r.fill_rect(Rect::new(rect.x, rect.y, thickness, rect.h), color);
    r.fill_rect(Rect::new(rect.x + rect.w - thickness, rect.y, thickness, rect.h), color);
}

/// Draw the modal over whatever the browser drew, and record where it takes a press: the whole
/// screen, so that a press beside a button reaches nothing under it, and each button that is on.
pub(super) fn render_download_modal<R: Renderer>(r: &mut R, modal: &DownloadModal, hot: &mut Vec<(Rect, Hot)>) {
    let th = theme();
    let screen = Rect::new(0.0, 0.0, CW as f32, CH as f32);
    r.fill_rect(screen, Color { a: BACKDROP_ALPHA, ..Color::BLACK });
    hot.push((screen, Hot::OverlayPanel));
    let panel = Rect::new((screen.w - PANEL_W) * 0.5, (screen.h - PANEL_H) * 0.5, PANEL_W, PANEL_H);
    r.fill_rect(panel, th.panel_hi);
    outline(r, panel, HAIRLINE, th.divider);
    let x = panel.x + PANEL_PAD_X;
    let room = panel.w - PANEL_PAD_X * 2.0;
    draw_text(r, x, panel.y + HEADING_Y, HEADING_SCALE, th.accent, DownloadModalText::Heading.get(language()));
    draw_text(r, x, panel.y + TITLE_Y, TITLE_SCALE, th.text, &fit_text(missing_chart_title(&modal.entry), TITLE_SCALE, room));
    draw_text(r, x, panel.y + ARTIST_Y, ARTIST_SCALE, th.text_dim, &fit_text(modal.entry.artist.trim(), ARTIST_SCALE, room));
    let place = format!("{} {}", modal.table_title, modal.level_title);
    draw_text(r, x, panel.y + PLACE_Y, PLACE_SCALE, th.text_muted, &fit_text(&place, PLACE_SCALE, room));
    for (line, text) in NOTE_LINES.iter().enumerate() {
        draw_text(r, x, panel.y + NOTE_Y + NOTE_GAP * line as f32, NOTE_SCALE, th.text, &fit_text(text.get(language()), NOTE_SCALE, room));
    }
    if let Some(status) = modal.status_line() {
        draw_text(r, x, panel.y + STATUS_Y, STATUS_SCALE, th.accent, &fit_text(&status, STATUS_SCALE, room));
    }
    let focus = modal.focused_button();
    let buttons = modal.buttons();
    let count = buttons.len() as f32;
    let width = (room - BUTTON_GAP * (count - 1.0)) / count;
    let top = panel.y + panel.h - BUTTONS_FROM_FOOT;
    for (slot, (button, on)) in buttons.into_iter().enumerate() {
        let rect = Rect::new(x + (width + BUTTON_GAP) * slot as f32, top, width, BUTTON_H);
        let focused = focus == Some(button);
        r.fill_rect(rect, if focused { th.button_active } else { th.button });
        if focused {
            outline(r, rect, FOCUS_OUTLINE, th.focus);
        }
        let color = if on { th.text } else { th.text_muted };
        let label = fit_text(button.label(), BUTTON_SCALE, rect.w);
        draw_text_centered(r, rect.x + rect.w * 0.5, centred_text_y(rect.y, rect.h, BUTTON_SCALE), BUTTON_SCALE, color, &label);
        if on {
            hot.push((rect, Hot::DownloadButton(slot)));
        }
    }
    draw_text(r, x, panel.y + panel.h - HINT_FROM_FOOT, HINT_SCALE, th.text_muted, DownloadModalText::KeyHint.get(language()));
}

impl SelectState {
    /// Open the modal over the chart `missing` names, which closes the record modal. A chart the
    /// table no longer lists there is only said to be missing.
    pub(super) fn open_download_modal(&mut self, shared: &AppShared, missing: &MissingRef) {
        match DownloadModal::of(shared, missing) {
            Some(modal) => {
                self.record_modal = None;
                self.download_modal = Some(modal);
            }
            None => notify_text(Level::Info, DownloadModalText::NotInTheLibrary, &[]),
        }
    }

    /// Open the modal over the bar under the cursor, when that is a chart the library does not hold.
    /// Answers whether it is one.
    pub(super) fn open_download_modal_at_cursor(&mut self, shared: &AppShared) -> bool {
        let Some(SelectItem::Missing(missing)) = shared.select_items.get(shared.sel) else {
            return false;
        };
        let missing = *missing;
        self.open_download_modal(shared, &missing);
        true
    }

    /// One key going down while the modal is up: the arrows across move the cursor, Enter presses
    /// the button under it and Esc closes the modal. A key bound to a lane does what its controller
    /// key does ([`SelectState::download_modal_lane`]). Every other key is taken and does nothing.
    pub(super) fn download_modal_key(&mut self, shared: &mut AppShared, code: KeyCode) {
        match code {
            KeyCode::Escape => self.download_modal = None,
            KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                if let Some(modal) = self.download_modal.as_mut() {
                    modal.step_focus(code == KeyCode::ArrowRight);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => self.press_focused_download_button(shared),
            code => {
                if let Some(index) = shared.lane_input_for(code).and_then(|(lane, dir)| key_index_of(shared.mode, lane, dir)) {
                    self.download_modal_lane(shared, index);
                }
            }
        }
    }

    /// One controller key going down while the modal is up, named by its index in the mode that is
    /// up: the turntable moves the cursor, a key that opens a folder presses the button under it and
    /// one that closes a folder closes the modal. The keys are those of the table the browser reads
    /// them by.
    pub(super) fn download_modal_lane(&mut self, shared: &mut AppShared, index: usize) {
        let layout = if skin_status(shared) == SkinStatus::Ready { KeyLayout::of_browser(shared) } else { BUILT_IN_KEY_LAYOUT };
        match index {
            SCRATCH_FORWARD_INDEX | SCRATCH_BACKWARD_INDEX => {
                if let Some(modal) = self.download_modal.as_mut() {
                    modal.step_focus(index == SCRATCH_FORWARD_INDEX);
                }
            }
            _ if layout.carries(index, SelectKey::FolderOpen) => self.press_focused_download_button(shared),
            _ if layout.carries(index, SelectKey::FolderClose) => self.download_modal = None,
            _ => {}
        }
    }

    /// A press of the mouse while the modal is up, on what the frame drew under it: a button that is
    /// on is pressed, and anything else is taken and does nothing.
    pub(super) fn download_modal_click(&mut self, shared: &mut AppShared, hit: Option<Hot>) {
        let Some(Hot::DownloadButton(place)) = hit else {
            return;
        };
        let Some(modal) = self.download_modal.as_mut() else {
            return;
        };
        let Some((button, true)) = modal.buttons().get(place).copied() else {
            return;
        };
        modal.focus = button;
        self.press_download_button(shared, button);
    }

    fn press_focused_download_button(&mut self, shared: &mut AppShared) {
        if let Some(button) = self.download_modal.as_ref().and_then(DownloadModal::focused_button) {
            self.press_download_button(shared, button);
        }
    }

    /// Bring what the modal shows of the download job up to date, once a frame and after anything
    /// the modal itself did to the job.
    pub(super) fn follow_the_download(&mut self, shared: &AppShared) {
        if let Some(modal) = self.download_modal.as_mut() {
            modal.job = crate::download::status(shared);
        }
    }

    /// Do what `button` is for.
    fn press_download_button(&mut self, shared: &mut AppShared, button: ModalButton) {
        let Some(modal) = self.download_modal.as_mut() else {
            return;
        };
        match button {
            ModalButton::GetChart => {
                if shared.config.library.download_folder_path().is_some() {
                    begin_download(shared, &modal.entry);
                } else if modal.folder_picker.is_none() {
                    modal.folder_picker = Some((self.download_hands.pick_folder)());
                }
            }
            ModalButton::OpenSongUrl => {
                if let Some(url) = modal.song_url.as_deref() {
                    (self.download_hands.open_url)(url);
                }
            }
            ModalButton::OpenChartUrl => {
                if let Some(url) = modal.chart_url.as_deref() {
                    (self.download_hands.open_url)(url);
                }
            }
            ModalButton::Close => self.download_modal = None,
            ModalButton::Cancel => cancel_download(shared),
        }
        self.follow_the_download(shared);
    }

    /// Take the answer of the download folder picker the modal opened, once a frame: a folder picked
    /// is kept as the download folder, written out, and the chart is got into it. A picker closed
    /// without a folder leaves everything as it was.
    pub(super) fn poll_download_folder(&mut self, shared: &mut AppShared) {
        let Some(modal) = self.download_modal.as_mut() else {
            return;
        };
        let Some(picker) = modal.folder_picker.as_ref() else {
            return;
        };
        match picker.poll() {
            DialogState::Open => {}
            DialogState::Dismissed => modal.folder_picker = None,
            DialogState::Picked(folder) => {
                modal.folder_picker = None;
                shared.config.library.download_folder = Some(folder.to_string_lossy().into_owned());
                shared.save_settings();
                begin_download(shared, &modal.entry);
                modal.job = crate::download::status(shared);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;

    use super::*;
    use crate::text::settings::InstalledLanguage;

    const BUTTONS: [ModalButton; 5] = [ModalButton::GetChart, ModalButton::OpenSongUrl, ModalButton::OpenChartUrl, ModalButton::Close, ModalButton::Cancel];

    #[test]
    fn a_button_says_what_it_does_in_the_installed_language() {
        assert_eq!(BUTTONS.map(ModalButton::label), ["GET CHART", "OPEN SONG URL", "OPEN CHART URL", "CLOSE", "CANCEL"]);
        let _korean = InstalledLanguage::new(Language::Korean);
        assert_eq!(BUTTONS.map(ModalButton::label), ["차트 받기", "곡 URL 열기", "차트 URL 열기", "닫기", "취소"]);
        let _japanese = InstalledLanguage::new(Language::Japanese);
        assert_eq!(BUTTONS.map(ModalButton::label), ["譜面を取得", "曲のURLを開く", "譜面のURLを開く", "閉じる", "キャンセル"]);
    }

    #[test]
    fn the_notice_of_a_chart_the_table_no_longer_lists_is_worded_in_the_language_it_is_taken_in() {
        crate::notify::exclusive(|| {
            notify_text(Level::Info, DownloadModalText::NotInTheLibrary, &[]);
            let _korean = InstalledLanguage::new(Language::Korean);
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert_eq!(said, [(Level::Info, "라이브러리에 없습니다".to_string())]);
        });
    }
}

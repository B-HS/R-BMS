//! The settings screen as data plus one draw call: the tab strip, the (scrolling) row list, the
//! in-place editor, the network status line, and the inline rival list.
//!
//! Extracted from the frame loop so the screen can be rendered headless in tests and so a tab with
//! more rows than fit on screen scrolls instead of drawing off the bottom.

use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_right, fit_text, text_width, theme};

/// Panel geometry, matching the layout the screen has always used.
const PANEL_W: f32 = 720.0;
const CANVAS_W: f32 = 1280.0;
const TITLE_Y: f32 = 36.0;
const HINT_Y: f32 = 78.0;
const TABS_Y: f32 = 104.0;
const TAB_H: f32 = 34.0;
const TAB_GAP: f32 = 8.0;
const TAB_PAD: f32 = 28.0;
const ROWS_TOP: f32 = 160.0;
const ROW_PITCH: f32 = 50.0;
const ROW_H: f32 = 42.0;
const STATUS_Y: f32 = 672.0;

/// The inline rival list, drawn centred over the row list.
const RIVALS_X: f32 = 380.0;
const RIVALS_Y: f32 = 150.0;
const RIVALS_W: f32 = 520.0;
const RIVALS_H: f32 = 420.0;
const RIVALS_ROWS_TOP: f32 = 72.0;
const RIVALS_ROW_PITCH: f32 = 38.0;
const RIVALS_ROW_H: f32 = 32.0;

const TITLE_SCALE: f32 = 3.0;
const HINT_SCALE: f32 = 1.2;
const TAB_SCALE: f32 = 1.6;
const LABEL_SCALE: f32 = 1.8;
const VALUE_SCALE: f32 = 2.0;
const STATUS_SCALE: f32 = 1.2;
const RIVAL_SCALE: f32 = 1.5;

/// How far a row's label and value sit in from the row's own edges, and down from its top.
const ROW_PAD: f32 = 20.0;
const ROW_TEXT_Y: f32 = 13.0;

/// The note beside a row: how large it is written, how far below the row's top, and how much room
/// is left between it and the label before it and the value after it.
const NOTE_SCALE: f32 = 1.2;
const NOTE_Y: f32 = 18.0;
const NOTE_GAP: f32 = 16.0;

/// Widest a row's value is drawn, as a share of the panel.
const VALUE_MAX_SHARE: f32 = 0.6;

/// Keyboard hint under the title.
pub(crate) const SETTINGS_HINT: &str = "TAB SWITCH   UP DOWN MOVE   LEFT RIGHT CHANGE   ENTER OPEN   ESC SAVE/BACK";

/// Keyboard hint inside the inline rival list.
pub(crate) const RIVALS_HINT: &str = "UP DOWN MOVE   ENTER ADD   D REMOVE   ESC SAVE/CLOSE";

/// Title of the inline rival list.
pub(crate) const RIVALS_TITLE: &str = "RIVALS";

/// One row of the settings screen as it is drawn.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct SettingsLine {
    pub(crate) label: String,
    pub(crate) value: String,
    /// What is said beside the row, or `None` for a row shown bare. A note is written small between
    /// the label and the value, a step brighter on the focused row so it reads against the
    /// highlight, and the value of an annotated row is dimmed until the row is focused.
    pub(crate) note: Option<&'static str>,
}

impl SettingsLine {
    /// A row with nothing said beside it.
    #[cfg(test)]
    pub(crate) fn bare(label: &str, value: &str) -> SettingsLine {
        SettingsLine { label: label.to_string(), value: value.to_string(), note: None }
    }
}

/// Everything the settings screen draws this frame, assembled before the render pass.
pub(crate) struct SettingsScene {
    pub(crate) tabs: Vec<&'static str>,
    pub(crate) tab: usize,
    pub(crate) rows: Vec<SettingsLine>,
    pub(crate) sel: usize,
    /// Text being typed into the focused row, already masked when the row is a secret.
    pub(crate) editor: Option<String>,
    /// Last network result or error, shown under the rows.
    pub(crate) status: String,
    /// The inline rival list, when it is open.
    pub(crate) rivals: Option<RivalsScene>,
}

/// The inline rival list's own rows and editor.
pub(crate) struct RivalsScene {
    pub(crate) rows: Vec<String>,
    pub(crate) sel: usize,
    pub(crate) editor: Option<String>,
}

/// A clickable region of the settings screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SettingsHot {
    Tab(usize),
    Row(usize),
    RivalRow(usize),
}

/// How many setting rows fit between the tab strip and the status line.
pub(crate) fn visible_rows() -> usize {
    ((((STATUS_Y - 12.0) - ROWS_TOP) / ROW_PITCH).floor() as usize).max(1)
}

/// Index of the first drawn row so `sel` stays on screen.
pub(crate) fn scroll_start(len: usize, sel: usize) -> usize {
    let visible = visible_rows();
    if len <= visible {
        return 0;
    }
    sel.saturating_sub(visible / 2).min(len - visible)
}

/// How many rival rows fit in the inline list.
pub(crate) fn rivals_visible_rows() -> usize {
    (((RIVALS_H - RIVALS_ROWS_TOP - 8.0) / RIVALS_ROW_PITCH).floor() as usize).max(1)
}

fn rivals_scroll_start(len: usize, sel: usize) -> usize {
    let visible = rivals_visible_rows();
    if len <= visible {
        return 0;
    }
    sel.saturating_sub(visible / 2).min(len - visible)
}

/// Where a row's note starts and how much room it has: what is left of the row between the label
/// and the value as they are drawn, less a gap on either side.
fn note_span(x0: f32, label: &str, value: &str) -> (f32, f32) {
    let left = x0 + ROW_PAD + text_width(label, LABEL_SCALE) + NOTE_GAP;
    let right = x0 + PANEL_W - ROW_PAD - text_width(value, VALUE_SCALE) - NOTE_GAP;
    (left, right - left)
}

/// Draw the screen and return its clickable regions.
pub(crate) fn render_settings<R: Renderer>(r: &mut R, scene: &SettingsScene) -> Vec<(Rect, SettingsHot)> {
    let th = theme();
    let mut hot = Vec::new();
    let x0 = (CANVAS_W - PANEL_W) * 0.5;
    r.clear(th.bg);
    draw_text(r, x0, TITLE_Y, TITLE_SCALE, th.text, "SETTINGS");
    draw_text(r, x0, HINT_Y, HINT_SCALE, th.text_muted, SETTINGS_HINT);

    let mut tx = x0;
    for (index, name) in scene.tabs.iter().enumerate() {
        let on = index == scene.tab;
        let w = text_width(name, TAB_SCALE) + TAB_PAD;
        let rect = Rect::new(tx, TABS_Y, w, TAB_H);
        r.fill_rect(rect, if on { th.button } else { th.panel });
        draw_text(r, tx + TAB_PAD * 0.5, TABS_Y + 8.0, TAB_SCALE, if on { Color::YELLOW } else { th.text_dim }, name);
        hot.push((rect, SettingsHot::Tab(index)));
        tx += w + TAB_GAP;
    }

    let start = scroll_start(scene.rows.len(), scene.sel);
    for (slot, index) in (start..(start + visible_rows()).min(scene.rows.len())).enumerate() {
        let SettingsLine { label, value, note } = &scene.rows[index];
        let y = ROWS_TOP + slot as f32 * ROW_PITCH;
        let on = index == scene.sel;
        let rect = Rect::new(x0, y, PANEL_W, ROW_H);
        r.fill_rect(rect, if on { th.button } else { th.panel });
        draw_text(r, x0 + ROW_PAD, y + ROW_TEXT_Y, LABEL_SCALE, if on { th.text } else { th.text_dim }, label);
        let editing = on && scene.editor.is_some();
        let shown = match (&scene.editor, editing) {
            (Some(buffer), true) => buffer.clone(),
            _ => value.clone(),
        };
        let color = if editing {
            th.accent
        } else if on {
            Color::YELLOW
        } else if note.is_some() {
            th.text_dim
        } else {
            th.text
        };
        let fitted = fit_text(&shown, VALUE_SCALE, PANEL_W * VALUE_MAX_SHARE);
        draw_text_right(r, x0 + PANEL_W - ROW_PAD, y + ROW_TEXT_Y, VALUE_SCALE, color, &fitted);
        if let Some(note) = *note {
            let (left, room) = note_span(x0, label, &fitted);
            if room > 0.0 {
                draw_text(r, left, y + NOTE_Y, NOTE_SCALE, if on { th.text_dim } else { th.text_muted }, &fit_text(note, NOTE_SCALE, room));
            }
        }
        hot.push((rect, SettingsHot::Row(index)));
    }

    if scene.rows.len() > visible_rows() {
        draw_text_right(r, x0 + PANEL_W - 20.0, STATUS_Y - 24.0, STATUS_SCALE, th.text_muted, &format!("{} / {}", scene.sel + 1, scene.rows.len()));
    }
    if !scene.status.is_empty() {
        draw_text(r, x0, STATUS_Y, STATUS_SCALE, th.accent, &fit_text(&scene.status, STATUS_SCALE, PANEL_W));
    }

    if let Some(rivals) = &scene.rivals {
        hot.extend(render_rivals(r, rivals));
    }
    hot
}

fn render_rivals<R: Renderer>(r: &mut R, scene: &RivalsScene) -> Vec<(Rect, SettingsHot)> {
    let th = theme();
    let mut hot = Vec::new();
    r.fill_rect(Rect::new(RIVALS_X, RIVALS_Y, RIVALS_W, RIVALS_H), th.panel_hi);
    r.fill_rect(Rect::new(RIVALS_X, RIVALS_Y, RIVALS_W, 2.0), th.divider);
    draw_text(r, RIVALS_X + 16.0, RIVALS_Y + 14.0, TAB_SCALE, th.text, RIVALS_TITLE);
    draw_text(r, RIVALS_X + 16.0, RIVALS_Y + 44.0, HINT_SCALE, th.text_muted, RIVALS_HINT);

    let start = rivals_scroll_start(scene.rows.len(), scene.sel);
    for (slot, index) in (start..(start + rivals_visible_rows()).min(scene.rows.len())).enumerate() {
        let y = RIVALS_Y + RIVALS_ROWS_TOP + slot as f32 * RIVALS_ROW_PITCH;
        let on = index == scene.sel;
        let rect = Rect::new(RIVALS_X + 8.0, y, RIVALS_W - 16.0, RIVALS_ROW_H);
        r.fill_rect(rect, if on { th.button } else { th.panel });
        let is_add = index + 1 == scene.rows.len();
        let color = if is_add {
            Color::GREEN
        } else if on {
            th.text
        } else {
            th.text_dim
        };
        let shown = match (&scene.editor, on && is_add) {
            (Some(buffer), true) => buffer.clone(),
            _ => scene.rows[index].clone(),
        };
        draw_text(r, RIVALS_X + 20.0, y + 8.0, RIVAL_SCALE, color, &fit_text(&shown, RIVAL_SCALE, RIVALS_W - 48.0));
        hot.push((rect, SettingsHot::RivalRow(index)));
    }
    hot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_panel::{RIVAL_ADD_ROW, network_row_label};
    use rbms_config::{Config, SettingTab, tab_rows};
    use rbms_render::CpuCanvas;

    /// Rows the NETWORK tab shows, taken from the settings table so the scene the tests render is
    /// the one the screen builds.
    fn network_rows() -> Vec<SettingsLine> {
        tab_rows(SettingTab::Network, &Config::default()).into_iter().map(|id| SettingsLine::bare(network_row_label(id).unwrap(), "value")).collect()
    }

    fn network_scene(sel: usize) -> SettingsScene {
        SettingsScene {
            tabs: vec!["PLAY", "GAUGE", "JUDGE", "DISPLAY", "INPUT", "NETWORK"],
            tab: 5,
            rows: network_rows(),
            sel,
            editor: None,
            status: "logged in as dj".to_string(),
            rivals: None,
        }
    }

    #[test]
    fn the_network_tab_has_more_rows_than_fit_so_it_scrolls() {
        let scene = network_scene(0);
        assert!(scene.rows.len() > visible_rows(), "the tab that motivated scrolling still needs it");
        assert_eq!(scroll_start(scene.rows.len(), 0), 0);
        let last = scene.rows.len() - 1;
        assert_eq!(scroll_start(scene.rows.len(), last), scene.rows.len() - visible_rows());
    }

    /// The tab strip grew a ninth tab. It is laid out left to right from the panel's own left edge,
    /// so a strip wider than the canvas would draw the last tab off the screen, where it could
    /// neither be read nor clicked.
    #[test]
    fn the_whole_tab_strip_is_drawn_on_the_screen() {
        rbms_render::font::use_embedded_fonts_only();
        let x0 = (CANVAS_W - PANEL_W) * 0.5;
        let width: f32 = SettingTab::ALL.iter().map(|tab| text_width(tab.label(), TAB_SCALE) + TAB_PAD + TAB_GAP).sum();
        assert!(x0 + width - TAB_GAP <= CANVAS_W, "the tab strip is {width} wide and runs off the {CANVAS_W} wide screen");
    }

    #[test]
    fn a_short_tab_never_scrolls() {
        assert_eq!(scroll_start(3, 2), 0);
        assert_eq!(scroll_start(visible_rows(), visible_rows() - 1), 0);
    }

    #[test]
    fn rendering_reports_one_region_per_tab_and_drawn_row() {
        let scene = network_scene(0);
        let mut canvas = CpuCanvas::new(1280, 720);
        let hot = render_settings(&mut canvas, &scene);
        let tabs = hot.iter().filter(|(_, h)| matches!(h, SettingsHot::Tab(_))).count();
        let rows = hot.iter().filter(|(_, h)| matches!(h, SettingsHot::Row(_))).count();
        assert_eq!(tabs, scene.tabs.len());
        assert_eq!(rows, visible_rows(), "only the rows that fit are clickable");
        assert_ne!(canvas.signature_hash(8, 8), CpuCanvas::new(1280, 720).signature_hash(8, 8));
    }

    #[test]
    fn the_selected_row_is_always_drawn_even_at_the_end_of_a_long_tab() {
        let last = network_rows().len() - 1;
        let scene = network_scene(last);
        let mut canvas = CpuCanvas::new(1280, 720);
        let hot = render_settings(&mut canvas, &scene);
        assert!(hot.iter().any(|(_, h)| *h == SettingsHot::Row(last)));
    }

    #[test]
    fn the_inline_rival_list_renders_over_the_rows_and_is_clickable() {
        let mut scene = network_scene(network_rows().len() - 1);
        scene.rivals = Some(RivalsScene { rows: vec!["friend".into(), RIVAL_ADD_ROW.into()], sel: 1, editor: Some("typing_".into()) });
        let mut canvas = CpuCanvas::new(1280, 720);
        let hot = render_settings(&mut canvas, &scene);
        let rivals: Vec<usize> = hot
            .iter()
            .filter_map(|(_, h)| match h {
                SettingsHot::RivalRow(index) => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(rivals, vec![0, 1]);
    }

    /// The values an annotated row can show, widest included: what a toggle reads, what a cycle
    /// steps through, and the two ends of a percentage.
    fn shown_values(id: rbms_config::SettingId) -> Vec<String> {
        use rbms_config::{CUSTOM_VALUE, OFF_VALUE, ON_VALUE, PERCENT_SCALE, PERCENT_UNIT, SettingKind, descriptor};
        match descriptor(id).kind {
            SettingKind::Toggle => vec![ON_VALUE.to_string(), OFF_VALUE.to_string()],
            SettingKind::Cycle { values } => values.iter().map(|value| (*value).to_string()).chain([CUSTOM_VALUE.to_string()]).collect(),
            SettingKind::Percent { min, max, .. } => [min, max].iter().map(|end| format!("{}{PERCENT_UNIT}", (end * PERCENT_SCALE).round() as i32)).collect(),
            other => panic!("{id:?} is annotated and is a {other:?}, whose values this test does not know how to list"),
        }
    }

    /// A note is only worth writing whole: one cut short by the value beside it says less than no
    /// note at all. Every row that can carry one has room for it next to the widest value it shows.
    #[test]
    fn every_note_fits_between_its_label_and_its_widest_value() {
        rbms_render::font::use_embedded_fonts_only();
        let x0 = (CANVAS_W - PANEL_W) * 0.5;
        let mut annotated = 0;
        for row in rbms_config::SETTINGS {
            let Some(skinned) = row.skinned else {
                continue;
            };
            annotated += 1;
            for value in shown_values(row.id) {
                let (left, room) = note_span(x0, row.label, &value);
                let needed = text_width(skinned.note, NOTE_SCALE);
                assert!(left > x0 && needed <= room, "{:?} showing {value}: the note needs {needed} and has {room}", row.id);
                assert_eq!(fit_text(skinned.note, NOTE_SCALE, room), skinned.note, "{:?} showing {value}: the note is drawn cut short", row.id);
            }
        }
        assert!(annotated > 0, "no row carries a note, so nothing was measured");
    }

    /// Two rows of the DISPLAY tab with the cursor on the second, and `note` beside the first.
    fn noted_scene(note: Option<&'static str>) -> SettingsScene {
        SettingsScene {
            tabs: vec!["PLAY", "DISPLAY"],
            tab: 1,
            rows: vec![SettingsLine { note, ..SettingsLine::bare("JUDGE TEXT Y", "0%") }, SettingsLine::bare("LETTERBOX", "ON")],
            sel: 1,
            editor: None,
            status: String::new(),
            rivals: None,
        }
    }

    /// Bytes one pixel of a drawn frame takes.
    const RGBA_BYTES: usize = 4;

    /// Every pixel of the screen drawn for `scene`.
    fn drawn(scene: &SettingsScene) -> Vec<u8> {
        rbms_render::font::use_embedded_fonts_only();
        let mut canvas = CpuCanvas::new(1280, 720);
        render_settings(&mut canvas, scene);
        canvas.pixels().to_vec()
    }

    /// A note adds to its own row and to nothing else: the rows above and below it, the tab strip
    /// and the title are drawn pixel for pixel as they are on a screen with no note at all.
    #[test]
    fn a_note_is_drawn_inside_the_row_it_belongs_to() {
        let bare = drawn(&noted_scene(None));
        let noted = drawn(&noted_scene(Some(rbms_config::BUILT_IN_ONLY_NOTE)));
        assert!(bare != noted, "a row with a note is drawn the same as one without");

        let row_bytes = CANVAS_W as usize * RGBA_BYTES;
        let (top, bottom) = (ROWS_TOP as usize * row_bytes, (ROWS_TOP + ROW_H) as usize * row_bytes);
        assert!(bare[..top] == noted[..top], "the note changed something above its row");
        assert!(bare[bottom..] == noted[bottom..], "the note changed something below its row");
    }

    #[test]
    fn an_empty_status_and_editor_still_render() {
        let mut scene = network_scene(2);
        scene.status = String::new();
        scene.editor = Some("*****_".to_string());
        let mut canvas = CpuCanvas::new(1280, 720);
        assert!(!render_settings(&mut canvas, &scene).is_empty());
    }
}

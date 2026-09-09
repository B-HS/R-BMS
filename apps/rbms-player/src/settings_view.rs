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

/// Keyboard hint under the title.
pub(crate) const SETTINGS_HINT: &str = "TAB SWITCH   UP DOWN MOVE   LEFT RIGHT CHANGE   ENTER OPEN   ESC SAVE/BACK";

/// Keyboard hint inside the inline rival list.
pub(crate) const RIVALS_HINT: &str = "UP DOWN MOVE   ENTER ADD   D REMOVE   ESC SAVE/CLOSE";

/// Title of the inline rival list.
pub(crate) const RIVALS_TITLE: &str = "RIVALS";

/// Everything the settings screen draws this frame, assembled before the render pass.
pub(crate) struct SettingsScene {
    pub(crate) tabs: Vec<&'static str>,
    pub(crate) tab: usize,
    pub(crate) rows: Vec<(&'static str, String)>,
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
        let (label, value) = &scene.rows[index];
        let y = ROWS_TOP + slot as f32 * ROW_PITCH;
        let on = index == scene.sel;
        let rect = Rect::new(x0, y, PANEL_W, ROW_H);
        r.fill_rect(rect, if on { th.button } else { th.panel });
        draw_text(r, x0 + 20.0, y + 13.0, LABEL_SCALE, if on { th.text } else { th.text_dim }, label);
        let editing = on && scene.editor.is_some();
        let shown = match (&scene.editor, editing) {
            (Some(buffer), true) => buffer.clone(),
            _ => value.clone(),
        };
        let color = if editing {
            th.accent
        } else if on {
            Color::YELLOW
        } else {
            th.text
        };
        draw_text_right(r, x0 + PANEL_W - 20.0, y + 13.0, VALUE_SCALE, color, &fit_text(&shown, VALUE_SCALE, PANEL_W * 0.6));
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
    use crate::ir_panel::{NETWORK_SETTING_ROWS, RIVAL_ADD_ROW, network_row_label};
    use rbms_render::CpuCanvas;

    fn network_scene(sel: usize) -> SettingsScene {
        SettingsScene {
            tabs: vec!["PLAY", "GAUGE", "JUDGE", "DISPLAY", "INPUT", "NETWORK"],
            tab: 5,
            rows: NETWORK_SETTING_ROWS.iter().map(|&i| (network_row_label(i).unwrap(), "value".to_string())).collect(),
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
        let last = NETWORK_SETTING_ROWS.len() - 1;
        let scene = network_scene(last);
        let mut canvas = CpuCanvas::new(1280, 720);
        let hot = render_settings(&mut canvas, &scene);
        assert!(hot.iter().any(|(_, h)| *h == SettingsHot::Row(last)));
    }

    #[test]
    fn the_inline_rival_list_renders_over_the_rows_and_is_clickable() {
        let mut scene = network_scene(NETWORK_SETTING_ROWS.len() - 1);
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

    #[test]
    fn an_empty_status_and_editor_still_render() {
        let mut scene = network_scene(2);
        scene.status = String::new();
        scene.editor = Some("*****_".to_string());
        let mut canvas = CpuCanvas::new(1280, 720);
        assert!(!render_settings(&mut canvas, &scene).is_empty());
    }
}

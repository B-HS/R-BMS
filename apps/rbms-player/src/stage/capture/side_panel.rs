//! The side menu of the default pack's browser: the settings panel and the volume panel that the two
//! tabs on the left edge of the screen open over the info window.
//!
//! The panel is held to the grid of the screen it opens on: it starts on the line the song block, the
//! list and its scroll track start at, ends on the line they end at, and leaves what is written above
//! that line -- the tops of the letters of the first line of the song block -- unseen.
//!
//! Like the other captures of the default pack this saves frames for a person to look at only where
//! [`CAPTURE_DIR_ENV`] is set; the checks run wherever the tests do.

use winit::event::MouseButton;

use super::*;
use crate::pointer::PointerInput;

const LEFT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: true };

/// The two tabs of the side menu, by the name their frames are saved under, with the row of the
/// middle of each counted down from the top of the 1920 by 1080 document, and the column of it.
const SIDE_TABS: [(&str, f32); 2] = [("settings", 492.0), ("volume", 588.0)];
const SIDE_TAB_COLUMN: f32 = 24.0;

/// The moment of the browser's scene the panels are opened at, and how much later each frame of the
/// slide that brings a panel in is drawn than the one before it, in milliseconds. The panel slides
/// for 200 of them and four steps of 300 settle it.
const OPEN_FROM_MS: i64 = 3_000;
const SLIDE_STEP_MS: i64 = 300;
const SLIDE_STEPS: usize = 4;

/// The rows the panel's top rule and its bottom edge are on, in the pixels of the document: the
/// grid line the song block starts at, and the line the info window's columns end at, which is the
/// last row of the panel's body and the one under it the ground.
const PANEL_TOP: u32 = 136;
const PANEL_BODY_LAST: u32 = 967;

/// The columns of the document the panel covers, and the rows above its top rule that nothing may
/// show in: the eight between the line under the head and the grid line.
const PANEL_COLUMNS: std::ops::Range<u32> = 64..1096;
const ABOVE_THE_PANEL: std::ops::Range<u32> = 128..PANEL_TOP;

/// The colours of the panel as the default theme draws it: the top rule in the accent the pack comes
/// with, the body in the raised face, and the ground of the screen.
const ACCENT: crate::Color = crate::Color::rgb(255, 255, 255);
const RAISED: crate::Color = crate::Color::rgb(38, 38, 38);
const GROUND: crate::Color = crate::Color::rgb(0, 0, 0);

/// One whole frame of the browser at `at_ms`: its update, its draw, and the frame's end, which is
/// what hands a skin's writes on to the browser.
fn frame_at(app: &mut App, at_ms: i64, pixels: &mut HeadlessCanvas, deadline: Instant) -> Result<(), String> {
    browser_frame_at(app, at_ms, pixels, deadline)?;
    app.shared.finish_skin_frame(&mut Canvas::Headless(pixels));
    Ok(())
}

/// The cursor, as the window reports it, over the point of the document `row` rows down and
/// `column` columns across, whatever size the document is drawn at.
fn cursor_over(column: f32, row: f32, size: (u32, u32)) -> (f32, f32) {
    (column / size.0 as f32 * UI_SIZE.0 as f32, row / size.1 as f32 * UI_SIZE.1 as f32)
}

/// Each panel of the side menu, opened by a press on its tab and closed by another, starts on the
/// grid line of the song block with its rule, has the raised face down to the line the columns of
/// the info window end at, and has the ground above it and under it.
#[test]
fn the_default_packs_side_panels_start_and_end_on_the_lines_of_the_browsers_grid_with_the_ground_above_them() {
    let mut app = with_browse_library(first_run_app("side-panel"), "side-panel").unwrap_or_else(|reason| panic!("side panel: {reason}"));
    let size = authored_size(&app, SKIN_TYPE_MUSIC_SELECT).unwrap_or_else(|| panic!("side panel: {NO_DOCUMENT}"));
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    app.shared.open_for_tests([SelectFolder::AllSongs]);
    app.shared.rebuild_select_items();
    capture_browser_view(&mut app, &mut pixels, size, "side-base", SelectState::new()).unwrap_or_else(|reason| panic!("side panel: {reason}"));

    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    let mut at_ms = OPEN_FROM_MS;
    let mut settle = |app: &mut App, pixels: &mut HeadlessCanvas| {
        for _ in 0..SLIDE_STEPS {
            frame_at(app, at_ms, pixels, deadline).unwrap_or_else(|reason| panic!("side panel: {reason}"));
            at_ms += SLIDE_STEP_MS;
        }
        frame_at(app, at_ms, pixels, deadline).unwrap_or_else(|reason| panic!("side panel: {reason}"));
    };
    for (name, row) in SIDE_TABS {
        let tab = cursor_over(SIDE_TAB_COLUMN, row, size);
        assert!(app.shared.skin_pointer(tab, LEFT_PRESS), "{name}: the tab took no press");
        settle(&mut app, &mut pixels);
        save(&format!("side-{name}"), CAPTURE_EXTENSION, size, pixels.rgba());

        let middle = PANEL_COLUMNS.start + (PANEL_COLUMNS.end - PANEL_COLUMNS.start) / 2;
        assert_eq!(
            pixels.pixel_at(middle, PANEL_TOP),
            ACCENT,
            "{name}: the rule of the panel is not on row {PANEL_TOP}, the grid line the song block starts at"
        );
        assert_eq!(pixels.pixel_at(middle, PANEL_TOP + 2), RAISED, "{name}: the face of the panel does not start under its rule");
        assert_eq!(
            pixels.pixel_at(middle, PANEL_BODY_LAST),
            RAISED,
            "{name}: the face of the panel does not reach row {PANEL_BODY_LAST}, where the info window's columns end"
        );
        assert_eq!(pixels.pixel_at(middle, PANEL_BODY_LAST + 1), GROUND, "{name}: the panel runs under the line the info window's columns end at");
        assert!(
            is_flat(&pixels, PANEL_COLUMNS, ABOVE_THE_PANEL) && pixels.pixel_at(PANEL_COLUMNS.start, ABOVE_THE_PANEL.start) == GROUND,
            "{name}: something is drawn over the panel's top rule, between the line under the head and the grid line"
        );

        assert!(app.shared.skin_pointer(tab, LEFT_PRESS), "{name}: the tab took no press to close the panel");
        settle(&mut app, &mut pixels);
    }
}

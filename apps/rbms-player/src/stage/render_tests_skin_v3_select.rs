//! The shipped bundle's browser document, held to compiling clean.
//!
//! The bundle is the one skin whose faults are this project's own: a missing source, an object type
//! this build does not draw, an expression that will not compile, all of it reaches the player as a
//! warning toast and none of it should ever be shipped. The browser document is checked here and the
//! remaining screens in the files beside this one, so a failure names the screen that broke.
//!
//! Past compiling, the document has to actually stand in for what it claims: the wheel it draws must
//! answer a click on its rows, the blocks it names in `replace` must all have their objects, and the
//! option panel it owns must reach the screen when the panel is open. Those are checked against the
//! frame the browser really draws rather than against the draw list, because a document that
//! compiles and draws nothing is the failure this bundle has had before.

use rbms_library::Library;
use rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT;

use crate::stage::render_tests::render_into;
use crate::stage::render_tests_skin::{assert_bundled_document_compiles_clean, bundled_app, render_until_screen_compiled};
use crate::stage::select::tests::{entry, press, record};
use crate::stage::{Canvas, FrameCtx, HeadlessCanvas, KeyInput, SelectState, Stage, StageId};
use crate::{App, CH, CW, Color, Hot, SelectView, app_options};

/// The wheel slot the document centres its focused chart on, from `select.json5`.
const CENTER_SLOT: u32 = 7;

/// Where the wheel's rows start and how tall each one is, measured down from the head of the screen
/// as the document writes them.
const ROW_TOP: u32 = 60;
const ROW_PITCH: u32 = 40;

/// The left edge of the focused row, which is the furthest left any row of the wheel reaches.
const ROW_LEFT: f32 = 680.0;

/// A pixel inside the option panel the document draws, well clear of its edges.
const PANEL_PROBE: (u32, u32) = (300, 300);

/// The body of the score panel, below its heading and above the row that counts plays, where the
/// focused chart's own record is written, as `(x, y, w, h)` measured down from the head of the
/// screen.
///
/// The play count is left out on purpose: it is answered for every chart, recorded or not, so a
/// rectangle covering it could never tell a chart with nothing on it from one with a record.
const SCORE_PANEL_BODY: (u32, u32, u32, u32) = (40, 376, 300, 76);

/// The five rows the score panel lists the focused chart's record on, in the order the document
/// places them: best EX, break count, lamp, when it was set, and how often it was played.
const SCORE_PANEL_ROWS: [(u32, u32, u32, u32); 5] = [(40, 378, 288, 16), (40, 397, 288, 16), (40, 416, 288, 16), (40, 435, 288, 16), (40, 454, 288, 16)];

/// The statistic cells of the chart panel, two columns of three, each wide enough for the whole
/// `label value` line the state hands it.
const STAT_CELLS: [(u32, u32, u32, u32); 6] =
    [(256, 242, 200, 16), (256, 266, 200, 16), (256, 290, 200, 16), (460, 242, 200, 16), (460, 266, 200, 16), (460, 290, 200, 16)];

/// The body of the record panel beside it, which carries how often the chart was played and when it
/// last was.
const RECORD_PANEL_BODY: (u32, u32, u32, u32) = (396, 386, 252, 54);

/// A chart whose md5 the fixture library gives the first row, for putting a record on it.
const FIRST_ROW_MD5: &str = "md5-First light";

/// The clear, score, break count and time the fixture record carries.
const FIXTURE_RECORD: (u8, u32, u32, i64) = (5, 900, 4, 1_000);

/// How bright a channel has to be before a probe counts the pixel as drawn text or a lit panel.
const BRIGHT: u8 = 180;

/// How bright a pixel's strongest channel has to be before a probe counts it as ink rather than the
/// panel behind it, for the lines this document writes in a colour of its own rather than in white.
const LIT_CHANNEL: u8 = 140;

#[test]
fn the_bundles_browser_document_compiles_without_a_warning() {
    let (mut app, _settings) = bundled_app("v3-select");
    assert_bundled_document_compiles_clean(&mut app, SKIN_TYPE_MUSIC_SELECT, || Stage::Select(Box::new(SelectState::new())));
}

/// An app with the bundle installed, a handful of charts to browse, and its browser document
/// compiled, with the last frame it drew.
fn browsing(tag: &str) -> (App, HeadlessCanvas) {
    let (mut app, _settings) = bundled_app(tag);
    app.shared.library = Library::from_songs(vec![
        entry("First light", "Artist A", "11"),
        entry("Blue orbit", "Artist B", "4"),
        entry("Morning star", "Artist C", "9"),
        entry("Passing clouds", "Artist D", "12"),
        entry("Night arrival", "Artist E", "7"),
    ]);
    app.shared.select_view = SelectView::AllSongs;
    app.shared.rebuild_select_items();

    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(
        render_until_screen_compiled(&mut app, SKIN_TYPE_MUSIC_SELECT, &mut pixels, || Stage::Select(Box::new(SelectState::new()))),
        "the browser document never finished compiling"
    );
    (app, pixels)
}

/// Whether any pixel of `rect`, written as `(x, y, w, h)` measured down from the head of the screen,
/// is bright enough to be ink the document put there.
///
/// A probe rather than an exact colour because the text the document draws is anti-aliased against
/// whatever panel is behind it: what is asserted is that something light reached the rectangle, which
/// is the difference between a line of text and an empty panel.
fn has_bright_pixel(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
    let (x, y, w, h) = rect;
    (y..y + h).any(|py| {
        (x..x + w).any(|px| {
            let pixel = pixels.pixel_at(px, py);
            pixel.r >= BRIGHT && pixel.g >= BRIGHT && pixel.b >= BRIGHT
        })
    })
}

/// Whether any pixel of `rect` is ink in any colour, which is how a line the document writes in cyan
/// or steel is told from the dark panel under it.
fn has_lit_pixel(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
    let (x, y, w, h) = rect;
    (y..y + h).any(|py| {
        (x..x + w).any(|px| {
            let pixel = pixels.pixel_at(px, py);
            pixel.r.max(pixel.g).max(pixel.b) >= LIT_CHANNEL
        })
    })
}

/// A fingerprint of one rectangle, for telling a box that changed from a box that did not.
///
/// Used where the panel already carries a line before the change, so "something is lit" cannot tell
/// the two frames apart and only the pixels themselves can.
fn region_checksum(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> u64 {
    let (x, y, w, h) = rect;
    (y..y + h).fold(0_u64, |outer, py| {
        (x..x + w).fold(outer, |sum, px| {
            let pixel = pixels.pixel_at(px, py);
            sum.wrapping_mul(31).wrapping_add(u64::from(pixel.r) + u64::from(pixel.g) * 3 + u64::from(pixel.b) * 7)
        })
    })
}

/// The centre of the wheel slot `slot` lands on, in canvas pixels.
fn slot_centre(slot: u32) -> (u32, u32) {
    (900, ROW_TOP + ROW_PITCH * slot + ROW_PITCH / 2)
}

/// Saves the frame for the eye to check, when a capture directory was asked for.
fn save(name: &str, pixels: &HeadlessCanvas) {
    let Some(directory) = std::env::var_os("RBMS_SKIN_CAPTURE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).expect("create the capture folder");
    let image = image::RgbaImage::from_fn(CW, CH, |x, y| {
        let pixel = pixels.pixel_at(x, y);
        image::Rgba([pixel.r, pixel.g, pixel.b, pixel.a])
    });
    image.save(directory.join(format!("{name}.png"))).expect("save the capture");
}

/// Every block the document lists in `replace` has the objects that block demands, so the browser
/// really does stand aside for all four rather than drawing half its own screen under the document.
#[test]
fn the_browser_document_stands_in_for_every_block_it_names() {
    let (app, _pixels) = browsing("v3-select-blocks");
    let content = app.shared.screen_content(SKIN_TYPE_MUSIC_SELECT).select;
    assert!(content.list, "the document did not take the row list over");
    assert!(content.detail, "the document did not take the detail pane over");
    assert!(content.topbar, "the document did not take the top bar over");
    assert!(content.options, "the document did not take the option panel over");
}

/// The wheel is drawn and clickable: the focused slot carries a bar the empty slots above it do not,
/// and every row rectangle the browser hit-tests comes from the wheel rather than from the built-in
/// list it replaced.
#[test]
fn the_browser_document_draws_its_wheel_and_answers_a_click_on_its_rows() {
    let (app, pixels) = browsing("v3-select-wheel");
    save("v3-select", &pixels);

    let (centre_x, centre_y) = slot_centre(CENTER_SLOT);
    let (empty_x, empty_y) = slot_centre(0);
    assert_ne!(
        pixels.pixel_at(centre_x, centre_y),
        pixels.pixel_at(empty_x, empty_y),
        "the focused wheel slot drew nothing the slots past the end of the list did not"
    );

    let rows: Vec<f32> = app.shared.hot.iter().filter(|(_, hot)| matches!(hot, Hot::SelectRow(_))).map(|(rect, _)| rect.x).collect();
    assert_eq!(rows.len(), 5, "the wheel offered a click on {} rows for a library of five", rows.len());
    assert!(rows.iter().all(|x| *x >= ROW_LEFT), "a row rectangle sits left of the wheel, so the built-in list is still placing them");
}

/// The detail pane the document draws carries the chart's own words and its bottom bar carries the
/// buttons that lead somewhere, which is what the browser gave up when it stood aside.
#[test]
fn the_browser_document_draws_the_detail_pane_and_the_bottom_bar() {
    let (app, pixels) = browsing("v3-select-detail");

    assert!(has_bright_pixel(&pixels, (40, 80, 420, 44)), "the focused chart's title is missing from the detail pane");
    assert!(has_bright_pixel(&pixels, (24, 676, 640, 28)), "the bottom bar drew none of the buttons the document declares");

    for action in [Hot::NavSearch, Hot::NavSort, Hot::NavFolders, Hot::NavTables, Hot::NavRecords, Hot::NavSettings] {
        assert!(app.shared.hot.iter().any(|(_, hot)| *hot == action), "the document's hotspot table did not answer {action:?}");
    }
}

/// Opening the option panel reaches the screen through the document, not through the built-in
/// overlay: the frame the browser draws changes inside the panel the document places, and it changes
/// on a plain redraw of the browser rather than on an overlay pass.
#[test]
fn the_browser_document_draws_the_option_panel_it_replaced() {
    let (mut app, mut pixels) = browsing("v3-select-options");
    let closed = pixels.pixel_at(PANEL_PROBE.0, PANEL_PROBE.1);

    let key: KeyInput<'_> = press(crate::KeyCode::F1);
    let now = std::time::Instant::now();
    let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
    assert!(app_options::options_key(&mut ctx, StageId::Select, false, &key), "F1 did not open the option panel");

    render_into(&mut app, Stage::Select(Box::new(SelectState::new())), &mut pixels);
    let opened = pixels.pixel_at(PANEL_PROBE.0, PANEL_PROBE.1);
    assert_ne!(opened, closed, "the option panel the document declares never reached the screen");
    assert!(has_bright_pixel(&pixels, (44, 252, 300, 20)), "the panel's first row drew no label");
    save("v3-options", &pixels);

    let drawn = pixels.pixel_checksum();
    let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
    {
        let mut target = Canvas::Headless(&mut pixels);
        App::draw_overlays(&app.stage, &mut ctx, &mut target);
    }
    assert_eq!(pixels.pixel_checksum(), drawn, "the built-in option overlay drew over the panel the document had already replaced");
}

/// The two record panels the document took the detail pane over with really carry the chart's
/// record, rather than standing as headings over empty boxes.
///
/// The browser drew clear counts, a best score, a rank bar and the recent plays before it stood
/// aside; a document that claims the pane and then names none of those ids loses all of it silently,
/// which is why what is checked here is the frame rather than the object table.
#[test]
fn the_browser_documents_record_panels_carry_the_focused_charts_record() {
    let (mut app, mut pixels) = browsing("v3-select-records");
    assert!(!has_bright_pixel(&pixels, SCORE_PANEL_BODY), "a chart with nothing recorded on it drew a score line");
    let before = region_checksum(&pixels, RECORD_PANEL_BODY);

    let (clear, ex, breaks, played_at) = FIXTURE_RECORD;
    app.shared.scores.push(record(FIRST_ROW_MD5, clear, ex, breaks, played_at));
    app.shared.scores.rebuild_index();
    render_into(&mut app, Stage::Select(Box::new(SelectState::new())), &mut pixels);
    save("v3-select-records", &pixels);

    assert!(has_bright_pixel(&pixels, SCORE_PANEL_BODY), "the score panel drew none of the chart's best score");
    assert_ne!(region_checksum(&pixels, RECORD_PANEL_BODY), before, "the record panel did not answer the play that was recorded");
}

/// The score panel lists the focused chart's record over five rows rather than the three it once
/// had, and each of them is a row of its own rather than a line that ran into its neighbour.
///
/// The play count answers for every chart, so it is the row that proves the panel is placed at all;
/// the other four only fill once something is recorded, which is what separates a panel that reads
/// its state from a panel drawing headings over nothing.
#[test]
fn the_score_panel_lists_the_focused_charts_record_over_five_rows() {
    let (mut app, mut pixels) = browsing("v3-select-score-rows");
    assert!(has_lit_pixel(&pixels, SCORE_PANEL_ROWS[4]), "the play count row drew nothing for a chart that has never been played");
    for row in &SCORE_PANEL_ROWS[..4] {
        assert!(!has_lit_pixel(&pixels, *row), "the row at {row:?} drew a record for a chart that has none");
    }

    let (clear, ex, breaks, played_at) = FIXTURE_RECORD;
    app.shared.scores.push(record(FIRST_ROW_MD5, clear, ex, breaks, played_at));
    app.shared.scores.rebuild_index();
    render_into(&mut app, Stage::Select(Box::new(SelectState::new())), &mut pixels);

    for row in SCORE_PANEL_ROWS {
        assert!(has_lit_pixel(&pixels, row), "the row at {row:?} drew nothing once the chart carried a record");
    }
}

/// Each statistic cell of the chart panel has a box of its own, wide enough for the line the state
/// writes into it, and all six are inside the panel the frame leaves clear.
#[test]
fn the_chart_panels_statistic_cells_each_carry_a_line() {
    let (_app, pixels) = browsing("v3-select-stats");
    for cell in STAT_CELLS {
        assert!(has_bright_pixel(&pixels, cell), "the statistic cell at {cell:?} drew nothing");
    }
}

/// The theme the bundle ships is the browser's fallback layout, so it has to parse and it has to
/// name the same rectangles the document draws in: a theme that failed to parse falls back to the
/// built-in one silently, which would leave the two layouts disagreeing with nothing to say so.
#[test]
fn the_bundles_theme_places_the_native_browser_where_the_document_does() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/skins/steel-neon-v3/theme.ron"))
        .expect("the bundle ships a theme beside its documents");
    let theme = rbms_render::ThemeConfig::parse(&source);
    let select = theme.select_layout.expect("the theme states a browser layout");
    assert_eq!(select.list_rect, Some((700.0, 60.0, 556.0, 600.0)), "the list column moved away from the wheel the document draws");
    assert_eq!(select.cover_rect, Some((470.0, 56.0, 190.0, 142.0)), "the cover square moved away from the document's own bga object");
    assert_eq!(select.row_height, Some(ROW_PITCH as f32), "a native row is no longer the height of a wheel slot");

    let options = theme.options_layout.expect("the theme states an option panel layout");
    assert_eq!(options.rect, Some((24.0, 200.0, 656.0, 460.0)), "the native option panel moved away from the one the document draws");
}

/// The frame the document draws leaves no black behind: the bundle's own backdrop reaches the corners
/// rather than the cleared canvas the layered path starts from.
#[test]
fn the_browser_documents_backdrop_covers_the_screen() {
    let (_app, pixels) = browsing("v3-select-backdrop");
    for (x, y) in [(2, 2), (CW - 3, 2), (2, CH - 3), (CW - 3, CH - 3)] {
        assert_ne!(pixels.pixel_at(x, y), Color::BLACK, "the corner at {x},{y} was left on the cleared canvas");
    }
}

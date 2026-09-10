//! Headless render snapshots of the screens around a run — loading and the two library managers —
//! and of the message strip that is drawn over whichever of them is up.

use std::time::Instant;

use rbms_render::ToastLevel;

use crate::App;
use crate::stage::render_tests::{FRAME_DT, app, render};
use crate::stage::{Canvas, FoldersState, FrameCtx, HeadlessCanvas, LoadingState, Stage, TablesState};
use crate::toast::ToastQueue;

/// Draw the message strip over a frame that has already been painted, the way the frame loop draws
/// it over whichever screen is up.
fn overlay_toasts(app: &App, pixels: &mut HeadlessCanvas) {
    let mut canvas = Canvas::Headless(pixels);
    crate::toast::draw(&app.shared.toasts, &mut canvas);
}

/// One frame the way the app draws it: the screen, and then the overlay pass that settles the
/// window fit and paints what sits over every screen.
fn render_over(app: &mut App, stage: Stage) -> HeadlessCanvas {
    let mut pixels = render(app, stage);
    let mut canvas = Canvas::Headless(&mut pixels);
    let mut ctx = FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: FRAME_DT };
    App::draw_overlays(&app.stage, &mut ctx, &mut canvas);
    pixels
}

#[test]
fn the_loading_screen_paints_its_bar() {
    let mut app = app();
    let pixels = render(&mut app, Stage::Loading(LoadingState::song(0)));
    assert!(pixels.painted_pixels() > 0, "the loading screen drew nothing");
    assert!(app.shared.hot.is_empty(), "there is nothing to click while a chart loads");
}

/// The loading screen animates while a scan runs, so two frames a step apart have to differ — a bar
/// that did not move would read as a frozen app.
#[test]
fn the_loading_screen_animates_between_frames() {
    let mut app = app();
    let first = render(&mut app, Stage::Loading(LoadingState::song(0))).pixel_checksum();
    app.shared.frame_count += 24;
    let later = render(&mut app, Stage::Loading(LoadingState::song(0))).pixel_checksum();
    assert_ne!(first, later, "the loading screen looks frozen");
}

/// A scan and a table fetch are different waits and say so, or a user watching an http request
/// would think the library was being walked again.
#[test]
fn each_kind_of_wait_paints_a_frame_of_its_own() {
    let mut app = app();
    let scan_db = std::env::temp_dir().join(format!("rbms-shell-scan-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&scan_db);
    let scan = render(&mut app, Stage::Loading(LoadingState::scan(Vec::new(), Vec::new(), scan_db.clone(), false))).signature();
    let _ = std::fs::remove_file(&scan_db);
    let source = rbms_config::TableSource { name: "table".into(), location: "/no/such/table.json".into() };
    let fetching = LoadingState::table(&app.shared, source);
    let table = render(&mut app, Stage::Loading(fetching)).signature();
    assert_ne!(scan, table, "a scan and a table fetch look the same");
}

#[test]
fn the_two_library_managers_paint_their_own_lists() {
    let mut app = app();
    let folders = render(&mut app, Stage::Folders(FoldersState::new()));
    assert!(folders.painted_pixels() > 0, "the folder manager drew nothing");
    let tables = render(&mut app, Stage::Tables(TablesState::new()));
    assert!(tables.painted_pixels() > 0, "the table manager drew nothing");
    assert_ne!(folders.signature(), tables.signature(), "the two managers paint the same frame");
}

/// The URL entry is the one place in the app with a caret in it, and a caret that is not drawn is a
/// caret the user cannot aim.
#[test]
fn the_url_entry_draws_what_is_being_typed_and_where_the_caret_is() {
    let mut app = app();
    let mut typing = TablesState::new();
    typing.begin_url_entry_for_tests("https://example.test/table.json");
    let entry = render(&mut app, Stage::Tables(typing));

    let mut moved = TablesState::new();
    moved.begin_url_entry_for_tests("https://example.test/table.json");
    moved.move_caret_home_for_tests();
    let caret_moved = render(&mut app, Stage::Tables(moved));

    assert!(entry.painted_pixels() > 0, "the entry box drew nothing");
    assert_ne!(entry.pixel_checksum(), caret_moved.pixel_checksum(), "the caret is drawn in the same place wherever it is");
}

/// An overlay with nothing to say leaves the screen under it exactly as it was, which is what lets
/// every other screen's snapshot stay still while the notification bus runs underneath.
#[test]
fn an_empty_message_queue_leaves_the_screen_under_it_alone() {
    crate::notify::exclusive(|| {
        let mut app = app();
        let bare = render(&mut app, Stage::Folders(FoldersState::new())).pixel_checksum();
        let mut pixels = render(&mut app, Stage::Folders(FoldersState::new()));
        overlay_toasts(&app, &mut pixels);
        assert_eq!(bare, pixels.pixel_checksum(), "an empty queue painted over the screen");
    });
}

/// The point of the bus: a failure reported from anywhere — a worker thread included — is on the
/// screen the next frame, rather than only in a terminal nobody is watching.
#[test]
fn a_reported_failure_is_painted_over_whichever_screen_is_up() {
    crate::notify::exclusive(|| {
        let mut app = app();
        let bare = render(&mut app, Stage::Folders(FoldersState::new())).pixel_checksum();

        crate::notify::notify(crate::notify::Level::Error, "chart not found: /songs/missing.bms");
        app.shared.toasts.pump(Instant::now());
        assert_eq!(app.shared.toasts.active().len(), 1, "the frame loop takes what was reported");

        let mut pixels = render(&mut app, Stage::Folders(FoldersState::new()));
        overlay_toasts(&app, &mut pixels);
        assert_ne!(bare, pixels.pixel_checksum(), "the message was not drawn");
    });
}

/// The strip is drawn over every screen, not just the ones that report, so it has to paint the same
/// way on a screen it knows nothing about.
#[test]
fn the_message_strip_paints_the_same_over_any_screen() {
    crate::notify::exclusive(|| {
        let mut app = app();
        let mut queue = ToastQueue::default();
        queue.push(ToastLevel::Warn, "audio: reopen failed", Instant::now());
        app.shared.toasts = queue;

        let mut over_folders = render(&mut app, Stage::Folders(FoldersState::new()));
        let bare_folders = over_folders.pixel_checksum();
        overlay_toasts(&app, &mut over_folders);

        let mut over_tables = render(&mut app, Stage::Tables(TablesState::new()));
        let bare_tables = over_tables.pixel_checksum();
        overlay_toasts(&app, &mut over_tables);

        assert_ne!(bare_folders, over_folders.pixel_checksum());
        assert_ne!(bare_tables, over_tables.pixel_checksum());
    });
}

/// The window fit is pointed at the DISPLAY setting on every frame's overlay pass, so the row takes
/// without waiting for the next load. There is no surface behind a headless canvas to fit, so what
/// is checked here is that asking for one changes nothing about what the screen draws.
#[test]
fn asking_for_the_letterbox_does_not_change_what_a_screen_draws() {
    let mut app = app();
    app.shared.config.display.letterbox = false;
    let stretched = render_over(&mut app, Stage::Loading(LoadingState::song(0))).pixel_checksum();
    app.shared.config.display.letterbox = true;
    let fitted = render_over(&mut app, Stage::Loading(LoadingState::song(0))).pixel_checksum();
    assert_eq!(stretched, fitted, "the fit is the surface's, not the screen's");
}

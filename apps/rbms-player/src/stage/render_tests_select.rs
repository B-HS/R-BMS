//! Headless render snapshots of the song browser.

use std::time::Instant;

use rbms_library::Library;

use crate::stage::render_tests::{app, render};
use crate::stage::select::tests::{entry, press, record};
use crate::stage::{FrameCtx, SelectState, Stage, StageHandler};
use crate::{KeyCode, SelectView};

/// The browser is the one screen that is entered without anything having been loaded, so an empty
/// library has to paint a usable frame rather than nothing at all.
#[test]
fn the_browser_paints_a_frame_with_an_empty_library() {
    let mut app = app();
    assert!(app.shared.library.songs().is_empty(), "the fixture has no songs");
    let pixels = render(&mut app, Stage::Select(Box::new(SelectState::new())));
    assert!(pixels.painted_pixels() > 0, "the browser drew nothing");
    assert!(!app.shared.hot.is_empty(), "its navigation buttons stay clickable with no songs");
}

/// The browser is the only screen whose rows are clicked, so its clickable regions have to be
/// rebuilt every frame rather than accumulated.
#[test]
fn the_browser_rebuilds_its_clickable_regions_each_frame() {
    let mut app = app();
    render(&mut app, Stage::Select(Box::new(SelectState::new())));
    let first = app.shared.hot.len();
    render(&mut app, Stage::Select(Box::new(SelectState::new())));
    assert_eq!(app.shared.hot.len(), first, "a second frame doubled the clickable regions");
}

/// Stepping the sort order has to change what is painted, or the sort key would move the list
/// without the header saying so.
#[test]
fn the_sort_order_reaches_the_painted_frame() {
    let mut app = app();
    let before = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();
    app.shared.config.library.sort = app.shared.config.library.sort.next();
    let after = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();
    assert_ne!(before, after, "the browser paints the same frame under two different sort orders");
}

/// A filter with its panel closed shows only as a line in the top bar, so that line has to reach
/// the frame — a filtered library that looks unfiltered looks like a lost library.
#[test]
fn a_filter_reaches_the_painted_frame_with_its_panel_closed() {
    let mut app = app();
    let before = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();
    app.shared.config.library.favorite_only = true;
    let after = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();
    assert_ne!(before, after, "the browser paints the same frame filtered and unfiltered");
}

/// The filter panel is drawn over the row list, so opening it has to change what is painted.
#[test]
fn the_filter_panel_paints_over_the_browser() {
    let mut app = app();
    let closed = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();
    let mut state = SelectState::new();
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, press(KeyCode::F2));
    let open = render(&mut app, Stage::Select(Box::new(state))).pixel_checksum();
    assert_ne!(closed, open, "opening the filter panel painted nothing");
}

/// A starred chart and a ranked one carry marks the row draws itself, so each has to reach the
/// frame rather than only the scene behind it.
#[test]
fn a_starred_chart_and_its_dj_rank_reach_the_painted_frame() {
    let mut app = app();
    app.shared.library = Library::from_songs(vec![entry("song", "artist", "12")]);
    app.shared.select_view = SelectView::AllSongs;
    app.shared.rebuild_select_items();
    let plain = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();

    app.shared.favorites.toggle("md5-song");
    let starred = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();
    assert_ne!(plain, starred, "starring a chart did not mark its row");

    app.shared.scores.push(record("md5-song", 5, 900, 4, 1_000));
    app.shared.scores.rebuild_index();
    let ranked = render(&mut app, Stage::Select(Box::new(SelectState::new()))).pixel_checksum();
    assert_ne!(starred, ranked, "the chart's DJ rank did not reach its row");
}

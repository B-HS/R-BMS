//! A level of a difficulty table lists the charts the library does not hold after the ones it does,
//! dimmed (`SongBar` with `existsSong` false, `Config.showNoSongExistingBar`): kept or taken out by
//! the filters as a chart with nothing recorded, no level and no mode would be, put among the held
//! charts by the orderings the way the reference puts them, never counted in a folder's number,
//! and found again by its md5 once the library holds it.
//!
//! The table is put in the browser by hand, matched against the library as a scan matches it;
//! nothing asks the network.

use rbms_config::{SortMode, TableSource};
use rbms_library::{Library, SongEntry};
use rbms_render::skin_render::frame::BarKind;
use rbms_skin::property::generated::{STRING_ARTIST, STRING_FULLTITLE, STRING_TITLE};
use rbms_table::{DifficultyTable, Md5Index, TableEntry};
use winit::keyboard::KeyCode;

use super::list::{ChartFacts, ClearFilter, missing_chart_meta, missing_chart_title};
use super::scene::wheel_bar;
use super::skinned_tests::Browser;
use super::tests::entry;
use super::*;
use crate::app_result::following_song;
use crate::select_tree::{ListPlace, show_the_list_again};
use crate::skin_host::ClusterState;
use crate::skin_host::chart::ChartState;
use crate::tablesrc::MatchedTable;

/// The charts the library holds, in title order.
const TITLES: [&str; 3] = ["alpha", "amber", "delta"];

/// What the table calls itself.
const TABLE: &str = "TABLE";

/// The chart of the table the tests turn into one the library holds, and who it is by.
const BETA_MD5: &str = "miss-beta";
const BETA_ARTIST: &str = "Beta Artist";

/// The chart of the table that has no title, which is called by its md5.
const UNTITLED_MD5: &str = "miss-untitled";

/// The rows of the open level that the library does not hold, under the default order: the table's
/// own order.
const MISSING_IN_TABLE_ORDER: [&str; 4] = ["missing Beta Missing", "missing Aardvark Missing", "missing miss-untitled", "missing Amber"];

/// One entry of the table.
fn chart(md5: &str, level: &str, title: &str, artist: &str) -> TableEntry {
    TableEntry {
        md5: md5.to_owned(),
        sha256: String::new(),
        level: level.to_owned(),
        title: title.to_owned(),
        artist: artist.to_owned(),
        url: String::new(),
        url_diff: String::new(),
    }
}

/// Level 1 of the table lists two charts the library holds and four it does not -- one of them named
/// twice, one with no title and one titled as a held chart is -- and level 2 one it holds.
fn entries() -> Vec<TableEntry> {
    vec![
        chart("md5-alpha", "1", "alpha", "a"),
        chart(BETA_MD5, "1", "Beta Missing", BETA_ARTIST),
        chart("md5-amber", "1", "amber", "a"),
        chart("miss-aardvark", "1", "Aardvark Missing", "b"),
        chart(BETA_MD5, "1", "Beta Missing", BETA_ARTIST),
        chart(UNTITLED_MD5, "1", "", ""),
        chart("miss-amber", "1", "Amber", "c"),
        chart("md5-delta", "2", "delta", "a"),
    ]
}

/// Match the table against the library the browser holds now, as a scan does.
fn rematch(shared: &mut AppShared) {
    let table = DifficultyTable::from_parts(None, entries());
    let index = Md5Index::new(shared.library.md5s());
    shared.set_tables(vec![MatchedTable::of(TABLE.to_owned(), &table, &index)]);
}

/// A browser no skin draws, holding the table, open on its first level in the table's own order.
fn browser_in_the_level(tag: &str) -> Browser {
    let mut browser = Browser::over(tag, TITLES.map(|title| entry(title, "a", "5")).to_vec(), false);
    let shared = &mut browser.app.shared;
    shared.config.library.tables = vec![TableSource { name: TABLE.to_owned(), location: format!("https://tables.test/missing/{tag}/"), group: String::new() }];
    rematch(shared);
    shared.config.library.sort = SortMode::Default;
    let path = [shared.table_folder(0), shared.level_folder(0, 0)];
    shared.open_for_tests(path);
    shared.sel = 0;
    shared.rebuild_select_items();
    browser
}

/// The rows of the browser as it stands: a held chart by its title, a missing one by its title
/// after `missing`, a folder by its label.
fn rows(shared: &AppShared) -> Vec<String> {
    shared
        .select_items
        .iter()
        .map(|item| match item {
            SelectItem::Song(index) => shared.library.songs()[*index].title.clone(),
            SelectItem::Folder { label, .. } => label.clone(),
            SelectItem::Missing(missing) => format!("missing {}", shared.missing_chart(missing).map(missing_chart_title).unwrap_or_default()),
            SelectItem::Course(course) => shared.table_course(course).map(|course| course.name.clone()).unwrap_or_default(),
        })
        .collect()
}

/// The rows the list holds after rebuilding it through `filter`.
fn rows_through(shared: &mut AppShared, filter: SelectFilter) -> Vec<String> {
    shared.rebuild_select_items_with(filter);
    rows(shared)
}

/// How many rows of the list are charts the library does not hold.
fn missing_rows(shared: &AppShared) -> usize {
    shared.select_items.iter().filter(|item| matches!(item, SelectItem::Missing(_))).count()
}

/// The rows of the open level under the order `sort`.
fn rows_sorted(shared: &mut AppShared, sort: SortMode) -> Vec<String> {
    shared.config.library.sort = sort;
    shared.rebuild_select_items();
    rows(shared)
}

#[test]
fn a_level_lists_its_missing_charts_after_the_owned_ones() {
    let browser = browser_in_the_level("missing-default");
    let mut expected = vec!["alpha".to_owned(), "amber".to_owned()];
    expected.extend(MISSING_IN_TABLE_ORDER.map(str::to_owned));
    assert_eq!(rows(&browser.app.shared), expected, "a chart the table names twice is one bar");
}

#[test]
fn title_order_mixes_missing_and_owned_charts() {
    let mut browser = browser_in_the_level("missing-title");
    assert_eq!(
        rows_sorted(&mut browser.app.shared, SortMode::Title),
        ["missing Aardvark Missing", "alpha", "amber", "missing Amber", "missing Beta Missing", "missing miss-untitled"],
        "a missing chart goes by its title, after a held chart of the same title"
    );
}

#[test]
fn other_orders_put_missing_charts_last() {
    let mut browser = browser_in_the_level("missing-other-orders");
    for sort in [SortMode::Level, SortMode::Clear, SortMode::Artist] {
        assert_eq!(
            rows_sorted(&mut browser.app.shared, sort),
            ["alpha", "amber", "missing Aardvark Missing", "missing Amber", "missing Beta Missing", "missing miss-untitled"],
            "{sort:?}"
        );
    }
}

#[test]
fn favourites_a_mode_or_a_level_range_hide_missing_charts() {
    let mut browser = browser_in_the_level("missing-filters");
    let shared = &mut browser.app.shared;
    let filters = [
        SelectFilter { favorite_only: true, ..SelectFilter::default() },
        SelectFilter { mode: Some(Mode::BEAT_7K), ..SelectFilter::default() },
        SelectFilter { level_from: Some(1), ..SelectFilter::default() },
        SelectFilter { level_to: Some(12), ..SelectFilter::default() },
    ];
    for filter in filters {
        rows_through(shared, filter);
        assert_eq!(missing_rows(shared), 0, "{filter:?}");
    }
    assert_eq!(
        rows_through(shared, SelectFilter { mode: Some(Mode::BEAT_7K), ..SelectFilter::default() }),
        ["alpha", "amber"],
        "the held charts of the mode stay"
    );
}

#[test]
fn not_cleared_keeps_missing_charts_and_cleared_hides_them() {
    let mut browser = browser_in_the_level("missing-clear-filter");
    let shared = &mut browser.app.shared;
    for clear in [ClearFilter::Any, ClearFilter::Unplayed, ClearFilter::Unclear] {
        rows_through(shared, SelectFilter { clear, ..SelectFilter::default() });
        assert_eq!(missing_rows(shared), MISSING_IN_TABLE_ORDER.len(), "{clear:?}");
    }
    for clear in [ClearFilter::Cleared, ClearFilter::FullCombo] {
        rows_through(shared, SelectFilter { clear, ..SelectFilter::default() });
        assert_eq!(missing_rows(shared), 0, "{clear:?}");
    }
}

#[test]
fn the_level_count_counts_owned_charts_only() {
    let mut browser = browser_in_the_level("missing-counts");
    let shared = &mut browser.app.shared;
    let table = shared.table_folder(0);
    shared.open_for_tests([table]);
    shared.rebuild_select_items();
    assert_eq!(rows(shared), ["LV 1 (2)", "LV 2 (1)"]);
    shared.open_for_tests([]);
    shared.rebuild_select_items();
    assert_eq!(rows(shared), ["ALL SONGS (3)", "TABLE (3)"]);
}

#[test]
fn hiding_missing_charts_in_the_settings_takes_their_bars_away() {
    let mut browser = browser_in_the_level("missing-hidden");
    let shared = &mut browser.app.shared;
    shared.config.library.show_missing_charts = false;
    shared.rebuild_select_items();
    assert_eq!(rows(shared), ["alpha", "amber"]);
}

#[test]
fn a_missing_chart_is_a_song_bar_not_on_disk() {
    let mut browser = browser_in_the_level("missing-bar");
    let shared = &browser.app.shared;
    let bars = shared.select_bars(&ChartFacts::unread(&shared.library), None, true);
    let beta = &bars[2];
    assert_eq!((beta.kind, beta.title.as_str()), (BarKind::Song { exists: false }, "Beta Missing"));
    assert_eq!(beta.missing, Some(BarMissing { artist: BETA_ARTIST.to_owned() }));
    assert!(beta.chart.is_none() && beta.lamp.is_none() && beta.distribution.is_none(), "nothing the library or the records say");
    assert_eq!(bars[4].title, UNTITLED_MD5, "a chart the table gives no title is called by its md5");
    let wheel = wheel_bar(beta, 0);
    assert_eq!((wheel.kind, wheel.level, wheel.lamp, wheel.features), (BarKind::Song { exists: false }, 0, 0, 0));
    assert!(bars[0].missing.is_none() && bars[0].kind == BarKind::Song { exists: true }, "a held chart is the bar it was");

    browser.app.shared.sel = 2;
    browser.frame();
    let scene = browser.state.cached_scene.as_ref().expect("the browser drew its scene");
    assert_eq!(
        scene.rows.iter().map(|row| row.missing).collect::<Vec<_>>(),
        [false, false, true, true, true, true],
        "the built-in rows of the missing charts are dimmed"
    );
    assert!(
        matches!(&scene.detail, SelectDetail::Missing { title, artist } if title == "Beta Missing" && artist == BETA_ARTIST),
        "the panel names the missing chart"
    );
}

#[test]
fn a_missing_bar_names_its_title_and_artist() {
    let beta = chart(BETA_MD5, "1", "Beta Missing", BETA_ARTIST);
    let meta = missing_chart_meta(&beta);
    let state = ChartState::Chart(&meta);
    let text = |id: i32| state.text(id).map(|text| text.into_owned());
    assert_eq!(
        [STRING_TITLE, STRING_FULLTITLE, STRING_ARTIST].map(text),
        [Some("Beta Missing".to_owned()), Some("Beta Missing".to_owned()), Some(BETA_ARTIST.to_owned())]
    );
    assert_eq!((meta.md5, meta.mode, meta.notes), (BETA_MD5, None, None), "nothing the chart's file would say");
}

#[test]
fn the_cursor_on_a_missing_chart_stays_when_it_becomes_owned() {
    let mut browser = browser_in_the_level("missing-becomes-owned");
    let shared = &mut browser.app.shared;
    shared.sel = 2;
    let place = ListPlace::of(shared);

    let mut songs = shared.library.songs().to_vec();
    songs.push(SongEntry { md5: BETA_MD5.to_owned(), ..entry("Beta Missing", BETA_ARTIST, "5") });
    shared.library = Library::from_songs(songs);
    rematch(shared);
    show_the_list_again(shared, &place);

    let under = match &shared.select_items[shared.sel] {
        SelectItem::Song(index) => shared.library.songs()[*index].md5.clone(),
        SelectItem::Folder { .. } | SelectItem::Missing(_) | SelectItem::Course(_) => String::new(),
    };
    assert_eq!(under, BETA_MD5, "the cursor is not on the chart that arrived");
    assert_eq!(missing_rows(shared), MISSING_IN_TABLE_ORDER.len() - 1, "the chart is no longer listed as missing");
}

#[test]
fn entering_a_missing_chart_starts_nothing() {
    crate::notify::exclusive(|| {
        let mut browser = browser_in_the_level("missing-enter");
        browser.app.shared.sel = 2;
        assert!(matches!(browser.key(KeyCode::Enter), Transition::Stay), "a missing chart started something");
        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        assert_eq!(said, [(crate::notify::Level::Info, NOT_IN_THE_LIBRARY.to_owned())]);
        assert_eq!(browser.app.shared.sel, 2, "the cursor moved");
    });
}

#[test]
fn the_next_song_after_a_run_skips_missing_charts() {
    let mut browser = browser_in_the_level("missing-next-song");
    let shared = &mut browser.app.shared;
    let alpha = shared.select_items.iter().position(|item| matches!(item, SelectItem::Song(0))).expect("alpha is listed");
    shared.chart_path = shared.library.songs()[0].path.to_string_lossy().into_owned();
    shared.select_items.swap(alpha + 1, alpha + 2);
    assert!(matches!(shared.select_items[alpha + 1], SelectItem::Missing(_)), "the fixture puts a missing chart after alpha");
    let amber = shared.select_items.iter().position(|item| matches!(item, SelectItem::Song(1))).expect("amber is listed");
    assert_eq!(following_song(shared), Some((amber, 1)), "the walk stopped on a chart that is not there");
}

/// The download folder is scanned with the song folders, so a player who has only chosen it has a
/// folder to look in: an empty list then says it holds no charts rather than asking for a folder.
#[test]
fn a_player_with_only_a_download_folder_is_told_there_are_no_charts() {
    let mut browser = Browser::over("missing-only-downloads", Vec::new(), false);
    browser.frame();
    let hint = |browser: &Browser| browser.state.cached_scene.as_ref().and_then(|scene| scene.empty_hint).map(|(title, _)| title);
    assert_eq!(hint(&browser), Some("WELCOME TO rbms"), "a player with no folder at all is asked for one");

    browser.app.shared.config.library.download_folder = Some("/home/player/downloads".to_owned());
    browser.app.shared.rebuild_select_items();
    browser.frame();
    assert_eq!(hint(&browser), Some("NO CHARTS"), "the download folder is a folder the library is read from");
}

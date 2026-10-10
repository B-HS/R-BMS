//! The browser's list as a model: what a bar carries, how a folder's charts are counted, that the
//! built-in rows made from the model are the rows the browser always drew, and that a skin's frame
//! is given the bars and the pictures of the chart under the cursor.
//!
//! The library here is written by the tests themselves: a song database in memory holding a few
//! rows, read back the way the application reads its own.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rbms_course::{Course, CourseChart, CourseConstraint};
use rbms_library::songdb::{FEATURE_LONG_NOTE, FEATURE_MINE_NOTE, FEATURE_RANDOM, FEATURE_UNDEFINED_LN, SongDb, SongRow, mode_id};
use rbms_render::skin_render::frame::{BarDistribution, BarKind, SongBar};

use super::images::{SongImages, Wanted};
use super::list::{ChartFacts, course_bars};
use super::scene::wheel_bar;
use super::tests::{app, press, record, release};
use super::*;
use crate::App;
use crate::stage::HeadlessCanvas;
use crate::stage::capture::{app_in, scene_frame_at, settings_of, skin_folder_of};

/// The folder the fixture charts say they are in. Nothing is read from it.
const CHART_FOLDER: &str = "/rbms-fixture";

/// When the fixture charts entered the database, in seconds, and a second well after any of them
/// stopped being new.
const ADDED_AT: i64 = 1_000_000;
const DAY_SECS: i64 = 86_400;

/// The EX ceiling of every fixture record, which is what [`record`] writes.
const RECORD_MAX_EX: u32 = 1000;

/// Longest a test waits for the worker that reads pictures.
const PICTURE_WAIT: Duration = Duration::from_secs(20);

/// One chart of the fixture library as the database holds it.
fn chart_row(file: &str, title: &str, level: &str, difficulty: i32, feature: i32) -> SongRow {
    SongRow {
        path: format!("{CHART_FOLDER}/{file}"),
        md5: format!("md5-{file}"),
        title: title.to_string(),
        level: level.to_string(),
        difficulty,
        mode: mode_id(Mode::BEAT_7K),
        feature,
        adddate: ADDED_AT,
        ..SongRow::default()
    }
}

/// An app whose library was read back from a database holding `rows`, browsing the flat list in
/// title order.
fn browsing_rows(mut app: App, rows: &[SongRow]) -> App {
    let mut db = SongDb::open_in_memory().expect("a database in memory opens");
    db.migrate().expect("the layout is created");
    db.upsert_batch(rows).expect("the rows are stored");
    app.shared.library = crate::library::stored_library(&db);
    app.shared.song_db = Some(db);
    app.shared.select_view = SelectView::AllSongs;
    app.shared.config.library.sort = SortMode::Title;
    app.shared.rebuild_select_items();
    app
}

/// The five charts most of these tests browse, in the title order the list shows them in.
fn fixture_rows() -> Vec<SongRow> {
    vec![
        SongRow {
            subtitle: "[ANOTHER]".to_string(),
            stagefile: "stage.png".to_string(),
            banner: "banner.png".to_string(),
            backbmp: "back.png".to_string(),
            ..chart_row("a.bms", "alpha", "12", 4, FEATURE_UNDEFINED_LN | FEATURE_MINE_NOTE)
        },
        chart_row("b.bms", "beta", "?", 9, FEATURE_RANDOM),
        chart_row("c.bms", "gamma", "7", 2, 0),
        SongRow { mode: mode_id(Mode::BEAT_14K), ..chart_row("d.bms", "omega", "3", 1, FEATURE_LONG_NOTE) },
        chart_row("e.bms", "zeta", "1", 0, 0),
    ]
}

/// The fixture library with a record on three of its charts.
fn fixture_app() -> App {
    let mut app = browsing_rows(app(), &fixture_rows());
    app.shared.scores = ScoreBook::from_records(vec![
        record("md5-a.bms", 7, 900, 2, 1_000),
        record("md5-a.bms", 4, 950, 9, 2_000),
        record("md5-c.bms", 1, 100, 40, 1_000),
        record("md5-d.bms", 10, RECORD_MAX_EX, 0, 1_000),
    ]);
    app.shared.favorites.toggle("md5-c.bms");
    app
}

fn facts_of(app: &App) -> ChartFacts {
    ChartFacts::read(app.shared.song_db.as_ref(), &app.shared.library)
}

fn bar_titled<'a>(bars: &'a [SelectBar], title: &str) -> &'a SelectBar {
    bars.iter().find(|bar| bar.title == title).unwrap_or_else(|| panic!("no bar is called {title}"))
}

/// A chart's bar says what the library, the score book and the song database each know about it,
/// and what none of them knows is left at nothing.
#[test]
fn a_chart_bar_carries_what_the_library_the_records_and_the_database_know() {
    let app = fixture_app();
    let bars = app.shared.select_bars(&facts_of(&app), None, true);
    assert_eq!(bars.iter().map(|bar| bar.title.as_str()).collect::<Vec<_>>(), ["alpha", "beta", "gamma", "omega", "zeta"]);

    let alpha = bar_titled(&bars, "alpha");
    assert_eq!(alpha.kind, BarKind::Song { exists: true });
    assert_eq!(alpha.full_title(), "alpha [ANOTHER]", "the reference writes the subtitle after the title on a bar");
    assert_eq!(alpha.lamp, Some(7), "the best clear of any run, as the reference numbers its clears");
    assert_eq!((alpha.rival_lamp, alpha.trophy, alpha.distribution), (None, None, None));
    let chart = alpha.chart.as_ref().expect("a chart bar has its chart");
    assert_eq!((chart.level, chart.level_text.as_str(), chart.difficulty), (12, "12", 4));
    assert_eq!(chart.features, SongBar::FEATURE_UNDEFINED_LN | SongBar::FEATURE_MINE_NOTE);
    assert_eq!(chart.added_at, Some(ADDED_AT));
    assert_eq!(chart.best, Some(BarScore { ex: 950, max_ex: RECORD_MAX_EX }), "the run that scored highest, not the one that cleared best");
    assert!(!chart.favorite);
    let folder = Path::new(CHART_FOLDER);
    assert_eq!(
        chart.images,
        BarImages { stagefile: Some(folder.join("stage.png")), banner: Some(folder.join("banner.png")), backbmp: Some(folder.join("back.png")) }
    );

    let beta = bar_titled(&bars, "beta");
    let chart = beta.chart.as_ref().expect("a chart bar has its chart");
    assert_eq!((chart.level, chart.level_text.as_str(), chart.difficulty), (0, "?", 9), "a level that is not a number reads as zero");
    assert_eq!((beta.lamp, chart.best), (None, None), "nothing was recorded on it");
    assert_eq!(chart.images, BarImages::default(), "it names no pictures");
    assert_eq!(beta.full_title(), "beta");

    assert!(bar_titled(&bars, "gamma").chart.as_ref().is_some_and(|chart| chart.favorite));
    assert_eq!(bar_titled(&bars, "gamma").lamp, Some(1));
}

/// Without the song database's facts a chart is still a bar; it only has nothing to be labelled by.
#[test]
fn a_bar_built_without_the_database_has_no_features_and_no_arrival() {
    let app = fixture_app();
    let bars = app.shared.select_bars(&ChartFacts::unread(&app.shared.library), None, true);
    let chart = bar_titled(&bars, "alpha").chart.as_ref().expect("a chart bar has its chart");
    assert_eq!((chart.features, chart.added_at, chart.images.backbmp.as_ref()), (0, None, None));
    assert!(chart.images.stagefile.is_some(), "the library's entry names the stage file by itself");
}

/// What a wheel reads off a chart's bar is what the reference's renderer asks its bar for.
#[test]
fn a_wheel_bar_is_the_charts_bar_as_the_reference_asks_for_it() {
    let app = fixture_app();
    let bars = app.shared.select_bars(&facts_of(&app), None, true);

    let alpha = wheel_bar(bar_titled(&bars, "alpha"), ADDED_AT + DAY_SECS);
    assert_eq!(alpha.kind, BarKind::Song { exists: true });
    assert_eq!(alpha.title, "alpha [ANOTHER]");
    assert_eq!((alpha.level, alpha.difficulty, alpha.lamp, alpha.rival_lamp), (12, 4, 7, 0));
    assert_eq!(alpha.features, SongBar::FEATURE_UNDEFINED_LN | SongBar::FEATURE_MINE_NOTE);
    assert!(alpha.is_new, "a chart is new up to and including a day after it arrived");
    assert!(!wheel_bar(bar_titled(&bars, "alpha"), ADDED_AT + DAY_SECS + 1).is_new, "and not a second longer");

    let beta = wheel_bar(bar_titled(&bars, "beta"), ADDED_AT + DAY_SECS + 1);
    assert_eq!((beta.level, beta.difficulty, beta.lamp), (0, 9, 0), "nothing recorded reads as no play");
    assert_eq!(beta.features, SongBar::FEATURE_RANDOM);
}

/// The wheel's bars are kept until a chart on show stops being new, and remade then.
#[test]
fn the_wheels_bars_are_remade_when_a_chart_stops_being_new() {
    let app = fixture_app();
    let mut state = SelectState::new();
    state.facts = facts_of(&app);
    state.refresh_bars(&app.shared);

    assert!(state.bars.wheel(ADDED_AT).iter().all(|bar| bar.is_new));
    assert!(state.bars.wheel(ADDED_AT + DAY_SECS).iter().all(|bar| bar.is_new));
    assert!(state.bars.wheel(ADDED_AT + DAY_SECS + 1).iter().all(|bar| !bar.is_new), "the bars were not remade once the day was over");
    assert_eq!(state.bars.wheel(ADDED_AT + DAY_SECS + 1).len(), state.bars.bars().len());
}

/// The rows of the root: the flat list is the reference's folder of charts, a table is its table
/// bar and a level of one is the hash folder under it. Only the first and the last are counted.
#[test]
fn the_rows_that_open_are_folders_and_tables_and_only_some_are_counted() {
    let mut app = fixture_app();
    app.shared.table_names = vec!["TABLE".to_string()];
    app.shared.table_levels = vec![vec![("1".to_string(), vec![0, 1]), ("2".to_string(), vec![2, 3, 4])]];
    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();
    let root = app.shared.select_bars(&facts_of(&app), None, true);
    assert_eq!(root.iter().map(|bar| bar.kind).collect::<Vec<_>>(), [BarKind::Folder, BarKind::Table]);
    assert!(root[0].title.starts_with("ALL SONGS"), "a folder is called what the browser has always called it");
    assert!(root[0].distribution.is_some(), "the charts of a folder are counted");
    assert_eq!((root[1].distribution, root[1].lamp), (None, None), "a table's bars are levels, so there is nothing to count under it");
    assert!(root.iter().all(|bar| bar.chart.is_none() && bar.course.is_none()));

    app.shared.select_view = SelectView::TableLevels(0);
    app.shared.rebuild_select_items();
    let levels = app.shared.select_bars(&facts_of(&app), None, true);
    assert_eq!(levels.iter().map(|bar| bar.kind).collect::<Vec<_>>(), [BarKind::Table, BarKind::Table]);
    let counted: Vec<u32> = levels.iter().map(|bar| bar.distribution.map_or(0, |counted| counted.lamps.iter().sum())).collect();
    assert_eq!(counted, [2, 3], "each level counts the charts of its own");
}

/// A folder counts each chart once under its best lamp and once under the rank of its best score
/// (`exscore * 27 / (notes * 2)`), a chart nobody played under no play and the lowest rank, and
/// only the charts of the mode the browser is filtered to.
#[test]
fn a_folder_counts_its_charts_by_lamp_and_by_rank() {
    let mut app = fixture_app();
    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();

    let all = app.shared.select_bars(&facts_of(&app), None, true)[0].distribution.expect("the folder is counted");
    let mut expected = BarDistribution::default();
    for lamp in [7, 0, 1, 10, 0] {
        expected.lamps[lamp] += 1;
    }
    for rank in [950 * 27 / 1000, 0, 100 * 27 / 1000, 27, 0] {
        expected.ranks[rank] += 1;
    }
    assert_eq!(all, expected);
    assert_eq!(app.shared.select_bars(&facts_of(&app), None, true)[0].lamp, Some(0), "a folder's lamp is the lowest one any of its charts sits on");

    let sevens = app.shared.select_bars(&facts_of(&app), Some(Mode::BEAT_7K), true)[0].distribution.expect("the folder is counted");
    assert_eq!(sevens.lamps.iter().sum::<u32>(), 4, "the chart of another mode is not counted");
    assert_eq!(sevens.lamps[10], 0);

    app.shared.scores = ScoreBook::from_records(fixture_rows().iter().map(|row| record(&row.md5, 5, 500, 0, 1_000)).collect());
    let cleared = &app.shared.select_bars(&facts_of(&app), None, true)[0];
    assert_eq!(cleared.lamp, Some(5), "with every chart cleared the lowest lamp is that clear");
    assert_eq!(wheel_bar(cleared, ADDED_AT).lamp, 5);
    assert!(wheel_bar(cleared, ADDED_AT).distribution.is_some());
}

fn course(name: &str, md5s: &[&str], constraints: Vec<CourseConstraint>) -> Course {
    let charts = md5s.iter().map(|md5| CourseChart { md5: (*md5).to_string(), sha256: String::new(), title: (*md5).to_string() }).collect();
    let mut course = Course { name: name.to_string(), charts, constraints, ..Course::default() };
    assert!(course.validate(), "the fixture course is one the browser would keep");
    course
}

/// A course is the reference's grade bar: complete when the library holds every chart of it, and
/// labelled with the features of all its charts when it is. Its clear and its medal have no source
/// in this build.
#[test]
fn a_course_bar_says_whether_it_is_complete_and_what_its_charts_are_made_of() {
    let app = fixture_app();
    let mut courses = CourseList::from_courses(vec![
        course("WHOLE", &["md5-a.bms", "md5-b.bms"], vec![CourseConstraint::Class]),
        course("BROKEN", &["md5-a.bms", "md5-missing"], Vec::new()),
    ]);
    courses.resolve_in_library(&app.shared.library);
    let bars = course_bars(&courses, &app.shared.library, &facts_of(&app));

    assert_eq!(bars.iter().map(|bar| bar.kind).collect::<Vec<_>>(), [BarKind::Course { complete: true }, BarKind::Course { complete: false }]);
    let whole = bars[0].course.as_ref().expect("a course bar has its course");
    assert_eq!(whole.stages, 2);
    assert_eq!(whole.features, SongBar::FEATURE_UNDEFINED_LN | SongBar::FEATURE_MINE_NOTE | SongBar::FEATURE_RANDOM);
    assert!(!whole.badges.is_empty(), "the constraint is labelled");
    assert_eq!(wheel_bar(&bars[0], ADDED_AT).features, whole.features);
    assert_eq!(bars[1].course.as_ref().map(|course| course.features), Some(0), "an incomplete course is labelled by nothing");
    assert!(bars.iter().all(|bar| bar.lamp.is_none() && bar.trophy.is_none() && bar.chart.is_none()));
    assert!(wheel_bar(&bars[0], ADDED_AT).trophy.is_none());
}

/// The row the built-in browser drew for a chart before the list had a model, written out as it
/// was then.
fn row_as_it_was_drawn(shared: &AppShared, index: usize) -> SelectRow {
    let entry = &shared.library.songs()[index];
    let lamp = shared.scores.best_clear_for_md5(&entry.md5).map(|clear| clear_label_color(clear_type_from_id(clear)).1).unwrap_or(Color::rgb(44, 44, 54));
    let best = shared.scores.for_md5(&entry.md5).into_iter().max_by_key(|record| record.ex_score);
    SelectRow {
        folder: false,
        title: entry.title.clone(),
        mode_short: mode_short(entry.mode),
        mode_color: mode_color(entry.mode),
        level: entry.level.clone(),
        difficulty_color: difficulty_color(entry.difficulty),
        lamp,
        folder_count: None,
        dj_level: best.filter(|_| shared.config.display.score_graph).map(|best| RANK_BANDS[dj_rank(best.ex_score, best.max_ex)].0),
        favorite: shared.favorites.contains(&entry.md5),
    }
}

/// Everything a built-in row is drawn from.
type RowParts<'a> = (bool, &'a str, &'static str, Color, &'a str, Color, Color, Option<usize>, Option<&'static str>, bool);

fn parts(row: &SelectRow) -> RowParts<'_> {
    (
        row.folder,
        row.title.as_str(),
        row.mode_short,
        row.mode_color,
        row.level.as_str(),
        row.difficulty_color,
        row.lamp,
        row.folder_count,
        row.dj_level,
        row.favorite,
    )
}

/// The rows the built-in browser draws are made from the model now, and they are the rows it drew
/// when it read the library itself: charts with and without records, starred or not, with the rank
/// shown and hidden, and the rows that open.
#[test]
fn the_built_in_rows_made_from_the_model_are_the_rows_the_browser_always_drew() {
    let mut app = fixture_app();
    for score_graph in [true, false] {
        app.shared.config.display.score_graph = score_graph;
        let mut state = SelectState::new();
        let scene = {
            state.refresh_bars(&app.shared);
            state.build_scene(&app.shared)
        };
        assert_eq!(scene.rows.len(), app.shared.select_items.len());
        for (row, item) in scene.rows.iter().zip(&app.shared.select_items) {
            let SelectItem::Song(index) = item else {
                panic!("the flat list holds charts only");
            };
            assert_eq!(parts(row), parts(&row_as_it_was_drawn(&app.shared, *index)), "score graph {score_graph}");
        }
    }

    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();
    let mut state = SelectState::new();
    state.refresh_bars(&app.shared);
    let scene = state.build_scene(&app.shared);
    let row = &scene.rows[0];
    let Some(SelectItem::Folder { label, .. }) = app.shared.select_items.first() else {
        panic!("the root begins with a folder");
    };
    assert_eq!(parts(row), (true, label.as_str(), "", Color::GRAY, "", Color::GRAY, Color::rgb(44, 44, 54), None, None, false));
}

/// The same for the course tab, whose rows are folders that carry the constraint badges as their
/// level and are marked when a stage is missing.
#[test]
fn the_built_in_course_rows_made_from_the_model_are_the_rows_the_browser_always_drew() {
    let app = fixture_app();
    let courses = vec![course("WHOLE", &["md5-a.bms", "md5-b.bms"], vec![CourseConstraint::Class]), course("BROKEN", &["md5-missing"], Vec::new())];
    let mut state = SelectState::on_courses(courses, &app.shared.library);
    state.refresh_bars(&app.shared);
    let scene = state.build_scene(&app.shared);

    let drawn = state.courses.rows();
    assert_eq!(scene.rows.len(), drawn.len());
    for (row, was) in scene.rows.iter().zip(&drawn) {
        let badges = was.badges.join(" ");
        let (difficulty, lamp) = if was.playable { (Color::WHITE, Color::rgb(44, 44, 54)) } else { (Color::RED, Color::rgb(70, 30, 30)) };
        assert_eq!(parts(row), (true, was.title.as_str(), "", Color::GRAY, badges.as_str(), difficulty, lamp, None, None, false));
    }
    assert!(matches!(&scene.detail, SelectDetail::Folder { label, count: 2 } if label == "WHOLE"), "the focused course names itself and its stages");
}

/// A browser no skin draws never asks the song database what its charts are made of, and builds
/// its bars without it; the bars are rebuilt when the list is.
#[test]
fn the_browser_builds_its_bars_without_the_database_until_a_skin_asks() {
    let mut app = fixture_app();
    let mut state = SelectState::new();
    state.refresh_bars(&app.shared);
    assert_eq!(state.bars.bars().len(), 5);
    assert!(
        state.bars.bars().iter().all(|bar| bar.chart.as_ref().is_some_and(|chart| chart.features == 0)),
        "the database was read for a browser no skin draws"
    );

    app.shared.search = "alp".to_string();
    app.shared.rebuild_select_items();
    state.refresh_bars(&app.shared);
    assert_eq!(state.bars.bars().iter().map(|bar| bar.title.as_str()).collect::<Vec<_>>(), ["alpha"], "the bars did not follow the list");
}

/// The built-in browser's list stops at its ends, as it always has.
#[test]
fn without_a_skin_the_list_still_stops_at_its_ends() {
    let mut app = fixture_app();
    let mut state = SelectState::new();
    let now = Instant::now();
    app.shared.sel = 4;
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(KeyCode::ArrowDown));
    assert_eq!(app.shared.sel, 4);
    app.shared.sel = 0;
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(KeyCode::ArrowUp));
    assert_eq!(app.shared.sel, 0);
}

/// Write a picture of one flat colour and answer where it is.
fn write_picture(folder: &Path, name: &str, size: (u32, u32), color: Color) -> PathBuf {
    let file = folder.join(name);
    image::RgbaImage::from_pixel(size.0, size.1, image::Rgba([color.r, color.g, color.b, color.a])).save(&file).expect("the picture is written");
    file
}

/// Poll `images` until the worker has answered everything it was asked.
fn settle(images: &mut SongImages) {
    let deadline = Instant::now() + PICTURE_WAIT;
    while !images.is_settled() {
        assert!(Instant::now() < deadline, "the worker never answered");
        std::thread::yield_now();
        images.poll();
    }
}

/// The pictures of the chart under the cursor are read off the frame loop. A slot whose file
/// changed gives its picture up at once and takes the new one in when it has been read; a slot
/// whose file did not change keeps what it holds; a file that is not a picture leaves its slot
/// empty.
#[test]
fn the_pictures_follow_the_cursor_and_a_picture_that_was_left_is_given_up_at_once() {
    let folder = settings_of("select-pictures").with_file_name("pictures");
    std::fs::create_dir_all(&folder).expect("the picture folder is writable");
    let first = write_picture(&folder, "first.png", (4, 2), Color::RED);
    let second = write_picture(&folder, "second.png", (6, 3), Color::GREEN);
    let banner = write_picture(&folder, "banner.png", (8, 1), Color::BLUE);
    let broken = folder.join("broken.png");
    std::fs::write(&broken, b"not a picture").expect("the broken file is written");

    let mut images = SongImages::default();
    assert!(images.is_settled() && images.cover().is_none());
    images.want(Wanted { stagefile: Some(&first), banner: Some(&banner), backbmp: None });
    assert_eq!(images.sizes(), [None, None, None], "nothing is shown before it has been read");
    settle(&mut images);
    assert_eq!(images.sizes(), [Some((4, 2)), Some((8, 1)), None]);
    assert_eq!(images.cover().map(|cover| (cover.width, cover.height)), Some((4, 2)), "the cover is the stage file when there is one");

    images.want(Wanted { stagefile: Some(&second), banner: Some(&banner), backbmp: Some(&first) });
    assert_eq!(images.sizes(), [None, Some((8, 1)), None], "the stage file of the chart that was left is still held, or the banner was dropped");
    settle(&mut images);
    assert_eq!(images.sizes(), [Some((6, 3)), Some((8, 1)), Some((4, 2))]);

    images.want(Wanted { stagefile: Some(&broken), banner: Some(&banner), backbmp: Some(&first) });
    settle(&mut images);
    assert_eq!(images.sizes(), [None, Some((8, 1)), Some((4, 2))], "a file that is not a picture shows nothing");
    assert_eq!(images.cover().map(|cover| (cover.width, cover.height)), Some((8, 1)), "with no stage file the cover is the banner");

    images.want(Wanted::default());
    assert_eq!(images.sizes(), [None, None, None]);
    assert!(images.is_settled());
}

/// The size the fixture document is authored at and drawn at.
const DOCUMENT_SIZE: (u32, u32) = (320, 180);

/// The side of one cell of the fixture's sheet, in pixels. Each cell is one flat colour.
const CELL: u32 = 8;

/// How many bar images, lamps and labels the fixture's wheel declares.
const BAR_IMAGES: u32 = 7;
const LAMPS: u32 = 11;
const LABELS: u32 = 5;

/// The colours of the sheet's cells: bar image `index`, lamp `index` and label `index`.
fn bar_color(index: u32) -> Color {
    Color::rgb(u8::try_from(40 + 30 * index).unwrap_or(u8::MAX), 0, 0)
}

fn lamp_color(index: u32) -> Color {
    Color::rgb(0, u8::try_from(30 + 20 * index).unwrap_or(u8::MAX), 0)
}

fn label_color(index: u32) -> Color {
    Color::rgb(0, 0, u8::try_from(50 + 40 * index).unwrap_or(u8::MAX))
}

/// Where the fixture's wheel puts its three slots, top to bottom, in the pixels of the target, and
/// where on a slot its bar, its lamp and each of its labels are sampled.
const SLOT_TOPS: [u32; 3] = [40, 80, 120];
const SLOT_MIDDLE: u32 = 10;
const BAR_SAMPLE_X: u32 = 190;
const LAMP_SAMPLE_X: u32 = 105;
const FIRST_LABEL_SAMPLE_X: u32 = 125;
const LABEL_STEP: u32 = 10;

/// Where the fixture draws the stage file and the banner, as the pixel each is sampled at.
const STAGEFILE_SAMPLE: (u32, u32) = (20, 160);
const BANNER_SAMPLE: (u32, u32) = (70, 160);

/// The colours of the two pictures the fixture charts name.
const STAGEFILE_COLOR: Color = Color::rgb(200, 100, 50);
const BANNER_COLOR: Color = Color::rgb(50, 100, 200);
const OTHER_STAGEFILE_COLOR: Color = Color::rgb(120, 20, 220);

/// Images cut side by side from the row of the sheet `row` cells down, named `<name>-<index>`.
fn cells(name: &str, count: u32, row: u32) -> String {
    (0..count)
        .map(|index| format!(r#"{{"id":"{name}-{index}","src":"sheet","x":{},"y":{},"w":{CELL},"h":{CELL}}}"#, index * CELL, row * CELL))
        .collect::<Vec<_>>()
        .join(",")
}

/// A browser document that is a wheel of three slots, the middle one under the cursor, with a lamp
/// and five labels on each bar, and the stage file and the banner of the chart under the cursor
/// drawn below it.
fn wheel_document() -> String {
    let slots = [120, 80, 40].map(|y| format!(r#"{{"id":"bars","dst":[{{"x":100,"y":{y},"w":100,"h":20}}]}}"#)).join(",");
    let lamps = (0..LAMPS).map(|index| format!(r#"{{"id":"lamp-{index}","dst":[{{"x":0,"y":0,"w":10,"h":20}}]}}"#)).collect::<Vec<_>>().join(",");
    let labels =
        (0..LABELS).map(|index| format!(r#"{{"id":"label-{index}","dst":[{{"x":{},"y":0,"w":10,"h":20}}]}}"#, 20 + 10 * index)).collect::<Vec<_>>().join(",");
    let set = (0..BAR_IMAGES).map(|index| format!(r#""bar-{index}""#)).collect::<Vec<_>>().join(",");
    format!(
        r#"{{
            "type": {SKIN_TYPE_MUSIC_SELECT}, "name": "wheel", "w": {}, "h": {},
            "source": [{{ "id": "sheet", "path": "sheet.png" }}],
            "image": [{},{},{}],
            "imageset": [{{ "id": "bars", "images": [{set}] }}],
            "songlist": {{
                "id": "wheel", "center": 1, "clickable": [0, 1, 2],
                "liston": [{slots}], "listoff": [{slots}],
                "lamp": [{lamps}], "label": [{labels}]
            }},
            "destination": [
                {{ "id": "wheel" }},
                {{ "id": "-100", "dst": [{{ "x": 0, "y": 0, "w": 40, "h": 40 }}] }},
                {{ "id": "-102", "dst": [{{ "x": 50, "y": 0, "w": 40, "h": 40 }}] }}
            ]
        }}"#,
        DOCUMENT_SIZE.0,
        DOCUMENT_SIZE.1,
        cells("bar", BAR_IMAGES, 0),
        cells("lamp", LAMPS, 1),
        cells("label", LABELS, 2),
    )
}

/// An app whose browser is drawn by the wheel document, over the fixture library with its pictures
/// written to disk where the charts say they are.
fn wheel_app(tag: &str) -> App {
    let settings = settings_of(tag);
    let skin = skin_folder_of(&settings);
    let sheet = image::RgbaImage::from_fn(LAMPS * CELL, 3 * CELL, |x, y| {
        let color = match y / CELL {
            0 => bar_color(x / CELL),
            1 => lamp_color(x / CELL),
            _ => label_color(x / CELL),
        };
        image::Rgba([color.r, color.g, color.b, color.a])
    });
    sheet.save(skin.join("sheet.png")).expect("the sheet is written");
    let document = skin.join("wheel.json");
    std::fs::write(&document, wheel_document()).expect("the document is written");
    let mut config = Config::default();
    config.skin.select(SKIN_TYPE_MUSIC_SELECT, Some(document.to_string_lossy().into_owned()));
    config.library.preview = false;

    let charts = settings.with_file_name("charts");
    std::fs::create_dir_all(&charts).expect("the chart folder is writable");
    write_picture(&charts, "stage.png", (4, 4), STAGEFILE_COLOR);
    write_picture(&charts, "banner.png", (4, 4), BANNER_COLOR);
    write_picture(&charts, "other.png", (4, 4), OTHER_STAGEFILE_COLOR);
    let rows: Vec<SongRow> = fixture_rows()
        .into_iter()
        .map(|row| {
            let file = Path::new(&row.path).file_name().map(PathBuf::from).unwrap_or_default();
            let stagefile = if row.title == "gamma" { "other.png".to_string() } else { row.stagefile.clone() };
            SongRow { path: charts.join(file).to_string_lossy().into_owned(), stagefile, ..row }
        })
        .collect();
    let mut app = browsing_rows(app_in(settings, config), &rows);
    app.shared.audio_failed = true;
    app.shared.scores = ScoreBook::from_records(vec![record("md5-a.bms", 7, 900, 2, 1_000), record("md5-c.bms", 4, 100, 40, 1_000)]);
    app.stage = Stage::Select(Box::new(SelectState::new()));
    app
}

/// Run the browser's frames, update and draw, until `done` says the canvas shows what is waited
/// for.
fn frames_until(app: &mut App, pixels: &mut HeadlessCanvas, what: &str, done: impl Fn(&App, &HeadlessCanvas) -> bool) {
    let deadline = Instant::now() + PICTURE_WAIT;
    loop {
        scene_frame_at(app, 0, &mut Canvas::Headless(pixels));
        if done(app, pixels) {
            return;
        }
        assert!(Instant::now() < deadline, "the browser never showed {what}: {:?}", app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT));
    }
}

fn pictures_are_in(app: &App) -> bool {
    matches!(&app.stage, Stage::Select(state) if state.pictures_are_in())
}

/// The colour of the bar, the lamp and the five labels of the wheel's slot `slot`.
fn slot_colors(pixels: &HeadlessCanvas, slot: usize) -> (Color, Color, [Color; 5]) {
    let y = SLOT_TOPS[slot] + SLOT_MIDDLE;
    let labels = [0, 1, 2, 3, 4].map(|index| pixels.pixel_at(FIRST_LABEL_SAMPLE_X + LABEL_STEP * index, y));
    (pixels.pixel_at(BAR_SAMPLE_X, y), pixels.pixel_at(LAMP_SAMPLE_X, y), labels)
}

/// A skin's wheel is drawn from the model: each slot shows the bar the ring puts there, cut from
/// the image of its kind, with the lamp the player holds on it and the labels of what its chart is
/// made of. The stage file and the banner are those of the chart under the cursor, and they change
/// with it.
#[test]
fn a_skins_wheel_is_drawn_from_the_model_with_the_pictures_of_the_chart_under_the_cursor() {
    let mut app = wheel_app("select-wheel");
    let mut pixels = HeadlessCanvas::new(DOCUMENT_SIZE.0, DOCUMENT_SIZE.1);
    frames_until(&mut app, &mut pixels, "its document", |app, _| app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) && pictures_are_in(app));
    frames_until(&mut app, &mut pixels, "the pictures of the first chart", |_, pixels| {
        pixels.pixel_at(STAGEFILE_SAMPLE.0, STAGEFILE_SAMPLE.1) == STAGEFILE_COLOR
    });

    let song = bar_color(0);
    let bare = [song; 5];
    let (bar, lamp, labels) = slot_colors(&pixels, 1);
    assert_eq!((bar, lamp), (song, lamp_color(7)), "the bar under the cursor is alpha, cleared on the fourth gauge up");
    assert_eq!(labels, [label_color(0), song, label_color(2), song, song], "a long note of the player's kind and a mine");
    assert_eq!(slot_colors(&pixels, 0), (song, lamp_color(0), bare), "the slot above goes round the end of the list to zeta");
    assert_eq!(
        slot_colors(&pixels, 2),
        (song, lamp_color(0), [song, label_color(1), song, song, song]),
        "the slot below is beta, which picks its notes at random"
    );
    assert_eq!(pixels.pixel_at(BANNER_SAMPLE.0, BANNER_SAMPLE.1), BANNER_COLOR);

    let mut now = Instant::now();
    arrow_step(&mut app, &mut now, KeyCode::ArrowDown);
    assert_eq!(arrow_step(&mut app, &mut now, KeyCode::ArrowDown), 2);
    frames_until(&mut app, &mut pixels, "the pictures of the third chart", |app, pixels| {
        pictures_are_in(app) && pixels.pixel_at(STAGEFILE_SAMPLE.0, STAGEFILE_SAMPLE.1) == OTHER_STAGEFILE_COLOR
    });
    assert_eq!(slot_colors(&pixels, 1), (song, lamp_color(4), bare), "the bar under the cursor is gamma");
    assert_ne!(pixels.pixel_at(BANNER_SAMPLE.0, BANNER_SAMPLE.1), BANNER_COLOR, "gamma names no banner, so the one of the chart that was left is gone");

    app.shared.select_view = SelectView::Root;
    app.shared.sel = 0;
    app.shared.rebuild_select_items();
    frames_until(&mut app, &mut pixels, "the root", |app, pixels| pictures_are_in(app) && slot_colors(pixels, 1).0 == bar_color(1));
    assert_eq!(slot_colors(&pixels, 1).1, lamp_color(0), "a folder holding a chart nobody played sits on no play");
    assert_ne!(pixels.pixel_at(STAGEFILE_SAMPLE.0, STAGEFILE_SAMPLE.1), OTHER_STAGEFILE_COLOR, "a folder has no stage file");
}

/// How far the frame clock is moved on between the frames that carry an arrow out: past the slide a
/// press starts, so the wheel is at rest before the next press.
const SLIDE_REST: Duration = Duration::from_millis(rbms_render::skin_render::frame::SCROLL_DURATION_LOW_MS as u64 + 1);

/// One press of an arrow on the browser a skin draws, as the window hands it over, and the two
/// frames that carry it out: the one that reads the key and moves the cursor, and, with the key let
/// go, one after the slide it started is over. Answers where the cursor is.
fn arrow_step(app: &mut App, now: &mut Instant, code: KeyCode) -> usize {
    for input in [press(code), release(code)] {
        app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now: *now, dt: 0.0 }, input);
        *now += SLIDE_REST;
        app.stage.update(&mut FrameCtx { shared: &mut app.shared, now: *now, dt: 0.0 });
    }
    app.shared.sel
}

/// When a skin draws the browser the cursor goes round the ends of the list, as the reference's
/// does, and an empty list has nowhere to go and nothing to draw.
#[test]
fn with_a_skin_the_cursor_goes_round_the_ends_of_the_list() {
    let mut app = wheel_app("select-ring");
    let mut pixels = HeadlessCanvas::new(DOCUMENT_SIZE.0, DOCUMENT_SIZE.1);
    frames_until(&mut app, &mut pixels, "its document", |app, _| app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT));
    scene_frame_at(&mut app, 0, &mut Canvas::Headless(&mut pixels));
    let mut now = Instant::now();
    let mut step = |app: &mut App, code: KeyCode| arrow_step(app, &mut now, code);

    assert_eq!(step(&mut app, KeyCode::ArrowUp), 4, "up from the first bar is the last");
    assert_eq!(step(&mut app, KeyCode::ArrowDown), 0, "down from the last bar is the first");
    assert_eq!(step(&mut app, KeyCode::ArrowDown), 1);

    app.shared.search = "no chart is called this".to_string();
    app.shared.rebuild_select_items();
    assert!(app.shared.select_items.is_empty());
    assert_eq!(step(&mut app, KeyCode::ArrowDown), 0);
    assert_eq!(step(&mut app, KeyCode::ArrowUp), 0);
    scene_frame_at(&mut app, 0, &mut Canvas::Headless(&mut pixels));
    scene_frame_at(&mut app, 0, &mut Canvas::Headless(&mut pixels));
    assert!(SLOT_TOPS.iter().all(|top| pixels.pixel_at(BAR_SAMPLE_X, top + SLOT_MIDDLE) != bar_color(0)), "an empty list drew a bar");
}

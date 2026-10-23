//! Golden signature test for the song-select screen.

mod golden_harness;

use golden_harness::{check, golden_canvas, signature_of};
use rbms_render::{Color, CoverState, DensityView, DetailView, RecordRowView, RecordsView, SelectDetail, SelectRow, SelectView, StatCell, render_select};

const GOLDEN_SELECT: u64 = 0xdca2_e6b0_4935_5413;
const GOLDEN_SELECT_MISSING: u64 = 0xe5e5_223e_62d9_1f7f;

fn select_view() -> SelectView {
    let row = |folder: bool, title: &str, mode: &'static str, level: &str, lamp: Color| SelectRow {
        folder,
        title: title.into(),
        mode_short: mode,
        mode_color: Color::BLUE,
        level: level.into(),
        difficulty_color: Color::RED,
        lamp,
        folder_count: folder.then_some(86),
        dj_level: (!folder).then_some("AA"),
        favorite: !folder,
        missing: false,
    };
    let rec = |when: &str, ex: u32| RecordRowView { when: when.into(), lamp: Color::WHITE, lamp_label: "HARD", ex, max_ex: 1848, bp: 12, trend: None };
    let detail = DetailView {
        accent: Color::BLUE,
        title: "Legend of Eastern Rabbit -SKY DEFENDER- [7K ANOTHER]".into(),
        subtitle: "Touhou Rabbit Legend".into(),
        artist: "ost1".into(),
        genre_maker: "Oriental Rock  ·  notecharter".into(),
        mode_short: "7K",
        mode_color: Color::BLUE,
        level: "12".into(),
        difficulty_color: Color::RED,
        difficulty_name: "ANOTHER",
        cover: CoverState::None,
        stats: vec![
            StatCell { label: "BPM", value: "145-190".into() },
            StatCell { label: "NOTES", value: "1357 (48LN)".into() },
            StatCell { label: "LENGTH", value: "2:11".into() },
        ],
        density: Some(DensityView { bins: (0..130).map(|i| (i % 11) as u32).collect(), peak: 17.0, avg: 9.0, end: 11.0 }),
        records: RecordsView {
            plays: 12,
            clears: 8,
            best: Some(rec("2026-05-31 01:02", 1602)),
            rank_bar: Some((1602, 1848)),
            recent: vec![rec("2026-05-31 01:02", 1602), rec("2026-05-30 22:14", 1543)],
        },
    };
    SelectView {
        rows: vec![
            row(true, "INSANE BMS DIFFICULTY TABLE", "", "", Color::GRAY),
            row(false, "Legend of Eastern Rabbit -SKY DEFENDER-", "7K", "7", Color::GREEN),
            row(false, "Another Song With A Fairly Long Title Indeed", "5K", "5", Color::BLUE),
            row(false, "Short", "7K", "3", Color::GRAY),
        ],
        sel: 1,
        header: "ROOT / INSANE TABLE / LEVEL 12".into(),
        guide: "UP DOWN  LEFT BACK  RIGHT/ENTER OPEN",
        detail: SelectDetail::Song(Box::new(detail)),
        modal: None,
        score_graph: true,
        search: None,
        sort: "DEFAULT",
        filter: Some("LV 10\u{2013}12  FAVOURITES".into()),
        empty_hint: None,
    }
}

#[test]
fn select_screen_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    let hot = render_select(&mut canvas, &select_view());
    assert!(!hot.is_empty(), "select reports clickable regions");
    check(&canvas, "select", "GOLDEN_SELECT", GOLDEN_SELECT);
}

/// A row's DJ rank and its favourite marker are small marks on a busy row, so this pins that the
/// signature is fine enough to see either of them go.
#[test]
fn the_signature_detects_a_dropped_row_mark() {
    let base = signature_of(|c| {
        render_select(c, &select_view());
    });
    let no_rank = signature_of(|c| {
        let mut view = select_view();
        for row in &mut view.rows {
            row.dj_level = None;
        }
        render_select(c, &view);
    });
    assert_ne!(base, no_rank, "removing every row's DJ rank changes the signature");
    let no_star = signature_of(|c| {
        let mut view = select_view();
        for row in &mut view.rows {
            row.favorite = false;
        }
        render_select(c, &view);
    });
    assert_ne!(base, no_star, "removing every favourite marker changes the signature");
}

/// A filter with its panel closed is only visible as this line, so it has to reach the frame.
#[test]
fn the_signature_detects_a_dropped_filter_line() {
    let base = signature_of(|c| {
        render_select(c, &select_view());
    });
    let unfiltered = signature_of(|c| {
        let mut view = select_view();
        view.filter = None;
        render_select(c, &view);
    });
    assert_ne!(base, unfiltered, "dropping the filter line changes the signature");
}

/// The fixture title wraps to two lines, and the detail panel draws the subtitle only when it does
/// not, so the artist line is the fixture's droppable detail row. This pins the harness's
/// sensitivity for this screen.
#[test]
fn the_signature_detects_a_dropped_select_detail_line() {
    let base = signature_of(|c| {
        render_select(c, &select_view());
    });
    let dropped = signature_of(|c| {
        let mut view = select_view();
        if let SelectDetail::Song(detail) = &mut view.detail {
            detail.artist = String::new();
        }
        render_select(c, &view);
    });
    assert_ne!(base, dropped, "removing the artist line changes the signature");
}

/// A level of a table that lists charts the library does not hold: the held charts as they are drawn
/// anywhere, the missing ones after them dimmed with no lamp, mode or level, and the cursor on one of
/// them, whose panel names it and says it is not here.
fn missing_charts_view() -> SelectView {
    let held = |title: &str, level: &str, lamp: Color| SelectRow {
        folder: false,
        title: title.into(),
        mode_short: "7K",
        mode_color: Color::BLUE,
        level: level.into(),
        difficulty_color: Color::RED,
        lamp,
        folder_count: None,
        dj_level: None,
        favorite: false,
        missing: false,
    };
    let missing = |title: &str| SelectRow { missing: true, mode_short: "", level: String::new(), lamp: Color::GRAY, ..held(title, "", Color::GRAY) };
    SelectView {
        rows: vec![
            held("Owned Chart", "12", Color::GREEN),
            held("Another Owned One", "12", Color::BLUE),
            missing("Not Downloaded Yet"),
            missing("Also Missing"),
        ],
        sel: 2,
        header: "STELLA / STELLA / st3".into(),
        guide: "UP DOWN  LEFT BACK  RIGHT/ENTER OPEN",
        detail: SelectDetail::Missing { title: "Not Downloaded Yet".into(), artist: "Some Artist".into() },
        modal: None,
        score_graph: true,
        search: None,
        sort: "DEFAULT",
        filter: None,
        empty_hint: None,
    }
}

#[test]
fn built_in_list_with_missing_charts() {
    let mut canvas = golden_canvas();
    render_select(&mut canvas, &missing_charts_view());
    check(&canvas, "select with missing charts", "GOLDEN_SELECT_MISSING", GOLDEN_SELECT_MISSING);
}

/// A missing row is drawn apart from a held one, and its panel's artist line reaches the frame.
#[test]
fn the_signature_detects_a_missing_row_drawn_as_a_held_one() {
    let base = signature_of(|c| {
        render_select(c, &missing_charts_view());
    });
    let held = signature_of(|c| {
        let mut view = missing_charts_view();
        for row in &mut view.rows {
            row.missing = false;
        }
        render_select(c, &view);
    });
    assert_ne!(base, held, "drawing the missing rows as held ones changes the signature");
    let no_artist = signature_of(|c| {
        let mut view = missing_charts_view();
        view.detail = SelectDetail::Missing { title: "Not Downloaded Yet".into(), artist: String::new() };
        render_select(c, &view);
    });
    assert_ne!(base, no_artist, "dropping the artist of the missing chart changes the signature");
}

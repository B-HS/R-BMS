//! Golden signature tests for the three composed screens (result, song select, play HUD).
//!
//! Each test renders a fixed view into a [`CpuCanvas`] and compares
//! `CpuCanvas::signature_hash(GOLDEN_COLS, GOLDEN_ROWS)` — a coarse, quantized block signature — to a
//! baked literal. That catches a moved panel, a dropped element or a recoloured row while tolerating
//! sub-pixel antialiasing differences.
//!
//! Every screen here is rendered through [`rbms_render::font::use_embedded_fonts_only`]. The default
//! text engine scans installed system fonts and falls back to them for codepoints the bundled font
//! lacks, so the same string rasterizes differently on each operating system and the signature is
//! host-specific. With only the bundled face loaded the render depends on nothing outside this
//! repository, so one literal holds on every host.
//!
//! Regenerating the goldens after an intentional visual change: run `cargo test -p rbms-render
//! --test golden`; any failing assertion prints the current value of all four constants as a block
//! ready to paste over the ones below, so a change touching several screens needs one run.

use rbms_model::Mode;
use rbms_render::{
    Color, CoverState, CpuCanvas, DensityView, DetailView, HudView, RecordRowView, RecordsView, Renderer, ResultPalette, ResultView, SelectDetail, SelectRow,
    SelectView, Skin, SkinConfig, StatCell, render_hud, render_result, render_result_with_palette, render_select,
};

/// Signature grid. 64×36 keeps the 16:9 reference aspect, so each block is a square 20×20 region.
/// A 16×9 grid was measured to be too coarse: an 80×80 block averaged a whole text run away, so a
/// changed EX score digit or a swapped clear label hashed identically. See the sensitivity tests.
const GOLDEN_COLS: u32 = 64;
const GOLDEN_ROWS: u32 = 36;
const SCREEN_W: u32 = 1280;
const SCREEN_H: u32 = 720;

const GOLDEN_RESULT: u64 = 0x3989_b4eb_039b_a998;
const GOLDEN_RESULT_SKIN: u64 = 0x6dbb_65c3_2707_304b;
const GOLDEN_SELECT: u64 = 0x1615_42fd_055a_fe80;
const GOLDEN_HUD: u64 = 0x3e77_31ab_195e_dba9;

/// The constant behind each golden, in the order [`current_signatures`] returns them, so a failure
/// can name the line to edit.
const GOLDEN_NAMES: [&str; 4] = ["GOLDEN_RESULT", "GOLDEN_RESULT_SKIN", "GOLDEN_SELECT", "GOLDEN_HUD"];

/// Render every golden screen and return its signature, in [`GOLDEN_NAMES`] order.
fn current_signatures() -> [u64; 4] {
    [
        signature_of(|c| render_result(c, &result_view())),
        signature_of(|c| render_result_with_palette(c, &result_view(), &ResultPalette::from_skin(&SkinConfig::default()))),
        signature_of(|c| {
            render_select(c, &select_view());
        }),
        signature_of(hud_screen),
    ]
}

/// The four `const` lines as they should read right now, so one failing run regenerates them all.
fn regenerated_constants() -> String {
    let mut out = String::new();
    for (name, hash) in GOLDEN_NAMES.iter().zip(current_signatures()) {
        let (hi, lo) = ((hash >> 32) as u32, hash as u32);
        out.push_str(&format!("const {name}: u64 = 0x{:04x}_{:04x}_{:04x}_{:04x};\n", hi >> 16, hi & 0xffff, lo >> 16, lo & 0xffff));
    }
    out
}

fn check(canvas: &CpuCanvas, screen: &str, expected: u64) {
    let actual = canvas.signature_hash(GOLDEN_COLS, GOLDEN_ROWS);
    assert_eq!(actual, expected, "{screen} golden signature changed; replace all four constants with:\n{}", regenerated_constants());
}

/// A blank screen-sized canvas whose text is rendered from the bundled font alone, so the result is
/// identical on every host. See the module docs.
fn golden_canvas() -> CpuCanvas {
    rbms_render::font::use_embedded_fonts_only();
    CpuCanvas::new(SCREEN_W, SCREEN_H)
}

fn result_view() -> ResultView {
    ResultView {
        title: "GOLDEN RESULT".into(),
        counts: [712, 64, 21, 8, 5, 2],
        ex_score: 1488,
        max_score: 1624,
        max_combo: 612,
        total_notes: 812,
        fast: 30,
        slow: 40,
        gauge: 78.0,
        clear_label: "HARD CLEAR",
        clear_color: Color::WHITE,
        prev_best_ex: Some(1440),
        prev_ex: Some(1502),
        show_graph: true,
    }
}

fn signature_of(draw: impl FnOnce(&mut CpuCanvas)) -> u64 {
    let mut canvas = golden_canvas();
    draw(&mut canvas);
    canvas.signature_hash(GOLDEN_COLS, GOLDEN_ROWS)
}

#[test]
fn result_screen_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    render_result(&mut canvas, &result_view());
    check(&canvas, "result", GOLDEN_RESULT);
}

/// The player draws the result screen through `render_result_with_palette` with a palette built from
/// the active skin, whose judge colours and labels differ from the built-in ones. This golden pins
/// the pixels a user actually sees; `result_screen_matches_its_golden_signature` pins the library
/// default.
#[test]
fn result_screen_with_the_default_skin_palette_matches_its_golden_signature() {
    let palette = ResultPalette::from_skin(&SkinConfig::default());
    let mut canvas = golden_canvas();
    render_result_with_palette(&mut canvas, &result_view(), &palette);
    check(&canvas, "result (skin palette)", GOLDEN_RESULT_SKIN);
}

/// The two result goldens must not collide: the built-in palette paints PGREAT hot pink and labels
/// it "PGREAT", the default skin paints it green and labels it "PERFECT".
#[test]
fn the_builtin_and_skin_result_goldens_differ() {
    assert_ne!(GOLDEN_RESULT, GOLDEN_RESULT_SKIN, "the skin palette repaints and relabels the judge rows");
}

/// The harness only earns its keep if it fails on the regressions it claims to catch, so these pin
/// its sensitivity: a one-digit score change, a swapped clear label and a dropped detail line must
/// each move the signature.
#[test]
fn the_signature_detects_a_single_changed_score_digit() {
    let base = signature_of(|c| render_result(c, &result_view()));
    let bumped = signature_of(|c| render_result(c, &ResultView { ex_score: 1489, ..result_view() }));
    assert_ne!(base, bumped, "an EX score of 1489 instead of 1488 changes the signature");
}

#[test]
fn the_signature_detects_a_swapped_clear_label() {
    let base = signature_of(|c| render_result(c, &result_view()));
    let failed = signature_of(|c| render_result(c, &ResultView { clear_label: "FAILED", ..result_view() }));
    assert_ne!(base, failed, "relabelling HARD CLEAR to FAILED changes the signature");
}

/// The fixture title wraps to two lines, and the detail panel draws the subtitle only when it does
/// not, so the artist line is the fixture's droppable detail row.
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
        empty_hint: None,
    }
}

#[test]
fn select_screen_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    let hot = render_select(&mut canvas, &select_view());
    assert!(!hot.is_empty(), "select reports clickable regions");
    check(&canvas, "select", GOLDEN_SELECT);
}

/// The play HUD over its cleared background, drawn on the 7K default skin.
fn hud_screen(canvas: &mut CpuCanvas) {
    let skin = Skin::default_for(Mode::BEAT_7K, SCREEN_W as f32, SCREEN_H as f32);
    let hud = HudView {
        combo: 123,
        last_judge: Some(0),
        last_fast: false,
        fast: 30,
        slow: 40,
        counts: [712, 64, 21, 8, 5, 2],
        ex_score: 1488,
        gauge: 78.0,
        green_number: 310.0,
        max_ex: 1624,
        best_ex: Some(1502),
    };
    canvas.clear(Color::BLACK);
    render_hud(canvas, &skin, &hud);
}

#[test]
fn hud_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    hud_screen(&mut canvas);
    check(&canvas, "hud", GOLDEN_HUD);
}

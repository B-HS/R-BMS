//! Golden signature tests for the play screen: the HUD, and the field layouts a five-key chart is
//! given when it is laid out on its own terms.

mod golden_harness;

use golden_harness::{SCREEN_H, SCREEN_W, check, golden_canvas, signature_of};
use rbms_chart::to_model;
use rbms_model::Mode;
use rbms_parser::parse;
use rbms_render::playfield::LaneShade;
use rbms_render::{Color, CpuCanvas, HudView, PlayfieldView, Renderer, Skin, SkinConfig, render_hud, render_lane_cover, render_playfield_view};

const GOLDEN_HUD: u64 = 0x1781_093b_726e_d94a;
const GOLDEN_FIVE_KEY_FIELD: u64 = 0xd202_a9cc_05f1_8e51;
const GOLDEN_SEVEN_KEY_FIELD: u64 = 0xf15b_3ad8_9d56_8acd;

/// A one-measure chart with every lane of a five-key layout filled, so the field golden is taken of
/// notes rather than of an empty box.
const CHART: &[u8] = b"#PLAYER 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00111:0101\r\n#00112:0100\r\n#00113:0001\r\n#00114:0101\r\n#00115:0100\r\n#00116:0001\r\n";

/// Where in the chart the field goldens are taken.
const FIELD_MICROTIME_US: i64 = 400_000;

/// Scroll speed the field goldens are taken at.
const FIELD_HISPEED: f64 = 1.5;

/// The HUD the golden is taken of.
fn hud_view() -> HudView<'static> {
    HudView {
        mode_label: "7K",
        pace: None,
        combo: 123,
        last_judge: Some(0),
        last_fast: false,
        fast: [26, 4],
        slow: [35, 5],
        counts: [712, 64, 21, 8, 5, 2],
        ex_score: 1488,
        gauge: 78.0,
        green_number: 310.0,
        white_number: 145.0,
        judge_text_y: 0.0,
        max_ex: 1624,
        best_ex: Some(1502),
    }
}

/// One HUD over a cleared background, drawn on the 7K default skin.
fn draw_hud(canvas: &mut CpuCanvas, hud: &HudView<'_>) {
    let skin = Skin::default_for(Mode::BEAT_7K, SCREEN_W as f32, SCREEN_H as f32);
    canvas.clear(Color::BLACK);
    render_hud(canvas, &skin, hud);
}

/// The play HUD the golden is taken of.
fn hud_screen(canvas: &mut CpuCanvas) {
    draw_hud(canvas, &hud_view());
}

/// One playfield frame in `mode`, with the five-key layout switched on or off.
fn draw_field(canvas: &mut CpuCanvas, mode: Mode, five_key_layout: bool) {
    let model = to_model(&parse(CHART), mode);
    let cfg = SkinConfig { five_key_layout, ..SkinConfig::default() };
    let skin = Skin::build(&cfg, mode, SCREEN_W as f32, SCREEN_H as f32);
    let view = PlayfieldView {
        timelines: &model.timelines,
        microtime: FIELD_MICROTIME_US,
        hispeed: FIELD_HISPEED,
        beam_on: &[],
        beam_off: &[],
        constant: false,
        legacy_note: false,
    };
    render_playfield_view(canvas, &skin, &view);
}

#[test]
fn hud_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    hud_screen(&mut canvas);
    check(&canvas, "hud", "GOLDEN_HUD", GOLDEN_HUD);
}

/// A five-key chart laid out on its own terms: the same note width a seven-key chart has, in a
/// narrower field, rather than six lanes stretched across the whole box.
#[test]
fn the_five_key_field_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    draw_field(&mut canvas, Mode::BEAT_5K, true);
    check(&canvas, "five key field", "GOLDEN_FIVE_KEY_FIELD", GOLDEN_FIVE_KEY_FIELD);
}

/// The seven-key field is what the option must not touch, so its golden pins that the layout branch
/// only reaches the modes it names.
#[test]
fn the_seven_key_field_is_unchanged_by_the_five_key_layout() {
    let mut canvas = golden_canvas();
    draw_field(&mut canvas, Mode::BEAT_7K, true);
    check(&canvas, "seven key field", "GOLDEN_SEVEN_KEY_FIELD", GOLDEN_SEVEN_KEY_FIELD);
    assert_eq!(signature_of(|c| draw_field(c, Mode::BEAT_7K, false)), signature_of(|c| draw_field(c, Mode::BEAT_7K, true)));
}

/// Switching the layout on has to move the five-key field, or the row is wired to nothing.
#[test]
fn the_five_key_layout_narrows_the_field_it_is_switched_on_for() {
    let stretched = signature_of(|c| draw_field(c, Mode::BEAT_5K, false));
    let narrow = signature_of(|c| draw_field(c, Mode::BEAT_5K, true));
    assert_ne!(stretched, narrow, "the five-key layout does not reach the field");
    let ten_key_stretched = signature_of(|c| draw_field(c, Mode::BEAT_10K, false));
    let ten_key_narrow = signature_of(|c| draw_field(c, Mode::BEAT_10K, true));
    assert_ne!(ten_key_stretched, ten_key_narrow, "a ten-key chart is two five-key sides and follows the same row");
}

/// The harness only earns its keep if it fails on the regressions it claims to catch, so this pins
/// its sensitivity for this screen: an emptier gauge and a broken combo both have to move it.
#[test]
fn the_signature_detects_a_drained_gauge_and_a_broken_combo() {
    let base = signature_of(hud_screen);
    let drained = signature_of(|canvas| draw_hud(canvas, &HudView { gauge: 12.0, ..hud_view() }));
    assert_ne!(base, drained, "a gauge at 12% instead of 78% changes the signature");
    let broken = signature_of(|canvas| draw_hud(canvas, &HudView { combo: 0, ..hud_view() }));
    assert_ne!(base, broken, "a broken combo changes the signature");
}

/// The two shades are drawn over the field, so each has to move the frame on its own and the two
/// together have to differ from either alone.
#[test]
fn each_lane_shade_moves_the_field_on_its_own() {
    let field = |shade: LaneShade| {
        signature_of(move |canvas| {
            draw_field(canvas, Mode::BEAT_7K, false);
            let skin = Skin::default_for(Mode::BEAT_7K, SCREEN_W as f32, SCREEN_H as f32);
            render_lane_cover(canvas, &skin, shade);
        })
    };
    let bare = field(LaneShade::default());
    let covered = field(LaneShade { cover: 0.3, hidden: 0.0 });
    let hidden = field(LaneShade { cover: 0.0, hidden: 0.3 });
    let both = field(LaneShade { cover: 0.3, hidden: 0.3 });
    assert_ne!(bare, covered, "the cover does not reach the field");
    assert_ne!(bare, hidden, "the hidden band does not reach the field");
    assert_ne!(covered, hidden, "the two shades are drawn at the same end of the field");
    assert_ne!(both, covered, "the hidden band is lost when a cover is up");
}

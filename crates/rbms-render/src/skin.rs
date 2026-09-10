use std::path::{Path, PathBuf};

use rbms_model::{LnKind, Mode};
use serde::Deserialize;
use thiserror::Error;

use crate::{Color, Rect};

/// How many lanes a five-key field is given the width of when it is laid out on its own terms: a
/// seven-key side plus its turntable. A five-key chart then has the note width a seven-key chart
/// has, in a narrower field, instead of six lanes stretched across the same box.
const FIVE_KEY_WIDTH_LANES: usize = 8;

/// Whether a mode is a five-key one, either on its own or as the two sides of a ten-key chart.
fn is_five_key(mode: Mode) -> bool {
    matches!(mode.name, "BEAT_5K" | "BEAT_10K")
}

/// Why a skin file could not be turned into a [`SkinConfig`]. Carries the offending path so the
/// caller can report which skin failed without re-deriving it.
#[derive(Debug, Error)]
pub enum SkinError {
    #[error("cannot read skin file {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid skin definition in {path}")]
    Parse {
        path: PathBuf,
        /// Boxed because a RON error is far larger than the rest of this enum, and every `load`
        /// caller would otherwise pay that size on the success path too.
        #[source]
        source: Box<ron::error::SpannedError>,
    },
}

/// Data-driven skin definition (loadable from a RON file). `field_x`/`field_width` are
/// fractions of the screen width; vertical fields (`top_y`/`judge_y`/`note_height`) and the
/// `bga` rect are pixels in the reference 1280×720 (16:9) space. New skins are authored by
/// editing these fields, not code.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SkinConfig {
    pub field_x: f32,
    pub field_width: f32,
    pub top_y: f32,
    pub judge_y: f32,
    pub note_height: f32,
    pub scratch_left: bool,
    pub lift: f32,
    pub key_color: [u8; 3],
    pub key_color_alt: [u8; 3],
    pub scratch_color: [u8; 3],
    pub mine_color: [u8; 3],
    pub lane_bg: [u8; 3],
    pub lane_bg_alpha: u8,
    pub judge_line: [u8; 3],
    pub beam_color: [u8; 3],
    pub beam_alpha: u8,
    pub beam_height_frac: f32,
    pub outline_color: [u8; 3],
    pub outline_alpha: u8,
    pub divider_color: [u8; 3],
    pub divider_alpha: u8,
    pub bg: [u8; 3],
    pub bga: Option<[f32; 4]>,
    /// Render a 2-player mode (10K/14K) as two separate fields — P1 left, P2 right, turntables on
    /// the outer edges (IIDX DP). `false` keeps the legacy single 16-lane field.
    pub dual_field: bool,
    /// Gap between the two fields in dual layout, as a fraction of screen width.
    pub dual_gap: f32,
    /// Per-judge colours (PG, GR, GD, BD, PR, MS) for the combo flash, counters and result rows.
    pub judge_colors: [[u8; 3]; 6],
    /// Full judge labels shown on the combo flash (e.g. "PERFECT").
    pub judge_labels: [String; 6],
    /// Short judge labels shown in the live counter list (e.g. "PG").
    pub judge_labels_short: [String; 6],
    /// Gauge bar height (px) and the clear / warning percent thresholds that switch its colour.
    pub gauge_height: f32,
    pub gauge_clear_threshold: f32,
    pub gauge_warn_threshold: f32,
    pub gauge_color_clear: [u8; 3],
    pub gauge_color_warn: [u8; 3],
    pub gauge_color_fail: [u8; 3],
    /// HUD text positions as pixels ABOVE the judgment line: combo number, last-judge label, and
    /// the FAST/SLOW tag. Authoring these moves the HUD without touching code.
    pub combo_y_offset: f32,
    pub judge_text_y_offset: f32,
    pub fastslow_y_offset: f32,
    /// Key bomb (note-hit explosion at the judgment line): on/off, base size (px) and lifetime (ms).
    /// Colour comes from `judge_colors` so a skin restyles it without code.
    pub bomb_enabled: bool,
    pub bomb_height: f32,
    pub bomb_duration_ms: u32,
    /// Colours of the two charge-note flavours, which a plain long note does not use. A charge note
    /// has to be released on time and a hell charge note drains while it is let go, so they are told
    /// apart from an ordinary long note by colour rather than by shape.
    pub charge_note_color: [u8; 3],
    pub hell_charge_note_color: [u8; 3],
    /// Opacity of a long-note body, and of a charge note's, which is drawn more solidly because it
    /// is a lane the player has to keep held rather than a tail that resolves itself.
    pub long_note_body_alpha: u8,
    pub charge_note_body_alpha: u8,
    /// Lay a five-key chart out with the note width of a seven-key one rather than stretching its
    /// six lanes across the whole field.
    pub five_key_layout: bool,
}

impl Default for SkinConfig {
    fn default() -> Self {
        SkinConfig {
            field_x: 0.281,
            field_width: 0.281,
            top_y: 60.0,
            judge_y: 620.0,
            note_height: 14.0,
            scratch_left: false,
            lift: 0.0,
            key_color: [235, 235, 235],
            key_color_alt: [70, 130, 230],
            scratch_color: [230, 70, 70],
            mine_color: [70, 220, 120],
            lane_bg: [18, 18, 24],
            lane_bg_alpha: 160,
            judge_line: [200, 40, 40],
            beam_color: [200, 220, 255],
            beam_alpha: 150,
            beam_height_frac: 1.0,
            outline_color: [90, 90, 110],
            outline_alpha: 200,
            divider_color: [60, 60, 75],
            divider_alpha: 150,
            bg: [0, 0, 0],
            bga: Some([905.0, 360.0, 350.0, 255.0]),
            dual_field: true,
            dual_gap: 0.03,
            judge_colors: [[70, 220, 120], [70, 130, 230], [230, 210, 70], [230, 150, 60], [230, 70, 70], [150, 40, 40]],
            judge_labels: ["PERFECT".into(), "GREAT".into(), "GOOD".into(), "BAD".into(), "POOR".into(), "MISS".into()],
            judge_labels_short: ["PG".into(), "GR".into(), "GD".into(), "BD".into(), "PR".into(), "MS".into()],
            gauge_height: 14.0,
            gauge_clear_threshold: 80.0,
            gauge_warn_threshold: 20.0,
            gauge_color_clear: [70, 220, 120],
            gauge_color_warn: [230, 210, 70],
            gauge_color_fail: [230, 70, 70],
            combo_y_offset: 240.0,
            judge_text_y_offset: 180.0,
            fastslow_y_offset: 145.0,
            bomb_enabled: true,
            bomb_height: 56.0,
            bomb_duration_ms: 140,
            charge_note_color: [250, 200, 60],
            hell_charge_note_color: [230, 90, 200],
            long_note_body_alpha: 90,
            charge_note_body_alpha: 140,
            five_key_layout: false,
        }
    }
}

impl SkinConfig {
    /// Read and parse a RON skin definition from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<SkinConfig, SkinError> {
        let path = path.as_ref();
        let src = std::fs::read_to_string(path).map_err(|source| SkinError::Read { path: path.to_path_buf(), source })?;
        ron::from_str(&src).map_err(|source| SkinError::Parse { path: path.to_path_buf(), source: Box::new(source) })
    }
}

fn col(c: [u8; 3]) -> Color {
    Color::rgb(c[0], c[1], c[2])
}

/// A resolved skin: per-lane geometry, colours and regions computed from a `SkinConfig`
/// for a given mode and screen. Replaces the hard-coded lane layout.
pub struct Skin {
    pub x: Vec<f32>,
    pub w: Vec<f32>,
    /// Contiguous note fields as `(x0, width)` — one for SP, two for a dual-layout DP mode. Used to
    /// draw the judgment line, outline and dividers per field (so they don't span the DP gap).
    pub fields: Vec<(f32, f32)>,
    pub top_y: f32,
    pub judge_y: f32,
    pub note_height: f32,
    pub scratch: Vec<bool>,
    pub key_color: Color,
    pub key_color_alt: Color,
    pub scratch_color: Color,
    pub mine_color: Color,
    pub lane_bg: Color,
    pub judge_line: Color,
    pub beam_color: Color,
    pub beam_alpha: u8,
    pub beam_height: f32,
    pub outline: Color,
    pub divider: Color,
    pub bg: Color,
    pub bga: Option<Rect>,
    pub judge_colors: [Color; 6],
    pub judge_labels: [String; 6],
    pub judge_labels_short: [String; 6],
    pub gauge_height: f32,
    pub gauge_clear_threshold: f32,
    pub gauge_warn_threshold: f32,
    pub gauge_color_clear: Color,
    pub gauge_color_warn: Color,
    pub gauge_color_fail: Color,
    pub combo_y_offset: f32,
    pub judge_text_y_offset: f32,
    pub fastslow_y_offset: f32,
    pub bomb_enabled: bool,
    pub bomb_size: f32,
    pub bomb_us: i64,
    pub charge_note_color: Color,
    pub hell_charge_note_color: Color,
    pub long_note_body_alpha: u8,
    pub charge_note_body_alpha: u8,
}

impl Skin {
    pub fn build(cfg: &SkinConfig, mode: Mode, screen_w: f32, _screen_h: f32) -> Skin {
        let n = mode.key;
        let players = (mode.player as usize).max(1);
        let field_x0 = screen_w * cfg.field_x;
        let dual = cfg.dual_field && players >= 2 && n.is_multiple_of(players);

        let mut x = vec![0.0f32; n];
        let mut fields: Vec<(f32, f32)> = Vec::new();
        let lane_w;
        let width_lanes = |lanes: usize| if cfg.five_key_layout && is_five_key(mode) { FIVE_KEY_WIDTH_LANES } else { lanes };
        if dual {
            let per_side = n / players;
            let gap = screen_w * cfg.dual_gap;
            let dual_left = screen_w * 0.03;
            let right_limit = cfg.bga.map(|b| b[0]).unwrap_or(screen_w) - 24.0;
            let fit_side = ((right_limit - dual_left) - gap * (players - 1) as f32) / players as f32;
            let box_w = (screen_w * cfg.field_width).min(fit_side.max(40.0));
            lane_w = box_w / width_lanes(per_side) as f32;
            let side_w = lane_w * per_side as f32;
            let inset = (box_w - side_w) * 0.5;
            for side in 0..players {
                let base = dual_left + inset + side as f32 * (box_w + gap);
                fields.push((base, side_w));
                let lanes: Vec<usize> = (side * per_side..(side + 1) * per_side).collect();
                let scr: Vec<usize> = lanes.iter().copied().filter(|&l| mode.is_scratch(l)).collect();
                let keys: Vec<usize> = lanes.iter().copied().filter(|&l| !mode.is_scratch(l)).collect();
                let outer_left = side == 0;
                let order: Vec<usize> = if outer_left {
                    scr.iter().copied().chain(keys.iter().copied()).collect()
                } else {
                    keys.iter().copied().chain(scr.iter().copied()).collect()
                };
                for (vis, &lane) in order.iter().enumerate() {
                    x[lane] = base + vis as f32 * lane_w;
                }
            }
        } else {
            let box_w = screen_w * cfg.field_width;
            lane_w = box_w / width_lanes(n) as f32;
            let field_w = lane_w * n as f32;
            let field_x0 = field_x0 + (box_w - field_w) * 0.5;
            let scratch_lanes: Vec<usize> = mode.scratch.to_vec();
            let non: Vec<usize> = (0..n).filter(|l| !mode.is_scratch(*l)).collect();
            let order: Vec<usize> = if cfg.scratch_left {
                scratch_lanes.iter().copied().chain(non.iter().copied()).collect()
            } else {
                non.iter().copied().chain(scratch_lanes.iter().copied()).collect()
            };
            for (vis, &lane) in order.iter().enumerate() {
                x[lane] = field_x0 + vis as f32 * lane_w;
            }
            fields.push((field_x0, field_w));
        }
        let w = vec![lane_w; n];

        let judge_y = cfg.judge_y - (cfg.judge_y - cfg.top_y) * cfg.lift.clamp(0.0, 0.9);
        let scratch = (0..n).map(|l| mode.is_scratch(l)).collect();
        let bga = cfg.bga.map(|b| Rect::new(b[0], b[1], b[2], b[3]));
        let beam_height = (judge_y - cfg.top_y) * cfg.beam_height_frac.clamp(0.0, 1.0);

        Skin {
            x,
            w,
            fields,
            top_y: cfg.top_y,
            judge_y,
            note_height: cfg.note_height,
            scratch,
            key_color: col(cfg.key_color),
            key_color_alt: col(cfg.key_color_alt),
            scratch_color: col(cfg.scratch_color),
            mine_color: col(cfg.mine_color),
            lane_bg: Color { r: cfg.lane_bg[0], g: cfg.lane_bg[1], b: cfg.lane_bg[2], a: cfg.lane_bg_alpha },
            judge_line: col(cfg.judge_line),
            beam_color: col(cfg.beam_color),
            beam_alpha: cfg.beam_alpha,
            beam_height,
            outline: Color { r: cfg.outline_color[0], g: cfg.outline_color[1], b: cfg.outline_color[2], a: cfg.outline_alpha },
            divider: Color { r: cfg.divider_color[0], g: cfg.divider_color[1], b: cfg.divider_color[2], a: cfg.divider_alpha },
            bg: col(cfg.bg),
            bga,
            judge_colors: cfg.judge_colors.map(col),
            judge_labels: cfg.judge_labels.clone(),
            judge_labels_short: cfg.judge_labels_short.clone(),
            gauge_height: cfg.gauge_height,
            gauge_clear_threshold: cfg.gauge_clear_threshold,
            gauge_warn_threshold: cfg.gauge_warn_threshold,
            gauge_color_clear: col(cfg.gauge_color_clear),
            gauge_color_warn: col(cfg.gauge_color_warn),
            gauge_color_fail: col(cfg.gauge_color_fail),
            combo_y_offset: cfg.combo_y_offset,
            judge_text_y_offset: cfg.judge_text_y_offset,
            fastslow_y_offset: cfg.fastslow_y_offset,
            bomb_enabled: cfg.bomb_enabled,
            bomb_size: cfg.bomb_height,
            bomb_us: cfg.bomb_duration_ms as i64 * 1000,
            charge_note_color: col(cfg.charge_note_color),
            hell_charge_note_color: col(cfg.hell_charge_note_color),
            long_note_body_alpha: cfg.long_note_body_alpha,
            charge_note_body_alpha: cfg.charge_note_body_alpha,
        }
    }

    pub fn default_for(mode: Mode, screen_w: f32, screen_h: f32) -> Skin {
        Skin::build(&SkinConfig::default(), mode, screen_w, screen_h)
    }

    pub fn lane_count(&self) -> usize {
        self.x.len()
    }

    pub fn lane_height(&self) -> f32 {
        (self.judge_y - self.top_y).abs()
    }

    pub fn lane_center(&self, lane: usize) -> f32 {
        self.x[lane] + self.w[lane] * 0.5
    }

    pub fn note_color(&self, lane: usize) -> Color {
        if self.scratch[lane] {
            self.scratch_color
        } else if lane.is_multiple_of(2) {
            self.key_color
        } else {
            self.key_color_alt
        }
    }

    /// Colour of a long note of `flavour` in `lane`.
    ///
    /// A charge note and a hell charge note carry obligations a plain long note does not — the first
    /// has to be let go on time, the second drains the gauge for as long as it is not held — so each
    /// gets its own colour instead of the lane's. An undefined flavour is one the LN MODE has yet to
    /// resolve, and is drawn as the plain note it will become if nothing resolves it.
    pub fn long_note_color(&self, lane: usize, flavour: LnKind) -> Color {
        match flavour {
            LnKind::Cn => self.charge_note_color,
            LnKind::Hcn => self.hell_charge_note_color,
            LnKind::Ln | LnKind::Undefined => self.note_color(lane),
        }
    }

    /// Opacity a long note's body is drawn at, which is higher for the two charge flavours because
    /// they are lanes the player has to keep held rather than tails that resolve on their own.
    pub fn long_note_alpha(&self, flavour: LnKind) -> u8 {
        match flavour {
            LnKind::Cn | LnKind::Hcn => self.charge_note_body_alpha,
            LnKind::Ln | LnKind::Undefined => self.long_note_body_alpha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_partial_fills_defaults() {
        let cfg: SkinConfig = ron::from_str("(field_width: 0.6, lift: 0.3, scratch_left: true)").unwrap();
        assert_eq!(cfg.field_width, 0.6);
        assert_eq!(cfg.lift, 0.3);
        assert!(cfg.scratch_left);
        assert_eq!(cfg.field_x, 0.281);
        assert_eq!(cfg.note_height, 14.0);
    }

    #[test]
    fn scratch_left_reorders_lanes() {
        let cfg = SkinConfig { scratch_left: true, ..SkinConfig::default() };
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 480.0, 640.0);
        let min_x = skin.x.iter().copied().fold(f32::MAX, f32::min);
        assert_eq!(skin.x[7], min_x, "scratch lane 7 should be leftmost");
    }

    #[test]
    fn sp_mode_has_one_field() {
        let skin = Skin::build(&SkinConfig::default(), Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.fields.len(), 1, "SP renders a single field");
    }

    #[test]
    fn dual_field_14k_splits_p1_left_p2_right_with_outer_scratches() {
        let skin = Skin::build(&SkinConfig::default(), Mode::BEAT_14K, 1280.0, 720.0);
        assert_eq!(skin.fields.len(), 2, "DP renders two fields");
        let p1_right = (0..8).map(|l| skin.x[l] + skin.w[l]).fold(f32::MIN, f32::max);
        let p2_left = (8..16).map(|l| skin.x[l]).fold(f32::MAX, f32::min);
        assert!(p1_right <= p2_left, "P1 field is entirely left of P2 (gap between)");
        let p1_min = (0..8).map(|l| skin.x[l]).fold(f32::MAX, f32::min);
        assert_eq!(skin.x[7], p1_min, "P1 turntable (lane 7) is on the outer-left edge");
        let all_max = (0..16).map(|l| skin.x[l]).fold(f32::MIN, f32::max);
        assert_eq!(skin.x[15], all_max, "P2 turntable (lane 15) is on the outer-right edge");
    }

    #[test]
    fn skin_ron_overrides_hud_palette_and_offsets() {
        let cfg: SkinConfig =
            ron::from_str(r#"(judge_labels: ("PG!","GR!","GD!","BD!","PR!","MS!"), combo_y_offset: 300.0, gauge_clear_threshold: 85.0)"#).unwrap();
        assert_eq!(cfg.judge_labels[0], "PG!");
        assert_eq!(cfg.combo_y_offset, 300.0);
        assert_eq!(cfg.gauge_clear_threshold, 85.0);
        assert_eq!(cfg.judge_labels_short[0], "PG", "unspecified HUD fields keep defaults");
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.judge_labels[0], "PG!");
        assert_eq!(skin.gauge_clear_threshold, 85.0);
    }

    #[test]
    fn dual_field_clears_the_bga_column() {
        for cfg in [SkinConfig::default(), ron::from_str("(field_width: 0.42, bga: Some((900.0,360.0,340.0,255.0)))").unwrap()] {
            let skin = Skin::build(&cfg, Mode::BEAT_14K, 1280.0, 720.0);
            let field_right = (0..16).map(|l| skin.x[l] + skin.w[l]).fold(f32::MIN, f32::max);
            let bga_left = skin.bga.map(|b| b.x).unwrap_or(1280.0);
            assert!(field_right <= bga_left, "DP fields (right edge {field_right}) must clear the BGA (left {bga_left})");
        }
    }

    #[test]
    fn dual_field_can_be_disabled() {
        let cfg = SkinConfig { dual_field: false, ..SkinConfig::default() };
        let skin = Skin::build(&cfg, Mode::BEAT_14K, 1280.0, 720.0);
        assert_eq!(skin.fields.len(), 1, "single-field 14K when dual_field is off");
    }

    #[test]
    fn lane_count_equals_mode_key() {
        assert_eq!(Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0).lane_count(), 8);
        assert_eq!(Skin::default_for(Mode::BEAT_5K, 1280.0, 720.0).lane_count(), 6);
        assert_eq!(Skin::default_for(Mode::BEAT_14K, 1280.0, 720.0).lane_count(), 16);
        assert_eq!(Skin::default_for(Mode::POPN_9K, 1280.0, 720.0).lane_count(), 9);
    }

    #[test]
    fn lift_zero_keeps_judge_line_at_configured_y() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.judge_y, 620.0, "lift 0 leaves judge_y unchanged");
        assert_eq!(skin.top_y, 60.0);
    }

    #[test]
    fn lift_raises_judge_line_toward_top() {
        let cfg = SkinConfig { lift: 0.5, ..SkinConfig::default() };
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.judge_y, 340.0);
        assert!(skin.judge_y < 620.0, "lift moves the judge line up");
    }

    #[test]
    fn lift_clamps_above_point_nine() {
        let cfg = SkinConfig { lift: 5.0, ..SkinConfig::default() };
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert!((skin.judge_y - 116.0).abs() < 1e-3, "lift clamps to 0.9 (got {})", skin.judge_y);
        assert!(skin.judge_y > skin.top_y, "judge line never crosses above the top edge");
    }

    #[test]
    fn lift_clamps_negative_to_zero() {
        let cfg = SkinConfig { lift: -3.0, ..SkinConfig::default() };
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.judge_y, 620.0, "negative lift clamps to 0 (no change)");
    }

    #[test]
    fn lane_height_is_absolute_distance_judge_to_top() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.lane_height(), 560.0);
        assert_eq!(skin.lane_height(), (skin.judge_y - skin.top_y).abs());
    }

    #[test]
    fn note_color_scratch_keys_alternate() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.note_color(7), skin.scratch_color, "scratch lane uses scratch colour");
        assert_eq!(skin.note_color(0), skin.key_color, "even key lane uses key colour");
        assert_eq!(skin.note_color(2), skin.key_color, "even key lane uses key colour");
        assert_eq!(skin.note_color(1), skin.key_color_alt, "odd key lane uses alt colour");
        assert_eq!(skin.note_color(3), skin.key_color_alt, "odd key lane uses alt colour");
    }

    #[test]
    fn lane_center_is_x_plus_half_width() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        for lane in 0..skin.lane_count() {
            assert_eq!(skin.lane_center(lane), skin.x[lane] + skin.w[lane] * 0.5);
        }
    }

    #[test]
    fn all_lanes_have_uniform_width() {
        let skin = Skin::default_for(Mode::BEAT_14K, 1280.0, 720.0);
        let w0 = skin.w[0];
        assert!(skin.w.iter().all(|&w| (w - w0).abs() < 1e-4), "all lanes are equal width");
        assert!(w0 > 0.0, "lane width is positive");
    }

    #[test]
    fn beam_height_is_field_height_when_full_fraction() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        assert!((skin.beam_height - (skin.judge_y - skin.top_y)).abs() < 1e-3);
    }

    #[test]
    fn beam_height_frac_clamps_to_unit_range() {
        let mut cfg = SkinConfig { beam_height_frac: 4.0, ..SkinConfig::default() };
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert!((skin.beam_height - (skin.judge_y - skin.top_y)).abs() < 1e-3);

        cfg.beam_height_frac = -1.0;
        let skin2 = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin2.beam_height, 0.0, "negative frac clamps to 0");
    }

    #[test]
    fn lane_bg_carries_config_alpha() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.lane_bg.a, 160, "lane_bg keeps its configured alpha (not forced opaque)");
        assert_eq!(skin.outline.a, 200);
        assert_eq!(skin.divider.a, 150);
    }

    #[test]
    fn sp_lanes_left_to_right_in_visual_order_no_scratch_left() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        for lane in 0..6 {
            assert!(skin.x[lane] < skin.x[lane + 1], "key lane {lane} left of {}", lane + 1);
        }
        let max_x = skin.x.iter().copied().fold(f32::MIN, f32::max);
        assert_eq!(skin.x[7], max_x, "scratch lane 7 is rightmost when scratch_left=false");
    }

    #[test]
    fn dual_field_10k_splits_two_fields_with_outer_scratches() {
        let skin = Skin::build(&SkinConfig::default(), Mode::BEAT_10K, 1280.0, 720.0);
        assert_eq!(skin.fields.len(), 2, "10K DP renders two fields");
        let p1_right = (0..6).map(|l| skin.x[l] + skin.w[l]).fold(f32::MIN, f32::max);
        let p2_left = (6..12).map(|l| skin.x[l]).fold(f32::MAX, f32::min);
        assert!(p1_right <= p2_left, "P1 field entirely left of P2");
        let p1_min = (0..6).map(|l| skin.x[l]).fold(f32::MAX, f32::min);
        assert_eq!(skin.x[5], p1_min, "P1 scratch (lane 5) is on the outer-left edge");
        let all_max = (0..12).map(|l| skin.x[l]).fold(f32::MIN, f32::max);
        assert_eq!(skin.x[11], all_max, "P2 scratch (lane 11) is on the outer-right edge");
    }

    #[test]
    fn popn_single_player_is_never_dual() {
        let skin = Skin::build(&SkinConfig::default(), Mode::POPN_9K, 1280.0, 720.0);
        assert_eq!(skin.fields.len(), 1, "single-player POPN is one field");
        assert!(skin.scratch.iter().all(|&s| !s), "POPN has no scratch lanes");
    }

    #[test]
    fn scratch_flags_match_mode() {
        let skin = Skin::default_for(Mode::BEAT_14K, 1280.0, 720.0);
        for lane in 0..16 {
            assert_eq!(skin.scratch[lane], lane == 7 || lane == 15, "scratch flag for lane {lane}");
        }
    }

    #[test]
    fn bomb_us_is_duration_ms_times_1000() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.bomb_us, 140_000, "bomb_duration_ms 140 -> 140000 us");
        assert!(skin.bomb_enabled);
        assert_eq!(skin.bomb_size, 56.0);
    }

    #[test]
    fn bga_rect_mirrors_config_tuple() {
        let skin = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        let bga = skin.bga.expect("default config has a BGA rect");
        assert_eq!((bga.x, bga.y, bga.w, bga.h), (905.0, 360.0, 350.0, 255.0));
    }

    #[test]
    fn bga_none_when_config_omits_it() {
        let cfg: SkinConfig = ron::from_str("(bga: None)").unwrap();
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert!(skin.bga.is_none(), "no BGA rect when config says None");
    }

    #[test]
    fn field_width_scales_with_screen_width_in_sp() {
        let wide = Skin::default_for(Mode::BEAT_7K, 2560.0, 720.0);
        let narrow = Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0);
        assert!(wide.w[0] > narrow.w[0], "wider screen yields wider SP lanes");
    }

    #[test]
    fn loading_a_missing_skin_file_reports_the_path() {
        let missing = std::env::temp_dir().join("rbms-render-no-such-skin.ron");
        let err = SkinConfig::load(&missing).expect_err("a missing file cannot load");
        assert!(matches!(err, SkinError::Read { .. }), "an unreadable path is a Read error, got {err:?}");
        assert!(err.to_string().contains("rbms-render-no-such-skin.ron"), "the message names the file: {err}");
        assert!(std::error::Error::source(&err).is_some(), "the io error is kept as the source");
    }

    #[test]
    fn loading_malformed_ron_reports_a_parse_error() {
        let path = std::env::temp_dir().join("rbms-render-broken-skin.ron");
        std::fs::write(&path, "(field_width: this is not ron").expect("write fixture");
        let err = SkinConfig::load(&path).expect_err("malformed RON cannot load");
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, SkinError::Parse { .. }), "bad syntax is a Parse error, got {err:?}");
        assert!(err.to_string().contains("rbms-render-broken-skin.ron"), "the message names the file: {err}");
        assert!(std::error::Error::source(&err).is_some(), "the RON error is kept as the source");
    }

    /// A five-key chart laid out on its own terms gets the note width a seven-key one has, so the
    /// lanes stop being fat and the field is narrower instead.
    #[test]
    fn the_five_key_layout_gives_a_five_key_chart_seven_key_lane_width() {
        let stretched = Skin::build(&SkinConfig::default(), Mode::BEAT_5K, 1280.0, 720.0);
        let cfg = SkinConfig { five_key_layout: true, ..SkinConfig::default() };
        let narrow = Skin::build(&cfg, Mode::BEAT_5K, 1280.0, 720.0);
        let seven = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert!(narrow.w[0] < stretched.w[0], "the lanes stop being stretched: {} against {}", narrow.w[0], stretched.w[0]);
        assert!((narrow.w[0] - seven.w[0]).abs() < 1e-3, "and take the width a seven-key lane has");
        let (_, narrow_w) = narrow.fields[0];
        let (_, stretched_w) = stretched.fields[0];
        assert!(narrow_w < stretched_w, "so the field itself is narrower");
    }

    /// The narrower field stays where the wider one was centred, so switching the row does not slide
    /// the chart across the screen.
    #[test]
    fn the_five_key_layout_keeps_the_field_centred_where_it_was() {
        let stretched = Skin::build(&SkinConfig::default(), Mode::BEAT_5K, 1280.0, 720.0);
        let cfg = SkinConfig { five_key_layout: true, ..SkinConfig::default() };
        let narrow = Skin::build(&cfg, Mode::BEAT_5K, 1280.0, 720.0);
        let centre = |skin: &Skin| skin.fields[0].0 + skin.fields[0].1 * 0.5;
        assert!((centre(&narrow) - centre(&stretched)).abs() < 1e-3, "{} against {}", centre(&narrow), centre(&stretched));
    }

    /// A ten-key chart is two five-key sides, so it follows the same row and each side keeps its
    /// outer turntable.
    #[test]
    fn the_five_key_layout_narrows_both_sides_of_a_ten_key_chart() {
        let cfg = SkinConfig { five_key_layout: true, ..SkinConfig::default() };
        let narrow = Skin::build(&cfg, Mode::BEAT_10K, 1280.0, 720.0);
        let stretched = Skin::build(&SkinConfig::default(), Mode::BEAT_10K, 1280.0, 720.0);
        assert_eq!(narrow.fields.len(), 2, "ten-key stays a two-field layout");
        assert!(narrow.fields[0].1 < stretched.fields[0].1, "each side is narrower");
        let p1_right = (0..6).map(|l| narrow.x[l] + narrow.w[l]).fold(f32::MIN, f32::max);
        let p2_left = (6..12).map(|l| narrow.x[l]).fold(f32::MAX, f32::min);
        assert!(p1_right <= p2_left, "the two sides still do not overlap");
        assert_eq!(narrow.x[5], (0..6).map(|l| narrow.x[l]).fold(f32::MAX, f32::min), "P1 turntable stays on the outer edge");
    }

    /// The row names five-key modes, so nothing else may move under it.
    #[test]
    fn the_five_key_layout_leaves_every_other_mode_alone() {
        let cfg = SkinConfig { five_key_layout: true, ..SkinConfig::default() };
        for mode in [Mode::BEAT_7K, Mode::BEAT_14K, Mode::POPN_9K] {
            let on = Skin::build(&cfg, mode, 1280.0, 720.0);
            let off = Skin::build(&SkinConfig::default(), mode, 1280.0, 720.0);
            assert_eq!(on.x, off.x, "{} moved", mode.name);
            assert_eq!(on.w, off.w, "{} changed width", mode.name);
        }
    }

    /// The charge flavours are skin data, so a RON that names them is what decides the colours.
    #[test]
    fn a_skin_ron_can_restyle_the_charge_note_flavours() {
        let cfg: SkinConfig = ron::from_str("(charge_note_color: (1,2,3), hell_charge_note_color: (4,5,6), charge_note_body_alpha: 200)").unwrap();
        let skin = Skin::build(&cfg, Mode::BEAT_7K, 1280.0, 720.0);
        assert_eq!(skin.long_note_color(0, LnKind::Cn), Color::rgb(1, 2, 3));
        assert_eq!(skin.long_note_color(0, LnKind::Hcn), Color::rgb(4, 5, 6));
        assert_eq!(skin.long_note_alpha(LnKind::Hcn), 200);
        assert_eq!(skin.long_note_alpha(LnKind::Ln), SkinConfig::default().long_note_body_alpha, "an unnamed field keeps its default");
    }

    /// Every field this branch added has to fall back on its own, so a skin written before they
    /// existed keeps loading byte for byte.
    #[test]
    fn a_skin_written_before_these_fields_existed_still_loads() {
        let cfg: SkinConfig = ron::from_str("(field_width: 0.6)").unwrap();
        let default = SkinConfig::default();
        assert_eq!(cfg.charge_note_color, default.charge_note_color);
        assert_eq!(cfg.hell_charge_note_color, default.hell_charge_note_color);
        assert_eq!(cfg.long_note_body_alpha, default.long_note_body_alpha);
        assert_eq!(cfg.charge_note_body_alpha, default.charge_note_body_alpha);
        assert!(!cfg.five_key_layout, "the layout is off until a skin or the settings ask for it");
    }

    #[test]
    fn loading_a_valid_skin_file_fills_the_unset_fields_with_defaults() {
        let path = std::env::temp_dir().join("rbms-render-valid-skin.ron");
        std::fs::write(&path, "(field_width: 0.5, lift: 0.25)").expect("write fixture");
        let cfg = SkinConfig::load(&path).expect("a well-formed skin loads");
        std::fs::remove_file(&path).ok();
        assert_eq!(cfg.field_width, 0.5);
        assert_eq!(cfg.lift, 0.25);
        assert_eq!(cfg.note_height, SkinConfig::default().note_height, "unset fields keep their defaults");
    }
}

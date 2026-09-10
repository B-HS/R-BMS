//! The graph and visualiser objects a document declares, split out of [`crate::model`] because
//! their palettes make them the longest records in the mirror.
//!
//! Every colour is kept as the document wrote it, an `RRGGBB` or `RRGGBBAA` hex string, and is
//! parsed by the renderer rather than here: the mirror's job is to survive round-tripping a
//! third-party document, not to validate it.

use serde::Deserialize;

/// The gauge history graph and the palette it draws each clear band with.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GaugeGraph {
    pub id: String,
    pub color: Vec<String>,
    #[serde(rename = "assistClearBGColor")]
    pub assist_clear_bg_color: String,
    #[serde(rename = "assistAndEasyFailBGColor")]
    pub assist_and_easy_fail_bg_color: String,
    #[serde(rename = "grooveFailBGColor")]
    pub groove_fail_bg_color: String,
    #[serde(rename = "grooveClearAndHardBGColor")]
    pub groove_clear_and_hard_bg_color: String,
    #[serde(rename = "exHardBGColor")]
    pub ex_hard_bg_color: String,
    #[serde(rename = "hazardBGColor")]
    pub hazard_bg_color: String,
    #[serde(rename = "assistClearLineColor")]
    pub assist_clear_line_color: String,
    #[serde(rename = "assistAndEasyFailLineColor")]
    pub assist_and_easy_fail_line_color: String,
    #[serde(rename = "grooveFailLineColor")]
    pub groove_fail_line_color: String,
    #[serde(rename = "grooveClearAndHardLineColor")]
    pub groove_clear_and_hard_line_color: String,
    #[serde(rename = "exHardLineColor")]
    pub ex_hard_line_color: String,
    #[serde(rename = "hazardLineColor")]
    pub hazard_line_color: String,
    #[serde(rename = "borderlineColor")]
    pub borderline_color: String,
    #[serde(rename = "borderColor")]
    pub border_color: String,
}

impl Default for GaugeGraph {
    fn default() -> Self {
        Self {
            id: String::new(),
            color: Vec::new(),
            assist_clear_bg_color: "440044".to_owned(),
            assist_and_easy_fail_bg_color: "004444".to_owned(),
            groove_fail_bg_color: "004400".to_owned(),
            groove_clear_and_hard_bg_color: "440000".to_owned(),
            ex_hard_bg_color: "444400".to_owned(),
            hazard_bg_color: "444444".to_owned(),
            assist_clear_line_color: "ff00ff".to_owned(),
            assist_and_easy_fail_line_color: "00ffff".to_owned(),
            groove_fail_line_color: "00ff00".to_owned(),
            groove_clear_and_hard_line_color: "ff0000".to_owned(),
            ex_hard_line_color: "ffff00".to_owned(),
            hazard_line_color: "cccccc".to_owned(),
            borderline_color: "ff0000".to_owned(),
            border_color: "440000".to_owned(),
        }
    }
}

/// Milliseconds a judge graph holds a bar before it fades.
const JUDGE_GRAPH_DELAY_MS: i32 = 500;

/// The per-judgement bar graph on the result screen.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct JudgeGraph {
    pub id: String,
    #[serde(rename = "type")]
    pub graph_type: i32,
    #[serde(rename = "backTexOff")]
    pub back_tex_off: i32,
    pub delay: i32,
    #[serde(rename = "orderReverse")]
    pub order_reverse: i32,
    #[serde(rename = "noGap")]
    pub no_gap: i32,
    #[serde(rename = "noGapX")]
    pub no_gap_x: i32,
}

impl Default for JudgeGraph {
    fn default() -> Self {
        Self { id: String::new(), graph_type: 0, back_tex_off: 0, delay: JUDGE_GRAPH_DELAY_MS, order_reverse: 0, no_gap: 0, no_gap_x: 0 }
    }
}

/// Pixels wide a graph line is drawn by default.
const DEFAULT_LINE_WIDTH: i32 = 1;

/// The tempo timeline drawn along a chart's length.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BpmGraph {
    pub id: String,
    pub delay: i32,
    #[serde(rename = "lineWidth")]
    pub line_width: i32,
    #[serde(rename = "mainBPMColor")]
    pub main_bpm_color: String,
    #[serde(rename = "minBPMColor")]
    pub min_bpm_color: String,
    #[serde(rename = "maxBPMColor")]
    pub max_bpm_color: String,
    #[serde(rename = "otherBPMColor")]
    pub other_bpm_color: String,
    #[serde(rename = "stopLineColor")]
    pub stop_line_color: String,
    #[serde(rename = "transitionLineColor")]
    pub transition_line_color: String,
}

impl Default for BpmGraph {
    fn default() -> Self {
        Self {
            id: String::new(),
            delay: 0,
            line_width: 2,
            main_bpm_color: "00ff00".to_owned(),
            min_bpm_color: "0000ff".to_owned(),
            max_bpm_color: "ff0000".to_owned(),
            other_bpm_color: "ffff00".to_owned(),
            stop_line_color: "ff00ff".to_owned(),
            transition_line_color: "7f7f7f".to_owned(),
        }
    }
}

/// Pixels wide the hit-error and timing visualisers are by default.
const VISUALIZER_WIDTH: i32 = 301;

/// Milliseconds either side of the judge line those visualisers span.
const VISUALIZER_JUDGE_WIDTH_MS: i32 = 150;

/// The running hit-error strip: where recent notes landed against the judge line.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HitErrorVisualizer {
    pub id: String,
    pub width: i32,
    #[serde(rename = "judgeWidthMillis")]
    pub judge_width_millis: i32,
    #[serde(rename = "lineWidth")]
    pub line_width: i32,
    #[serde(rename = "colorMode")]
    pub color_mode: i32,
    #[serde(rename = "hiterrorMode")]
    pub hiterror_mode: i32,
    #[serde(rename = "emaMode")]
    pub ema_mode: i32,
    #[serde(rename = "lineColor")]
    pub line_color: String,
    #[serde(rename = "centerColor")]
    pub center_color: String,
    #[serde(rename = "PGColor")]
    pub pgreat_color: String,
    #[serde(rename = "GRColor")]
    pub great_color: String,
    #[serde(rename = "GDColor")]
    pub good_color: String,
    #[serde(rename = "BDColor")]
    pub bad_color: String,
    #[serde(rename = "PRColor")]
    pub poor_color: String,
    #[serde(rename = "emaColor")]
    pub ema_color: String,
    pub alpha: f32,
    #[serde(rename = "windowLength")]
    pub window_length: i32,
    pub transparent: i32,
    #[serde(rename = "drawDecay")]
    pub draw_decay: i32,
}

impl Default for HitErrorVisualizer {
    fn default() -> Self {
        Self {
            id: String::new(),
            width: VISUALIZER_WIDTH,
            judge_width_millis: VISUALIZER_JUDGE_WIDTH_MS,
            line_width: DEFAULT_LINE_WIDTH,
            color_mode: 1,
            hiterror_mode: 1,
            ema_mode: 1,
            line_color: "99CCFF80".to_owned(),
            center_color: "FFFFFFFF".to_owned(),
            pgreat_color: "99CCFF80".to_owned(),
            great_color: "F2CB3080".to_owned(),
            good_color: "14CC8f80".to_owned(),
            bad_color: "FF1AB380".to_owned(),
            poor_color: "CC292980".to_owned(),
            ema_color: "FF0000FF".to_owned(),
            alpha: 0.1,
            window_length: 30,
            transparent: 0,
            draw_decay: 1,
        }
    }
}

/// The judge-window ruler drawn behind the hit-error strip.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TimingVisualizer {
    pub id: String,
    pub width: i32,
    #[serde(rename = "judgeWidthMillis")]
    pub judge_width_millis: i32,
    #[serde(rename = "lineWidth")]
    pub line_width: i32,
    #[serde(rename = "lineColor")]
    pub line_color: String,
    #[serde(rename = "centerColor")]
    pub center_color: String,
    #[serde(rename = "PGColor")]
    pub pgreat_color: String,
    #[serde(rename = "GRColor")]
    pub great_color: String,
    #[serde(rename = "GDColor")]
    pub good_color: String,
    #[serde(rename = "BDColor")]
    pub bad_color: String,
    #[serde(rename = "PRColor")]
    pub poor_color: String,
    pub transparent: i32,
    #[serde(rename = "drawDecay")]
    pub draw_decay: i32,
}

impl Default for TimingVisualizer {
    fn default() -> Self {
        Self {
            id: String::new(),
            width: VISUALIZER_WIDTH,
            judge_width_millis: VISUALIZER_JUDGE_WIDTH_MS,
            line_width: DEFAULT_LINE_WIDTH,
            line_color: "00FF00FF".to_owned(),
            center_color: "FFFFFFFF".to_owned(),
            pgreat_color: "000088FF".to_owned(),
            great_color: "008800FF".to_owned(),
            good_color: "888800FF".to_owned(),
            bad_color: "880000FF".to_owned(),
            poor_color: "000000FF".to_owned(),
            transparent: 0,
            draw_decay: 1,
        }
    }
}

/// The histogram of a session's timing spread, with its mean and deviation markers.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TimingDistributionGraph {
    pub id: String,
    pub width: i32,
    #[serde(rename = "lineWidth")]
    pub line_width: i32,
    #[serde(rename = "graphColor")]
    pub graph_color: String,
    #[serde(rename = "averageColor")]
    pub average_color: String,
    #[serde(rename = "devColor")]
    pub dev_color: String,
    #[serde(rename = "PGColor")]
    pub pgreat_color: String,
    #[serde(rename = "GRColor")]
    pub great_color: String,
    #[serde(rename = "GDColor")]
    pub good_color: String,
    #[serde(rename = "BDColor")]
    pub bad_color: String,
    #[serde(rename = "PRColor")]
    pub poor_color: String,
    #[serde(rename = "drawAverage")]
    pub draw_average: i32,
    #[serde(rename = "drawDev")]
    pub draw_dev: i32,
}

impl Default for TimingDistributionGraph {
    fn default() -> Self {
        Self {
            id: String::new(),
            width: VISUALIZER_WIDTH,
            line_width: DEFAULT_LINE_WIDTH,
            graph_color: "00FF00FF".to_owned(),
            average_color: "FFFFFFFF".to_owned(),
            dev_color: "FFFFFFFF".to_owned(),
            pgreat_color: "000088FF".to_owned(),
            great_color: "008800FF".to_owned(),
            good_color: "888800FF".to_owned(),
            bad_color: "880000FF".to_owned(),
            poor_color: "000000FF".to_owned(),
            draw_average: 1,
            draw_dev: 1,
        }
    }
}

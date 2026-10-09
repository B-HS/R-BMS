//! The drawable objects a skin document declares: images, numbers, text, sliders, graphs, notes,
//! covers and the panes each screen owns.
//!
//! Split out of [`crate::model`] to keep one file per concern; the shape and defaults are the
//! reference implementation's `JsonSkin.java` verbatim, as the parent module's notes explain.

use serde::Deserialize;

use super::{Animation, Destination, PropertyRef, SINGLE_DIVISION};

/// A still or animated image cut from a [`Source`].
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ImageDef {
    pub id: String,
    pub src: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
    pub timer: Option<PropertyRef>,
    pub cycle: i32,
    pub len: i32,
    #[serde(rename = "ref")]
    pub reference: i32,
    pub act: Option<PropertyRef>,
    pub click: i32,
}

impl Default for ImageDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            divx: SINGLE_DIVISION,
            divy: SINGLE_DIVISION,
            timer: None,
            cycle: 0,
            len: 0,
            reference: 0,
            act: None,
            click: 0,
        }
    }
}

/// A set of images one of which is shown, chosen by an integer property.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ImageSet {
    pub id: String,
    #[serde(rename = "ref")]
    pub reference: i32,
    pub value: Option<PropertyRef>,
    pub images: Vec<String>,
    pub act: Option<PropertyRef>,
    pub click: i32,
}

/// A number drawn from a digit strip.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ValueDef {
    pub id: String,
    pub src: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
    pub timer: Option<PropertyRef>,
    pub cycle: i32,
    pub align: i32,
    pub digit: i32,
    pub padding: i32,
    pub zeropadding: i32,
    pub space: i32,
    #[serde(rename = "ref")]
    pub reference: i32,
    pub value: Option<PropertyRef>,
    pub offset: Vec<ValueDef>,
}

impl Default for ValueDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            divx: SINGLE_DIVISION,
            divy: SINGLE_DIVISION,
            timer: None,
            cycle: 0,
            align: 0,
            digit: 0,
            padding: 0,
            zeropadding: 0,
            space: 0,
            reference: 0,
            value: None,
            offset: Vec::new(),
        }
    }
}

/// A fractional number drawn from a digit strip, with its own integer and fraction widths.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FloatValueDef {
    pub id: String,
    pub src: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
    pub timer: Option<PropertyRef>,
    pub cycle: i32,
    pub align: i32,
    pub fketa: i32,
    pub iketa: i32,
    pub gain: f32,
    #[serde(rename = "isSignvisible")]
    pub is_sign_visible: bool,
    pub padding: i32,
    pub zeropadding: i32,
    pub space: i32,
    #[serde(rename = "ref")]
    pub reference: i32,
    pub value: Option<PropertyRef>,
    pub offset: Vec<ValueDef>,
}

impl Default for FloatValueDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            divx: SINGLE_DIVISION,
            divy: SINGLE_DIVISION,
            timer: None,
            cycle: 0,
            align: 0,
            fketa: 0,
            iketa: 0,
            gain: 1.0,
            is_sign_visible: false,
            padding: 0,
            zeropadding: 0,
            space: 0,
            reference: 0,
            value: None,
            offset: Vec::new(),
        }
    }
}

/// A run of text drawn with a [`FontDef`].
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TextDef {
    pub id: String,
    pub font: String,
    pub size: i32,
    pub align: i32,
    #[serde(rename = "ref")]
    pub reference: i32,
    pub value: Option<PropertyRef>,
    pub event: Option<PropertyRef>,
    #[serde(rename = "constantText")]
    pub constant_text: Option<String>,
    pub editable: bool,
    pub wrapping: bool,
    pub overflow: i32,
    #[serde(rename = "outlineColor")]
    pub outline_color: String,
    #[serde(rename = "outlineWidth")]
    pub outline_width: f32,
    #[serde(rename = "shadowColor")]
    pub shadow_color: String,
    #[serde(rename = "shadowOffsetX")]
    pub shadow_offset_x: f32,
    #[serde(rename = "shadowOffsetY")]
    pub shadow_offset_y: f32,
    #[serde(rename = "shadowSmoothness")]
    pub shadow_smoothness: f32,
}

impl Default for TextDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            font: String::new(),
            size: 0,
            align: 0,
            reference: 0,
            value: None,
            event: None,
            constant_text: None,
            editable: false,
            wrapping: false,
            overflow: 0,
            outline_color: "ffffff00".to_owned(),
            outline_width: 0.0,
            shadow_color: "ffffff00".to_owned(),
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            shadow_smoothness: 0.0,
        }
    }
}

/// A draggable slider bound to a normalised property.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SliderDef {
    pub id: String,
    pub src: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
    pub timer: Option<PropertyRef>,
    pub cycle: i32,
    pub angle: i32,
    pub range: i32,
    #[serde(rename = "type")]
    pub slider_type: i32,
    pub changeable: bool,
    pub value: Option<PropertyRef>,
    pub event: Option<PropertyRef>,
    #[serde(rename = "isRefNum")]
    pub is_ref_num: bool,
    pub min: i32,
    pub max: i32,
}

impl Default for SliderDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            divx: SINGLE_DIVISION,
            divy: SINGLE_DIVISION,
            timer: None,
            cycle: 0,
            angle: 0,
            range: 0,
            slider_type: 0,
            changeable: true,
            value: None,
            event: None,
            is_ref_num: false,
            min: 0,
            max: 0,
        }
    }
}

/// A bar graph bound to a normalised property.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GraphDef {
    pub id: String,
    pub src: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
    pub timer: Option<PropertyRef>,
    pub cycle: i32,
    pub angle: i32,
    #[serde(rename = "type")]
    pub graph_type: i32,
    pub value: Option<PropertyRef>,
    #[serde(rename = "isRefNum")]
    pub is_ref_num: bool,
    pub min: i32,
    pub max: i32,
}

impl Default for GraphDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            divx: SINGLE_DIVISION,
            divy: SINGLE_DIVISION,
            timer: None,
            cycle: 0,
            angle: SINGLE_DIVISION,
            graph_type: 0,
            value: None,
            is_ref_num: false,
            min: 0,
            max: 0,
        }
    }
}

/// Percent of the authored size a note keeps in each axis when the lane is not resized.
const FULL_EXPANSION_PERCENT: i32 = 100;

/// The note images and their shared destination, one entry per lane.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NoteSet {
    pub id: String,
    pub note: Vec<String>,
    pub lnstart: Vec<String>,
    pub lnend: Vec<String>,
    pub lnbody: Vec<String>,
    #[serde(rename = "lnbodyActive")]
    pub lnbody_active: Vec<String>,
    pub lnactive: Vec<String>,
    pub hcnstart: Vec<String>,
    pub hcnend: Vec<String>,
    pub hcnbody: Vec<String>,
    pub hcnactive: Vec<String>,
    #[serde(rename = "hcnbodyActive")]
    pub hcnbody_active: Vec<String>,
    pub hcndamage: Vec<String>,
    #[serde(rename = "hcnbodyMiss")]
    pub hcnbody_miss: Vec<String>,
    pub hcnreactive: Vec<String>,
    #[serde(rename = "hcnbodyReactive")]
    pub hcnbody_reactive: Vec<String>,
    pub mine: Vec<String>,
    pub hidden: Vec<String>,
    pub processed: Vec<String>,
    pub dst: Vec<Animation>,
    pub dst2: Option<i32>,
    pub expansionrate: Vec<i32>,
    pub size: Vec<f32>,
    pub group: Vec<Destination>,
    pub bpm: Vec<Destination>,
    pub stop: Vec<Destination>,
    pub time: Vec<Destination>,
}

impl Default for NoteSet {
    fn default() -> Self {
        Self {
            id: String::new(),
            note: Vec::new(),
            lnstart: Vec::new(),
            lnend: Vec::new(),
            lnbody: Vec::new(),
            lnbody_active: Vec::new(),
            lnactive: Vec::new(),
            hcnstart: Vec::new(),
            hcnend: Vec::new(),
            hcnbody: Vec::new(),
            hcnactive: Vec::new(),
            hcnbody_active: Vec::new(),
            hcndamage: Vec::new(),
            hcnbody_miss: Vec::new(),
            hcnreactive: Vec::new(),
            hcnbody_reactive: Vec::new(),
            mine: Vec::new(),
            hidden: Vec::new(),
            processed: Vec::new(),
            dst: Vec::new(),
            dst2: None,
            expansionrate: vec![FULL_EXPANSION_PERCENT, FULL_EXPANSION_PERCENT],
            size: Vec::new(),
            group: Vec::new(),
            bpm: Vec::new(),
            stop: Vec::new(),
            time: Vec::new(),
        }
    }
}

/// The gauge bar: how many parts it is cut into and how it animates.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GaugeDef {
    pub id: String,
    pub nodes: Vec<String>,
    pub parts: i32,
    #[serde(rename = "type")]
    pub gauge_type: i32,
    pub range: i32,
    pub cycle: i32,
    pub starttime: i32,
    pub endtime: i32,
}

impl Default for GaugeDef {
    fn default() -> Self {
        Self { id: String::new(), nodes: Vec::new(), parts: 50, gauge_type: 0, range: 3, cycle: 33, starttime: 0, endtime: 500 }
    }
}

/// A cover that hides notes above a line, as the hidden modifier draws it.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HiddenCover {
    pub id: String,
    pub src: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
    pub timer: Option<PropertyRef>,
    pub cycle: i32,
    #[serde(rename = "disapearLine")]
    pub disappear_line: i32,
    #[serde(rename = "isDisapearLineLinkLift")]
    pub disappear_line_follows_lift: bool,
}

impl Default for HiddenCover {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            divx: SINGLE_DIVISION,
            divy: SINGLE_DIVISION,
            timer: None,
            cycle: 0,
            disappear_line: -1,
            disappear_line_follows_lift: true,
        }
    }
}

/// A cover that hides notes below a line, as the lift modifier draws it.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LiftCover {
    pub id: String,
    pub src: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub divx: i32,
    pub divy: i32,
    pub timer: Option<PropertyRef>,
    pub cycle: i32,
    #[serde(rename = "disapearLine")]
    pub disappear_line: i32,
    #[serde(rename = "isDisapearLineLinkLift")]
    pub disappear_line_follows_lift: bool,
}

impl Default for LiftCover {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            divx: SINGLE_DIVISION,
            divy: SINGLE_DIVISION,
            timer: None,
            cycle: 0,
            disappear_line: -1,
            disappear_line_follows_lift: false,
        }
    }
}

/// The background animation surface.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BgaDef {
    pub id: String,
}

/// The preview pane the skin selection screen draws.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SkinPreview {
    pub id: String,
}

/// How many practice rows are visible when a document names no count.
const DEFAULT_PRACTICE_ROWS: i32 = 10;

/// The practice configuration pane and how many rows it shows.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Practice {
    pub id: String,
    #[serde(rename = "visibleItems")]
    pub visible_items: i32,
}

impl Default for Practice {
    fn default() -> Self {
        Self { id: String::new(), visible_items: DEFAULT_PRACTICE_ROWS }
    }
}

/// One judgement's pop-up: its images and the count that rides with them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct JudgeDef {
    pub id: String,
    pub index: i32,
    pub images: Vec<Destination>,
    pub numbers: Vec<Destination>,
    pub shift: bool,
}

/// The song wheel: which row is centred and what each row draws.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SongList {
    pub id: String,
    pub center: i32,
    pub clickable: Vec<i32>,
    pub listoff: Vec<Destination>,
    pub liston: Vec<Destination>,
    pub text: Vec<Destination>,
    pub level: Vec<Destination>,
    pub lamp: Vec<Destination>,
    pub playerlamp: Vec<Destination>,
    pub rivallamp: Vec<Destination>,
    pub trophy: Vec<Destination>,
    pub label: Vec<Destination>,
    pub graph: Option<Destination>,
}

/// A character sprite sheet in the reference's pomyu-chara format.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PmChara {
    pub id: String,
    pub src: String,
    pub color: i32,
    #[serde(rename = "type")]
    pub chara_type: Option<i32>,
    pub side: i32,
}

impl Default for PmChara {
    fn default() -> Self {
        Self { id: String::new(), src: String::new(), color: SINGLE_DIVISION, chara_type: None, side: SINGLE_DIVISION }
    }
}

/// How the skin selection screen lays this document's own customisation rows out.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SkinConfigurationProperty {
    #[serde(rename = "customBMS")]
    pub custom_bms: Vec<String>,
    #[serde(rename = "defaultCategory")]
    pub default_category: i32,
    #[serde(rename = "customPropertyCount")]
    pub custom_property_count: i32,
    #[serde(rename = "customOffsetStyle")]
    pub custom_offset_style: i32,
}

impl Default for SkinConfigurationProperty {
    fn default() -> Self {
        Self { custom_bms: Vec::new(), default_category: 0, custom_property_count: -1, custom_offset_style: 0 }
    }
}

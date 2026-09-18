//! The commands every comma-separated body shares, and the builder that turns them into a
//! [`SkinDef`].
//!
//! A `#SRC_*` line declares an object and becomes the current one of its family; the `#DST_*` lines
//! that follow add keyframes to it. That pairing is positional rather than by index, exactly as the
//! reference pairs them, so a body that declares two images and then two destination blocks gets
//! them in the order it wrote them.

use std::path::{Path, PathBuf};

use crate::model::{
    Destination, GaugeDef, GraphDef, HiddenCover, ImageDef, ImageSet, LiftCover, PropertyRef, RectDef, STRETCH_UNSET, SkinDef, SliderDef, Source, TextDef,
    ValueDef,
};

use super::convert::{FIELD_COUNT, Geometry, Rect, parse_fields, push_keyframe, sort_keyframes};
use super::header::CsvHeader;
use super::{Commands, Outcome, graphs, play, select, skin_relative};

/// The lowest image index that names a picture the running game owns rather than one the document
/// loaded, which this build has no object for.
const RUNTIME_IMAGE_BASE: i32 = 100;

/// The `parts` a gauge falls back to when it names none.
const DEFAULT_GAUGE_PARTS: i32 = 50;

/// The `parts` a nine-key gauge falls back to, which is the one screen the reference counts
/// differently.
const POPN_GAUGE_PARTS: i32 = 24;

/// The play document type of the nine-key screen, whose gauge is counted in [`POPN_GAUGE_PARTS`].
const SKIN_TYPE_PLAY_9KEYS: i32 = 4;

/// Cells one gauge node set takes.
const GAUGE_NODES: usize = 4;

/// Cells one extended gauge node set takes.
const GAUGE_NODES_EX: usize = 8;

/// How wide a gauge is per part when its destination sizes itself from the per-part step.
const GAUGE_PARTS_PER_STEP: i32 = 50;

/// The `#SRC_BUTTON` field value that makes the button clickable.
const BUTTON_CLICKABLE: i32 = 1;

/// The click behaviour of a button whose step field is positive: step forwards.
const CLICK_FORWARD: i32 = 0;

/// The click behaviour of a button whose step field is negative: step backwards.
const CLICK_BACKWARD: i32 = 1;

/// The click behaviour of a button whose step field is zero: open the row.
const CLICK_OPEN: i32 = 2;

/// The slider directions measured along the horizontal axis, whose range scales with the width.
const HORIZONTAL_SLIDER_DIRECTIONS: [i32; 2] = [1, 3];

/// The offset the reference adds to every bar graph's property id, which is what separates the
/// graph ids from the plain integer ids sharing the numbering.
const BARGRAPH_PROPERTY_BASE: i32 = 100;

/// The bar graph direction that grows downwards, whose destination is flipped rather than
/// normalised.
const BARGRAPH_DOWNWARD: i32 = 1;

/// The smallest cell grid a number strip is read from. Below this the reference declares no object
/// at all, because the strip cannot hold ten digits.
const MIN_NUMBER_CELLS: i32 = 10;

/// The cell count whose strips carry a negative half, and the digits one half of such a set holds.
const SIGNED_SET_CELLS: i32 = 24;

/// The `zeropadding` a signed strip falls back to when its line leaves the field empty.
const DEFAULT_SIGNED_PADDING: i32 = 2;

/// Which object family a `#DST_*` line adds to.
#[derive(Debug, Default)]
struct Current {
    image: Option<usize>,
    number: Option<usize>,
    text: Option<usize>,
    slider: Option<usize>,
    graph: Option<usize>,
    button: Option<usize>,
    onmouse: Option<usize>,
    gauge: Option<usize>,
}

/// Everything one document's commands build up.
pub(crate) struct Builder {
    pub(crate) def: SkinDef,
    pub(crate) geometry: Geometry,
    pub(crate) skin_type: i32,
    root: PathBuf,
    directory: PathBuf,
    /// One entry per `#IMAGE` line, holding the source id it was registered under.
    images: Vec<String>,
    /// One entry per `#IMAGESET` line, holding the image id its region was registered under.
    sets: Vec<String>,
    current: Current,
    stretch: i32,
    serial: usize,
    gauge_step: (i32, i32),
    graph_direction: i32,
    pub(crate) charts: graphs::ChartState,
    pub(crate) play: play::PlayState,
    pub(crate) select: select::SelectState,
}

impl Builder {
    /// A builder for the document `header` describes.
    pub(crate) fn new(header: &CsvHeader, root: &Path, directory: &Path) -> Self {
        let def = SkinDef {
            skin_type: header.skin_type,
            name: header.name.clone(),
            author: header.author.clone(),
            property: header.properties.clone(),
            filepath: header.filepaths.clone(),
            offset: header.offsets.clone(),
            ..SkinDef::default()
        };
        Self {
            def,
            geometry: Geometry::new(header.resolution),
            skin_type: header.skin_type,
            root: root.to_path_buf(),
            directory: directory.to_path_buf(),
            images: Vec::new(),
            sets: Vec::new(),
            current: Current::default(),
            stretch: STRETCH_UNSET,
            serial: 0,
            gauge_step: (0, 0),
            graph_direction: 0,
            charts: graphs::ChartState::default(),
            play: play::PlayState::default(),
            select: select::SelectState::default(),
        }
    }

    /// The finished document, with the objects that only make sense once every line has been read.
    pub(crate) fn finish(mut self) -> SkinDef {
        play::finish(&mut self);
        select::finish(&mut self);
        for destination in &mut self.def.destination {
            sort_keyframes(destination);
        }
        self.def.destination.retain(|destination| !destination.dst.is_empty());
        self.def
    }

    /// A fresh object id, unique across every family.
    pub(crate) fn next_id(&mut self, kind: &str) -> String {
        self.serial += 1;
        format!("{kind}-{}", self.serial)
    }

    /// The source id image slot `gr` was registered under.
    pub(crate) fn source_id(&self, gr: i32) -> Option<String> {
        usize::try_from(gr).ok().and_then(|index| self.images.get(index)).cloned()
    }

    /// Declares one image cut from the slot a `#SRC_*` line names, and answers its id.
    pub(crate) fn add_image(&mut self, values: &[i32; FIELD_COUNT], kind: &str) -> Option<String> {
        let src = self.source_id(values[2])?;
        let id = self.next_id(kind);
        self.def.image.push(ImageDef {
            id: id.clone(),
            src,
            x: values[3],
            y: values[4],
            w: values[5],
            h: values[6],
            divx: values[7].max(1),
            divy: values[8].max(1),
            timer: timer_of(values[10]),
            cycle: values[9],
            ..ImageDef::default()
        });
        Some(id)
    }

    /// Declares a top-level destination for `id` and answers where it landed.
    pub(crate) fn add_destination(&mut self, id: String) -> usize {
        self.def.destination.push(Destination { id, ..Destination::default() });
        self.def.destination.len() - 1
    }

    /// Adds one keyframe to the destination at `index`.
    pub(crate) fn keyframe(&mut self, index: usize, rect: Rect, values: &[i32; FIELD_COUNT], fields: &[&str], prepend: &[i32]) {
        if let Some(destination) = self.def.destination.get_mut(index) {
            push_keyframe(destination, rect, values, fields, prepend);
        }
    }

    /// A path a command named, as a pattern relative to the document's own directory.
    pub(crate) fn path_of(&self, raw: &str) -> String {
        skin_relative(raw, &self.root, &self.directory)
    }
}

/// The timer a `#DST_*` or `#SRC_*` line names, where anything but a positive id means none.
pub(crate) fn timer_of(id: i32) -> Option<PropertyRef> {
    (id > 0).then_some(PropertyRef::Id(id))
}

impl Commands for Builder {
    fn execute(&mut self, name: &str, fields: &[&str], warnings: &mut Vec<String>) -> Outcome {
        if let Some(outcome) = common(self, name, fields, warnings) {
            return outcome;
        }
        if let Some(outcome) = play::execute(self, name, fields, warnings) {
            return outcome;
        }
        if let Some(outcome) = select::execute(self, name, fields, warnings) {
            return outcome;
        }
        Outcome::Unknown
    }
}

/// The commands every screen's body understands.
fn common(builder: &mut Builder, name: &str, fields: &[&str], warnings: &mut Vec<String>) -> Option<Outcome> {
    let values = parse_fields(fields);
    match name {
        "INCLUDE" => return Some(Outcome::Include(fields.get(1).unwrap_or(&"").to_string())),
        "IMAGE" => image(builder, fields),
        "LR2FONT" => {}
        "SRC_IMAGE" => source_image(builder, &values, warnings),
        "DST_IMAGE" => destination_image(builder, &values, fields),
        "IMAGESET" => image_set_region(builder, &values),
        "SRC_IMAGESET" => image_set(builder, &values),
        "SRC_NUMBER" => source_number(builder, &values, fields),
        "DST_NUMBER" => plain_destination(builder, builder.current.number, &values, fields),
        "SRC_TEXT" => source_text(builder, &values),
        "DST_TEXT" => plain_destination(builder, builder.current.text, &values, fields),
        "SRC_SLIDER" | "SRC_SLIDER_REFNUMBER" => source_slider(builder, &values, name == "SRC_SLIDER_REFNUMBER"),
        "DST_SLIDER" => plain_destination(builder, builder.current.slider, &values, fields),
        "SRC_BARGRAPH" | "SRC_BARGRAPH_REFNUMBER" => source_bargraph(builder, &values, name == "SRC_BARGRAPH_REFNUMBER"),
        "DST_BARGRAPH" => destination_bargraph(builder, &values, fields),
        "SRC_BUTTON" => source_button(builder, &values),
        "DST_BUTTON" => plain_destination(builder, builder.current.button, &values, fields),
        "SRC_ONMOUSE" => source_onmouse(builder, &values),
        "DST_ONMOUSE" => plain_destination(builder, builder.current.onmouse, &values, fields),
        "SRC_GROOVEGAUGE" => source_gauge(builder, &values, GAUGE_NODES, warnings),
        "SRC_GROOVEGAUGE_EX" => source_gauge(builder, &values, GAUGE_NODES_EX, warnings),
        "DST_GROOVEGAUGE" => destination_gauge(builder, &values, fields),
        "STRETCH" => builder.stretch = values[1],
        "STARTINPUT" => builder.def.input = values[1],
        "SCENETIME" => builder.def.scene = values[1],
        "FADEOUT" => builder.def.fadeout = values[1],
        "CLOSE" => builder.def.close = values[1],
        "PLAYSTART" => builder.def.playstart = values[1],
        "LOADEND" => builder.def.loadend = values[1],
        "LOADSTART" => {}
        "FINISHMARGIN" => builder.def.finishmargin = values[1],
        "JUDGETIMER" => builder.def.judgetimer = values[1],
        "SETOPTION" | "IF" | "ELSEIF" | "ELSE" | "ENDIF" => {}
        "INFORMATION" | "RESOLUTION" | "CUSTOMOPTION" | "CUSTOMFILE" | "CUSTOMOFFSET" | "CUSTOMOPTION_ADDITION_SETTING" | "ENDOFHEADER" => {}
        _ => return None,
    }
    Some(Outcome::Handled)
}

/// One keyframe on the current object of a family, in the plain conversion most commands use.
pub(crate) fn plain_destination(builder: &mut Builder, index: Option<usize>, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let Some(index) = index else { return };
    let rect = builder.geometry.dst_rect(values, false);
    builder.keyframe(index, rect, values, fields, &[]);
}

/// `#IMAGE,path`. A file that is not there keeps its slot so the indices after it still line up.
fn image(builder: &mut Builder, fields: &[&str]) {
    let path = builder.path_of(fields.get(1).unwrap_or(&""));
    let id = builder.next_id("source");
    builder.images.push(id.clone());
    builder.def.source.push(Source { id, path });
}

/// `#SRC_IMAGE,(index),slot,x,y,w,h,divx,divy,cycle,timer`.
fn source_image(builder: &mut Builder, values: &[i32; FIELD_COUNT], warnings: &mut Vec<String>) {
    builder.current.image = None;
    if values[2] >= RUNTIME_IMAGE_BASE {
        let line = format!("image {} is one the running game owns, which this build has no object for", values[2]);
        if !warnings.contains(&line) {
            warnings.push(line);
        }
        return;
    }
    let Some(id) = builder.add_image(values, "image") else { return };
    builder.current.image = Some(builder.add_destination(id));
}

/// `#DST_IMAGE`, the only destination that also carries the stretch mode a `#STRETCH` line set.
fn destination_image(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let Some(index) = builder.current.image else { return };
    let rect = builder.geometry.dst_rect(values, true);
    builder.keyframe(index, rect, values, fields, &[]);
    let stretch = builder.stretch;
    if let Some(destination) = builder.def.destination.get_mut(index) {
        destination.stretch = stretch;
    }
}

/// `#IMAGESET,(index),slot,x,y,w,h,divx,divy,cycle,timer`, one member of the set declared next.
fn image_set_region(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    let Some(id) = builder.add_image(values, "set-member") else {
        builder.sets.push(String::new());
        return;
    };
    builder.sets.push(id);
}

/// `#SRC_IMAGESET,value id,timer,cycle,count,member,member,...`.
fn image_set(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    builder.current.image = None;
    let count = values[4].max(0) as usize;
    let images: Vec<String> = (0..count)
        .filter_map(|member| values.get(5 + member))
        .filter_map(|slot| usize::try_from(*slot).ok())
        .filter_map(|slot| builder.sets.get(slot))
        .filter(|id| !id.is_empty())
        .cloned()
        .collect();
    if images.is_empty() {
        return;
    }
    let id = builder.next_id("imageset");
    builder.def.imageset.push(ImageSet { id: id.clone(), reference: values[1], images, ..ImageSet::default() });
    builder.current.image = Some(builder.add_destination(id));
}

/// `#SRC_NUMBER,(index),slot,x,y,w,h,divx,divy,cycle,timer,value,align,digits,padding,space`.
///
/// A strip whose grid holds fewer than ten cells declares nothing, because it cannot hold the
/// digits. A grid that is a multiple of twenty-four carries a negative half, which reserves one
/// more place for the sign and reads its padding from its own field.
fn source_number(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    builder.current.number = None;
    let cells = values[7].max(1) * values[8].max(1);
    if cells < MIN_NUMBER_CELLS {
        return;
    }
    let Some(src) = builder.source_id(values[2]) else { return };
    let signed = cells % SIGNED_SET_CELLS == 0;
    let padding = match fields.get(14) {
        Some(field) if !field.is_empty() => values[14],
        _ => DEFAULT_SIGNED_PADDING,
    };
    let id = builder.next_id("number");
    builder.def.value.push(ValueDef {
        id: id.clone(),
        src,
        x: values[3],
        y: values[4],
        w: values[5],
        h: values[6],
        divx: values[7].max(1),
        divy: values[8].max(1),
        timer: timer_of(values[10]),
        cycle: values[9],
        align: values[12],
        digit: if signed { values[13] + 1 } else { values[13] },
        zeropadding: if signed { padding } else { 0 },
        space: values[15],
        reference: values[11],
        ..ValueDef::default()
    });
    builder.current.number = Some(builder.add_destination(id));
}

/// `#SRC_TEXT,(index),font,value,align,editable,panel`.
///
/// The font index is dropped: this build draws every run with its own engine font rather than with
/// the bitmap font a comma-separated skin ships.
fn source_text(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    let id = builder.next_id("text");
    builder.def.text.push(TextDef { id: id.clone(), align: values[4], reference: values[3], editable: values[5] != 0, ..TextDef::default() });
    builder.current.text = Some(builder.add_destination(id));
}

/// `#SRC_SLIDER,(index),slot,x,y,w,h,divx,divy,cycle,timer,direction,range,value,disabled`.
fn source_slider(builder: &mut Builder, values: &[i32; FIELD_COUNT], ref_num: bool) {
    builder.current.slider = None;
    let Some(src) = builder.source_id(values[2]) else { return };
    let range = if HORIZONTAL_SLIDER_DIRECTIONS.contains(&values[11]) { builder.geometry.scale_x(values[12]) } else { builder.geometry.scale_y(values[12]) };
    let id = builder.next_id("slider");
    builder.def.slider.push(SliderDef {
        id: id.clone(),
        src,
        x: values[3],
        y: values[4],
        w: values[5],
        h: values[6],
        divx: values[7].max(1),
        divy: values[8].max(1),
        timer: timer_of(values[10]),
        cycle: values[9],
        angle: values[11],
        range,
        slider_type: values[13],
        changeable: !ref_num && values[14] == 0,
        is_ref_num: ref_num,
        min: if ref_num { values[15] } else { 0 },
        max: if ref_num { values[16] } else { 0 },
        ..SliderDef::default()
    });
    builder.current.slider = Some(builder.add_destination(id));
}

/// `#SRC_BARGRAPH,(index),slot,x,y,w,h,divx,divy,cycle,timer,value,direction`.
fn source_bargraph(builder: &mut Builder, values: &[i32; FIELD_COUNT], ref_num: bool) {
    builder.current.graph = None;
    let Some(src) = builder.source_id(values[2]) else { return };
    let id = builder.next_id("graph");
    builder.def.graph.push(GraphDef {
        id: id.clone(),
        src,
        x: values[3],
        y: values[4],
        w: values[5],
        h: values[6],
        divx: values[7].max(1),
        divy: values[8].max(1),
        timer: timer_of(values[10]),
        cycle: values[9],
        angle: values[12],
        graph_type: if ref_num { values[11] } else { values[11] + BARGRAPH_PROPERTY_BASE },
        is_ref_num: ref_num,
        min: if ref_num { values[13] } else { 0 },
        max: if ref_num { values[14] } else { 0 },
        ..GraphDef::default()
    });
    builder.graph_direction = values[12];
    builder.current.graph = Some(builder.add_destination(id));
}

/// `#DST_BARGRAPH`. A graph that grows downwards is flipped rather than normalised, which is what
/// leaves it with a negative height the renderer draws upwards from the line it was anchored to.
fn destination_bargraph(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let Some(index) = builder.current.graph else { return };
    let rect = if builder.graph_direction == BARGRAPH_DOWNWARD {
        builder.geometry.rect(values[3], values[4] + values[6], values[5], -values[6])
    } else {
        builder.geometry.dst_rect(values, false)
    };
    builder.keyframe(index, rect, values, fields, &[]);
}

/// `#SRC_BUTTON,(index),slot,x,y,w,h,divx,divy,cycle,timer,event,clickable,(NULL),step,groups`.
fn source_button(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    builder.current.button = None;
    let Some(id) = builder.add_image(values, "button") else { return };
    let groups = if values[15] > 0 { values[15] } else { values[7].max(1) * values[8].max(1) };
    let click = if values[14] > 0 {
        CLICK_FORWARD
    } else if values[14] < 0 {
        CLICK_BACKWARD
    } else {
        CLICK_OPEN
    };
    if let Some(image) = builder.def.image.iter_mut().find(|image| image.id == id) {
        image.len = groups;
        image.reference = values[11];
        if values[12] == BUTTON_CLICKABLE {
            image.act = Some(PropertyRef::Id(values[11]));
            image.click = click;
        }
    }
    builder.current.button = Some(builder.add_destination(id));
}

/// `#SRC_ONMOUSE,(index),slot,x,y,w,h,divx,divy,cycle,timer,(NULL),rect x,rect y,rect w,rect h`.
///
/// The hit rectangle's vertical position is written from the bottom of the source rectangle, which
/// is why it is taken out of the source height rather than used as written.
fn source_onmouse(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    builder.current.onmouse = None;
    let Some(id) = builder.add_image(values, "onmouse") else { return };
    let index = builder.add_destination(id);
    let rect = RectDef { x: values[12], y: values[6] - values[13] - values[15], w: values[14], h: values[15] };
    if let Some(destination) = builder.def.destination.get_mut(index) {
        destination.mouse_rect = Some(rect);
    }
    builder.current.onmouse = Some(index);
}

/// `#SRC_GROOVEGAUGE,side,slot,x,y,w,h,divx,divy,cycle,timer,step x,step y,parts,animation,range,cycle,start,end`.
fn source_gauge(builder: &mut Builder, values: &[i32; FIELD_COUNT], per_set: usize, warnings: &mut Vec<String>) {
    builder.current.gauge = None;
    let Some(src) = builder.source_id(values[2]) else { return };
    let (divx, divy) = (values[7].max(1), values[8].max(1));
    let cells = (divx * divy) as usize;
    if cells < per_set {
        warnings.push(format!("gauge strip holds {cells} cells, which is fewer than the {per_set} one set needs"));
        return;
    }
    let (width, height) = (values[5] / divx, values[6] / divy);
    let mut nodes = Vec::with_capacity(per_set);
    for cell in 0..per_set {
        let (column, row) = (cell as i32 % divx, cell as i32 / divx);
        let id = builder.next_id("gauge-node");
        builder.def.image.push(ImageDef {
            id: id.clone(),
            src: src.clone(),
            x: values[3] + width * column,
            y: values[4] + height * row,
            w: width,
            h: height,
            ..ImageDef::default()
        });
        nodes.push(id);
    }
    let default_parts = if builder.skin_type == SKIN_TYPE_PLAY_9KEYS { POPN_GAUGE_PARTS } else { DEFAULT_GAUGE_PARTS };
    let id = builder.next_id("gauge");
    builder.def.gauge = Some(GaugeDef {
        id: id.clone(),
        nodes,
        parts: if values[13] > 0 { values[13] } else { default_parts },
        gauge_type: values[14],
        range: values[15],
        cycle: values[16],
        starttime: values[17],
        endtime: values[18],
    });
    builder.gauge_step = (values[11], values[12]);
    builder.current.gauge = Some(builder.add_destination(id));
}

/// `#DST_GROOVEGAUGE`, whose size comes from the per-part step when the source line named one.
fn destination_gauge(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let Some(index) = builder.current.gauge else { return };
    let (step_x, step_y) = builder.gauge_step;
    let width = if step_x.abs() >= 1 { builder.geometry.scale_x(step_x * GAUGE_PARTS_PER_STEP) } else { builder.geometry.scale_x(values[5]) };
    let height = if step_y.abs() >= 1 { builder.geometry.scale_y(step_y * GAUGE_PARTS_PER_STEP) } else { builder.geometry.scale_y(values[6]) };
    let left = builder.geometry.scale_x(values[3]) - if step_x < 0 { builder.geometry.scale_x(step_x) } else { 0 };
    let rect = Rect { x: left, y: crate::model::DEFAULT_SKIN_HEIGHT - builder.geometry.scale_y(values[4]) - height, w: width, h: height };
    builder.keyframe(index, rect, values, fields, &[]);
}

/// Declares a hidden-note cover from a `#SRC_HIDDEN` line and answers its destination index.
pub(crate) fn add_hidden_cover(builder: &mut Builder, values: &[i32; FIELD_COUNT], lift: bool) -> Option<usize> {
    let src = builder.source_id(values[2])?;
    let id = builder.next_id(if lift { "lift" } else { "hidden" });
    let (divx, divy) = (values[7].max(1), values[8].max(1));
    if lift {
        builder.def.lift_cover.push(LiftCover {
            id: id.clone(),
            src,
            x: values[3],
            y: values[4],
            w: values[5],
            h: values[6],
            divx,
            divy,
            timer: timer_of(values[10]),
            cycle: values[9],
            disappear_line: values[11],
            disappear_line_follows_lift: values[12] != 0,
        });
    } else {
        builder.def.hidden_cover.push(HiddenCover {
            id: id.clone(),
            src,
            x: values[3],
            y: values[4],
            w: values[5],
            h: values[6],
            divx,
            divy,
            timer: timer_of(values[10]),
            cycle: values[9],
            disappear_line: values[11],
            disappear_line_follows_lift: values[12] != 0,
        });
    }
    Some(builder.add_destination(id))
}

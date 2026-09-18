//! The three charts a body draws beside the field or the wheel: the note distribution, the tempo
//! timeline and the judge-window ruler.
//!
//! Each is declared by a source line that carries the field size and the palette, and placed by a
//! destination line that carries only the corner: the size comes from the source rather than from
//! the destination, which is why these three are converted apart from every other command.

use crate::model::{BpmGraph, JudgeGraph, TimingVisualizer};

use super::Outcome;
use super::body::Builder;
use super::convert::{FIELD_COUNT, parse_fields};

/// A chart declared but not yet placed, holding the field size its destination will be drawn at.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ChartSlot {
    destination: Option<usize>,
    size: (i32, i32),
}

/// The charts one body declares.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ChartState {
    notes: ChartSlot,
    bpm: ChartSlot,
    timing: ChartSlot,
}

/// Reads one field as text, trimmed, which is how every palette entry is written.
fn colour(fields: &[&str], index: usize) -> Option<String> {
    fields.get(index).map(|field| field.trim()).filter(|field| !field.is_empty()).map(str::to_owned)
}

/// Runs one chart command, or answers `None` when the name is not one of them.
pub(crate) fn execute(builder: &mut Builder, name: &str, fields: &[&str], _warnings: &mut Vec<String>) -> Option<Outcome> {
    let values = parse_fields(fields);
    match name {
        "SRC_NOTECHART" | "SRC_NOTECHART_1P" | "SRC_NOTECHART_2P" => source_notes(builder, &values),
        "DST_NOTECHART" | "DST_NOTECHART_1P" | "DST_NOTECHART_2P" => place(builder, |charts| charts.notes, &values, fields),
        "SRC_BPMCHART" => source_bpm(builder, &values, fields),
        "DST_BPMCHART" => place(builder, |charts| charts.bpm, &values, fields),
        "SRC_TIMING_1P" | "SRC_TIMING_2P" => source_timing(builder, &values, fields),
        "DST_TIMING_1P" | "DST_TIMING_2P" => place(builder, |charts| charts.timing, &values, fields),
        _ => return None,
    }
    Some(Outcome::Handled)
}

/// `#SRC_NOTECHART*,type,(slot),(x),(y),(w),(h),(divx),(divy),(cycle),(timer),field w,field h,(start),(end),delay,back,reverse,no gap,no gap x`.
fn source_notes(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    let id = builder.next_id("notechart");
    builder.def.judgegraph.push(JudgeGraph {
        id: id.clone(),
        graph_type: values[1],
        delay: values[15],
        back_tex_off: values[16],
        order_reverse: values[17],
        no_gap: values[18],
        no_gap_x: values[19],
    });
    let destination = builder.add_destination(id);
    builder.charts.notes = ChartSlot { destination: Some(destination), size: (values[11], values[12]) };
}

/// `#SRC_BPMCHART,field w,field h,delay,line width,main,min,max,other,stop,transition`.
fn source_bpm(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let id = builder.next_id("bpmchart");
    let mut graph = BpmGraph { id: id.clone(), delay: values[3], line_width: values[4], ..BpmGraph::default() };
    if let Some(value) = colour(fields, 5) {
        graph.main_bpm_color = value;
    }
    if let Some(value) = colour(fields, 6) {
        graph.min_bpm_color = value;
    }
    if let Some(value) = colour(fields, 7) {
        graph.max_bpm_color = value;
    }
    if let Some(value) = colour(fields, 8) {
        graph.other_bpm_color = value;
    }
    if let Some(value) = colour(fields, 9) {
        graph.stop_line_color = value;
    }
    if let Some(value) = colour(fields, 10) {
        graph.transition_line_color = value;
    }
    builder.def.bpmgraph.push(graph);
    let destination = builder.add_destination(id);
    builder.charts.bpm = ChartSlot { destination: Some(destination), size: (values[1], values[2]) };
}

/// `#SRC_TIMING_*,(index),(slot),(x),width,height,judge ms,line width,line,centre,PG,GR,GD,BD,PR,transparent,decay`.
fn source_timing(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let id = builder.next_id("timing");
    let mut graph = TimingVisualizer {
        id: id.clone(),
        width: values[4],
        judge_width_millis: values[6],
        line_width: values[7],
        transparent: values[15],
        draw_decay: values[16],
        ..TimingVisualizer::default()
    };
    for (index, slot) in [
        &mut graph.line_color,
        &mut graph.center_color,
        &mut graph.pgreat_color,
        &mut graph.great_color,
        &mut graph.good_color,
        &mut graph.bad_color,
        &mut graph.poor_color,
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(value) = colour(fields, index + 8) {
            *slot = value;
        }
    }
    builder.def.timingvisualizer.push(graph);
    let destination = builder.add_destination(id);
    builder.charts.timing = ChartSlot { destination: Some(destination), size: (values[4], values[5]) };
}

/// Places one chart, taking its size from the source line and its corner from this one.
fn place(builder: &mut Builder, pick: fn(&ChartState) -> ChartSlot, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let slot = pick(&builder.charts);
    let Some(index) = slot.destination else { return };
    let rect = builder.geometry.rect(values[3], values[4], slot.size.0, slot.size.1);
    builder.keyframe(index, rect, values, fields, &[]);
}

//! NOTES RADAR: six per-chart difficulty axes derived from a decoded [`Model`].
//!
//! [`radar_raw`] measures the six raw metrics once per chart (the value to persist), and
//! [`radar_display`] maps them onto the 0..=200 display scale (the value to draw). Keeping the two
//! apart means a change to the display curves or scale constants never invalidates stored raw
//! values; only a change to a metric definition does, and that bumps [`RADAR_VERSION`].
//!
//! Every metric counts the same "press events": a normal note or a long-note head, in any lane.
//! A long-note tail is never a press event and a mine is not a note, so the result does not depend
//! on the long-note flavour (LN, CN, HCN or still undefined), which the player's LN MODE decides at
//! play time. This differs on purpose from [`NoteKind::counts_as_playable_note`], which also counts
//! the tail of a declared CN or HCN.

use rbms_model::{Micros, Mode, Model, NoteKind, TimeLine};

/// Version of the raw metric definitions in this module. Raise it whenever [`radar_raw`] would
/// return a different value for the same chart, so stored values can be recomputed.
pub const RADAR_VERSION: u32 = 1;

/// Number of radar axes.
pub const RADAR_AXIS_COUNT: usize = 6;

/// Largest display value; a raw value at or past the last breakpoint of its axis maps here.
pub const RADAR_DISPLAY_MAX: f32 = 200.0;

const RADAR_MIN_SPAN_US: Micros = crate::MICROS_PER_SECOND;

const RADAR_PEAK_WINDOW_US: Micros = 4 * crate::MICROS_PER_SECOND;

const RADAR_SOFLAN_WINDOW_US: Micros = 2 * crate::MICROS_PER_SECOND;

const SOFLAN_RATIO_MIN: f64 = 0.01;

const SOFLAN_RATIO_MAX: f64 = 1.26;

const CHARGE_HEAD_SCORE: f64 = 3.0;

const CHARGE_SIMULTANEOUS_SCORE: f64 = 3.0;

const CHARGE_PASSING_SCORE: f64 = 4.0;

const CHORD_MIN_SIMULTANEOUS: usize = 2;

const CHORD_WEIGHT_OFFSET: usize = 6;

const CHORD_WEIGHT_DENOMINATOR: f64 = 40.0;

/// Display-side multiplier that brings the SOF-LAN raw sum onto the breakpoint scale of its axis.
///
/// Provisional: the community definition of this axis publishes no unit, so the value is not
/// derived from the arcade game. It is tuned against a library of single-play charts so that the
/// 90th percentile of the charts with a non-zero raw value lands near display 150, and it is the
/// first thing to adjust after looking at real charts.
pub const RADAR_SOFLAN_SCALE: f64 = 13.0;

/// Display-side multiplier that brings the CHARGE raw rate onto the breakpoint scale of its axis.
/// Provisional and tuned by the same rule as [`RADAR_SOFLAN_SCALE`].
pub const RADAR_CHARGE_SCALE: f64 = 3.0;

/// Display-side multiplier that brings the CHORD raw rate onto the breakpoint scale of its axis.
/// Provisional and tuned by the same rule as [`RADAR_SOFLAN_SCALE`].
pub const RADAR_CHORD_SCALE: f64 = 300.0;

const BREAKPOINT_COUNT: usize = 4;

type Curve = [f64; BREAKPOINT_COUNT];

/// Display value reached at each of the four breakpoints of a curve; the segments between them are
/// linear, and the first segment starts at zero.
const DISPLAY_AT_BREAKPOINT: Curve = [100.0, 150.0, 175.0, RADAR_DISPLAY_MAX as f64];

/// Breakpoints `p0..p3` per axis in [`RadarAxis`] order for single-play modes, from the published
/// reverse engineering of the arcade game (UkiyoAmeto/iidx33-notesradar, `docs/algorithm.md` 2.1).
const SINGLE_CURVES: [Curve; RADAR_AXIS_COUNT] = [
    [10.2, 15.3, 17.7, 20.5],
    [70.0, 108.0, 125.0, 136.0],
    [1.4, 2.6, 3.9, 6.2],
    [440.0, 1000.0, 2000.0, 3800.0],
    [160.0, 310.0, 440.0, 900.0],
    [500.0, 1010.0, 1200.0, 1450.0],
];

/// Breakpoints `p0..p3` per axis in [`RadarAxis`] order for double-play modes, from the same source
/// as [`SINGLE_CURVES`]. The PEAK row takes the third breakpoint (122) from that source's parameter
/// table; an older community measurement of the same curve reads 125 there.
const DOUBLE_CURVES: [Curve; RADAR_AXIS_COUNT] = [
    [10.4, 15.4, 17.9, 20.0],
    [70.0, 113.0, 122.0, 170.0],
    [1.4, 2.6, 3.9, 7.0],
    [440.0, 1000.0, 2000.0, 4000.0],
    [145.0, 280.0, 400.0, 750.0],
    [500.0, 1000.0, 1200.0, 1500.0],
];

/// Multiplier applied to a raw value before its curve, per axis in [`RadarAxis`] order.
const AXIS_SCALE: [f64; RADAR_AXIS_COUNT] = [1.0, 1.0, 1.0, RADAR_SOFLAN_SCALE, RADAR_CHARGE_SCALE, RADAR_CHORD_SCALE];

/// One radar axis. The discriminants are the axis indices, which follow the order the axes are
/// drawn clockwise from the top of the hexagon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadarAxis {
    Notes = 0,
    Peak = 1,
    Scratch = 2,
    Soflan = 3,
    Charge = 4,
    Chord = 5,
}

impl RadarAxis {
    pub const ALL: [RadarAxis; RADAR_AXIS_COUNT] =
        [RadarAxis::Notes, RadarAxis::Peak, RadarAxis::Scratch, RadarAxis::Soflan, RadarAxis::Charge, RadarAxis::Chord];
}

/// The six raw metrics of a chart, before any display scaling.
///
/// `notes`, `scratch`, `charge` and `chord` are rates per second of the chart's playing span (first
/// to last press event, at least one second). `peak` is a press-event count and `soflan` a
/// weighted note sum; neither is divided by time.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RadarRaw {
    /// Press events per second.
    pub notes: f64,
    /// Most press events in any window of four seconds that starts at a press event.
    pub peak: f64,
    /// Press events on scratch lanes per second.
    pub scratch: f64,
    /// Notes after each speed change, weighted by how fast the change was and how soon the note
    /// follows it.
    pub soflan: f64,
    /// Long-note difficulty score per second: each long note scores for the notes pressed with it
    /// and for the notes pressed while it is held.
    pub charge: f64,
    /// Weighted count of simultaneous presses per second.
    pub chord: f64,
}

impl RadarRaw {
    pub fn value(&self, axis: RadarAxis) -> f64 {
        match axis {
            RadarAxis::Notes => self.notes,
            RadarAxis::Peak => self.peak,
            RadarAxis::Scratch => self.scratch,
            RadarAxis::Soflan => self.soflan,
            RadarAxis::Charge => self.charge,
            RadarAxis::Chord => self.chord,
        }
    }
}

#[derive(Clone, Copy)]
enum RadarStyle {
    Single,
    Double,
}

fn radar_style(mode: &Mode) -> Option<RadarStyle> {
    if *mode == Mode::BEAT_5K || *mode == Mode::BEAT_7K {
        return Some(RadarStyle::Single);
    }
    if *mode == Mode::BEAT_10K || *mode == Mode::BEAT_14K {
        return Some(RadarStyle::Double);
    }
    None
}

fn is_press(kind: &NoteKind) -> bool {
    matches!(kind, NoteKind::Normal | NoteKind::LongStart { .. })
}

fn press_count(timeline: &TimeLine, mode: &Mode) -> usize {
    timeline.notes.iter().take(mode.key).flatten().filter(|note| is_press(&note.kind)).count()
}

fn seconds(us: Micros) -> f64 {
    us as f64 / crate::MICROS_PER_SECOND as f64
}

struct Presses {
    times: Vec<Micros>,
    scratch: usize,
    long_notes: Vec<(Micros, Micros)>,
}

fn collect_presses(model: &Model, mode: &Mode) -> Presses {
    let mut times = Vec::new();
    let mut scratch = 0;
    let mut long_notes = Vec::new();
    let mut open_heads: Vec<Option<Micros>> = vec![None; mode.key];
    for timeline in &model.timelines {
        for (lane, note) in timeline.notes.iter().enumerate().take(mode.key) {
            let Some(note) = note else {
                continue;
            };
            if is_press(&note.kind) {
                times.push(timeline.time_us);
                if mode.is_scratch(lane) {
                    scratch += 1;
                }
            }
            match note.kind {
                NoteKind::LongStart { .. } => {
                    if let Some(unclosed) = open_heads[lane].replace(timeline.time_us) {
                        long_notes.push((unclosed, unclosed));
                    }
                }
                NoteKind::LongEnd { .. } => {
                    if let Some(head) = open_heads[lane].take() {
                        long_notes.push((head, timeline.time_us));
                    }
                }
                NoteKind::Normal | NoteKind::Mine { .. } => {}
            }
        }
    }
    long_notes.extend(open_heads.into_iter().flatten().map(|head| (head, head)));
    times.sort_unstable();
    Presses { times, scratch, long_notes }
}

fn span_seconds(times: &[Micros]) -> f64 {
    let span = match (times.first(), times.last()) {
        (Some(first), Some(last)) => last.saturating_sub(*first),
        _ => 0,
    };
    seconds(span.max(RADAR_MIN_SPAN_US))
}

fn peak_count(times: &[Micros]) -> usize {
    let mut end = 0;
    let mut peak = 0;
    for (start, time) in times.iter().enumerate() {
        let limit = time.saturating_add(RADAR_PEAK_WINDOW_US);
        end = end.max(start + 1);
        while end < times.len() && times[end] < limit {
            end += 1;
        }
        peak = peak.max(end - start);
    }
    peak
}

fn chord_weight(simultaneous: usize) -> f64 {
    ((simultaneous - 1) * (simultaneous + CHORD_WEIGHT_OFFSET)) as f64 / CHORD_WEIGHT_DENOMINATOR
}

fn chord_sum(times: &[Micros]) -> f64 {
    let mut total = 0.0;
    for simultaneous in times.chunk_by(|a, b| a == b).map(<[Micros]>::len) {
        if simultaneous >= CHORD_MIN_SIMULTANEOUS {
            total += chord_weight(simultaneous);
        }
    }
    total
}

fn charge_sum(times: &[Micros], long_notes: &[(Micros, Micros)]) -> f64 {
    let mut total = 0.0;
    for &(head, tail) in long_notes {
        let before_head = times.partition_point(|time| *time < head);
        let through_head = times.partition_point(|time| *time <= head);
        let through_tail = times.partition_point(|time| *time <= tail.max(head));
        let simultaneous = (through_head - before_head).saturating_sub(1);
        let passing = through_tail - through_head;
        total += CHARGE_HEAD_SCORE + CHARGE_SIMULTANEOUS_SCORE * simultaneous as f64 + CHARGE_PASSING_SCORE * passing as f64;
    }
    total
}

fn scrolled_speed(timeline: &TimeLine) -> f64 {
    let speed = (timeline.bpm * timeline.scroll).abs();
    if speed.is_finite() { speed } else { 0.0 }
}

fn timeline_speed(timeline: &TimeLine) -> f64 {
    if timeline.stop_us > 0 { 0.0 } else { scrolled_speed(timeline) }
}

fn speed_change_ratio(before: f64, after: f64) -> f64 {
    let slowest = before.min(after);
    if slowest <= 0.0 {
        return SOFLAN_RATIO_MAX;
    }
    ((before - after).abs() / slowest).clamp(SOFLAN_RATIO_MIN, SOFLAN_RATIO_MAX)
}

fn soflan_sum(model: &Model, mode: &Mode) -> f64 {
    let Some(first) = model.timelines.first() else {
        return 0.0;
    };
    let mut speed = scrolled_speed(first);
    let mut window: Option<(Micros, f64)> = None;
    let mut sum = 0.0;
    for timeline in &model.timelines {
        let now = timeline_speed(timeline);
        if now != speed {
            window = Some((timeline.time_us, speed_change_ratio(speed, now)));
            speed = now;
        }
        let Some((start, ratio)) = window else {
            continue;
        };
        let elapsed = timeline.time_us.saturating_sub(start).max(0);
        if elapsed < RADAR_SOFLAN_WINDOW_US {
            sum += press_count(timeline, mode) as f64 * ratio * (1.0 - elapsed as f64 / RADAR_SOFLAN_WINDOW_US as f64);
        }
    }
    sum
}

/// Measures the six raw radar metrics of `model` for `mode`.
///
/// Returns `None` when the mode has no radar: only the 5-key and 7-key modes (single-play curves)
/// and the 10-key and 14-key modes (double-play curves) do. A chart without any press event yields
/// all zeros.
///
/// Time is measured between the first and last press event, with a floor of one second so a one-note
/// or all-at-once chart cannot divide by zero. The result is finite for any model.
pub fn radar_raw(model: &Model, mode: &Mode) -> Option<RadarRaw> {
    radar_style(mode)?;
    let Presses { times, scratch, long_notes } = collect_presses(model, mode);
    let span = span_seconds(&times);
    Some(RadarRaw {
        notes: times.len() as f64 / span,
        peak: peak_count(&times) as f64,
        scratch: scratch as f64 / span,
        soflan: soflan_sum(model, mode),
        charge: charge_sum(&times, &long_notes) / span,
        chord: chord_sum(&times) / span,
    })
}

fn display_on_curve(value: f64, curve: &Curve) -> f64 {
    let value = value.max(0.0);
    if value <= curve[0] {
        return DISPLAY_AT_BREAKPOINT[0] * (value / curve[0]);
    }
    for segment in 1..BREAKPOINT_COUNT {
        if value <= curve[segment] {
            let fraction = (value - curve[segment - 1]) / (curve[segment] - curve[segment - 1]);
            let low = DISPLAY_AT_BREAKPOINT[segment - 1];
            return low + (DISPLAY_AT_BREAKPOINT[segment] - low) * fraction;
        }
    }
    DISPLAY_AT_BREAKPOINT[BREAKPOINT_COUNT - 1]
}

/// Maps raw metrics onto the 0..=200 display scale of each axis, in [`RadarAxis`] order.
///
/// Each axis is a four-segment piecewise-linear curve: 0 to 100 up to its first breakpoint, then
/// 150, 175 and 200 at the next three, and 200 from the last one on. Single-play modes use the
/// single-play breakpoints and double-play modes the double-play ones. A mode without a radar
/// yields all zeros; a negative or NaN raw value counts as zero.
pub fn radar_display(raw: &RadarRaw, mode: &Mode) -> [f32; RADAR_AXIS_COUNT] {
    let curves = match radar_style(mode) {
        Some(RadarStyle::Single) => &SINGLE_CURVES,
        Some(RadarStyle::Double) => &DOUBLE_CURVES,
        None => return [0.0; RADAR_AXIS_COUNT],
    };
    RadarAxis::ALL.map(|axis| {
        let index = axis as usize;
        display_on_curve(raw.value(axis) * AXIS_SCALE[index], &curves[index]) as f32
    })
}

#[cfg(test)]
mod tests;

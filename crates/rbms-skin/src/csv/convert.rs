//! The numbers every CSV command shares: how a field is read, how a source rectangle becomes a
//! document rectangle, and how one `#DST_*` line becomes a keyframe.
//!
//! A CSV skin is authored against the resolution its header names, with the origin at the top left,
//! while this build's documents are authored against 1280x720 with the origin at the bottom left.
//! Every rectangle therefore passes through [`Geometry`] once, at the moment its command is read,
//! so nothing downstream has to know which format the document was written in.

use crate::model::{Animation, DEFAULT_SKIN_HEIGHT, DEFAULT_SKIN_WIDTH, Destination, DestinationOption, PropertyRef};

/// How many fields of a command line are read as numbers. The reference fills a fixed array of this
/// width before the offset ids begin, so a shorter line leaves the rest at zero.
pub(crate) const FIELD_COUNT: usize = 22;

/// The first field an offset id may appear in, which is one past the last numbered argument.
pub(crate) const OFFSET_FIELD: usize = 21;

/// The resolutions `#RESOLUTION` numbers, in the order its argument indexes them.
const RESOLUTIONS: [(i32, i32); 4] = [(640, 480), (1280, 720), (1920, 1080), (3840, 2160)];

/// The resolution a header that names none is authored against.
pub(crate) const DEFAULT_RESOLUTION: (i32, i32) = RESOLUTIONS[0];

/// How many anchor points a destination's `center` field numbers. A value outside the range leaves
/// the anchor where it was, as it does in the reference.
const CENTER_COUNT: i32 = 10;

/// The character a CSV field writes a minus sign as, which is also how a draw condition says "not".
pub(crate) const NEGATION: char = '!';

/// The resolution index `index` names, or `None` when the header names one this build has no size
/// for.
pub(crate) fn resolution(index: i32) -> Option<(i32, i32)> {
    usize::try_from(index).ok().and_then(|index| RESOLUTIONS.get(index)).copied()
}

/// One field as a number, read the way the reference reads it: `!` stands for a minus sign and
/// spaces are dropped. Anything that is still not a number answers `None`, which every caller but
/// the draw conditions turns into a zero.
pub(crate) fn parse_number(field: &str) -> Option<i32> {
    let cleaned: String = field.chars().filter(|glyph| *glyph != ' ').map(|glyph| if glyph == NEGATION { '-' } else { glyph }).collect();
    cleaned.parse().ok()
}

/// Every numbered argument of one command line, with an unreadable or missing field as zero.
pub(crate) fn parse_fields(fields: &[&str]) -> [i32; FIELD_COUNT] {
    let mut values = [0; FIELD_COUNT];
    for (index, slot) in values.iter_mut().enumerate().skip(1) {
        let Some(field) = fields.get(index) else { break };
        *slot = parse_number(field).unwrap_or_default();
    }
    values
}

/// The offset ids a `#DST_*` line carries past its numbered arguments, after the ones the command
/// itself prepends.
///
/// The reference strips every character that is not a digit or a minus sign before reading each
/// one, so a field written `(offset)` contributes nothing and a field written `12)` contributes 12.
pub(crate) fn read_offsets(fields: &[&str], prepend: &[i32]) -> Vec<i32> {
    let mut offsets = prepend.to_vec();
    for field in fields.iter().skip(OFFSET_FIELD) {
        let digits: String = field.chars().filter(|glyph| glyph.is_ascii_digit() || *glyph == '-').collect();
        if let Ok(id) = digits.parse::<i32>() {
            offsets.push(id);
        }
    }
    offsets
}

/// A rectangle already converted into the document's own space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

/// The conversion from the resolution a header names into this build's document space.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Geometry {
    source: (i32, i32),
}

impl Geometry {
    /// The conversion out of `source`, which is what `#RESOLUTION` named.
    pub(crate) fn new(source: (i32, i32)) -> Self {
        Self { source: (source.0.max(1), source.1.max(1)) }
    }

    /// One horizontal measurement, scaled.
    pub(crate) fn scale_x(self, value: i32) -> i32 {
        scaled(value, DEFAULT_SKIN_WIDTH, self.source.0)
    }

    /// One vertical measurement, scaled but not flipped, which is what a height is.
    pub(crate) fn scale_y(self, value: i32) -> i32 {
        scaled(value, DEFAULT_SKIN_HEIGHT, self.source.1)
    }

    /// The bottom edge of a rectangle whose top edge the document wrote at `y`.
    pub(crate) fn bottom(self, y: i32, h: i32) -> i32 {
        DEFAULT_SKIN_HEIGHT - self.scale_y(y.saturating_add(h))
    }

    /// One whole rectangle, top-left and y-down as the document wrote it, in document space.
    pub(crate) fn rect(self, x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x: self.scale_x(x), y: self.bottom(y, h), w: self.scale_x(w), h: self.scale_y(h) }
    }

    /// The rectangle of a `#DST_*` line, with the negative width and height normalisation the
    /// reference applies to the commands that ask for it.
    pub(crate) fn dst_rect(self, values: &[i32; FIELD_COUNT], normalise: bool) -> Rect {
        let (mut x, mut y, mut w, mut h) = (values[3], values[4], values[5], values[6]);
        if normalise {
            if w < 0 {
                x += w;
                w = -w;
            }
            if h < 0 {
                y += h;
                h = -h;
            }
        }
        self.rect(x, y, w, h)
    }
}

/// `value * target / source`, rounded to the nearest whole pixel.
fn scaled(value: i32, target: i32, source: i32) -> i32 {
    (f64::from(value) * f64::from(target) / f64::from(source)).round() as i32
}

/// Adds one keyframe to `destination` and fills the whole-object fields this line names.
///
/// The reference keeps blend, filter, anchor, loop point and timer on the object rather than on the
/// keyframe, and each is taken from the first line that names a value other than zero
/// (`SkinObject.setDestination`); draw conditions and offset ids are taken from the first line only.
pub(crate) fn push_keyframe(destination: &mut Destination, rect: Rect, values: &[i32; FIELD_COUNT], fields: &[&str], prepend: &[i32]) {
    let first = destination.dst.is_empty();
    destination.dst.push(Animation {
        time: Some(i64::from(values[2])),
        x: Some(rect.x),
        y: Some(rect.y),
        w: Some(rect.w),
        h: Some(rect.h),
        acc: Some(values[7]),
        a: Some(values[8]),
        r: Some(values[9]),
        g: Some(values[10]),
        b: Some(values[11]),
        angle: Some(values[14]),
        ..Animation::default()
    });
    if destination.blend == 0 {
        destination.blend = values[12];
    }
    if destination.filter == 0 {
        destination.filter = values[13];
    }
    if destination.center == 0 && values[15] > 0 && values[15] < CENTER_COUNT {
        destination.center = values[15];
    }
    if destination.loop_ms == 0 {
        destination.loop_ms = values[16];
    }
    if destination.timer.is_none() && values[17] > 0 {
        destination.timer = Some(PropertyRef::Id(values[17]));
    }
    if first {
        destination.op = [values[18], values[19], values[20]].into_iter().filter(|id| *id != 0).map(|id| DestinationOption { id, property: None }).collect();
        destination.offsets = read_offsets(fields, prepend);
    }
}

/// Puts a destination's keyframes back in time order, which is the order the reference inserts them
/// in and the order the interpolator reads them in.
pub(crate) fn sort_keyframes(destination: &mut Destination) {
    destination.dst.sort_by_key(|frame| frame.time.unwrap_or_default());
}

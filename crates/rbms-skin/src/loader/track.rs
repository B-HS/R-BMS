//! Turning a document's `destination` record into the [`DestinationTrack`] the interpolator reads.
//!
//! Two jobs live here. One is the sentinel fill: a document leaves out every field that does not
//! change, and the reference fills the first keyframe from type defaults and every later one from
//! its predecessor (`JSONSkinLoader.setDestination`). The other is turning the document's condition
//! lists and its counter-clockwise angles into the forms the interpolator expects, so that by the
//! time a keyframe reaches it, nothing is missing and nothing needs reinterpreting.

use std::collections::BTreeSet;

use crate::SkinError;
use crate::dst::{Acc, DestinationTrack, DrawCondition, Keyframe, MouseRect, SkinColor, SkinRect, TimerRef};
use crate::model::{Animation, Destination, PropertyRef};

/// The alpha and colour channels a first keyframe starts at when a document names none.
const DEFAULT_CHANNEL: i32 = 255;

/// The lowest value a colour channel may take.
const MIN_CHANNEL: i32 = 0;

/// The `acc` a keyframe carries when it shapes nothing, which leaves the object's one acceleration
/// for a later keyframe to claim.
const ACC_UNCLAIMED: i32 = 0;

/// A whole keyframe once every unset field has been filled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Filled {
    time: i64,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    clip_x: Option<i32>,
    clip_y: Option<i32>,
    clip_w: Option<i32>,
    clip_h: Option<i32>,
    acc: i32,
    a: i32,
    r: i32,
    g: i32,
    b: i32,
    angle: i32,
}

impl Filled {
    /// The values a first keyframe takes for the fields it leaves unset.
    fn first(frame: &Animation, path: &str) -> Result<Self, SkinError> {
        Ok(Self {
            time: keyframe_time(frame.time, path)?,
            x: frame.x.unwrap_or(0),
            y: frame.y.unwrap_or(0),
            w: frame.w.unwrap_or(0),
            h: frame.h.unwrap_or(0),
            clip_x: frame.clip_x,
            clip_y: frame.clip_y,
            clip_w: frame.clip_w,
            clip_h: frame.clip_h,
            acc: frame.acc.unwrap_or(0),
            a: frame.a.unwrap_or(DEFAULT_CHANNEL),
            r: frame.r.unwrap_or(DEFAULT_CHANNEL),
            g: frame.g.unwrap_or(DEFAULT_CHANNEL),
            b: frame.b.unwrap_or(DEFAULT_CHANNEL),
            angle: frame.angle.unwrap_or(0),
        })
    }

    /// The values a later keyframe takes, inheriting anything it leaves unset from `self`.
    ///
    /// Clip is the one field with no type default: an unset clip on the first keyframe means the
    /// object is not clipped, and only a keyframe that follows one with a clip inherits it.
    fn next(&self, frame: &Animation, path: &str) -> Result<Self, SkinError> {
        Ok(Self {
            time: frame.time.map_or(Ok(self.time), |time| keyframe_time(Some(time), path))?,
            x: frame.x.unwrap_or(self.x),
            y: frame.y.unwrap_or(self.y),
            w: frame.w.unwrap_or(self.w),
            h: frame.h.unwrap_or(self.h),
            clip_x: frame.clip_x.or(self.clip_x),
            clip_y: frame.clip_y.or(self.clip_y),
            clip_w: frame.clip_w.or(self.clip_w),
            clip_h: frame.clip_h.or(self.clip_h),
            acc: frame.acc.unwrap_or(self.acc),
            a: frame.a.unwrap_or(self.a),
            r: frame.r.unwrap_or(self.r),
            g: frame.g.unwrap_or(self.g),
            b: frame.b.unwrap_or(self.b),
            angle: frame.angle.unwrap_or(self.angle),
        })
    }

    /// The clip rectangle, which a document only has once all four of its edges are known.
    fn clip(&self) -> Option<SkinRect> {
        match (self.clip_x, self.clip_y, self.clip_w, self.clip_h) {
            (Some(x), Some(y), Some(w), Some(h)) => Some(SkinRect::new(x as f32, y as f32, w as f32, h as f32)),
            _ => None,
        }
    }

    /// The interpolator's keyframe.
    ///
    /// The document's `angle` turns counter-clockwise on a y-up axis, the renderer's turns
    /// clockwise on a y-down one, so the sign flips here and nowhere else.
    fn keyframe(&self) -> Keyframe {
        Keyframe {
            time_ms: self.time,
            rect: SkinRect::new(self.x as f32, self.y as f32, self.w as f32, self.h as f32),
            clip: self.clip(),
            color: SkinColor::rgba(channel(self.r), channel(self.g), channel(self.b), channel(self.a)),
            angle_deg: -(self.angle as f32),
        }
    }
}

/// Clamps a colour channel into the byte the renderer wants.
///
/// The reference lets a document write anything and hands it to a float colour, where the driver
/// clamps it; clamping here keeps the same picture without the wrap a cast would give.
fn channel(value: i32) -> u8 {
    value.clamp(MIN_CHANNEL, DEFAULT_CHANNEL) as u8
}

/// A keyframe time, held to the width the reference declares the field at.
fn keyframe_time(time: Option<i64>, path: &str) -> Result<i64, SkinError> {
    let time = time.unwrap_or(0);
    if i32::try_from(time).is_err() {
        return Err(SkinError::NumberRange { path: path.to_owned(), field: "dst.time".to_owned(), value: time.to_string() });
    }
    Ok(time)
}

/// What a track needs from its surroundings while it is being built.
pub(crate) struct TrackContext<'a> {
    /// Whether an option id is one this build implements. An unimplemented id drops its condition
    /// rather than hiding the object. It is asked only about ids `declared_options` does not carry.
    pub known_option: fn(i32) -> bool,
    /// The option ids the document declares for itself, which are known by definition.
    pub declared_options: &'a BTreeSet<i32>,
    /// Which of those the player's choices currently turn on.
    pub enabled_options: &'a BTreeSet<i32>,
    /// The name the document was read from, for error messages.
    pub path: &'a str,
    /// Set on the play screen's judge counts alone, where offsets resize without moving.
    pub relative: bool,
    /// Where a recoverable problem is recorded.
    pub warnings: &'a mut Vec<String>,
}

/// The keyframes of one destination, filled in and sorted, with the one acceleration the object
/// interpolates them by.
///
/// The acceleration is read before the sort, in the order the document wrote its keyframes: the
/// first one whose filled `acc` is not zero claims it (`SkinObject.setDestination`,
/// `this.acc == 0`). A value outside 1..=3 claims it just the same and then interpolates linearly,
/// which is what keeps a later keyframe from shaping the object.
fn keyframes(destination: &Destination, path: &str) -> Result<(Vec<Keyframe>, Acc), SkinError> {
    let mut filled: Vec<Filled> = Vec::with_capacity(destination.dst.len());
    for frame in &destination.dst {
        let next = match filled.last() {
            Some(previous) => previous.next(frame, path)?,
            None => Filled::first(frame, path)?,
        };
        filled.push(next);
    }
    let acc = filled.iter().map(|frame| frame.acc).find(|acc| *acc != ACC_UNCLAIMED).map_or(Acc::Linear, Acc::from_id);
    let mut frames: Vec<Keyframe> = filled.iter().map(Filled::keyframe).collect();
    frames.sort_by_key(|frame| frame.time_ms);
    Ok((frames, acc))
}

/// The draw conditions of one destination, integer options first and the `draw` field last, or
/// `None` when the document's own customisation choices already rule the object out.
///
/// An `op` entry with neither an id nor a property -- what a Lua boolean leaves behind -- adds no
/// condition: its zero id is dropped with every other zero.
///
/// An id the document declares for itself cannot change while the document is loaded, so it is
/// settled here rather than every frame: an unmet one drops the object and a met one leaves no
/// condition behind, which is what `Skin.prepare` does when it removes objects and empties the
/// option list of the ones it keeps. Only the ids that address running game state stay as
/// conditions.
///
/// A script a JSON document wrote as a string has been compiled into a function by the time a track
/// is built (`super::script`). One that has not is source no interpreter ever saw, and the track is
/// refused with [`SkinError::LuaUnavailable`] rather than drawn as though the condition were not
/// there.
fn draw_conditions(destination: &Destination, context: &mut TrackContext<'_>) -> Result<Option<Vec<DrawCondition>>, SkinError> {
    let declared = context.declared_options;
    let enabled = context.enabled_options;
    let known_option = context.known_option;
    let is_declared = |id: i32| declared.contains(&id.saturating_abs());
    let is_known = |id: i32| known_option(id);

    let ids: Vec<i32> = destination.op.iter().filter(|option| option.property.is_none()).map(|option| option.id).collect();
    if ids.iter().any(|id| is_declared(*id) && !crate::loader::option_holds(*id, enabled)) {
        return Ok(None);
    }
    let runtime: Vec<i32> = ids.into_iter().filter(|id| !is_declared(*id)).collect();
    let mut conditions = crate::dst::draw_conditions_from_ops(&runtime, is_known);

    let expressions = destination.op.iter().filter_map(|option| option.property.as_ref()).chain(destination.draw.iter());
    for expression in expressions {
        match expression {
            PropertyRef::Id(id) if *id == 0 => {}
            PropertyRef::Id(id) if is_declared(*id) => {
                if !crate::loader::option_holds(*id, enabled) {
                    return Ok(None);
                }
            }
            PropertyRef::Id(id) => {
                if is_known(id.saturating_abs()) {
                    conditions.push(DrawCondition::Option(*id));
                }
            }
            PropertyRef::Func(function) => conditions.push(DrawCondition::Function(*function)),
            PropertyRef::Name(name) => conditions.push(DrawCondition::Name(name.clone())),
            PropertyRef::Expr(_) => return Err(SkinError::LuaUnavailable),
        }
    }
    Ok(Some(conditions))
}

/// The timer a destination animates against.
///
/// A skin may name it with an id or hand over a function that computes it. Source nobody compiled
/// into a function names no timer the interpolator can follow, so it is reported and the animation
/// runs on the caller's clock instead.
fn timer_of(destination: &Destination, context: &mut TrackContext<'_>) -> Option<TimerRef> {
    let property = destination.timer.as_ref()?;
    let timer = property.timer();
    if timer.is_none() {
        context.warnings.push(format!(
            "{}: object {:?} names its timer with the expression {:?}, which runs on the frame clock instead",
            context.path,
            destination.id,
            property.expr().or(property.name()).unwrap_or_default()
        ));
    }
    timer
}

/// Builds one destination track, or reports that this document's own choices never draw it.
///
/// The offset list is the document's `offsets` with its single `offset` appended, which is what
/// `JSONSkinLoader.setDestination` hands the object, and it is appended even when it is zero.
pub(crate) fn build_track(destination: &Destination, context: &mut TrackContext<'_>) -> Result<Option<DestinationTrack>, SkinError> {
    let Some(draw_conditions) = draw_conditions(destination, context)? else {
        return Ok(None);
    };
    let mut offsets = destination.offsets.clone();
    offsets.push(destination.offset);
    let (frames, acc) = keyframes(destination, context.path)?;

    Ok(Some(DestinationTrack {
        timer: timer_of(destination, context),
        acc,
        loop_ms: i64::from(destination.loop_ms),
        blend: destination.blend,
        filter: destination.filter,
        center: destination.center,
        offsets,
        relative: context.relative,
        frames,
        draw_conditions,
        mouse_rect: destination.mouse_rect.map(|rect| MouseRect { x: rect.x as f32, y: rect.y as f32, w: rect.w as f32, h: rect.h as f32 }),
        stretch: destination.stretch,
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{TrackContext, build_track};
    use crate::dst::{DestinationTrack, DrawCondition, LuaFnId, TimerRef};
    use crate::model::{Animation, Destination, DestinationOption, PropertyRef};
    use crate::timer::TimerId;

    /// An option id the test context reports as one the build implements.
    const KNOWN_OPTION: i32 = 40;

    /// Builds `destination` on a screen that declares no options of its own, answering the track and
    /// the warnings it left.
    fn built(destination: &Destination) -> (DestinationTrack, Vec<String>) {
        let none = BTreeSet::new();
        let mut warnings = Vec::new();
        let mut context = TrackContext {
            known_option: |id| id == KNOWN_OPTION,
            declared_options: &none,
            enabled_options: &none,
            path: "test.luaskin",
            relative: false,
            warnings: &mut warnings,
        };
        let track = build_track(destination, &mut context).expect("nothing in the track needs compiling").expect("no option rules the object out");
        (track, warnings)
    }

    /// A destination with one keyframe and nothing else.
    fn destination() -> Destination {
        Destination { id: "object".to_owned(), dst: vec![Animation::default()], ..Destination::default() }
    }

    #[test]
    fn a_function_in_draw_becomes_a_function_condition() {
        let function = LuaFnId(12);
        let (track, warnings) = built(&Destination { draw: Some(PropertyRef::Func(function)), ..destination() });
        assert_eq!(track.draw_conditions, vec![DrawCondition::Function(function)]);
        assert!(warnings.is_empty(), "a function needs no compiling and no sandbox: {warnings:?}");
    }

    #[test]
    fn conditions_keep_the_order_the_reference_evaluates_them_in() {
        let (in_op, in_draw) = (LuaFnId(1), LuaFnId(2));
        let op = vec![
            DestinationOption { id: 0, property: Some(PropertyRef::Func(in_op)) },
            DestinationOption { id: KNOWN_OPTION, property: None },
            DestinationOption { id: 0, property: Some(PropertyRef::Name("!is_autoplay".to_owned())) },
        ];
        let (track, _) = built(&Destination { op, draw: Some(PropertyRef::Func(in_draw)), ..destination() });
        assert_eq!(
            track.draw_conditions,
            vec![
                DrawCondition::Option(KNOWN_OPTION),
                DrawCondition::Function(in_op),
                DrawCondition::Name("!is_autoplay".to_owned()),
                DrawCondition::Function(in_draw),
            ],
            "integer options first, then the properties of `op` in order, then `draw`"
        );
    }

    #[test]
    fn an_entry_a_lua_boolean_left_behind_gates_nothing() {
        let op = vec![DestinationOption::UNCONDITIONAL, DestinationOption::UNCONDITIONAL];
        let (track, warnings) = built(&Destination { op, ..destination() });
        assert!(track.draw_conditions.is_empty(), "the object draws unconditionally: {:?}", track.draw_conditions);
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_function_names_the_timer_it_computes() {
        let function = LuaFnId(5);
        let (track, warnings) = built(&Destination { timer: Some(PropertyRef::Func(function)), ..destination() });
        assert_eq!(track.timer, Some(TimerRef::Lua(function)));
        assert!(warnings.is_empty(), "a function timer is not a fallback and says nothing: {warnings:?}");
    }

    #[test]
    fn an_id_still_names_its_timer() {
        let (track, _) = built(&Destination { timer: Some(PropertyRef::Id(41)), ..destination() });
        assert_eq!(track.timer, Some(TimerRef::Id(TimerId(41))));
    }

    #[test]
    fn text_in_a_timer_field_names_no_timer_and_says_so() {
        for property in [PropertyRef::Name("main_state.timer(41)".to_owned()), PropertyRef::Expr("main_state.timer(41)".to_owned())] {
            let (track, warnings) = built(&Destination { timer: Some(property), ..destination() });
            assert_eq!(track.timer, None, "a timer has no names, so text is source nobody compiled");
            assert_eq!(warnings.len(), 1);
            assert!(warnings[0].contains("main_state.timer(41)") && warnings[0].contains("frame clock"), "warned {:?}", warnings[0]);
        }
    }
}

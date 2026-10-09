//! Turning a document's `destination` record into the [`DestinationTrack`] the interpolator reads.
//!
//! Two jobs live here. One is the sentinel fill: a document leaves out every field that does not
//! change, and the reference fills the first keyframe from type defaults and every later one from
//! its predecessor (`JSONSkinLoader.setDestination`). The other is turning the document's condition
//! lists and its counter-clockwise angles into the forms the interpolator expects, so that by the
//! time a keyframe reaches it, nothing is missing and nothing needs reinterpreting.

use crate::SkinError;
use crate::dst::{Acc, DestinationTrack, DrawCondition, Keyframe, MouseRect, OpLists, SkinColor, SkinRect, TimerRef, draw_conditions_from_ops};
use crate::model::{Animation, Destination, PropertyRef};
use crate::property::{NameSpace, reference_implements};

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
    /// The name the document was read from, for error messages.
    pub path: &'a str,
    /// Set on the play screen's judge counts alone, where offsets resize without moving.
    pub relative: bool,
    /// Where a recoverable problem is recorded.
    pub warnings: &'a mut Vec<String>,
}

/// One destination, assembled, beside the part of its `op` list no frame ever evaluates.
#[derive(Debug, Clone)]
pub(crate) struct BuiltTrack {
    pub track: DestinationTrack,
    /// The ids of the `op` list no built-in property answers, sign included (`SkinObject.dstop`).
    /// They are the skin's own options, and whoever prepares the skin checks them once against the
    /// choices its customisation rows carry (`Skin.prepare`).
    pub options: Vec<i32>,
}

/// Whether the reference has a built-in boolean property under `id`, whichever sign the skin wrote
/// it with (`BooleanPropertyFactory.getBooleanProperty(int)`).
///
/// This is the reference's own list and not a question for the host: an id that is on it is a
/// condition the running game answers, and an id that is not can only be one of the skin's own
/// options.
fn is_builtin_option(id: i32) -> bool {
    reference_implements(NameSpace::Boolean, id)
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

/// The conditions of one destination: the ones a frame evaluates, in the order it evaluates them,
/// and the skin's own options beside them.
///
/// The frame's list is every integer of `op` a built-in property answers, then every property of
/// `op` -- a function, a name -- in the order the document wrote them, then `draw`
/// (`SkinObject.setDrawCondition` followed by `addDrawCondition`, with `draw` appended last by
/// `JSONSkinLoader.setDestination`). All of them must hold, and a frame stops at the first that does
/// not, so a function placed in `op` or `draw` is only called while every built-in id of `op`
/// holds, wherever in the list the document wrote those.
///
/// An integer of `op` no built-in property answers is one of the skin's own options and goes to the
/// other list. A number in `draw`, or a property spelled as a number, is not: the reference looks it
/// up as a property and, finding none, leaves the object without that condition. An `op` entry with
/// neither an id nor a property -- what a Lua boolean leaves behind -- adds nothing either way.
///
/// The reference registers an object's conditions as it reads the object's first keyframe, so a
/// destination with no keyframe registers none of either kind.
///
/// A script a JSON document wrote as a string has been compiled into a function by the time a track
/// is built (`super::script`). One that has not is source no interpreter ever saw, and the track is
/// refused with [`SkinError::LuaUnavailable`] rather than drawn as though the condition were not
/// there.
fn conditions(destination: &Destination) -> Result<OpLists, SkinError> {
    if destination.dst.is_empty() {
        return Ok(OpLists::default());
    }
    let ids: Vec<i32> = destination.op.iter().filter(|option| option.property.is_none()).map(|option| option.id).collect();
    let mut lists = draw_conditions_from_ops(&ids, is_builtin_option);

    let properties = destination.op.iter().filter_map(|option| option.property.as_ref()).chain(destination.draw.iter());
    for property in properties {
        match property {
            PropertyRef::Id(id) => {
                if is_builtin_option(*id) {
                    lists.conditions.push(DrawCondition::Option(*id));
                }
            }
            PropertyRef::Func(function) => lists.conditions.push(DrawCondition::Function(*function)),
            PropertyRef::Name(name) => lists.conditions.push(DrawCondition::Name(name.clone())),
            PropertyRef::Expr(_) => return Err(SkinError::LuaUnavailable),
        }
    }
    Ok(lists)
}

/// The timer a destination animates against.
///
/// A skin may name it with an id or hand over a function that computes it. A negative id names no
/// timer, which is not a fault and is not reported. Source nobody compiled into a function names no
/// timer the interpolator can follow either, so that is reported and the animation runs on the
/// caller's clock instead.
fn timer_of(destination: &Destination, context: &mut TrackContext<'_>) -> Option<TimerRef> {
    let property = destination.timer.as_ref()?;
    let timer = property.timer();
    if timer.is_none() && property.id().is_none() {
        context.warnings.push(format!(
            "{}: object {:?} names its timer with the expression {:?}, which runs on the frame clock instead",
            context.path,
            destination.id,
            property.expr().or(property.name()).unwrap_or_default()
        ));
    }
    timer
}

/// Builds one destination track.
///
/// The offset list is the document's `offsets` with its single `offset` appended, which is what
/// `JSONSkinLoader.setDestination` hands the object, and it is appended even when it is zero.
pub(crate) fn build_track(destination: &Destination, context: &mut TrackContext<'_>) -> Result<BuiltTrack, SkinError> {
    let OpLists { conditions: draw_conditions, options } = conditions(destination)?;
    let mut offsets = destination.offsets.clone();
    offsets.push(destination.offset);
    let (frames, acc) = keyframes(destination, context.path)?;

    let track = DestinationTrack {
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
    };
    Ok(BuiltTrack { track, options })
}

#[cfg(test)]
mod tests {
    use super::{BuiltTrack, TrackContext, build_track};
    use crate::dst::{DrawCondition, LuaFnId, TimerRef};
    use crate::model::{Animation, Destination, DestinationOption, PropertyRef};
    use crate::property::generated::{OPTION_AUTOPLAYON, OPTION_BGAOFF};
    use crate::timer::TimerId;

    /// An id the reference has no built-in option under, which only a skin's own customisation rows
    /// can therefore give a meaning to.
    const SKIN_OPTION: i32 = 900;

    /// Builds `destination`, answering what it came to and the warnings it left.
    fn built(destination: &Destination) -> (BuiltTrack, Vec<String>) {
        let mut warnings = Vec::new();
        let mut context = TrackContext { path: "test.luaskin", relative: false, warnings: &mut warnings };
        let built = build_track(destination, &mut context).expect("nothing in the track needs compiling");
        (built, warnings)
    }

    /// A destination with one keyframe and nothing else.
    fn destination() -> Destination {
        Destination { id: "object".to_owned(), dst: vec![Animation::default()], ..Destination::default() }
    }

    /// An `op` list of plain ids.
    fn ids(ids: &[i32]) -> Vec<DestinationOption> {
        ids.iter().map(|id| DestinationOption { id: *id, property: None }).collect()
    }

    #[test]
    fn a_function_in_draw_becomes_a_function_condition() {
        let function = LuaFnId(12);
        let (built, warnings) = built(&Destination { draw: Some(PropertyRef::Func(function)), ..destination() });
        assert_eq!(built.track.draw_conditions, vec![DrawCondition::Function(function)]);
        assert!(warnings.is_empty(), "a function needs no compiling and no sandbox: {warnings:?}");
    }

    #[test]
    fn conditions_keep_the_order_the_reference_evaluates_them_in() {
        let (in_op, in_draw) = (LuaFnId(1), LuaFnId(2));
        let op = vec![
            DestinationOption { id: 0, property: Some(PropertyRef::Func(in_op)) },
            DestinationOption { id: OPTION_BGAOFF, property: None },
            DestinationOption { id: 0, property: Some(PropertyRef::Name("!is_autoplay".to_owned())) },
            DestinationOption { id: -OPTION_AUTOPLAYON, property: None },
        ];
        let (built, _) = built(&Destination { op, draw: Some(PropertyRef::Func(in_draw)), ..destination() });
        assert_eq!(
            built.track.draw_conditions,
            vec![
                DrawCondition::Option(OPTION_BGAOFF),
                DrawCondition::Option(-OPTION_AUTOPLAYON),
                DrawCondition::Function(in_op),
                DrawCondition::Name("!is_autoplay".to_owned()),
                DrawCondition::Function(in_draw),
            ],
            "every built-in id of `op` first, then the properties of `op` in order, then `draw`"
        );
        assert!(built.options.is_empty());
    }

    #[test]
    fn an_id_no_built_in_property_answers_is_one_of_the_skins_own_options() {
        let op = ids(&[SKIN_OPTION, OPTION_BGAOFF, -SKIN_OPTION, 0, SKIN_OPTION]);
        let (built, warnings) = built(&Destination { op, ..destination() });
        assert_eq!(built.track.draw_conditions, vec![DrawCondition::Option(OPTION_BGAOFF)], "only the built-in id is left for a frame to ask");
        assert_eq!(built.options, vec![SKIN_OPTION, -SKIN_OPTION], "the other is kept under each sign it was written with, once, and the zero is dropped");
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_number_in_draw_is_a_property_or_nothing_and_never_one_of_the_skins_own_options() {
        let (known, _) = built(&Destination { draw: Some(PropertyRef::Id(-OPTION_BGAOFF)), ..destination() });
        assert_eq!(known.track.draw_conditions, vec![DrawCondition::Option(-OPTION_BGAOFF)]);

        let (unknown, _) = built(&Destination { draw: Some(PropertyRef::Id(SKIN_OPTION)), ..destination() });
        assert!(unknown.track.draw_conditions.is_empty(), "the reference finds no property and leaves the object unconditional");
        assert!(unknown.options.is_empty(), "and `draw` never reaches the option list");
    }

    #[test]
    fn a_destination_with_no_keyframe_registers_no_condition() {
        let bare = Destination { dst: Vec::new(), op: ids(&[SKIN_OPTION, OPTION_BGAOFF]), draw: Some(PropertyRef::Func(LuaFnId(3))), ..destination() };
        let (built, _) = built(&bare);
        assert!(built.track.draw_conditions.is_empty(), "conditions are registered as the first keyframe is read");
        assert!(built.options.is_empty(), "so nothing can remove the object for its options either");
    }

    #[test]
    fn an_entry_a_lua_boolean_left_behind_gates_nothing() {
        let op = vec![DestinationOption::UNCONDITIONAL, DestinationOption::UNCONDITIONAL];
        let (built, warnings) = built(&Destination { op, ..destination() });
        assert!(built.track.draw_conditions.is_empty(), "the object draws unconditionally: {:?}", built.track.draw_conditions);
        assert!(built.options.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_function_names_the_timer_it_computes() {
        let function = LuaFnId(5);
        let (built, warnings) = built(&Destination { timer: Some(PropertyRef::Func(function)), ..destination() });
        assert_eq!(built.track.timer, Some(TimerRef::Lua(function)));
        assert!(warnings.is_empty(), "a function timer is not a fallback and says nothing: {warnings:?}");
    }

    #[test]
    fn an_id_still_names_its_timer() {
        let (built, _) = built(&Destination { timer: Some(PropertyRef::Id(41)), ..destination() });
        assert_eq!(built.track.timer, Some(TimerRef::Id(TimerId(41))));
    }

    #[test]
    fn a_negative_id_names_no_timer_and_zero_names_timer_zero() {
        let (negative, warnings) = built(&Destination { timer: Some(PropertyRef::Id(-1)), ..destination() });
        assert_eq!(negative.track.timer, None, "the object animates on the scene clock");
        assert!(warnings.is_empty(), "which is what the skin asked for, so nothing is reported: {warnings:?}");

        let (zero, _) = built(&Destination { timer: Some(PropertyRef::Id(0)), ..destination() });
        assert_eq!(zero.track.timer, Some(TimerRef::Id(TimerId(0))), "zero is a timer like any other, and one nothing switches on");
    }

    #[test]
    fn text_in_a_timer_field_names_no_timer_and_says_so() {
        for property in [PropertyRef::Name("main_state.timer(41)".to_owned()), PropertyRef::Expr("main_state.timer(41)".to_owned())] {
            let (built, warnings) = built(&Destination { timer: Some(property), ..destination() });
            assert_eq!(built.track.timer, None, "a timer has no names, so text is source nobody compiled");
            assert_eq!(warnings.len(), 1);
            assert!(warnings[0].contains("main_state.timer(41)") && warnings[0].contains("frame clock"), "warned {:?}", warnings[0]);
        }
    }
}

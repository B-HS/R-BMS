//! Turning a document's `destination` record into the [`DestinationTrack`] the interpolator reads.
//!
//! Two jobs live here. One is the sentinel fill: a document leaves out every field that does not
//! change, and the reference fills the first keyframe from type defaults and every later one from
//! its predecessor (`JSONSkinLoader.setDestination`). The other is turning the document's condition
//! lists and its counter-clockwise angles into the forms the interpolator expects, so that by the
//! time a keyframe reaches it, nothing is missing and nothing needs reinterpreting.

use std::collections::BTreeSet;

use crate::SkinError;
use crate::dst::{Acc, DestinationTrack, DrawCondition, Keyframe, MouseRect, SkinColor, SkinRect};
use crate::model::{Animation, Destination, PropertyRef};
use crate::timer::TimerId;

/// The alpha and colour channels a first keyframe starts at when a document names none.
const DEFAULT_CHANNEL: i32 = 255;

/// The lowest value a colour channel may take.
const MIN_CHANNEL: i32 = 0;

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
            acc: Acc::from_id(self.acc),
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

/// Compiles one expression-typed field, or reports that this build cannot.
#[cfg(feature = "lua")]
fn compile(lua: Option<&crate::lua::LuaSandbox>, source: &str) -> Result<crate::dst::LuaExprId, SkinError> {
    match lua {
        Some(lua) => lua.compile(source),
        None => Err(SkinError::LuaUnavailable),
    }
}

/// The build without Lua refuses any document that carries an expression, rather than reading it as
/// a silent false.
#[cfg(not(feature = "lua"))]
fn compile(_lua: Option<&()>, _source: &str) -> Result<crate::dst::LuaExprId, SkinError> {
    Err(SkinError::LuaUnavailable)
}

/// The sandbox a build with Lua compiles expressions into.
#[cfg(feature = "lua")]
pub(crate) type Sandbox = crate::lua::LuaSandbox;

/// The placeholder a build without Lua carries in the sandbox's place.
#[cfg(not(feature = "lua"))]
pub(crate) type Sandbox = ();

/// The keyframes of one destination, filled in and sorted.
fn keyframes(destination: &Destination, path: &str) -> Result<Vec<Keyframe>, SkinError> {
    let mut filled: Vec<Filled> = Vec::with_capacity(destination.dst.len());
    for frame in &destination.dst {
        let next = match filled.last() {
            Some(previous) => previous.next(frame, path)?,
            None => Filled::first(frame, path)?,
        };
        filled.push(next);
    }
    let mut frames: Vec<Keyframe> = filled.iter().map(Filled::keyframe).collect();
    frames.sort_by_key(|frame| frame.time_ms);
    Ok(frames)
}

/// The draw conditions of one destination, integer options first and the `draw` field last, or
/// `None` when the document's own customisation choices already rule the object out.
///
/// An id the document declares for itself cannot change while the document is loaded, so it is
/// settled here rather than every frame: an unmet one drops the object and a met one leaves no
/// condition behind, which is what `Skin.prepare` does when it removes objects and empties the
/// option list of the ones it keeps. Only the ids that address running game state stay as
/// conditions.
fn draw_conditions(destination: &Destination, lua: Option<&Sandbox>, context: &mut TrackContext<'_>) -> Result<Option<Vec<DrawCondition>>, SkinError> {
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
            PropertyRef::Expr(source) => conditions.push(DrawCondition::Lua(compile(lua, source)?)),
        }
    }
    Ok(Some(conditions))
}

/// The timer a destination animates against.
///
/// A document may name it with an expression. The interpolator addresses timers by id alone, so an
/// expression-named timer is reported and the animation runs on the caller's clock instead.
fn timer_of(destination: &Destination, context: &mut TrackContext<'_>) -> Option<TimerId> {
    match destination.timer.as_ref()? {
        PropertyRef::Id(id) => Some(TimerId(*id)),
        PropertyRef::Expr(source) => {
            context.warnings.push(format!(
                "{}: object {:?} names its timer with the expression {source:?}, which runs on the frame clock instead",
                context.path, destination.id
            ));
            None
        }
    }
}

/// Builds one destination track, or reports that this document's own choices never draw it.
///
/// The offset list is the document's `offsets` with its single `offset` appended, which is what
/// `JSONSkinLoader.setDestination` hands the object, and it is appended even when it is zero.
pub(crate) fn build_track(destination: &Destination, lua: Option<&Sandbox>, context: &mut TrackContext<'_>) -> Result<Option<DestinationTrack>, SkinError> {
    let Some(draw_conditions) = draw_conditions(destination, lua, context)? else {
        return Ok(None);
    };
    let mut offsets = destination.offsets.clone();
    offsets.push(destination.offset);

    Ok(Some(DestinationTrack {
        timer: timer_of(destination, context),
        loop_ms: i64::from(destination.loop_ms),
        blend: destination.blend,
        filter: destination.filter,
        center: destination.center,
        offsets,
        relative: context.relative,
        frames: keyframes(destination, context.path)?,
        draw_conditions,
        mouse_rect: destination.mouse_rect.map(|rect| MouseRect { x: rect.x as f32, y: rect.y as f32, w: rect.w as f32, h: rect.h as f32 }),
        stretch: destination.stretch,
    }))
}

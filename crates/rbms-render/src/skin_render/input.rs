//! Pointer input against a drawn skin: which object a press lands on, and which slider a drag moves.
//!
//! A skin takes the pointer the way it draws, by walking its objects, only from the top down
//! (`Skin.mousePressed` and `Skin.mouseDragged`): a press goes to the topmost drawn object that has
//! something to do with it, and a drag goes to sliders alone. Hovering is not an event at all -- a
//! destination's `mouseRect` reads the pointer the frame carries while it is prepared, and an object
//! the pointer is not over is simply not drawn, which is also what keeps it from being pressed.
//!
//! Nothing here runs anything. Judging an event is plain geometry over where the last prepared frame
//! put each object ([`SkinInputMap`]), and what comes out is a list of [`SkinAction`]s: the event to
//! run with its argument, or the value to write. Running them is the host's, because an event may be
//! a function in the skin's interpreter and a write may move a setting.
//!
//! The song wheel takes a press like any other object, where it stands among them, and offers it to
//! the bars of its `clickable` slots in the order the document listed those (`SkinBar.mousePressed`,
//! which hands the press to `BarRenderer.mousePressed`). A bar is pressed where its slot's
//! destination puts it, not where a slide is carrying it, and what comes out is which bar it was:
//! opening it, playing it or closing the folder is the browser's.
//!
//! Two things the reference does with the pointer are not judged here:
//!
//! - The wheel is no object's. The reference adds it up and the screen that is up reads the total
//!   (`BMSPlayerInputProcessor.getScroll`), so [`SkinPointer::Scroll`] reaches no object and is left
//!   to the screen.
//! - An editable text that is being typed into is confirmed by a press outside it
//!   (`SkinTextInput.commitIfOutside`). A press on one is reported as [`SkinAction::FocusText`] and
//!   stops the walk as it does in the reference; the typing itself is not here.
//!
//! The pointer is in document coordinates, y up, the space every region is in. The reference holds
//! it as whole pixels of the output resolution (`KeyBoardInputProcesseor.touchDown`); here it is
//! whatever fraction of a document pixel the window reported.

use rbms_skin::dst::{LuaFnId, SkinRect};
use rbms_skin::loader::LoadedSkin;
use rbms_skin::model::{EventRef, FloatWriterRef, SliderDef};
use rbms_skin::property::{NameSpace, id_of_name, reference_writes};

use super::frame::PreparedFrame;
use super::object::{Body, SkinObject};
use super::text_input::input_bounds;
use super::{SkinScreen, songlist};

/// The `click` that hands an event the direction of the button that was pressed.
const CLICK_BUTTON: i32 = 0;

/// The `click` that hands an event the opposite of the pressed button's direction.
const CLICK_BUTTON_REVERSED: i32 = 1;

/// The `click` that hands an event which half of the object was pressed, left or right.
const CLICK_HORIZONTAL_HALF: i32 = 2;

/// The `click` that hands an event which half of the object was pressed, lower or upper.
const CLICK_VERTICAL_HALF: i32 = 3;

/// The argument of a press that asks for the next of something.
const ARGUMENT_NEXT: i32 = 1;

/// The argument of a press that asks for the previous of something.
const ARGUMENT_PREVIOUS: i32 = -1;

/// A slider whose handle moves up as its value grows.
const SLIDER_UP: i32 = 0;

/// A slider whose handle moves right as its value grows.
const SLIDER_RIGHT: i32 = 1;

/// A slider whose handle moves down as its value grows.
const SLIDER_DOWN: i32 = 2;

/// A slider whose handle moves left as its value grows.
const SLIDER_LEFT: i32 = 3;

/// How near an end of its travel, in document pixels, a slider is taken to be at that end
/// (`SkinSlider.mousePressed`).
const SLIDER_SNAP: f32 = 1.0;

/// The value of a slider at the start of its travel, and at the end of it.
const SLIDER_EMPTY: f32 = 0.0;
const SLIDER_FULL: f32 = 1.0;

/// Which button a press was made with, numbered as the reference's click table is indexed
/// (`SkinObject.mousePressed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinPointerButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

impl SkinPointerButton {
    /// The direction a press with this button asks for (`buttonEvents = {1, -1, 1, 1, -1}`).
    pub const fn argument(self) -> i32 {
        match self {
            SkinPointerButton::Left | SkinPointerButton::Middle | SkinPointerButton::Back => ARGUMENT_NEXT,
            SkinPointerButton::Right | SkinPointerButton::Forward => ARGUMENT_PREVIOUS,
        }
    }
}

/// One thing the pointer did over a skin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkinPointer {
    /// A button went down.
    Press(SkinPointerButton),
    /// The pointer moved with a button held.
    Drag,
    /// The wheel turned by this many lines, positive towards the player. No object takes it.
    Scroll(f32),
}

/// What a pressed object runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinEvent {
    /// An event id, for the host's [`exec_event`](rbms_skin::property::SkinHost::exec_event). The
    /// second argument of an event run by a press is always zero (`Event.exec(state, arg1)`).
    Id(i32),
    /// A function in the skin's interpreter, called with the one argument.
    Function(LuaFnId),
}

/// Where a moved slider writes its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinWriter {
    /// A rate id, for the host's [`write_rate`](rbms_skin::property::SkinHost::write_rate).
    Rate(i32),
    /// A function in the skin's interpreter, called with the value.
    Function(LuaFnId),
}

/// One thing the host is to do because of a pointer event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkinAction {
    /// Run an event with this argument.
    Event { event: SkinEvent, argument: i32 },
    /// Write this value.
    Write { writer: SkinWriter, value: f32 },
    /// Start typing into the editable text that is the screen's object number `object`.
    FocusText { object: usize },
    /// A bar of the song wheel was pressed with the left button (`MusicSelector.select`): a bar that
    /// opens is opened, and any other starts play -- of the bar under the cursor, whichever bar was
    /// pressed, because that is the one the reference's browser goes on to read.
    ///
    /// `bar` is the bar's place in the list the frame was prepared with and `offset` is how many bars
    /// below the cursor's it was, negative above it, for a host whose cursor has moved since. `slot`
    /// is the slot of the wheel it was drawn on.
    SelectBar { slot: usize, offset: i32, bar: usize },
    /// A bar of the song wheel was pressed with any other button (`BarManager.close`): the folder
    /// that is open is closed, and at the top of the list the sort order moves on instead.
    CloseBar,
}

/// What one object does with the pointer, settled when its screen is built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Interaction {
    /// Nothing: a press passes through it to whatever is drawn beneath.
    None,
    /// An image or an image set with an `act`.
    Click { event: SkinEvent, kind: i32 },
    /// A slider with somewhere to write.
    Slide { direction: i32, range: f32, writer: SkinWriter },
    /// An editable text.
    Edit { align: i32 },
    /// A song wheel, whose bars take the press. Which bars those are is settled frame by frame, so a
    /// frame's targets carry one [`Interaction::Bar`] for each in its place.
    Bars,
    /// One bar of a song wheel as a prepared frame left it. Only a frame's targets hold one.
    Bar { slot: usize, offset: i32, bar: usize },
}

/// What a click event reference runs, or `None` when it is source the loader never compiled.
///
/// A number is always an event: one the reference defines nothing for is still run, as a request
/// its screen ignores (`EventFactory.getEvent(int)`). A name is one the loader found in the
/// reference's own table.
fn event_of(act: &EventRef) -> Option<SkinEvent> {
    match act {
        EventRef::Id(id) => Some(SkinEvent::Id(*id)),
        EventRef::Lua(function) => Some(SkinEvent::Function(*function)),
        EventRef::Name(name) => id_of_name(NameSpace::Event, name).map(SkinEvent::Id),
        EventRef::Script(_) => None,
    }
}

/// A rate id as somewhere to write, or `None` when the reference has no writer under it
/// (`FloatPropertyFactory.getRateWriter`).
fn rate_writer(id: i32) -> Option<SkinWriter> {
    reference_writes(NameSpace::Rate, id).then_some(SkinWriter::Rate(id))
}

/// Where a slider writes, by the three ways the reference builds one
/// (`JsonSkinObjectLoader`, the `slider` branch): a slider with a `value` writes to its `event` and
/// nowhere when it has none; one that scales a number itself never writes; any other writes to its
/// own `type` when it is `changeable` and the reference can write that rate.
fn slider_writer(def: &SliderDef) -> Option<SkinWriter> {
    if def.value.is_some() {
        return match def.event.as_ref()? {
            FloatWriterRef::Id(id) => rate_writer(*id),
            FloatWriterRef::Lua(function) => Some(SkinWriter::Function(*function)),
            FloatWriterRef::Name(name) => id_of_name(NameSpace::Rate, name).and_then(rate_writer),
            FloatWriterRef::Script(_) => None,
        };
    }
    if def.is_ref_num || !def.changeable {
        return None;
    }
    rate_writer(def.slider_type)
}

/// What the object built from the destination called `id` does with the pointer.
///
/// Only an image and an image set are given their `act` (`JsonSkinObjectLoader`), and the image is
/// looked for first, as it is when the object is built.
fn interaction_of(skin: &LoadedSkin, id: &str, body: &Body) -> Interaction {
    let def = &skin.def;
    match body {
        Body::Image(_) | Body::Movie(_) => {
            let declared = def
                .image
                .iter()
                .find(|image| image.id == id)
                .map(|image| (image.act.as_ref(), image.click))
                .or_else(|| def.imageset.iter().find(|set| set.id == id).map(|set| (set.act.as_ref(), set.click)));
            match declared {
                Some((Some(act), kind)) => event_of(act).map_or(Interaction::None, |event| Interaction::Click { event, kind }),
                _ => Interaction::None,
            }
        }
        Body::Slider(slider) => def
            .slider
            .iter()
            .find(|declared| declared.id == id)
            .and_then(slider_writer)
            .map_or(Interaction::None, |writer| Interaction::Slide { direction: slider.direction, range: slider.range, writer }),
        Body::TextInput(input) => Interaction::Edit { align: input.shown().align },
        Body::SongList(_) => Interaction::Bars,
        _ => Interaction::None,
    }
}

/// What each of a screen's objects does with the pointer, by the same index as `objects`.
///
/// `kept` names, for each object, the destination of `skin` it was built from.
pub(crate) fn interactions(skin: &LoadedSkin, kept: &[usize], objects: &[SkinObject]) -> Vec<Interaction> {
    objects
        .iter()
        .zip(kept)
        .map(|(object, destination)| skin.destinations.get(*destination).map_or(Interaction::None, |named| interaction_of(skin, &named.id, &object.body)))
        .collect()
}

/// Whether a point is inside a region, edges included (`Rectangle.contains`, and the test
/// `SkinObject.mousePressed` writes out). A region of negative extent holds no point.
fn holds(region: SkinRect, x: f32, y: f32) -> bool {
    region.x <= x && region.x + region.w >= x && region.y <= y && region.y + region.h >= y
}

/// The value a slider is set to when its travel is pressed `travelled` from its start, of `range` in
/// all: an end when the press is within [`SLIDER_SNAP`] of it, the start winning when both are.
fn slider_value(travelled: f32, range: f32) -> f32 {
    if travelled.abs() < SLIDER_SNAP {
        return SLIDER_EMPTY;
    }
    if (travelled - range).abs() < SLIDER_SNAP {
        return SLIDER_FULL;
    }
    travelled / range
}

/// One drawn object that does something with the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Target {
    /// Which of its screen's objects this is.
    object: usize,
    /// Where the last prepared frame put it: its region after its offsets and before any stretch.
    region: SkinRect,
    interaction: Interaction,
}

impl Target {
    /// What a press at `(x, y)` makes of this object, or `None` when the press is not its to take
    /// (`SkinObject.mousePressed`, `SkinSlider.mousePressed`, and the editable text test of
    /// `Skin.mousePressed`).
    fn pressed(&self, button: SkinPointerButton, x: f32, y: f32) -> Option<SkinAction> {
        match self.interaction {
            Interaction::None | Interaction::Bars => None,
            Interaction::Bar { slot, offset, bar } => {
                let action = if button == SkinPointerButton::Left { SkinAction::SelectBar { slot, offset, bar } } else { SkinAction::CloseBar };
                holds(self.region, x, y).then_some(action)
            }
            Interaction::Click { event, kind } => {
                let region = self.region;
                let argument = match kind {
                    CLICK_BUTTON => button.argument(),
                    CLICK_BUTTON_REVERSED => -button.argument(),
                    CLICK_HORIZONTAL_HALF if x >= region.x + region.w / 2.0 => ARGUMENT_NEXT,
                    CLICK_HORIZONTAL_HALF => ARGUMENT_PREVIOUS,
                    CLICK_VERTICAL_HALF if y >= region.y + region.h / 2.0 => ARGUMENT_NEXT,
                    CLICK_VERTICAL_HALF => ARGUMENT_PREVIOUS,
                    _ => return None,
                };
                holds(region, x, y).then_some(SkinAction::Event { event, argument })
            }
            Interaction::Slide { .. } => self.slid(x, y),
            Interaction::Edit { align } => holds(input_bounds(self.region, align), x, y).then_some(SkinAction::FocusText { object: self.object }),
        }
    }

    /// The write a pointer at `(x, y)` makes of this object when it is a slider and the pointer is
    /// on its travel (`SkinSlider.mousePressed`).
    ///
    /// The travel starts at the region's own corner and runs `range` along the slider's direction;
    /// across it, it is as wide as the region. The handle's own extent along the direction is no
    /// part of it.
    fn slid(&self, x: f32, y: f32) -> Option<SkinAction> {
        let Interaction::Slide { direction, range, writer } = self.interaction else {
            return None;
        };
        let region = self.region;
        let across_x = region.x <= x && region.x + region.w >= x;
        let across_y = region.y <= y && region.y + region.h >= y;
        let travelled = match direction {
            SLIDER_UP if across_x && region.y <= y && region.y + range >= y => y - region.y,
            SLIDER_RIGHT if across_y && region.x <= x && region.x + range >= x => x - region.x,
            SLIDER_DOWN if across_x && region.y - range <= y && region.y >= y => region.y - y,
            SLIDER_LEFT if across_y && region.x >= x && region.x - range <= x => region.x - x,
            _ => return None,
        };
        Some(SkinAction::Write { writer, value: slider_value(travelled, range) })
    }
}

/// Where one prepared frame left everything that takes the pointer, which is all a pointer event is
/// judged against.
///
/// It is made from a frame and outlives it: the host keeps the map of the frame it last drew and
/// judges the events that arrive before the next one against it, as the reference judges a press
/// against the regions its last `prepare` left.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkinInputMap {
    /// The drawn objects that take the pointer, in the order they are drawn. A song wheel is one
    /// target for each of its bars that takes a press, the bar offered the press first coming last.
    targets: Vec<Target>,
}

impl SkinInputMap {
    /// Whether no drawn object takes the pointer at all.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// How many drawn objects take the pointer.
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// What a button going down at `at` does (`Skin.mousePressed`): the objects are offered the
    /// press from the one drawn last to the one drawn first, and the first to take it is the only
    /// one that does. An object with nothing to do with a press does not stop it reaching the ones
    /// beneath.
    ///
    /// Empty when no object took the press.
    pub fn press(&self, button: SkinPointerButton, at: (f32, f32)) -> Vec<SkinAction> {
        let (x, y) = at;
        self.targets.iter().rev().find_map(|target| target.pressed(button, x, y)).into_iter().collect()
    }

    /// What the pointer moving to `at` with a button held does (`Skin.mouseDragged`): only sliders
    /// are offered it, from the top down, and the first whose travel it is on is set.
    pub fn drag(&self, at: (f32, f32)) -> Vec<SkinAction> {
        let (x, y) = at;
        self.targets.iter().rev().find_map(|target| target.slid(x, y)).into_iter().collect()
    }

    /// Whether `at` is where the editable text that is the screen's object number `object` takes the
    /// pointer, as the last prepared frame left it (`SkinText.getInputBounds`). A frame that did not
    /// draw it, and an object that is no editable text, take nothing. A press outside it ends the
    /// typing (`SkinTextInput.commitIfOutside`).
    pub fn text_holds(&self, object: usize, at: (f32, f32)) -> bool {
        self.targets.iter().find(|target| target.object == object).is_some_and(|target| match target.interaction {
            Interaction::Edit { align } => holds(input_bounds(target.region, align), at.0, at.1),
            _ => false,
        })
    }

    /// What one pointer event at `at` does. A turn of the wheel does nothing here.
    pub fn pointer(&self, event: SkinPointer, at: (f32, f32)) -> Vec<SkinAction> {
        match event {
            SkinPointer::Press(button) => self.press(button, at),
            SkinPointer::Drag => self.drag(at),
            SkinPointer::Scroll(_) => Vec::new(),
        }
    }
}

impl SkinScreen {
    /// Where `prepared` left every object of this screen that takes the pointer.
    ///
    /// Only an object the frame draws is in it: one its conditions, its timer, its own value or the
    /// pointer rectangle it is shown under left out of the frame takes nothing, exactly as the
    /// reference skips an object whose `draw` its last `prepare` cleared.
    ///
    /// A song wheel's bars are read from the wheel itself, which holds them as the frame prepared
    /// last left them, so `prepared` has to be that frame.
    pub fn input_map(&self, prepared: &PreparedFrame) -> SkinInputMap {
        let mut targets = Vec::new();
        for (object, (interaction, placed)) in self.interactions.iter().zip(prepared.placed()).enumerate() {
            match (interaction, placed, self.objects.get(object).map(|object| &object.body)) {
                (Interaction::None, _, _) | (_, None, _) => {}
                (Interaction::Bars, Some(_), Some(Body::SongList(wheel))) => {
                    let bars = songlist::bar_targets(wheel).into_iter().rev();
                    targets.extend(bars.map(|bar| Target {
                        object,
                        region: bar.region,
                        interaction: Interaction::Bar { slot: bar.slot, offset: bar.offset, bar: bar.bar },
                    }));
                }
                (interaction, Some(resolved), _) => targets.push(Target { object, region: resolved.rect, interaction: *interaction }),
            }
        }
        SkinInputMap { targets }
    }

    /// Judges one pointer event at `at`, in document coordinates, against this screen's objects as
    /// `prepared` left them, and answers what the host is to do about it. Empty when no object took
    /// the event.
    pub fn pointer(&self, prepared: &PreparedFrame, at: (f32, f32), event: SkinPointer) -> Vec<SkinAction> {
        self.input_map(prepared).pointer(event, at)
    }
}

#[cfg(test)]
mod tests;

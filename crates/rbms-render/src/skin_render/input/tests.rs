//! Unit tests for pointer input: the geometry of a press and a drag over hand-placed targets, and
//! what a small Lua skin written out here turns into once it is loaded, built and prepared.

use std::path::{Path, PathBuf};

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::lua_skin::{LuaSkinOptions, load_lua_skin};
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig};
use rbms_skin::lua::LuaBudget;
use rbms_skin::property::MapHost;
use rbms_skin::property::generated::{RATE_MASTERVOLUME, RATE_MUSIC_PROGRESS};
use rbms_skin::timer::TimerState;

use super::super::frame::PreparedFrame;
use super::super::{FrameData, SkinAssets, SkinFrame, SkinImage, SkinScreen};
use super::{Interaction, SkinAction, SkinEvent, SkinInputMap, SkinPointer, SkinPointerButton, SkinWriter, Target};
use crate::font::TextContext;
use crate::{BYTES_PER_PIXEL, CpuCanvas};

/// The region every hand-placed button stands on: 100 wide and 40 high with its corner at (200, 300).
const BUTTON: SkinRect = SkinRect::new(200.0, 300.0, 100.0, 40.0);

/// A point well inside [`BUTTON`], in its left and lower quarter.
const INSIDE: (f32, f32) = (210.0, 305.0);

/// A point in the right and upper quarter of [`BUTTON`].
const FAR_CORNER: (f32, f32) = (290.0, 335.0);

/// Two event ids no test gives a meaning to.
const LOWER_EVENT: i32 = 19;
const UPPER_EVENT: i32 = 90;

/// How far a hand-placed slider travels, and the corner its travel starts at.
const TRAVEL: f32 = 200.0;
const SLIDER: SkinRect = SkinRect::new(400.0, 300.0, 20.0, 30.0);

/// Every button a press can be made with.
const BUTTONS: [SkinPointerButton; 5] =
    [SkinPointerButton::Left, SkinPointerButton::Right, SkinPointerButton::Middle, SkinPointerButton::Back, SkinPointerButton::Forward];

/// A map of hand-placed targets, in the order they are drawn.
fn map(targets: &[(SkinRect, Interaction)]) -> SkinInputMap {
    SkinInputMap {
        targets: targets.iter().enumerate().map(|(object, (region, interaction))| Target { object, region: *region, interaction: *interaction }).collect(),
    }
}

/// A button that runs event `id` with the argument its `click` kind picks.
fn button(id: i32, kind: i32) -> Interaction {
    Interaction::Click { event: SkinEvent::Id(id), kind }
}

/// The one event a press ran and its argument.
fn ran(actions: &[SkinAction]) -> (SkinEvent, i32) {
    match actions {
        [SkinAction::Event { event, argument }] => (*event, *argument),
        other => panic!("expected exactly one event, got {other:?}"),
    }
}

/// The one value a pointer event wrote.
fn wrote(actions: &[SkinAction]) -> f32 {
    match actions {
        [SkinAction::Write { value, .. }] => *value,
        other => panic!("expected exactly one write, got {other:?}"),
    }
}

#[test]
fn a_press_goes_to_the_topmost_object_that_takes_it_and_to_no_other() {
    let stacked = map(&[(BUTTON, button(LOWER_EVENT, 0)), (BUTTON, button(UPPER_EVENT, 0))]);
    assert_eq!(ran(&stacked.press(SkinPointerButton::Left, INSIDE)), (SkinEvent::Id(UPPER_EVENT), 1), "the object drawn last is offered the press first");

    let beside = SkinRect::new(BUTTON.x + BUTTON.w + 10.0, BUTTON.y, BUTTON.w, BUTTON.h);
    let apart = map(&[(BUTTON, button(LOWER_EVENT, 0)), (beside, button(UPPER_EVENT, 0))]);
    assert_eq!(ran(&apart.press(SkinPointerButton::Left, INSIDE)), (SkinEvent::Id(LOWER_EVENT), 1), "an object the press misses does not stop it");
    assert_eq!(apart.press(SkinPointerButton::Left, (0.0, 0.0)), Vec::new(), "a press over nothing does nothing");
}

#[test]
fn each_button_asks_for_the_next_or_the_previous_as_the_reference_table_has_it() {
    let arguments: Vec<i32> = BUTTONS.iter().map(|button| button.argument()).collect();
    assert_eq!(arguments, [1, -1, 1, 1, -1]);

    let plain = map(&[(BUTTON, button(LOWER_EVENT, 0))]);
    let reversed = map(&[(BUTTON, button(LOWER_EVENT, 1))]);
    for pressed in BUTTONS {
        assert_eq!(ran(&plain.press(pressed, INSIDE)).1, pressed.argument(), "click 0 hands on the direction of {pressed:?}");
        assert_eq!(ran(&reversed.press(pressed, INSIDE)).1, -pressed.argument(), "click 1 hands on the opposite of {pressed:?}");
    }
}

#[test]
fn a_split_object_answers_which_half_was_pressed_whatever_the_button() {
    let sideways = map(&[(BUTTON, button(LOWER_EVENT, 2))]);
    let upright = map(&[(BUTTON, button(LOWER_EVENT, 3))]);
    let middle = (BUTTON.x + BUTTON.w / 2.0, BUTTON.y + BUTTON.h / 2.0);
    for pressed in BUTTONS {
        assert_eq!(ran(&sideways.press(pressed, INSIDE)).1, -1, "the left half is the previous");
        assert_eq!(ran(&sideways.press(pressed, FAR_CORNER)).1, 1, "the right half is the next");
        assert_eq!(ran(&sideways.press(pressed, middle)).1, 1, "the middle column belongs to the right half");
        assert_eq!(ran(&upright.press(pressed, INSIDE)).1, -1, "the lower half is the previous");
        assert_eq!(ran(&upright.press(pressed, FAR_CORNER)).1, 1, "the upper half is the next");
        assert_eq!(ran(&upright.press(pressed, middle)).1, 1, "the middle row belongs to the upper half");
    }
}

#[test]
fn a_click_kind_the_reference_has_no_case_for_takes_nothing() {
    let stacked = map(&[(BUTTON, button(LOWER_EVENT, 0)), (BUTTON, button(UPPER_EVENT, 4)), (BUTTON, button(UPPER_EVENT, -1))]);
    assert_eq!(ran(&stacked.press(SkinPointerButton::Left, INSIDE)).0, SkinEvent::Id(LOWER_EVENT), "the press falls through to the object beneath");
}

#[test]
fn a_region_holds_its_edges_and_one_of_negative_extent_holds_nothing() {
    let plain = map(&[(BUTTON, button(LOWER_EVENT, 0))]);
    for corner in [(BUTTON.x, BUTTON.y), (BUTTON.x + BUTTON.w, BUTTON.y + BUTTON.h)] {
        assert_eq!(plain.press(SkinPointerButton::Left, corner).len(), 1, "{corner:?} is on the edge and inside");
    }
    assert_eq!(plain.press(SkinPointerButton::Left, (BUTTON.x - 0.5, BUTTON.y)), Vec::new());
    assert_eq!(plain.press(SkinPointerButton::Left, (BUTTON.x, BUTTON.y + BUTTON.h + 0.5)), Vec::new());

    let flipped = map(&[(SkinRect::new(BUTTON.x + BUTTON.w, BUTTON.y, -BUTTON.w, BUTTON.h), button(LOWER_EVENT, 0))]);
    assert_eq!(flipped.press(SkinPointerButton::Left, INSIDE), Vec::new(), "a mirrored object is drawn over the point but its region is not");
}

/// A slider at [`SLIDER`] that travels [`TRAVEL`] in `direction`.
fn slider(direction: i32) -> SkinInputMap {
    map(&[(SLIDER, Interaction::Slide { direction, range: TRAVEL, writer: SkinWriter::Rate(RATE_MASTERVOLUME) })])
}

#[test]
fn a_slider_is_set_by_how_far_along_its_travel_the_pointer_is_in_each_of_the_four_directions() {
    let across_x = SLIDER.x + SLIDER.w / 2.0;
    let across_y = SLIDER.y + SLIDER.h / 2.0;
    let quarter = TRAVEL / 4.0;

    assert_eq!(wrote(&slider(0).drag((across_x, SLIDER.y + quarter))), 0.25, "0 travels upwards");
    assert_eq!(wrote(&slider(1).drag((SLIDER.x + quarter, across_y))), 0.25, "1 travels to the right");
    assert_eq!(wrote(&slider(2).drag((across_x, SLIDER.y - quarter))), 0.25, "2 travels downwards");
    assert_eq!(wrote(&slider(3).drag((SLIDER.x - quarter, across_y))), 0.25, "3 travels to the left");

    assert_eq!(slider(0).drag((across_x, SLIDER.y - quarter)), Vec::new(), "behind the start of the travel is off it");
    assert_eq!(slider(1).drag((SLIDER.x + TRAVEL + 1.5, across_y)), Vec::new(), "past the end of the travel is off it");
    assert_eq!(slider(2).drag((SLIDER.x + SLIDER.w + 1.0, SLIDER.y - quarter)), Vec::new(), "beside the travel is off it");
    assert_eq!(slider(3).drag((SLIDER.x - quarter, SLIDER.y + SLIDER.h + 1.0)), Vec::new());
    assert_eq!(slider(4).drag((across_x, across_y)), Vec::new(), "a direction the reference has no case for never moves");
}

#[test]
fn a_slider_snaps_to_either_end_within_a_pixel_of_it() {
    let across_x = SLIDER.x + SLIDER.w / 2.0;
    let across_y = SLIDER.y + SLIDER.h / 2.0;
    let near = 0.75;

    assert_eq!(wrote(&slider(0).drag((across_x, SLIDER.y + near))), 0.0);
    assert_eq!(wrote(&slider(0).drag((across_x, SLIDER.y + TRAVEL - near))), 1.0);
    assert_eq!(wrote(&slider(1).drag((SLIDER.x + near, across_y))), 0.0);
    assert_eq!(wrote(&slider(1).drag((SLIDER.x + TRAVEL - near, across_y))), 1.0);
    assert_eq!(wrote(&slider(2).drag((across_x, SLIDER.y - near))), 0.0);
    assert_eq!(wrote(&slider(2).drag((across_x, SLIDER.y - TRAVEL + near))), 1.0);
    assert_eq!(wrote(&slider(3).drag((SLIDER.x - near, across_y))), 0.0);
    assert_eq!(wrote(&slider(3).drag((SLIDER.x - TRAVEL + near, across_y))), 1.0);

    let stub = map(&[(SLIDER, Interaction::Slide { direction: 1, range: 0.5, writer: SkinWriter::Rate(RATE_MASTERVOLUME) })]);
    assert_eq!(wrote(&stub.drag((SLIDER.x + 0.5, across_y))), 0.0, "a travel shorter than the snap is at its start everywhere");
}

#[test]
fn a_drag_reaches_sliders_alone_and_a_press_sets_a_slider_too() {
    let on_travel = (SLIDER.x + TRAVEL / 2.0, SLIDER.y + SLIDER.h / 2.0);
    let covering = SkinRect::new(SLIDER.x, SLIDER.y, TRAVEL, SLIDER.h);
    let covered =
        map(&[(SLIDER, Interaction::Slide { direction: 1, range: TRAVEL, writer: SkinWriter::Rate(RATE_MASTERVOLUME) }), (covering, button(UPPER_EVENT, 0))]);
    assert_eq!(wrote(&covered.drag(on_travel)), 0.5, "a button drawn over a slider does not take the drag");
    assert_eq!(ran(&covered.press(SkinPointerButton::Left, on_travel)).0, SkinEvent::Id(UPPER_EVENT), "but it does take the press");

    let bare = slider(1);
    assert_eq!(
        bare.press(SkinPointerButton::Right, on_travel),
        vec![SkinAction::Write { writer: SkinWriter::Rate(RATE_MASTERVOLUME), value: 0.5 }],
        "a press on a slider's travel sets it, whichever button made it"
    );
}

#[test]
fn an_editable_text_takes_a_press_inside_the_bounds_its_alignment_gives_it() {
    let left = map(&[(BUTTON, Interaction::Edit { align: 0 })]);
    let centred = map(&[(BUTTON, Interaction::Edit { align: 1 })]);
    let right = map(&[(BUTTON, Interaction::Edit { align: 2 })]);
    let before = (BUTTON.x - BUTTON.w + 5.0, INSIDE.1);
    let astride = (BUTTON.x - BUTTON.w / 2.0 + 5.0, INSIDE.1);

    assert_eq!(left.press(SkinPointerButton::Left, INSIDE), vec![SkinAction::FocusText { object: 0 }]);
    assert_eq!(left.press(SkinPointerButton::Left, astride), Vec::new());
    assert_eq!(centred.press(SkinPointerButton::Left, astride).len(), 1, "a centred text is typed into from half its width before its x");
    assert_eq!(centred.press(SkinPointerButton::Left, before), Vec::new());
    assert_eq!(right.press(SkinPointerButton::Left, before).len(), 1, "a right-aligned text ends at its x");
    assert_eq!(right.press(SkinPointerButton::Left, FAR_CORNER), Vec::new());

    let over = map(&[(BUTTON, button(LOWER_EVENT, 0)), (BUTTON, Interaction::Edit { align: 0 })]);
    assert_eq!(over.press(SkinPointerButton::Left, INSIDE), vec![SkinAction::FocusText { object: 1 }], "the text is the one object that takes the press");
    assert_eq!(over.drag(INSIDE), Vec::new(), "and a drag is nothing to it");
}

#[test]
fn the_wheel_reaches_no_object() {
    let everything =
        map(&[(BUTTON, button(LOWER_EVENT, 0)), (BUTTON, Interaction::Slide { direction: 1, range: TRAVEL, writer: SkinWriter::Rate(RATE_MASTERVOLUME) })]);
    assert_eq!(everything.pointer(SkinPointer::Scroll(1.0), INSIDE), Vec::new());
    assert_eq!(everything.pointer(SkinPointer::Press(SkinPointerButton::Left), INSIDE).len(), 1);
    assert_eq!(everything.pointer(SkinPointer::Drag, (BUTTON.x + TRAVEL / 2.0, INSIDE.1)).len(), 1);
    assert_eq!(everything.len(), 2);
    assert!(SkinInputMap::default().is_empty());
}

/// The size the fixture is authored at and drawn on.
const SCREEN: (u32, u32) = (1280, 720);

/// The size of the white panel that stands in for the fixture's one source.
const PANEL: (u32, u32) = (16, 16);

/// A skin with buttons and sliders of every kind the loader tells apart. Each button is 100 by 40
/// and each slider travels 200 to the right from a 20 by 30 handle.
///
/// Bottom to top: `under` (event 19), a `cover` with no `act` over the same spot, `numbered`
/// (event 90, split left and right), `scripted` (a function), `named` (the event the reference calls
/// `favorite_chart`), `hidden` (gated by a function that answers false, over `numbered`), `hover` (drawn only
/// while the pointer is over the lower half of its own region), a set of two images with an `act`,
/// and then the sliders.
const INPUT_SKIN: &str = r#"
local function shape(id) return { id = id, src = 0, x = 0, y = 0, w = 16, h = 16 } end
local function button(id, act, click)
    local image = shape(id)
    image.act = act
    image.click = click
    return image
end
local function knob(id, fields)
    local slider = shape(id)
    slider.angle = 1
    slider.range = 200
    for key, value in pairs(fields) do slider[key] = value end
    return slider
end
local function at(x, y) return { { x = x, y = y, w = 100, h = 40 } } end
local function handle(x, y) return { { x = x, y = y, w = 20, h = 30 } } end
return {
    type = 6, name = "input", w = 1280, h = 720,
    source = { { id = 0, path = "panel.tex" } },
    image = {
        button("under", 19), shape("cover"), button("numbered", 90, 2), button("scripted", function(direction) end),
        button("named", "favorite_chart"), button("hidden", 12), button("hover", 13), shape("first"), shape("second"),
    },
    imageset = { { id = "pair", images = { "first", "second" }, act = 14, click = 1 } },
    slider = {
        knob("volume", { type = VOLUME_RATE }),
        knob("progress", { type = PROGRESS_RATE }),
        knob("fixed", { type = VOLUME_RATE, changeable = false }),
        knob("scaled", { type = VOLUME_RATE, isRefNum = true, min = 0, max = 10 }),
        knob("computed", { value = function() return 0.5 end, event = function(value) end }),
        knob("readonly", { value = function() return 0.5 end }),
        knob("routed", { value = function() return 0.5 end, event = VOLUME_RATE }),
    },
    destination = {
        { id = "under", dst = at(0, 0) },
        { id = "cover", dst = at(0, 0) },
        { id = "numbered", dst = at(200, 0) },
        { id = "scripted", dst = at(400, 0) },
        { id = "named", dst = at(600, 0) },
        { id = "hidden", draw = function() return false end, dst = at(200, 0) },
        { id = "hover", mouseRect = { x = 0, y = 0, w = 100, h = 20 }, dst = at(800, 0) },
        { id = "pair", dst = at(1000, 0) },
        { id = "volume", dst = handle(0, 100) },
        { id = "progress", dst = handle(0, 200) },
        { id = "fixed", dst = handle(0, 300) },
        { id = "scaled", dst = handle(0, 400) },
        { id = "computed", dst = handle(0, 500) },
        { id = "readonly", dst = handle(0, 600) },
        { id = "routed", dst = handle(400, 100) },
    },
}
"#;

/// A white panel for whatever source the fixture names.
struct PanelAssets;

impl SkinAssets for PanelAssets {
    fn image(&mut self, _path: &Path) -> Option<SkinImage> {
        SkinImage::new(PANEL.0, PANEL.1, vec![u8::MAX; (PANEL.0 * PANEL.1) as usize * BYTES_PER_PIXEL])
    }
}

/// The fixture loaded and built. Its folder is removed with it.
struct Fixture {
    root: PathBuf,
    skin: LoadedSkin,
    screen: SkinScreen,
    host: MapHost,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        crate::font::use_embedded_fonts_only();
        let root = std::env::temp_dir().join(format!("rbms-render-skin-input-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        std::fs::write(root.join("panel.tex"), []).expect("the source file is writable");
        let source = INPUT_SKIN.replace("VOLUME_RATE", &RATE_MASTERVOLUME.to_string()).replace("PROGRESS_RATE", &RATE_MUSIC_PROGRESS.to_string());
        let entry = root.join("input.luaskin");
        std::fs::write(&entry, source).expect("the skin is writable");

        let host = MapHost::new();
        let user = SkinUserConfig::default();
        let load = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, rbms_model::Mode::BEAT_7K) };
        let skin = load_lua_skin(&entry, &LuaSkinOptions { load, budget: LuaBudget::default() }, &host).expect("the fixture loads");
        let mut canvas = CpuCanvas::new(SCREEN.0, SCREEN.1);
        let mut text = TextContext::embedded_only();
        let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut PanelAssets);
        assert_eq!(screen.warnings(), &[] as &[String], "the fixture builds whole");
        Fixture { root, skin, screen, host }
    }

    /// One frame prepared with the pointer at `mouse`.
    fn prepare(&self, mouse: Option<(f32, f32)>) -> PreparedFrame {
        let timers = TimerState::new();
        let runtime = self.skin.runtime().expect("a Lua skin has an interpreter");
        runtime
            .frame(&self.host, |bound| {
                self.screen.prepare(&SkinFrame { now_us: 0, timers: &timers, state: &self.host, lua: Some(bound), mouse, data: FrameData::default() })
            })
            .expect("the host binds")
    }

    /// What a left press at `at` does, judged against a frame prepared with the pointer there.
    fn press(&self, at: (f32, f32)) -> Vec<SkinAction> {
        self.screen.pointer(&self.prepare(Some(at)), at, SkinPointer::Press(SkinPointerButton::Left))
    }

    /// What a drag to `at` does.
    fn drag(&self, at: (f32, f32)) -> Vec<SkinAction> {
        self.screen.pointer(&self.prepare(Some(at)), at, SkinPointer::Drag)
    }
}

#[test]
fn a_built_screen_gives_each_image_the_event_its_document_declared() {
    let fixture = Fixture::new("events");

    assert_eq!(ran(&fixture.press((50.0, 20.0))), (SkinEvent::Id(LOWER_EVENT), 1), "an image with no act lets the press through to the one beneath");
    assert_eq!(ran(&fixture.press((210.0, 20.0))), (SkinEvent::Id(UPPER_EVENT), -1), "an object whose condition fails is not there to be pressed");
    assert_eq!(ran(&fixture.press((290.0, 20.0))), (SkinEvent::Id(UPPER_EVENT), 1));
    let scripted = ran(&fixture.press((450.0, 20.0)));
    assert!(matches!(scripted, (SkinEvent::Function(_), 1)), "a function is handed over as it is: {scripted:?}");
    assert_eq!(ran(&fixture.press((650.0, 20.0))), (SkinEvent::Id(UPPER_EVENT), 1), "a named event is the id the reference gives the name");
    assert_eq!(ran(&fixture.press((1050.0, 20.0))), (SkinEvent::Id(14), -1), "an image set carries an act of its own");
    assert_eq!(fixture.press((1050.0, 60.0)), Vec::new());
}

#[test]
fn an_object_shown_only_under_the_pointer_is_pressed_only_while_it_is_shown() {
    let fixture = Fixture::new("hover");
    let over = (850.0, 10.0);
    let above = (850.0, 30.0);

    assert_eq!(ran(&fixture.press(over)).0, SkinEvent::Id(13), "the pointer is inside the rectangle the object is shown under");
    assert_eq!(fixture.press(above), Vec::new(), "inside the object's region but outside that rectangle, it is not drawn at all");

    let shown = fixture.prepare(Some(over));
    let unshown = fixture.prepare(Some(above));
    let away = fixture.prepare(None);
    assert_eq!(shown.visible_count(), unshown.visible_count() + 1, "which is the object being hovered and nothing else");
    assert_eq!(unshown.visible_count(), away.visible_count(), "with no pointer at all it is not drawn either");
    assert_eq!(
        fixture.screen.pointer(&unshown, over, SkinPointer::Press(SkinPointerButton::Left)),
        Vec::new(),
        "a press is judged against the frame that was prepared"
    );
}

#[test]
fn a_built_screen_gives_each_slider_the_writer_the_reference_would() {
    let fixture = Fixture::new("sliders");
    let writer = |actions: &[SkinAction]| match actions {
        [SkinAction::Write { writer, value }] => (*writer, *value),
        other => panic!("expected exactly one write, got {other:?}"),
    };

    assert_eq!(writer(&fixture.drag((50.0, 110.0))), (SkinWriter::Rate(RATE_MASTERVOLUME), 0.25), "a changeable slider writes the rate it reads");
    assert_eq!(fixture.drag((50.0, 210.0)), Vec::new(), "a rate the reference cannot write is not dragged");
    assert_eq!(fixture.drag((50.0, 310.0)), Vec::new(), "nor is a slider that is not changeable");
    assert_eq!(fixture.drag((50.0, 410.0)), Vec::new(), "nor one that scales a number itself");
    assert_eq!(writer(&fixture.drag((100.0, 510.0))).1, 0.5, "a slider with a value writes to its event");
    assert!(matches!(writer(&fixture.drag((100.0, 510.0))).0, SkinWriter::Function(_)));
    assert_eq!(fixture.drag((50.0, 610.0)), Vec::new(), "and to nothing when it has no event");
    assert_eq!(writer(&fixture.drag((500.0, 110.0))), (SkinWriter::Rate(RATE_MASTERVOLUME), 0.5), "an event that is a number is a rate to write");

    assert_eq!(fixture.drag((50.0, 20.0)), Vec::new(), "a drag over a button moves nothing");
    assert_eq!(writer(&fixture.press((50.0, 110.0))).1, 0.25, "a press on the travel sets the slider as a drag does");
}

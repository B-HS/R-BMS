//! The default pack's play scene drawn with rows of its settings switched.
//!
//! The gate in the sibling module draws every play document of the default pack as it comes, and so
//! does the dump tool. A row of the pack's settings moves everything on the screen, and the layout it
//! moves the screen to is read by neither: the rows a player switches on the skin menu are only ever
//! read here.
//!
//! Four tests hold the pack to what its rows say on every machine, on frames drawn small. With the
//! lane width switched to wide, every key mode still compiles with nothing said against it and plays,
//! each lane is half as wide again as the lane it is by default, and no two lanes share a pixel.
//! Whichever way the rows of a wide document are switched -- the side the field is on, the side the
//! turntable is on, the score graph -- the pictures of a chart and the gauge stay on the screen and
//! clear of the lanes. Whichever way the rows that move a field are switched, the field ends on the
//! foot of its judgement line, with nothing of it drawn below but the bombs, whose rings are whole
//! and sit on the middle of the line. And whichever item a row of colours is switched to, every key
//! mode that offers the row compiles with nothing said against it and plays, on a ground that is
//! black unless the theme row says slate.
//!
//! One more draws the same scenes at the size the pack is authored at for a person to look at, and
//! only where [`CAPTURE_DIR_ENV`] names a folder: every key mode as it comes and with wide lanes, the
//! rows of a single field switched, the three covers of the field drawn, the last stage of a course,
//! a run walked from its loading, through a long note that is held, to its full combo and to its
//! failure, and the rows of colours switched.

use rbms_skin::dst::SkinOffset;
use rbms_skin::model::Animation;

use super::*;

/// The row of the pack's play documents that says how wide a lane is, and its item for the wider one.
const LANE_WIDTH_ROW: &str = "Lane Width";
const WIDE_LANES: &str = "Wide";

/// How many times the width of a lane as it comes a wide one is, as a fraction.
const WIDE_LANE_TIMES: (i32, i32) = (3, 2);

/// The other rows of a single field, each with the item it is not on as it comes.
const FIELD_ON_THE_RIGHT: (&str, &str) = ("Field Side", "Right");
const SCRATCH_ON_THE_RIGHT: (&str, &str) = ("Scratch Side", "Right");
const WITHOUT_SCORE_GRAPH: (&str, &str) = ("Score Graph", "Off");
const WIDE: (&str, &str) = (LANE_WIDTH_ROW, WIDE_LANES);

/// The size the frames the pack is held to are drawn at, 16 to 9 like the pack and a third of its size.
const HELD_ROWS_SIZE: (u32, u32) = (640, 360);

/// How far into its chart a held run is played, in milliseconds: enough for notes to be on the field.
const HELD_ROWS_CHART_MS: i64 = 3_000;

/// The moment of a chart a captured run is drawn at, in milliseconds.
const ROWS_SHOT_MS: i64 = 9_000;

/// A moment of the charts written for a mode at which a long note is held, in milliseconds: the
/// fifth measure of a chart at 140 beats a minute has one from its first beat to its fourth.
const ROWS_HELD_SHOT_MS: i64 = 7_500;

/// The course a run is said to be the last stage of in the captures that draw one.
const CAPTURED_COURSE: &str = "Capture Course";
const CAPTURED_COURSE_STAGES: usize = 2;

/// The shares of the field the three covers take in the captures that draw them.
const CAPTURED_COVER: f32 = 0.3;
const CAPTURED_LIFT: f32 = 0.2;
const CAPTURED_HIDDEN: f32 = 0.2;

/// The ids the pack's play documents place the pictures of a chart and the gauge under.
const BGA_OBJECT: &str = "bga";
const GAUGE_OBJECT: &str = "gauge";

/// The parts of a field that are drawn on its lanes and stay where the document puts them, by the
/// offsets that say how opaque each is: the faces of the lanes, the lines between them, the
/// judgement line, the beams of the keys that are down, and the bombs. All but the bombs end where
/// the judgement line does.
const LANE_FACES: &str = "Lane Background Opacity";
const LANE_DIVIDERS: &str = "Lane Divider Opacity";
const JUDGEMENT_LINE: &str = "Judge Line Opacity";
const KEY_BEAMS: &str = "Key Beam Opacity";
const BOMBS: &str = "Bomb Opacity";
const PARTS_ENDING_ON_THE_LINE: [&str; 4] = [LANE_FACES, LANE_DIVIDERS, JUDGEMENT_LINE, KEY_BEAMS];

/// The alpha an opacity offset holds when its part is taken away.
const PART_TAKEN_AWAY: f32 = -255.0;

/// How far under a field a frame is held to showing nothing, in the pixels the document is
/// authored in: the gap the document leaves before the gauge or the readouts under the field, short
/// of the rows a round letter of a readout reaches over its line into on a frame drawn small.
const CLEAR_UNDER_A_FIELD: i32 = 12;

/// The offset the pack's play documents once declared for the keys they drew under a field. A
/// settings file written then still holds a value under it, here the one that took the part away.
const RETIRED_KEY_DISPLAY_OFFSET: i32 = 105;

/// The rows of the pack's play documents that say what its colours are, each with the items it
/// offers, the one it comes with first: the theme of the faces and the letters, the colour of what
/// is picked out, the colours of the notes of a field with a turntable, and the colour of the bombs.
const THEME_ROW: &str = "Theme";
const SLATE_THEME: &str = "Slate";
const ACCENT_ROW: &str = "Accent";
const NOTE_COLOURS_ROW: &str = "Note Colors";
const BOMB_COLOUR_ROW: &str = "Bomb Color";
const THEMES: [&str; 2] = ["Mono", SLATE_THEME];
const ACCENTS: [&str; 5] = ["White", "Sky", "Amber", "Green", "Pink"];
const NOTE_COLOURS: [&str; 5] = ["White-Blue", "White-Deep Blue", "White-Yellow", "White-Green", "Mono"];
const BOMB_COLOURS: [&str; 2] = ["Note", "White"];

/// The ground of the screen under each theme: black as the pack comes, and the dark slate the pack
/// was first drawn on.
const MONO_GROUND: crate::Color = crate::Color::rgb(0, 0, 0);
const SLATE_GROUND: crate::Color = crate::Color::rgb(12, 15, 20);

/// The rows of colours switched for a person to look at.
const SKY_ACCENT: (&str, &str) = (ACCENT_ROW, "Sky");
const SLATE_WITH_SKY: [(&str, &str); 2] = [(THEME_ROW, SLATE_THEME), SKY_ACCENT];
const AMBER_ACCENT: (&str, &str) = (ACCENT_ROW, "Amber");
const GREEN_ACCENT: (&str, &str) = (ACCENT_ROW, "Green");
const PINK_ACCENT: (&str, &str) = (ACCENT_ROW, "Pink");
const DEEP_BLUE_NOTES: (&str, &str) = (NOTE_COLOURS_ROW, "White-Deep Blue");
const YELLOW_NOTES: (&str, &str) = (NOTE_COLOURS_ROW, "White-Yellow");
const GREEN_NOTES: (&str, &str) = (NOTE_COLOURS_ROW, "White-Green");
const MONO_NOTES: (&str, &str) = (NOTE_COLOURS_ROW, "Mono");
const WHITE_BOMBS: (&str, &str) = (BOMB_COLOUR_ROW, "White");

/// One key mode of the pack's play documents: the name its captures are saved under, the title and
/// the channels of the chart written for it, and the mode the chart is read in when its channels
/// cannot tell.
struct RowMode {
    name: &'static str,
    title: &'static str,
    sides: &'static [SideChannels],
    mode: Option<rbms_model::Mode>,
    /// The rows a document of the mode offers beside the lane width, each with its other item.
    rows: &'static [(&'static str, &'static str)],
}

const SINGLE_ROWS: [(&str, &str); 3] = [FIELD_ON_THE_RIGHT, SCRATCH_ON_THE_RIGHT, WITHOUT_SCORE_GRAPH];

const FIVE: RowMode = RowMode { name: "play5", title: "Five Keys", sides: &[FIVE_KEYS_FIRST], mode: None, rows: &SINGLE_ROWS };
const SEVEN: RowMode = RowMode { name: "play7", title: "Seven Keys", sides: &[SEVEN_KEYS_FIRST], mode: None, rows: &SINGLE_ROWS };
const NINE: RowMode = RowMode { name: "play9", title: "Nine Keys", sides: &[NINE_KEYS], mode: Some(rbms_model::Mode::POPN_9K), rows: &[WITHOUT_SCORE_GRAPH] };
const TEN: RowMode =
    RowMode { name: "play10", title: "Ten Keys", sides: &[FIVE_KEYS_FIRST, FIVE_KEYS_SECOND], mode: Some(rbms_model::Mode::BEAT_10K), rows: &[] };
const FOURTEEN: RowMode = RowMode { name: "play14", title: "Fourteen Keys", sides: &[SEVEN_KEYS_FIRST, SEVEN_KEYS_SECOND], mode: None, rows: &[] };

const ROW_MODES: [&RowMode; 5] = [&FIVE, &SEVEN, &NINE, &TEN, &FOURTEEN];

/// A rectangle of a document, in the pixels the document is authored in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placed {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Placed {
    /// The rectangle a frame of an animation gives all four sides of.
    fn of(frame: &Animation) -> Option<Placed> {
        Some(Placed { x: frame.x?, y: frame.y?, w: frame.w?, h: frame.h? })
    }

    /// Whether the two rectangles share a pixel.
    fn overlaps(&self, other: &Placed) -> bool {
        self.x < other.x + other.w && other.x < self.x + self.w && self.y < other.y + other.h && other.y < self.y + self.h
    }

    /// Whether the rectangle lies wholly on a screen of `width` by `height`.
    fn is_on(&self, (width, height): (i32, i32)) -> bool {
        self.w > 0 && self.h > 0 && self.x >= 0 && self.y >= 0 && self.x + self.w <= width && self.y + self.h <= height
    }
}

/// The default pack's play scene entered for a chart of one key mode with rows of its document
/// switched, its document compiled.
struct RowsRun {
    app: App,
    screen: i32,
    size: (u32, u32),
    play_time_ms: i64,
    clocks: PlayClocks,
    pixels: HeadlessCanvas,
    arrived: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    _keysounds: std::sync::mpsc::Sender<(u32, rbms_audio::DecodedAudio)>,
}

impl RowsRun {
    /// Enter the play scene for a chart of `mode` with each of `rows` switched to the item named,
    /// as the last stage of a course when `course` says so, and wait for the document. The frames
    /// are drawn at `size`, or at the size the document is authored at when there is none. Answers
    /// why the scene was not entered, which a row the document does not offer is one reason for.
    ///
    /// The course is there before the document is read: what a document draws for a course is
    /// settled when it is compiled.
    fn enter(tag: &str, mode: &RowMode, rows: &[(&str, &str)], size: Option<(u32, u32)>, ending: PlayEnding, course: bool) -> Result<RowsRun, String> {
        let name = format!("{tag}.bms");
        let model = chart_model(&mode_chart(mode.title, mode.sides), &name, mode.mode);
        let chart = settings_of(&format!("{tag}-chart")).with_file_name(&name);
        let EnteredPlay { mut app, screen, size: authored, play_time_ms, arrived, _keysounds } = enter_play_scene(&default_pack(), tag, &chart, model, ending)?;
        for (row, item) in rows {
            if !choose_in_skin(&mut app, screen, row, item) {
                return Err(format!("the document has no row {row:?} with an item {item:?}"));
            }
        }
        if course {
            let stage = |number: usize| CourseChart { md5: format!("{number:032x}"), sha256: String::new(), title: format!("Stage {number}") };
            let charts = (1..=CAPTURED_COURSE_STAGES).map(stage).collect();
            let mut course = CourseRun::new(Course { name: CAPTURED_COURSE.to_owned(), charts, ..Course::default() }, 0.0);
            course.index = CAPTURED_COURSE_STAGES - 1;
            app.shared.course_run = Some(course);
        }
        app.shared.skins.request_for(&app.shared.config, screen);
        let size = size.unwrap_or(authored);
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        wait_for_scene_document(&mut app, screen, &mut Canvas::Headless(&mut pixels))?;
        Ok(RowsRun { app, screen, size, play_time_ms, clocks: PlayClocks::default(), pixels, arrived, _keysounds })
    }

    /// The document the scene is drawn with.
    fn document(&self) -> &rbms_skin::loader::LoadedSkin {
        self.app.shared.skins.document(self.screen).expect("a compiled screen has its document")
    }

    /// The size the document is authored at.
    fn authored(&self) -> (i32, i32) {
        (self.document().def.w, self.document().def.h)
    }

    /// The rectangle of every lane of the document's note field, in the order the lanes are numbered.
    fn lanes(&self) -> Vec<Placed> {
        let field = self.document().def.note.as_ref().expect("a play document has a note field");
        field.dst.iter().map(|lane| Placed::of(lane).expect("a lane has all four sides")).collect()
    }

    /// The rectangle the object called `id` is first placed at.
    fn placed(&self, id: &str) -> Placed {
        let destination = self.document().def.destination.iter().find(|destination| destination.id == id);
        destination.and_then(|destination| destination.dst.first()).and_then(Placed::of).unwrap_or_else(|| panic!("the document places no {id}"))
    }

    /// The id of the offset the document declares under the name `part`.
    fn part_id(&self, part: &str) -> i32 {
        let declared = self.document().def.offset.iter().find(|offset| offset.name == part);
        declared.map(|offset| offset.id).unwrap_or_else(|| panic!("the document declares no {part}"))
    }

    /// The rectangles of every frame of every destination the document draws as the part whose
    /// opacity the offset called `part` says.
    fn part(&self, part: &str) -> Vec<Placed> {
        let def = &self.document().def;
        let id = self.part_id(part);
        def.destination
            .iter()
            .filter(|destination| destination.offsets.contains(&id))
            .flat_map(|destination| destination.dst.iter().filter_map(Placed::of))
            .collect()
    }

    /// Walk the scene from its loading until its chart plays, handing `shoot` the name of each
    /// moment on the way once its frame is drawn.
    fn open(&mut self, mut shoot: impl FnMut(&str, &HeadlessCanvas)) -> Result<(), String> {
        let timings = (i64::from(self.document().play.loadend), i64::from(self.document().play.playstart));
        self.clocks.open(&mut self.app, &mut self.pixels, &self.arrived, timings, |_, pixels, _, moment| {
            shoot(moment, pixels);
            Ok(())
        })
    }

    /// Play the chart on to `chart_ms` and draw the frame of that moment.
    fn play_to(&mut self, chart_ms: i64) -> Result<(), String> {
        let left_ms = chart_ms - play_screen(&self.app)?.chart_ms();
        self.clocks.play_on(&mut self.app, left_ms, &mut Canvas::Headless(&mut self.pixels))
    }

    /// Hold the document to having compiled with nothing said against it.
    fn assert_clean(&self, label: &str) {
        assert_eq!(self.app.shared.skin_failure(self.screen), None, "{label}: the document failed");
        assert_eq!(self.app.shared.skin_warnings(self.screen), &[] as &[String], "{label}: the document compiled with warnings");
    }
}

/// The name a run is entered and reported under: its mode and the items its rows are switched to.
fn label_of(mode: &RowMode, rows: &[(&str, &str)]) -> String {
    let mut label = mode.name.to_owned();
    for (row, item) in rows {
        label.push_str(&format!("-{}-{}", row.replace(' ', ""), item.replace(' ', "")));
    }
    label.to_ascii_lowercase()
}

/// Every key mode of the pack offers the lane width row. With it switched to wide the document
/// compiles with nothing said against it, each lane is as wide as [`WIDE_LANE_TIMES`] says and as
/// tall as it was, the lanes are on the screen and no two of them share a pixel, and the scene
/// plays and draws.
#[test]
fn the_default_pack_draws_every_play_scene_with_lanes_half_as_wide_again_when_its_row_says_so() {
    for mode in ROW_MODES {
        let label = mode.name;
        let enter = |tag: &str, rows: &[(&str, &str)]| {
            RowsRun::enter(&format!("rows-{label}-{tag}"), mode, rows, Some(HELD_ROWS_SIZE), PlayEnding::StillPlaying, false)
                .unwrap_or_else(|reason| panic!("{label} {tag}: not entered: {reason}"))
        };
        let normal = enter("normal", &[]);
        let mut wide = enter("wide", &[WIDE]);
        normal.assert_clean(label);
        wide.assert_clean(label);

        let (as_it_comes, widened) = (normal.lanes(), wide.lanes());
        assert_eq!(as_it_comes.len(), widened.len(), "{label}: a wide field has another number of lanes");
        for (index, (lane, wider)) in as_it_comes.iter().zip(&widened).enumerate() {
            assert_eq!(wider.w * WIDE_LANE_TIMES.1, lane.w * WIDE_LANE_TIMES.0, "{label}: lane {index} is {} wide by default and {} wide", lane.w, wider.w);
            assert_eq!((wider.y, wider.h), (lane.y, lane.h), "{label}: lane {index} of a wide field is not as tall as it was");
        }
        let screen = wide.authored();
        for (index, lane) in widened.iter().enumerate() {
            assert!(lane.is_on(screen), "{label}: wide lane {index} at {lane:?} runs off the screen");
            for (other_index, other) in widened.iter().enumerate().skip(index + 1) {
                assert!(!lane.overlaps(other), "{label}: wide lanes {index} and {other_index} share pixels: {lane:?} and {other:?}");
            }
        }

        wide.open(|_, _| {}).unwrap_or_else(|reason| panic!("{label}: a wide field was not opened: {reason}"));
        wide.play_to(HELD_ROWS_CHART_MS).unwrap_or_else(|reason| panic!("{label}: a wide field was not played: {reason}"));
        wide.assert_clean(label);
        assert!(wide.pixels.quad_count() > 0, "{label}: nothing was drawn of a wide field");
        assert!(!is_flat(&wide.pixels, 0..HELD_ROWS_SIZE.0, 0..HELD_ROWS_SIZE.1), "{label}: the frame of a wide field is one colour");
    }
}

/// Every way of switching the rows a document offers beside the lane width, the lanes wide.
fn wide_row_sets(mode: &RowMode) -> Vec<Vec<(&'static str, &'static str)>> {
    (0..1usize << mode.rows.len())
        .map(|picked| {
            let others = mode.rows.iter().enumerate().filter(|(index, _)| picked & (1 << index) != 0).map(|(_, row)| *row);
            std::iter::once(WIDE).chain(others).collect()
        })
        .collect()
}

/// However the rows of a wide document are switched, it compiles with nothing said against it, and
/// the pictures of the chart and the gauge are on the screen and clear of every lane.
#[test]
fn the_default_pack_keeps_the_pictures_and_the_gauge_clear_of_wide_lanes_however_its_rows_are_switched() {
    for mode in ROW_MODES {
        for rows in wide_row_sets(mode) {
            let label = label_of(mode, &rows);
            let run = RowsRun::enter(&format!("rows-clear-{label}"), mode, &rows, Some(HELD_ROWS_SIZE), PlayEnding::StillPlaying, false)
                .unwrap_or_else(|reason| panic!("{label}: not entered: {reason}"));
            run.assert_clean(&label);
            let screen = run.authored();
            let lanes = run.lanes();
            for id in [BGA_OBJECT, GAUGE_OBJECT] {
                let placed = run.placed(id);
                assert!(placed.is_on(screen), "{label}: the {id} at {placed:?} runs off the screen");
                for (index, lane) in lanes.iter().enumerate() {
                    assert!(!placed.overlaps(lane), "{label}: the {id} at {placed:?} is drawn over lane {index} at {lane:?}");
                }
            }
            assert!(!run.placed(BGA_OBJECT).overlaps(&run.placed(GAUGE_OBJECT)), "{label}: the pictures are drawn over the gauge");
        }
    }
}

/// Every way of switching the rows that move the lanes of a field: the lane width, and the sides
/// the field and the turntable are on where the document offers them.
fn field_row_sets(mode: &RowMode) -> Vec<Vec<(&'static str, &'static str)>> {
    let moving: Vec<(&str, &str)> =
        std::iter::once(WIDE).chain(mode.rows.iter().copied().filter(|row| [FIELD_ON_THE_RIGHT, SCRATCH_ON_THE_RIGHT].contains(row))).collect();
    (0..1usize << moving.len()).map(|picked| moving.iter().enumerate().filter(|(index, _)| picked & (1 << index) != 0).map(|(_, row)| *row).collect()).collect()
}

/// However the rows that move a field are switched, every field of every key mode ends on the foot
/// of its judgement line. The line lies across the whole field with its top where the lanes of the
/// note field end, so a note comes to rest on it; the faces of the lanes and the lines between them
/// end where the line does; and no part that is drawn on the lanes reaches below it but the bombs,
/// each of which is a whole ring with its middle on the middle of the line. In a frame of the run as
/// it plays with the bombs taken away, the rows right under each field are the ground of the screen.
/// A value a settings file still holds for the keys the documents once drew under a field is read
/// without a word and changes nothing.
#[test]
fn the_default_pack_ends_every_field_on_the_foot_of_its_judgement_line() {
    for mode in ROW_MODES {
        for rows in field_row_sets(mode) {
            let label = label_of(mode, &rows);
            let mut run = RowsRun::enter(&format!("rows-foot-{label}"), mode, &rows, Some(HELD_ROWS_SIZE), PlayEnding::StillPlaying, false)
                .unwrap_or_else(|reason| panic!("{label}: not entered: {reason}"));
            run.assert_clean(&label);
            let lanes = run.lanes();
            let lines = run.part(JUDGEMENT_LINE);
            let (faces, dividers) = (run.part(LANE_FACES), run.part(LANE_DIVIDERS));
            let on_the_lanes: Vec<Placed> = PARTS_ENDING_ON_THE_LINE.iter().flat_map(|part| run.part(part)).collect();
            let (bombs, bomb_part) = (run.part(BOMBS), run.part_id(BOMBS));

            let mut feet = Vec::new();
            for (index, side) in lanes.chunks(lanes.len() / mode.sides.len()).enumerate() {
                let left = side.iter().map(|lane| lane.x).min().unwrap_or_default();
                let right = side.iter().map(|lane| lane.x + lane.w).max().unwrap_or_default();
                let rest = side.iter().map(|lane| lane.y).max().unwrap_or_default();
                let on_this_field = |placed: &&Placed| placed.x < right && left < placed.x + placed.w;
                assert!(side.iter().all(|lane| lane.y == rest), "{label}: the lanes of field {index} do not end on one row");

                let foot =
                    lines.iter().filter(on_this_field).map(|line| line.y).min().unwrap_or_else(|| panic!("{label}: field {index} has no judgement line"));
                assert!(foot < rest, "{label}: the judgement line of field {index} is not under its lanes: its foot is at {foot} and the lanes end at {rest}");
                let across = Placed { x: left, y: foot, w: right - left, h: rest - foot };
                assert!(lines.contains(&across), "{label}: no judgement line of field {index} lies at {across:?}");

                let faces_here: Vec<&Placed> = faces.iter().filter(on_this_field).collect();
                assert_eq!(faces_here.len(), side.len(), "{label}: field {index} does not have a face a lane");
                for face in faces_here {
                    assert_eq!(face.y, foot, "{label}: a lane face of field {index} at {face:?} does not end where the judgement line does");
                }
                for divider in dividers.iter().filter(on_this_field) {
                    assert_eq!(divider.y, foot, "{label}: a line between lanes of field {index} at {divider:?} does not end where the judgement line does");
                }
                for placed in on_the_lanes.iter().filter(on_this_field) {
                    assert!(placed.y >= foot, "{label}: {placed:?} is drawn below the judgement line of field {index}, whose foot is at {foot}");
                }
                let bombs_here: Vec<&Placed> = bombs.iter().filter(on_this_field).collect();
                assert!(bombs_here.len() >= side.len(), "{label}: field {index} does not have a bomb a lane");
                for bomb in bombs_here {
                    assert_eq!(bomb.w, bomb.h, "{label}: the bomb at {bomb:?} of field {index} is not a whole ring");
                    assert_eq!(bomb.y * 2 + bomb.h, foot + rest, "{label}: the bomb at {bomb:?} of field {index} is not on the middle of the judgement line");
                }
                feet.push((left, right, foot));
            }

            let document = run.app.shared.skins.document_path(&run.app.shared.config, run.screen).expect("a compiled screen has its document").to_owned();
            let taken_away = SkinOffset { a: PART_TAKEN_AWAY, ..SkinOffset::default() };
            let offsets = &mut run.app.shared.config.skin.customise(&document).offsets;
            offsets.insert(RETIRED_KEY_DISPLAY_OFFSET, taken_away);
            offsets.insert(bomb_part, taken_away);
            run.open(|_, _| {}).unwrap_or_else(|reason| panic!("{label}: not opened: {reason}"));
            run.play_to(HELD_ROWS_CHART_MS).unwrap_or_else(|reason| panic!("{label}: not played: {reason}"));
            run.assert_clean(&label);

            let (width, height) = run.authored();
            let (across, down) = (HELD_ROWS_SIZE.0 as f32 / width as f32, HELD_ROWS_SIZE.1 as f32 / height as f32);
            let ground = run.pixels.pixel_at(0, 0);
            for (left, right, foot) in feet {
                let columns = (left as f32 * across).ceil() as u32..(right as f32 * across).floor() as u32;
                let under = height - foot;
                let rows = (under as f32 * down).ceil() as u32..((under + CLEAR_UNDER_A_FIELD) as f32 * down).floor() as u32;
                assert!(!columns.is_empty() && !rows.is_empty(), "{label}: the frame has no rows under the field at {left}");
                assert_eq!(
                    run.pixels.pixel_at(columns.start, rows.start),
                    ground,
                    "{label}: the rows under the field at {left} are not the ground of the screen"
                );
                assert!(is_flat(&run.pixels, columns, rows), "{label}: something is drawn under the judgement line of the field at {left}");
            }
        }
    }
}

/// The rows of colours a document of `mode` offers, each with its items. The buttons of a field
/// with no turntable keep the colours the mode gives them, so its document has no row for the notes.
fn colour_rows(mode: &RowMode) -> Vec<(&'static str, &'static [&'static str])> {
    let mut rows: Vec<(&str, &[&str])> = vec![(THEME_ROW, &THEMES), (ACCENT_ROW, &ACCENTS), (BOMB_COLOUR_ROW, &BOMB_COLOURS)];
    if mode.mode != Some(rbms_model::Mode::POPN_9K) {
        rows.push((NOTE_COLOURS_ROW, &NOTE_COLOURS));
    }
    rows
}

/// Whichever item a row of colours is switched to, every key mode that offers the row compiles
/// with nothing said against it, plays and draws; and the ground of its screen is black, unless the
/// theme row is switched to slate, when it is the slate the pack was first drawn on. A field with
/// no turntable offers no row for the colours of its notes.
#[test]
fn the_default_pack_draws_every_play_scene_on_black_whichever_colours_its_rows_say_unless_the_theme_is_slate() {
    for mode in ROW_MODES {
        for (row, items) in colour_rows(mode) {
            for item in items {
                let rows = [(row, *item)];
                let label = label_of(mode, &rows);
                let mut run = RowsRun::enter(&format!("rows-colours-{label}"), mode, &rows, Some(HELD_ROWS_SIZE), PlayEnding::StillPlaying, false)
                    .unwrap_or_else(|reason| panic!("{label}: not entered: {reason}"));
                run.assert_clean(&label);
                run.open(|_, _| {}).unwrap_or_else(|reason| panic!("{label}: not opened: {reason}"));
                run.play_to(HELD_ROWS_CHART_MS).unwrap_or_else(|reason| panic!("{label}: not played: {reason}"));
                run.assert_clean(&label);
                assert!(!is_flat(&run.pixels, 0..HELD_ROWS_SIZE.0, 0..HELD_ROWS_SIZE.1), "{label}: the frame is one colour");
                let ground = if rows[0] == (THEME_ROW, SLATE_THEME) { SLATE_GROUND } else { MONO_GROUND };
                assert_eq!(run.pixels.pixel_at(0, 0), ground, "{label}: the ground of the screen is not the one its theme says");
            }
        }
    }
    let unoffered = RowsRun::enter("rows-colours-play9-notes", &NINE, &[MONO_NOTES], Some(HELD_ROWS_SIZE), PlayEnding::StillPlaying, false);
    assert!(unoffered.is_err(), "a field with no turntable offers a row for the colours of its notes");
}

/// One scene the pack is captured with.
struct RowShot {
    mode: &'static RowMode,
    rows: &'static [(&'static str, &'static str)],
    /// Whether the three covers of the field are drawn.
    covers: bool,
    /// Whether the run is the last stage of a course.
    course: bool,
    /// Whether the loading and the ready states are captured on the way in.
    opening: bool,
    ending: PlayEnding,
}

impl RowShot {
    /// A run the engine plays, drawn while it plays.
    const fn playing(mode: &'static RowMode, rows: &'static [(&'static str, &'static str)]) -> RowShot {
        RowShot { mode, rows, covers: false, course: false, opening: false, ending: PlayEnding::StillPlaying }
    }

    /// The same with the three covers of the field drawn.
    const fn covered(mode: &'static RowMode, rows: &'static [(&'static str, &'static str)]) -> RowShot {
        RowShot { covers: true, ..RowShot::playing(mode, rows) }
    }

    /// The same as the last stage of a course.
    const fn in_course(mode: &'static RowMode, rows: &'static [(&'static str, &'static str)]) -> RowShot {
        RowShot { course: true, ..RowShot::playing(mode, rows) }
    }

    /// A run the engine plays, drawn from its loading to its full combo.
    const fn walked(mode: &'static RowMode, rows: &'static [(&'static str, &'static str)]) -> RowShot {
        RowShot { opening: true, ending: PlayEnding::Fade, ..RowShot::playing(mode, rows) }
    }

    /// A run nobody plays, drawn as its failure closes it.
    const fn failing(mode: &'static RowMode, rows: &'static [(&'static str, &'static str)]) -> RowShot {
        RowShot { ending: PlayEnding::Failure, ..RowShot::playing(mode, rows) }
    }
}

/// The scenes that are captured: first the ones every row is left as it comes in or that leave the
/// lanes as they come, then the ones with wide lanes, then the ones with the rows of colours
/// switched.
const ROW_SHOTS: [RowShot; 46] = [
    RowShot::playing(&FIVE, &[]),
    RowShot::walked(&SEVEN, &[]),
    RowShot::playing(&NINE, &[]),
    RowShot::playing(&TEN, &[]),
    RowShot::playing(&FOURTEEN, &[]),
    RowShot::playing(&SEVEN, &[FIELD_ON_THE_RIGHT, SCRATCH_ON_THE_RIGHT, WITHOUT_SCORE_GRAPH]),
    RowShot::playing(&NINE, &[WITHOUT_SCORE_GRAPH]),
    RowShot::covered(&SEVEN, &[]),
    RowShot::failing(&SEVEN, &[]),
    RowShot::in_course(&SEVEN, &[]),
    RowShot::in_course(&NINE, &[]),
    RowShot::in_course(&FOURTEEN, &[]),
    RowShot::playing(&FIVE, &[WIDE]),
    RowShot::walked(&SEVEN, &[WIDE]),
    RowShot::playing(&NINE, &[WIDE]),
    RowShot::playing(&TEN, &[WIDE]),
    RowShot::walked(&FOURTEEN, &[WIDE]),
    RowShot::playing(&SEVEN, &[WIDE, FIELD_ON_THE_RIGHT, SCRATCH_ON_THE_RIGHT, WITHOUT_SCORE_GRAPH]),
    RowShot::playing(&SEVEN, &[WIDE, WITHOUT_SCORE_GRAPH]),
    RowShot::playing(&SEVEN, &[WIDE, FIELD_ON_THE_RIGHT]),
    RowShot::playing(&SEVEN, &[WIDE, SCRATCH_ON_THE_RIGHT]),
    RowShot::playing(&FIVE, &[WIDE, FIELD_ON_THE_RIGHT, SCRATCH_ON_THE_RIGHT]),
    RowShot::playing(&NINE, &[WIDE, WITHOUT_SCORE_GRAPH]),
    RowShot::covered(&SEVEN, &[WIDE]),
    RowShot::covered(&NINE, &[WIDE]),
    RowShot::covered(&FOURTEEN, &[WIDE]),
    RowShot::failing(&SEVEN, &[WIDE]),
    RowShot::failing(&FOURTEEN, &[WIDE]),
    RowShot::in_course(&NINE, &[WIDE]),
    RowShot::in_course(&FOURTEEN, &[WIDE]),
    RowShot::walked(&SEVEN, &SLATE_WITH_SKY),
    RowShot::covered(&SEVEN, &SLATE_WITH_SKY),
    RowShot::failing(&SEVEN, &SLATE_WITH_SKY),
    RowShot::playing(&NINE, &SLATE_WITH_SKY),
    RowShot::playing(&FOURTEEN, &SLATE_WITH_SKY),
    RowShot::walked(&SEVEN, &[SKY_ACCENT]),
    RowShot::walked(&SEVEN, &[AMBER_ACCENT]),
    RowShot::playing(&SEVEN, &[GREEN_ACCENT]),
    RowShot::playing(&SEVEN, &[PINK_ACCENT]),
    RowShot::covered(&SEVEN, &[AMBER_ACCENT]),
    RowShot::walked(&SEVEN, &[DEEP_BLUE_NOTES]),
    RowShot::walked(&SEVEN, &[YELLOW_NOTES]),
    RowShot::walked(&SEVEN, &[GREEN_NOTES]),
    RowShot::walked(&SEVEN, &[MONO_NOTES]),
    RowShot::walked(&SEVEN, &[MONO_NOTES, WHITE_BOMBS]),
    RowShot::walked(&FOURTEEN, &[YELLOW_NOTES, WHITE_BOMBS]),
];

/// Draw one scene at the size its document is authored at and save its frames as
/// `rows-<mode>-<items>[-covers][-course][-fail]-<moment>`. Answers the names that were saved.
fn capture_rows(shot: &RowShot) -> Result<Vec<String>, String> {
    use crate::skin_host::play::PlayPhase;

    let covered = if shot.covers { "-covers" } else { "" };
    let in_course = if shot.course { "-course" } else { "" };
    let failing = if shot.ending == PlayEnding::Failure { "-fail" } else { "" };
    let prefix = format!("rows-{}{covered}{in_course}{failing}", label_of(shot.mode, shot.rows));
    let mut run = RowsRun::enter(&prefix, shot.mode, shot.rows, None, shot.ending, shot.course)?;
    if shot.covers {
        let play = &mut run.app.shared.config.play;
        (play.enable_cover, play.cover) = (true, CAPTURED_COVER);
        (play.enable_lift, play.lift) = (true, CAPTURED_LIFT);
        (play.enable_hidden, play.hidden) = (true, CAPTURED_HIDDEN);
    }
    let size = run.size;
    let mut saved = Vec::new();
    let mut keep = |moment: &str, pixels: &HeadlessCanvas| {
        let name = format!("{prefix}-{moment}");
        save(&name, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(name);
    };

    let opening = shot.opening;
    run.open(|moment, pixels| {
        if opening {
            keep(moment, pixels);
        }
    })?;
    match shot.ending {
        PlayEnding::StillPlaying => {
            run.play_to(ROWS_SHOT_MS)?;
            keep("play", &run.pixels);
        }
        PlayEnding::Fade => {
            run.play_to(ROWS_HELD_SHOT_MS)?;
            keep("held", &run.pixels);
            run.play_to(ROWS_SHOT_MS)?;
            keep("play", &run.pixels);
            run.play_to(run.play_time_ms - rbms_play::PLAY_TIME_MARGIN_MS + PLAY_FULL_COMBO_AFTER_MS)?;
            keep("fullcombo", &run.pixels);
        }
        PlayEnding::Failure => {
            let mut waited = 0;
            while play_screen(&run.app)?.phase() != Some(PlayPhase::Failed) {
                run.clocks.frame_after(&mut run.app, PLAY_STEP_MS, None)?;
                waited += 1;
                if waited > PLAY_FAIL_STEPS {
                    return Err("the run nobody played never failed".to_owned());
                }
            }
            let close_ms = i64::from(run.document().play.close);
            run.clocks.play_on(&mut run.app, close_ms / 2, &mut Canvas::Headless(&mut run.pixels))?;
            keep("failed", &run.pixels);
        }
    }
    Ok(saved)
}

/// Captures the default pack's play scene with rows of its settings switched, for a person to look
/// at: without [`CAPTURE_DIR_ENV`] this passes without drawing anything. With it, every scene has to
/// draw, and the pack's folder has to be left as it was.
#[test]
fn the_play_scene_of_the_default_pack_is_captured_with_its_rows_switched() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let pack = default_pack();
    let before = files_under(&pack);
    for shot in &ROW_SHOTS {
        let label = label_of(shot.mode, shot.rows);
        let saved = capture_rows(shot).unwrap_or_else(|reason| panic!("{label}: not captured: {reason}"));
        println!("{label}: captured {}", saved.join(", "));
    }
    assert_eq!(files_under(&pack), before, "capturing the default pack's play scene changed its folder");
}

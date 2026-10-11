//! The default pack's play field as a player who set it up sees it: a judgement that stays where it
//! is put, and parts that are taken off the screen one by one.
//!
//! The pack's play documents let a player say how opaque each part of the screen is, where the
//! judgement stands over the field and whether it is shown at all. Each of those is an offset or a
//! row of the document, and none of them is read by the captures of the sibling modules, which draw
//! the documents as they come. Here a run is entered with rows switched and offsets set the way the
//! skin menu stores them, and played by hand a beat at a time or left to the engine. A press is made
//! with what the sibling module of runs by hand makes one with ([`by_hand`]).
//!
//! What the pack is held to on every machine, on frames drawn at half its size:
//!
//! - a judgement is drawn alike on every frame from the one it is made on to the last one it is
//!   shown on, and is gone after that: nothing of it moves, grows, fades or changes colour;
//! - its word starts on the same column whether the combo beside it has one figure, two or three;
//! - the offset that says where the judgement stands moves it up by as many pixels as it is set to
//!   and down by as many when it is set below nothing, and the row that hides it leaves none of it;
//! - with every part at no opacity and the judgement hidden a frame of a run that is being played is
//!   one colour, and with any one part back at full opacity it no longer is;
//! - the keys of the skin menu take the parts off the field of a run as setting their offsets does:
//!   what the menu stores is what the run is drawn with.
//!
//! Two more draw for a person to look at, and only where [`CAPTURE_DIR_ENV`] names a folder. One
//! draws the default pack at the size it is authored at: a judgement frame by frame, the judgements a
//! press can take, the judgement moved and hidden, the field with parts taken off it, and the wider
//! fields. The other draws the judgement of the pack [`SKIN_PACK_ENV`] names frame by frame, so what
//! another pack does with a judgement can be laid beside it.

use rbms_config::SettingTab;
use rbms_skin::dst::SkinOffset;

use super::by_hand::{self, FIRST_SIDE_KEY, GOOD_OFF_MS, GREAT_OFF_MS, HAND_HOLD_MS, HELD_SIZE, SECOND_SIDE_KEY};
use super::*;
use crate::stage::{SkinMenuState, StageHandler};

/// The tempo of a chart played a beat at a time, how long one of its beats lasts at it, and how many
/// of them a measure has.
const BEAT_BPM: u32 = 120;
const BEAT_MS: i64 = 500;
const BEATS_A_MEASURE: usize = 4;

/// The measure of the first note, which leaves the field time to empty of its opening before it.
const FIRST_BEAT_MEASURE: usize = 1;

/// How many measures past the notes that are played the chart's last note is, so the run is still
/// playing when its last frame is drawn.
const MEASURES_LEFT_OVER: usize = 30;

/// The moments after a press the frames its judgement is held still over are drawn at, in
/// milliseconds: one frame apart at first, which is where a judgement that pops would be moving, then
/// through the rest of the half second the default pack shows a judgement for, to its last frame.
const STILL_FRAMES_MS: [i64; 10] = [0, 16, 32, 48, 64, 80, 96, 250, 440, 496];

/// A moment after a press by which its judgement is gone and the next note is not yet due.
const JUDGEMENT_GONE_MS: i64 = 540;

/// The moment after a press a frame of its judgement is drawn at for a person to look at.
const JUDGEMENT_LOOKED_AT_MS: i64 = 200;

/// The combos a judgement's word is held to the same place beside.
const COUNTED_COMBOS: [usize; 3] = [9, 10, 100];

/// The rows of the pack's play documents that say what is written over the judgement and whether the
/// judgement is drawn, each with an item of it.
const NO_DIFFERENCE: (&str, &str) = ("Score Difference", "Off");
const BEST_DIFFERENCE: (&str, &str) = ("Score Difference", "Best");
const NO_DETAIL: (&str, &str) = ("Judge Detail", "Off");
const MILLISECONDS: (&str, &str) = ("Judge Detail", "Milliseconds");
const HIDDEN_JUDGEMENT: (&str, &str) = ("Judge Display", "Hide");
const WIDE_LANES: (&str, &str) = ("Lane Width", "Wide");
const FIELD_ON_THE_RIGHT: (&str, &str) = ("Field Side", "Right");

/// The offset of the pack's play documents that says where the judgement stands, and how far it is
/// moved in the frames that hold it to that, in pixels of the screen.
const JUDGE_POSITION: &str = "Judge Position";
const JUDGEMENT_MOVED_BY: f32 = 100.0;

/// What the name of an offset that says how opaque a part is ends with.
const OPACITY_SUFFIX: &str = " Opacity";

/// The opacity an offset leaves a part at when nothing is set, in percent, and the alpha a part that
/// is fully opaque has.
const FULL_OPACITY: f32 = 100.0;
const OPAQUE_ALPHA: f32 = 255.0;

/// A part at no opacity and one at half of it, in percent.
const NO_OPACITY: f32 = 0.0;
const HALF_OPACITY: f32 = 50.0;

/// The parts a player who wants nothing on the field but what is played takes off the screen: the
/// faces of the lanes, the lines between them, and the readouts around the field.
const BARE_FIELD_PARTS: [&str; 9] = [
    "Lane Background Opacity",
    "Lane Divider Opacity",
    "Key Display Opacity",
    "Bar Line Opacity",
    "Progress Bar Opacity",
    "Score Graph Opacity",
    "Song Header Opacity",
    "Speed Display Opacity",
    "Timing Opacity",
];

/// How many presses take a GOOD ahead of the one captured in the frame that shows a run behind what
/// it is played against.
const STROKES_BEHIND: usize = 70;

/// The moment of a chart the engine plays that its frame is drawn at, in milliseconds.
const PLAYED_SHOT_MS: i64 = 9_000;

/// The shares of the field the three covers take in the frames that draw them.
const PART_COVER: f32 = 0.3;
const PART_LIFT: f32 = 0.15;
const PART_HIDDEN: f32 = 0.1;

/// How many presses of an arrow with Shift held take a part from all of its opacity to none on the
/// skin menu, and what its row reads then.
const WIDE_STEPS_TO_NOTHING: usize = 4;
const NOTHING_ON_THE_MENU: &str = "0%";

/// The offset that leaves a part at `percent` of its opacity: what is added to its alpha, which is
/// nothing for a part left whole.
fn opacity(percent: f32) -> SkinOffset {
    SkinOffset { a: ((percent - FULL_OPACITY) * OPAQUE_ALPHA / FULL_OPACITY).round(), ..SkinOffset::default() }
}

/// The offset that moves what it is applied to up by `pixels` of the screen.
fn raised(pixels: f32) -> SkinOffset {
    SkinOffset { y: pixels, ..SkinOffset::default() }
}

/// A chart with a note on every beat of `channel` from [`FIRST_BEAT_MEASURE`] on, as many as
/// `strokes`, and one far behind them.
fn beat_chart(channel: &str, strokes: usize) -> String {
    let mut chart = format!(
        "#PLAYER 1\n#GENRE Capture Genre\n#TITLE Played A Beat At A Time\n#ARTIST Capture Artist\n#BPM {BEAT_BPM}\n#PLAYLEVEL 3\n#DIFFICULTY 2\n#RANK 3\n#TOTAL 300\n#WAV01 a.wav\n"
    );
    let measures = strokes.div_ceil(BEATS_A_MEASURE);
    for measure in 0..measures {
        let notes = (strokes - measure * BEATS_A_MEASURE).min(BEATS_A_MEASURE);
        let beats = format!("{}{}", "01".repeat(notes), "00".repeat(BEATS_A_MEASURE - notes));
        chart.push_str(&format!("#{:03}{channel}:{beats}\n", FIRST_BEAT_MEASURE + measure));
    }
    chart.push_str(&format!("#{:03}{channel}:01\n", FIRST_BEAT_MEASURE + measures + MEASURES_LEFT_OVER));
    chart
}

/// When the note of the stroke at `index` is, in milliseconds of the chart.
fn beat_ms(index: usize) -> i64 {
    (FIRST_BEAT_MEASURE * BEATS_A_MEASURE + index) as i64 * BEAT_MS
}

/// Who plays a run and what its chart is.
#[derive(Clone, Copy)]
enum Played<'a> {
    /// The harness, a beat at a time, on a chart of this many notes on `channel`.
    ByHand { channel: &'a str, strokes: usize },
    /// The engine, on the chart written for the mode with notes on every lane of `sides`.
    ByTheEngine { sides: &'a [SideChannels] },
}

/// What a run of the play field is entered with: the settings folder of its own app, the pack, the
/// mode and who plays it, the size it is drawn at (the pack's own when `None`), and the rows and the
/// offsets of the pack's document that are set before the document is read.
struct FieldSetup<'a> {
    tag: &'a str,
    pack: &'a Path,
    mode: rbms_model::Mode,
    played: Played<'a>,
    size: Option<(u32, u32)>,
    rows: &'a [(&'a str, &'a str)],
    offsets: &'a [(&'a str, SkinOffset)],
}

/// A pack's play scene entered for one run and walked until its chart plays.
struct FieldRun {
    app: App,
    screen: i32,
    size: (u32, u32),
    clocks: PlayClocks,
    /// The lane the notes of a run played by hand are on.
    lane: usize,
    pixels: HeadlessCanvas,
    _keysounds: std::sync::mpsc::Sender<(u32, rbms_audio::DecodedAudio)>,
}

impl FieldRun {
    /// Enter the play scene `setup` describes and walk it until the chart plays. Answers why the
    /// scene was not entered, which a row or an offset the document does not have is one reason for.
    fn enter(setup: &FieldSetup<'_>) -> Result<FieldRun, String> {
        let name = format!("{}.bms", setup.tag);
        let (text, ending) = match setup.played {
            Played::ByHand { channel, strokes } => (beat_chart(channel, strokes), PlayEnding::Failure),
            Played::ByTheEngine { sides } => (mode_chart("Played By The Engine", sides), PlayEnding::StillPlaying),
        };
        let model = chart_model(&text, &name, Some(setup.mode));
        let lane = by_hand::played_lane(&model)?;
        let chart = settings_of(&format!("{}-chart", setup.tag)).with_file_name(&name);
        let EnteredPlay { mut app, screen, size, play_time_ms: _, arrived, _keysounds } = enter_play_scene(setup.pack, setup.tag, &chart, model, ending)?;
        for (row, item) in setup.rows {
            if !choose_in_skin(&mut app, screen, row, item) {
                return Err(format!("the document has no row {row:?} with an item {item:?}"));
            }
        }
        app.shared.skins.request_for(&app.shared.config, screen);
        let size = setup.size.unwrap_or(size);
        let mut run = FieldRun { app, screen, size, clocks: PlayClocks::default(), lane, pixels: HeadlessCanvas::new(size.0, size.1), _keysounds };
        for (offset, value) in setup.offsets {
            run.set(offset, *value)?;
        }
        wait_for_scene_document(&mut run.app, screen, &mut Canvas::Headless(&mut run.pixels))?;
        let skin = run.document();
        let timings = (i64::from(skin.play.loadend), i64::from(skin.play.playstart));
        run.clocks.open(&mut run.app, &mut run.pixels, &arrived, timings, |_, _, _, _| Ok(()))?;
        Ok(run)
    }

    /// The document the scene is drawn with.
    fn document(&self) -> &rbms_skin::loader::LoadedSkin {
        self.app.shared.skins.document(self.screen).expect("a compiled screen has its document")
    }

    /// The names of the offsets of the document that say how opaque a part is.
    fn parts(&self) -> Vec<String> {
        let header = self.app.shared.skins.header_of(&self.app.shared.config, self.screen).expect("a pack's screen has its header");
        header.offsets.iter().filter(|offset| offset.name.ends_with(OPACITY_SUFFIX)).map(|offset| offset.name.clone()).collect()
    }

    /// Set the offset called `name` the way the skin menu stores one. An offset is read out of the
    /// stored choices on every frame, so the next frame is drawn with it.
    fn set(&mut self, name: &str, value: SkinOffset) -> Result<(), String> {
        let shared = &mut self.app.shared;
        let id = shared
            .skins
            .header_of(&shared.config, self.screen)
            .and_then(|header| header.offsets.iter().find(|offset| offset.name == name))
            .map(|offset| offset.id);
        let document = shared.skins.document_path(&shared.config, self.screen).map(str::to_owned);
        let (Some(id), Some(document)) = (id, document) else {
            return Err(format!("the document has no offset {name:?}"));
        };
        shared.config.skin.customise(&document).offsets.insert(id, value);
        Ok(())
    }

    /// Set every part of the document to `percent` of its opacity.
    fn set_every_part(&mut self, percent: f32) -> Result<(), String> {
        self.parts().iter().try_for_each(|part| self.set(part, opacity(percent)))
    }

    /// Put the key of the run's lane down `lead_ms` ahead of the note at `index`, and answer when.
    fn press(&mut self, index: usize, lead_ms: i64) -> Result<i64, String> {
        let down_ms = beat_ms(index) - lead_ms;
        by_hand::press(&mut self.app, &mut self.clocks, self.lane, down_ms)?;
        Ok(down_ms)
    }

    /// Let the key that went down at `down_ms` up again, [`HAND_HOLD_MS`] later.
    fn release(&mut self, down_ms: i64) -> Result<(), String> {
        by_hand::release(&mut self.app, &mut self.clocks, self.lane, down_ms)
    }

    /// Play the note at `index` with a press `lead_ms` ahead of it, and answer when the key went down.
    fn strike(&mut self, index: usize, lead_ms: i64) -> Result<i64, String> {
        let down_ms = self.press(index, lead_ms)?;
        self.release(down_ms)?;
        Ok(down_ms)
    }

    /// Play every note before the one at `index` on the beat.
    fn strike_up_to(&mut self, from: usize, index: usize) -> Result<(), String> {
        (from..index).try_for_each(|played| self.strike(played, 0).map(|_| ()))
    }

    /// Run the scene on to `at_ms` of the chart and draw the frame of that moment.
    fn draw_at(&mut self, at_ms: i64) -> Result<(), String> {
        by_hand::run_to(&mut self.app, &mut self.clocks, at_ms)?;
        self.redraw()
    }

    /// Draw the moment the scene stands at once more, with whatever was set since.
    fn redraw(&mut self) -> Result<(), String> {
        self.clocks.frame_after(&mut self.app, 0, Some(&mut Canvas::Headless(&mut self.pixels)))
    }

    /// Save the frame that was drawn last under `name`, when a capture folder was asked for.
    fn keep(&self, name: &str) {
        save(name, CAPTURE_EXTENSION, self.size, self.pixels.rgba());
    }

    /// The columns and the rows of the frame the first field's lanes take up above its judgement
    /// line, which is where a judgement is drawn.
    fn field(&self) -> (Range<u32>, Range<u32>) {
        self.field_of(0, 1)
    }

    /// The same for the field at `index` of a document that has `fields` of them.
    fn field_of(&self, index: usize, fields: usize) -> (Range<u32>, Range<u32>) {
        let skin = self.document();
        let lanes = &skin.def.note.as_ref().expect("a play document has a note field").dst;
        let side = lanes.chunks(lanes.len().div_ceil(fields).max(1)).nth(index).expect("the document has the field asked for");
        let across = self.size.0 as f32 / skin.def.w.max(1) as f32;
        let down = self.size.1 as f32 / skin.def.h.max(1) as f32;
        let left = side.iter().filter_map(|lane| lane.x).min().unwrap_or_default();
        let right = side.iter().filter_map(|lane| Some(lane.x? + lane.w?)).max().unwrap_or_default();
        let line = side.iter().filter_map(|lane| lane.y).max().unwrap_or_default();
        let columns = (left as f32 * across).ceil() as u32..(right as f32 * across).floor() as u32;
        (columns, 0..((skin.def.h - line) as f32 * down).floor() as u32)
    }

    /// The pixels of `columns` and `rows` of the frame that was drawn last, row by row.
    fn cut(&self, columns: &Range<u32>, rows: &Range<u32>) -> Vec<crate::Color> {
        rows.clone().flat_map(|y| columns.clone().map(move |x| (x, y))).map(|(x, y)| self.pixels.pixel_at(x, y)).collect()
    }

    /// The box of the pixels of `columns` and `rows` that are not the colour of the first of them:
    /// its left, its top, its right and its bottom. `None` when the rectangle is one colour.
    fn ink(&self, columns: &Range<u32>, rows: &Range<u32>) -> Option<InkBox> {
        let ground = self.pixels.pixel_at(columns.start, rows.start);
        let mut found: Option<InkBox> = None;
        for y in rows.clone() {
            for x in columns.clone() {
                if self.pixels.pixel_at(x, y) != ground {
                    let bounds = found.get_or_insert(InkBox { left: x, top: y, right: x, bottom: y });
                    (bounds.left, bounds.right) = (bounds.left.min(x), bounds.right.max(x));
                    bounds.bottom = y;
                }
            }
        }
        found
    }

    /// Whether the whole frame that was drawn last is one colour.
    fn is_blank(&self) -> bool {
        is_flat(&self.pixels, 0..self.size.0, 0..self.size.1)
    }
}

/// Take the parts called `parts` off the screen of the play document of `screen` with the keys of the
/// skin menu, the way a player does it: the menu is opened over the app's own settings on that
/// screen, the cursor is walked to the row of each part, the row is stepped down with Shift held
/// until it reads nothing, and Escape writes the settings out and leaves.
///
/// The menu is not put up as the app's screen: the run that is being played stays where it is, and
/// its next frame is drawn with what the menu stored.
fn take_off_with_the_skin_menu(app: &mut App, screen: i32, parts: &[String]) -> Result<(), String> {
    app.shared.config.skin.screen = screen;
    let mut menu = SkinMenuState::new();
    let key = |app: &mut App, menu: &mut SkinMenuState, input: KeyInput<'static>| {
        menu.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, input)
    };
    menu.on_enter(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
    let rows = app.shared.settings_rows(SettingTab::Skin);
    let mut cursor = 0;
    for part in parts {
        let label = part.to_ascii_uppercase();
        let at = rows
            .iter()
            .position(|row| app.shared.settings_scene_line(*row).label == label)
            .ok_or_else(|| format!("the skin menu lists no row called {label}"))?;
        let (towards, presses) = if at >= cursor { (KeyCode::ArrowDown, at - cursor) } else { (KeyCode::ArrowUp, cursor - at) };
        for _ in 0..presses {
            key(app, &mut menu, key_down(towards));
        }
        cursor = at;
        key(app, &mut menu, key_down(KeyCode::ShiftLeft));
        for _ in 0..WIDE_STEPS_TO_NOTHING {
            key(app, &mut menu, key_down(KeyCode::ArrowLeft));
        }
        key(app, &mut menu, key_up(KeyCode::ShiftLeft));
        let reads = app.shared.settings_scene_line(rows[at]).value;
        if reads != NOTHING_ON_THE_MENU {
            return Err(format!("{label} reads {reads} after {WIDE_STEPS_TO_NOTHING} wide steps down"));
        }
    }
    match key(app, &mut menu, key_down(KeyCode::Escape)) {
        Transition::Back => Ok(()),
        _ => Err("Escape did not leave the skin menu".to_owned()),
    }
}

/// The box of what is drawn on a ground of one colour, its sides included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InkBox {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

/// The parts that are drawn over the lanes beside the judgement. With these at no opacity the
/// judgement is the only thing over the ground of the field, so its box can be measured.
const AROUND_THE_JUDGEMENT: [&str; 8] = [
    "Lane Background Opacity",
    "Lane Divider Opacity",
    "Judge Line Opacity",
    "Key Beam Opacity",
    "Bomb Opacity",
    "Note Opacity",
    "Bar Line Opacity",
    "Lane Cover Opacity",
];

/// A run of the default pack by hand on seven keys, drawn at [`HELD_SIZE`] with nothing over
/// its lanes but its judgement, and with `rows` switched and `offsets` set.
fn judged_run(tag: &str, strokes: usize, rows: &[(&str, &str)], offsets: &[(&str, SkinOffset)]) -> FieldRun {
    let pack = default_pack();
    let cleared = AROUND_THE_JUDGEMENT.iter().map(|part| (*part, opacity(NO_OPACITY)));
    let offsets: Vec<(&str, SkinOffset)> = cleared.chain(offsets.iter().copied()).collect();
    let played = Played::ByHand { channel: FIRST_SIDE_KEY, strokes };
    let setup = FieldSetup { tag, pack: &pack, mode: rbms_model::Mode::BEAT_7K, played, size: Some(HELD_SIZE), rows, offsets: &offsets };
    FieldRun::enter(&setup).unwrap_or_else(|reason| panic!("{tag}: not entered: {reason}"))
}

/// A judgement of the default pack is drawn alike on every frame from the one it is made on to the
/// last one it is shown on, with the line over it and without, and is gone after that.
#[test]
fn the_default_pack_holds_a_judgement_still_for_as_long_as_it_is_shown() {
    let plain: [(&str, &str); 2] = [NO_DIFFERENCE, NO_DETAIL];
    for (label, rows, lead_ms) in [("plain", plain.as_slice(), 0), ("lined", [].as_slice(), GREAT_OFF_MS)] {
        let mut run = judged_run(&format!("parts-still-{label}"), 1, rows, &[]);
        let (columns, rows) = run.field();
        let down_ms = run.press(0, lead_ms).unwrap_or_else(|reason| panic!("{label}: not pressed: {reason}"));
        run.redraw().unwrap_or_else(|reason| panic!("{label}: not drawn: {reason}"));
        let first = run.cut(&columns, &rows);
        let stood = run.ink(&columns, &rows).unwrap_or_else(|| panic!("{label}: nothing is drawn over the field on the frame a press is judged on"));
        run.release(down_ms).unwrap_or_else(|reason| panic!("{label}: not released: {reason}"));
        for after_ms in STILL_FRAMES_MS {
            run.draw_at(down_ms + after_ms).unwrap_or_else(|reason| panic!("{label}: not drawn: {reason}"));
            assert_eq!(run.ink(&columns, &rows), Some(stood), "{label}: the judgement stands elsewhere {after_ms} ms after it was made");
            assert!(run.cut(&columns, &rows) == first, "{label}: the judgement is drawn otherwise {after_ms} ms after it was made");
        }
        run.draw_at(down_ms + JUDGEMENT_GONE_MS).unwrap_or_else(|reason| panic!("{label}: not drawn: {reason}"));
        assert_eq!(run.ink(&columns, &rows), None, "{label}: the judgement is still drawn {JUDGEMENT_GONE_MS} ms after it was made");
    }
}

/// The word of a judgement starts on the same column and is drawn alike whether the combo beside it
/// has one figure, two or three, and the two of them take up the same box.
#[test]
fn the_default_pack_keeps_the_word_of_a_judgement_in_place_as_its_combo_grows() {
    let last = COUNTED_COMBOS[COUNTED_COMBOS.len() - 1];
    let mut run = judged_run("parts-combo", last, &[NO_DIFFERENCE, NO_DETAIL], &[]);
    let (columns, rows) = run.field();
    let word = {
        let skin = run.document();
        let placed = skin.def.judge.first().and_then(|judge| judge.images.first()).and_then(|word| word.dst.first()).expect("the document places a judgement");
        let across = run.size.0 as f32 / skin.def.w.max(1) as f32;
        let (x, w) = (placed.x.unwrap_or_default() as f32, placed.w.unwrap_or_default() as f32);
        (x * across).floor() as u32..((x + w) * across).ceil() as u32
    };
    let mut played = 0;
    let mut seen: Vec<(usize, InkBox, Vec<crate::Color>)> = Vec::new();
    for combo in COUNTED_COMBOS {
        run.strike_up_to(played, combo - 1).unwrap_or_else(|reason| panic!("not played up to a combo of {combo}: {reason}"));
        let down_ms = run.strike(combo - 1, 0).unwrap_or_else(|reason| panic!("the note of combo {combo} was not played: {reason}"));
        played = combo;
        run.draw_at(down_ms + JUDGEMENT_LOOKED_AT_MS).unwrap_or_else(|reason| panic!("combo {combo}: not drawn: {reason}"));
        let judge = play_screen(&run.app).unwrap_or_else(|reason| panic!("combo {combo}: {reason}")).session.judge();
        assert_eq!(judge.combo as usize, combo, "the run stands at another combo than the notes played on the beat: {:?}", judge.counts);
        let stood = run.ink(&columns, &rows).unwrap_or_else(|| panic!("combo {combo}: nothing is drawn over the field"));
        seen.push((combo, stood, run.cut(&word, &rows)));
    }
    let (_, first_box, first_word) = &seen[0];
    for (combo, stood, drawn) in &seen[1..] {
        assert_eq!(stood, first_box, "the judgement takes up another box at a combo of {combo} than at one of {}", COUNTED_COMBOS[0]);
        assert!(drawn == first_word, "the word of the judgement is drawn otherwise at a combo of {combo} than at one of {}", COUNTED_COMBOS[0]);
    }
}

/// The offset that says where the judgement stands moves all of it up by as many pixels of the
/// screen as it is set to, and down by as many when it is set below nothing; and the row that hides
/// the judgement leaves none of it on the field.
#[test]
fn the_default_pack_moves_its_judgement_by_its_position_and_hides_it_when_told_to() {
    let stood_with = |label: &str, rows: &[(&str, &str)], offsets: &[(&str, SkinOffset)]| {
        let mut run = judged_run(&format!("parts-position-{label}"), 1, rows, offsets);
        let (columns, rows) = run.field();
        let down_ms = run.strike(0, GREAT_OFF_MS).unwrap_or_else(|reason| panic!("{label}: not played: {reason}"));
        run.draw_at(down_ms + JUDGEMENT_LOOKED_AT_MS).unwrap_or_else(|reason| panic!("{label}: not drawn: {reason}"));
        run.ink(&columns, &rows)
    };
    let moved = JUDGEMENT_MOVED_BY as u32;
    let home = stood_with("home", &[], &[]).expect("a judgement and the line over it are drawn over the field");
    let up = stood_with("up", &[], &[(JUDGE_POSITION, raised(JUDGEMENT_MOVED_BY))]);
    let down = stood_with("down", &[], &[(JUDGE_POSITION, raised(-JUDGEMENT_MOVED_BY))]);
    assert_eq!(
        up,
        Some(InkBox { top: home.top - moved, bottom: home.bottom - moved, ..home }),
        "a position above nothing does not move the judgement up by it"
    );
    assert_eq!(
        down,
        Some(InkBox { top: home.top + moved, bottom: home.bottom + moved, ..home }),
        "a position below nothing does not move the judgement down by it"
    );
    assert_eq!(stood_with("hidden", &[HIDDEN_JUDGEMENT], &[]), None, "a judgement that is hidden is drawn all the same");
}

/// With every part at no opacity and the judgement hidden, a frame of a run that is being played is
/// one colour: while a key is down, while what a key lit fades, and with the covers of the field
/// drawn. And with any one part back at full opacity it no longer is, so each part is taken off the
/// screen by its own offset and by no other.
#[test]
fn the_default_pack_draws_nothing_of_a_part_at_no_opacity() {
    /// How long after a key went up what it lit is still fading.
    const FADING_AFTER_RELEASE_MS: i64 = 40;
    /// The note the key is held down on: the last beat of a measure, so the line of the next
    /// measure is on the field with it.
    const HELD_NOTE: usize = BEATS_A_MEASURE - 1;
    let pack = default_pack();
    let played = Played::ByHand { channel: FIRST_SIDE_KEY, strokes: BEATS_A_MEASURE };
    let setup =
        FieldSetup { tag: "parts-none", pack: &pack, mode: rbms_model::Mode::BEAT_7K, played, size: Some(HELD_SIZE), rows: &[HIDDEN_JUDGEMENT], offsets: &[] };
    let mut run = FieldRun::enter(&setup).unwrap_or_else(|reason| panic!("not entered: {reason}"));
    let play = &mut run.app.shared.config.play;
    (play.enable_cover, play.cover) = (true, PART_COVER);
    (play.enable_lift, play.lift) = (true, PART_LIFT);
    (play.enable_hidden, play.hidden) = (true, PART_HIDDEN);
    let parts = run.parts();
    assert!(!parts.is_empty(), "the document declares no part to take off the screen");
    run.set_every_part(NO_OPACITY).unwrap_or_else(|reason| panic!("{reason}"));

    let down_ms = run.strike(0, 0).unwrap_or_else(|reason| panic!("not played: {reason}"));
    run.draw_at(down_ms + HAND_HOLD_MS + FADING_AFTER_RELEASE_MS).unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    assert!(run.is_blank(), "something of a part at no opacity is drawn while what a key lit fades");

    run.strike_up_to(1, HELD_NOTE).unwrap_or_else(|reason| panic!("not played: {reason}"));
    run.press(HELD_NOTE, 0).unwrap_or_else(|reason| panic!("not pressed: {reason}"));
    run.redraw().unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    assert!(run.is_blank(), "something of a part at no opacity is drawn while a key is down");
    for part in &parts {
        run.set(part, opacity(FULL_OPACITY)).unwrap_or_else(|reason| panic!("{reason}"));
        run.redraw().unwrap_or_else(|reason| panic!("{part}: not drawn: {reason}"));
        assert!(!run.is_blank(), "{part}: nothing is drawn of the one part left at full opacity");
        run.set(part, opacity(NO_OPACITY)).unwrap_or_else(|reason| panic!("{reason}"));
    }
    assert_eq!(run.app.shared.skin_warnings(run.screen), &[] as &[String], "the document compiled with warnings");
}

/// The keys of the skin menu take the parts of a play document off the field of a run that is being
/// played: with every part stepped down to nothing on the menu and the judgement hidden, the next
/// frame of the run is one colour, as it is with the offsets set where the menu stores them. What
/// the menu wrote out is the offset that takes all of a part's alpha away, for each part.
#[test]
fn the_keys_of_the_skin_menu_take_the_parts_of_the_default_pack_off_the_field() {
    let pack = default_pack();
    let played = Played::ByHand { channel: FIRST_SIDE_KEY, strokes: BEATS_A_MEASURE };
    let setup =
        FieldSetup { tag: "parts-menu", pack: &pack, mode: rbms_model::Mode::BEAT_7K, played, size: Some(HELD_SIZE), rows: &[HIDDEN_JUDGEMENT], offsets: &[] };
    let mut run = FieldRun::enter(&setup).unwrap_or_else(|reason| panic!("not entered: {reason}"));
    let parts = run.parts();
    assert!(!parts.is_empty(), "the document declares no part to take off the screen");
    let down_ms = run.press(0, 0).unwrap_or_else(|reason| panic!("not pressed: {reason}"));
    run.redraw().unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    assert!(!run.is_blank(), "nothing is drawn of a field nobody set up");

    let screen = run.screen;
    take_off_with_the_skin_menu(&mut run.app, screen, &parts).unwrap_or_else(|reason| panic!("{reason}"));
    run.redraw().unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    assert!(run.is_blank(), "something of a part the skin menu took to nothing is drawn while a key is down");
    run.release(down_ms).unwrap_or_else(|reason| panic!("not released: {reason}"));

    let shared = &run.app.shared;
    let header = shared.skins.header_of(&shared.config, screen).expect("a pack's screen has its header");
    let document = shared.skins.document_path(&shared.config, screen).expect("a pack's screen has its document");
    let stored = &shared.config.skin.customisation(document).expect("the menu stored what it was stepped to").offsets;
    for part in &parts {
        let id = header.offsets.iter().find(|offset| offset.name == *part).map(|offset| offset.id).expect("a part is an offset of the header");
        assert_eq!(stored.get(&id), Some(&opacity(NO_OPACITY)), "{part} is not stored as the offset that takes all of its alpha away");
    }
    let written = rbms_config::load(&shared.settings_path).unwrap_or_else(|reason| panic!("the settings Escape wrote do not read back: {reason:?}"));
    assert_eq!(written.config.skin.customisation(document).map(|entry| &entry.offsets), Some(stored), "Escape did not write the parts out");
}

/// The lowercase name a capture of a part is saved under.
fn file_name_of(part: &str) -> String {
    part.trim_end_matches(OPACITY_SUFFIX).replace(' ', "-").to_ascii_lowercase()
}

/// Draw a judgement of the pack at `pack` frame by frame on seven keys at the size the pack is
/// authored at, as `<prefix>-<judgement>-<ms after the press>`: one that came on the beat and one
/// that came early. Answers what was saved.
fn capture_a_judgement_frame_by_frame(pack: &Path, prefix: &str) -> Result<Vec<String>, String> {
    let strokes = [("on-the-beat", 0), ("early", GREAT_OFF_MS)];
    let played = Played::ByHand { channel: FIRST_SIDE_KEY, strokes: strokes.len() * BEATS_A_MEASURE };
    let setup = FieldSetup { tag: prefix, pack, mode: rbms_model::Mode::BEAT_7K, played, size: None, rows: &[], offsets: &[] };
    let mut run = FieldRun::enter(&setup)?;
    let mut saved = Vec::new();
    for (index, (judgement, lead_ms)) in strokes.into_iter().enumerate() {
        let note = index * BEATS_A_MEASURE;
        run.strike_up_to(note.saturating_sub(BEATS_A_MEASURE - 1), note)?;
        let down_ms = run.press(note, lead_ms)?;
        run.redraw()?;
        let mut released = false;
        for after_ms in STILL_FRAMES_MS.into_iter().chain([JUDGEMENT_GONE_MS]) {
            if !released && after_ms >= HAND_HOLD_MS {
                run.release(down_ms)?;
                released = true;
            }
            run.draw_at(down_ms + after_ms)?;
            let name = format!("{prefix}-{judgement}-{after_ms:03}ms");
            run.keep(&name);
            saved.push(name);
        }
    }
    Ok(saved)
}

/// Draw the judgements a press can take on the default pack's field, each with the line over it, and
/// the judgement beside combos of one figure, two and three: `<prefix>-<name>`.
fn capture_the_judgements(prefix: &str, mode: rbms_model::Mode, channel: &str, rows: &[(&str, &str)]) -> Result<Vec<String>, String> {
    let last = COUNTED_COMBOS[COUNTED_COMBOS.len() - 1];
    let strokes: [(&str, Option<i64>); 6] = [
        ("great-fast", Some(GREAT_OFF_MS)),
        ("great-slow", Some(-GREAT_OFF_MS)),
        ("good-fast", Some(GOOD_OFF_MS)),
        ("good-slow", Some(-GOOD_OFF_MS)),
        ("missed", None),
        ("on-the-beat", Some(0)),
    ];
    let pack = default_pack();
    let played = Played::ByHand { channel, strokes: last + strokes.len() };
    let mut run = FieldRun::enter(&FieldSetup { tag: prefix, pack: &pack, mode, played, size: None, rows, offsets: &[] })?;
    let mut saved = Vec::new();
    let mut played = 0;
    for combo in COUNTED_COMBOS {
        run.strike_up_to(played, combo - 1)?;
        let down_ms = run.strike(combo - 1, 0)?;
        played = combo;
        run.draw_at(down_ms + JUDGEMENT_LOOKED_AT_MS)?;
        let name = format!("{prefix}-combo-{combo:03}");
        run.keep(&name);
        saved.push(name);
    }
    for (index, (name, lead_ms)) in strokes.into_iter().enumerate() {
        let note = last + index;
        match lead_ms {
            Some(lead_ms) => {
                let down_ms = run.strike(note, lead_ms)?;
                run.draw_at(down_ms + JUDGEMENT_LOOKED_AT_MS)?;
            }
            None => run.draw_at(beat_ms(note) + BEAT_MS - GOOD_OFF_MS - PLAY_STEP_MS)?,
        }
        let name = format!("{prefix}-{name}");
        run.keep(&name);
        let judge = play_screen(&run.app)?.session.judge();
        saved.push(format!("{name}: combo {}, EX {}, PG/GR/GD/BD/PR/MS {:?}", judge.combo, judge.ex_score, judge.counts));
    }
    Ok(saved)
}

/// Draw one judgement of the default pack on seven keys with `rows` switched and `offsets` set, as
/// `<prefix>`: a GREAT that came early, after `behind` presses that each took a GOOD and left the run
/// that much further behind what it is played against.
fn capture_a_judgement_with(prefix: &str, behind: usize, rows: &[(&str, &str)], offsets: &[(&str, SkinOffset)]) -> Result<Vec<String>, String> {
    let pack = default_pack();
    let played = Played::ByHand { channel: FIRST_SIDE_KEY, strokes: behind + 1 };
    let mut run = FieldRun::enter(&FieldSetup { tag: prefix, pack: &pack, mode: rbms_model::Mode::BEAT_7K, played, size: None, rows, offsets })?;
    (0..behind).try_for_each(|note| run.strike(note, GOOD_OFF_MS).map(|_| ()))?;
    let down_ms = run.strike(behind, GREAT_OFF_MS)?;
    run.draw_at(down_ms + JUDGEMENT_LOOKED_AT_MS)?;
    run.keep(prefix);
    Ok(vec![prefix.to_owned()])
}

/// Draw a run the engine plays of `mode` with `rows` switched, the covers of its field drawn when
/// `covered`: as it comes (`<prefix>-whole`), with the parts of [`BARE_FIELD_PARTS`] taken off
/// (`-bare`), and, when `each` says so, with each part alone at no opacity (`-without-<part>`) and
/// at half of it (`-half-<part>`), and with every part at half of it (`-half`).
fn capture_the_parts(
    prefix: &str,
    mode: rbms_model::Mode,
    sides: &[SideChannels],
    rows: &[(&str, &str)],
    covered: bool,
    each: bool,
) -> Result<Vec<String>, String> {
    let pack = default_pack();
    let mut run = FieldRun::enter(&FieldSetup { tag: prefix, pack: &pack, mode, played: Played::ByTheEngine { sides }, size: None, rows, offsets: &[] })?;
    if covered {
        let play = &mut run.app.shared.config.play;
        (play.enable_cover, play.cover) = (true, PART_COVER);
        (play.enable_lift, play.lift) = (true, PART_LIFT);
        (play.enable_hidden, play.hidden) = (true, PART_HIDDEN);
    }
    let mut saved = Vec::new();
    run.draw_at(PLAYED_SHOT_MS)?;
    let mut keep = |run: &mut FieldRun, name: String| -> Result<(), String> {
        run.redraw()?;
        run.keep(&name);
        saved.push(name);
        Ok(())
    };
    keep(&mut run, format!("{prefix}-whole"))?;
    let parts = run.parts();
    for part in BARE_FIELD_PARTS.iter().filter(|part| parts.iter().any(|declared| declared == *part)) {
        run.set(part, opacity(NO_OPACITY))?;
    }
    keep(&mut run, format!("{prefix}-bare"))?;
    run.set_every_part(FULL_OPACITY)?;
    if each {
        for part in &parts {
            run.set(part, opacity(NO_OPACITY))?;
            keep(&mut run, format!("{prefix}-without-{}", file_name_of(part)))?;
            run.set(part, opacity(HALF_OPACITY))?;
            keep(&mut run, format!("{prefix}-half-{}", file_name_of(part)))?;
            run.set(part, opacity(FULL_OPACITY))?;
        }
        run.set_every_part(HALF_OPACITY)?;
        keep(&mut run, format!("{prefix}-half"))?;
    }
    Ok(saved)
}

/// Draw a run the engine plays of seven keys as it comes (`<prefix>-before`), then with the parts of
/// [`BARE_FIELD_PARTS`] taken off it with the keys of the skin menu (`<prefix>-after`).
fn capture_a_field_bared_from_the_skin_menu(prefix: &str) -> Result<Vec<String>, String> {
    let pack = default_pack();
    let played = Played::ByTheEngine { sides: &[SEVEN_KEYS_FIRST] };
    let mut run = FieldRun::enter(&FieldSetup { tag: prefix, pack: &pack, mode: rbms_model::Mode::BEAT_7K, played, size: None, rows: &[], offsets: &[] })?;
    run.draw_at(PLAYED_SHOT_MS)?;
    run.keep(&format!("{prefix}-before"));
    let screen = run.screen;
    let bare: Vec<String> = BARE_FIELD_PARTS.iter().map(|part| (*part).to_owned()).collect();
    take_off_with_the_skin_menu(&mut run.app, screen, &bare)?;
    run.redraw()?;
    run.keep(&format!("{prefix}-after"));
    Ok(vec![format!("{prefix}-before"), format!("{prefix}-after")])
}

/// Captures the play field of the default pack as a player who set it up sees it, for a person to
/// look at: without [`CAPTURE_DIR_ENV`] this passes without drawing anything. With it, every scene
/// has to draw, and the pack's folder has to be left as it was.
#[test]
fn the_play_field_of_the_default_pack_is_captured_with_its_judgement_and_its_parts_set() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let pack = default_pack();
    let before = files_under(&pack);
    let seven = rbms_model::Mode::BEAT_7K;
    let fourteen = rbms_model::Mode::BEAT_14K;
    let up = [(JUDGE_POSITION, raised(JUDGEMENT_MOVED_BY))];
    let down = [(JUDGE_POSITION, raised(-JUDGEMENT_MOVED_BY))];
    let captures: Vec<(&str, Result<Vec<String>, String>)> = vec![
        ("frames", capture_a_judgement_frame_by_frame(&pack, "parts-frames")),
        ("judgements", capture_the_judgements("parts-judge", seven, FIRST_SIDE_KEY, &[])),
        ("judgements in milliseconds", capture_the_judgements("parts-judge-ms", seven, FIRST_SIDE_KEY, &[MILLISECONDS, BEST_DIFFERENCE])),
        ("judgements of a wide field", capture_the_judgements("parts-judge-wide", seven, FIRST_SIDE_KEY, &[WIDE_LANES, FIELD_ON_THE_RIGHT])),
        ("judgements of five keys", capture_the_judgements("parts-judge-play5", rbms_model::Mode::BEAT_5K, FIRST_SIDE_KEY, &[])),
        ("judgements of the second side", capture_the_judgements("parts-judge-play14", fourteen, SECOND_SIDE_KEY, &[])),
        ("judgements of a wide second side", capture_the_judgements("parts-judge-play14-wide", fourteen, SECOND_SIDE_KEY, &[WIDE_LANES])),
        ("behind", capture_a_judgement_with("parts-line-behind", STROKES_BEHIND, &[], &[])),
        ("no difference", capture_a_judgement_with("parts-line-detail-only", 0, &[NO_DIFFERENCE], &[])),
        ("no detail", capture_a_judgement_with("parts-line-difference-only", 0, &[NO_DETAIL], &[])),
        ("moved up", capture_a_judgement_with("parts-position-up", 0, &[], &up)),
        ("moved down", capture_a_judgement_with("parts-position-down", 0, &[], &down)),
        ("hidden", capture_a_judgement_with("parts-position-hidden", 0, &[HIDDEN_JUDGEMENT], &[])),
        ("bared from the skin menu", capture_a_field_bared_from_the_skin_menu("parts-menu-bare")),
        ("parts of seven keys", capture_the_parts("parts-play7", seven, &[SEVEN_KEYS_FIRST], &[], false, true)),
        ("parts of a covered field", capture_the_parts("parts-play7-covers", seven, &[SEVEN_KEYS_FIRST], &[], true, false)),
        ("parts of wide lanes", capture_the_parts("parts-play7-wide", seven, &[SEVEN_KEYS_FIRST], &[WIDE_LANES], false, false)),
        ("parts of nine keys", capture_the_parts("parts-play9", rbms_model::Mode::POPN_9K, &[NINE_KEYS], &[], false, false)),
        ("parts of fourteen keys", capture_the_parts("parts-play14", fourteen, &[SEVEN_KEYS_FIRST, SEVEN_KEYS_SECOND], &[], false, false)),
        ("parts of wide fourteen keys", capture_the_parts("parts-play14-wide", fourteen, &[SEVEN_KEYS_FIRST, SEVEN_KEYS_SECOND], &[WIDE_LANES], false, false)),
    ];
    for (label, outcome) in captures {
        let saved = outcome.unwrap_or_else(|reason| panic!("{label}: not captured: {reason}"));
        println!("{label}: captured {}", saved.join(", "));
    }
    assert_eq!(files_under(&pack), before, "capturing the default pack's play field changed its folder");
}

/// Captures a judgement of a pack somebody else wrote frame by frame, from the frame it is made on
/// to the frame after it has gone.
///
/// Opt-in like the captures of the parent module: without [`SKIN_PACK_ENV`] this passes without
/// drawing anything. With it, the pack's folder has to be left as it was.
#[test]
fn a_judgement_of_a_skin_pack_named_by_the_environment_is_captured_frame_by_frame() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);
    for line in drawn_or_passed_over("pack-judgement", capture_a_judgement_frame_by_frame(&pack, "pack-judgement")).into_iter().flatten() {
        println!("{line}");
    }
    assert_eq!(files_under(&pack), before, "capturing the pack's judgement changed its folder");
}

//! The default pack's play scene drawn with colours of its colour rows set.
//!
//! A colour row the player has not set leaves its part drawn in the colours the pack's own rows
//! pick, with no tint, so a document read with nothing set is the one the pack drew before it had
//! colour rows. A row the player has set makes its part white in the document and names the row as
//! its tint, so the colour is multiplied in on every frame, and the notes of a lane whose row is set
//! are drawn from the pure white notes of the sheet.
//!
//! Five tests hold the pack to that on every machine: every play document declares its colour rows
//! under the same name and id as every other that has the row; a document read with no colour set
//! tints nothing, and one read with every colour set tints a white or grey part with each row; the
//! face of a black key whose colour is set is drawn in exactly that colour; a lane's notes and its
//! long notes are cut from the pure white notes exactly when a colour reaches them; and the long
//! notes of a black key follow its notes until their own row is set. One more draws the scenes for
//! a person to look at, only where [`CAPTURE_DIR_ENV`] names a folder.

use std::collections::BTreeMap;

use rbms_skin::model::{Destination, NO_TINT};

use super::*;

/// The colours the captures set by hand, by the name of their row: a red for the notes of the white
/// keys, a green for the long notes of the black keys, a see-through navy for the faces of the black
/// keys, and a cyan for the notes of the turntable.
const RED_WHITE_KEY_NOTES: (&str, SkinColor) = ("White Key Note", SkinColor::rgba(0xFF, 0x40, 0x40, 0xFF));
const GREEN_BLACK_KEY_LNS: (&str, SkinColor) = ("Black Key LN", SkinColor::rgba(0x30, 0xE0, 0x50, 0xFF));
const NAVY_BLACK_KEY_LANES: (&str, SkinColor) = ("Black Key Lane", SkinColor::rgba(0x20, 0x30, 0x60, 0xC0));
const CYAN_SCRATCH_NOTES: (&str, SkinColor) = ("Scratch Note", SkinColor::rgba(0x40, 0xE0, 0xFF, 0xFF));
const SET_BY_HAND: [(&str, SkinColor); 4] = [RED_WHITE_KEY_NOTES, GREEN_BLACK_KEY_LNS, NAVY_BLACK_KEY_LANES, CYAN_SCRATCH_NOTES];

/// A red for the notes of the black keys, which the long notes of the black keys follow until
/// their own row is set.
const RED_BLACK_KEY_NOTES: (&str, SkinColor) = ("Black Key Note", SkinColor::rgba(0xFF, 0x40, 0x40, 0xFF));

/// The lane of a seven-key field, in the order the document numbers its lanes, that the long notes
/// of the charts written for a mode are in: the fourth key, a black one.
const LONG_NOTE_LANE: usize = 3;

/// How many times one channel of a pixel has to be over each of the other two for the pixel to be
/// counted as of that channel's hue.
const HUE_MARGIN: u16 = 2;

/// The colours every row of a document is set to in turn when all of them are set: none of them a
/// colour the pack comes with, so a part left in its own colour shows.
const EVERY_ROW_COLOURS: [SkinColor; 6] = [
    SkinColor::rgba(0xFF, 0x60, 0x30, 0xFF),
    SkinColor::rgba(0x30, 0xA0, 0xFF, 0xFF),
    SkinColor::rgba(0x90, 0xF0, 0x40, 0xFF),
    SkinColor::rgba(0xD0, 0x50, 0xF0, 0xFF),
    SkinColor::rgba(0x30, 0x40, 0x50, 0xFF),
    SkinColor::rgba(0xFF, 0xD0, 0x40, 0xC0),
];

/// The row of the faces of the black keys, the colour the pixel test sets it to, and the lane of a
/// seven-key field numbered from the left that is a black key when the turntable is on the left.
const BLACK_KEY_LANE_ROW: &str = "Black Key Lane";
const OPAQUE_NAVY: SkinColor = SkinColor::rgba(0x20, 0x30, 0x60, 0xFF);
const SECOND_KEY_LANE: usize = 1;

/// How far above the foot of the lanes the face is read, in the pixels the document is authored in:
/// over the pulse of the judgement line and under where any note is while the chart loads.
const FACE_SAMPLE_RISE: i32 = 40;

/// The colour rows a document declares, each name with its id.
fn declared_colours(run: &RowsRun) -> BTreeMap<String, i32> {
    let header = run.app.shared.skins.header_of(&run.app.shared.config, run.screen).expect("a compiled screen has its header");
    header.colors.iter().map(|row| (row.name.clone(), row.id)).collect()
}

/// Every colour row of a document set to the colours of [`EVERY_ROW_COLOURS`] in turn.
fn every_row_set(run: &RowsRun) -> Vec<(String, SkinColor)> {
    declared_colours(run).into_keys().zip(EVERY_ROW_COLOURS.iter().cycle().copied()).collect()
}

/// Every destination of a document the colour rows can reach: the ones it places and the lines of
/// the bars of its note field.
fn destinations(run: &RowsRun) -> Vec<&Destination> {
    let def = &run.document().def;
    def.destination.iter().chain(def.note.iter().flat_map(|note| note.group.iter())).collect()
}

/// The colour ids the notes and the long notes of each lane are tinted by, without the lanes that
/// are not tinted.
fn note_tints(run: &RowsRun) -> Vec<i32> {
    run.document().def.note.iter().flat_map(|note| note.tint.iter().chain(&note.long_tint).copied()).filter(|id| *id != NO_TINT).collect()
}

/// A run of `mode` entered for the held tests, with `colours` set.
fn held_run(tag: &str, mode: &RowMode, colours: &[(&str, SkinColor)]) -> RowsRun {
    RowsRun::enter_coloured(tag, mode, &[], colours, Some(HELD_ROWS_SIZE), PlayEnding::StillPlaying, false)
        .unwrap_or_else(|reason| panic!("{tag}: not entered: {reason}"))
}

/// Every play document declares colour rows, no two of them under one name or one id and each with
/// a default that is a colour; and a row of one name has the same id in every document that has it,
/// so a colour set on one screen and applied to the others lands on the same part.
#[test]
fn every_play_document_of_the_default_pack_declares_its_colour_rows_under_the_ids_the_others_do() {
    let mut ids_by_name: BTreeMap<String, (i32, &str)> = BTreeMap::new();
    for mode in ROW_MODES {
        let run = held_run(&format!("colour-rows-{}", mode.name), mode, &[]);
        run.assert_clean(mode.name);
        let header = run.app.shared.skins.header_of(&run.app.shared.config, run.screen).expect("a compiled screen has its header");
        assert!(!header.colors.is_empty(), "{}: the document declares no colour row", mode.name);
        let mut ids = std::collections::BTreeSet::new();
        for row in &header.colors {
            assert!(row.def.as_deref().and_then(SkinColor::from_hex).is_some(), "{}: the row {:?} has no colour for its default", mode.name, row.name);
            assert!(row.id >= 0 && ids.insert(row.id), "{}: the row {:?} has an id {} that cannot be told apart", mode.name, row.name, row.id);
            let (id, first) = *ids_by_name.entry(row.name.clone()).or_insert((row.id, mode.name));
            assert_eq!(row.id, id, "{}: the row {:?} has the id {} where {first} gives it {id}", mode.name, row.name, row.id);
        }
        assert_eq!(declared_colours(&run).len(), header.colors.len(), "{}: two colour rows share a name", mode.name);
    }
}

/// A document read with no colour set tints nothing. One read with every colour set compiles with
/// nothing said against it, names every row as the tint of a part or of the notes of a lane, draws
/// every tinted part from a grey so that the colour comes out as it was set, and plays and draws.
#[test]
fn the_default_pack_tints_a_part_only_when_its_colour_is_set() {
    for mode in ROW_MODES {
        let label = mode.name;
        let plain = held_run(&format!("colour-plain-{label}"), mode, &[]);
        plain.assert_clean(label);
        assert!(destinations(&plain).iter().all(|destination| destination.tint.is_none()), "{label}: a part is tinted with no colour set");
        assert!(note_tints(&plain).is_empty(), "{label}: the notes of a lane are tinted with no colour set");

        let set = every_row_set(&plain);
        let colours: Vec<(&str, SkinColor)> = set.iter().map(|(name, colour)| (name.as_str(), *colour)).collect();
        let mut run = held_run(&format!("colour-every-{label}"), mode, &colours);
        run.assert_clean(label);
        let parts = destinations(&run);
        let mut named: std::collections::BTreeSet<i32> = note_tints(&run).into_iter().collect();
        for destination in parts.iter().filter(|destination| destination.tint.is_some()) {
            named.extend(destination.tint);
            for frame in &destination.dst {
                let (r, g, b) = (frame.r.unwrap_or(i32::from(u8::MAX)), frame.g.unwrap_or(i32::from(u8::MAX)), frame.b.unwrap_or(i32::from(u8::MAX)));
                assert!(r == g && g == b, "{label}: the part {:?} tinted by {:?} is drawn from {r},{g},{b}, not a grey", destination.id, destination.tint);
            }
        }
        for (name, id) in declared_colours(&run) {
            assert!(named.contains(&id), "{label}: nothing is tinted by the row {name:?} ({id})");
        }

        run.open(|_, _| {}).unwrap_or_else(|reason| panic!("{label}: not opened: {reason}"));
        run.play_to(HELD_ROWS_CHART_MS).unwrap_or_else(|reason| panic!("{label}: not played: {reason}"));
        run.assert_clean(label);
        assert!(!is_flat(&run.pixels, 0..HELD_ROWS_SIZE.0, 0..HELD_ROWS_SIZE.1), "{label}: the frame is one colour");
    }
}

/// The face of a black key of a seven-key field, read while the chart loads, is the ground of the
/// theme with no colour set and exactly the colour of its row once that is set.
#[test]
fn the_face_of_a_black_key_is_drawn_in_the_colour_its_row_is_set_to() {
    let face = |tag: &str, colours: &[(&str, SkinColor)]| {
        let mut run = held_run(tag, &SEVEN, colours);
        let lane = run.lanes()[SECOND_KEY_LANE];
        let (width, height) = run.authored();
        let column = ((lane.x * 2 + lane.w) as f32 / 2.0 * HELD_ROWS_SIZE.0 as f32 / width as f32) as u32;
        let row = ((height - lane.y - FACE_SAMPLE_RISE) as f32 * HELD_ROWS_SIZE.1 as f32 / height as f32) as u32;
        let mut read = None;
        run.open(|moment, pixels| {
            if read.is_none() {
                read = Some((moment.to_owned(), pixels.pixel_at(column, row)));
            }
        })
        .unwrap_or_else(|reason| panic!("{tag}: not opened: {reason}"));
        run.assert_clean(tag);
        read.unwrap_or_else(|| panic!("{tag}: no moment of the opening was drawn"))
    };
    let (moment, plain) = face("colour-face-plain", &[]);
    assert_eq!(plain, MONO_GROUND, "the face of a black key with no colour set at {moment} is not the ground of the theme");
    let (moment, navy) = face("colour-face-navy", &[(BLACK_KEY_LANE_ROW, OPAQUE_NAVY)]);
    assert_eq!(navy, crate::Color::rgb(OPAQUE_NAVY.r, OPAQUE_NAVY.g, OPAQUE_NAVY.b), "the face of a black key at {moment} is not the colour its row is set to");
}

/// The pixels of one column that are mostly red and mostly green, in that order.
fn reds_and_greens(pixels: &HeadlessCanvas, column: u32) -> (usize, usize) {
    let leads = |lead: u8, others: [u8; 2]| others.iter().all(|other| u16::from(lead) > u16::from(*other) * HUE_MARGIN);
    let column: Vec<crate::Color> = (0..HELD_ROWS_SIZE.1).map(|row| pixels.pixel_at(column, row)).collect();
    let reds = column.iter().filter(|pixel| leads(pixel.r, [pixel.g, pixel.b])).count();
    let greens = column.iter().filter(|pixel| leads(pixel.g, [pixel.r, pixel.b])).count();
    (reds, greens)
}

/// The long notes of the black keys have a colour row of their own. Until it is set they are drawn
/// in the colour the notes of the black keys are set to; once it is set they are drawn in it. The
/// lane is read while its long note is held, when the chart has no plain note in it, so every
/// coloured pixel of it is the long note's.
#[test]
fn the_long_notes_of_a_black_key_follow_its_notes_until_their_own_colour_is_set() {
    let column_of = |tag: &str, colours: &[(&str, SkinColor)]| {
        let mut run = held_run(tag, &SEVEN, colours);
        run.open(|_, _| {}).unwrap_or_else(|reason| panic!("{tag}: not opened: {reason}"));
        run.play_to(ROWS_HELD_SHOT_MS).unwrap_or_else(|reason| panic!("{tag}: not played: {reason}"));
        run.assert_clean(tag);
        let lane = run.lanes()[LONG_NOTE_LANE];
        let (width, _) = run.authored();
        let column = ((lane.x * 2 + lane.w) as f32 / 2.0 * HELD_ROWS_SIZE.0 as f32 / width as f32) as u32;
        reds_and_greens(&run.pixels, column)
    };
    let (reds, greens) = column_of("colour-long-follows", &[RED_BLACK_KEY_NOTES]);
    assert!(reds > 0 && greens == 0, "the notes alone set: {reds} red and {greens} green pixels in the lane of the long note");
    let (reds_apart, greens_apart) = column_of("colour-long-apart", &[RED_BLACK_KEY_NOTES, GREEN_BLACK_KEY_LNS]);
    assert!(greens_apart > 0 && reds_apart == 0, "both set: {reds_apart} red and {greens_apart} green pixels in the lane of the long note");
}

/// The end of the name of an image cut from the pure white notes of the sheet, which a colour is
/// multiplied into.
const PURE_IMAGE_SUFFIX: &str = "_pure";

/// Which of the colour rows of the black keys' notes and long notes each case sets.
const NOTE_AND_LONG_CASES: [(&str, &[(&str, SkinColor)]); 4] =
    [("none", &[]), ("notes", &[RED_BLACK_KEY_NOTES]), ("longs", &[GREEN_BLACK_KEY_LNS]), ("both", &[RED_BLACK_KEY_NOTES, GREEN_BLACK_KEY_LNS])];

/// A lane's notes, and apart from them its long notes, are cut from the pure white notes exactly
/// when a colour is multiplied into them: setting the long notes' row alone leaves the notes in
/// their own colours, not white with nothing to colour them.
#[test]
fn a_lanes_notes_and_long_notes_are_cut_from_the_pure_sheet_exactly_when_a_colour_reaches_them() {
    for (tag, colours) in NOTE_AND_LONG_CASES {
        let run = held_run(&format!("colour-pure-{tag}"), &SEVEN, colours);
        run.assert_clean(tag);
        let notes = run.document().def.note.as_ref().unwrap_or_else(|| panic!("{tag}: the document has no notes"));
        for lane in 0..notes.note.len() {
            let pure = |images: &[String]| images.get(lane).is_some_and(|id| id.ends_with(PURE_IMAGE_SUFFIX));
            assert_eq!(pure(&notes.note), notes.tint_of(lane).is_some(), "{tag}: the notes of lane {lane}");
            assert_eq!(pure(&notes.lnstart), notes.long_tint_of(lane).is_some(), "{tag}: the long notes of lane {lane}");
        }
        let longs_tinted = (0..notes.note.len()).any(|lane| notes.long_tint_of(lane).is_some());
        assert_eq!(longs_tinted, !colours.is_empty(), "{tag}: whether any long note is coloured");
    }
}

/// One scene captured with colours set: the mode, the name it is saved under, and whether every row
/// is set or only the ones [`SET_BY_HAND`] names.
struct ColourShot {
    mode: &'static RowMode,
    name: &'static str,
    every_row: bool,
}

const COLOUR_SHOTS: [ColourShot; 5] = [
    ColourShot { mode: &SEVEN, name: "by-hand", every_row: false },
    ColourShot { mode: &SEVEN, name: "every-row", every_row: true },
    ColourShot { mode: &FIVE, name: "by-hand", every_row: false },
    ColourShot { mode: &NINE, name: "every-row", every_row: true },
    ColourShot { mode: &FOURTEEN, name: "every-row", every_row: true },
];

/// Draw one scene with its colours set at the size its document is authored at, from its loading to
/// its full combo, and save its frames as `colours-<mode>-<name>-<moment>`. Answers the names saved.
fn capture_colours(shot: &ColourShot) -> Result<Vec<String>, String> {
    let prefix = format!("colours-{}-{}", shot.mode.name, shot.name);
    let colours: Vec<(String, SkinColor)> = if shot.every_row {
        let plain = RowsRun::enter(&format!("{prefix}-rows"), shot.mode, &[], Some(HELD_ROWS_SIZE), PlayEnding::StillPlaying, false)?;
        every_row_set(&plain)
    } else {
        SET_BY_HAND.iter().map(|(name, colour)| ((*name).to_owned(), *colour)).collect()
    };
    let named: Vec<(&str, SkinColor)> = colours.iter().map(|(name, colour)| (name.as_str(), *colour)).collect();
    let mut run = RowsRun::enter_coloured(&prefix, shot.mode, &[], &named, None, PlayEnding::Fade, false)?;
    let size = run.size;
    let mut saved = Vec::new();
    let mut keep = |moment: &str, pixels: &HeadlessCanvas| {
        let name = format!("{prefix}-{moment}");
        save(&name, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(name);
    };
    run.open(&mut keep)?;
    run.play_to(ROWS_HELD_SHOT_MS)?;
    keep("held", &run.pixels);
    run.play_to(ROWS_SHOT_MS)?;
    keep("play", &run.pixels);
    run.assert_clean(&prefix);
    Ok(saved)
}

/// Captures the default pack's play scene with colours of its colour rows set, for a person to look
/// at: without [`CAPTURE_DIR_ENV`] this passes without drawing anything. With it, every scene has to
/// draw, and the pack's folder has to be left as it was.
#[test]
fn the_play_scene_of_the_default_pack_is_captured_with_its_colours_set() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let pack = default_pack();
    let before = files_under(&pack);
    for shot in &COLOUR_SHOTS {
        let saved = capture_colours(shot).unwrap_or_else(|reason| panic!("{} {}: not captured: {reason}", shot.mode.name, shot.name));
        println!("{} {}: captured {}", shot.mode.name, shot.name, saved.join(", "));
    }
    assert_eq!(files_under(&pack), before, "capturing the default pack's play scene changed its folder");
}

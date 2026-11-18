//! The default pack's judgement measured on frames of several sizes and shapes, in the document's own
//! pixels: the word, each figure of the combo, the difference over it, the timing word beside that,
//! and the judgement line under all of them.
//!
//! An image part is placed in the document's space and scaled onto the frame as it is, so it lands on
//! the same document pixels on every frame: a 720p frame, a 4K one, and a 16:10 one with the letterbox
//! on or off. That is held to within half a document pixel, measured by the middle of the part's ink
//! so the frame's own pixel grid does not count, or within half a pixel of a frame whose pixel is
//! larger than that: a frame covers a pixel that its centre falls in, so a part that starts between
//! two pixels lands on one of them, on the GPU as on the CPU. A text part is set on whole pixels of the frame
//! (`docs/acknowledge/2026-10-22-adr-ui-text-device-pixels.md`), so the timing word is held to the
//! corner of its ink to within a pixel of each of the two frames compared.
//!
//! The lift moves the judgement with the line it stands on, so a lifted run is held the same way and
//! its judgement to the same distance from that line.
//!
//! The pack's JUDGE POSITION is a nudge the player stores, and a nudge is read in the document's own
//! pixels: a judgement raised by [`RAISED_BY`] stands that many document pixels over an unraised one
//! on every frame, held the same way, while the judgement line under it stays where it was.
//!
//! Each frame's measurements are printed as a row of a table, which is what a run with
//! `--nocapture` shows.

use super::*;

/// The frames the judgement is measured on: two sizes of the 16:9 the pack is authored at either
/// side of the size it is authored at, that size, and a 16:10 window drawn letterboxed (its 16:9
/// middle) and stretched (all of it). The largest RESOLUTION is left out: it measured the same as
/// the others and is most of what the test would take to draw.
const SIZES: [(u32, u32); 5] = [(1280, 720), (1920, 1080), (2560, 1440), (1728, 972), (1728, 1117)];

/// The frames a lifted run is measured on.
const LIFTED_SIZES: [(u32, u32); 3] = [(1280, 720), (1920, 1080), (2560, 1440)];

/// How far a raised run's JUDGE POSITION lifts its judgement, in document pixels.
const RAISED_BY: f32 = 30.0;

/// The frame the others are compared with: the pack's own size.
const REFERENCE_SIZE: (u32, u32) = (1920, 1080);

/// How far the middle of an image part may stand from where the reference frame has it, in document
/// pixels.
const IMAGE_SLACK: f64 = 0.5;

/// The combo the run begins with, so the judgement it is measured on is drawn in front of
/// [`FIGURES`] figures.
const CARRIED_COMBO: u32 = 647;
const FIGURES: i32 = 3;

/// The shortest run of empty columns that tells the parts of the line over the judgement apart, in
/// document pixels: under the 16 the pack leaves between the difference and the timing word, and
/// over what a figure or a letter leaves empty inside itself.
const PART_GAP: f64 = 8.0;

/// The shortest run of empty rows that tells one band of the field from the next, in pixels of the
/// frame.
const BAND_GAP: u32 = 1;

/// The part [`AROUND_THE_JUDGEMENT`] clears that is kept here, so the judgement is measured against
/// the line it stands on.
const JUDGE_LINE_PART: &str = "Judge Line Opacity";

/// The middle of a pixel, from its corner.
const PIXEL_MIDDLE: f64 = 0.5;

/// The ink of one part, in document pixels: the top left corner of the pixels it touches, and the
/// middle of its ink weighed by how far each pixel stands from the ground.
#[derive(Clone, Copy, Debug)]
struct Ink {
    left: f64,
    top: f64,
    x: f64,
    y: f64,
}

/// The parts of one judgement as one frame draws them.
#[derive(Clone, Debug)]
struct Judgement {
    word: Ink,
    figures: Vec<Ink>,
    difference: Ink,
    timing: Ink,
    line: Ink,
}

/// A frame read in the document's pixels.
struct Measured<'a> {
    run: &'a FieldRun,
    ground: crate::Color,
    /// Pixels of the frame to a document pixel, across and down.
    scale: (f64, f64),
}

impl Measured<'_> {
    /// How far a pixel of the frame stands from the ground, from nothing to one.
    fn weight(&self, x: u32, y: u32) -> f64 {
        let pixel = self.run.pixels.pixel_at(x, y);
        let apart = |one: u8, other: u8| f64::from(one.abs_diff(other));
        apart(pixel.r, self.ground.r).max(apart(pixel.g, self.ground.g)).max(apart(pixel.b, self.ground.b)) / f64::from(u8::MAX)
    }

    /// The ink of `columns` and `rows`, or `None` when they are the ground.
    fn ink(&self, columns: &Range<u32>, rows: &Range<u32>) -> Option<Ink> {
        let (mut mass, mut x, mut y) = (0.0, 0.0, 0.0);
        let (mut left, mut top) = (u32::MAX, u32::MAX);
        for row in rows.clone() {
            for column in columns.clone() {
                let weight = self.weight(column, row);
                if weight > 0.0 {
                    (left, top) = (left.min(column), top.min(row));
                    mass += weight;
                    x += weight * (f64::from(column) + PIXEL_MIDDLE);
                    y += weight * (f64::from(row) + PIXEL_MIDDLE);
                }
            }
        }
        (mass > 0.0).then(|| {
            let (across, down) = self.scale;
            Ink { left: f64::from(left) / across, top: f64::from(top) / down, x: x / mass / across, y: y / mass / down }
        })
    }

    /// The runs of `lines` whose ink is not the ground, apart from each other by `gap` empty ones.
    fn runs(lines: Range<u32>, gap: u32, inked: impl Fn(u32) -> bool) -> Vec<Range<u32>> {
        let mut found: Vec<Range<u32>> = Vec::new();
        for line in lines.filter(|line| inked(*line)) {
            match found.last_mut() {
                Some(last) if line - last.end < gap => last.end = line + 1,
                _ => found.push(line..line + 1),
            }
        }
        found
    }

    /// Where the frame drew the judgement of the run's first field.
    fn judgement(run: &FieldRun) -> Result<Judgement, String> {
        let skin = run.document();
        let scale = (f64::from(run.size.0) / f64::from(skin.def.w.max(1)), f64::from(run.size.1) / f64::from(skin.def.h.max(1)));
        let (field, _) = run.field();
        let frame = Measured { run, ground: run.pixels.pixel_at(field.start, 0), scale };
        let line = run.judge_line(0);
        let word_columns = run.columns(line.word_left, line.word_right);
        let row_inked = |row: u32| field.clone().any(|column| frame.weight(column, row) > 0.0);
        let bands = Measured::runs(0..run.size.1, BAND_GAP, row_inked);
        let over_word = |band: &Range<u32>| band.clone().any(|row| word_columns.clone().any(|column| frame.weight(column, row) > 0.0));
        let at = bands.iter().position(over_word).ok_or("nothing is drawn over the word's columns")?;
        let (Some(over), Some(band), Some(under)) = (bands.get(at), bands.get(at + 1), bands.get(at + 2)) else {
            return Err(format!("the judgement is not drawn between a line over it and the judgement line: bands {bands:?}"));
        };
        let missing = |part: &str| format!("no {part} is drawn");
        let word = frame.ink(&word_columns, band).ok_or_else(|| missing("word"))?;
        let figures = (0..FIGURES)
            .map(|place| frame.ink(&run.columns(line.place(place), line.place(place + 1)), band).ok_or_else(|| missing(&format!("figure {place}"))))
            .collect::<Result<Vec<Ink>, String>>()?;
        let gap = (PART_GAP * scale.0).ceil() as u32;
        let parts = Measured::runs(field.clone(), gap, |column| over.clone().any(|row| frame.weight(column, row) > 0.0));
        let [difference, timing] = parts.as_slice() else {
            return Err(format!("the line over the judgement is not the difference and the timing word: columns {parts:?}"));
        };
        Ok(Judgement {
            word,
            figures,
            difference: frame.ink(difference, over).ok_or_else(|| missing("difference"))?,
            timing: frame.ink(timing, over).ok_or_else(|| missing("timing word"))?,
            line: frame.ink(&field, under).ok_or_else(|| missing("judgement line"))?,
        })
    }
}

/// How a measured run stands its judgement.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stance {
    /// As the pack has it.
    Plain,
    /// Lifted, with the lane cover down.
    Lifted,
    /// Raised by [`RAISED_BY`] on the pack's JUDGE POSITION.
    Raised,
}

impl Stance {
    /// What the run's tag calls it.
    fn name(self) -> &'static str {
        match self {
            Stance::Plain => "plain",
            Stance::Lifted => "lifted",
            Stance::Raised => "raised",
        }
    }
}

/// A run by hand on the default pack drawn at `size`, played early by a GREAT so the timing word
/// shows, with nothing over its lanes but the judgement and the line it stands on, and stood as
/// `stance` says. Answers where the judgement was drawn.
///
/// `test` names the test the run is drawn for, so two tests that draw the same run at once keep to
/// folders of their own.
fn judged_at(test: &str, size: (u32, u32), stance: Stance) -> Judgement {
    let tag = format!("judge-sizes-{test}-{}-{}x{}", stance.name(), size.0, size.1);
    let cleared = AROUND_THE_JUDGEMENT.iter().filter(|part| **part != JUDGE_LINE_PART).map(|part| (*part, opacity(NO_OPACITY)));
    let position = (stance == Stance::Raised).then(|| (JUDGE_POSITION, raised(RAISED_BY)));
    let offsets: Vec<(&str, SkinOffset)> = cleared.chain(position).collect();
    let pack = default_pack();
    let played = Played::ByHand { channel: FIRST_SIDE_KEY, strokes: 1 };
    let setup = FieldSetup {
        tag: &tag,
        pack: &pack,
        mode: rbms_model::Mode::BEAT_7K,
        played,
        size: Some(size),
        rows: &[NO_NOTES_PER_SECOND],
        offsets: &offsets,
        carried: CARRIED_COMBO,
    };
    let mut run = FieldRun::enter(&setup).unwrap_or_else(|reason| panic!("{tag}: not entered: {reason}"));
    if stance == Stance::Lifted {
        let play = &mut run.app.shared.config.play;
        (play.enable_lift, play.lift, play.enable_cover, play.cover) = (true, PART_LIFT, true, PART_COVER);
    }
    let down_ms = run.strike(0, GREAT_OFF_MS).unwrap_or_else(|reason| panic!("{tag}: not played: {reason}"));
    run.draw_at(down_ms + JUDGEMENT_LOOKED_AT_MS).unwrap_or_else(|reason| panic!("{tag}: not drawn: {reason}"));
    run.keep(&tag);
    let judgement = Measured::judgement(&run).unwrap_or_else(|reason| panic!("{tag}: {reason}"));
    let figures: Vec<String> = judgement.figures.iter().map(|figure| format!("({:.2}, {:.2})", figure.x, figure.y)).collect();
    println!(
        "{tag}: word ({:.2}, {:.2}) figures {} difference ({:.2}, {:.2}) timing corner ({:.2}, {:.2}) middle ({:.2}, {:.2}) line {:.2}, word over line {:.2}",
        judgement.word.x,
        judgement.word.y,
        figures.join(" "),
        judgement.difference.x,
        judgement.difference.y,
        judgement.timing.left,
        judgement.timing.top,
        judgement.timing.x,
        judgement.timing.y,
        judgement.line.y,
        judgement.line.y - judgement.word.y,
    );
    judgement
}

/// The pixels of a frame of `size` to a document pixel, across and down, for the default pack.
fn pixels_a_unit(size: (u32, u32)) -> (f64, f64) {
    (f64::from(size.0) / f64::from(REFERENCE_SIZE.0), f64::from(size.1) / f64::from(REFERENCE_SIZE.1))
}

/// `judgement` as it stands raised by `by` document pixels: everything over the judgement line goes
/// up, and the line stays.
fn raised_by(judgement: &Judgement, by: f64) -> Judgement {
    let up = |ink: &Ink| Ink { top: ink.top - by, y: ink.y - by, ..*ink };
    Judgement {
        word: up(&judgement.word),
        figures: judgement.figures.iter().map(up).collect(),
        difference: up(&judgement.difference),
        timing: up(&judgement.timing),
        line: judgement.line,
    }
}

/// Holds the judgement drawn on a frame of `size` to the one drawn on the reference frame.
fn assert_alike(label: &str, size: (u32, u32), drawn: &Judgement, reference: &Judgement) {
    let (scale, reference_scale) = (pixels_a_unit(size), pixels_a_unit(REFERENCE_SIZE));
    let image_slack = (IMAGE_SLACK.max(PIXEL_MIDDLE / scale.0), IMAGE_SLACK.max(PIXEL_MIDDLE / scale.1));
    let near = |part: &str, one: &Ink, other: &Ink| {
        let (dx, dy) = ((one.x - other.x).abs(), (one.y - other.y).abs());
        assert!(
            dx <= image_slack.0 && dy <= image_slack.1,
            "{label} {size:?}: the {part} is {dx:.2}, {dy:.2} document pixels from where {REFERENCE_SIZE:?} has it"
        );
    };
    near("word", &drawn.word, &reference.word);
    for (place, (one, other)) in drawn.figures.iter().zip(&reference.figures).enumerate() {
        near(&format!("figure {place}"), one, other);
    }
    near("difference", &drawn.difference, &reference.difference);
    let below = (drawn.line.y - drawn.word.y) - (reference.line.y - reference.word.y);
    assert!(below.abs() <= image_slack.1, "{label} {size:?}: the word stands {below:.2} document pixels further from its line than on {REFERENCE_SIZE:?}");

    let slack = (1.0 / scale.0 + 1.0 / reference_scale.0, 1.0 / scale.1 + 1.0 / reference_scale.1);
    let (dx, dy) = ((drawn.timing.left - reference.timing.left).abs(), (drawn.timing.top - reference.timing.top).abs());
    assert!(
        dx <= slack.0 && dy <= slack.1,
        "{label} {size:?}: the timing word starts {dx:.2}, {dy:.2} document pixels from where {REFERENCE_SIZE:?} starts it"
    );
}

/// The judgement lands on the same document pixels on every frame size and shape.
#[test]
fn the_default_pack_draws_its_judgement_on_the_same_document_pixels_at_every_size() {
    let reference = judged_at("same", REFERENCE_SIZE, Stance::Plain);
    for size in SIZES.into_iter().filter(|size| *size != REFERENCE_SIZE) {
        assert_alike("plain", size, &judged_at("same", size, Stance::Plain), &reference);
    }
}

/// A judgement raised on the pack's JUDGE POSITION stands [`RAISED_BY`] document pixels over the
/// unraised one on every frame size and shape, the reference frame among them: a nudge is read in the
/// document's own pixels, not in the frame's. Every size is drawn before any is held, so a run with
/// `--nocapture` prints the whole table.
#[test]
fn the_default_pack_raises_its_judgement_by_the_same_document_pixels_at_every_size() {
    let reference = raised_by(&judged_at("raise", REFERENCE_SIZE, Stance::Plain), f64::from(RAISED_BY));
    let drawn: Vec<((u32, u32), Judgement)> = SIZES.into_iter().map(|size| (size, judged_at("raise", size, Stance::Raised))).collect();
    for (size, judgement) in &drawn {
        assert_alike("raised", *size, judgement, &reference);
    }
}

/// A lifted judgement lands on the same document pixels on every frame size, and stands as far from
/// the lifted line as an unlifted one does from its own.
#[test]
fn the_default_pack_lifts_its_judgement_alike_at_every_size() {
    let reference = judged_at("lift", REFERENCE_SIZE, Stance::Lifted);
    let unlifted = judged_at("lift", REFERENCE_SIZE, Stance::Plain);
    let below = (reference.line.y - reference.word.y) - (unlifted.line.y - unlifted.word.y);
    assert!(below.abs() <= IMAGE_SLACK, "the lifted word stands {below:.2} document pixels further from its line than an unlifted one");
    assert!(reference.word.y < unlifted.word.y, "the lift does not raise the judgement");
    for size in LIFTED_SIZES.into_iter().filter(|size| *size != REFERENCE_SIZE) {
        assert_alike("lifted", size, &judged_at("lift", size, Stance::Lifted), &reference);
    }
}

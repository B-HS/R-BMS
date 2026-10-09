//! Tests of the skin text block composer and of the font table it draws from: the tight line width
//! alignment measures, the capital line a block hangs from, whole-pixel advances at the generated
//! size, the three overflow rules, wrapping, the fallback for a character a font lacks, and how a
//! loaded font is named.

use super::block::{BLOCK_MAX_DIM, BlockAlign, BlockFit, BlockSpec, TextBlock, small_letter_fit};
use super::{BUNDLED_FONT, Face, TextContext};
use crate::BYTES_PER_PIXEL;

/// The em size most of these blocks are composed at.
const EM: f32 = 40.0;

/// A width no test line reaches.
const ROOMY: f32 = 4000.0;

/// A width the long test line does not fit in.
const NARROW: f32 = 120.0;

/// How far a measured edge may sit from the computed one: glyphs land on whole pixels.
const PIXEL: f32 = 1.0;

/// A line a good deal longer than [`NARROW`] at [`EM`].
const LONG_LINE: &str = "OVERFLOWING LINE OF TEXT";

/// An engine holding only the bundled face, loaded once more the way a skin loads its own font.
fn engine() -> (TextContext, String) {
    let mut text = TextContext::embedded_only();
    let family = text.load_font(BUNDLED_FONT.to_vec()).expect("the bundled font loads as a skin font");
    (text, family)
}

/// A left-aligned, unfitted block of `text` at [`EM`] in a roomy width.
fn spec<'a>(text: &'a str, family: &'a str) -> BlockSpec<'a> {
    BlockSpec { text, family, em_px: EM, design_px: EM, width: ROOMY, align: BlockAlign::Left, fit: BlockFit::Overflow, max_dim: BLOCK_MAX_DIM }
}

/// The columns of a block that hold any ink at all.
fn inked_columns(block: &TextBlock) -> Vec<u32> {
    let lit = |x: u32| (0..block.height).any(|y| block.rgba[((y * block.width + x) as usize) * BYTES_PER_PIXEL + 3] != 0);
    (0..block.width).filter(|x| lit(*x)).collect()
}

/// The first column of every separate run of inked columns.
fn run_starts(block: &TextBlock) -> Vec<u32> {
    let columns = inked_columns(block);
    columns.iter().enumerate().filter(|(index, column)| *index == 0 || columns[index - 1] + 1 != **column).map(|(_, column)| *column).collect()
}

#[test]
fn a_line_starts_at_the_ink_of_its_first_glyph_and_ends_at_the_ink_of_its_last() {
    let (mut text, family) = engine();
    let block = text.compose_block(&spec("Hi", &family)).expect("two letters compose");

    assert_eq!(block.left, 0.0, "the first glyph's bearing is taken off, so its ink sits on the anchor");
    assert_eq!(block.drawn_width, block.width as f32, "an unfitted block is drawn at the size it was composed at");
    assert!((block.drawn_width - block.layout_width).abs() <= PIXEL, "the measured width is the ink's, not the advances': {block:?}");
    assert_eq!(block.lines, 1);
}

#[test]
fn alignment_pushes_a_line_against_the_width_it_was_given() {
    let (mut text, family) = engine();
    let width = 300.0;
    let aligned = |text: &mut TextContext, align| text.compose_block(&BlockSpec { width, align, ..spec("ALIGN", &family) }).expect("the word composes");

    let left = aligned(&mut text, BlockAlign::Left);
    let centre = aligned(&mut text, BlockAlign::Center);
    let right = aligned(&mut text, BlockAlign::Right);

    assert_eq!(left.left, 0.0);
    assert!((centre.left - (width - centre.layout_width) / 2.0).abs() <= PIXEL, "a centred line has the same room on both sides: {}", centre.left);
    assert!((right.left + right.drawn_width - width).abs() <= PIXEL, "a right-aligned line ends where the width does: {}", right.left);
    assert_eq!(left.rgba, right.rgba, "alignment moves a line and changes nothing in it");
}

#[test]
fn a_block_hangs_from_the_capital_line() {
    let (mut text, family) = engine();
    let capital = text.compose_block(&spec("M", &family)).expect("a capital composes");
    let descending = text.compose_block(&spec("Mg", &family)).expect("a descender composes");

    assert!(capital.top.abs() <= PIXEL, "the top of a capital is the line the block is hung from, to the pixel its edge is smoothed over: {}", capital.top);
    assert!((capital.height as f32) < EM, "a capital is shorter than the em it is set in: {}", capital.height);
    assert_eq!(descending.top, capital.top, "a descender does not move the capital line");
    assert!(descending.height > capital.height, "it only reaches further below it");
}

#[test]
fn the_destination_height_is_the_em_whatever_size_the_font_is_generated_at() {
    let (mut text, family) = engine();
    let native = text.compose_block(&spec("M", &family)).expect("at its generated size");
    let scaled = text.compose_block(&BlockSpec { design_px: EM / 2.0, ..spec("M", &family) }).expect("generated at half the size it is drawn at");
    let doubled = text.compose_block(&BlockSpec { em_px: EM * 2.0, ..spec("M", &family) }).expect("drawn at twice the size it is generated at");

    assert!((scaled.height as f32 - native.height as f32).abs() <= PIXEL, "the glyph is as tall either way: {} against {}", scaled.height, native.height);
    assert!(scaled.top.abs() <= PIXEL, "and hangs from the same line: {}", scaled.top);
    assert!((doubled.height as f32 - 2.0 * native.height as f32).abs() <= 2.0 * PIXEL, "twice the em is twice the glyph: {}", doubled.height);
}

#[test]
fn advances_are_whole_pixels_at_the_generated_size() {
    let (mut text, family) = engine();
    for scale in [4u32, 5] {
        let block = text.compose_block(&BlockSpec { design_px: EM / scale as f32, ..spec("||||", &family) }).expect("four bars compose");
        let starts = run_starts(&block);
        assert_eq!(starts.len(), 4, "each bar is a run of its own: {starts:?}");
        for pair in starts.windows(2) {
            assert_eq!((pair[1] - pair[0]) % scale, 0, "an advance rounded at the generated size lands on a multiple of the scale {scale}: {starts:?}");
        }
    }
}

#[test]
fn a_line_too_long_runs_on_is_squeezed_or_is_cut() {
    let (mut text, family) = engine();
    let fitted = |text: &mut TextContext, fit| text.compose_block(&BlockSpec { width: NARROW, fit, ..spec(LONG_LINE, &family) });

    let overflow = fitted(&mut text, BlockFit::Overflow).expect("an overflowing line composes");
    assert!(overflow.layout_width > NARROW, "nothing holds an overflowing line back: {}", overflow.layout_width);
    assert_eq!(overflow.drawn_width, overflow.width as f32);

    let shrunk = fitted(&mut text, BlockFit::Shrink).expect("a squeezed line composes");
    assert!((shrunk.layout_width - NARROW).abs() < 0.01, "a squeezed line is exactly as wide as its destination: {}", shrunk.layout_width);
    assert!(shrunk.drawn_width < shrunk.width as f32 && shrunk.drawn_width <= NARROW + PIXEL, "it is drawn narrower than it was composed");
    assert_eq!((shrunk.height, &shrunk.rgba), (overflow.height, &overflow.rgba), "only its width changes: the glyphs are the same and as tall");

    let cut = fitted(&mut text, BlockFit::Truncate).expect("a cut line keeps its start");
    assert!(cut.layout_width <= NARROW, "a cut line fits: {}", cut.layout_width);
    assert_eq!(cut.drawn_width, cut.width as f32, "and is not squeezed");
    let prefixes: Vec<TextBlock> = (1..LONG_LINE.len()).filter_map(|end| text.compose_block(&spec(&LONG_LINE[..end], &family))).collect();
    assert!(prefixes.iter().any(|prefix| prefix.rgba == cut.rgba), "what is left is the start of the line with nothing put in place of the rest");
}

#[test]
fn a_line_that_fits_is_left_alone_by_every_overflow_rule() {
    let (mut text, family) = engine();
    let plain = text.compose_block(&spec("FITS", &family)).expect("the word composes");
    for fit in [BlockFit::Shrink, BlockFit::Truncate, BlockFit::Wrap] {
        assert_eq!(text.compose_block(&BlockSpec { fit, ..spec("FITS", &family) }).as_ref(), Some(&plain), "{fit:?} changes nothing in a line that fits");
    }
}

#[test]
fn a_squeezed_line_keeps_its_alignment() {
    let (mut text, family) = engine();
    let squeezed = |text: &mut TextContext, align| {
        text.compose_block(&BlockSpec { width: NARROW, fit: BlockFit::Shrink, align, ..spec(LONG_LINE, &family) }).expect("a squeezed line composes")
    };
    for align in [BlockAlign::Left, BlockAlign::Center, BlockAlign::Right] {
        let block = squeezed(&mut text, align);
        assert!(block.left.abs() <= PIXEL, "{align:?}: a line squeezed to the whole width starts where the width does: {}", block.left);
        assert!((block.left + block.drawn_width - NARROW).abs() <= PIXEL, "{align:?}: and ends where it ends");
    }
}

#[test]
fn wrapping_breaks_a_line_at_its_spaces_and_inside_a_word_that_has_none() {
    let (mut text, family) = engine();
    let width = 150.0;
    let single = text.compose_block(&spec("ONE", &family)).expect("one word composes");

    let words = text.compose_block(&BlockSpec { width, fit: BlockFit::Wrap, ..spec("ONE TWO THREE FOUR FIVE SIX", &family) }).expect("words wrap");
    assert!(words.lines > 1, "a line of words longer than the width is broken: {} lines", words.lines);
    assert!(words.layout_width <= width, "no line is longer than the width: {}", words.layout_width);
    assert!(words.height > single.height * 2, "the lines stack below one another: {} rows", words.height);

    let unbroken = text.compose_block(&BlockSpec { width, fit: BlockFit::Wrap, ..spec("AAAAAAAAAAAAAAAAAAAAAAAA", &family) }).expect("a long word wraps");
    assert!(unbroken.lines > 1, "a word with no space in it is broken where it stops fitting");
    assert!(unbroken.layout_width <= width);

    let right = text
        .compose_block(&BlockSpec { width, fit: BlockFit::Wrap, align: BlockAlign::Right, ..spec("ONE TWO THREE FOUR FIVE SIX", &family) })
        .expect("words wrap right-aligned");
    assert!((right.left + right.drawn_width - width).abs() <= PIXEL, "every wrapped line is pushed against the right edge");
}

#[test]
fn a_block_that_would_show_nothing_is_not_composed() {
    let (mut text, family) = engine();
    let composed = |text: &mut TextContext, spec: BlockSpec<'_>| text.compose_block(&spec).is_some();

    assert!(!composed(&mut text, spec("", &family)), "an empty string");
    assert!(!composed(&mut text, spec("   ", &family)), "a string of nothing but spaces");
    assert!(!composed(&mut text, BlockSpec { design_px: 0.0, ..spec("A", &family) }), "a font generated at no size, which the reference cannot make");
    assert!(!composed(&mut text, BlockSpec { em_px: 0.0, ..spec("A", &family) }), "a destination of no height");
    assert!(!composed(&mut text, BlockSpec { width: f32::NAN, ..spec("A", &family) }), "a width that is not a number");
    assert!(!composed(&mut text, BlockSpec { width: 0.0, fit: BlockFit::Shrink, ..spec("A", &family) }), "a line squeezed into no width");
}

#[test]
fn a_block_is_white_ink_and_no_longer_than_its_caller_can_hold() {
    let (mut text, family) = engine();
    let block = text.compose_block(&spec("Ink", &family)).expect("the word composes");
    assert!(block.rgba.chunks_exact(BYTES_PER_PIXEL).all(|pixel| pixel[..3] == [u8::MAX; 3]), "every pixel is white, so a tint is the whole colour");
    assert!(block.rgba.chunks_exact(BYTES_PER_PIXEL).any(|pixel| pixel[3] == u8::MAX), "and the ink itself is opaque");
    assert_eq!(block.rgba.len(), (block.width * block.height) as usize * BYTES_PER_PIXEL);

    let limit = 16;
    let cropped = text.compose_block(&BlockSpec { max_dim: limit, ..spec(LONG_LINE, &family) }).expect("a cropped line composes");
    assert!(cropped.width <= limit && cropped.height <= limit, "a block is cut off at the longest edge its caller holds");
}

#[test]
fn a_character_no_reachable_font_has_is_still_drawn() {
    let (mut text, family) = engine();
    let block = text.compose_block(&spec("A한", &family)).expect("the line composes");
    let latin = text.compose_block(&spec("A", &family)).expect("the letter composes");
    assert!(block.width > latin.width, "with nothing to fall back to, the missing character takes its place as the font's own empty box");
}

/// Skin fonts carry one script and song titles carry any. Where the reference draws an empty box
/// for a character its font lacks, this draws it from whichever font of the host has it. What a
/// host has installed is not this test's to decide, so a script nothing on the host covers is
/// reported and skipped rather than failed.
#[test]
fn a_character_the_skin_font_lacks_is_drawn_from_a_host_font_that_has_it() {
    let mut text = TextContext::new();
    let family = text.load_font(BUNDLED_FONT.to_vec()).expect("the bundled font loads as a skin font");
    let face = text.face(&family);
    let mixed = "한글 かな カナ Latin";

    let mut uncovered: Vec<char> = Vec::new();
    for character in mixed.chars().filter(|character| !character.is_whitespace()) {
        let mut bytes = [0u8; 4];
        let glyph =
            text.shape_block(&face, character.encode_utf8(&mut bytes), EM, None).into_iter().flatten().next().expect("every character shapes to a glyph");
        if glyph.glyph_id == 0 {
            uncovered.push(character);
        } else {
            let single = text.compose_block(&spec(character.encode_utf8(&mut bytes), &family)).expect("a covered character composes");
            assert!(!inked_columns(&single).is_empty(), "{character:?} has ink");
        }
    }
    assert!(!uncovered.iter().any(char::is_ascii), "the skin font draws its own script itself: {uncovered:?}");
    if uncovered.is_empty() {
        let block = text.compose_block(&spec(mixed, &family)).expect("the mixed line composes");
        let runs = run_starts(&block).len();
        assert!(runs >= mixed.split_whitespace().count(), "every word of the mixed line reaches the block: {runs} runs of ink");
    } else {
        eprintln!("no font on this host covers {uncovered:?}; the fallback for those characters was not exercised");
    }
}

#[test]
fn a_font_file_loaded_twice_is_one_face_under_one_name() {
    let (mut text, family) = engine();
    let faces = text.fs.db().len();
    assert_eq!(text.load_font(BUNDLED_FONT.to_vec()).as_deref(), Some(family.as_str()), "the same bytes answer the name they were given");
    assert_eq!(text.fs.db().len(), faces, "and add nothing to the database");
}

#[test]
fn two_cuts_of_one_family_are_named_apart() {
    let (mut text, family) = engine();
    let regular = text.face(&family);
    assert_eq!(text.name_for(&regular), family, "the cut already loaded keeps the family's own name");

    let black = Face { weight: cosmic_text::Weight::BLACK, id: None, ..regular.clone() };
    let named = text.name_for(&black);
    assert_ne!(named, family, "a second cut cannot share it, or one of the two could never be asked for");
    assert!(named.starts_with(&family) && named.contains(&black.weight.0.to_string()), "it is told apart by its weight: {named:?}");

    text.faces.insert(named.clone(), black.clone());
    assert_eq!(text.face(&named), black, "and asking for that name asks for that cut");
    assert_eq!(text.face("a family nothing was loaded under"), Face::of_family("a family nothing was loaded under"));
}

#[test]
fn a_font_with_no_kern_table_is_laid_out_unkerned() {
    let (mut text, family) = engine();
    let face = text.face(&family);
    assert!(!text.kerned(&face), "the bundled face keeps its kerning in a positioning table, which the reference never reads");

    let mut first_advance = |line: &str| text.shape_block(&face, line, EM, None).into_iter().flatten().next().map(|glyph| glyph.w);
    assert_eq!(first_advance("AV"), first_advance("A"), "so a pair it would tuck together keeps its plain advance");
}

#[test]
fn the_capital_height_is_a_whole_pixel_of_the_generated_size() {
    let (mut text, family) = engine();
    let face = text.face(&family);
    for design in [16.0f32, 25.0, 40.0, 70.0] {
        let metrics = text.design_metrics(&family, &face, design).expect("the bundled face has capitals");
        assert_eq!(metrics.cap.fract(), 0.0, "the reference reads the height of a capital fitted to the grid: {metrics:?}");
        assert!(metrics.cap > design / 2.0 && metrics.cap < design, "which is most of the em and never all of it: {metrics:?}");
        assert!(metrics.line >= design, "and baselines sit at least an em apart: {metrics:?}");
    }
}

#[test]
fn the_small_letters_are_fitted_to_a_pixel_row_unless_that_moves_the_em_too_far() {
    assert_eq!(small_letter_fit(22.0, 40.0), 1.0, "a top already on a row is left there");
    assert_eq!(small_letter_fit(22.25, 40.0), 22.0 / 22.25, "a top a little over a row comes down to it");
    assert_eq!(small_letter_fit(22.5, 40.0), 23.0 / 22.5, "a top well over it goes up to the next");
    assert_eq!(small_letter_fit(1.3, 40.0), 1.0, "a fit that would move the top of the em two pixels is given up");
    assert_eq!(small_letter_fit(0.0, 40.0), 1.0, "a font with no small letter to measure is not rescaled");
}

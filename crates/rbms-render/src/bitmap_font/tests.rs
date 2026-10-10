//! Tests of the bitmap font reader and of the layout carried over from the reference's library.
//!
//! Most of them read the font this repository keeps for the purpose, `tests/skin/fonts/mini.fnt`,
//! which is written the way the tool the published fonts were made with writes one: every glyph is
//! padded by two pixels on each side, and the advances are the unpadded ones. It has four glyphs
//! on two pages -- `A` (ink six by eight) and `B` (ink four by eight, one pixel right of the pen)
//! on the first, `y` (ink six by ten, reaching four below the baseline) and an ideograph (ink eight
//! by eight) on the second -- a space with no pixels, one kerning pair that pulls `B` two pixels
//! towards an `A` before it, and three lines that are broken on purpose.

use std::path::{Path, PathBuf};

use super::*;

/// The size the fixture was made at.
const FIXTURE_SIZE: f32 = 10.0;

/// The fixture's line height and baseline.
const FIXTURE_LINE: f32 = 16.0;
const FIXTURE_BASE: f32 = 12.0;

/// The padding on every side of each of the fixture's glyphs.
const FIXTURE_PAD: f32 = 2.0;

/// The advances of the fixture's `A`, `B` and space.
const ADVANCE_A: f32 = 8.0;
const ADVANCE_B: f32 = 7.0;
const ADVANCE_SPACE: f32 = 4.0;

/// How much closer the fixture sets a `B` to an `A` before it.
const KERN_AB: f32 = 2.0;

/// The width of the ink of the fixture's `A`.
const INK_A: f32 = 6.0;

/// A width no test line comes near.
const ROOMY: f32 = 100.0;

/// The ideograph the fixture has a glyph for.
const IDEOGRAPH: char = '\u{6f22}';

/// A character the fixture has no glyph for.
const MISSING: char = 'X';

/// The advance a test gives the character the fixture lacks.
const MISSING_ADVANCE: f32 = 5.0;

/// The folder the fixture is in, which is also the skin folder it is read inside.
fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("skin").join("fonts")
}

fn fixture() -> BitmapFont {
    let path = fixture_dir().join("mini.fnt");
    let text = std::fs::read_to_string(&path).expect("the fixture font is readable");
    BitmapFont::parse(&text, &path, &fixture_dir()).expect("the fixture font parses")
}

/// Reads `text` as a font file in the fixture's folder.
fn parsed(text: &str) -> Result<BitmapFont, String> {
    BitmapFont::parse(text, &fixture_dir().join("made-up.fnt"), &fixture_dir())
}

/// A file with the two header lines, one page and whatever `rest` adds.
fn with_header(rest: &str) -> String {
    format!(
        "info face=\"Made Up\" size=20 padding=1,2,3,4 spacing=-6,-4\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64 pages=1 packed=0\npage id=0 file=\"mini_0.png\"\n{rest}"
    )
}

/// Lays `text` out in the fixture at one to one, answering a stand-in for the character it lacks.
fn laid(font: &BitmapFont, text: &str, target: f32, align: BlockAlign, fit: BlockFit) -> Laid {
    font.layout_fitted(text, &mut |ch| resolve(font, ch), target, align, fit, 1.0).0
}

fn resolve(font: &BitmapFont, ch: char) -> Option<Piece> {
    font.piece(ch).or((ch == MISSING).then_some(Piece::Fallback { ch, advance: MISSING_ADVANCE }))
}

/// Where the pen stands for each piece of a run, measured from the left edge of the layout.
fn pens(run: &Run) -> Vec<f32> {
    run.placed().map(|(pen, _)| run.x + pen).collect()
}

fn text_of(run: &Run) -> String {
    run.pieces.iter().map(Piece::ch).collect()
}

#[test]
fn the_fixture_reads_as_the_font_it_describes() {
    let font = fixture();

    assert_eq!(font.original_size(), FIXTURE_SIZE);
    assert_eq!((font.line_height(), font.base()), (FIXTURE_LINE, FIXTURE_BASE));
    assert_eq!(font.page_size(), (32.0, 16.0));
    assert_eq!(font.padding(), Padding { top: FIXTURE_PAD, right: FIXTURE_PAD, bottom: FIXTURE_PAD, left: FIXTURE_PAD });
    assert_eq!(font.glyph_count(), 5, "A, B, y, the ideograph and the space");
    assert_eq!(font.page_count(), 2);
    assert_eq!(font.page(0), Some(fixture_dir().join("mini_0.png").as_path()));
    assert_eq!(font.page(1), Some(fixture_dir().join("mini_1.png").as_path()));
    assert_eq!(font.page(2), None);

    let ideograph = font.glyph(IDEOGRAPH).expect("the ideograph has a glyph");
    assert_eq!((ideograph.page, ideograph.x, ideograph.width, ideograph.xadvance), (1, 10, 12, 10));
    assert_eq!(font.kerning('A', 'B'), -(KERN_AB as i32));
    assert_eq!(font.kerning('B', 'A'), 0, "the pair with no amount is not a pair");
}

#[test]
fn a_broken_line_is_skipped_and_said() {
    let font = fixture();

    assert!(font.glyph('C').is_none(), "the glyph whose x is no number is not kept");
    let warnings = font.warnings();
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert!(warnings[0].contains("line 9") && warnings[0].contains("glyph"), "{warnings:?}");
    assert!(warnings[1].contains("line 12") && warnings[1].contains("not a font record"), "{warnings:?}");
    assert!(warnings[2].contains("line 15") && warnings[2].contains("kerning"), "{warnings:?}");
}

#[test]
fn the_padding_comes_off_the_capital_height_and_widens_the_space() {
    let font = fixture();

    let capital = font.glyph('B').expect("the fixture has a B, the first capital the library looks for that it has");
    assert_eq!(font.cap_height(), capital.height as f32 - 2.0 * FIXTURE_PAD);
    assert_eq!(font.ascent(), FIXTURE_BASE - font.cap_height());
    assert_eq!(capital.yoffset as f32 + FIXTURE_PAD, font.ascent(), "a capital's ink starts on the capital line");

    let space = font.glyph(' ').expect("the fixture has a space");
    assert_eq!(space.width as f32, FIXTURE_PAD + ADVANCE_SPACE + FIXTURE_PAD);
    assert_eq!(space.xoffset as f32, -FIXTURE_PAD);
    assert_eq!(font.space_width(), space.width as f32);
}

#[test]
fn a_file_written_by_the_usual_tool_reads_whatever_its_line_ends_and_spacing() {
    let glyphs = "chars count=2\nchar id=77      x=0    y=0    width=9    height=14   xoffset=-4   yoffset=3    xadvance=11   page=0    chnl=0 \nchar id=108     x=9    y=0    width=7    height=14   xoffset=-4   yoffset=3    xadvance=5    page=0    chnl=0 \n";
    let text = with_header(glyphs).replace('\n', "\r\n");
    let font = parsed(&text).expect("the file parses");

    assert!(font.warnings().is_empty(), "{:?}", font.warnings());
    assert_eq!(font.original_size(), 20.0);
    assert_eq!(font.padding(), Padding { top: 1.0, right: 2.0, bottom: 3.0, left: 4.0 });
    assert_eq!(font.cap_height(), 14.0 - 4.0, "the padding above and below comes off the capital");
    let space = font.glyph(' ').expect("a font with no space is given one");
    assert_eq!(space.xadvance, 5, "the made-up space advances like the small L");
    assert_eq!((space.width, space.xoffset), (4 + 5 + 2, -4));
}

#[test]
fn a_font_that_gives_no_size_is_as_large_as_its_line() {
    let text = "info face=\"No Size\" padding=0,0,0,0\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64 pages=1\npage id=0 file=mini_0.png\nchar id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\n";
    let font = parsed(text).expect("the file parses");

    assert_eq!(font.original_size(), 30.0);
    assert_eq!(font.page(0), Some(fixture_dir().join("mini_0.png").as_path()), "a file name need not be quoted");
}

#[test]
fn a_page_outside_the_skin_folder_is_refused() {
    let text = "info size=20 padding=0,0,0,0\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64 pages=3\npage id=0 file=\"../outside.png\"\npage id=1 file=\"/etc/outside.png\"\npage id=2 file=\"deeper\\inside.png\"\nchar id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\n";
    let font = parsed(text).expect("the file parses");

    assert_eq!(font.page_count(), 3);
    assert_eq!(font.page(0), None, "a page above the skin folder is not read");
    assert_eq!(font.page(1), None, "a page somewhere else altogether is not read");
    assert_eq!(font.page(2), Some(fixture_dir().join("deeper").join("inside.png").as_path()), "a page in a folder of the skin is, either slash");
    assert_eq!(font.warnings().iter().filter(|warning| warning.contains("left out")).count(), 2, "{:?}", font.warnings());
}

/// One glyph line of a made-up font: an `A` with pixels.
const GLYPH_A: &str = "char id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\n";

/// The largest number a line of a font file can give, which is also more pages than any machine
/// has room to list.
const HUGE: i32 = i32::MAX;

/// The pages are the `page` lines in the order they come, as the library reads them. A line that
/// numbers itself anything but its place is a page out of turn: its place is kept, with no image,
/// so the pages after it are still the ones their glyphs name.
#[test]
fn a_page_is_the_place_of_its_line_and_one_numbered_otherwise_is_left_out() {
    let text = format!(
        "info size=20 padding=0,0,0,0\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64 pages=4\npage id=0 file=\"a.png\"\npage id=7 file=\"b.png\"\npage file=\"c.png\"\npage id=3\n{GLYPH_A}"
    );
    let font = parsed(&text).expect("the file parses");

    assert_eq!(font.page_count(), 4);
    assert_eq!(font.page(0), Some(fixture_dir().join("a.png").as_path()));
    assert_eq!(font.page(1), None, "the second page line says it is page seven");
    assert_eq!(font.page(2), Some(fixture_dir().join("c.png").as_path()), "a line that gives itself no number is the page of its place");
    assert_eq!(font.page(3), None, "a page with no file has no image");
    assert_eq!(font.skipped(), 2);
    assert!(font.warnings()[0].contains("line 4") && font.warnings()[0].contains("page 1"), "{:?}", font.warnings());
    assert!(font.warnings()[1].contains("line 6") && font.warnings()[1].contains("no file"), "{:?}", font.warnings());
}

/// No number in a font file sizes anything that is kept. A page that numbers itself two thousand
/// million, a header that says there are that many pages and a glyph that says it is on the last of
/// them are all read, and the font has the one page its one `page` line is.
#[test]
fn a_number_in_the_file_never_says_how_much_is_kept() {
    let numbered = format!("info size=20 padding=0,0,0,0\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64 pages=1\npage id={HUGE} file=\"a.png\"\n{GLYPH_A}");
    let font = parsed(&numbered).expect("the file parses");
    assert_eq!((font.page_count(), font.page(0)), (1, None), "a page numbered out of turn is left out, not made room for");
    assert_eq!(font.skipped(), 1);

    let past_any_number = "info size=20 padding=0,0,0,0\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64\npage id=99999999999999999999999999 file=\"a.png\"\n"
        .to_string()
        + GLYPH_A;
    assert_eq!(parsed(&past_any_number).expect("the file parses").page(0), None, "nor is one numbered past any number");

    let declared = format!(
        "info size=20 padding=0,0,0,0\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64 pages={HUGE}\npage id=0 file=\"a.png\"\nchar id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page={HUGE}\n"
    );
    let font = parsed(&declared).expect("the file parses");
    assert_eq!(font.page_count(), 1, "the pages are the page lines, however many the header says there are");
    assert_eq!(font.page(font.glyph('A').expect("the glyph is kept").page), None, "a glyph on a page the file has no line for draws from nothing");
    assert_eq!(font.skipped(), 0);
}

/// An advance and its kerning are added as whole numbers that wrap, which is what the library's
/// arithmetic does with a file that gives numbers too large to add.
#[test]
fn an_advance_too_large_to_add_its_kerning_to_wraps_around() {
    let text = with_header(&format!(
        "char id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance={HUGE} page=0\nchar id=66 x=9 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\nkerning first=65 second=66 amount=1\n"
    ));
    let font = parsed(&text).expect("the file parses");

    let run = &laid(&font, "AB", ROOMY, BlockAlign::Left, BlockFit::Overflow).runs[0];
    assert_eq!(pens(run), [0.0, HUGE.wrapping_add(1) as f32]);
    for fit in [BlockFit::Shrink, BlockFit::Truncate, BlockFit::Wrap] {
        for align in [BlockAlign::Left, BlockAlign::Center, BlockAlign::Right] {
            laid(&font, "AB BA AAB", ROOMY, align, fit);
        }
    }
}

/// Only the first few skipped lines are kept in words. A file is as long as whoever wrote it made
/// it, and each of its lines may be one that is skipped.
#[test]
fn the_lines_skipped_are_counted_and_only_the_first_few_are_kept_in_words() {
    const BROKEN_LINES: usize = 10_000;
    let text = with_header(GLYPH_A) + &"x\n".repeat(BROKEN_LINES);
    let font = parsed(&text).expect("the file parses");

    assert_eq!(font.skipped(), BROKEN_LINES);
    assert_eq!(font.warnings().len(), WARNINGS_KEPT);
    assert!(font.warnings()[0].contains("line 5") && font.warnings()[0].contains("not a font record"), "{:?}", font.warnings());
}

#[test]
fn a_font_with_nothing_to_lay_a_line_out_by_is_an_error() {
    let headless = "info size=20 padding=0,0,0,0\nchar id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\n";
    assert!(parsed(headless).is_err(), "no common line");

    let empty = with_header("char id=32 x=0 y=0 width=0 height=0 xoffset=0 yoffset=0 xadvance=5 page=0\n");
    assert!(parsed(&empty).is_err(), "no glyph with pixels");
}

#[test]
fn the_missing_glyph_and_the_pairs_past_the_basic_plane_are_not_kept() {
    let text = with_header(
        "char id=0 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\nchar id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\nchar id=131072 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\nkerning first=65 second=131072 amount=-3\nkerning first=66 second=65 amount=-3\nkerning first=65 second=65 amount=-300\n",
    );
    let font = parsed(&text).expect("the file parses");

    assert_eq!(font.glyph_count(), 3, "A, the glyph past the basic plane and the made-up space");
    assert!(font.glyph('\u{20000}').is_some());
    assert_eq!(font.kerning('A', '\u{20000}'), 0, "a pair past the basic plane is dropped");
    assert_eq!(font.kerning('B', 'A'), 0, "a pair whose first glyph the font lacks is dropped");
    assert_eq!(font.kerning('A', 'A'), i32::from(-300i32 as i8), "an amount is kept in a byte, as the library keeps it");
}

#[test]
fn the_wave_dash_and_the_fullwidth_tilde_stand_in_for_each_other() {
    let text = with_header(
        "char id=65 x=0 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=11 page=0\nchar id=65374 x=9 y=0 width=9 height=14 xoffset=0 yoffset=3 xadvance=13 page=0\nkerning first=65374 second=65 amount=-3\n",
    );
    let font = parsed(&text).expect("the file parses");

    let own = font.piece('\u{ff5e}').expect("the font has the tilde");
    let borrowed = font.piece('\u{301c}').expect("the wave dash borrows the tilde's glyph");
    assert!(matches!(own, Piece::Glyph { kerned: true, .. }));
    assert!(matches!(borrowed, Piece::Glyph { ch: '\u{301c}', kerned: false, glyph } if glyph.id == 0xff5e));
    assert!(font.piece(MISSING).is_none());

    let kerned = font.layout("\u{ff5e}A", &mut |ch| font.piece(ch), ROOMY, BlockAlign::Left, BlockFit::Overflow, (1.0, 1.0));
    let plain = font.layout("\u{301c}A", &mut |ch| font.piece(ch), ROOMY, BlockAlign::Left, BlockFit::Overflow, (1.0, 1.0));
    assert_eq!(pens(&kerned.runs[0]), [0.0, 10.0]);
    assert_eq!(pens(&plain.runs[0]), [0.0, 13.0], "a borrowed glyph brings no kerning with it");
}

#[test]
fn a_line_starts_at_its_first_origin_and_ends_at_its_last_advance() {
    let font = fixture();
    let line = laid(&font, "AB", ROOMY, BlockAlign::Left, BlockFit::Overflow);

    assert_eq!(line.runs.len(), 1);
    assert_eq!(pens(&line.runs[0]), [0.0, ADVANCE_A - KERN_AB]);
    assert_eq!(line.width, ADVANCE_A - KERN_AB + ADVANCE_B, "the padding moves neither end of a plain line");
    assert_eq!(line.runs[0].width, line.width);
}

#[test]
fn a_line_is_pushed_against_the_side_its_alignment_names() {
    let font = fixture();
    let width = ADVANCE_A - KERN_AB + ADVANCE_B;

    assert_eq!(laid(&font, "AB", ROOMY, BlockAlign::Left, BlockFit::Overflow).runs[0].x, 0.0);
    assert_eq!(laid(&font, "AB", ROOMY, BlockAlign::Center, BlockFit::Overflow).runs[0].x, (ROOMY - width) / 2.0);
    assert_eq!(laid(&font, "AB", ROOMY, BlockAlign::Right, BlockFit::Overflow).runs[0].x, ROOMY - width);
}

#[test]
fn a_scale_multiplies_every_advance() {
    let font = fixture();
    let line = font.layout("AB", &mut |ch| font.piece(ch), ROOMY, BlockAlign::Left, BlockFit::Overflow, (2.0, 2.0));

    assert_eq!(pens(&line.runs[0]), [0.0, 2.0 * (ADVANCE_A - KERN_AB)]);
    assert_eq!(line.width, 2.0 * (ADVANCE_A - KERN_AB + ADVANCE_B));
}

#[test]
fn a_character_the_font_lacks_takes_the_room_its_stand_in_asks_for() {
    let font = fixture();
    let line = laid(&font, "AXB", ROOMY, BlockAlign::Left, BlockFit::Overflow);

    assert_eq!(pens(&line.runs[0]), [0.0, ADVANCE_A, ADVANCE_A + MISSING_ADVANCE]);
    assert_eq!(line.width, ADVANCE_A + MISSING_ADVANCE + ADVANCE_B);
    assert!(matches!(line.runs[0].pieces[1], Piece::Fallback { ch: MISSING, .. }));

    let skipped = font.layout("A\u{7}B", &mut |ch| font.piece(ch), ROOMY, BlockAlign::Left, BlockFit::Overflow, (1.0, 1.0));
    assert_eq!(text_of(&skipped.runs[0]), "AB", "a character nothing draws takes no room");
    assert_eq!(pens(&skipped.runs[0]), [0.0, ADVANCE_A - KERN_AB]);
}

#[test]
fn shrinking_squeezes_a_long_line_to_exactly_its_width() {
    let font = fixture();
    let natural = 4.0 * ADVANCE_A;
    let target = natural / 2.0;

    let (line, scale_x) = font.layout_fitted("AAAA", &mut |ch| font.piece(ch), target, BlockAlign::Left, BlockFit::Shrink, 1.0);
    assert_eq!(scale_x, 0.5);
    assert_eq!(line.width, target);
    assert_eq!(pens(&line.runs[0]), [0.0, 4.0, 8.0, 12.0]);

    let (fits, unsqueezed) = font.layout_fitted("AAAA", &mut |ch| font.piece(ch), natural, BlockAlign::Left, BlockFit::Shrink, 1.0);
    assert_eq!(unsqueezed, 1.0, "a line exactly as wide as its room is left alone");
    assert_eq!(fits.width, natural);

    let (long, kept) = font.layout_fitted("AAAA", &mut |ch| font.piece(ch), target, BlockAlign::Left, BlockFit::Overflow, 1.0);
    assert_eq!((long.width, kept), (natural, 1.0), "a line allowed to run on is not squeezed");
}

#[test]
fn cutting_keeps_the_glyphs_whose_advance_ends_inside_and_ends_the_line_at_ink() {
    let font = fixture();
    let cut = |target: f32| laid(&font, "AAAA", target, BlockAlign::Left, BlockFit::Truncate);

    let two = cut(20.0);
    assert_eq!(text_of(&two.runs[0]), "AA", "the third A would end at 24");
    assert_eq!(two.width, ADVANCE_A + INK_A, "the line ends at the last glyph's ink, the padding taken off");

    let three = cut(24.0);
    assert_eq!(text_of(&three.runs[0]), "AAA", "an advance that ends exactly on the width is inside it");
    assert_eq!(three.width, 2.0 * ADVANCE_A + INK_A);

    assert_eq!(text_of(&cut(4.0 * ADVANCE_A).runs[0]), "AAAA", "a line that fits is not cut");

    let none = cut(5.0);
    assert!(none.runs.iter().all(|run| run.pieces.is_empty()), "a width the second glyph already starts past keeps nothing");
    assert_eq!(text_of(&laid(&font, "A", 1.0, BlockAlign::Left, BlockFit::Truncate).runs[0]), "A", "a lone glyph is never cut");

    let right = laid(&font, "AAAA", 20.0, BlockAlign::Right, BlockFit::Truncate);
    assert_eq!(right.runs[0].x, 20.0 - (ADVANCE_A + INK_A), "what is left is aligned by its new width");

    let first_line_only = laid(&font, "AAAA\nB", 20.0, BlockAlign::Left, BlockFit::Truncate);
    assert_eq!(first_line_only.runs.len(), 1, "nothing after the cut is laid out");
}

#[test]
fn wrapping_breaks_after_the_last_space_and_keeps_that_space_on_the_line() {
    let font = fixture();
    let wrapped = laid(&font, "AA AA", 20.0, BlockAlign::Left, BlockFit::Wrap);

    assert_eq!(wrapped.runs.len(), 2);
    assert_eq!((text_of(&wrapped.runs[0]), wrapped.runs[0].row), ("AA ".to_string(), 0));
    assert_eq!((text_of(&wrapped.runs[1]), wrapped.runs[1].row), ("AA".to_string(), 1));
    assert_eq!(wrapped.runs[0].width, 2.0 * ADVANCE_A + ADVANCE_SPACE, "a line that ends on a space keeps the space's advance");
    assert_eq!(wrapped.runs[1].width, 2.0 * ADVANCE_A);
    assert_eq!(wrapped.width, 2.0 * ADVANCE_A + ADVANCE_SPACE);
}

#[test]
fn a_wrapped_line_starts_at_its_first_glyphs_ink_and_the_one_before_ends_at_ink() {
    let font = fixture();
    let b = font.glyph('B').expect("the fixture has a B");

    let plain = laid(&font, "BA", ROOMY, BlockAlign::Left, BlockFit::Wrap);
    assert_eq!(pens(&plain.runs[0])[0], 0.0, "an unwrapped line starts at its first glyph's origin");

    let wrapped = laid(&font, "AAA BA", 30.0, BlockAlign::Left, BlockFit::Wrap);
    assert_eq!(text_of(&wrapped.runs[1]), "BA");
    let ink_offset = b.xoffset as f32 + FIXTURE_PAD;
    assert_eq!(pens(&wrapped.runs[1])[0], -ink_offset, "the continuation is pulled left by how far its first glyph's ink sits from the pen");

    let unbroken = laid(&font, "AAAA", 20.0, BlockAlign::Left, BlockFit::Wrap);
    assert_eq!(unbroken.runs.iter().map(text_of).collect::<Vec<_>>(), ["AA", "AA"], "a word with no space is broken before the glyph that overflows");
    assert_eq!(unbroken.runs[0].width, ADVANCE_A + INK_A, "the line that was broken ends at its last glyph's ink");
    assert_eq!(unbroken.runs[1].width, 2.0 * ADVANCE_A, "the last line ends at its last glyph's advance");
}

#[test]
fn a_width_no_wider_than_a_space_is_not_wrapped_in() {
    let font = fixture();
    let narrow = laid(&font, "AAAA", font.space_width(), BlockAlign::Left, BlockFit::Wrap);

    assert_eq!(narrow.runs.len(), 1, "one line a glyph would be the worst wrapping there is");
    assert_eq!(text_of(&narrow.runs[0]), "AAAA");
}

#[test]
fn a_line_feed_starts_a_line_and_an_empty_line_still_takes_a_row() {
    let font = fixture();

    let two = laid(&font, "A\nB", ROOMY, BlockAlign::Left, BlockFit::Overflow);
    assert_eq!(two.runs.iter().map(|run| (text_of(run), run.row)).collect::<Vec<_>>(), [("A".to_string(), 0), ("B".to_string(), 1)]);
    assert_eq!(two.width, ADVANCE_A, "the width is the longest line's");

    let gap = laid(&font, "A\n\nB", ROOMY, BlockAlign::Right, BlockFit::Overflow);
    assert_eq!(gap.runs.iter().map(|run| run.row).collect::<Vec<_>>(), [0, 2]);
    assert_eq!(gap.runs.iter().map(|run| run.x).collect::<Vec<_>>(), [ROOMY - ADVANCE_A, ROOMY - ADVANCE_B], "each line is aligned by its own width");
}

#[test]
fn the_extension_alone_makes_a_font_a_bitmap_font() {
    assert!(is_bitmap_font(Path::new("font/main.fnt")));
    assert!(is_bitmap_font(Path::new("font/MAIN.FNT")));
    assert!(!is_bitmap_font(Path::new("font/main.ttf")));
    assert!(!is_bitmap_font(Path::new("font/fnt")));
}

#[test]
fn a_file_is_parsed_once_until_its_bytes_change() {
    let bytes = std::fs::read(fixture_dir().join("mini.fnt")).expect("the fixture font is readable");
    let path = fixture_dir().join("read-twice.fnt");

    let first = load(&path, &fixture_dir(), &bytes).expect("the fixture font loads");
    let again = load(&path, &fixture_dir(), &bytes).expect("the fixture font loads again");
    assert!(Arc::ptr_eq(&first, &again), "the same bytes answer the font that was parsed before");

    let edited = String::from_utf8_lossy(&bytes).replace("size=10", "size=12");
    let reread = load(&path, &fixture_dir(), edited.as_bytes()).expect("the edited font loads");
    assert!(!Arc::ptr_eq(&first, &reread));
    assert_eq!(reread.original_size(), 12.0);

    assert!(load(&path, &fixture_dir(), b"not a font").is_err());
    load(&path, &fixture_dir(), &bytes).expect("the fixture font loads once more");
}

/// Which of a font's pages are inside the skin folder is part of what is parsed, so the same file
/// read inside another folder is another font.
#[test]
fn a_file_read_inside_another_skin_folder_is_parsed_for_that_folder() {
    let text =
        "info size=20 padding=0,0,0,0\ncommon lineHeight=30 base=24 scaleW=64 scaleH=64 pages=1\npage id=0 file=\"../above.png\"\n".to_string() + GLYPH_A;
    let path = fixture_dir().join("two-folders.fnt");
    let above = fixture_dir().parent().expect("the fixture folder is in a folder").to_path_buf();

    let inside = load(&path, &fixture_dir(), text.as_bytes()).expect("the font loads");
    let wider = load(&path, &above, text.as_bytes()).expect("the font loads inside the folder above");
    assert_eq!(inside.page(0), None, "the page is above the first folder");
    assert_eq!(wider.page(0), Some(above.join("above.png").as_path()), "and inside the second");
    assert!(Arc::ptr_eq(&inside, &load(&path, &fixture_dir(), text.as_bytes()).expect("the font loads again")), "each folder keeps its own parse");
}

/// A font no screen draws with any more is let go of when a font of another skin folder is read; one
/// still in use is kept, whatever folder it is from.
#[test]
fn reading_a_font_of_another_folder_lets_go_of_the_fonts_nobody_holds() {
    let bytes = std::fs::read(fixture_dir().join("mini.fnt")).expect("the fixture font is readable");
    let above = fixture_dir().parent().expect("the fixture folder is in a folder").to_path_buf();
    let (unheld_path, held_path) = (fixture_dir().join("let-go-of.fnt"), fixture_dir().join("still-held.fnt"));

    let unheld = Arc::downgrade(&load(&unheld_path, &fixture_dir(), &bytes).expect("the fixture font loads"));
    let held = load(&held_path, &fixture_dir(), &bytes).expect("the fixture font loads");
    load(&fixture_dir().join("elsewhere.fnt"), &above, &bytes).expect("the fixture font loads inside the folder above");

    assert!(unheld.upgrade().is_none(), "the font nobody held went when another folder's font was read");
    assert!(Arc::ptr_eq(&held, &load(&held_path, &fixture_dir(), &bytes).expect("the fixture font loads again")), "the one in use was kept");
}

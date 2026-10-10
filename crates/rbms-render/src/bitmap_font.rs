//! Bitmap fonts: the BMFont text format a skin's `.fnt` file is written in, read and laid out the
//! way the reference reads and lays it out.
//!
//! The reference hands a `.fnt` file to libGDX (`SkinTextBitmap.SkinTextBitmapSource`, over
//! `BitmapFont.BitmapFontData`, `GlyphLayout` and `BitmapFontCache` of libGDX 1.9.9), so what a line
//! of such a font measures and where each glyph lands is whatever that library computes. This module
//! is that computation and nothing else: it reads no image and draws nothing. What it follows:
//!
//! - **The file.** `info` gives the size the font was made at and the padding its maker put around
//!   every glyph; `common` gives the line height, the baseline and the number of pages; each `page`
//!   names an image beside the file; each `char` is a glyph and each `kerning` a pair adjustment.
//!   A glyph with an id of zero or less is the library's "missing glyph" and is not kept, because a
//!   character a font lacks is drawn from another font here ([`Piece::Fallback`]).
//! - **The padding.** The capital height is the height of the first capital the font has, less the
//!   padding above and below it, and the distance from the top of a line to its baseline's glyph
//!   origin is the baseline less that capital height. A space the file gives no width is widened by
//!   the padding on either side. None of this moves the first or the last glyph of a plain line:
//!   the library measures those tightly only for a run that ends at a colour tag, which the
//!   reference never enables.
//! - **The line.** The first glyph's origin is the left edge of the line and the line ends at the
//!   last glyph's advance. A line that is cut short or wrapped ends instead at the ink of the glyph
//!   it now ends on, less the right padding, and a wrapped line's continuation starts at the ink of
//!   its first glyph, less the left padding.
//! - **The scale.** Padding is scaled by the vertical scale on both axes, as the library does.
//!
//! Two things are not the reference's. A page image is looked for beside the `.fnt` file and nowhere
//! outside the skin's folder; a page that names a file elsewhere is refused with a warning and its
//! glyphs draw nothing. And a file the library would give up on line by line -- a record missing a
//! number, a page numbered out of turn -- loses that line here and keeps the rest.
//!
//! A font file is a skin's, which is to say a stranger's: nothing in it is trusted to be small or
//! sensible. No number in it sizes anything that is kept -- the pages are as many as the `page` lines
//! and the glyphs as many as the `char` lines -- only the first few of the lines that were skipped
//! are kept in words, and the arithmetic on a glyph's numbers is the library's own, which wraps.

use std::collections::HashMap;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use rbms_skin::resolve::contained;

use crate::font::{BlockAlign, BlockFit, CAP_CHARS, slack};
use crate::locked;

#[cfg(test)]
mod tests;

/// The extension that makes a skin's font a bitmap font (`JsonSkinObjectLoader.createText`: the
/// path, lower-cased, ends in `.fnt`).
const FONT_EXTENSION: &str = "fnt";

/// The glyph a font with no space takes its space's advance from (`BitmapFontData.load`).
const SPACE_STAND_IN: char = 'l';

/// The last character the library keeps glyphs and kerning pairs for: a Java `char`.
const LAST_KEPT_ID: u32 = 0xFFFF;

/// How far past the width a glyph may reach before the line is wrapped or cut
/// (`GlyphLayout.setText`).
const WRAP_SLACK: f32 = 0.0001;

/// How many numbers a `padding` field holds: top, right, bottom, left.
const PADDING_SIDES: usize = 4;

/// The page a glyph is on when its line names none.
const FIRST_PAGE: usize = 0;

/// How many of the lines skipped while a file is read are kept in words; the rest are only counted.
/// A broken file is broken on every line, and a file may have any number of lines.
pub const WARNINGS_KEPT: usize = 3;

/// The wave dash and the fullwidth tilde, each of which the reference draws with the other's glyph
/// when a font has only one of them (`SkinTextBitmapSource.getEquivalentBmpCodePoint`).
const WAVE_DASH: char = '\u{301c}';
const FULLWIDTH_TILDE: char = '\u{ff5e}';

/// Whether a skin's font file is a bitmap font, which its extension alone decides.
pub fn is_bitmap_font(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(FONT_EXTENSION))
}

/// The character the reference draws in place of `ch` when a font lacks it, for the one pair of
/// characters it treats as the same.
pub fn equivalent_of(ch: char) -> Option<char> {
    match ch {
        WAVE_DASH => Some(FULLWIDTH_TILDE),
        FULLWIDTH_TILDE => Some(WAVE_DASH),
        _ => None,
    }
}

/// The characters a line is broken after and that never end a line with their ink
/// (`BitmapFontData.isWhitespace`).
fn is_whitespace(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\r' | ' ')
}

/// One glyph of a font, as its `char` line gives it: where it sits on its page, in pixels from the
/// page's top left, and where that rectangle is put relative to the pen, with `yoffset` measured
/// down from the top of the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitmapGlyph {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub xoffset: i32,
    pub yoffset: i32,
    pub xadvance: i32,
    pub page: usize,
}

impl BitmapGlyph {
    /// Whether the glyph has pixels to draw.
    pub fn has_ink(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// The room a font's maker left around every glyph, in pixels of the page.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// A bitmap font: its glyphs, its kerning pairs, the pages they are drawn from and the handful of
/// measurements a line is laid out by.
#[derive(Debug, Clone, PartialEq)]
pub struct BitmapFont {
    original_size: f32,
    line_height: f32,
    base: f32,
    page_size: (f32, f32),
    padding: Padding,
    pages: Vec<Option<PathBuf>>,
    glyphs: HashMap<u32, BitmapGlyph>,
    kernings: HashMap<(u32, u32), i32>,
    cap_height: f32,
    space_width: f32,
    skipped: Skipped,
}

/// What was skipped while a file was read: the first [`WARNINGS_KEPT`] lines in words, and how many
/// there were in all.
#[derive(Debug, Clone, Default, PartialEq)]
struct Skipped {
    said: Vec<String>,
    count: usize,
}

impl Skipped {
    /// Counts one more skipped line, and keeps what `line` says of it while there is room.
    fn note(&mut self, line: impl FnOnce() -> String) {
        if self.said.len() < WARNINGS_KEPT {
            self.said.push(line());
        }
        self.count += 1;
    }
}

/// One `key=value` field of a line. A value in quotes runs to the closing quote.
fn fields(line: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(equals) = rest.find('=') {
        let key = rest[..equals].rsplit(char::is_whitespace).next().unwrap_or_default();
        let after = &rest[equals + 1..];
        let (value, tail) = match after.strip_prefix('"') {
            Some(quoted) => match quoted.find('"') {
                Some(end) => (&quoted[..end], &quoted[end + 1..]),
                None => (quoted.trim_end(), ""),
            },
            None => {
                let end = after.find(char::is_whitespace).unwrap_or(after.len());
                (&after[..end], &after[end..])
            }
        };
        found.push((key, value));
        rest = tail;
    }
    found
}

/// The whole number a line gives for `key`, or `None` when it gives none or one that is no number.
fn number(fields: &[(&str, &str)], key: &str) -> Option<i32> {
    fields.iter().find(|(name, _)| *name == key).and_then(|(_, value)| value.parse().ok())
}

/// The file a `page` line names: everything after the last `file=`, with one pair of quotes taken
/// off (`BitmapFontData.load`: `.*file="?([^"]+)"?`).
fn page_file(line: &str) -> Option<&str> {
    const KEY: &str = "file=";
    let after = &line[line.rfind(KEY)? + KEY.len()..];
    let unquoted = after.strip_prefix('"').unwrap_or(after);
    let name = unquoted.split('"').next().unwrap_or_default().trim_end();
    (!name.is_empty()).then_some(name)
}

/// The four numbers of an `info` line's `padding`, or `None` when it does not hold exactly four.
fn padding_of(fields: &[(&str, &str)]) -> Option<Padding> {
    let (_, value) = fields.iter().find(|(name, _)| *name == "padding")?;
    let sides: Vec<f32> = value.split(',').map(|side| side.trim().parse::<i32>().map(|side| side as f32)).collect::<Result<_, _>>().ok()?;
    (sides.len() == PADDING_SIDES).then(|| Padding { top: sides[0], right: sides[1], bottom: sides[2], left: sides[3] })
}

/// The glyph a `char` line describes and the id it gives, or `None` for a line missing one of the
/// numbers every glyph has. The id is kept apart because it may be zero or less.
fn glyph_of(fields: &[(&str, &str)]) -> Option<(i32, BitmapGlyph)> {
    let id = number(fields, "id")?;
    let glyph = BitmapGlyph {
        id: u32::try_from(id).unwrap_or_default(),
        x: number(fields, "x")?,
        y: number(fields, "y")?,
        width: number(fields, "width")?,
        height: number(fields, "height")?,
        xoffset: number(fields, "xoffset")?,
        yoffset: number(fields, "yoffset")?,
        xadvance: number(fields, "xadvance")?,
        page: number(fields, "page").and_then(|page| usize::try_from(page).ok()).unwrap_or(FIRST_PAGE),
    };
    Some((id, glyph))
}

/// The number a `page` line gives itself, when it gives one: the digits after its `id=`
/// (`BitmapFontData.load`: `.*id=(\\d+)`). `Some(None)` is a number too long to be one.
fn page_number(fields: &[(&str, &str)]) -> Option<Option<usize>> {
    let (_, value) = fields.iter().find(|(name, _)| *name == "id")?;
    (!value.is_empty() && value.bytes().all(|digit| digit.is_ascii_digit())).then(|| value.parse().ok())
}

/// What the header lines of a file said, before the glyphs are measured.
#[derive(Debug, Default)]
struct Header {
    size: Option<i32>,
    padding: Padding,
    line_height: Option<i32>,
    base: Option<i32>,
    page_size: (f32, f32),
}

impl BitmapFont {
    /// Reads the text of a `.fnt` file found at `file` inside the skin folder `root`.
    ///
    /// A line that is not a record, or a record missing a number it has to have, is skipped and
    /// counted ([`BitmapFont::skipped`], [`BitmapFont::warnings`]). A font with no `common` line,
    /// or with no glyph that has pixels, cannot be laid out at all and is an error, which is where
    /// the reference gives the font up as well.
    ///
    /// The pages are the `page` lines in the order they come, the first being page zero, which is
    /// how the library reads them: a line that numbers itself anything else is a page out of turn,
    /// and its place is kept with no image in it (`BitmapFontData.load`: "Page IDs must be indices
    /// starting at 0"). How many pages the `common` line says there are is not read.
    pub fn parse(text: &str, file: &Path, root: &Path) -> Result<BitmapFont, String> {
        let directory = file.parent().unwrap_or(root);
        let mut header = Header::default();
        let mut pages: Vec<Option<PathBuf>> = Vec::new();
        let mut glyphs: HashMap<u32, BitmapGlyph> = HashMap::new();
        let mut pairs: Vec<(i32, i32, i32)> = Vec::new();
        let mut skipped = Skipped::default();

        for (index, line) in text.lines().enumerate() {
            let row = index + 1;
            let line = line.trim_start_matches('\u{feff}');
            let tag = line.split_whitespace().next().unwrap_or_default();
            let parsed = fields(line);
            match tag {
                "" | "chars" | "kernings" => {}
                "info" => {
                    header.size = number(&parsed, "size");
                    match padding_of(&parsed) {
                        Some(padding) => header.padding = padding,
                        None => skipped.note(|| format!("line {row} gives no padding of four numbers, so none is taken")),
                    }
                }
                "common" => {
                    header.line_height = number(&parsed, "lineHeight");
                    header.base = number(&parsed, "base");
                    header.page_size = (number(&parsed, "scaleW").unwrap_or_default() as f32, number(&parsed, "scaleH").unwrap_or_default() as f32);
                }
                "page" => {
                    let id = pages.len();
                    let out_of_turn = page_number(&parsed).is_some_and(|number| number != Some(id));
                    let image = match page_file(line) {
                        _ if out_of_turn => {
                            skipped.note(|| format!("line {row} numbers itself another page than page {id}, which it is, so the page is left out"));
                            None
                        }
                        None => {
                            skipped.note(|| format!("line {row} is a page with no file, so page {id} is left out"));
                            None
                        }
                        Some(name) => contained(root, &directory.join(name.replace('\\', "/")))
                            .inspect_err(|error| skipped.note(|| format!("page {id} is left out: {error}")))
                            .ok(),
                    };
                    pages.push(image);
                }
                "char" => match glyph_of(&parsed) {
                    Some((id, glyph)) if id > 0 => {
                        glyphs.insert(glyph.id, glyph);
                    }
                    Some(_) => {}
                    None => skipped.note(|| format!("line {row} is a glyph missing one of its numbers")),
                },
                "kerning" => match (number(&parsed, "first"), number(&parsed, "second"), number(&parsed, "amount")) {
                    (Some(first), Some(second), Some(amount)) => pairs.push((first, second, amount)),
                    _ => skipped.note(|| format!("line {row} is a kerning pair missing one of its numbers")),
                },
                _ => skipped.note(|| format!("line {row} is not a font record")),
            }
        }

        let (Some(line_height), Some(base)) = (header.line_height, header.base) else {
            return Err("it has no common line giving a line height and a base".to_string());
        };
        let first_inked = glyphs.values().filter(|glyph| glyph.has_ink()).min_by_key(|glyph| glyph.id).copied();
        let Some(first_inked) = first_inked else {
            return Err("it has no glyph with any pixels".to_string());
        };

        let padding = header.padding;
        let space_id = u32::from(' ');
        let stand_in = glyphs.get(&u32::from(SPACE_STAND_IN)).copied().unwrap_or(first_inked);
        let space = glyphs.entry(space_id).or_insert(BitmapGlyph {
            id: space_id,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            xoffset: 0,
            yoffset: 0,
            xadvance: stand_in.xadvance,
            page: FIRST_PAGE,
        });
        if space.width == 0 {
            space.width = (padding.left + space.xadvance as f32 + padding.right) as i32;
            space.xoffset = (-padding.left) as i32;
        }
        let space_width = space.width as f32;

        let capital = CAP_CHARS.iter().find_map(|capital| glyphs.get(&u32::from(*capital)));
        let tallest = glyphs.values().filter(|glyph| glyph.has_ink()).map(|glyph| glyph.height).max().unwrap_or_default();
        let cap_height = capital.map_or(tallest, |capital| capital.height) as f32 - (padding.top + padding.bottom);

        let kept = |id: i32| u32::try_from(id).ok().filter(|id| *id <= LAST_KEPT_ID);
        let kernings = pairs
            .into_iter()
            .filter_map(|(first, second, amount)| Some(((kept(first)?, kept(second)?), i32::from(amount as i8))))
            .filter(|((first, _), _)| glyphs.contains_key(first))
            .collect();

        let original_size = header.size.filter(|size| *size != 0).unwrap_or(line_height) as f32;
        Ok(BitmapFont {
            original_size,
            line_height: line_height as f32,
            base: base as f32,
            page_size: header.page_size,
            padding,
            pages,
            glyphs,
            kernings,
            cap_height,
            space_width,
            skipped,
        })
    }

    /// The size the font was made at, which a text's own size is divided by to get its scale
    /// (`SkinTextBitmapSource.getOriginalSize`): the `info` line's `size`, or the line height when
    /// the file gives none.
    pub fn original_size(&self) -> f32 {
        self.original_size
    }

    /// The distance from one line's top to the next line's.
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// The distance from the top of a line to its baseline, as the file gives it.
    pub fn base(&self) -> f32 {
        self.base
    }

    /// The size the file says its pages are, which a distance field font's shadow is measured in.
    pub fn page_size(&self) -> (f32, f32) {
        self.page_size
    }

    /// The room left around every glyph.
    pub fn padding(&self) -> Padding {
        self.padding
    }

    /// The height of a capital with the padding taken off (`BitmapFontData.capHeight`).
    pub fn cap_height(&self) -> f32 {
        self.cap_height
    }

    /// How far below the capital line the top of a line of the file sits
    /// (`BitmapFontData.ascent`): a glyph whose `yoffset` is this much has its top on the capital
    /// line, which is the line a text's destination puts its top edge on.
    pub fn ascent(&self) -> f32 {
        self.base - self.cap_height
    }

    /// The width of a space, which is the narrowest width a line is ever wrapped in.
    pub fn space_width(&self) -> f32 {
        self.space_width
    }

    /// How many pages the font draws from.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The image file of one page, or `None` for a page the file does not name or names outside the
    /// skin's folder.
    pub fn page(&self, index: usize) -> Option<&Path> {
        self.pages.get(index).and_then(|page| page.as_deref())
    }

    /// How many glyphs the font has.
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// The glyph of one character, when the font has it.
    pub fn glyph(&self, ch: char) -> Option<&BitmapGlyph> {
        self.glyphs.get(&u32::from(ch))
    }

    /// How much closer or further apart `first` and `second` are set than their advances say.
    pub fn kerning(&self, first: char, second: char) -> i32 {
        self.kernings.get(&(u32::from(first), u32::from(second))).copied().unwrap_or_default()
    }

    /// The first lines that were skipped while the file was read, in words: no more than
    /// [`WARNINGS_KEPT`] of them, however many [`BitmapFont::skipped`] counts.
    pub fn warnings(&self) -> &[String] {
        &self.skipped.said
    }

    /// How many lines were skipped while the file was read.
    pub fn skipped(&self) -> usize {
        self.skipped.count
    }

    /// What the font itself draws `ch` with: its own glyph, or the glyph of the one character the
    /// reference takes for the same. `None` for a character the font has neither of, which the
    /// caller draws from another font.
    pub fn piece(&self, ch: char) -> Option<Piece> {
        if let Some(glyph) = self.glyph(ch) {
            return Some(Piece::Glyph { ch, glyph: *glyph, kerned: true });
        }
        let borrowed = equivalent_of(ch).and_then(|other| self.glyph(other))?;
        Some(Piece::Glyph { ch, glyph: *borrowed, kerned: false })
    }
}

/// One character of a line being laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Piece {
    /// A glyph of the font. `kerned` is off for a glyph borrowed from another character, which the
    /// reference copies without its kerning pairs.
    Glyph { ch: char, glyph: BitmapGlyph, kerned: bool },
    /// A character the font lacks, drawn from another font. It takes `advance` of the line, in the
    /// font's own units, and is as wide as that.
    Fallback { ch: char, advance: f32 },
}

impl Piece {
    /// The character this piece stands for.
    pub fn ch(&self) -> char {
        match self {
            Piece::Glyph { ch, .. } | Piece::Fallback { ch, .. } => *ch,
        }
    }

    fn xoffset(&self) -> f32 {
        match self {
            Piece::Glyph { glyph, .. } => glyph.xoffset as f32,
            Piece::Fallback { .. } => 0.0,
        }
    }

    fn width(&self) -> f32 {
        match self {
            Piece::Glyph { glyph, .. } => glyph.width as f32,
            Piece::Fallback { advance, .. } => *advance,
        }
    }

    fn xadvance(&self) -> f32 {
        match self {
            Piece::Glyph { glyph, .. } => glyph.xadvance as f32,
            Piece::Fallback { advance, .. } => *advance,
        }
    }
}

/// One line of a laid out text (`GlyphLayout.GlyphRun`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Run {
    /// Where the line starts, measured from the left edge of the width it was laid out in.
    pub x: f32,
    /// How many line heights below the first line this one sits.
    pub row: usize,
    pub pieces: Vec<Piece>,
    /// How far the pen moves before each piece, and after the last one: one more entry than there
    /// are pieces.
    advances: Vec<f32>,
    /// The width the line is aligned by.
    pub width: f32,
}

impl Run {
    /// Every piece of the line with where the pen stands for it, measured from [`Run::x`].
    pub fn placed(&self) -> impl Iterator<Item = (f32, &Piece)> {
        let mut pen = 0.0;
        self.pieces.iter().zip(&self.advances).map(move |(piece, advance)| {
            pen += advance;
            (pen, piece)
        })
    }
}

/// A laid out text (`GlyphLayout`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Laid {
    pub runs: Vec<Run>,
    /// The width of the longest line, which is what a text too wide for its destination is shrunk
    /// by.
    pub width: f32,
}

/// The scale a line is laid out at and the padding as that scale leaves it.
#[derive(Debug, Clone, Copy)]
struct Scaled {
    x: f32,
    pad_left: f32,
    pad_right: f32,
}

/// Where a line that overflowed at the piece before `start` is broken: after the last whitespace
/// before the word that overflowed, or nowhere (`BitmapFontData.getWrapIndex`).
fn wrap_index(pieces: &[Piece], start: usize) -> usize {
    let mut index = start.saturating_sub(1);
    while index >= 1 && is_whitespace(pieces[index].ch()) {
        index -= 1;
    }
    while index >= 1 {
        if is_whitespace(pieces[index].ch()) {
            return index + 1;
        }
        index -= 1;
    }
    0
}

/// Ends a line at the ink of the glyph it now ends on, less the right padding
/// (`GlyphLayout.adjustLastGlyph`). A line that ends on whitespace keeps that whitespace's advance.
fn adjust_last(run: &mut Run, scaled: Scaled) {
    let (Some(last), Some(end)) = (run.pieces.last(), run.advances.last_mut()) else {
        return;
    };
    if is_whitespace(last.ch()) {
        return;
    }
    let width = (last.xoffset() + last.width()) * scaled.x - scaled.pad_right;
    run.width += width - *end;
    *end = width;
}

/// Breaks `run` before the piece at `at`, leaving the pieces before it and answering the rest as
/// the line that follows (`GlyphLayout.wrap`). `measured` is how many advances the run's width
/// already counts.
fn wrap(run: &mut Run, at: usize, measured: usize, scaled: Scaled) -> Run {
    let mut counted = measured;
    while counted < at {
        run.width += run.advances[counted];
        counted += 1;
    }
    while counted > at + 1 {
        counted -= 1;
        run.width -= run.advances[counted];
    }
    let mut rest = Run::default();
    if at < run.pieces.len() {
        rest.pieces = run.pieces.split_off(at);
        let tail = run.advances.split_off(at + 1);
        rest.advances = Vec::with_capacity(tail.len() + 1);
        rest.advances.push(-rest.pieces[0].xoffset() * scaled.x - scaled.pad_left);
        rest.advances.extend(tail);
    }
    adjust_last(run, scaled);
    rest
}

/// Cuts `run` down to the pieces whose advance still ends inside `target`, with nothing put in
/// place of the rest (`GlyphLayout.truncate` with an empty replacement). A line whose second piece
/// already starts past the width keeps nothing at all.
fn truncate(run: &mut Run, target: f32, scaled: Scaled) {
    let mut count = 0;
    let mut reach = 0.0;
    while count < run.advances.len() {
        let advance = run.advances[count];
        reach += advance;
        if reach > target {
            run.width = reach - advance;
            break;
        }
        count += 1;
    }
    if count > 1 {
        run.pieces.truncate(count - 1);
        run.advances.truncate(count);
        adjust_last(run, scaled);
    } else {
        run.pieces.clear();
        run.advances.clear();
    }
}

impl BitmapFont {
    /// One line's pieces and the advance before each (`BitmapFontData.getGlyphs` with loose
    /// bounds): the first piece's origin is where the line starts, each later one follows the
    /// advance and the kerning of the one before it, and the line ends at the last one's advance.
    ///
    /// An advance and its kerning are added as the whole numbers the file gave, which in the
    /// library wrap around when a file gives ones too large to add, and so do here.
    fn run_of(&self, line: &str, resolve: &mut dyn FnMut(char) -> Option<Piece>, scale_x: f32) -> Run {
        let mut run = Run::default();
        let mut before: Option<Piece> = None;
        for ch in line.chars() {
            let Some(piece) = resolve(ch) else {
                continue;
            };
            run.advances.push(match before {
                None => 0.0,
                Some(Piece::Glyph { ch: first, glyph, kerned: true }) => glyph.xadvance.wrapping_add(self.kerning(first, ch)) as f32 * scale_x,
                Some(other) => other.xadvance() * scale_x,
            });
            run.pieces.push(piece);
            before = Some(piece);
        }
        if let Some(last) = before {
            run.advances.push(last.xadvance() * scale_x);
        }
        run
    }

    /// Lays `text` out in `target` pixels at `scale` (across, down), asking `resolve` what each
    /// character is drawn with (`GlyphLayout.setText`). A character `resolve` answers nothing for
    /// takes no room.
    ///
    /// [`BlockFit::Shrink`] is laid out like [`BlockFit::Overflow`] here: squeezing is a second
    /// layout at a narrower scale, which [`BitmapFont::layout_fitted`] does.
    pub fn layout(&self, text: &str, resolve: &mut dyn FnMut(char) -> Option<Piece>, target: f32, align: BlockAlign, fit: BlockFit, scale: (f32, f32)) -> Laid {
        let scaled = Scaled { x: scale.0, pad_left: self.padding.left * scale.1, pad_right: self.padding.right * scale.1 };
        let cut = fit == BlockFit::Truncate;
        let wrapping = cut || (fit == BlockFit::Wrap && target > self.space_width * scaled.x);

        let mut runs: Vec<Run> = Vec::new();
        let mut width = 0.0f32;
        let mut reach = 0.0f32;
        let mut row = 0;
        let mut lines = text.split('\n').peekable();
        while let Some(line) = lines.next() {
            let mut run = Run { row, ..self.run_of(line, resolve, scaled.x) };
            let mut stopped = false;
            let mut index = 0;
            while index < run.advances.len() {
                let advance = run.advances[index];
                reach += advance;
                let overflows = wrapping && reach > target && index > 1 && {
                    let last = run.pieces[index - 1];
                    reach - advance + (last.xoffset() + last.width()) * scaled.x - WRAP_SLACK > target
                };
                if !overflows {
                    run.width += advance;
                    index += 1;
                    continue;
                }
                if cut {
                    truncate(&mut run, target, scaled);
                    reach = run.width;
                    stopped = true;
                    break;
                }
                let found = wrap_index(&run.pieces, index);
                let at = if found == 0 || found >= run.pieces.len() { index - 1 } else { found };
                let rest = wrap(&mut run, at, index, scaled);
                width = width.max(run.width);
                runs.push(std::mem::replace(&mut run, rest));
                row += 1;
                run.row = row;
                reach = 0.0;
                index = 0;
            }
            if !run.pieces.is_empty() || stopped {
                runs.push(run);
            }
            if stopped {
                break;
            }
            if lines.peek().is_some() {
                width = width.max(reach);
                reach = 0.0;
                row += 1;
            }
        }
        width = width.max(reach);

        for run in &mut runs {
            run.x += slack(align, target, run.width);
        }
        Laid { runs, width }
    }

    /// Lays `text` out at `scale` and, for [`BlockFit::Shrink`], lays it out again squeezed
    /// sideways when its longest line is wider than `target` (`SkinTextBitmap.updateLayout`).
    /// Answers the layout and the horizontal scale it was made at.
    pub fn layout_fitted(
        &self,
        text: &str,
        resolve: &mut dyn FnMut(char) -> Option<Piece>,
        target: f32,
        align: BlockAlign,
        fit: BlockFit,
        scale: f32,
    ) -> (Laid, f32) {
        let laid = self.layout(text, resolve, target, align, fit, (scale, scale));
        if fit != BlockFit::Shrink || laid.width <= target {
            return (laid, scale);
        }
        let squeezed = scale * target / laid.width;
        if !(squeezed.is_finite() && squeezed > 0.0) {
            return (Laid::default(), scale);
        }
        (self.layout(text, resolve, target, align, fit, (squeezed, scale)), squeezed)
    }
}

/// A fingerprint of a font file's bytes, which tells a file that was written over from the one
/// that was read before.
fn content_digest(bytes: &[u8]) -> (usize, u64) {
    let mut hasher = std::hash::DefaultHasher::new();
    hasher.write(bytes);
    (bytes.len(), hasher.finish())
}

/// One font file as it was last read: the fingerprint of its bytes and what they parsed into.
type CachedFont = ((usize, u64), Arc<BitmapFont>);

/// What a parsed font is kept under: the skin folder it was read inside and the path of its file.
/// The folder is part of it because which of a font's pages are inside the folder is part of what
/// was parsed.
type FontKey = (PathBuf, PathBuf);

/// The bitmap fonts read so far (`BitmapFontCache`). A skin's screens are built again whenever they
/// are entered, and a published font is a megabyte of text, so the parse is kept; it is redone only
/// for a file whose bytes have changed.
///
/// What is kept is every font a screen still draws with, and beside those the fonts of the skin
/// folder a font was last parsed for: reading a font of another folder, which is what choosing
/// another skin comes to, lets go of the ones no screen holds any more.
static FONTS: LazyLock<Mutex<HashMap<FontKey, CachedFont>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The font in `bytes`, the contents of the `.fnt` file at `path` inside the skin folder `root`.
///
/// The same file is parsed once however many screens name it. An error is the one line that says
/// why the font cannot be used.
pub fn load(path: &Path, root: &Path, bytes: &[u8]) -> Result<Arc<BitmapFont>, String> {
    let digest = content_digest(bytes);
    let key = (root.to_path_buf(), path.to_path_buf());
    let mut fonts = locked(&FONTS);
    if let Some((known, font)) = fonts.get(&key)
        && *known == digest
    {
        return Ok(Arc::clone(font));
    }
    let font = Arc::new(BitmapFont::parse(&String::from_utf8_lossy(bytes), path, root)?);
    fonts.retain(|(folder, _), (_, kept)| folder == root || Arc::strong_count(kept) > 1);
    fonts.insert(key, (digest, Arc::clone(&font)));
    Ok(font)
}

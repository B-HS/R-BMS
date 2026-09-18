//! The character definition a play document's `pmchara` object draws from.
//!
//! A `pmchara` object names no image source of its own. Its `src` points at a `.chp` text file, or
//! at a directory holding one, and that file names the images, cuts them into sprite rectangles and
//! lists one animation row per motion. This module reads that file and hands back a resolved
//! [`CharaDef`]; deciding which motion plays and where each frame lands on screen belongs to the
//! renderer.
//!
//! The format is the reference implementation's `PomyuCharaLoader`: tab-separated directives, a
//! `#xx` rectangle table addressed by two base-36 characters, and animation rows whose columns are
//! runs of two-character fields. The file is text in MS932, which is why this module decodes rather
//! than reading UTF-8.

use std::path::{Path, PathBuf};

use crate::SkinError;
use crate::loader::LoadedSkin;
use crate::resolve::{contained, pattern_for};

/// How many image slots a definition carries: each of the four kinds in its first- and
/// second-player colour.
pub const IMAGE_SLOT_COUNT: usize = 8;

/// The sheet `#Pattern` and `#Layer` rows are cut from.
pub const SLOT_CHAR_BMP: usize = 0;

/// The same sheet in the second player's colour.
pub const SLOT_CHAR_BMP_2P: usize = 1;

/// The sheet `#Texture` rows are cut from.
pub const SLOT_CHAR_TEX: usize = 2;

/// The same sheet in the second player's colour.
pub const SLOT_CHAR_TEX_2P: usize = 3;

/// The portrait sheet the face types are cut from.
pub const SLOT_CHAR_FACE: usize = 4;

/// The same portrait in the second player's colour.
pub const SLOT_CHAR_FACE_2P: usize = 5;

/// The browser icon.
pub const SLOT_SELECT_CG: usize = 6;

/// The same icon in the second player's colour.
pub const SLOT_SELECT_CG_2P: usize = 7;

/// How many rectangles the `#xx` table holds: every two-character base-36 address.
pub const RECT_TABLE_LEN: usize = 1296;

/// How many motions a definition may give a frame time and a loop point for.
pub const MOTION_COUNT: usize = 20;

/// Milliseconds one frame lasts when the file names no time, and the time a non-positive one is
/// replaced with.
pub const DEFAULT_FRAME_MS: i32 = 100;

/// The shortest frame time a file may state. Anything below it reads as [`DEFAULT_FRAME_MS`].
const MIN_FRAME_MS: i32 = 1;

/// The loop point of a motion whose every frame repeats, which is what a file that names none has.
pub const NO_LOOP: i32 = -1;

/// The rectangle the upper-body portrait takes when the file overrides nothing.
pub const DEFAULT_FACE_UPPER: CharaRect = CharaRect { x: 0, y: 0, w: 256, h: 256 };

/// The rectangle the full portrait takes when the file overrides nothing.
pub const DEFAULT_FACE_ALL: CharaRect = CharaRect { x: 320, y: 0, w: 320, h: 480 };

/// Characters one field of an animation column takes.
const FIELD_WIDTH: usize = 2;

/// The field that stands for "interpolate from the last stated value to the next one".
const INTERPOLATED_FIELD: &str = "--";

/// The radix a rectangle address is written in.
const ADDRESS_RADIX: usize = 36;

/// The largest value an alpha or angle field states.
const FIELD_MAX: i32 = 255;

/// The alpha a frame takes when its column says nothing.
const OPAQUE: i32 = 255;

/// The angle units a full turn is written in, which is not the 360 the engine turns by.
const ANGLE_UNITS: i32 = 256;

/// Degrees in a full turn.
const FULL_TURN_DEG: i32 = 360;

/// How many numbers a rectangle row states.
const RECT_FIELDS: usize = 4;

/// How many characters a `#xx` directive is: the hash and the two address characters.
const ADDRESS_DIRECTIVE_LEN: usize = 3;

/// The character a trailing comment starts with, and the pair that starts one mid-field.
const COMMENT_PREFIX: char = '/';
const COMMENT_MARK: &str = "//";

/// One rectangle of the `#xx` table, in the pixels of whichever sheet reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharaRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Which sheet an animation row draws from, and in what order rows reach the screen.
///
/// The reference draws every `#Pattern` row, then every `#Texture` row, then every `#Layer` row, so
/// the order of this enum is the draw order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharaSheet {
    /// `#Pattern`, also spelled `#Patern`, cut from [`SLOT_CHAR_BMP`].
    Pattern,
    /// `#Texture`, cut from [`SLOT_CHAR_TEX`].
    Texture,
    /// `#Layer`, cut from [`SLOT_CHAR_BMP`] like a pattern but drawn over everything.
    Layer,
}

impl CharaSheet {
    /// The image slot this sheet reads, in the first- or second-player colour.
    pub const fn slot(self, second_player: bool) -> usize {
        let base = match self {
            Self::Pattern | Self::Layer => SLOT_CHAR_BMP,
            Self::Texture => SLOT_CHAR_TEX,
        };
        if second_player { base + 1 } else { base }
    }
}

/// One frame of one animation row, with everything its columns stated already resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharaFrame {
    /// Which `#xx` rectangle of the sheet this frame shows.
    pub source: usize,
    /// Where it lands inside the definition's own [`CharaDef::size`] box.
    pub destination: CharaRect,
    /// The frame's own alpha, over the destination the document gave the object.
    pub alpha: u8,
    /// The frame's own rotation, already converted from the file's 256ths of a turn.
    pub angle_deg: i32,
}

/// One animation row: a motion number and the frames it plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharaRow {
    pub sheet: CharaSheet,
    pub motion: i32,
    pub frames: Vec<CharaFrame>,
}

/// A character definition, read and resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharaDef {
    /// The image each slot names, relative to the definition file, or `None` for a slot the file
    /// leaves empty.
    pub images: [Option<PathBuf>; IMAGE_SLOT_COUNT],
    /// The box every frame's destination is stated in.
    pub size: (i32, i32),
    /// Milliseconds one frame of each motion lasts, already filled in from `#Anime`.
    pub frame_ms: [i32; MOTION_COUNT],
    /// The frame each motion repeats from, or [`NO_LOOP`] when the whole motion repeats.
    pub loop_frame: [i32; MOTION_COUNT],
    pub face_upper: CharaRect,
    pub face_all: CharaRect,
    /// The `#xx` rectangles, in draw order per sheet.
    pub rects: Vec<CharaRect>,
    pub rows: Vec<CharaRow>,
}

impl CharaDef {
    /// One `#xx` rectangle, or an empty one for an address the file never stated.
    pub fn rect(&self, address: usize) -> CharaRect {
        self.rects.get(address).copied().unwrap_or_default()
    }

    /// Whether any row reads the texture sheet, which is what makes `#CharTex` mandatory.
    pub fn uses_texture(&self) -> bool {
        self.rows.iter().any(|row| row.sheet == CharaSheet::Texture)
    }

    /// Which colour this definition can actually be drawn in.
    ///
    /// The second-player colour needs its own pattern sheet, and its own texture sheet as well when
    /// any row reads one; a definition missing either falls back to the first player's colour
    /// (`PomyuCharaLoader.load`).
    pub fn second_player_colour(&self, asked: bool) -> bool {
        asked && self.images[SLOT_CHAR_BMP_2P].is_some() && (!self.uses_texture() || self.images[SLOT_CHAR_TEX_2P].is_some())
    }
}

/// Whether a path names a definition file.
fn is_chp(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("chp"))
}

/// The definition file a `src` names, following `PomyuCharaLoader.findChpFile`.
///
/// A `src` that is a definition file is taken as it stands; anything else is treated as a directory
/// and its first definition file wins. Directory entries are sorted so two machines pick the same
/// one, which the reference leaves to the filesystem.
pub fn find_chp_file(src: &Path) -> Option<PathBuf> {
    if is_chp(src) && src.is_file() {
        return Some(src.to_path_buf());
    }
    let directory = if is_chp(src) { src.parent()? } else { src };
    let mut names: Vec<PathBuf> = std::fs::read_dir(directory).ok()?.flatten().map(|entry| entry.path()).filter(|path| is_chp(path)).collect();
    names.sort();
    names.into_iter().next()
}

/// Reads and resolves the definition `src` names, with every image path checked against `root`.
///
/// `root` is the skin root, and nothing outside it may be opened: a definition that names
/// `../../secret.png` is refused the same way a document's own source would be.
pub fn read_chara(root: &Path, src: &Path) -> Result<CharaDef, SkinError> {
    let file =
        find_chp_file(src).ok_or_else(|| SkinError::Read(std::io::Error::new(std::io::ErrorKind::NotFound, format!("no .chp under {}", src.display()))))?;
    let file = contained(root, &file)?;
    let bytes = std::fs::read(&file).map_err(SkinError::Read)?;
    let (text, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
    let mut def = parse_chara(&text);
    if def.images[SLOT_CHAR_BMP].is_none() {
        return Err(SkinError::Read(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{} names no #CharBMP", file.display()))));
    }
    let directory = file.parent().unwrap_or(root).to_path_buf();
    for slot in def.images.iter_mut() {
        let Some(relative) = slot.take() else {
            continue;
        };
        *slot = contained(root, &directory.join(relative)).ok();
    }
    Ok(def)
}

/// The definition file one `src` names: a source id the document declared, or a path relative to the
/// document itself.
///
/// A `pmchara` object's `src` is not one of the document's image sources, so it is resolved here
/// rather than through the source table the renderer already holds.
pub fn chara_source_path(skin: &LoadedSkin, src: &str) -> Option<PathBuf> {
    if let Some(path) = skin.sources.get(src) {
        return Some(path.clone());
    }
    let directory = skin.path.parent().unwrap_or(&skin.root).to_path_buf();
    contained(&skin.root, Path::new(&pattern_for(&directory, src))).ok()
}

/// Every image file this document's characters name, without duplicates.
///
/// A host that reads a document's files before it builds a screen has to read these too: a `.chp`
/// definition names its own sheets, so none of them is in the document's `source` list and none of
/// them would otherwise be prepared.
pub fn chara_image_paths(skin: &LoadedSkin) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for object in &skin.def.pmchara {
        if seen.contains(&object.src) {
            continue;
        }
        seen.push(object.src.clone());
        let Some(path) = chara_source_path(skin, &object.src) else {
            continue;
        };
        let Ok(def) = read_chara(&skin.root, &path) else {
            continue;
        };
        for file in def.images.into_iter().flatten() {
            if !files.contains(&file) {
                files.push(file);
            }
        }
    }
    files
}

/// The fields of one line, with empty fields dropped and comments cut off
/// (`PomyuCharaLoader.PMparseStr`).
fn fields(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for field in line.split('\t') {
        if field.is_empty() {
            continue;
        }
        if field.starts_with(COMMENT_PREFIX) {
            break;
        }
        if let Some(cut) = field.find(COMMENT_MARK) {
            out.push(&field[..cut]);
            break;
        }
        out.push(field);
    }
    out
}

/// One whole number, read the way the reference reads one: every character that is not a digit or a
/// minus sign is dropped first.
fn parse_int(text: &str) -> Option<i32> {
    let digits: String = text.chars().filter(|character| character.is_ascii_digit() || *character == '-').collect();
    digits.parse().ok()
}

/// The value one character carries in the rectangle table's radix, with anything else reading as
/// zero (`PomyuCharaLoader.PMparseInt`).
fn address_digit(character: char) -> usize {
    match character {
        '0'..='9' => character as usize - '0' as usize,
        'a'..='z' => character as usize - 'a' as usize + 10,
        'A'..='Z' => character as usize - 'A' as usize + 10,
        _ => 0,
    }
}

/// The rectangle address a two-character field states, always inside the table.
fn parse_address(field: &str) -> usize {
    let mut characters = field.chars();
    let high = characters.next().map_or(0, address_digit);
    let low = characters.next().map_or(0, address_digit);
    high * ADDRESS_RADIX + low
}

/// The value a two-character alpha or angle field states, or `None` when it is not one.
fn parse_field_value(field: &str) -> Option<i32> {
    let digits: String = field.chars().filter(|character| character.is_ascii_hexdigit()).collect();
    if digits.len() != field.len() {
        return None;
    }
    i32::from_str_radix(&digits, 16).ok().filter(|value| (0..=FIELD_MAX).contains(value))
}

/// One animation column, split into its two-character fields.
fn columns(text: &str) -> Vec<&str> {
    (0..text.len() / FIELD_WIDTH).map(|index| &text[index * FIELD_WIDTH..index * FIELD_WIDTH + FIELD_WIDTH]).collect()
}

/// Drops from a column every character the reference drops before reading it.
fn clean_column(text: &str) -> String {
    text.chars().filter(|character| character.is_ascii_alphanumeric() || *character == '-').collect()
}

/// How long the run of interpolated fields starting at `index` is.
fn run_length(fields: &[&str], index: usize) -> usize {
    fields[index..].iter().take_while(|field| **field == INTERPOLATED_FIELD).count()
}

/// The destination rectangle of every frame, from the row's second column.
///
/// A column that states nothing puts every frame over the whole [`CharaDef::size`] box, which is
/// what a row with no destination column means. A run of [`INTERPOLATED_FIELD`] fields is a
/// straight line from the last stated rectangle to the next one, in as many steps as the run is
/// long plus one.
fn destinations(column: &str, count: usize, rects: &[CharaRect], size: (i32, i32)) -> Vec<CharaRect> {
    let whole = CharaRect { x: 0, y: 0, w: size.0, h: size.1 };
    let fields = columns(column);
    if fields.is_empty() {
        return vec![whole; count];
    }
    let mut out = vec![whole; fields.len()];
    let mut start = whole;
    let mut end = whole;
    let mut index = 0;
    while index < fields.len() {
        if fields[index] != INTERPOLATED_FIELD {
            start = rects.get(parse_address(fields[index])).copied().unwrap_or_default();
            out[index] = start;
            index += 1;
            continue;
        }
        let run = run_length(&fields, index);
        if let Some(next) = fields.get(index + run) {
            end = rects.get(parse_address(next)).copied().unwrap_or_default();
        }
        let divisions = run as i32 + 1;
        for step in 0..run {
            let rate = step as i32 + 1;
            out[index + step] = CharaRect {
                x: start.x + (end.x - start.x) * rate / divisions,
                y: start.y + (end.y - start.y) * rate / divisions,
                w: start.w + (end.w - start.w) * rate / divisions,
                h: start.h + (end.h - start.h) * rate / divisions,
            };
        }
        index += run;
    }
    out
}

/// One scalar column -- alpha or angle -- for every frame, with the same interpolation rule and the
/// same default for a field that states nothing readable.
fn scalars(column: &str, count: usize, default: i32, scale_to_degrees: bool) -> Vec<i32> {
    let fields = columns(column);
    if fields.is_empty() {
        return vec![default; count];
    }
    let scale = |value: i32| if scale_to_degrees { (value * FULL_TURN_DEG + ANGLE_UNITS / 2) / ANGLE_UNITS } else { value };
    let mut out = vec![default; fields.len()];
    let mut start = 0;
    let mut end = 0;
    let mut index = 0;
    while index < fields.len() {
        if fields[index] != INTERPOLATED_FIELD {
            if let Some(value) = parse_field_value(fields[index]) {
                start = scale(value);
                out[index] = start;
            }
            index += 1;
            continue;
        }
        let run = run_length(&fields, index);
        if let Some(value) = fields.get(index + run).and_then(|field| parse_field_value(field)) {
            end = scale(value);
        }
        let divisions = run as i32 + 1;
        for step in 0..run {
            out[index + step] = start + (end - start) * (step as i32 + 1) / divisions;
        }
        index += run;
    }
    out
}

/// What one pass over the file gathers before the rows are resolved.
#[derive(Debug, Default)]
struct RawChara {
    images: [Option<PathBuf>; IMAGE_SLOT_COUNT],
    size: (i32, i32),
    anime: Option<i32>,
    frame_ms: [Option<i32>; MOTION_COUNT],
    loop_frame: [i32; MOTION_COUNT],
    face_upper: CharaRect,
    face_all: CharaRect,
    rects: Vec<CharaRect>,
    rows: Vec<(CharaSheet, String)>,
}

/// The image slot one directive fills, or `None` when the directive names no image.
fn image_slot(directive: &str) -> Option<usize> {
    let slot = match directive.to_ascii_lowercase().as_str() {
        "#charbmp" => SLOT_CHAR_BMP,
        "#charbmp2p" => SLOT_CHAR_BMP_2P,
        "#chartex" => SLOT_CHAR_TEX,
        "#chartex2p" => SLOT_CHAR_TEX_2P,
        "#charface" => SLOT_CHAR_FACE,
        "#charface2p" => SLOT_CHAR_FACE_2P,
        "#selectcg" => SLOT_SELECT_CG,
        "#selectcg2p" => SLOT_SELECT_CG_2P,
        _ => return None,
    };
    Some(slot)
}

/// The sheet one directive opens an animation row on, or `None` when it opens none.
fn row_sheet(directive: &str) -> Option<CharaSheet> {
    match directive.to_ascii_lowercase().as_str() {
        "#pattern" | "#patern" => Some(CharaSheet::Pattern),
        "#texture" => Some(CharaSheet::Texture),
        "#layer" => Some(CharaSheet::Layer),
        _ => None,
    }
}

/// Reads one rectangle row, whose directive is the hash and a two-character address.
fn read_rect_row(raw: &mut RawChara, directive: &str, data: &[&str]) -> bool {
    if directive.chars().count() != ADDRESS_DIRECTIVE_LEN || data.len() <= RECT_FIELDS {
        return false;
    }
    let address = parse_address(&directive[1..]);
    let Some(values) = data[1..=RECT_FIELDS].iter().map(|field| parse_int(field)).collect::<Option<Vec<i32>>>() else {
        return true;
    };
    raw.rects[address] = CharaRect { x: values[0], y: values[1], w: values[2], h: values[3] };
    true
}

/// Reads one rectangle override -- a portrait's own rectangle -- into `target`.
fn read_rect_override(target: &mut CharaRect, data: &[&str]) {
    if data.len() <= RECT_FIELDS {
        return;
    }
    let Some(values) = data[1..=RECT_FIELDS].iter().map(|field| parse_int(field)).collect::<Option<Vec<i32>>>() else {
        return;
    };
    *target = CharaRect { x: values[0], y: values[1], w: values[2], h: values[3] };
}

/// The motion a per-motion directive addresses and the number it states, when both are readable and
/// the motion is one the tables hold.
fn motion_value(data: &[&str]) -> Option<(usize, i32)> {
    if data.len() <= 2 {
        return None;
    }
    let motion = usize::try_from(parse_int(data[1])?).ok()?;
    let value = parse_int(data[2])?;
    (motion < MOTION_COUNT).then_some((motion, value))
}

/// Reads one line into the gathered file.
fn read_line(raw: &mut RawChara, line: &str) {
    if !line.starts_with('#') {
        return;
    }
    let raw_fields: Vec<&str> = line.split('\t').collect();
    if raw_fields.len() < 2 {
        return;
    }
    let data = fields(line);
    let Some(directive) = data.first().copied() else {
        return;
    };
    if let Some(slot) = image_slot(directive) {
        if data.len() > 1 {
            raw.images[slot] = Some(PathBuf::from(data[1].replace('\\', "/")));
        }
        return;
    }
    if let Some(sheet) = row_sheet(directive) {
        raw.rows.push((sheet, line.to_owned()));
        return;
    }
    match directive.to_ascii_lowercase().as_str() {
        "#flame" | "#frame" => {
            if let Some((motion, value)) = motion_value(&data) {
                raw.frame_ms[motion] = Some(value);
            }
        }
        "#anime" => {
            if data.len() > 1 {
                raw.anime = parse_int(data[1]);
            }
        }
        "#size" => {
            if data.len() > 2
                && let (Some(width), Some(height)) = (parse_int(data[1]), parse_int(data[2]))
            {
                raw.size = (width, height);
            }
        }
        "#loop" => {
            if let Some((motion, value)) = motion_value(&data) {
                raw.loop_frame[motion] = value;
            }
        }
        "#charfaceuppersize" => read_rect_override(&mut raw.face_upper, &data),
        "#charfaceallsize" => read_rect_override(&mut raw.face_all, &data),
        _ => {
            read_rect_row(raw, directive, &data);
        }
    }
}

/// Resolves one gathered animation row into its frames.
fn resolve_row(raw: &RawChara, sheet: CharaSheet, line: &str) -> Option<CharaRow> {
    let data = fields(line);
    let motion = parse_int(data.get(1)?)?;
    let mut column = [String::new(), String::new(), String::new(), String::new()];
    for (index, slot) in column.iter_mut().enumerate() {
        if let Some(text) = data.get(index + 2) {
            *slot = clean_column(text);
        }
    }
    let count = column[0].len() / FIELD_WIDTH;
    if count == 0 || !column[0].len().is_multiple_of(FIELD_WIDTH) {
        return None;
    }
    if column[1..].iter().any(|text| !text.is_empty() && text.len() != column[0].len()) {
        return None;
    }
    let destination = destinations(&column[1], count, &raw.rects, raw.size);
    let alpha = scalars(&column[2], count, OPAQUE, false);
    let angle = scalars(&column[3], count, 0, true);
    let frames = columns(&column[0])
        .into_iter()
        .enumerate()
        .map(|(index, field)| CharaFrame {
            source: parse_address(field),
            destination: destination[index],
            alpha: alpha[index].clamp(0, FIELD_MAX) as u8,
            angle_deg: angle[index],
        })
        .collect();
    Some(CharaRow { sheet, motion, frames })
}

/// Reads a definition's text. Nothing here touches the filesystem, so a caller with the text in
/// hand -- a test, or a bundle check -- reads it the same way a load does.
pub fn parse_chara(text: &str) -> CharaDef {
    let mut raw =
        RawChara { rects: vec![CharaRect::default(); RECT_TABLE_LEN], face_upper: DEFAULT_FACE_UPPER, face_all: DEFAULT_FACE_ALL, ..RawChara::default() };
    raw.loop_frame = [NO_LOOP; MOTION_COUNT];
    for line in text.lines() {
        read_line(&mut raw, line.trim_end_matches('\r'));
    }

    let anime = raw.anime.unwrap_or(DEFAULT_FRAME_MS);
    let mut frame_ms = [DEFAULT_FRAME_MS; MOTION_COUNT];
    for (slot, stated) in frame_ms.iter_mut().zip(raw.frame_ms) {
        let value = stated.unwrap_or(anime);
        *slot = if value < MIN_FRAME_MS { DEFAULT_FRAME_MS } else { value };
    }

    let mut rows: Vec<CharaRow> = Vec::with_capacity(raw.rows.len());
    for sheet in [CharaSheet::Pattern, CharaSheet::Texture, CharaSheet::Layer] {
        for (row_sheet, line) in raw.rows.iter().filter(|(row_sheet, _)| *row_sheet == sheet) {
            if let Some(row) = resolve_row(&raw, *row_sheet, line) {
                rows.push(row);
            }
        }
    }

    CharaDef {
        images: raw.images,
        size: raw.size,
        frame_ms,
        loop_frame: raw.loop_frame,
        face_upper: raw.face_upper,
        face_all: raw.face_all,
        rects: raw.rects,
        rows,
    }
}

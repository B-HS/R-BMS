//! Reading a `.lr2crs` course list.
//!
//! The file is XML: a `<courselist>` of `<course>` elements, each with a `<title>`, a `<line>` (the
//! number of keys of its charts), a `<hash>` and a `<type>`. Only those four tags are read, with
//! `str::find`, and anything else in the file is passed over. The hash is 32 characters the format
//! puts in front, then the 32-character MD5 of every chart in play order.
//!
//! The file is read as it is written and no entity is decoded: a title that has one fails, because
//! the title goes into the id of the course and a wrong title would be a different id.

use encoding_rs::SHIFT_JIS;

/// Characters of a `<hash>` before the first chart MD5.
pub const HASH_PREFIX_LENGTH: usize = 32;

/// Characters of one chart MD5.
const MD5_LENGTH: usize = 32;

const COURSE_OPEN: &str = "<course>";
const COURSE_CLOSE: &str = "</course>";

/// One course of a course list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lr2crsCourse {
    /// As written, spaces at either end included.
    pub title: String,
    pub line: u32,
    /// Lower case; a prefix of [`HASH_PREFIX_LENGTH`] characters and one MD5 per chart.
    pub hash: String,
    pub kind: u32,
}

impl Lr2crsCourse {
    pub fn hash_prefix(&self) -> &str {
        &self.hash[..HASH_PREFIX_LENGTH]
    }

    /// The MD5 of each chart, in play order.
    pub fn md5s(&self) -> Vec<String> {
        self.hash.as_bytes()[HASH_PREFIX_LENGTH..].chunks(MD5_LENGTH).map(|chunk| String::from_utf8_lossy(chunk).into_owned()).collect()
    }
}

/// The text of a course list file, which is Shift_JIS.
pub fn decode(bytes: &[u8]) -> Result<String, String> {
    SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(|text| text.into_owned())
        .ok_or_else(|| "the course list is not valid Shift_JIS".to_string())
}

fn tag_text<'a>(block: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let from = block.find(&open)? + open.len();
    let to = from + block[from..].find(&close)?;
    Some(&block[from..to])
}

fn number_of(block: &str, tag: &str, position: usize) -> Result<u32, String> {
    let text = tag_text(block, tag).ok_or_else(|| format!("course {position} has no <{tag}>"))?;
    text.trim().parse().map_err(|_| format!("course {position} has <{tag}> {text:?}, which is not a whole number"))
}

fn course_of(block: &str, position: usize) -> Result<Lr2crsCourse, String> {
    let title = tag_text(block, "title").ok_or_else(|| format!("course {position} has no <title>"))?;
    if title.contains(['&', '<']) {
        return Err(format!("the title of course {position} ({title:?}) holds markup, which is not read"));
    }
    let hash = tag_text(block, "hash").ok_or_else(|| format!("course {position} has no <hash>"))?.trim().to_ascii_lowercase();
    let is_whole = hash.len() > HASH_PREFIX_LENGTH && (hash.len() - HASH_PREFIX_LENGTH).is_multiple_of(MD5_LENGTH);
    if !is_whole || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "the hash of course {position} ({title:?}) is not {HASH_PREFIX_LENGTH} characters and then {MD5_LENGTH} per chart, in hexadecimal"
        ));
    }
    Ok(Lr2crsCourse { title: title.to_string(), line: number_of(block, "line", position)?, hash, kind: number_of(block, "type", position)? })
}

/// Every course of the list, in the order the file has them.
pub fn parse(text: &str) -> Result<Vec<Lr2crsCourse>, String> {
    let mut courses = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(COURSE_OPEN) {
        let after = &rest[start + COURSE_OPEN.len()..];
        let position = courses.len() + 1;
        let end = after.find(COURSE_CLOSE).ok_or_else(|| format!("course {position} is never closed"))?;
        courses.push(course_of(&after[..end], position)?);
        rest = &after[end + COURSE_CLOSE.len()..];
    }
    if courses.is_empty() {
        return Err("the course list holds no course".to_string());
    }
    Ok(courses)
}

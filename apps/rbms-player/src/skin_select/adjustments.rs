//! A pack's own adjustments: the choices somebody made in the pack's documents, written into the
//! pack so it is drawn that way wherever its folder is taken.
//!
//! The settings file stores what a player chose inside a document under the path the document was
//! found at, and by the numbers the document gives its options and offsets. Neither travels: a path
//! is this machine's, and a number is whatever the document's author happens to use this version.
//! So the file written into an exported pack ([`ADJUSTMENTS_FILE`]) names a document by its name
//! inside the pack, an option and the item it is on by their names, a file slot by its name and the
//! chosen file by its path inside the pack, and an offset by its name with a figure for each axis.
//!
//! It is read when a pack is ([`read`]), and what it says is laid over each document as that pack's
//! own defaults ([`given_by`]): a choice the player stored for the same row wins, and dropping that
//! choice comes back to what the pack gave. A line that names something the document does not
//! declare -- an option, an item, a file, an offset -- is passed over, so a file written for one
//! version of a pack does no harm to the next.
//!
//! The file is a RON document like the settings file, under a name and an extension no skin loader
//! takes for a skin, and carries the version of its own layout.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use rbms_config::SkinCustomisation;
use rbms_skin::dst::SkinOffset;
use rbms_skin::loader::{SkinHeader, selected_option};
use rbms_skin::resolve::{CustomFile, RANDOM_SELECTION, pattern_for};
use serde::{Deserialize, Serialize};

use super::OffsetAxis;

/// The file a pack's adjustments are kept in, in the pack's folder. Its extension is none a skin
/// document is written with, here or in the reference, so a walk of the folder passes it over.
pub(crate) const ADJUSTMENTS_FILE: &str = "rbms-adjustments.ron";

/// The layout this build writes, and the newest it reads.
pub(crate) const ADJUSTMENTS_VERSION: u32 = 1;

/// The longest adjustments file that is read. One written for a pack of a dozen documents with a
/// hundred rows each is a few tens of kilobytes; a larger file is not one this player wrote.
pub(crate) const ADJUSTMENTS_MAX_BYTES: u64 = 512 * 1024;

/// The most lines an adjustments file may have and still be read.
pub(crate) const ADJUSTMENTS_MAX_LINES: usize = 16 * 1024;

/// What separates the parts of a path inside a pack, on every platform.
const PACK_SEPARATOR: char = '/';

/// The adjustments of one pack, as the file holds them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PackAdjustments {
    /// The layout the file was written in.
    pub(crate) version: u32,
    /// Each adjusted document, by its name inside the pack.
    pub(crate) documents: BTreeMap<String, DocumentAdjustments>,
}

/// What was chosen inside one document, by the names the document gives its rows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct DocumentAdjustments {
    /// Option name to the name of the item it is on.
    pub(crate) options: BTreeMap<String, String>,
    /// File slot name to what fills it.
    pub(crate) files: BTreeMap<String, ChosenFile>,
    /// Offset name to the figure of each of its axes.
    pub(crate) offsets: BTreeMap<String, AxisFigures>,
}

impl DocumentAdjustments {
    /// Whether nothing was chosen in the document at all.
    fn is_empty(&self) -> bool {
        self.options.is_empty() && self.files.is_empty() && self.offsets.is_empty()
    }
}

/// What fills a file slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ChosenFile {
    /// One of the slot's files, drawn afresh each time the document is read.
    Random,
    /// The file at this path inside the pack, with `/` between its parts.
    File(String),
}

/// The figure of each axis of one offset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AxisFigures {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) r: f32,
    pub(crate) a: f32,
}

impl From<SkinOffset> for AxisFigures {
    fn from(offset: SkinOffset) -> Self {
        AxisFigures { x: offset.x, y: offset.y, w: offset.w, h: offset.h, r: offset.r, a: offset.a }
    }
}

impl AxisFigures {
    /// The offset these figures make, each held to how far its axis may be nudged. A figure that is
    /// not a number is no nudge.
    fn offset(self) -> SkinOffset {
        let mut offset = SkinOffset { x: self.x, y: self.y, w: self.w, h: self.h, r: self.r, a: self.a };
        for axis in OffsetAxis::ALL {
            let figure = axis.get(offset);
            let held = if figure.is_finite() { figure.clamp(-axis.limit(), axis.limit()) } else { super::OFFSET_RESET };
            axis.set(&mut offset, held);
        }
        offset
    }
}

/// The adjustments file of the pack in `pack`. `Ok(None)` for a pack that has none, and `Err` with
/// one line of why for a file that is there and cannot be used: too large, not text this build can
/// read, or written in a layout newer than this build knows.
///
/// Only a regular file is read. A link under that name leads wherever whoever made the pack pointed
/// it, and is not followed.
pub(crate) fn read(pack: &Path) -> Result<Option<PackAdjustments>, String> {
    let path = pack.join(ADJUSTMENTS_FILE);
    let found = match std::fs::symlink_metadata(&path) {
        Ok(found) => found,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !found.file_type().is_file() {
        return Err("is not a regular file".to_owned());
    }
    if found.len() > ADJUSTMENTS_MAX_BYTES {
        return Err(format!("is larger than {ADJUSTMENTS_MAX_BYTES} bytes"));
    }
    let mut text = String::new();
    let file = std::fs::File::open(&path).map_err(|error| error.to_string())?;
    let read = file.take(ADJUSTMENTS_MAX_BYTES + 1).read_to_string(&mut text).map_err(|error| error.to_string())?;
    if read as u64 > ADJUSTMENTS_MAX_BYTES {
        return Err(format!("is larger than {ADJUSTMENTS_MAX_BYTES} bytes"));
    }
    parse(&text).map(Some)
}

/// The adjustments a file's text holds, or one line of why it holds none this build can use.
pub(crate) fn parse(text: &str) -> Result<PackAdjustments, String> {
    if text.lines().count() > ADJUSTMENTS_MAX_LINES {
        return Err(format!("has more than {ADJUSTMENTS_MAX_LINES} lines"));
    }
    let adjustments: PackAdjustments = ron::from_str(text).map_err(|error| error.to_string())?;
    if adjustments.version == 0 || adjustments.version > ADJUSTMENTS_VERSION {
        return Err(format!("is written in layout {}, and this build reads up to {ADJUSTMENTS_VERSION}", adjustments.version));
    }
    Ok(adjustments)
}

/// The text of an adjustments file, in the layout this build writes.
pub(crate) fn text(documents: BTreeMap<String, DocumentAdjustments>) -> Result<String, String> {
    let adjustments = PackAdjustments { version: ADJUSTMENTS_VERSION, documents };
    let text = ron::ser::to_string_pretty(&adjustments, ron::ser::PrettyConfig::default()).map_err(|error| error.to_string())?;
    Ok(format!("{text}\n"))
}

/// The name a document of a pack is written under in the pack's adjustments: its file's name, since
/// a pack's documents are the ones directly in its folder.
pub(crate) fn document_name(header: &SkinHeader) -> Option<String> {
    header.path.file_name().map(|name| name.to_string_lossy().into_owned())
}

/// What the choices in force for one document of the pack in `pack` are written as, or `None` when
/// nothing is chosen in it. `chosen` is by the document's own numbers, as the settings file has it.
///
/// A choice of something the document no longer declares -- an option of an earlier version, a file
/// that has been taken away -- has no name to be written under and is left out.
pub(crate) fn written_for(header: &SkinHeader, pack: &Path, chosen: &SkinCustomisation) -> Option<DocumentAdjustments> {
    let mut written = DocumentAdjustments::default();
    for property in &header.properties {
        let Some(stored) = chosen.properties.get(&property.name) else {
            continue;
        };
        let op = selected_option(property, Some(*stored));
        if let Some(item) = property.item.iter().find(|item| item.op == op) {
            written.options.entry(property.name.clone()).or_insert_with(|| item.name.clone());
        }
    }
    for slot in &header.custom_files {
        let file = match chosen.filepaths.get(&slot.name).map(String::as_str) {
            Some(RANDOM_SELECTION) => Some(ChosenFile::Random),
            Some(name) if slot.candidates.iter().any(|candidate| candidate == name) => path_in_pack(slot, pack, name).map(ChosenFile::File),
            _ => None,
        };
        if let Some(file) = file {
            written.files.entry(slot.name.clone()).or_insert(file);
        }
    }
    for def in &header.offsets {
        if let Some(offset) = chosen.offsets.get(&def.id).filter(|offset| **offset != SkinOffset::default()) {
            written.offsets.entry(def.name.clone()).or_insert_with(|| AxisFigures::from(*offset));
        }
    }
    (!written.is_empty()).then_some(written)
}

/// What a pack's adjustments give one of its documents, by the document's own numbers: the stored
/// half of what a loader is handed. Whatever names something the document does not declare is
/// passed over.
pub(crate) fn given_by(header: &SkinHeader, pack: &Path, adjustments: &DocumentAdjustments) -> SkinCustomisation {
    let mut given = SkinCustomisation::default();
    for (name, item) in &adjustments.options {
        let declared = header.properties.iter().find(|property| property.name == *name);
        if let Some(found) = declared.and_then(|property| property.item.iter().find(|entry| entry.name == *item)) {
            given.properties.insert(name.clone(), found.op);
        }
    }
    for (name, file) in &adjustments.files {
        let Some(slot) = header.custom_files.iter().find(|slot| slot.name == *name) else {
            continue;
        };
        let chosen = match file {
            ChosenFile::Random => slot.candidates.iter().find(|candidate| *candidate == RANDOM_SELECTION),
            ChosenFile::File(path) => slot
                .candidates
                .iter()
                .filter(|candidate| *candidate != RANDOM_SELECTION)
                .find(|candidate| path_in_pack(slot, pack, candidate).as_deref() == Some(path)),
        };
        if let Some(chosen) = chosen {
            given.filepaths.insert(name.clone(), chosen.clone());
        }
    }
    for (name, figures) in &adjustments.offsets {
        if let Some(def) = header.offsets.iter().find(|def| def.name == *name) {
            given.offsets.insert(def.id, figures.offset());
        }
    }
    given
}

/// The path inside the pack in `pack` of the file `name` of one slot, with `/` between its parts,
/// or `None` for a slot whose files are not in the pack.
///
/// A slot's pattern is spelled from the folder of its document down, in the one spelling patterns
/// are compared in, so the pack's folder is spelled the same way before it is taken off the front.
fn path_in_pack(slot: &CustomFile, pack: &Path, name: &str) -> Option<String> {
    let root = pattern_for(pack, "");
    let folder = slot.pattern.rfind(PACK_SEPARATOR).map_or("", |at| &slot.pattern[..at]);
    let below = folder.strip_prefix(root.as_str())?;
    match below.strip_prefix(PACK_SEPARATOR) {
        Some(below) => Some(format!("{below}{PACK_SEPARATOR}{name}")),
        None if below.is_empty() => Some(name.to_owned()),
        None => None,
    }
}

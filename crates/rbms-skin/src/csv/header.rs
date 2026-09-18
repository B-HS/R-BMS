//! The header file of a comma-separated skin: which screen it draws, what it is called, and the
//! customisation rows it offers.
//!
//! The header is read on its own, both to list a document in the configuration screen and to settle
//! the option ids before a single body line is read, because those ids decide which `#IF` blocks of
//! the bodies are even parsed.

use std::path::Path;

use crate::SkinError;
use crate::loader::skin_type_mode;
use crate::model::{Filepath, OffsetDef, PropertyDef, PropertyItem, SKIN_TYPE_UNSET};

use super::convert::{DEFAULT_RESOLUTION, parse_fields, parse_number, resolution};
use super::{Commands, Outcome, Script, read_ms932, skin_relative};

/// What the header file said.
#[derive(Debug, Clone)]
pub struct CsvHeader {
    pub skin_type: i32,
    pub name: String,
    pub author: String,
    /// The resolution the document's coordinates are written in, which `#RESOLUTION` names.
    pub resolution: (i32, i32),
    pub properties: Vec<PropertyDef>,
    pub filepaths: Vec<Filepath>,
    pub offsets: Vec<OffsetDef>,
}

impl Default for CsvHeader {
    fn default() -> Self {
        Self {
            skin_type: SKIN_TYPE_UNSET,
            name: String::new(),
            author: String::new(),
            resolution: DEFAULT_RESOLUTION,
            properties: Vec::new(),
            filepaths: Vec::new(),
            offsets: Vec::new(),
        }
    }
}

/// The customisation rows every play document gets whether or not it asks for them, as the
/// reference adds them: the name, the option ids its items turn on, and what each item is called.
const PLAY_OPTIONS: &[(&str, &[i32], &[&str])] = &[
    ("BGA Size", &[30, 31], &["Normal", "Extend"]),
    ("Ghost", &[34, 35, 36, 37], &["Off", "Type A", "Type B", "Type C"]),
    ("Score Graph", &[38, 39], &["Off", "On"]),
    ("Judge Detail", &[1997, 1998, 1999], &["Off", "EARLY/LATE", "+-ms"]),
];

/// The nudge rows every play document gets, and which of the six axes each one allows.
const PLAY_OFFSETS: &[(&str, i32, [bool; 6])] = &[
    ("All offset(%)", 10, [true, true, true, true, false, false]),
    ("Notes offset", 30, [false, false, false, true, false, false]),
    ("Judge offset", 32, [true, true, true, true, false, true]),
    ("Judge Detail offset", 33, [true, true, true, true, false, true]),
];

/// How many axis flags a `#CUSTOMOFFSET` line carries.
const OFFSET_AXES: usize = 6;

/// The field a `#CUSTOMOFFSET` line's first axis flag sits in.
const FIRST_AXIS_FIELD: usize = 3;

/// The field a `#CUSTOMOPTION` line's first item name sits in, after its own name and its first id.
const FIRST_ITEM_FIELD: usize = 3;

/// Reads one header file.
pub(crate) fn parse(path: &Path, root: &Path, directory: &Path, limit: u64) -> Result<CsvHeader, SkinError> {
    let text = read_ms932(path, limit)?;
    let mut script = Script::new(std::collections::BTreeMap::new());
    let mut commands = HeaderCommands { header: CsvHeader::default(), root: root.to_path_buf(), directory: directory.to_path_buf() };
    script.run(&text, 0, &mut commands, &mut |_| None);
    Ok(commands.header)
}

/// The header commands, gathering into one [`CsvHeader`].
struct HeaderCommands {
    header: CsvHeader,
    root: std::path::PathBuf,
    directory: std::path::PathBuf,
}

impl Commands for HeaderCommands {
    fn execute(&mut self, name: &str, fields: &[&str], warnings: &mut Vec<String>) -> Outcome {
        match name {
            "INFORMATION" => self.information(fields),
            "RESOLUTION" => self.resolution(fields, warnings),
            "CUSTOMOPTION" => self.custom_option(fields),
            "CUSTOMFILE" => self.custom_file(fields),
            "CUSTOMOFFSET" => self.custom_offset(fields),
            "CUSTOMOPTION_ADDITION_SETTING" => self.addition_setting(fields),
            "INCLUDE" => {}
            _ => return Outcome::Unknown,
        }
        Outcome::Handled
    }
}

impl HeaderCommands {
    /// `#INFORMATION,type,name,author`. A play document also collects the four rows and four nudges
    /// the reference gives every play skin, which `#CUSTOMOPTION_ADDITION_SETTING` may take back.
    fn information(&mut self, fields: &[&str]) {
        self.header.skin_type = fields.get(1).and_then(|field| parse_number(field)).unwrap_or(SKIN_TYPE_UNSET);
        self.header.name = fields.get(2).unwrap_or(&"").trim().to_owned();
        self.header.author = fields.get(3).unwrap_or(&"").trim().to_owned();
        if skin_type_mode(self.header.skin_type).is_none() {
            return;
        }
        for (name, ids, items) in PLAY_OPTIONS {
            let item = ids.iter().zip(items.iter()).map(|(op, name)| PropertyItem { name: (*name).to_owned(), op: *op }).collect();
            self.header.properties.push(PropertyDef { name: (*name).to_owned(), item, ..PropertyDef::default() });
        }
        for (name, id, axes) in PLAY_OFFSETS {
            self.header.offsets.push(OffsetDef {
                name: (*name).to_owned(),
                id: *id,
                x: axes[0],
                y: axes[1],
                w: axes[2],
                h: axes[3],
                r: axes[4],
                a: axes[5],
                ..OffsetDef::default()
            });
        }
    }

    /// `#RESOLUTION,index`, which names the size the rest of the document's coordinates are in.
    fn resolution(&mut self, fields: &[&str], warnings: &mut Vec<String>) {
        let index = fields.get(1).and_then(|field| parse_number(field)).unwrap_or_default();
        match resolution(index) {
            Some(size) => self.header.resolution = size,
            None => warnings.push(format!("resolution {index} is not one this build knows a size for; {DEFAULT_RESOLUTION:?} was used")),
        }
    }

    /// `#CUSTOMOPTION,name,first id,item,item,...`, where item *i* turns on the id `first + i`.
    fn custom_option(&mut self, fields: &[&str]) {
        let base = fields.get(2).and_then(|field| parse_number(field)).unwrap_or_default();
        let item: Vec<PropertyItem> = fields
            .iter()
            .skip(FIRST_ITEM_FIELD)
            .filter(|field| !field.is_empty())
            .enumerate()
            .map(|(index, name)| PropertyItem { name: (*name).trim().to_owned(), op: base + index as i32 })
            .collect();
        let name = fields.get(1).unwrap_or(&"").trim().to_owned();
        self.header.properties.push(PropertyDef { name, item, ..PropertyDef::default() });
    }

    /// `#CUSTOMFILE,name,pattern,default`, a slot the player fills from the files the pattern
    /// matches.
    fn custom_file(&mut self, fields: &[&str]) {
        let path = skin_relative(fields.get(2).unwrap_or(&""), &self.root, &self.directory);
        let def = fields.get(3).map(|name| name.trim()).filter(|name| !name.is_empty()).map(str::to_owned);
        self.header.filepaths.push(Filepath { name: fields.get(1).unwrap_or(&"").trim().to_owned(), path, def, ..Filepath::default() });
    }

    /// `#CUSTOMOFFSET,name,id,x,y,w,h,r,a`, where an axis with no flag of its own is allowed.
    fn custom_offset(&mut self, fields: &[&str]) {
        let mut axes = [true; OFFSET_AXES];
        for (index, axis) in axes.iter_mut().enumerate() {
            let Some(field) = fields.get(index + FIRST_AXIS_FIELD) else { break };
            *axis = parse_number(field).unwrap_or_default() > 0;
        }
        let values = parse_fields(fields);
        self.header.offsets.push(OffsetDef {
            name: fields.get(1).unwrap_or(&"").trim().to_owned(),
            id: values[2],
            x: axes[0],
            y: axes[1],
            w: axes[2],
            h: axes[3],
            r: axes[4],
            a: axes[5],
            ..OffsetDef::default()
        });
    }

    /// `#CUSTOMOPTION_ADDITION_SETTING,bga,ghost,graph,judge`, where a zero drops the matching row
    /// the play document was given automatically.
    fn addition_setting(&mut self, fields: &[&str]) {
        for (index, (name, _, _)) in PLAY_OPTIONS.iter().enumerate() {
            let keep = fields.get(index + 1).and_then(|field| parse_number(field)).unwrap_or(1) != 0;
            if !keep {
                self.header.properties.retain(|property| property.name != *name);
            }
        }
    }
}

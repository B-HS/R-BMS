//! Reading a skin written in the comma-separated format into the same [`SkinDef`] a JSON document
//! parses into.
//!
//! The format is a header file, named with the [`CSV_EXTENSION`] extension, that says which screen
//! the skin draws and which customisation rows it offers, plus the comma-separated bodies it pulls
//! in with `#INCLUDE`. Both are read as MS932 rather than UTF-8, because that is the encoding the
//! format was written in and the only one its Japanese names survive.
//!
//! Nothing downstream knows this module exists: a converted document goes through the same
//! customisation rows, the same file resolution and the same renderer as a JSON one, because it
//! arrives as the same [`SkinDef`]. That is the whole point of converting rather than adding a
//! second draw path.

mod body;
mod convert;
mod graphs;
mod header;
mod play;
mod select;

pub use header::CsvHeader;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::SkinError;
use crate::loader::{
    LoadedSkin, Parsed, ParserKind, SkinHeader, SkinLoadOptions, assemble, declared_options, enabled_options, is_supported_skin_type, selected_option,
};
use crate::model::{Destination, PropertyDef, SKIN_TYPE_UNSET, SkinDef};
use crate::resolve::{Draw, FileResolver, build_filemap, contained, enumerate_custom_files, pattern_for};

/// The extension the header file of a comma-separated skin is named with. The header is the
/// document; the bodies it includes are parts of it rather than documents of their own.
pub const CSV_EXTENSION: &str = "lr2skin";

/// The prefix a path writes for "the skin folder", which is where every file a skin names lives.
const SKIN_ROOT_PREFIX: &str = "lr2files/theme";

/// How deep `#INCLUDE` may nest before the rest are dropped. A body that includes itself is a
/// mistake rather than a structure, and there is no legitimate skin this deep.
const MAX_INCLUDE_DEPTH: usize = 8;

/// How many files one document may include altogether.
const MAX_INCLUDES: usize = 1_024;

/// How many unknown command names a warning lists before it stops naming them.
const MAX_REPORTED_UNKNOWN: usize = 8;

/// Whether this path names the header file of a comma-separated skin.
pub fn is_csv_document(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| extension.eq_ignore_ascii_case(CSV_EXTENSION))
}

/// What one command line asked the reader to do.
pub(crate) enum Outcome {
    /// The command was one this build implements.
    Handled,
    /// The command is not one this build implements, and is counted for the warning summary.
    Unknown,
    /// The command names a file whose lines belong here, expanded into the same reader state.
    Include(String),
}

/// A set of commands one file's lines are dispatched to.
pub(crate) trait Commands {
    /// Runs one command line. `fields` is the whole line, the command name included, so a command
    /// that reads a field as text rather than as a number can reach it.
    fn execute(&mut self, name: &str, fields: &[&str], warnings: &mut Vec<String>) -> Outcome;
}

/// The reader state one document shares across every file it includes.
///
/// `#IF` is deliberately two flags rather than a stack. The reference keeps exactly one condition
/// and one skip flag, so a nested `#ENDIF` clears the outer block as well and a nested `#IF`
/// overwrites the outer condition. A skin written against that behaviour draws differently under a
/// reader with a proper stack, so this one does not have a stack.
pub(crate) struct Script {
    skip: bool,
    ifs: bool,
    options: BTreeMap<i32, i32>,
    /// Everything that went wrong without costing the document.
    pub(crate) warnings: Vec<String>,
    unknown: BTreeMap<String, usize>,
    includes: usize,
}

impl Script {
    /// A reader whose option ids answer as `options` says.
    pub(crate) fn new(options: BTreeMap<i32, i32>) -> Self {
        Self { skip: false, ifs: false, options, warnings: Vec::new(), unknown: BTreeMap::new(), includes: 0 }
    }

    /// Runs every line of one file, expanding whatever `open` can read for an `#INCLUDE`.
    pub(crate) fn run(&mut self, text: &str, depth: usize, commands: &mut dyn Commands, open: &mut dyn FnMut(&str) -> Option<String>) {
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if !line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split(',').collect();
            let Some(name) = fields.first().map(|name| name[1..].to_ascii_uppercase()) else {
                continue;
            };
            self.branch(&name, &fields);
            if self.skip {
                continue;
            }
            if name == "SETOPTION" {
                self.set_option(&fields);
            }
            match commands.execute(&name, &fields, &mut self.warnings) {
                Outcome::Handled => {}
                Outcome::Unknown => *self.unknown.entry(name).or_default() += 1,
                Outcome::Include(target) => self.include(&target, depth, commands, open),
            }
        }
    }

    /// Expands one included file into this same reader, which is what lets an unbalanced `#IF` in
    /// an included body leak into the file that included it.
    fn include(&mut self, target: &str, depth: usize, commands: &mut dyn Commands, open: &mut dyn FnMut(&str) -> Option<String>) {
        if depth >= MAX_INCLUDE_DEPTH {
            self.warnings.push(format!("include {target:?} nests deeper than {MAX_INCLUDE_DEPTH} files and was skipped"));
            return;
        }
        if self.includes >= MAX_INCLUDES {
            return;
        }
        self.includes += 1;
        if self.includes == MAX_INCLUDES {
            self.warnings.push(format!("the document expanded more than {MAX_INCLUDES} includes; the rest were skipped"));
            return;
        }
        let Some(text) = open(target) else {
            self.warnings.push(format!("include {target:?} could not be read"));
            return;
        };
        self.run(&text, depth + 1, commands, open);
    }

    /// Applies `#IF`, `#ELSEIF`, `#ELSE` and `#ENDIF` to the two flags this reader keeps.
    fn branch(&mut self, name: &str, fields: &[&str]) {
        match name {
            "IF" => {
                self.ifs = self.condition(fields);
                self.skip = !self.ifs;
            }
            "ELSEIF" => {
                if self.ifs {
                    self.skip = true;
                } else {
                    self.ifs = self.condition(fields);
                    self.skip = !self.ifs;
                }
            }
            "ELSE" => self.skip = self.ifs,
            "ENDIF" => {
                self.skip = false;
                self.ifs = false;
            }
            _ => {}
        }
    }

    /// Whether every option a guard names holds. An empty field is skipped and an unreadable one
    /// fails the whole guard.
    fn condition(&self, fields: &[&str]) -> bool {
        fields.iter().skip(1).filter(|field| !field.is_empty()).all(|field| self.holds(field))
    }

    /// Whether one option id holds, `!` meaning "and it must be off".
    ///
    /// The reference falls back to the running game's own state for an id no customisation row
    /// declares. Conversion happens before a screen exists, so an id this document never declares
    /// answers false here instead.
    fn holds(&self, field: &str) -> bool {
        let digits: String =
            field.chars().map(|glyph| if glyph == convert::NEGATION { '-' } else { glyph }).filter(|glyph| glyph.is_ascii_digit() || *glyph == '-').collect();
        let Ok(id) = digits.parse::<i32>() else { return false };
        if id >= 0 { self.options.get(&id) == Some(&1) } else { self.options.get(&-id) == Some(&0) }
    }

    /// Applies `#SETOPTION,id,value`, which turns one option on or off for the rest of the document.
    fn set_option(&mut self, fields: &[&str]) {
        let values = convert::parse_fields(fields);
        self.options.insert(values[1], i32::from(values[2] >= 1));
    }

    /// Adds one line naming the commands this build passed over, so a skin that needs something
    /// unimplemented says so once rather than not at all.
    pub(crate) fn finish(&mut self) {
        if self.unknown.is_empty() {
            return;
        }
        let mut names: Vec<(&String, &usize)> = self.unknown.iter().collect();
        names.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        let listed: Vec<String> = names.iter().take(MAX_REPORTED_UNKNOWN).map(|(name, count)| format!("#{name} x{count}")).collect();
        let total: usize = self.unknown.values().sum();
        self.warnings.push(format!("{total} command lines are not ones this build reads: {}", listed.join(", ")));
    }
}

/// The text of one MS932 file, refused when it is over the document ceiling.
///
/// Every file of a comma-separated skin is MS932, never UTF-8. The decoder replaces a byte sequence
/// it cannot read rather than failing, because one bad name is not worth losing a skin over.
pub(crate) fn read_ms932(path: &Path, limit: u64) -> Result<String, SkinError> {
    let size = std::fs::metadata(path).map_err(SkinError::Read)?.len();
    if size > limit {
        return Err(SkinError::TooLarge { path: path.to_string_lossy().into_owned(), actual: size, limit });
    }
    let bytes = std::fs::read(path).map_err(SkinError::Read)?;
    let (text, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
    Ok(text.into_owned())
}

/// The path a CSV file names, as a pattern relative to the document's own directory.
///
/// A path is written with backslashes and may start with the folder prefix that stands for the skin
/// folder. Everything downstream joins patterns onto the document's directory, so the prefix is
/// turned back into a relative walk rather than into an absolute path.
pub(crate) fn skin_relative(raw: &str, root: &Path, directory: &Path) -> String {
    let path = raw.trim().replace('\\', "/");
    let Some(tail) = strip_root_prefix(&path) else {
        return path;
    };
    let absolute = format!("{}/{}", root.to_string_lossy().replace('\\', "/").trim_end_matches('/'), tail);
    relative_path(directory, &absolute)
}

/// The part of a path that follows the skin-folder prefix, or `None` when it has none.
fn strip_root_prefix(path: &str) -> Option<&str> {
    let head = path.get(..SKIN_ROOT_PREFIX.len())?;
    if !head.eq_ignore_ascii_case(SKIN_ROOT_PREFIX) {
        return None;
    }
    Some(path[SKIN_ROOT_PREFIX.len()..].trim_start_matches('/'))
}

/// A walk from `from` to `to`, both written with forward slashes.
fn relative_path(from: &Path, to: &str) -> String {
    let base = from.to_string_lossy().replace('\\', "/");
    let base: Vec<&str> = base.split('/').filter(|part| !part.is_empty()).collect();
    let target: Vec<&str> = to.split('/').filter(|part| !part.is_empty()).collect();
    let shared = base.iter().zip(target.iter()).take_while(|(left, right)| left == right).count();
    let mut parts: Vec<&str> = vec![".."; base.len() - shared];
    parts.extend(target[shared..].iter().copied());
    parts.join("/")
}

/// Takes the destinations `ids` names out of the document, in the order `ids` lists them.
///
/// Several object kinds -- the wheel's rows, a pop-up's words, the measure lines -- are declared as
/// ordinary objects while the body is read and only belong to their parent once every line has been
/// seen. They are parked in the top-level list until then and lifted out here, one entry per slot:
/// a slot the body never declared comes back as an empty destination rather than as nothing, so the
/// slots behind it keep their places.
pub(crate) fn detach(def: &mut SkinDef, ids: &[String]) -> Vec<Destination> {
    let mut taken: Vec<Option<Destination>> = std::iter::repeat_with(|| None).take(ids.len()).collect();
    let mut kept = Vec::with_capacity(def.destination.len());
    for destination in std::mem::take(&mut def.destination) {
        match ids.iter().enumerate().find(|(at, id)| **id == destination.id && taken[*at].is_none()).map(|(at, _)| at) {
            Some(at) => taken[at] = Some(destination),
            None => kept.push(destination),
        }
    }
    def.destination = kept;
    taken.into_iter().map(Option::unwrap_or_default).collect()
}

/// The option ids the reader answers with: every id the document's own rows declare, on for the
/// item each row is switched to and off for the rest.
fn option_map(properties: &[PropertyDef], chosen: &BTreeMap<String, i32>) -> BTreeMap<i32, i32> {
    let mut options = BTreeMap::new();
    for property in properties {
        let selected = selected_option(property, chosen.get(&property.name).copied());
        for item in &property.item {
            options.insert(item.op, i32::from(item.op == selected));
        }
    }
    options
}

/// What a comma-separated document says about itself, without reading its bodies.
pub fn load_header(path: &Path, options: SkinLoadOptions<'_>) -> Result<SkinHeader, SkinError> {
    let path = contained(options.root, path)?;
    let directory = path.parent().unwrap_or(options.root).to_path_buf();
    let header = header::parse(&path, options.root, &directory, options.max_document_bytes)?;
    let document = SkinDef { filepath: header.filepaths.clone(), ..SkinDef::default() };

    Ok(SkinHeader {
        skin_type: header.skin_type,
        name: header.name.clone(),
        author: header.author.clone(),
        width: crate::model::DEFAULT_SKIN_WIDTH,
        height: crate::model::DEFAULT_SKIN_HEIGHT,
        parser: ParserKind::Csv,
        properties: header.properties.clone(),
        offsets: header.offsets.clone(),
        custom_files: enumerate_custom_files(&document, &directory, options.root),
        path,
    })
}

/// Reads a comma-separated document and everything it names.
pub fn load_skin(path: &Path, options: SkinLoadOptions<'_>) -> Result<LoadedSkin, SkinError> {
    let path = contained(options.root, path)?;
    let directory = path.parent().unwrap_or(options.root).to_path_buf();
    let header = header::parse(&path, options.root, &directory, options.max_document_bytes)?;
    if header.skin_type == SKIN_TYPE_UNSET {
        return Err(SkinError::TypeMissing);
    }
    if !is_supported_skin_type(header.skin_type) {
        return Err(SkinError::TypeUnsupported(header.skin_type));
    }

    let stub = SkinDef { filepath: header.filepaths.clone(), ..SkinDef::default() };
    let custom_files = enumerate_custom_files(&stub, &directory, options.root);
    let mut draw = options.rng_seed.map_or_else(Draw::from_environment, Draw::from_seed);
    let filemap = build_filemap(&custom_files, options.user, &mut draw);
    let mut resolver = FileResolver::new(options.root, filemap, draw);

    let mut script = Script::new(option_map(&header.properties, &options.user.properties));
    let mut builder = body::Builder::new(&header, options.root, &directory);
    let limit = options.max_document_bytes;
    let mut open = |target: &str| -> Option<String> {
        let pattern = pattern_for(&directory, &skin_relative(target, options.root, &directory));
        let file = resolver.resolve(&pattern).ok()?;
        read_ms932(&file, limit).ok()
    };
    let text = read_ms932(&path, limit)?;
    script.run(&text, 0, &mut builder, &mut open);
    script.finish();

    let def = builder.finish();
    let enabled = enabled_options(&def.property, &options.user.properties);
    let declared = declared_options(&def.property);
    let parsed = Parsed { def, parser: ParserKind::Csv, custom_files, enabled, declared, resolver, warnings: script.warnings };
    assemble(path, parsed, options)
}

/// The document a comma-separated file converts to, without the file resolution and track assembly
/// a whole load performs. Tests read this; a load goes through [`load_skin`].
pub fn parse_document(path: &Path, root: &Path, limit: u64, chosen: &BTreeMap<String, i32>) -> Result<(SkinDef, Vec<String>), SkinError> {
    let directory = path.parent().unwrap_or(root).to_path_buf();
    let header = header::parse(path, root, &directory, limit)?;
    let mut script = Script::new(option_map(&header.properties, chosen));
    let mut builder = body::Builder::new(&header, root, &directory);
    let mut open = |target: &str| -> Option<String> {
        let file = PathBuf::from(pattern_for(&directory, &skin_relative(target, root, &directory)));
        read_ms932(&contained(root, &file).ok()?, limit).ok()
    };
    let text = read_ms932(path, limit)?;
    script.run(&text, 0, &mut builder, &mut open);
    script.finish();
    Ok((builder.finish(), script.warnings))
}

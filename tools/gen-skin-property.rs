//! Generates `crates/rbms-skin/src/property/generated.rs` and its submodules from the reference
//! implementation's `SkinProperty.java` and the property factories beside it.
//!
//! The reference tree is not part of this repository, so this runs by hand and its output is
//! committed. Build, run and format it with:
//!
//! ```text
//! rustc --edition 2024 -O tools/gen-skin-property.rs -o target/gen-skin-property
//! target/gen-skin-property <path to SkinProperty.java> crates/rbms-skin/src/property
//! rustfmt --edition 2024 crates/rbms-skin/src/property/generated.rs crates/rbms-skin/src/property/generated/*.rs
//! ```
//!
//! It extracts every `public static final int <PREFIX>_<NAME> = <n>;` declaration in source order
//! and emits one `pub const` per declaration plus an `ALL_<PREFIX>` index array whose entries name
//! those constants, so the array and the constants cannot drift apart. `TIMER_*` belongs to
//! `tools/gen-skin-timer.rs`; this generator only counts those declarations, so the two committed
//! tables together account for every declaration in the source.
//!
//! From the `property/` directory next to that file it also reads the enums the reference resolves
//! a property *name* through -- `BooleanType`, `ValueType`, `IndexType`, `RateType`, `FloatType`,
//! `StringType` and `EventType` -- and emits one `(id, name)` table per enum into the `names`
//! module, together with the ids of the booleans the reference settles once instead of every frame
//! (`BooleanProperty.isStatic`) and the ids of the rates and strings that are declared with a writer. The numbered name families the reference builds at run time
//! (`practice_item3`, `ranking_exscore10`, ...) are not enum constants and cannot be read from the
//! source this way; `crates/rbms-skin/src/property/mod.rs` spells those out by hand.
//!
//! Output is deterministic: declarations keep source order throughout, groups and prefixes are
//! fixed lists, and no hashing container takes part. Running it twice on one input yields identical
//! bytes.

use std::fmt::Write as _;

/// The declaration prefix every extracted constant carries in the reference source.
const DECLARATION: &str = "public static final int ";

/// The name prefix owned by `tools/gen-skin-timer.rs`, counted here but never emitted.
const TIMER_PREFIX: &str = "TIMER_";

/// FNV-1a 64-bit offset basis, matching the signature hashing already used by `rbms-render`.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Exit code for a usage error, matching the convention `tools/gen-skin-timer.rs` uses.
const EXIT_USAGE: i32 = 2;

/// Exit code for a failure while reading or extracting.
const EXIT_FAILURE: i32 = 1;

/// The directory, beside the constant source, that holds the property factories.
const FACTORY_DIRECTORY: &str = "property";

/// The qualifier a factory may write in front of a constant of the constant source.
const CONSTANT_QUALIFIER: &str = "SkinProperty.";

/// The keyword an enum declaration opens with.
const ENUM_KEYWORD: &str = "enum ";

/// The keyword a class declaration opens with.
const CLASS_KEYWORD: &str = "class ";

/// The keyword that precedes a constructor call.
const NEW_KEYWORD: &str = "new ";

/// The return type every boolean helper method of the reference declares.
const BOOLEAN_HELPER_RETURN_TYPE: &str = "DrawProperty ";

/// The prefix of the four constants the reference classifies a boolean's stillness with.
const STATIC_TYPE_PREFIX: &str = "TYPE_";

/// The module the name tables are written to.
const NAMES_MODULE: &str = "names";

/// One enum of the reference whose constants are property names.
struct NameSpace {
    /// The factory file, inside [`FACTORY_DIRECTORY`].
    file: &'static str,
    /// The enum's Java name.
    java_enum: &'static str,
    /// The stem of the emitted table, `ALL_<stem>_NAME`.
    stem: &'static str,
    /// What the table's doc comment calls the id space.
    describes: &'static str,
    /// The emitted list of the ids that can be written as well as read, with its doc line, for the
    /// enums whose constants may carry a writer.
    writers: Option<(&'static str, &'static str)>,
}

/// The name tables, in the order they are emitted and hashed.
const NAME_SPACES: &[NameSpace] = &[
    NameSpace { file: "BooleanPropertyFactory.java", java_enum: "BooleanType", stem: "BOOLEAN", describes: "option", writers: None },
    NameSpace { file: "IntegerPropertyFactory.java", java_enum: "ValueType", stem: "INTEGER", describes: "number", writers: None },
    NameSpace { file: "IntegerPropertyFactory.java", java_enum: "IndexType", stem: "IMAGE_INDEX", describes: "image index", writers: None },
    NameSpace {
        file: "FloatPropertyFactory.java",
        java_enum: "RateType",
        stem: "RATE",
        describes: "rate",
        writers: Some(("WRITABLE_RATES", "The rate ids a slider can write back to (`FloatPropertyFactory.getRateWriter`).")),
    },
    NameSpace { file: "FloatPropertyFactory.java", java_enum: "FloatType", stem: "FLOAT", describes: "float", writers: None },
    NameSpace {
        file: "StringPropertyFactory.java",
        java_enum: "StringType",
        stem: "STRING",
        describes: "string",
        writers: Some(("WRITABLE_STRINGS", "The string ids an editable text can write back to (`StringPropertyFactory.getStringWriter`).")),
    },
    NameSpace { file: "EventFactory.java", java_enum: "EventType", stem: "EVENT", describes: "event", writers: None },
];

/// How many arguments a constant of a writable enum takes when it carries a writer: the id, the
/// reader and the writer.
const ARGUMENTS_WITH_WRITER: usize = 3;

/// The enum whose constants also carry a stillness classification.
const BOOLEAN_ENUM: &str = "BooleanType";

/// How long a boolean holds still, as the reference's four `TYPE_*` constants say it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StaticScope {
    /// `TYPE_NO_STATIC`.
    Never,
    /// `TYPE_STATIC_WITHOUT_MUSICSELECT`.
    OutsideSelect,
    /// `TYPE_STATIC_ON_RESULT`.
    OnResult,
    /// `TYPE_STATIC_ALL`.
    Always,
}

/// The reference constants and the scope each stands for, with the emitted list of each scope that
/// is not [`StaticScope::Never`].
const STATIC_SCOPES: &[(&str, StaticScope, Option<(&str, &str)>)] = &[
    ("TYPE_NO_STATIC", StaticScope::Never, None),
    (
        "TYPE_STATIC_WITHOUT_MUSICSELECT",
        StaticScope::OutsideSelect,
        Some(("STATIC_OUTSIDE_SELECT", "The option ids the reference settles once on every screen but the song browser.")),
    ),
    ("TYPE_STATIC_ON_RESULT", StaticScope::OnResult, Some(("STATIC_ON_RESULT", "The option ids the reference settles once on the two result screens."))),
    ("TYPE_STATIC_ALL", StaticScope::Always, Some(("STATIC_ALWAYS", "The option ids the reference settles once on every screen."))),
];

/// One enum constant that names a property.
struct NameEntry {
    name: String,
    id: i32,
    /// Set for the constants of [`BOOLEAN_ENUM`] alone.
    scope: Option<StaticScope>,
    /// How many arguments the constant is declared with.
    arguments: usize,
}

/// One emitted name table.
struct NameTable {
    space: &'static NameSpace,
    entries: Vec<NameEntry>,
}

/// One output module: a file name, its module doc lines, and the declaration prefixes it carries.
struct Group {
    module: &'static str,
    doc: &'static [&'static str],
    prefixes: &'static [&'static str],
}

/// The output modules, grouped by the registry kind that reads them. Splitting the table keeps
/// every generated file well inside the repository's file length limit.
const GROUPS: &[Group] = &[
    Group { module: "boolean", doc: &["Skin property identifiers the boolean registry reads."], prefixes: &["OPTION"] },
    Group { module: "integer", doc: &["Skin property identifiers the integer registry reads."], prefixes: &["NUMBER"] },
    Group {
        module: "float",
        doc: &[
            "Skin property identifiers the float registry reads.",
            "",
            "`RATE_*` is the canonical namespace and `SLIDER_*`/`BARGRAPH_*` are older names over the",
            "same ids; `FLOAT_*` is a separate id space that does not overlap them.",
        ],
        prefixes: &["RATE", "SLIDER", "BARGRAPH", "FLOAT"],
    },
    Group { module: "text", doc: &["Skin property identifiers the string registry reads."], prefixes: &["STRING"] },
    Group {
        module: "misc",
        doc: &[
            "Skin identifiers no registry accessor reads.",
            "",
            "Clickable controls, destination offsets, the per-lane judge values a play skin writes, the",
            "dynamic image slots and the custom event band.",
        ],
        prefixes: &["BUTTON", "OFFSET", "VALUE", "IMAGE", "EVENT"],
    },
];

/// One extracted declaration.
struct Entry {
    name: String,
    value: i32,
}

/// FNV-1a over the `NAME=VALUE` lines of every emitted table, prefix by prefix in the order
/// [`known_prefixes`] lists them and in source order within each.
///
/// Hashing per table rather than in raw source order is what lets the committed test re-derive the
/// value from the committed tables alone, since the source interleaves the prefixes.
fn checksum(extraction: &Extraction) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for prefix in known_prefixes() {
        for entry in extraction.by_prefix(prefix) {
            for byte in format!("{}={}\n", entry.name, entry.value).into_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
    }
    hash
}

/// Every prefix the generator knows, in the order the groups list them.
fn known_prefixes() -> Vec<&'static str> {
    GROUPS.iter().flat_map(|group| group.prefixes.iter().copied()).collect()
}

/// The prefix a declaration name belongs to, or `None` when the source grew one this generator does
/// not place.
fn prefix_of(name: &str) -> Option<&'static str> {
    known_prefixes().into_iter().find(|prefix| name.len() > prefix.len() + 1 && name.starts_with(prefix) && name.as_bytes()[prefix.len()] == b'_')
}

/// What one pass over the reference source found.
struct Extraction {
    entries: Vec<Entry>,
    timer_declarations: usize,
    /// Every declaration, timers included, for resolving an enum constant's id.
    constants: Vec<Entry>,
}

impl Extraction {
    /// Every declaration the source holds, timers included.
    fn total_declarations(&self) -> usize {
        self.entries.len() + self.timer_declarations
    }

    /// The entries carrying one prefix, in source order.
    fn by_prefix(&self, prefix: &str) -> Vec<&Entry> {
        self.entries.iter().filter(|entry| prefix_of(&entry.name) == Some(prefix)).collect()
    }
}

/// Pulls every non-timer constant out of the reference source in declaration order.
fn extract(source: &str) -> Result<Extraction, String> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut constants: Vec<Entry> = Vec::new();
    let mut timer_declarations = 0usize;
    for line in source.lines() {
        let Some(rest) = line.trim_start().strip_prefix(DECLARATION) else { continue };
        let Some((name, value)) = rest.split_once('=') else { continue };
        let name = name.trim();
        let value = value.trim().trim_end_matches(';').trim();
        if name.starts_with(TIMER_PREFIX) {
            timer_declarations += 1;
            if let Ok(value) = value.parse() {
                constants.push(Entry { name: name.to_owned(), value });
            }
            continue;
        }
        let value: i32 = value.parse().map_err(|_| format!("{name} has a non-literal value {value}"))?;
        if prefix_of(name).is_none() {
            return Err(format!("{name} carries a prefix this generator does not place; add it to GROUPS"));
        }
        if let Some(previous) = entries.iter().find(|entry| entry.name == name) {
            return Err(format!("{} is declared twice ({} then {})", name, previous.value, value));
        }
        entries.push(Entry { name: name.to_owned(), value });
        constants.push(Entry { name: name.to_owned(), value });
    }
    if entries.is_empty() {
        return Err("no property declarations found".to_owned());
    }
    if timer_declarations == 0 {
        return Err("no timer declarations found; the input is not the expected source".to_owned());
    }
    Ok(Extraction { entries, timer_declarations, constants })
}

/// The source with every comment and every string and character literal blanked out, byte for
/// byte, so brackets and identifiers can be matched without a literal getting in the way.
fn without_noise(source: &str) -> Vec<u8> {
    let bytes = source.as_bytes();
    let mut clean = bytes.to_vec();
    let mut index = 0;
    while index < bytes.len() {
        let rest = &bytes[index..];
        let end = if rest.starts_with(b"//") {
            index + rest.iter().position(|byte| *byte == b'\n').unwrap_or(rest.len())
        } else if rest.starts_with(b"/*") {
            index + rest.windows(2).skip(2).position(|pair| pair == b"*/").map_or(rest.len(), |position| position + 4)
        } else if rest[0] == b'"' || rest[0] == b'\'' {
            let mut cursor = 1;
            while cursor < rest.len() && rest[cursor] != rest[0] {
                cursor += if rest[cursor] == b'\\' { 2 } else { 1 };
            }
            index + (cursor + 1).min(rest.len())
        } else {
            index += 1;
            continue;
        };
        for byte in &mut clean[index..end] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
        index = end;
    }
    clean
}

/// Whether a byte can be part of a Java identifier.
fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The index of the bracket that closes the one at `open`.
fn closing(clean: &[u8], open: usize) -> Result<usize, String> {
    let mut depth = 0usize;
    for (index, byte) in clean.iter().enumerate().skip(open) {
        match byte {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err(format!("the bracket at byte {open} is never closed"))
}

/// The first place `keyword` and `name` stand together as whole words, with `opens` as the next
/// significant byte when one is asked for.
fn find_declaration(clean: &[u8], keyword: &str, name: &str, opens: Option<u8>) -> Option<usize> {
    let needle = format!("{keyword}{name}");
    let needle = needle.as_bytes();
    (0..clean.len().saturating_sub(needle.len())).find(|index| {
        let after = &clean[*index + needle.len()..];
        clean[*index..].starts_with(needle)
            && (*index == 0 || !is_identifier_byte(clean[*index - 1]))
            && after.first().is_some_and(|byte| !is_identifier_byte(*byte))
            && opens.is_none_or(|opens| after.iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&opens))
    })
}

/// The text between the braces of the body that follows `from`.
fn body_after(clean: &[u8], from: usize) -> Result<&[u8], String> {
    let open = from + clean[from..].iter().position(|byte| *byte == b'{').ok_or_else(|| format!("no body follows byte {from}"))?;
    Ok(&clean[open + 1..closing(clean, open)?])
}

/// The scope the first `TYPE_*` constant in `text` stands for, or `None` when it holds none.
fn first_static_scope(text: &[u8]) -> Result<Option<StaticScope>, String> {
    let prefix = STATIC_TYPE_PREFIX.as_bytes();
    let Some(start) = (0..text.len()).find(|index| text[*index..].starts_with(prefix) && (*index == 0 || !is_identifier_byte(text[*index - 1]))) else {
        return Ok(None);
    };
    let end = start + text[start..].iter().position(|byte| !is_identifier_byte(*byte)).unwrap_or(text.len() - start);
    let token = String::from_utf8_lossy(&text[start..end]);
    STATIC_SCOPES
        .iter()
        .find(|(name, ..)| *name == token)
        .map(|(_, scope, _)| Some(*scope))
        .ok_or_else(|| format!("{token} is a stillness class this generator does not know"))
}

/// How long the boolean built by `arguments` holds still: the `TYPE_*` constant the constant names
/// itself, or the one the helper class or helper method it is built with passes on.
fn static_scope_of(clean: &[u8], constant: &str, arguments: &[u8]) -> Result<StaticScope, String> {
    if let Some(scope) = first_static_scope(arguments)? {
        return Ok(scope);
    }
    let text = String::from_utf8_lossy(arguments);
    let text = text.trim_start();
    let helper: String =
        text.strip_prefix(NEW_KEYWORD).unwrap_or(text).trim_start().bytes().take_while(|byte| is_identifier_byte(*byte)).map(char::from).collect();
    let declaration = find_declaration(clean, CLASS_KEYWORD, &helper, None)
        .or_else(|| find_declaration(clean, BOOLEAN_HELPER_RETURN_TYPE, &helper, Some(b'(')))
        .ok_or_else(|| format!("{constant} is built with {helper}, which is declared nowhere this generator looks"))?;
    first_static_scope(body_after(clean, declaration)?)?.ok_or_else(|| format!("{constant} is built with {helper}, which names no stillness class"))
}

/// The id an enum constant's first argument spells: a literal or a constant of the constant source.
fn resolve_id(constant: &str, argument: &str, extraction: &Extraction) -> Result<i32, String> {
    let argument = argument.trim();
    let argument = argument.strip_prefix(CONSTANT_QUALIFIER).unwrap_or(argument);
    if let Ok(literal) = argument.parse() {
        return Ok(literal);
    }
    extraction
        .constants
        .iter()
        .find(|entry| entry.name == argument)
        .map(|entry| entry.value)
        .ok_or_else(|| format!("{constant} takes its id from {argument}, which is not a literal or a known constant"))
}

/// Reads the constants of one enum in declaration order.
fn extract_names(space: &'static NameSpace, source: &str, extraction: &Extraction) -> Result<NameTable, String> {
    let clean = without_noise(source);
    let declaration =
        find_declaration(&clean, ENUM_KEYWORD, space.java_enum, Some(b'{')).ok_or_else(|| format!("{} declares no enum {}", space.file, space.java_enum))?;
    let body = body_after(&clean, declaration)?;
    let mut entries: Vec<NameEntry> = Vec::new();
    let mut cursor = 0;
    loop {
        cursor += body[cursor..]
            .iter()
            .position(|byte| !byte.is_ascii_whitespace() && *byte != b',')
            .ok_or_else(|| format!("{} ends without a terminator", space.java_enum))?;
        if body[cursor] == b';' {
            break;
        }
        let name_end = cursor + body[cursor..].iter().position(|byte| !is_identifier_byte(*byte)).unwrap_or(0);
        let name = String::from_utf8_lossy(&body[cursor..name_end]).into_owned();
        let open = name_end + body[name_end..].iter().position(|byte| !byte.is_ascii_whitespace()).unwrap_or(0);
        if name.is_empty() || body[open] != b'(' {
            return Err(format!("{} holds something other than a constant after {:?}", space.java_enum, entries.last().map(|entry| entry.name.as_str())));
        }
        let close = closing(body, open)?;
        let arguments = &body[open + 1..close];
        let mut depth = 0usize;
        let commas: Vec<usize> = arguments
            .iter()
            .enumerate()
            .filter(|(_, byte)| {
                match byte {
                    b'(' | b'{' | b'[' => depth += 1,
                    b')' | b'}' | b']' => depth -= 1,
                    _ => {}
                }
                **byte == b',' && depth == 0
            })
            .map(|(index, _)| index)
            .collect();
        let comma = *commas.first().ok_or_else(|| format!("{name} takes a single argument"))?;
        let id = resolve_id(&name, &String::from_utf8_lossy(&arguments[..comma]), extraction)?;
        let scope = if space.java_enum == BOOLEAN_ENUM { Some(static_scope_of(&clean, &name, &arguments[comma + 1..])?) } else { None };
        if entries.iter().any(|entry| entry.name == name) {
            return Err(format!("{} declares {name} twice", space.java_enum));
        }
        entries.push(NameEntry { name, id, scope, arguments: commas.len() + 1 });
        cursor = close + 1;
        let next = cursor + body[cursor..].iter().position(|byte| !byte.is_ascii_whitespace()).unwrap_or(0);
        if body.get(next) == Some(&b'{') {
            cursor = closing(body, next)? + 1;
        }
    }
    if entries.is_empty() {
        return Err(format!("{} holds no constants", space.java_enum));
    }
    Ok(NameTable { space, entries })
}

/// The ids of one stillness class, in declaration order.
fn static_ids(tables: &[NameTable], scope: StaticScope) -> Vec<i32> {
    tables.iter().flat_map(|table| table.entries.iter()).filter(|entry| entry.scope == Some(scope)).map(|entry| entry.id).collect()
}

/// The ids of one table whose constants carry a writer, in declaration order.
fn writer_ids(table: &NameTable) -> Vec<i32> {
    table.entries.iter().filter(|entry| entry.arguments >= ARGUMENTS_WITH_WRITER).map(|entry| entry.id).collect()
}

/// FNV-1a over the `STEM:name=id` lines of every name table, then the `LIST=id` lines of every
/// stillness list and of every writer list, each in emitted order.
fn name_checksum(tables: &[NameTable]) -> u64 {
    let mut lines: Vec<String> = Vec::new();
    for table in tables {
        lines.extend(table.entries.iter().map(|entry| format!("{}:{}={}\n", table.space.stem, entry.name, entry.id)));
    }
    for (_, scope, list) in STATIC_SCOPES {
        if let Some((list, _)) = list {
            lines.extend(static_ids(tables, *scope).into_iter().map(|id| format!("{list}={id}\n")));
        }
    }
    for table in tables {
        if let Some((list, _)) = table.space.writers {
            lines.extend(writer_ids(table).into_iter().map(|id| format!("{list}={id}\n")));
        }
    }
    let mut hash = FNV_OFFSET_BASIS;
    for byte in lines.into_iter().flat_map(String::into_bytes) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Renders the name module: one table per enum, then the stillness lists.
fn render_names(tables: &[NameTable]) -> String {
    let mut out = String::new();
    out.push_str("//! The names a skin may write in place of a property id, and the options that hold still.\n");
    out.push_str("//!\n");
    out.push_str("//! Generated by `tools/gen-skin-property.rs` from the enums of the reference implementation's\n");
    out.push_str("//! property factories. Do not edit by hand: `name_table_checksum_matches` in\n");
    out.push_str("//! `crates/rbms-skin/tests/property_checksum.rs` re-derives the checksum, and a hand edit fails it.\n");
    for table in tables {
        out.push('\n');
        let _ = writeln!(out, "/// Every `{}` constant as `({} id, name)` in source order.", table.space.java_enum, table.space.describes);
        let _ = writeln!(out, "pub const ALL_{}_NAME: &[(i32, &str)] = &[", table.space.stem);
        for entry in &table.entries {
            let _ = writeln!(out, "    ({}, \"{}\"),", entry.id, entry.name);
        }
        out.push_str("];\n");
    }
    for (constant, scope, list) in STATIC_SCOPES {
        let Some((list, doc)) = list else { continue };
        out.push('\n');
        let _ = writeln!(out, "/// {doc}");
        let _ = writeln!(out, "///");
        let _ = writeln!(out, "/// These are the `BooleanType` constants built with `{constant}`, in source order.");
        let _ = writeln!(out, "pub const {list}: &[i32] = &[");
        for id in static_ids(tables, *scope) {
            let _ = writeln!(out, "    {id},");
        }
        out.push_str("];\n");
    }
    for table in tables {
        let Some((list, doc)) = table.space.writers else { continue };
        out.push('\n');
        let _ = writeln!(out, "/// {doc}");
        let _ = writeln!(out, "///");
        let _ = writeln!(out, "/// These are the `{}` constants declared with a writer, in source order.", table.space.java_enum);
        let _ = writeln!(out, "pub const {list}: &[i32] = &[");
        for id in writer_ids(table) {
            let _ = writeln!(out, "    {id},");
        }
        out.push_str("];\n");
    }
    out
}

/// Renders one group module: its constants, then the index array that names them.
fn render_group(group: &Group, extraction: &Extraction) -> String {
    let mut out = String::new();
    for line in group.doc {
        let _ = writeln!(out, "//!{}{line}", if line.is_empty() { "" } else { " " });
    }
    out.push_str("//!\n");
    out.push_str("//! Generated by `tools/gen-skin-property.rs` from the reference implementation's `SkinProperty.java`.\n");
    out.push_str("//! Do not edit by hand: `property_table_checksum_matches` in\n");
    out.push_str("//! `crates/rbms-skin/tests/property_checksum.rs` re-derives the table checksum, and a hand edit\n");
    out.push_str("//! fails it.\n");
    for prefix in group.prefixes {
        let entries = extraction.by_prefix(prefix);
        out.push('\n');
        for entry in &entries {
            let _ = writeln!(out, "pub const {}: i32 = {};", entry.name, entry.value);
        }
        out.push('\n');
        let _ = writeln!(out, "/// Every extracted `{prefix}_*` declaration as `(id, name)` in source order.");
        let _ = writeln!(out, "pub const ALL_{prefix}: &[(i32, &str)] = &[");
        for entry in &entries {
            let _ = writeln!(out, "    ({}, \"{}\"),", entry.name, entry.name);
        }
        out.push_str("];\n");
    }
    out
}

/// Renders the module root: the submodule declarations, the per-prefix counts and the checksum.
fn render_root(extraction: &Extraction, tables: &[NameTable]) -> String {
    let mut out = String::new();
    out.push_str("//! The generated property constant table, extracted from the reference implementation's\n");
    out.push_str("//! `SkinProperty.java`.\n");
    out.push_str("//!\n");
    out.push_str("//! Generated by `tools/gen-skin-property.rs` and committed as-is; never edited by hand. The\n");
    out.push_str("//! submodules are split by the registry kind that reads them, and every constant is re-exported\n");
    out.push_str("//! here so a caller names one module rather than five.\n");
    out.push_str("//!\n");
    out.push_str("//! `TIMER_*` lives in [`crate::timer`] instead, generated by its own tool. The two tables\n");
    out.push_str("//! together account for every declaration in the source, which\n");
    out.push_str("//! `property_table_covers_every_declaration` checks against [`REFERENCE_CONSTANT_COUNT`].\n\n");
    let mut modules: Vec<&str> = GROUPS.iter().map(|group| group.module).chain(std::iter::once(NAMES_MODULE)).collect();
    modules.sort_unstable();
    for module in &modules {
        let _ = writeln!(out, "pub mod {module};");
    }
    out.push('\n');
    for module in &modules {
        let _ = writeln!(out, "pub use {module}::*;");
    }
    out.push('\n');
    for prefix in known_prefixes() {
        let count = extraction.by_prefix(prefix).len();
        let _ = writeln!(out, "/// How many `{prefix}_*` declarations the reference source holds.");
        let _ = writeln!(out, "pub const {prefix}_CONSTANT_COUNT: usize = {count};\n");
    }
    out.push_str("/// How many non-timer declarations the reference source holds, the sum of the per-prefix counts.\n");
    let _ = writeln!(out, "pub const PROPERTY_CONSTANT_COUNT: usize = {};\n", extraction.entries.len());
    out.push_str("/// How many `public static final int` declarations the reference source holds in total, timers\n");
    out.push_str("/// included. [`PROPERTY_CONSTANT_COUNT`] plus [`crate::timer::TIMER_CONSTANT_COUNT`] must equal it.\n");
    let _ = writeln!(out, "pub const REFERENCE_CONSTANT_COUNT: usize = {};\n", extraction.total_declarations());
    out.push_str("/// Every emitted table with its prefix, in the order [`PROPERTY_TABLE_CHECKSUM`] hashes them.\n");
    out.push_str("pub const ALL_PROPERTY_TABLES: &[(&str, &[(i32, &str)])] = &[\n");
    for prefix in known_prefixes() {
        let _ = writeln!(out, "    (\"{prefix}\", ALL_{prefix}),");
    }
    out.push_str("];\n\n");
    out.push_str("/// FNV-1a 64 over the `NAME=VALUE` lines of [`ALL_PROPERTY_TABLES`], table by table and in\n");
    out.push_str("/// order within each.\n");
    let _ = writeln!(out, "pub const PROPERTY_TABLE_CHECKSUM: u64 = {:#018x};", checksum(extraction));
    for table in tables {
        let _ = writeln!(out, "\n/// How many constants the reference's `{}` enum holds.", table.space.java_enum);
        let _ = writeln!(out, "pub const {}_NAME_COUNT: usize = {};", table.space.stem, table.entries.len());
    }
    out.push_str("\n/// Every emitted name table with its stem, in the order [`NAME_TABLE_CHECKSUM`] hashes them.\n");
    out.push_str("pub const ALL_NAME_TABLES: &[(&str, &[(i32, &str)])] = &[\n");
    for table in tables {
        let _ = writeln!(out, "    (\"{stem}\", ALL_{stem}_NAME),", stem = table.space.stem);
    }
    out.push_str("];\n\n");
    out.push_str("/// Every emitted stillness list with its name, in the order [`NAME_TABLE_CHECKSUM`] hashes them.\n");
    out.push_str("pub const ALL_STATIC_LISTS: &[(&str, &[i32])] = &[\n");
    for (_, _, list) in STATIC_SCOPES {
        if let Some((list, _)) = list {
            let _ = writeln!(out, "    (\"{list}\", {list}),");
        }
    }
    out.push_str("];\n\n");
    out.push_str("/// Every emitted writer list with its name, in the order [`NAME_TABLE_CHECKSUM`] hashes them.\n");
    out.push_str("pub const ALL_WRITER_LISTS: &[(&str, &[i32])] = &[\n");
    for table in tables {
        if let Some((list, _)) = table.space.writers {
            let _ = writeln!(out, "    (\"{list}\", {list}),");
        }
    }
    out.push_str("];\n\n");
    out.push_str("/// FNV-1a 64 over the `STEM:name=id` lines of [`ALL_NAME_TABLES`] followed by the `LIST=id` lines\n");
    out.push_str("/// of [`ALL_STATIC_LISTS`] and of [`ALL_WRITER_LISTS`], each in order.\n");
    let _ = writeln!(out, "pub const NAME_TABLE_CHECKSUM: u64 = {:#018x};", name_checksum(tables));
    out
}

/// Writes one generated file, reporting the path so a run shows what it touched.
fn write(path: &std::path::Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, contents).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

/// Writes the module root and every group module under `dir`.
fn emit(dir: &std::path::Path, extraction: &Extraction, tables: &[NameTable]) -> Result<(), String> {
    write(&dir.join("generated.rs"), &render_root(extraction, tables))?;
    for group in GROUPS {
        write(&dir.join("generated").join(format!("{}.rs", group.module)), &render_group(group, extraction))?;
    }
    write(&dir.join("generated").join(format!("{NAMES_MODULE}.rs")), &render_names(tables))
}

/// Reads every name enum from the factories beside the constant source.
fn extract_all_names(input: &std::path::Path, extraction: &Extraction) -> Result<Vec<NameTable>, String> {
    let factories = input.parent().unwrap_or_else(|| std::path::Path::new("")).join(FACTORY_DIRECTORY);
    NAME_SPACES
        .iter()
        .map(|space| {
            let path = factories.join(space.file);
            let source = std::fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            extract_names(space, &source, extraction)
        })
        .collect()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(input) = args.next() else {
        eprintln!("usage: gen-skin-property <SkinProperty.java> <output directory>");
        std::process::exit(EXIT_USAGE);
    };
    let Some(output) = args.next() else {
        eprintln!("usage: gen-skin-property <SkinProperty.java> <output directory>");
        std::process::exit(EXIT_USAGE);
    };
    let source = match std::fs::read_to_string(&input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {input}: {error}");
            std::process::exit(EXIT_FAILURE);
        }
    };
    let extraction = match extract(&source) {
        Ok(extraction) => extraction,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(EXIT_FAILURE);
        }
    };
    for prefix in known_prefixes() {
        eprintln!("{prefix}: {}", extraction.by_prefix(prefix).len());
    }
    eprintln!(
        "extracted {} property declarations ({} timers left to the timer generator, {} total), checksum {:#018x}",
        extraction.entries.len(),
        extraction.timer_declarations,
        extraction.total_declarations(),
        checksum(&extraction)
    );
    let tables = match extract_all_names(std::path::Path::new(&input), &extraction) {
        Ok(tables) => tables,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(EXIT_FAILURE);
        }
    };
    for table in &tables {
        eprintln!("{}: {} names", table.space.java_enum, table.entries.len());
    }
    eprintln!("name checksum {:#018x}", name_checksum(&tables));
    if let Err(error) = emit(std::path::Path::new(&output), &extraction, &tables) {
        eprintln!("{error}");
        std::process::exit(EXIT_FAILURE);
    }
}

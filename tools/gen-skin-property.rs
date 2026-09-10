//! Generates `crates/rbms-skin/src/property/generated.rs` and its submodules from the reference
//! implementation's `SkinProperty.java`.
//!
//! The reference tree is not part of this repository, so this runs by hand and its output is
//! committed. Build, run and format it with:
//!
//! ```text
//! rustc -O tools/gen-skin-property.rs -o target/gen-skin-property
//! target/gen-skin-property <path to SkinProperty.java> crates/rbms-skin/src/property
//! cargo fmt --all
//! ```
//!
//! It extracts every `public static final int <PREFIX>_<NAME> = <n>;` declaration in source order
//! and emits one `pub const` per declaration plus an `ALL_<PREFIX>` index array whose entries name
//! those constants, so the array and the constants cannot drift apart. `TIMER_*` belongs to
//! `tools/gen-skin-timer.rs`; this generator only counts those declarations, so the two committed
//! tables together account for every declaration in the source.
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
    let mut timer_declarations = 0usize;
    for line in source.lines() {
        let Some(rest) = line.trim_start().strip_prefix(DECLARATION) else { continue };
        let Some((name, value)) = rest.split_once('=') else { continue };
        let name = name.trim();
        if name.starts_with(TIMER_PREFIX) {
            timer_declarations += 1;
            continue;
        }
        let value = value.trim().trim_end_matches(';').trim();
        let value: i32 = value.parse().map_err(|_| format!("{name} has a non-literal value {value}"))?;
        if prefix_of(name).is_none() {
            return Err(format!("{name} carries a prefix this generator does not place; add it to GROUPS"));
        }
        if let Some(previous) = entries.iter().find(|entry| entry.name == name) {
            return Err(format!("{} is declared twice ({} then {})", name, previous.value, value));
        }
        entries.push(Entry { name: name.to_owned(), value });
    }
    if entries.is_empty() {
        return Err("no property declarations found".to_owned());
    }
    if timer_declarations == 0 {
        return Err("no timer declarations found; the input is not the expected source".to_owned());
    }
    Ok(Extraction { entries, timer_declarations })
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
fn render_root(extraction: &Extraction) -> String {
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
    let mut modules: Vec<&str> = GROUPS.iter().map(|group| group.module).collect();
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
fn emit(dir: &std::path::Path, extraction: &Extraction) -> Result<(), String> {
    write(&dir.join("generated.rs"), &render_root(extraction))?;
    for group in GROUPS {
        write(&dir.join("generated").join(format!("{}.rs", group.module)), &render_group(group, extraction))?;
    }
    Ok(())
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
    if let Err(error) = emit(std::path::Path::new(&output), &extraction) {
        eprintln!("{error}");
        std::process::exit(EXIT_FAILURE);
    }
}

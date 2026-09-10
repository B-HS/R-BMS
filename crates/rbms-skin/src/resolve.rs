//! Turning the path patterns a document writes into files on disk.
//!
//! A document rarely names a file outright. It writes a wildcard (`gauge/*.png`), and the player
//! either substitutes the name the user picked or draws one at random from the directory. That is
//! `SkinLoader.getPath` and the customfile machinery around it, reproduced here with two additions
//! the reference does not have: every candidate list is sorted so a seeded draw is reproducible,
//! and every resolved path is checked to be inside the skin root before it is opened.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use crate::SkinError;
use crate::loader::SkinUserConfig;
use crate::model::SkinDef;

/// The character a pattern stands a variable file name on.
pub const WILDCARD: char = '*';

/// The character that splits a pattern's extension into a leading and a trailing half, letting a
/// document match `*_pressed.png` style suffixes.
pub const EXTENSION_JOIN: char = '|';

/// The value a user configuration stores to mean "draw one at load time".
pub const RANDOM_SELECTION: &str = "Random";

/// The separator patterns are written with, whatever the host platform uses.
const PATTERN_SEPARATOR: char = '/';

/// SplitMix64's odd increment, the constant its author specifies.
const SPLITMIX_GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;

/// SplitMix64's first mixing multiplier.
const SPLITMIX_MIX_A: u64 = 0xbf58_476d_1ce4_e5b9;

/// SplitMix64's second mixing multiplier.
const SPLITMIX_MIX_B: u64 = 0x94d0_49bb_1331_11eb;

/// Bits shifted out in SplitMix64's first and third xor-shift.
const SPLITMIX_SHIFT_WIDE: u32 = 30;

/// Bits shifted out in SplitMix64's second xor-shift.
const SPLITMIX_SHIFT_NARROW: u32 = 27;

/// Bits shifted out in SplitMix64's final xor-shift.
const SPLITMIX_SHIFT_FINAL: u32 = 31;

/// The environment variable that pins wildcard draws, for tests and golden runs.
pub const SEED_ENV: &str = "RBMS_SKIN_SEED";

/// A tiny reproducible generator, so a wildcard resolves to the same file for the same seed.
///
/// SplitMix64: the crate carries no random-number dependency, and a load draws a handful of indices
/// at most.
#[derive(Debug, Clone)]
pub struct Draw {
    state: u64,
}

impl Draw {
    /// A generator started from `seed`.
    pub const fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The generator a load uses when the caller pins no seed: [`SEED_ENV`] if it holds a number,
    /// and otherwise the wall clock, which is what makes an unpinned draw random at all.
    pub fn from_environment() -> Self {
        if let Ok(value) = std::env::var(SEED_ENV)
            && let Ok(seed) = value.trim().parse::<u64>()
        {
            return Self::from_seed(seed);
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|elapsed| elapsed.as_nanos() as u64).unwrap_or_default();
        Self::from_seed(now)
    }

    /// The next value in the sequence.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> SPLITMIX_SHIFT_WIDE)).wrapping_mul(SPLITMIX_MIX_A);
        z = (z ^ (z >> SPLITMIX_SHIFT_NARROW)).wrapping_mul(SPLITMIX_MIX_B);
        z ^ (z >> SPLITMIX_SHIFT_FINAL)
    }

    /// An index below `len`, or `None` when there is nothing to pick from.
    pub fn index(&mut self, len: usize) -> Option<usize> {
        (len > 0).then(|| (self.next_u64() % len as u64) as usize)
    }
}

/// A file slot the document offers the player, and the names it may be filled with.
///
/// Built once per load from the document's `filepath` list; the configuration screen reads it
/// rather than re-scanning directories every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomFile {
    /// The heading the configuration screen groups this slot under, empty when the document gave
    /// none.
    pub category: String,
    pub name: String,
    /// The wildcard pattern, rooted at the skin directory. This is the filemap key.
    pub pattern: String,
    /// The document's own suggestion, used when the player has chosen nothing.
    pub default: Option<String>,
    /// File names the pattern expands to, [`RANDOM_SELECTION`] first.
    pub candidates: Vec<String>,
}

/// Joins a document-relative path onto the skin directory, in the slash-separated form patterns and
/// filemap keys are both written in.
pub fn pattern_for(skin_dir: &Path, relative: &str) -> String {
    let base = skin_dir.to_string_lossy().replace('\\', "/");
    let relative = relative.replace('\\', "/");
    let base = base.trim_end_matches(PATTERN_SEPARATOR);
    if relative.is_empty() { base.to_owned() } else { format!("{base}{PATTERN_SEPARATOR}{relative}") }
}

/// The suffix a wildcard pattern accepts, following `SkinLoader.getPath`.
///
/// A plain `dir/*.png` accepts `.png`. A split pattern `dir/*_off|.png` accepts the text between the
/// wildcard and the first [`EXTENSION_JOIN`], with whatever follows the last one appended, which is
/// how a document matches a name's middle as well as its end.
pub fn wildcard_extension(pattern: &str) -> Option<String> {
    let star = pattern.rfind(WILDCARD)? + WILDCARD.len_utf8();
    let Some(first_join) = pattern.find(EXTENSION_JOIN) else {
        return Some(pattern[star..].to_owned());
    };
    if first_join < star {
        return Some(pattern[star..].to_owned());
    }
    let last_join = pattern.rfind(EXTENSION_JOIN).unwrap_or(first_join) + EXTENSION_JOIN.len_utf8();
    let head = &pattern[star..first_join];
    if last_join < pattern.len() { Some(format!("{head}{}", &pattern[last_join..])) } else { Some(head.to_owned()) }
}

/// Substitutes a chosen file name into a pattern, following `SkinLoader.getPath`'s first step.
///
/// A filemap key is a whole pattern, so the match is a prefix test; the text up to the wildcard is
/// kept, the chosen name takes the wildcard's place, and whatever the pattern had past the key is
/// appended. The longest matching key wins, which the reference leaves to hash order.
pub fn apply_filemap(pattern: &str, filemap: &BTreeMap<String, String>) -> Option<String> {
    let star = pattern.rfind(WILDCARD)?;
    let (key, name) = filemap.iter().filter(|(key, _)| pattern.starts_with(key.as_str())).max_by_key(|(key, _)| key.len())?;
    Some(format!("{}{}{}", &pattern[..star], name, &pattern[key.len()..]))
}

/// The directory part of a pattern, or `None` when it names no directory.
fn pattern_directory(pattern: &str) -> Option<&str> {
    pattern.rfind(PATTERN_SEPARATOR).map(|slash| &pattern[..slash])
}

/// The file names in `dir` whose path ends with `ext`, sorted.
///
/// The reference lower-cases the path but not the suffix, so an upper-case suffix matches nothing;
/// that is kept. Sorting is not the reference's -- directory order there is the filesystem's -- and
/// is what makes a seeded draw reproducible across machines.
fn scan_candidates(dir: &Path, ext: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter(|entry| entry.path().to_string_lossy().to_lowercase().ends_with(ext))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// Rewrites a path lexically, dropping `.` and applying `..`, without touching the filesystem.
///
/// `None` means the path climbed above its own start, which is a traversal attempt whether or not
/// the target exists.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                out.pop();
            }
            Component::Normal(part) => {
                depth += 1;
                out.push(part);
            }
        }
    }
    Some(out)
}

/// The canonical form of the deepest existing ancestor of `path`.
///
/// A pattern may resolve to a file that is not there yet, so the check has to stop where the
/// filesystem stops; symlinks are followed for every part that does exist.
fn canonical_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    loop {
        if let Ok(real) = current.canonicalize() {
            return Some(real);
        }
        current = current.parent()?;
    }
}

/// Accepts a resolved path only if it stays inside the skin root.
///
/// This has no counterpart in the reference. A skin is third-party data a player downloads, so a
/// document must not be able to name `../../.ssh/id_rsa` and have the player draw it; `..`
/// segments and symlinks that leave the tree are both refused.
pub fn contained(root: &Path, candidate: &Path) -> Result<PathBuf, SkinError> {
    let escape = || SkinError::PathEscape(candidate.to_string_lossy().into_owned());
    let normalized = normalize(candidate).ok_or_else(escape)?;
    let root_real = root.canonicalize().map_err(SkinError::Read)?;
    let real = canonical_existing_ancestor(&normalized).ok_or_else(escape)?;
    if real.starts_with(&root_real) { Ok(normalized) } else { Err(escape()) }
}

/// Resolves the patterns of one skin load, remembering what each one resolved to.
///
/// The cache is the reference's per-load behaviour: a pattern drawn once keeps its file for the
/// rest of the load, so two objects sharing a wildcard share an image.
#[derive(Debug)]
pub struct FileResolver {
    root: PathBuf,
    filemap: BTreeMap<String, String>,
    draw: Draw,
    cache: BTreeMap<String, PathBuf>,
}

impl FileResolver {
    /// A resolver over `root`, substituting `filemap` and drawing wildcards with `draw`.
    pub fn new(root: &Path, filemap: BTreeMap<String, String>, draw: Draw) -> Self {
        Self { root: root.to_path_buf(), filemap, draw, cache: BTreeMap::new() }
    }

    /// The filemap this resolver substitutes.
    pub fn filemap(&self) -> &BTreeMap<String, String> {
        &self.filemap
    }

    /// The file a pattern names, following `SkinLoader.getPath`.
    ///
    /// A filemap hit is substituted and returned; otherwise a wildcard is expanded and one
    /// candidate drawn. A pattern that expands to nothing is returned as written, so the caller
    /// reports one missing file rather than failing the whole document.
    pub fn resolve(&mut self, pattern: &str) -> Result<PathBuf, SkinError> {
        if let Some(hit) = self.cache.get(pattern) {
            return Ok(hit.clone());
        }
        let resolved = self.resolve_uncached(pattern)?;
        self.cache.insert(pattern.to_owned(), resolved.clone());
        Ok(resolved)
    }

    /// [`Self::resolve`] without the cache, so the cache stores only accepted paths.
    fn resolve_uncached(&mut self, pattern: &str) -> Result<PathBuf, SkinError> {
        if let Some(substituted) = apply_filemap(pattern, &self.filemap) {
            return contained(&self.root, Path::new(&substituted));
        }
        let Some(ext) = wildcard_extension(pattern) else {
            return contained(&self.root, Path::new(pattern));
        };
        let Some(directory) = pattern_directory(pattern) else {
            return contained(&self.root, Path::new(pattern));
        };
        let directory = contained(&self.root, Path::new(directory))?;
        let candidates = scan_candidates(&directory, &ext);
        match self.draw.index(candidates.len()) {
            Some(index) => contained(&self.root, &directory.join(&candidates[index])),
            None => contained(&self.root, Path::new(pattern)),
        }
    }
}

/// The file slots a document offers, read once per load.
///
/// Each `filepath` entry becomes a pattern rooted at the skin directory, which is the key the
/// filemap is later built under, and the directory behind it is scanned once for the names the
/// configuration screen cycles through.
pub fn enumerate_custom_files(def: &SkinDef, skin_dir: &Path, root: &Path) -> Vec<CustomFile> {
    def.filepath
        .iter()
        .map(|entry| {
            let pattern = pattern_for(skin_dir, &entry.path);
            let mut candidates = vec![RANDOM_SELECTION.to_owned()];
            if let Some(ext) = wildcard_extension(&pattern)
                && let Some(directory) = pattern_directory(&pattern)
                && let Ok(directory) = contained(root, Path::new(directory))
            {
                candidates.extend(scan_candidates(&directory, &ext));
            }
            CustomFile { category: entry.category.clone(), name: entry.name.clone(), pattern, default: entry.def.clone(), candidates }
        })
        .collect()
}

/// The names a custom file may actually be filled with, without the [`RANDOM_SELECTION`] entry.
fn selectable(file: &CustomFile) -> &[String] {
    match file.candidates.first().map(String::as_str) {
        Some(RANDOM_SELECTION) => &file.candidates[1..],
        _ => &file.candidates,
    }
}

/// Builds the filemap a load substitutes with, following `JSONSkinLoader`'s header pass.
///
/// A player's choice wins; the document's `def` fills in for a slot nobody has touched; and
/// [`RANDOM_SELECTION`] draws one of the candidates with `draw`. A slot that resolves to nothing
/// contributes no entry, which leaves its pattern to the wildcard path in [`FileResolver::resolve`].
pub fn build_filemap(files: &[CustomFile], user: &SkinUserConfig, draw: &mut Draw) -> BTreeMap<String, String> {
    let mut filemap = BTreeMap::new();
    for file in files {
        let chosen = user.filepaths.get(&file.name).map(String::as_str).or(file.default.as_deref());
        let selected = match chosen {
            Some(RANDOM_SELECTION) | None => selectable(file).get(draw.index(selectable(file).len()).unwrap_or_default()).cloned(),
            Some(name) => Some(name.to_owned()),
        };
        if let Some(selected) = selected {
            filemap.insert(file.pattern.clone(), selected);
        }
    }
    filemap
}

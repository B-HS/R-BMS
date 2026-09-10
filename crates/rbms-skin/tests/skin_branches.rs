//! Guarded clauses and includes: the two document features that run before anything is deserialised.
//!
//! A single-record field written as a list is a chain of guarded clauses and one of them survives; a
//! list-typed field reads its elements one at a time, and an element may carry a condition or name
//! another file. The two rules are different in the reference implementation
//! (`JsonSkinSerializer`'s `ObjectSerializer` and `ArraySerializer`) and reading one as the other
//! loses records, so both are pinned here.
//!
//! The budgets are this player's own: the reference bounds neither how deep includes nest nor how
//! many of them one document expands, and the loader runs on the frame loop.

use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::loader::{MAX_INCLUDE_DEPTH, MAX_INCLUDE_EXPANSIONS, SkinLoadOptions, SkinUserConfig, load_skin};

/// A seed every wildcard test pins, so a draw is the same on every machine.
const TEST_SEED: u64 = 7;

/// A scratch directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-branch-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Writes one document into the scratch directory and returns its path.
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).expect("the document should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Options over `root` with the wildcard draw pinned.
fn seeded<'a>(root: &'a Path, user: &'a SkinUserConfig) -> SkinLoadOptions<'a> {
    let mut options = SkinLoadOptions::new(root, user, Mode::BEAT_7K);
    options.rng_seed = Some(TEST_SEED);
    options
}

/// The ids of a loaded document's destinations, in draw order.
fn ids(skin: &rbms_skin::loader::LoadedSkin) -> Vec<String> {
    skin.destinations.iter().map(|named| named.id.clone()).collect()
}

/// A list-typed field reads its elements one at a time (`JsonSkinSerializer.ArraySerializer`): an
/// element carrying a condition alongside a payload keeps or drops just itself and everything else
/// stays where it is. Reading the whole list as a chain of guarded clauses instead -- which is what
/// a single-record field does -- would collapse a list of objects to whichever one matched first,
/// and a document that wrote its background and one optional panel that way would lose the
/// background.
#[test]
fn a_conditional_element_of_a_list_keeps_its_place_rather_than_replacing_the_list() {
    let scratch = Scratch::new("array-elements");
    let path = scratch.write(
        "skin.json",
        r#"{
            "type": 5,
            "property": [{ "category": "layout", "name": "Panel", "item": [{ "name": "on", "op": 901 }, { "name": "off", "op": 902 }], "def": "on" }],
            "destination": [
                { "id": "bg", "dst": [{ "time": 0 }] },
                { "if": [901], "value": { "id": "panel", "dst": [{ "time": 0 }] } },
                { "if": [902], "value": { "id": "other", "dst": [{ "time": 0 }] } },
                { "id": "footer", "dst": [{ "time": 0 }] }
            ]
        }"#,
    );
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document loads");

    assert_eq!(ids(&skin), vec!["bg", "panel", "footer"], "each element is kept or dropped on its own, in order");
}

/// The same element may carry several payloads under `values`, which are spread into the list where
/// the element stood.
#[test]
fn a_conditional_element_may_bring_several_records_at_once() {
    let scratch = Scratch::new("array-values");
    let path = scratch.write(
        "skin.json",
        r#"{
            "type": 5,
            "property": [{ "category": "layout", "name": "Panel", "item": [{ "name": "on", "op": 901 }], "def": "on" }],
            "destination": [
                { "id": "bg", "dst": [{ "time": 0 }] },
                { "if": [901], "values": [{ "id": "one", "dst": [{ "time": 0 }] }, { "id": "two", "dst": [{ "time": 0 }] }] }
            ]
        }"#,
    );
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document loads");

    assert_eq!(ids(&skin), vec!["bg", "one", "two"]);
}

/// An include standing in a list brings a list, and its records are spread into the parent where the
/// include stood (`JsonSkinSerializer.includeArray`). Substituting the list whole would leave a list
/// inside a list, which the mirror cannot read at all -- the whole document would be refused.
#[test]
fn an_include_that_holds_a_list_is_spread_into_the_list_it_stands_in() {
    let scratch = Scratch::new("include-array");
    scratch.write("parts.json", r#"[{ "id": "first", "dst": [{ "time": 0 }] }, { "id": "second", "dst": [{ "time": 0 }] }]"#);
    let path = scratch.write(
        "skin.json",
        r#"{ "type": 5, "destination": [{ "id": "bg", "dst": [{ "time": 0 }] }, { "include": "parts.json" }, { "id": "footer", "dst": [{ "time": 0 }] }] }"#,
    );
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document loads");

    assert_eq!(ids(&skin), vec!["bg", "first", "second", "footer"]);
}

/// The nesting limit bounds how deep includes go and says nothing about how wide they get: a file
/// that includes several files at each of several levels stays inside the depth limit and still
/// expands exponentially. A few kilobytes of documents would then hold the loader -- which runs on
/// the frame loop when a skin is chosen -- for as long as the author liked.
#[test]
fn a_document_that_expands_more_includes_than_its_budget_stops_expanding() {
    let scratch = Scratch::new("include-fanout");
    let fanout = 6;
    scratch.write("level0.json", r#"[{ "id": "leaf", "dst": [{ "time": 0 }] }]"#);
    for level in 1..MAX_INCLUDE_DEPTH {
        let children: Vec<String> = (0..fanout).map(|_| format!(r#"{{ "include": "level{}.json" }}"#, level - 1)).collect();
        scratch.write(&format!("level{level}.json"), &format!("[{}]", children.join(",")));
    }
    let path = scratch.write("skin.json", &format!(r#"{{ "type": 5, "destination": [{{ "include": "level{}.json" }}] }}"#, MAX_INCLUDE_DEPTH - 1));

    let user = SkinUserConfig::default();
    let started = std::time::Instant::now();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("a document over its include budget still loads");
    let spent = started.elapsed();

    assert!(spent < std::time::Duration::from_secs(2), "the expansion was not bounded: {spent:?}");
    assert!(skin.destinations.len() <= MAX_INCLUDE_EXPANSIONS, "no more records than the budget allowed: {}", skin.destinations.len());
    assert!(skin.warnings.iter().any(|warning| warning.contains("include")), "the document is told its includes were cut short: {:?}", skin.warnings);
}

/// A part file named from several places is read and parsed once, which is what makes an ordinary
/// skin cheaper rather than only bounding a hostile one.
#[test]
fn an_include_named_twice_is_read_once() {
    let scratch = Scratch::new("include-cache");
    scratch.write("part.json", r#"[{ "id": "part", "dst": [{ "time": 0 }] }]"#);
    let path = scratch.write("skin.json", r#"{ "type": 5, "destination": [{ "include": "part.json" }, { "include": "part.json" }] }"#);
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document loads");

    assert_eq!(ids(&skin), vec!["part", "part"], "reading it once must still put it in both places");
}

//! File resolution: the filemap substitution and wildcard expansion a document's paths go through,
//! the customfile slots those wildcards come from, and the containment rule that keeps every
//! resolved path inside the skin root.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rbms_skin::SkinError;
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_header};
use rbms_skin::model::{Filepath, SkinDef};
use rbms_skin::resolve::{Draw, FileResolver, apply_filemap, build_filemap, contained, enumerate_custom_files, pattern_for, wildcard_extension};

/// A seed the wildcard tests pin so a draw is the same on every machine.
const TEST_SEED: u64 = 7;

/// The fixtures directory this file reads from.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

/// The minimal fixture document's own directory, which is also its root.
fn minimal_root() -> PathBuf {
    fixtures().join("minimal")
}

/// A filemap from a list of pattern-to-name pairs.
fn filemap(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect()
}

/// A scratch directory that removes itself, for the tests that need real files.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-skin-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_plain_wildcard_accepts_its_extension() {
    assert_eq!(wildcard_extension("gauge/*.png").as_deref(), Some(".png"));
}

#[test]
fn a_split_wildcard_joins_both_halves() {
    assert_eq!(wildcard_extension("parts/*_off|.png").as_deref(), Some("_off.png"));
}

#[test]
fn a_split_wildcard_ending_at_the_bar_keeps_only_the_head() {
    assert_eq!(wildcard_extension("parts/*_off|").as_deref(), Some("_off"));
}

#[test]
fn a_pattern_without_a_wildcard_has_no_extension() {
    assert_eq!(wildcard_extension("images/frame.png"), None);
}

#[test]
fn a_bar_before_the_wildcard_does_not_invert_the_slice() {
    assert_eq!(wildcard_extension("odd|name/*.png").as_deref(), Some(".png"));
}

#[test]
fn a_filemap_key_equal_to_the_pattern_substitutes_the_name() {
    let map = filemap(&[("skin/gauge/*.png", "groove.png")]);
    assert_eq!(apply_filemap("skin/gauge/*.png", &map).as_deref(), Some("skin/gauge/groove.png"));
}

#[test]
fn a_filemap_key_that_is_a_prefix_keeps_the_pattern_tail() {
    let map = filemap(&[("skin/gauge/*", "groove")]);
    assert_eq!(apply_filemap("skin/gauge/*.png", &map).as_deref(), Some("skin/gauge/groove.png"));
}

#[test]
fn a_pattern_no_key_matches_is_left_alone() {
    let map = filemap(&[("skin/other/*.png", "groove.png")]);
    assert_eq!(apply_filemap("skin/gauge/*.png", &map), None);
}

#[test]
fn the_longest_matching_filemap_key_wins() {
    let map = filemap(&[("skin/", "short"), ("skin/gauge/*.png", "groove.png")]);
    assert_eq!(apply_filemap("skin/gauge/*.png", &map).as_deref(), Some("skin/gauge/groove.png"));
}

#[test]
fn a_pattern_without_a_wildcard_is_never_substituted() {
    let map = filemap(&[("images/frame.png", "other.png")]);
    assert_eq!(apply_filemap("images/frame.png", &map), None);
}

#[test]
fn a_path_inside_the_root_is_accepted() {
    let root = minimal_root();
    let accepted = contained(&root, &root.join("images/frame.png")).expect("a file inside the root is fine");
    assert!(accepted.ends_with("frame.png"));
}

#[test]
fn a_path_that_does_not_exist_yet_is_accepted_while_it_stays_inside() {
    let root = minimal_root();
    assert!(contained(&root, &root.join("images/not-there.png")).is_ok());
}

#[test]
fn a_parent_traversal_is_refused() {
    let root = minimal_root();
    let outcome = contained(&root, &root.join("../secret.txt"));
    assert!(matches!(outcome, Err(SkinError::PathEscape(_))), "a `..` escape must be refused, got {outcome:?}");
}

#[test]
fn a_traversal_that_climbs_above_its_own_start_is_refused() {
    let root = minimal_root();
    assert!(matches!(contained(&root, Path::new("../../etc/passwd")), Err(SkinError::PathEscape(_))));
}

#[test]
fn an_absolute_path_outside_the_root_is_refused() {
    let root = minimal_root();
    let outside = fixtures().join("secret.txt");
    assert!(matches!(contained(&root, &outside), Err(SkinError::PathEscape(_))));
}

#[cfg(unix)]
#[test]
fn a_symlink_that_leaves_the_root_is_refused() {
    let scratch = Scratch::new("symlink");
    let root = scratch.path().join("root");
    std::fs::create_dir_all(&root).expect("the root should be creatable");
    let outside = scratch.path().join("outside.txt");
    std::fs::write(&outside, "secret").expect("the outside file should be writable");
    let link = root.join("link.txt");
    std::os::unix::fs::symlink(&outside, &link).expect("the symlink should be creatable");

    assert!(matches!(contained(&root, &link), Err(SkinError::PathEscape(_))), "a symlink out of the root must be refused");
}

#[test]
fn a_draw_repeats_itself_for_the_same_seed() {
    let mut first = Draw::from_seed(TEST_SEED);
    let mut second = Draw::from_seed(TEST_SEED);
    let taken: Vec<u64> = (0..4).map(|_| first.next_u64()).collect();
    let repeated: Vec<u64> = (0..4).map(|_| second.next_u64()).collect();
    assert_eq!(taken, repeated);
}

#[test]
fn a_draw_over_nothing_picks_nothing() {
    assert_eq!(Draw::from_seed(TEST_SEED).index(0), None);
}

#[test]
fn custom_files_carry_the_directory_scan_and_the_random_entry() {
    let document = SkinDef {
        filepath: vec![Filepath { category: "layout".to_owned(), name: "Gauge".to_owned(), path: "gauge/*.png".to_owned(), def: Some("Random".to_owned()) }],
        ..SkinDef::default()
    };
    let root = minimal_root();
    let files = enumerate_custom_files(&document, &root, &root);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "Gauge");
    assert_eq!(files[0].category, "layout", "the heading the document grouped the slot under is dropped");
    assert!(files[0].pattern.ends_with("gauge/*.png"), "the pattern is rooted at the skin directory: {}", files[0].pattern);
    assert_eq!(files[0].candidates, vec!["Random", "groove.png", "hard.png", "shell.png"]);
}

#[test]
fn a_custom_file_whose_pattern_matches_nothing_offers_only_random() {
    let document = SkinDef {
        filepath: vec![Filepath { category: String::new(), name: "Nothing".to_owned(), path: "gauge/*.jpg".to_owned(), def: None }],
        ..SkinDef::default()
    };
    let root = minimal_root();
    let files = enumerate_custom_files(&document, &root, &root);
    assert_eq!(files[0].candidates, vec!["Random"]);
}

#[test]
fn a_players_choice_wins_over_the_documents_suggestion() {
    let root = minimal_root();
    let header = load_header(&root.join("skin.json"), SkinLoadOptions::new(&root, &SkinUserConfig::default(), rbms_model::Mode::BEAT_7K))
        .expect("the header should load");
    let mut user = SkinUserConfig::default();
    user.filepaths.insert("Gauge".to_owned(), "hard.png".to_owned());

    let map = build_filemap(&header.custom_files, &user, &mut Draw::from_seed(TEST_SEED));
    assert_eq!(map.values().next().map(String::as_str), Some("hard.png"));
}

#[test]
fn a_random_slot_resolves_the_same_way_for_the_same_seed() {
    let root = minimal_root();
    let header = load_header(&root.join("skin.json"), SkinLoadOptions::new(&root, &SkinUserConfig::default(), rbms_model::Mode::BEAT_7K))
        .expect("the header should load");
    let user = SkinUserConfig::default();

    let first = build_filemap(&header.custom_files, &user, &mut Draw::from_seed(TEST_SEED));
    let second = build_filemap(&header.custom_files, &user, &mut Draw::from_seed(TEST_SEED));
    assert_eq!(first, second);
    let chosen = first.values().next().expect("the slot should resolve to one of the scanned names");
    assert!(["groove.png", "hard.png", "shell.png"].contains(&chosen.as_str()), "drew {chosen}");
}

#[test]
fn a_slot_with_no_candidates_contributes_no_entry() {
    let document = SkinDef {
        filepath: vec![Filepath { category: String::new(), name: "Nothing".to_owned(), path: "gauge/*.jpg".to_owned(), def: None }],
        ..SkinDef::default()
    };
    let root = minimal_root();
    let files = enumerate_custom_files(&document, &root, &root);
    assert!(build_filemap(&files, &SkinUserConfig::default(), &mut Draw::from_seed(TEST_SEED)).is_empty());
}

#[test]
fn a_documents_suggestion_names_the_file_when_the_player_has_chosen_nothing() {
    let document = SkinDef {
        filepath: vec![Filepath { category: String::new(), name: "Gauge".to_owned(), path: "gauge/*.png".to_owned(), def: Some("hard.png".to_owned()) }],
        ..SkinDef::default()
    };
    let root = minimal_root();
    let files = enumerate_custom_files(&document, &root, &root);
    let map = build_filemap(&files, &SkinUserConfig::default(), &mut Draw::from_seed(TEST_SEED));
    assert_eq!(map.values().next().map(String::as_str), Some("hard.png"));
}

#[test]
fn a_built_filemap_key_substitutes_into_the_pattern_it_came_from() {
    let root = minimal_root();
    let document = SkinDef {
        filepath: vec![Filepath { category: String::new(), name: "Gauge".to_owned(), path: "gauge/*.png".to_owned(), def: Some("hard.png".to_owned()) }],
        ..SkinDef::default()
    };
    let files = enumerate_custom_files(&document, &root, &root);
    let map = build_filemap(&files, &SkinUserConfig::default(), &mut Draw::from_seed(TEST_SEED));

    let mut resolver = FileResolver::new(&root, map, Draw::from_seed(TEST_SEED));
    let resolved = resolver.resolve(&pattern_for(&root, "gauge/*.png")).expect("the pattern should resolve");
    assert!(resolved.ends_with("gauge/hard.png"), "resolved {}", resolved.display());
}

#[test]
fn a_wildcard_with_no_filemap_entry_draws_a_candidate_and_keeps_it() {
    let root = minimal_root();
    let mut resolver = FileResolver::new(&root, BTreeMap::new(), Draw::from_seed(TEST_SEED));
    let first = resolver.resolve(&pattern_for(&root, "gauge/*.png")).expect("the pattern should resolve");
    let again = resolver.resolve(&pattern_for(&root, "gauge/*.png")).expect("the pattern should resolve");

    assert_eq!(first, again, "a pattern drawn once keeps its file for the rest of the load");
    assert!(["groove.png", "hard.png", "shell.png"].iter().any(|name| first.ends_with(name)));
}

#[test]
fn two_seeds_are_free_to_draw_differently() {
    let root = minimal_root();
    let pattern = pattern_for(&root, "gauge/*.png");
    let drawn: std::collections::BTreeSet<PathBuf> =
        (0..16u64).map(|seed| FileResolver::new(&root, BTreeMap::new(), Draw::from_seed(seed)).resolve(&pattern).expect("resolves")).collect();
    assert!(drawn.len() > 1, "a wildcard over three files should not collapse to one across sixteen seeds");
}

#[test]
fn a_wildcard_that_expands_to_nothing_returns_the_pattern() {
    let root = minimal_root();
    let mut resolver = FileResolver::new(&root, BTreeMap::new(), Draw::from_seed(TEST_SEED));
    let resolved = resolver.resolve(&pattern_for(&root, "gauge/*.jpg")).expect("an empty expansion is not an error");
    assert!(resolved.ends_with("gauge/*.jpg"));
}

#[test]
fn a_pattern_that_leaves_the_root_is_refused_by_the_resolver() {
    let root = minimal_root();
    let mut resolver = FileResolver::new(&root, BTreeMap::new(), Draw::from_seed(TEST_SEED));
    let outcome = resolver.resolve(&pattern_for(&root, "../secret.txt"));
    assert!(matches!(outcome, Err(SkinError::PathEscape(_))), "got {outcome:?}");
}

#[test]
fn joining_a_relative_path_normalises_the_separator() {
    assert_eq!(pattern_for(Path::new("/skins/one/"), "images\\frame.png"), "/skins/one/images/frame.png");
}

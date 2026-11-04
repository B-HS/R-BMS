//! The training charts the score server writes, kept in `contract/training-chart/` at the root of
//! the repository: each `.bms` is the bytes the server's generator wrote, and each `.pattern.json`
//! is what this reader makes of them. The web tests compare the generator's own pattern document
//! with the same file (`web/tests/service/training/golden.test.ts`), so the times the server works
//! out for a training chart are the times the client plays it at.
//!
//! Each test compares the pattern of a chart with the file it is committed as and fails when they
//! differ. With `RBMS_UPDATE_CONTRACT=1` it writes the file instead; the change then goes through the
//! web tests.

use std::path::{Path, PathBuf};

use rbms_model::Mode;

use super::document::{Extracted, extract};

/// The environment variable that has the tests write the contract files rather than compare them.
const UPDATE_ENV: &str = "RBMS_UPDATE_CONTRACT";

/// What a failing comparison tells a person to do.
const UPDATE_HINT: &str = "if the change is meant, run `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-cli --locked training_contract`, then \
     `bun test tests/service/training/golden.test.ts` in web/, and commit the files with the change";

fn contract_folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("contract").join("training-chart")
}

fn updating() -> bool {
    std::env::var(UPDATE_ENV).is_ok_and(|value| value == "1")
}

/// Reads the golden chart `name` as the player reads a BEAT_7K chart and compares its pattern with
/// the committed file, or writes the file when [`UPDATE_ENV`] says so. The document is kept as the
/// reader writes it: parsing it into a `Value` and writing that again could move a float by one unit
/// in the last place, and the web side reads every number back exactly.
fn check_training_chart(name: &str) {
    let chart = contract_folder().join(format!("{name}.bms"));
    let bytes = std::fs::read(&chart).unwrap_or_else(|e| panic!("{} cannot be read ({e})", chart.display()));
    let document = match extract(&bytes, Path::new(&format!("{name}.bms")), Mode::BEAT_7K).expect("a training chart is read") {
        Extracted::Pattern { document, .. } => document,
        Extracted::OtherMode { md5 } => panic!("{name}.bms ({md5}) does not read as BEAT_7K"),
    };
    let mut rendered = String::from_utf8(document).expect("a pattern is UTF-8 JSON");
    rendered.push('\n');
    let path = contract_folder().join(format!("{name}.pattern.json"));
    if updating() {
        std::fs::write(&path, rendered).expect("the contract file is writable");
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} cannot be read ({e}); {UPDATE_HINT}", path.display()));
    assert!(committed == rendered, "{name}.pattern.json is not what the client reads from {name}.bms now; {UPDATE_HINT}");
}

#[test]
fn the_basic_training_chart_reads_as_the_server_wrote_it() {
    check_training_chart("basic-160");
}

#[test]
fn the_training_chart_with_tempo_changes_and_long_notes_reads_as_the_server_wrote_it() {
    check_training_chart("soflan-ln");
}

#[test]
fn the_training_chart_with_scratches_reads_as_the_server_wrote_it() {
    check_training_chart("scratch-mix");
}

//! Skin documents written into a temporary folder, shared by the tests of the SKIN tab and by the
//! settings screen's own render tests so both drive the same document rather than two that drifted
//! apart.

use std::path::{Path, PathBuf};

/// The `SkinType` id of the song browser, which is a screen this build draws.
pub(crate) const MUSIC_SELECT: i32 = 5;

/// The `SkinType` id of the seven-key play screen.
pub(crate) const PLAY_7KEYS: i32 = 0;

/// The `SkinType` id of the screen shown while a song loads, which the bundle fixture uses as a
/// second selected document beside the browser.
pub(crate) const DECIDE: i32 = 6;

/// A `SkinType` id this build has no screen for, used to prove the row still lists it.
pub(crate) const COURSE_RESULT: i32 = 15;

/// A document that declares one of every customisation row: a two-item property, a file slot
/// with two candidates, and an offset the player may nudge along two of its six axes.
pub(crate) const BROWSER: &str = r#"{
    "type": 5,
    "name": "Browser",
    "author": "dj",
    "w": 1920,
    "h": 1080,
    "category": [{ "name": "LANE", "item": ["COVER"] }],
    "property": [
        {
            "category": "Lane",
            "name": "Cover",
            "item": [{ "name": "On", "op": 901 }, { "name": "Off", "op": 902 }],
            "def": "Off"
        }
    ],
    "filepath": [{ "category": "Lane", "name": "Background", "path": "bg/*.png", "def": "day.png" }],
    "offset": [{ "category": "Lane", "name": "Judge", "id": 12, "x": true, "y": true }],
    "destination": []
}"#;

/// A play document written in the lenient dialect: a comment, a trailing comma and an unquoted
/// key, none of which strict JSON accepts.
pub(crate) const PLAY: &str = r#"{
    // the seven-key play screen
    type: 0,
    name: 'Lane',
    w: 1280,
    h: 720,
    destination: [],
}"#;

/// A second document of the browser bundle, declaring one of every customisation row at bundle
/// scope so the rows belong to the bundle rather than to the screen that happens to declare them.
pub(crate) const SHARED: &str = r#"{
    type: 6,
    name: 'Shared',
    w: 1280,
    h: 720,
    property: [
        {
            category: 'Play Option',
            name: 'Bga Size',
            item: [{ name: 'Large', op: 910 }, { name: 'Standard', op: 911 }],
            def: 'Large',
            scope: 'bundle',
        },
    ],
    filepath: [{ category: 'Play Option', name: 'Lane Cover', path: 'bg/*.png', def: 'day.png', scope: 'bundle' }],
    offset: [{ category: 'Play Option', name: 'Bga', id: 40, x: true, a: true, scope: 'bundle' }],
    destination: [],
}"#;

/// Write the documents into a skin folder beside `settings`, and answer the folder.
pub(crate) fn write(settings: &Path) -> PathBuf {
    let folder = settings.parent().unwrap_or(Path::new(".")).join("skin");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("browser/bg")).expect("the fixture folder is writable");
    std::fs::write(folder.join("browser/browser.json"), BROWSER).expect("the document is written");
    std::fs::write(folder.join("browser/bg/day.png"), []).expect("the candidate is written");
    std::fs::write(folder.join("browser/bg/night.png"), []).expect("the candidate is written");
    std::fs::write(folder.join("play.json5"), PLAY).expect("the document is written");
    std::fs::write(folder.join("notes.txt"), "not a skin").expect("the decoy is written");
    folder
}

/// Write [`SHARED`] into the browser bundle of a folder [`write`] made, and answer the path it was
/// written to so a test can select it for the screen it declares.
pub(crate) fn write_shared(folder: &Path) -> PathBuf {
    let path = folder.join("browser/shared.json5");
    std::fs::write(&path, SHARED).expect("the document is written");
    path
}

/// The label the browser document's property row shows, which the render test looks for on the
/// drawn screen.
pub(crate) const BROWSER_PROPERTY_LABEL: &str = "LANE > COVER";

/// The label the bundle-scope property row shows, which is the bundle's own heading rather than the
/// one the declaring document wrote.
pub(crate) const SHARED_PROPERTY_LABEL: &str = "BUNDLE > BGA SIZE";

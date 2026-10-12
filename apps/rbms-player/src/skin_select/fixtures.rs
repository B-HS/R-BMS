//! Skin documents written into a temporary folder, shared by the tests of the skin menu and by the
//! settings screen's own render tests so both drive the same document rather than two that drifted
//! apart.

use std::path::{Path, PathBuf};

/// The `SkinType` id of the song browser, which is a screen this build draws.
pub(crate) const MUSIC_SELECT: i32 = 5;

/// The `SkinType` id of the seven-key play screen.
pub(crate) const PLAY_7KEYS: i32 = 0;

/// The `SkinType` id of the course result screen.
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

/// The label the browser document's property row shows, which the render test looks for on the
/// drawn screen.
pub(crate) const BROWSER_PROPERTY_LABEL: &str = "LANE > COVER";

/// The folder and the file the fading play document is written to, under a skin folder.
const FADING_FOLDER: &str = "fading";
const FADING_FILE: &str = "play.json";

/// The file its one image source is written to, beside it: a white square every part is tinted
/// from.
const FADING_SOURCE: &str = "mark.png";
const FADING_SOURCE_EDGE: u32 = 2;

/// The offsets the fading play document declares, in the order it declares them: an opacity, an
/// offset that moves, three more opacities, a fifth, and one that both moves and fades.
pub(crate) const FADING_GROUND_OPACITY: i32 = 60;
pub(crate) const FADING_MARK_PLACE: i32 = 65;
pub(crate) const FADING_LINE_OPACITY: i32 = 61;
pub(crate) const FADING_JUDGE_LINE_OPACITY: i32 = 62;
pub(crate) const FADING_NOTES_OPACITY: i32 = 63;
pub(crate) const FADING_PANEL_OPACITY: i32 = 64;
pub(crate) const FADING_GLOW: i32 = 66;

/// What the menus call each of them, in capitals like every row.
pub(crate) const FADING_GROUND_ROW: &str = "LANE BACKGROUND OPACITY";
pub(crate) const FADING_LINE_ROW: &str = "LANE LINE OPACITY";
pub(crate) const FADING_JUDGE_LINE_ROW: &str = "JUDGE LINE OPACITY";
pub(crate) const FADING_NOTES_ROW: &str = "NOTES OPACITY";
pub(crate) const FADING_PANEL_ROW: &str = "PANEL OPACITY";

/// How many opacities the fading play document declares.
pub(crate) const FADING_OPACITIES: usize = 5;

/// The lanes of the fading play document, as where each starts and how wide it is, in the chart's
/// own lane order: the seven keys first and the turntable last.
const FADING_LANES: [(i32, i32); 8] = [(104, 40), (144, 32), (176, 40), (216, 32), (248, 40), (288, 32), (320, 40), (40, 64)];

/// Where its lanes stand and how tall its notes are, in its own pixels.
const FADING_LANE_FOOT: i32 = 220;
const FADING_LANE_HEIGHT: i32 = 500;
const FADING_NOTE_HEIGHT: i32 = 14;

/// A rectangle of the fading play document, as `(x, y, w, h)` with `y` measured up from its foot.
pub(crate) type PartRect = (i32, i32, i32, i32);

/// The colour a part of it is tinted, as red, green and blue.
pub(crate) type PartTint = (u8, u8, u8);

/// One flat part of it: the rectangle it covers and the colour it is tinted.
pub(crate) type FadingPart = (PartRect, PartTint);

/// The parts of the fading play document a test looks for on a frame: the ground under the lanes,
/// the panel beside them, the mark the place offset moves, the part that both moves and fades, and
/// the part that names the judgement offset every play skin has.
pub(crate) const FADING_GROUND: FadingPart = ((40, 220, 320, 500), (30, 40, 90));
pub(crate) const FADING_PANEL: FadingPart = ((420, 60, 800, 600), (70, 70, 70));
pub(crate) const FADING_MARK: FadingPart = ((600, 300, 80, 40), (255, 255, 255));
pub(crate) const FADING_GLOW_PART: FadingPart = ((800, 300, 80, 40), (255, 220, 0));
pub(crate) const FADING_JUDGE_PART: FadingPart = ((900, 500, 80, 24), (0, 255, 120));

/// The offset every play skin has for where the judgement is written, which the fading document's
/// stand-in for the judgement names.
pub(crate) const JUDGE_OFFSET: i32 = 32;

/// A seven-key play document made of flat parts, each of which names an offset of its own: a
/// ground under the lanes, a line between every two lanes, a line the notes are judged on, the
/// note field, a panel beside it, and three marks on the panel. Every part but the marks names an
/// opacity. The opacities are not declared together, so what lists them together has gathered
/// them.
pub(crate) fn fading_play_document() -> String {
    let part = |offset: i32, ((x, y, w, h), (r, g, b)): FadingPart| {
        format!(r#"{{ "id": "cell", "offset": {offset}, "dst": [{{ "time": 0, "x": {x}, "y": {y}, "w": {w}, "h": {h}, "r": {r}, "g": {g}, "b": {b} }}] }}"#)
    };
    let lanes: Vec<String> =
        FADING_LANES.iter().map(|(x, w)| format!(r#"{{ "x": {x}, "y": {FADING_LANE_FOOT}, "w": {w}, "h": {FADING_LANE_HEIGHT} }}"#)).collect();
    let lines: Vec<String> =
        FADING_LANES.iter().map(|(x, _)| part(FADING_LINE_OPACITY, ((*x, FADING_LANE_FOOT, 2, FADING_LANE_HEIGHT), (150, 150, 170)))).collect();
    format!(
        r#"{{
    "type": 0,
    "name": "Fading",
    "w": 1280,
    "h": 720,
    "offset": [
        {{ "name": "Lane Background Opacity", "id": {FADING_GROUND_OPACITY}, "a": true }},
        {{ "name": "Mark Place", "id": {FADING_MARK_PLACE}, "x": true, "y": true }},
        {{ "name": "Lane Line Opacity", "id": {FADING_LINE_OPACITY}, "a": true }},
        {{ "name": "Judge Line Opacity", "id": {FADING_JUDGE_LINE_OPACITY}, "a": true }},
        {{ "name": "Notes Opacity", "id": {FADING_NOTES_OPACITY}, "a": true }},
        {{ "name": "Panel Opacity", "id": {FADING_PANEL_OPACITY}, "a": true }},
        {{ "name": "Glow", "id": {FADING_GLOW}, "y": true, "a": true }}
    ],
    "source": [{{ "id": "mark", "path": "{FADING_SOURCE}" }}],
    "image": [{{ "id": "cell", "src": "mark", "x": 0, "y": 0, "w": {FADING_SOURCE_EDGE}, "h": {FADING_SOURCE_EDGE} }}],
    "note": {{ "id": "notes", "note": [{notes}], "size": [{sizes}], "dst": [{lanes}] }},
    "destination": [
        {ground},
        {lines},
        {judge_line},
        {{ "id": "notes", "offset": {FADING_NOTES_OPACITY} }},
        {panel},
        {mark},
        {glow},
        {judge}
    ]
}}"#,
        notes = vec!["\"cell\""; FADING_LANES.len()].join(", "),
        sizes = vec![FADING_NOTE_HEIGHT.to_string(); FADING_LANES.len()].join(", "),
        lanes = lanes.join(", "),
        ground = part(FADING_GROUND_OPACITY, FADING_GROUND),
        lines = lines.join(",\n        "),
        judge_line = part(FADING_JUDGE_LINE_OPACITY, ((40, FADING_LANE_FOOT - 6, 320, 6), (255, 40, 40))),
        panel = part(FADING_PANEL_OPACITY, FADING_PANEL),
        mark = part(FADING_MARK_PLACE, FADING_MARK),
        glow = part(FADING_GLOW, FADING_GLOW_PART),
        judge = part(JUDGE_OFFSET, FADING_JUDGE_PART),
    )
}

/// Write the fading play document and its image source into a folder of their own under
/// `skin_folder`, and answer the document's path.
pub(crate) fn write_fading_play(skin_folder: &Path) -> PathBuf {
    let folder = skin_folder.join(FADING_FOLDER);
    std::fs::create_dir_all(&folder).expect("the fixture folder is writable");
    let white = image::RgbaImage::from_pixel(FADING_SOURCE_EDGE, FADING_SOURCE_EDGE, image::Rgba([u8::MAX; 4]));
    white.save(folder.join(FADING_SOURCE)).expect("the source image is written");
    let path = folder.join(FADING_FILE);
    std::fs::write(&path, fading_play_document()).expect("the document is written");
    path
}

/// The adjustable pack's document for the song browser: two options, a file slot, an offset that
/// moves along two axes, and one that only fades.
pub(crate) const ADJUSTABLE_SELECT: &str = r#"{
    "type": 5, "name": "Select", "w": 1280, "h": 720,
    "property": [
        { "name": "Theme", "item": [{ "name": "Dark", "op": 901 }, { "name": "Light", "op": 902 }], "def": "Dark" },
        { "name": "Cover", "item": [{ "name": "On", "op": 903 }, { "name": "Off", "op": 904 }], "def": "On" }
    ],
    "filepath": [{ "name": "Background", "path": "bg/*.png", "def": "day.png" }],
    "offset": [
        { "name": "Judge", "id": 40, "x": true, "y": true },
        { "name": "Panel Opacity", "id": 41, "a": true }
    ],
    "destination": []
}"#;

/// Its document for the decide screen, which declares the same option and the same opacity under
/// numbers of its own.
const ADJUSTABLE_DECIDE: &str = r#"{
    "type": 6, "name": "Decide", "w": 1280, "h": 720,
    "property": [{ "name": "Theme", "item": [{ "name": "Dark", "op": 911 }, { "name": "Light", "op": 912 }], "def": "Dark" }],
    "offset": [{ "name": "Panel Opacity", "id": 51, "a": true }],
    "destination": []
}"#;

/// Its document for the result screen, whose option of the same name has no item called Light, and
/// whose offset of the same name moves along one axis only.
const ADJUSTABLE_RESULT: &str = r#"{
    "type": 7, "name": "Result", "w": 1280, "h": 720,
    "property": [{ "name": "Theme", "item": [{ "name": "Dark", "op": 921 }, { "name": "Mono", "op": 922 }], "def": "Dark" }],
    "offset": [{ "name": "Judge", "id": 60, "x": true }],
    "destination": []
}"#;

/// The files of the fixture pack that are not documents, each with what is in it.
pub(crate) const ADJUSTABLE_PACK_FILES: [(&str, &str); 5] =
    [("bg/day.png", "day"), ("bg/night.png", "night"), ("lua/parts/util.lua", "return {}"), ("font/main.fnt", "font"), ("sound/open.wav", "sound")];

/// What is in the fixture pack and is no part of an export: a hidden file, a hidden folder, and the
/// record an installer keeps.
pub(crate) const ADJUSTABLE_LEFT_OUT: [&str; 3] = [".thumbs", ".history/old.json", crate::assets::default_pack::MANIFEST_FILE];

/// Write one file of a fixture, making the folders on the way to it.
pub(crate) fn write_fixture_file(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a fixture file has a folder")).expect("the fixture folder is writable");
    std::fs::write(path, text).expect("the fixture file is written");
}

/// Write the adjustable pack into `home/pack`, and answer its folder: a pack of three documents that
/// declare an option and offsets under the same names, with the files they draw from and what an
/// export leaves out.
pub(crate) fn write_adjustable_pack(home: &Path) -> PathBuf {
    let pack = home.join("pack");
    for (name, text) in [("select.json", ADJUSTABLE_SELECT), ("decide.json", ADJUSTABLE_DECIDE), ("result.json", ADJUSTABLE_RESULT)] {
        write_fixture_file(&pack.join(name), text);
    }
    for (name, text) in ADJUSTABLE_PACK_FILES {
        write_fixture_file(&pack.join(name), text);
    }
    for name in ADJUSTABLE_LEFT_OUT {
        write_fixture_file(&pack.join(name), "this machine's own");
    }
    pack
}

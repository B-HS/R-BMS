//! Play documents that let the player set colours, which the colour rows of the library, the skin
//! menu and the play menu are tested against.
//!
//! Each is a play document made of white parts and notes, so a part tinted by a colour comes out
//! as exactly that colour: a panel that names one colour row, and a note field whose lanes name
//! another. The same document is written for the seven-key and the five-key screen, which is what
//! putting a colour on the other screens is tried on.

use std::path::{Path, PathBuf};

use rbms_skin::dst::SkinColor;
use rbms_skin::loader::{SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS};

/// The folder the documents are written to, under a skin folder, and the file each is.
const TINTED_FOLDER: &str = "tinted";
const TINTED_7K_FILE: &str = "play7.json";
const TINTED_5K_FILE: &str = "play5.json";

/// The one image source the documents draw from, beside them: a white square.
const TINTED_SOURCE: &str = "white.png";
const TINTED_SOURCE_EDGE: u32 = 2;

/// The colour ids the documents declare: the panel's, and the notes'.
pub(crate) const PANEL_COLOR: i32 = 120;
pub(crate) const NOTE_COLOR: i32 = 121;

/// What the menus call the two colour rows: the panel's by its name, and the notes' by the label
/// it is given, under its category.
pub(crate) const PANEL_COLOR_ROW: &str = "PANEL";
pub(crate) const NOTE_COLOR_ROW: &str = "LANE > NOTE COLOR";

/// The colour the panel's row starts at, which the document declares as its default.
pub(crate) const PANEL_DEFAULT: SkinColor = SkinColor::rgba(0x20, 0x30, 0x40, u8::MAX);

/// Where the panel is, as `(x, y, w, h)` in the document's own pixels with `y` measured up from its
/// foot, and a point on it nothing else covers.
const PANEL: (i32, i32, i32, i32) = (420, 60, 800, 600);
pub(crate) const ON_THE_PANEL: (f32, f32) = (500.0, 100.0);

/// The size the documents are authored at.
pub(crate) const TINTED_AUTHORED: (f32, f32) = (1280.0, 720.0);

/// The lanes of the note field, as where each starts and how wide it is: the seven keys first and
/// the turntable last.
const TINTED_LANES: [(i32, i32); 8] = [(104, 40), (144, 32), (176, 40), (216, 32), (248, 40), (288, 32), (320, 40), (40, 64)];
const TINTED_LANE_FOOT: i32 = 220;
const TINTED_LANE_HEIGHT: i32 = 500;
const TINTED_NOTE_HEIGHT: i32 = 14;

/// A play document of `skin_type` whose panel is tinted by [`PANEL_COLOR`] and whose notes, in
/// every lane, by [`NOTE_COLOR`].
pub(crate) fn tinted_play_document(skin_type: i32) -> String {
    let (x, y, w, h) = PANEL;
    let lanes: Vec<String> =
        TINTED_LANES.iter().map(|(x, w)| format!(r#"{{ "x": {x}, "y": {TINTED_LANE_FOOT}, "w": {w}, "h": {TINTED_LANE_HEIGHT} }}"#)).collect();
    format!(
        r#"{{
    "type": {skin_type},
    "name": "Tinted",
    "w": 1280,
    "h": 720,
    "color": [
        {{ "name": "Panel", "id": {PANEL_COLOR}, "def": "{panel_default}" }},
        {{ "category": "Lane", "name": "Notes", "label": "Note color", "id": {NOTE_COLOR} }}
    ],
    "source": [{{ "id": "white", "path": "{TINTED_SOURCE}" }}],
    "image": [{{ "id": "cell", "src": "white", "x": 0, "y": 0, "w": {TINTED_SOURCE_EDGE}, "h": {TINTED_SOURCE_EDGE} }}],
    "note": {{ "id": "notes", "note": [{notes}], "size": [{sizes}], "tint": [{tints}], "dst": [{lanes}] }},
    "destination": [
        {{ "id": "notes" }},
        {{ "id": "cell", "tint": {PANEL_COLOR}, "dst": [{{ "time": 0, "x": {x}, "y": {y}, "w": {w}, "h": {h} }}] }}
    ]
}}"#,
        panel_default = PANEL_DEFAULT.to_hex(),
        notes = vec!["\"cell\""; TINTED_LANES.len()].join(", "),
        sizes = vec![TINTED_NOTE_HEIGHT.to_string(); TINTED_LANES.len()].join(", "),
        tints = vec![NOTE_COLOR.to_string(); TINTED_LANES.len()].join(", "),
        lanes = lanes.join(", "),
    )
}

/// Write the seven-key and the five-key tinted documents and their image source into a folder of
/// their own under `skin_folder`, and answer the two documents' paths in that order.
pub(crate) fn write_tinted_play(skin_folder: &Path) -> (PathBuf, PathBuf) {
    let folder = skin_folder.join(TINTED_FOLDER);
    std::fs::create_dir_all(&folder).expect("the fixture folder is writable");
    let white = image::RgbaImage::from_pixel(TINTED_SOURCE_EDGE, TINTED_SOURCE_EDGE, image::Rgba([u8::MAX; 4]));
    white.save(folder.join(TINTED_SOURCE)).expect("the source image is written");
    let seven = folder.join(TINTED_7K_FILE);
    std::fs::write(&seven, tinted_play_document(SKIN_TYPE_PLAY_7KEYS)).expect("the document is written");
    let five = folder.join(TINTED_5K_FILE);
    std::fs::write(&five, tinted_play_document(SKIN_TYPE_PLAY_5KEYS)).expect("the document is written");
    (seven, five)
}

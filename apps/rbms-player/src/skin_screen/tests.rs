use super::*;

/// A window pixel becomes a document coordinate: scaled onto the authored size, and measured up
/// from the bottom rather than down from the top, which is the one flip a document expects.
#[test]
fn the_cursor_reaches_a_document_in_its_own_upward_coordinates() {
    let authored = (1280.0, 720.0);
    let at = document_cursor((640.0, 180.0), (1280, 720), authored).expect("a sized window maps the cursor");
    assert_eq!(at, (640.0, 540.0), "the document measures y up from its own bottom edge");

    let scaled = document_cursor((320.0, 90.0), (640, 360), authored).expect("a half-size window maps the cursor");
    assert_eq!(scaled, (640.0, 540.0), "the same spot on a smaller window is the same spot in the document");
}

/// A window with no extent has no coordinates to map onto, and must not divide by its own zero.
#[test]
fn a_window_with_no_extent_reports_no_cursor() {
    assert!(document_cursor((10.0, 10.0), (0, 720), (1280.0, 720.0)).is_none());
    assert!(document_cursor((10.0, 10.0), (1280, 0), (1280.0, 720.0)).is_none());
}

/// A frame's Lua budget has to cover every read a document makes, not only its draw conditions: a
/// `value`, a `floatvalue` or a `text` field takes an expression too, and an object is far more
/// likely to carry one of those than a gate. The type comment above the evaluator says every read
/// goes through the frame, and this is what makes that true rather than aspirational.
#[test]
fn every_kind_of_read_a_document_makes_spends_the_frames_lua_budget() {
    use rbms_render::SkinExprEval;
    use rbms_skin::dst::{DrawStateSource, LuaDrawEval, OffsetSource, SkinOffset};
    use rbms_skin::loader::Budget;
    use rbms_skin::property::{UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};

    struct Nothing;

    impl OffsetSource for Nothing {
        fn offset(&self, _id: i32) -> Option<SkinOffset> {
            None
        }
    }

    impl DrawStateSource for Nothing {
        fn boolean(&self, _id: i32) -> bool {
            UNMAPPED_BOOLEAN
        }
    }

    impl SkinStateSource for Nothing {
        fn integer(&self, _id: i32) -> i32 {
            UNMAPPED_INTEGER
        }

        fn float(&self, _id: i32) -> f32 {
            UNMAPPED_FLOAT
        }

        fn string(&self, _id: i32) -> &str {
            UNMAPPED_STRING
        }

        fn timer(&self, _id: i32) -> Option<i64> {
            None
        }

        fn now_ms(&self) -> i64 {
            0
        }
    }

    let root = std::env::temp_dir().join(format!("rbms-skin-budget-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("the scratch folder is writable");
    let sandbox = LuaSandbox::new(&root, Budget { max_calls_per_frame: 2, ..Budget::default() }).expect("the sandbox builds");
    let number = sandbox.compile("7").expect("compiles");
    let text = sandbox.compile("'x'").expect("compiles");

    let state = Nothing;
    let frame = SkinSandboxFrame::new(&sandbox, &state);
    assert_eq!(frame.eval_integer(number), Some(7));
    assert_eq!(frame.eval_text(text), Some("x".to_owned()));
    assert_eq!(frame.calls(), 2, "a value read and a text read each spent one evaluation");

    assert_eq!(frame.eval_float(number), None, "the third read is past the frame's budget");
    assert_eq!(frame.eval_draw(number), None, "and draw gating shares the same allowance");
}

/// Compiling a document reads nothing off the disk: every file it names was read on the worker pool
/// before the screen was built, and a file that is not in that map is one the screen goes without.
/// Falling back to reading it here would put the decode back on the frame loop -- which is the whole
/// thing the pool exists to keep it off.
#[test]
fn compiling_a_document_never_reads_a_file_itself() {
    let dir = std::env::temp_dir().join(format!("rbms-skin-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the scratch folder is writable");
    let image = dir.join("source.png");
    let font = dir.join("face.ttf");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255])).save(&image).expect("the image is written");
    std::fs::write(&font, b"not really a font").expect("the font is written");

    let mut assets = PlayerSkinAssets::prepared_only(BTreeMap::new());
    assert!(assets.image(&image).is_none(), "an image nobody prepared is one the screen goes without");
    assert!(assets.font(&font).is_none(), "and so is a font");

    let decoded = SkinImage::new(2, 2, vec![9; 16]).expect("a two by two image");
    let prepared = BTreeMap::from([
        ((SkinAssetKind::Image, image.clone()), SkinAsset::Image(decoded)),
        ((SkinAssetKind::Font, font.clone()), SkinAsset::Font(vec![7, 7])),
    ]);
    let mut assets = PlayerSkinAssets::prepared_only(prepared);
    assert_eq!(assets.image(&image).map(|image| image.rgba), Some(vec![9; 16]), "what the worker read is what the screen gets");
    assert_eq!(assets.font(&font), Some(vec![7, 7]));
}

/// A bundle-scope nudge is read by every screen of that bundle, and a nudge made inside one document
/// is read only by that document -- so the two are consulted narrowest first rather than one store
/// replacing the other.
///
/// The document's own store winning matters because the SKIN tab offers both rows: a player who
/// moved one screen's chart art after moving the whole bundle's would otherwise see the bundle's
/// value come back.
#[test]
fn a_document_nudge_is_read_over_the_one_made_for_its_whole_bundle() {
    let bundle_only = SkinOffset { x: 1.0, ..SkinOffset::default() };
    let both = SkinOffset { x: 2.0, ..SkinOffset::default() };
    let document_only = SkinOffset { x: 3.0, ..SkinOffset::default() };

    let mut bundle = SkinCustomisation::default();
    bundle.offsets.insert(40, bundle_only);
    bundle.offsets.insert(46, both);
    let mut document = SkinCustomisation::default();
    document.offsets.insert(46, document_only);
    document.offsets.insert(47, document_only);

    let merged = MergedOffsets { document: Some(&document), bundle: Some(&bundle) };
    assert_eq!(merged.offset(40), Some(bundle_only), "a row only the bundle carries is still read");
    assert_eq!(merged.offset(46), Some(document_only), "the document's own answer wins where both carry the row");
    assert_eq!(merged.offset(47), Some(document_only), "a row only the document carries is read");
    assert_eq!(merged.offset(48), None, "a row neither carries is answered by neither");

    assert_eq!(MergedOffsets::default().offset(40), None, "a document nobody has customised nudges nothing");
}

/// A play document that names a `bga` object has taken the chart's art over, and keeps it even once
/// every destination of that object is gated off.
///
/// Switching BGA SIZE to OFF drops the sized destinations at load, so nothing about the object is
/// left to draw -- and a built-in quad painted underneath would put back exactly the picture the
/// player just turned off. The document with no `bga` object at all is the other half of the rule:
/// it claims nothing, so the built-in slot paints as it always did.
#[test]
fn a_bga_object_claims_the_chart_art_even_with_every_destination_gated_off() {
    use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};

    /// The option id the bundle's BGA SIZE row switches to when the art is turned off.
    const BGA_OFF_OPTION: i32 = 912;
    /// The id it carries when the art is drawn at full size, which the sized destination is gated on.
    const BGA_LARGE_OPTION: i32 = 910;
    /// The name of the row those two ids belong to.
    const BGA_SIZE_ROW: &str = "BGA SIZE";

    let root = std::env::temp_dir().join(format!("rbms-skin-bga-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("the scratch folder is writable");
    let write = |name: &str, body: &str| {
        let path = root.join(name);
        std::fs::write(&path, body).expect("the document is writable");
        path
    };

    let sized = write(
        "sized.json",
        &format!(
            r#"{{ "type": 0, "w": 1280, "h": 720,
                  "category": [{{ "name": "PLAY OPTION", "item": ["{BGA_SIZE_ROW}"] }}],
                  "property": [{{ "category": "PLAY OPTION", "name": "{BGA_SIZE_ROW}", "def": "LARGE",
                                  "item": [{{ "name": "LARGE", "op": {BGA_LARGE_OPTION} }},
                                           {{ "name": "OFF", "op": {BGA_OFF_OPTION} }}] }}],
                  "bga": {{ "id": "play-bga" }},
                  "destination": [{{ "id": "play-bga", "op": [{BGA_LARGE_OPTION}],
                                     "dst": [{{ "time": 0, "x": 0, "y": 0, "w": 640, "h": 480 }}] }}] }}"#
        ),
    );
    let bare = write("bare.json", r#"{ "type": 0, "w": 1280, "h": 720 }"#);

    let turned_off = SkinUserConfig { properties: [(BGA_SIZE_ROW.to_owned(), BGA_OFF_OPTION)].into_iter().collect(), ..SkinUserConfig::default() };
    let gated = load_skin(&sized, SkinLoadOptions::new(&root, &turned_off, rbms_model::Mode::BEAT_7K)).expect("the sized document loads");
    assert!(!gated.destinations.iter().any(|track| track.id == "play-bga"), "turning the art off leaves the object with nothing to draw");
    assert!(document_places_chart_art(&gated.def), "the document still owns the chart art it declared");

    let drawn = SkinUserConfig::default();
    let shown = load_skin(&sized, SkinLoadOptions::new(&root, &drawn, rbms_model::Mode::BEAT_7K)).expect("the sized document loads");
    assert!(shown.destinations.iter().any(|track| track.id == "play-bga"), "the default size keeps the destination the art is drawn through");
    assert!(document_places_chart_art(&shown.def), "and owns the chart art either way");

    let bare = load_skin(&bare, SkinLoadOptions::new(&root, &drawn, rbms_model::Mode::BEAT_7K)).expect("the bare document loads");
    assert!(!document_places_chart_art(&bare.def), "a document with no bga object leaves the chart art to the built-in slot");
}

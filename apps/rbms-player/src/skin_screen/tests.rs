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

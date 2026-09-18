//! Reading a skin written in the comma-separated format.
//!
//! The fixtures next door are real files in that format, written as MS932 rather than UTF-8, which
//! is what the format is encoded in; the browser one carries a Japanese name so a decoding mistake
//! shows up as a wrong name rather than as nothing at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::csv::{is_csv_document, parse_document};
use rbms_skin::loader::{DEFAULT_MAX_DOCUMENT_BYTES, ParserKind, SkinLoadOptions, SkinUserConfig, load_header, load_skin};
use rbms_skin::model::{Destination, PropertyRef, SkinDef};

/// The folder the fixture documents live in, which is also the skin root every load is given.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("csv")
}

/// One fixture document, converted with no customisation choices made.
fn convert(name: &str) -> (SkinDef, Vec<String>) {
    let root = root();
    let path = root.join(name).join("skin.lr2skin");
    parse_document(&path, &root, DEFAULT_MAX_DOCUMENT_BYTES, &BTreeMap::new()).expect("the fixture document converts")
}

/// The destination of the object `id` names.
fn destination<'a>(def: &'a SkinDef, id: &str) -> &'a Destination {
    def.destination.iter().find(|entry| entry.id == id).unwrap_or_else(|| panic!("the document declares no destination {id:?}"))
}

/// The first keyframe of a destination, as `(x, y, w, h)`.
fn rect(destination: &Destination) -> (i32, i32, i32, i32) {
    let frame = destination.dst.first().expect("the destination has a keyframe");
    (frame.x.unwrap(), frame.y.unwrap(), frame.w.unwrap(), frame.h.unwrap())
}

#[test]
fn only_the_header_extension_names_a_comma_separated_document() {
    assert!(is_csv_document(Path::new("skin/browser/theme.lr2skin")));
    assert!(is_csv_document(Path::new("skin/browser/THEME.LR2SKIN")), "the extension is matched without regard to case");
    assert!(!is_csv_document(Path::new("skin/browser/body.csv")), "an included body is part of a document rather than one of its own");
    assert!(!is_csv_document(Path::new("skin/browser/theme.json")));
}

#[test]
fn the_header_reports_its_screen_name_author_and_customisation_rows() {
    let root = root();
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions::new(&root, &user, Mode::BEAT_7K);
    let header = load_header(&root.join("basic").join("skin.lr2skin"), options).expect("the header reads");

    assert_eq!(header.skin_type, 5, "the browser screen is the one the fixture declares");
    assert_eq!(header.name, "セレクト", "an MS932 name decoded as UTF-8 would not come back as its own glyphs");
    assert_eq!(header.author, "rbms tests");
    assert_eq!(header.parser, ParserKind::Csv);

    assert_eq!(header.properties.len(), 1, "a browser document is given none of the rows a play document is");
    let row = &header.properties[0];
    assert_eq!(row.name, "Lane Cover");
    assert_eq!(row.item.iter().map(|item| (item.name.as_str(), item.op)).collect::<Vec<_>>(), vec![("Off", 900), ("On", 901)]);

    assert_eq!(header.offsets.len(), 1);
    assert_eq!((header.offsets[0].id, header.offsets[0].x, header.offsets[0].y, header.offsets[0].w), (12, true, true, false));

    assert_eq!(header.custom_files.len(), 1);
    assert_eq!(header.custom_files[0].name, "Background");
    assert_eq!(header.custom_files[0].default.as_deref(), Some("day.png"));
    assert!(header.custom_files[0].candidates.iter().any(|name| name == "day.png"), "the folder the pattern names was scanned");
}

#[test]
fn a_play_header_collects_the_rows_and_nudges_every_play_document_gets() {
    let root = root();
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions::new(&root, &user, Mode::BEAT_7K);
    let header = load_header(&root.join("play").join("skin.lr2skin"), options).expect("the header reads");

    let names: Vec<&str> = header.properties.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, vec!["BGA Size", "Ghost", "Score Graph", "Judge Detail"]);
    assert_eq!(header.offsets.iter().map(|offset| offset.id).collect::<Vec<_>>(), vec![10, 30, 32, 33]);
}

#[test]
fn a_rectangle_is_scaled_out_of_the_resolution_the_header_names_and_flipped() {
    let (def, _) = convert("basic");
    let image = destination(&def, "image-3");
    assert_eq!(rect(image), (20, 615, 200, 75), "640x480 doubles horizontally, halves again vertically, and y counts from the bottom");
    assert_eq!(image.dst.len(), 2);
    assert_eq!(image.dst[1].time, Some(1000));
    assert_eq!(image.dst[1].x, Some(120));
}

#[test]
fn a_destination_carries_the_blend_loop_timer_conditions_and_offsets_of_its_line() {
    let (def, _) = convert("basic");
    let image = destination(&def, "image-3");

    assert_eq!(image.blend, 2);
    assert_eq!(image.loop_ms, 400);
    assert_eq!(image.timer.as_ref().and_then(PropertyRef::id), Some(41), "a positive timer field is a timer id");
    assert_eq!(image.op.iter().map(|option| option.id).collect::<Vec<_>>(), vec![901, -902], "'!' in a condition field is a minus sign");
    assert_eq!(image.offsets, vec![12, 13], "every field past the numbered arguments is an offset id");
}

#[test]
fn a_twenty_four_cell_strip_reserves_a_place_for_the_sign_and_an_eleven_cell_one_does_not() {
    let (def, _) = convert("basic");
    let signed = def.value.iter().find(|value| value.reference == 107).expect("the signed number is declared");
    assert_eq!((signed.divx, signed.divy), (24, 1));
    assert_eq!(signed.digit, 5, "a strip with a negative half keeps one place for the sign");
    assert_eq!(signed.zeropadding, 2, "an empty padding field on a signed strip means the alternate zero");
    assert_eq!((signed.align, signed.space), (1, 2));

    let plain = def.value.iter().find(|value| value.reference == 108).expect("the eleven-cell number is declared");
    assert_eq!((plain.divx, plain.divy), (11, 1));
    assert_eq!(plain.digit, 3, "a strip with no negative half keeps every place for a digit");
    assert_eq!(plain.zeropadding, 0, "the eleventh cell is the alternate zero, which the renderer forces on its own");
}

#[test]
fn a_clickable_button_carries_its_event_step_and_grouping() {
    let (def, _) = convert("basic");
    let button = def.image.iter().find(|image| image.id.starts_with("button-")).expect("the button is declared");
    assert_eq!(button.reference, 73, "the event id is also the property that picks which group is shown");
    assert_eq!(button.len, 2, "the source line regroups the strip into two");
    assert_eq!(button.act.as_ref().and_then(PropertyRef::id), Some(73));
    assert_eq!(button.click, 1, "a negative step field steps backwards");
}

#[test]
fn a_text_object_keeps_its_alignment_and_the_property_it_reads() {
    let (def, _) = convert("basic");
    let text = def.text.iter().find(|text| text.reference == 10).expect("the text object is declared");
    assert_eq!(text.align, 1);
    assert!(!text.editable);
    assert!(text.font.is_empty(), "the bitmap font index is dropped: this build draws every run with its own font");
}

#[test]
fn the_wheel_takes_one_slot_per_bar_destination_and_places_its_pieces_against_each_bar() {
    let (def, _) = convert("basic");
    let wheel = def.songlist.as_ref().expect("the browser body declares a wheel");

    assert_eq!(wheel.center, 1);
    assert_eq!(wheel.clickable, vec![0, 1]);
    assert_eq!(wheel.listoff.len(), 2, "one slot per unfocused bar rectangle");
    assert_eq!(wheel.liston.len(), 2);
    assert_eq!(rect(&wheel.listoff[0]), (600, 630, 600, 30), "the body declares slot 1 before slot 0, and each still lands on its own slot");
    assert_eq!(rect(&wheel.listoff[1]), (600, 600, 600, 30));
    assert!(wheel.listoff.iter().all(|slot| slot.id == "bar-body"), "every slot draws from the set the bar images were gathered into");

    assert_eq!(wheel.text.len(), 2, "the one title the body declares is spread over every slot");
    assert_eq!(rect(&wheel.text[0]), (616, 633, 400, 21), "a piece is placed against the top-left corner of its own bar");
    assert_eq!(rect(&wheel.text[1]), (616, 603, 400, 21));
    assert_eq!(wheel.level.len(), 2);

    let set = def.imageset.iter().find(|set| set.id == "bar-body").expect("the bar images were gathered into one set");
    assert_eq!(set.images.len(), 2, "one image per row kind the body declared");
}

#[test]
fn an_inner_endif_clears_the_block_around_it_and_an_include_leaves_its_own_open() {
    let (def, warnings) = convert("branch");
    assert_eq!(def.destination.len(), 1, "exactly one object survived the guards: {warnings:?}");
    assert_eq!(
        rect(&def.destination[0]),
        (40, 710, 10, 10),
        "the object after the inner #ENDIF is drawn although the block around it never held, and the one after the include is not, \
         because the include left its own #IF open"
    );
}

#[test]
fn the_note_object_takes_one_lane_rectangle_per_lane_it_declares() {
    let (def, _) = convert("play");
    let note = def.note.as_ref().expect("the play body declares a note set");

    assert_eq!(note.dst.len(), 8, "a seven-key field has eight lanes, the scratch included");
    assert_eq!((note.dst[7].x, note.dst[7].y, note.dst[7].w, note.dst[7].h), (Some(40), Some(100), Some(60), Some(400)), "lane 0 is the scratch");
    assert_eq!((note.dst[0].x, note.dst[0].w), (Some(100), Some(40)), "lane 1 is the first key");
    assert_eq!((note.dst[1].x, note.dst[1].w), (Some(140), Some(40)));

    assert_eq!(note.note.len(), 8);
    assert!(!note.note[7].is_empty(), "the scratch lane got the image its own source line named");
    assert!(note.note[2].is_empty(), "a lane the body never declared keeps its place and no image");
    assert_eq!(note.lnbody[0], note.lnbody_active[0], "one long-note body line fills both the held and unheld halves");
    assert!(!note.mine[0].is_empty());

    let field = destination(&def, &note.id);
    assert_eq!(rect(field), (40, 100, 140, 400), "the note object's own rectangle holds every lane");
    assert_eq!(field.offsets, vec![30], "the field follows the note nudge");
    assert_eq!(note.group.len(), 1, "the measure line became a bar line of the note set");
}

#[test]
fn a_judgement_pop_up_collects_its_words_counts_and_nudges() {
    let (def, _) = convert("play");
    assert_eq!(def.judge.len(), 1);
    let judge = &def.judge[0];
    assert_eq!(judge.index, 0);
    assert!(judge.shift, "a source line that does not name one leaves the pop-up sliding with its count");
    assert_eq!(judge.images.len(), 1);
    assert_eq!(rect(&judge.images[0]), (200, 400, 60, 20));
    assert_eq!(judge.images[0].offsets, vec![32, 3], "a pop-up follows the judge nudge and then the lift");

    assert_eq!(judge.numbers.len(), 1);
    let count = rect(&judge.numbers[0]);
    assert_eq!(count, (176, -340, 12, 20), "a centred count is anchored half its own width to the left and measured from the pop-up");
}

#[test]
fn the_gauge_takes_its_nodes_from_the_strip_and_its_size_from_the_per_part_step() {
    let (def, _) = convert("play");
    let gauge = def.gauge.as_ref().expect("the play body declares a gauge");
    assert_eq!(gauge.nodes.len(), 4, "one set is four cells: the front and back of each colour");
    assert_eq!(gauge.parts, 50);

    let destination = destination(&def, &gauge.id);
    assert_eq!(rect(destination), (400, 660, 100, 20), "a per-part step of two is fifty parts wide");
}

#[test]
fn the_judge_line_and_the_measure_line_follow_the_lift_nudge() {
    let (def, _) = convert("play");
    let line = destination(&def, def.image.iter().find(|image| image.id.starts_with("judgeline-")).map(|image| image.id.as_str()).expect("declared"));
    assert_eq!(rect(line), (40, 116, 140, 4));
    assert_eq!(line.offsets, vec![3]);

    let note = def.note.as_ref().expect("the play body declares a note set");
    assert_eq!(note.group[0].offsets, vec![3]);
}

#[test]
fn commands_this_build_does_not_read_are_counted_once_rather_than_dropped_in_silence() {
    let (_, warnings) = convert("basic");
    let summary = warnings.iter().find(|line| line.contains("not ones this build reads")).expect("the unread commands were reported");
    assert!(summary.starts_with("3 command lines"), "two of one name and one of another: {summary}");
    assert!(summary.contains("#SRC_UNSUPPORTED_THING x2"), "{summary}");
    assert!(summary.contains("#SRC_ANOTHER_UNKNOWN x1"), "{summary}");
}

#[test]
fn a_comma_separated_document_loads_through_the_ordinary_loader() {
    let root = root();
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, Mode::BEAT_7K) };
    let skin = load_skin(&root.join("basic").join("skin.lr2skin"), options).expect("the fixture document loads");

    assert_eq!(skin.parser, ParserKind::Csv);
    assert_eq!(skin.def.skin_type, 5);
    assert_eq!(skin.def.name, "セレクト");
    assert_eq!(skin.sources.len(), 2, "both image slots resolved to files under the skin root");
    assert!(skin.sources.values().all(|path| path.exists()), "a resolved source names a file that is there: {:?}", skin.sources);
    assert!(!skin.destinations.is_empty(), "the document's objects were assembled into tracks");
    assert_eq!(skin.custom_files.len(), 1);
}

#[test]
fn a_document_that_declares_a_screen_this_build_has_none_for_is_refused() {
    let root = root();
    let user = SkinUserConfig::default();
    let scratch = std::env::temp_dir().join(format!("rbms-csv-unsupported-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("the scratch folder is writable");
    std::fs::write(scratch.join("skin.lr2skin"), "#INFORMATION,11,Theme,dj\n").expect("the document is written");

    let options = SkinLoadOptions::new(&scratch, &user, Mode::BEAT_7K);
    let error = load_skin(&scratch.join("skin.lr2skin"), options).expect_err("a theme document has no screen here");
    assert_eq!(error.to_string(), "skin type 11 is not supported");
    let _ = std::fs::remove_dir_all(&scratch);
    let _ = root;
}

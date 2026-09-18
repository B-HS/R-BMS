//! Reading a character definition: its directives, its rectangle table and its animation rows.

use std::path::Path;

use rbms_skin::chp::{
    CharaSheet, DEFAULT_FACE_ALL, DEFAULT_FACE_UPPER, DEFAULT_FRAME_MS, NO_LOOP, SLOT_CHAR_BMP, SLOT_CHAR_BMP_2P, SLOT_CHAR_TEX, find_chp_file, parse_chara,
    read_chara,
};

/// The fixture every test here reads, which carries a two-frame neutral pattern.
fn fixture() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The fixture's text, read the way a load reads it.
fn fixture_text() -> String {
    std::fs::read_to_string(fixture().join("tests/fixtures/chp/chara/two-frame.chp")).expect("the fixture is readable")
}

#[test]
fn the_image_slots_name_the_files_the_directives_give_them() {
    let def = parse_chara(&fixture_text());
    assert_eq!(def.images[SLOT_CHAR_BMP].as_deref(), Some(Path::new("body.png")));
    assert_eq!(def.images[SLOT_CHAR_BMP_2P].as_deref(), Some(Path::new("body-2p.png")));
    assert_eq!(def.images[SLOT_CHAR_TEX], None);
}

#[test]
fn the_rectangle_table_is_addressed_in_base_thirty_six() {
    let def = parse_chara(&fixture_text());
    assert_eq!((def.rect(0).x, def.rect(0).y, def.rect(0).w, def.rect(0).h), (0, 240, 128, 32));
    assert_eq!((def.rect(2).x, def.rect(2).y, def.rect(2).w, def.rect(2).h), (0, 0, 60, 80));
    assert_eq!((def.rect(3).x, def.rect(3).y, def.rect(3).w, def.rect(3).h), (60, 0, 60, 80));
    assert_eq!((def.rect(36).x, def.rect(36).y, def.rect(36).w, def.rect(36).h), (10, 20, 60, 80));
    assert_eq!((def.rect(37).x, def.rect(37).y, def.rect(37).w, def.rect(37).h), (10, 12, 60, 80));
}

#[test]
fn the_size_box_and_the_portrait_rectangles_come_from_their_own_directives() {
    let def = parse_chara(&fixture_text());
    assert_eq!(def.size, (128, 128));
    assert_eq!(def.face_upper, DEFAULT_FACE_UPPER);
    assert_eq!(def.face_all, DEFAULT_FACE_ALL);
}

#[test]
fn a_motion_without_its_own_frame_time_takes_the_files_default() {
    let def = parse_chara(&fixture_text());
    assert_eq!(def.frame_ms[1], 240, "the neutral motion states its own time");
    assert_eq!(def.frame_ms[7], 150, "an unstated motion takes #Anime");
    assert_eq!(def.loop_frame[1], NO_LOOP);
    assert_eq!(def.loop_frame[10], 0, "#Loop states the frame the bad motion repeats from");
}

#[test]
fn a_frame_time_below_one_reads_as_the_default() {
    let def = parse_chara("#CharBMP\tbody.png\n#Anime\t0\n#Frame\t1\t-4\n");
    assert_eq!(def.frame_ms[1], DEFAULT_FRAME_MS);
    assert_eq!(def.frame_ms[2], DEFAULT_FRAME_MS);
}

#[test]
fn a_two_frame_pattern_resolves_every_column() {
    let def = parse_chara(&fixture_text());
    let neutral = def.rows.iter().find(|row| row.motion == 1).expect("the neutral row is read");
    assert_eq!(neutral.sheet, CharaSheet::Pattern);
    assert_eq!(neutral.frames.len(), 2);
    assert_eq!(neutral.frames[0].source, 2);
    assert_eq!(neutral.frames[1].source, 3);
    assert_eq!((neutral.frames[0].destination.x, neutral.frames[0].destination.y), (10, 20));
    assert_eq!((neutral.frames[1].destination.x, neutral.frames[1].destination.y), (10, 12));
    assert_eq!([neutral.frames[0].alpha, neutral.frames[1].alpha], [255, 128]);
    assert_eq!([neutral.frames[0].angle_deg, neutral.frames[1].angle_deg], [0, 180]);
}

#[test]
fn a_row_with_no_destination_column_covers_the_whole_size_box() {
    let def = parse_chara(&fixture_text());
    let bad = def.rows.iter().find(|row| row.motion == 10).expect("the bad row is read");
    assert_eq!(bad.frames.len(), 2);
    for frame in &bad.frames {
        assert_eq!((frame.destination.x, frame.destination.y, frame.destination.w, frame.destination.h), (0, 0, 128, 128));
        assert_eq!(frame.alpha, 255);
        assert_eq!(frame.angle_deg, 0);
    }
}

#[test]
fn an_interpolated_field_walks_from_the_last_stated_value_to_the_next() {
    let def = parse_chara("#CharBMP\tbody.png\n#Size\t100\t100\n#00\t0\t0\t10\t10\n#Pattern\t1\t00000000\t00000000\t00----ff\n");
    let row = def.rows.first().expect("the row is read");
    assert_eq!(row.frames.iter().map(|frame| frame.alpha).collect::<Vec<u8>>(), vec![0, 85, 170, 255]);
}

#[test]
fn the_rows_are_ordered_pattern_then_texture_then_layer() {
    let def = parse_chara(&fixture_text());
    let order: Vec<CharaSheet> = def.rows.iter().map(|row| row.sheet).collect();
    assert_eq!(order, vec![CharaSheet::Pattern, CharaSheet::Pattern, CharaSheet::Layer]);
}

#[test]
fn the_second_player_colour_needs_its_own_sheet() {
    let def = parse_chara(&fixture_text());
    assert!(def.second_player_colour(true), "the fixture carries a second-player sheet");
    assert!(!def.second_player_colour(false), "a document that asks for the first colour gets it");
    let single = parse_chara("#CharBMP\tbody.png\n");
    assert!(!single.second_player_colour(true), "a definition with one sheet cannot be the second colour");
}

#[test]
fn a_comment_field_ends_the_line() {
    let def = parse_chara("#CharBMP\tbody.png\n#Size\t64\t64\t// the box\n#Anime\t50\t//\t999\n");
    assert_eq!(def.size, (64, 64));
    assert_eq!(def.frame_ms[1], 50);
}

#[test]
fn the_definition_file_is_found_through_its_directory() {
    let directory = fixture().join("tests/fixtures/chp/chara");
    assert_eq!(find_chp_file(&directory), Some(directory.join("two-frame.chp")));
    assert_eq!(find_chp_file(&directory.join("two-frame.chp")), Some(directory.join("two-frame.chp")));
}

#[test]
fn reading_resolves_every_image_path_against_the_skin_root() {
    let root = fixture().join("tests/fixtures/chp");
    let def = read_chara(&root, &root.join("chara")).expect("the fixture is a readable definition");
    assert_eq!(def.images[SLOT_CHAR_BMP].as_deref(), Some(root.join("chara/body.png").as_path()));
    assert_eq!(def.images[SLOT_CHAR_BMP_2P].as_deref(), Some(root.join("chara/body-2p.png").as_path()));
}

#[test]
fn a_definition_without_a_pattern_sheet_is_refused() {
    let root = fixture().join("tests/fixtures/chp");
    let error = read_chara(&root, &root.join("nameless")).expect_err("a definition with no #CharBMP has nothing to draw");
    assert!(error.to_string().contains("#CharBMP"), "the error says what is missing: {error}");
}

#[test]
fn a_definition_that_names_a_file_outside_the_root_is_refused() {
    let root = fixture().join("tests/fixtures/chp");
    let def = read_chara(&root, &root.join("escaping")).expect("the definition itself is inside the root");
    assert_eq!(def.images[SLOT_CHAR_BMP], None, "the escaping path was dropped rather than opened");
}

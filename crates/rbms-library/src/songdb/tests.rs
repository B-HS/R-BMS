use std::collections::HashSet;

use rbms_model::Mode;

use super::*;

fn fresh() -> SongDb {
    let mut db = SongDb::open_in_memory().expect("open the in-memory database");
    db.migrate().expect("apply the schema");
    db
}

fn row(path: &str, title: &str) -> SongRow {
    SongRow {
        path: path.to_string(),
        md5: "0123456789abcdef0123456789abcdef".to_string(),
        sha256: "f".repeat(64),
        title: title.to_string(),
        subtitle: "[ANOTHER]".to_string(),
        artist: "artist".to_string(),
        subartist: "obj:someone".to_string(),
        genre: "genre".to_string(),
        maker: "maker".to_string(),
        level: "12".to_string(),
        difficulty: 3,
        mode: mode_id(Mode::BEAT_7K),
        judge: 2,
        total: 320.5,
        init_bpm: 174.0,
        min_bpm: 87,
        max_bpm: 174,
        length_ms: 123_456,
        notes: 1500,
        long_notes: 42,
        stagefile: "stage.png".to_string(),
        banner: "banner.png".to_string(),
        backbmp: "back.png".to_string(),
        preview: "preview.ogg".to_string(),
        folder: "/songs/pack".to_string(),
        favorite: 0,
        date: 1_700_000_000,
        adddate: 1_700_000_500,
        size: 65_536,
        feature: FEATURE_LONG_NOTE | FEATURE_STOP_SEQUENCE,
        content: CONTENT_BGA | CONTENT_PREVIEW,
    }
}

#[test]
fn migrate_creates_the_song_table_with_every_reference_column() {
    let db = fresh();
    let columns = db.columns_of("song").expect("read the song columns");
    let expected = [
        "path",
        "md5",
        "sha256",
        "title",
        "subtitle",
        "genre",
        "artist",
        "subartist",
        "tag",
        "folder",
        "parent",
        "stagefile",
        "banner",
        "backbmp",
        "preview",
        "level",
        "difficulty",
        "maxbpm",
        "minbpm",
        "length",
        "mode",
        "judge",
        "feature",
        "content",
        "date",
        "favorite",
        "adddate",
        "notes",
        "charthash",
        "rbms_size",
        "rbms_total",
        "rbms_init_bpm",
        "rbms_long_notes",
        "rbms_maker",
        "rbms_level_text",
        "rbms_scanned_at",
    ];
    assert_eq!(columns, expected, "the stored song columns are the reference set plus the rbms_ additions");
}

#[test]
fn migrate_creates_the_folder_table_with_every_reference_column() {
    let db = fresh();
    let columns = db.columns_of("folder").expect("read the folder columns");
    assert_eq!(columns, ["path", "title", "subtitle", "command", "banner", "parent", "type", "date", "adddate", "max"]);
}

#[test]
fn migrate_creates_the_detail_and_meta_tables() {
    let db = fresh();
    assert_eq!(
        db.columns_of("song_detail").expect("read the detail columns"),
        ["path", "duration_us", "peak_density", "avg_density", "end_density", "density_bins"]
    );
    assert_eq!(db.columns_of("meta").expect("read the meta columns"), ["key", "value"]);
}

#[test]
fn migrate_stamps_the_schema_version_and_is_idempotent() {
    let mut db = fresh();
    assert_eq!(db.migrate().expect("migrate again"), SCHEMA_VERSION, "a second migration of a current database changes nothing");
    let stored = schema::read_meta_u32(&db.conn, schema::META_SCHEMA_VERSION).expect("read the stored version");
    assert_eq!(stored, Some(SCHEMA_VERSION));
    assert!(schema::read_meta(&db.conn, schema::META_CREATED_AT).expect("read the creation stamp").is_some());
}

#[test]
fn a_database_from_a_newer_build_is_refused_rather_than_migrated() {
    let mut db = fresh();
    schema::write_meta(&db.conn, schema::META_SCHEMA_VERSION, &(SCHEMA_VERSION + 1).to_string()).expect("stamp a newer version");
    let err = db.migrate().expect_err("a newer database is not opened");
    assert!(matches!(err, SongDbError::Schema(_)), "the refusal is a schema error, not a statement failure");
    assert!(err.to_string().contains(&(SCHEMA_VERSION + 1).to_string()), "the message names the version found: {err}");
}

#[test]
fn an_upserted_song_comes_back_field_for_field() {
    let mut db = fresh();
    let stored = row("/songs/pack/chart.bms", "A Chart");
    db.upsert_batch(std::slice::from_ref(&stored)).expect("store the chart");
    let read = db.all_songs().expect("read the charts");
    assert_eq!(read, vec![stored], "every field of the row survives the round trip");
}

#[test]
fn upserting_the_same_path_refreshes_the_parse_and_keeps_the_user_data() {
    let mut db = fresh();
    let first = row("/songs/pack/chart.bms", "Old Title");
    db.upsert_batch(std::slice::from_ref(&first)).expect("store the chart");
    db.set_favorite(&first.path, 1).expect("star the chart");

    let mut second = row("/songs/pack/chart.bms", "New Title");
    second.notes = 1600;
    second.favorite = 0;
    second.adddate = 1_900_000_000;
    db.upsert_batch(std::slice::from_ref(&second)).expect("refresh the chart");

    let read = db.song(&first.path).expect("read the chart").expect("the chart is stored");
    assert_eq!(read.title, "New Title", "the rescan refreshes the parsed fields");
    assert_eq!(read.notes, 1600);
    assert_eq!(read.favorite, 1, "a rescan never clears the star");
    assert_eq!(read.adddate, first.adddate, "a rescan keeps the date the chart first appeared");
    assert_eq!(db.song_count().expect("count the charts"), 1, "the refresh replaced the row rather than adding one");
}

#[test]
fn songs_come_back_title_ordered_regardless_of_case() {
    let mut db = fresh();
    db.upsert_batch(&[row("/songs/pack/c.bms", "cherry"), row("/songs/pack/a.bms", "Apple"), row("/songs/pack/b.bms", "banana")]).expect("store the charts");
    let titles: Vec<String> = db.all_songs().expect("read the charts").into_iter().map(|s| s.title).collect();
    assert_eq!(titles, ["Apple", "banana", "cherry"]);
}

#[test]
fn stamp_index_returns_the_modification_time_and_size_of_every_chart() {
    let mut db = fresh();
    let mut first = row("/songs/pack/a.bms", "A");
    first.date = 111;
    first.size = 222;
    let mut second = row("/songs/pack/b.bms", "B");
    second.date = 333;
    second.size = 444;
    db.upsert_batch(&[first, second]).expect("store the charts");

    let stamps = db.stamp_index().expect("read the stamps");
    assert_eq!(stamps.len(), 2);
    assert_eq!(stamps.get("/songs/pack/a.bms"), Some(&(111, 222)));
    assert_eq!(stamps.get("/songs/pack/b.bms"), Some(&(333, 444)));
}

#[test]
fn by_md5_finds_every_copy_of_a_chart_whatever_case_it_is_asked_in() {
    let mut db = fresh();
    let mut here = row("/songs/pack/a.bms", "A");
    here.md5 = "abc123".to_string();
    let mut there = row("/songs/other/a.bms", "A");
    there.md5 = "abc123".to_string();
    let mut unrelated = row("/songs/pack/b.bms", "B");
    unrelated.md5 = "def456".to_string();
    db.upsert_batch(&[here, there, unrelated]).expect("store the charts");

    let found = db.by_md5("ABC123").expect("look the chart up");
    assert_eq!(found.len(), 2, "both copies of the chart come back");
    assert!(found.iter().all(|s| s.md5 == "abc123"));
}

#[test]
fn songs_in_folders_takes_only_the_configured_trees_and_not_their_lookalikes() {
    let mut db = fresh();
    db.upsert_batch(&[row("/songs/pack/a.bms", "A"), row("/songs/pack2/b.bms", "B"), row("/elsewhere/c.bms", "C")]).expect("store the charts");

    let found = db.songs_in_folders(&["/songs/pack".to_string()]).expect("read one tree");
    assert_eq!(found.len(), 1, "a sibling folder whose name merely starts the same way is not in the tree");
    assert_eq!(found[0].path, "/songs/pack/a.bms");

    let both = db.songs_in_folders(&["/songs/pack".to_string(), "/songs".to_string()]).expect("read overlapping trees");
    let paths: Vec<String> = both.into_iter().map(|s| s.path).collect();
    assert_eq!(paths, ["/songs/pack/a.bms", "/songs/pack2/b.bms"], "a chart under two configured trees is listed once");
}

#[test]
fn delete_missing_drops_only_charts_under_the_scanned_roots() {
    let mut db = fresh();
    db.upsert_batch(&[row("/songs/pack/gone.bms", "Gone"), row("/songs/pack/here.bms", "Here"), row("/elsewhere/kept.bms", "Kept")]).expect("store the charts");

    let alive: HashSet<String> = ["/songs/pack/here.bms".to_string()].into_iter().collect();
    let removed = db.delete_missing(&alive, &["/songs/pack".to_string()]).expect("drop the missing charts");
    assert_eq!(removed, 1);

    let left: Vec<String> = db.all_songs().expect("read the charts").into_iter().map(|s| s.path).collect();
    assert_eq!(left, ["/songs/pack/here.bms", "/elsewhere/kept.bms"], "a chart outside the scanned roots is left alone");
}

#[test]
fn a_deleted_chart_takes_its_cached_detail_with_it() {
    let mut db = fresh();
    let chart = row("/songs/pack/a.bms", "A");
    db.upsert_batch(std::slice::from_ref(&chart)).expect("store the chart");
    db.put_detail(&chart.path, &DetailRow { duration_us: 1, peak_density: 2.0, avg_density: 3.0, end_density: 4.0, density: vec![1, 2] })
        .expect("cache the detail");

    db.delete_missing(&HashSet::new(), &["/songs/pack".to_string()]).expect("drop the chart");
    assert!(db.detail(&chart.path).expect("read the detail").is_none(), "the detail row went with the chart");
}

#[test]
fn a_cached_detail_comes_back_with_its_density_bins_intact() {
    let mut db = fresh();
    let chart = row("/songs/pack/a.bms", "A");
    db.upsert_batch(std::slice::from_ref(&chart)).expect("store the chart");
    let detail = DetailRow { duration_us: 98_765, peak_density: 12.5, avg_density: 7.25, end_density: 9.0, density: vec![0, 3, 9, 4_000_000_000] };
    db.put_detail(&chart.path, &detail).expect("cache the detail");
    assert_eq!(db.detail(&chart.path).expect("read the detail"), Some(detail));
}

#[test]
fn a_detail_written_twice_replaces_rather_than_duplicates() {
    let mut db = fresh();
    let chart = row("/songs/pack/a.bms", "A");
    db.upsert_batch(std::slice::from_ref(&chart)).expect("store the chart");
    db.put_detail(&chart.path, &DetailRow { duration_us: 1, peak_density: 1.0, avg_density: 1.0, end_density: 1.0, density: vec![1] }).expect("cache once");
    let second = DetailRow { duration_us: 2, peak_density: 2.0, avg_density: 2.0, end_density: 2.0, density: vec![2, 2] };
    db.put_detail(&chart.path, &second).expect("cache again");
    assert_eq!(db.detail(&chart.path).expect("read the detail"), Some(second));
}

#[test]
fn details_are_written_alongside_the_charts_of_a_batch() {
    let mut db = fresh();
    let chart = row("/songs/pack/a.bms", "A");
    let detail = DetailRow { duration_us: 42, peak_density: 1.0, avg_density: 2.0, end_density: 3.0, density: vec![7] };
    db.upsert_batch_with_details(&[(chart.clone(), Some(detail.clone())), (row("/songs/pack/b.bms", "B"), None)]).expect("store the batch");
    assert_eq!(db.detail(&chart.path).expect("read the detail"), Some(detail));
    assert!(db.detail("/songs/pack/b.bms").expect("read the other detail").is_none(), "a chart with no computed detail stores none");
}

#[test]
fn the_numeric_level_column_is_derived_from_the_authored_one() {
    let mut db = fresh();
    let mut plain = row("/songs/pack/a.bms", "A");
    plain.level = "12".to_string();
    let mut decorated = row("/songs/pack/b.bms", "B");
    decorated.level = "\u{2605}12".to_string();
    db.upsert_batch(&[plain, decorated]).expect("store the charts");

    let read = db.all_songs().expect("read the charts");
    assert_eq!(read[0].level, "12", "the authored level text is what comes back");
    assert_eq!(read[1].level, "\u{2605}12");

    let mut stmt = db.conn.prepare("SELECT path, level FROM song ORDER BY path").expect("read the numeric levels");
    let levels: Vec<(String, i32)> =
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).expect("run the query").collect::<Result<Vec<_>, _>>().expect("collect the levels");
    assert_eq!(levels, [("/songs/pack/a.bms".to_string(), 12), ("/songs/pack/b.bms".to_string(), 0)]);
}

#[test]
fn the_parse_generation_decides_whether_the_next_scan_is_a_full_one() {
    let db = fresh();
    assert!(db.needs_full_rescan().expect("read the generation"), "a database no scan has completed against is scanned in full");
    db.set_parser_version(PARSER_VERSION).expect("stamp the generation");
    assert!(!db.needs_full_rescan().expect("read the generation"), "a database the current parser filled is scanned incrementally");
    db.set_parser_version(PARSER_VERSION + 1).expect("stamp a different generation");
    assert!(db.needs_full_rescan().expect("read the generation"), "rows parsed by another generation are re-read");
}

#[test]
fn mode_ids_round_trip_through_the_reference_numbering() {
    for mode in Mode::ALL.iter().chain(std::iter::once(&Mode::KEYBOARD_24K)) {
        let id = mode_id(*mode);
        assert_ne!(id, 0, "{} has a stored id", mode.name);
        assert_eq!(mode_from_id(id), Some(*mode), "{} comes back from its id", mode.name);
    }
}

#[test]
fn mode_ids_are_the_reference_enum_numbers() {
    assert_eq!(mode_id(Mode::BEAT_5K), 5);
    assert_eq!(mode_id(Mode::BEAT_7K), 7);
    assert_eq!(mode_id(Mode::BEAT_10K), 10);
    assert_eq!(mode_id(Mode::BEAT_14K), 14);
    assert_eq!(mode_id(Mode::POPN_9K), 9);
    assert_eq!(mode_id(Mode::KEYBOARD_24K), 25);
    assert_eq!(mode_from_id(0), None, "the reference reads zero as any mode, so no mode claims it");
}

#[test]
fn the_reference_feature_and_content_bits_are_the_values_the_reference_defines() {
    assert_eq!(
        [
            FEATURE_UNDEFINED_LN,
            FEATURE_MINE_NOTE,
            FEATURE_RANDOM,
            FEATURE_LONG_NOTE,
            FEATURE_CHARGE_NOTE,
            FEATURE_HELL_CHARGE_NOTE,
            FEATURE_STOP_SEQUENCE,
            FEATURE_SCROLL
        ],
        [1, 2, 4, 8, 16, 32, 64, 128]
    );
    assert_eq!([CONTENT_TEXT, CONTENT_BGA, CONTENT_PREVIEW, CONTENT_NO_KEYSOUND], [1, 2, 4, 128]);
}

#[test]
fn density_bins_survive_being_packed_and_unpacked() {
    let bins = vec![0u32, 1, 255, 256, u32::MAX];
    assert_eq!(rows::unpack_density(&rows::pack_density(&bins)), bins);
    assert!(rows::unpack_density(&[]).is_empty());
    assert_eq!(rows::unpack_density(&[1, 0, 0, 0, 9]), vec![1], "a trailing partial bin is dropped rather than read past");
}

#[test]
fn a_folder_prefix_ends_in_exactly_one_separator() {
    assert_eq!(with_trailing_slash("/songs/pack"), "/songs/pack/");
    assert_eq!(with_trailing_slash("/songs/pack/"), "/songs/pack/");
    assert_eq!(with_trailing_slash("/songs/pack///"), "/songs/pack/");
}

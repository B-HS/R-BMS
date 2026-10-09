//! Unit tests for the chart cluster: every id reads as the reference reads it from the selected
//! song, a chart the player does not know enough about is not guessed at, and a slot with no chart
//! reads as the reference's missing song.

use std::borrow::Cow;
use std::path::PathBuf;

use rbms_library::{ChartDetail, SongEntry};
use rbms_model::Mode;
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, INTEGER_ABSENT};

use super::{BpmRange, ChartContents, ChartMeta, ChartState, Density, NoteCounts};
use crate::skin_host::ClusterState;

/// The six difficulty options, `OPTION_DIFFICULTY0` to `OPTION_DIFFICULTY5`.
const DIFFICULTY_OPTIONS: [i32; 6] = [OPTION_DIFFICULTY0, OPTION_DIFFICULTY1, OPTION_DIFFICULTY2, OPTION_DIFFICULTY3, OPTION_DIFFICULTY4, OPTION_DIFFICULTY5];

/// The seven key-mode options.
const MODE_OPTIONS: [i32; 7] = [OPTION_7KEYSONG, OPTION_5KEYSONG, OPTION_14KEYSONG, OPTION_10KEYSONG, OPTION_9KEYSONG, OPTION_24KEYSONG, OPTION_24KEYDPSONG];

/// The five judge options, from the hardest to the easiest.
const JUDGE_OPTIONS: [i32; 5] = [OPTION_JUDGE_VERYHARD, OPTION_JUDGE_HARD, OPTION_JUDGE_NORMAL, OPTION_JUDGE_EASY, OPTION_JUDGE_VERYEASY];

/// A chart the player knows everything about.
fn known_chart() -> ChartMeta<'static> {
    ChartMeta {
        title: "Title",
        subtitle: "[ANOTHER]",
        genre: "GENRE",
        artist: "Artist",
        subartist: "feat. Guest",
        heading: None,
        md5: "0123456789abcdef0123456789abcdef",
        sha256: "sha-256-of-the-chart",
        table_name: "Insane",
        table_level: "★",
        level: 12,
        difficulty: 4,
        mode: Some(Mode::BEAT_7K),
        judge: Some(2),
        length_ms: Some(2 * 60_000 + 5_000 + 250),
        notes: Some(1500),
        bpm: Some(BpmRange { min: 120, max: 180 }),
        main_bpm: Some(150.9),
        note_counts: Some(NoteCounts { normal: 1000, long: 300, scratch: 150, long_scratch: 50 }),
        density: Some(Density { average: 9.876, peak: 21.345, end: 13.5 }),
        total: Some(350.8),
        contents: ChartContents {
            bga: Some(true),
            text: Some(false),
            long_note: Some(true),
            random_sequence: Some(false),
            bpm_stop: Some(true),
            stagefile: Some(true),
            banner: Some(false),
            backbmp: Some(true),
        },
    }
}

fn text_of(state: &ChartState<'_>, id: i32) -> Option<String> {
    state.text(id).map(Cow::into_owned)
}

fn options_that_are_on(state: &ChartState<'_>, ids: &[i32]) -> Vec<i32> {
    ids.iter().copied().filter(|id| state.boolean(*id) == Some(true)).collect()
}

#[test]
fn the_texts_of_a_chart_read_as_the_reference_reads_them() {
    let chart = known_chart();
    let state = ChartState::Chart(&chart);

    assert_eq!(text_of(&state, STRING_TITLE).as_deref(), Some("Title"));
    assert_eq!(text_of(&state, STRING_SUBTITLE).as_deref(), Some("[ANOTHER]"));
    assert_eq!(text_of(&state, STRING_FULLTITLE).as_deref(), Some("Title [ANOTHER]"));
    assert_eq!(text_of(&state, STRING_GENRE).as_deref(), Some("GENRE"));
    assert_eq!(text_of(&state, STRING_ARTIST).as_deref(), Some("Artist"));
    assert_eq!(text_of(&state, STRING_SUBARTIST).as_deref(), Some("feat. Guest"));
    assert_eq!(text_of(&state, STRING_FULLARTIST).as_deref(), Some("Artist feat. Guest"));
    assert_eq!(text_of(&state, STRING_SONG_HASH_MD5).as_deref(), Some("0123456789abcdef0123456789abcdef"));
    assert_eq!(text_of(&state, STRING_SONG_HASH_SHA256).as_deref(), Some("sha-256-of-the-chart"));
    assert_eq!(text_of(&state, STRING_TABLE_NAME).as_deref(), Some("Insane"));
    assert_eq!(text_of(&state, STRING_TABLE_LEVEL).as_deref(), Some("★"));
    assert_eq!(text_of(&state, STRING_TABLE_FULL).as_deref(), Some("★Insane"), "the full table name is the level, then the name");
}

#[test]
fn a_chart_without_a_subtitle_or_subartist_has_the_plain_full_texts() {
    let chart = ChartMeta { subtitle: "", subartist: "", ..known_chart() };
    let state = ChartState::Chart(&chart);

    assert_eq!(text_of(&state, STRING_FULLTITLE).as_deref(), Some("Title"));
    assert_eq!(text_of(&state, STRING_FULLARTIST).as_deref(), Some("Artist"));
}

#[test]
fn a_heading_replaces_the_title_and_the_full_title_but_nothing_else() {
    let chart = ChartMeta { heading: Some("Course Name"), ..known_chart() };
    let state = ChartState::Chart(&chart);

    assert_eq!(text_of(&state, STRING_TITLE).as_deref(), Some("Course Name"));
    assert_eq!(text_of(&state, STRING_FULLTITLE).as_deref(), Some("Course Name"));
    assert_eq!(text_of(&state, STRING_SUBTITLE).as_deref(), Some("[ANOTHER]"));
}

#[test]
fn the_numbers_of_a_chart_read_as_the_reference_reads_them() {
    let chart = known_chart();
    let state = ChartState::Chart(&chart);

    for folder_level in NUMBER_FOLDER_BEGINNER..=NUMBER_FOLDER_INSANE {
        assert_eq!(state.integer(folder_level), Some(12), "folder level {folder_level} shows the chart's own level");
    }
    assert_eq!(state.integer(NUMBER_PLAYLEVEL), Some(12));
    assert_eq!(state.integer(NUMBER_TOTALNOTES), Some(1500));
    assert_eq!(state.integer(NUMBER_TOTALNOTES2), Some(1500));
    assert_eq!(state.integer(NUMBER_MAXBPM), Some(180));
    assert_eq!(state.integer(NUMBER_MINBPM), Some(120));
    assert_eq!(state.integer(NUMBER_MAINBPM), Some(150), "the main tempo is cut to whole BPM");
    assert_eq!(state.integer(NUMBER_TOTALNOTE_NORMAL), Some(1000));
    assert_eq!(state.integer(NUMBER_TOTALNOTE_LN), Some(300));
    assert_eq!(state.integer(NUMBER_TOTALNOTE_SCRATCH), Some(150));
    assert_eq!(state.integer(NUMBER_TOTALNOTE_BSS), Some(50));
    assert_eq!(state.integer(NUMBER_SONGGAUGE_TOTAL), Some(350));
    assert_eq!(state.integer(NUMBER_JUDGERANK), Some(2));
    assert_eq!(state.integer(NUMBER_SONGLENGTH_MINUTE), Some(2));
    assert_eq!(state.integer(NUMBER_SONGLENGTH_SECOND), Some(5));
}

#[test]
fn the_density_digits_are_two_truncated_digits_after_the_whole_part() {
    let chart = known_chart();
    let state = ChartState::Chart(&chart);

    assert_eq!(state.integer(NUMBER_DENSITY_PEAK), Some(21));
    assert_eq!(state.integer(NUMBER_DENSITY_PEAK_AFTERDOT), Some(34), "21.345 reads 34, truncated and not rounded");
    assert_eq!(state.integer(NUMBER_DENSITY_END), Some(13));
    assert_eq!(state.integer(NUMBER_DENSITY_END_AFTERDOT), Some(50));
    assert_eq!(state.integer(NUMBER_DENSITY_AVERAGE), Some(9));
    assert_eq!(state.integer(NUMBER_DENSITY_AVERAGE_AFTERDOT), Some(87));
}

#[test]
fn the_length_of_a_chart_drops_the_hours() {
    let chart = ChartMeta { length_ms: Some(3_600_000 + 61_999), ..known_chart() };
    let state = ChartState::Chart(&chart);

    assert_eq!(state.integer(NUMBER_SONGLENGTH_MINUTE), Some(1), "the minute digit is the minute of the hour");
    assert_eq!(state.integer(NUMBER_SONGLENGTH_SECOND), Some(1));
}

#[test]
fn the_floats_of_a_chart_read_as_the_reference_reads_them() {
    let chart = known_chart();
    let state = ChartState::Chart(&chart);

    assert_eq!(state.float(FLOAT_CHART_PEAKDENSITY), Some(21.345_f64 as f32));
    assert_eq!(state.float(FLOAT_CHART_ENDDENSITY), Some(13.5_f64 as f32));
    assert_eq!(state.float(FLOAT_CHART_AVERAGEDENSITY), Some(9.876_f64 as f32));
    assert_eq!(state.float(FLOAT_CHART_TOTALGAUGE), Some(350.8_f64 as f32));
}

#[test]
fn exactly_one_difficulty_option_is_on_for_any_difficulty() {
    for (difficulty, on) in [
        (-3, OPTION_DIFFICULTY0),
        (0, OPTION_DIFFICULTY0),
        (1, OPTION_DIFFICULTY1),
        (3, OPTION_DIFFICULTY3),
        (5, OPTION_DIFFICULTY5),
        (6, OPTION_DIFFICULTY0),
        (99, OPTION_DIFFICULTY0),
    ] {
        let chart = ChartMeta { difficulty, ..known_chart() };
        let state = ChartState::Chart(&chart);

        assert_eq!(options_that_are_on(&state, &DIFFICULTY_OPTIONS), [on], "difficulty {difficulty}");
        assert!(DIFFICULTY_OPTIONS.iter().all(|id| state.boolean(*id).is_some()), "every difficulty option has an answer");
    }
}

#[test]
fn exactly_one_key_mode_option_is_on_for_a_known_mode() {
    for (mode, on) in [
        (Mode::BEAT_7K, OPTION_7KEYSONG),
        (Mode::BEAT_5K, OPTION_5KEYSONG),
        (Mode::BEAT_14K, OPTION_14KEYSONG),
        (Mode::BEAT_10K, OPTION_10KEYSONG),
        (Mode::POPN_9K, OPTION_9KEYSONG),
        (Mode::KEYBOARD_24K, OPTION_24KEYSONG),
    ] {
        let chart = ChartMeta { mode: Some(mode), ..known_chart() };
        let state = ChartState::Chart(&chart);

        assert_eq!(options_that_are_on(&state, &MODE_OPTIONS), [on], "{}", mode.name);
    }
}

#[test]
fn the_double_keyboard_option_is_off_for_every_mode_the_player_has() {
    for mode in Mode::ALL {
        let chart = ChartMeta { mode: Some(*mode), ..known_chart() };
        let state = ChartState::Chart(&chart);

        assert_eq!(state.boolean(OPTION_24KEYDPSONG), Some(false), "{}", mode.name);
    }
}

#[test]
fn the_judge_options_read_the_raw_judge_value_the_way_the_reference_bands_it() {
    for (judge, on) in [
        (0, Some(OPTION_JUDGE_VERYHARD)),
        (1, Some(OPTION_JUDGE_HARD)),
        (2, Some(OPTION_JUDGE_NORMAL)),
        (3, Some(OPTION_JUDGE_EASY)),
        (4, Some(OPTION_JUDGE_VERYEASY)),
        (5, None),
        (9, None),
        (10, Some(OPTION_JUDGE_VERYHARD)),
        (34, Some(OPTION_JUDGE_VERYHARD)),
        (35, Some(OPTION_JUDGE_HARD)),
        (59, Some(OPTION_JUDGE_HARD)),
        (60, Some(OPTION_JUDGE_NORMAL)),
        (84, Some(OPTION_JUDGE_NORMAL)),
        (85, Some(OPTION_JUDGE_EASY)),
        (109, Some(OPTION_JUDGE_EASY)),
        (110, Some(OPTION_JUDGE_VERYEASY)),
        (i32::MAX, Some(OPTION_JUDGE_VERYEASY)),
        (-1, None),
    ] {
        let chart = ChartMeta { judge: Some(judge), ..known_chart() };
        let state = ChartState::Chart(&chart);

        assert_eq!(options_that_are_on(&state, &JUDGE_OPTIONS), on.into_iter().collect::<Vec<_>>(), "judge value {judge}");
    }
}

#[test]
fn the_extras_of_a_chart_come_as_a_pair_of_options_with_one_of_them_on() {
    let chart = known_chart();
    let state = ChartState::Chart(&chart);

    assert_eq!((state.boolean(OPTION_NO_BGA), state.boolean(OPTION_BGA)), (Some(false), Some(true)));
    assert_eq!((state.boolean(OPTION_NO_LN), state.boolean(OPTION_LN)), (Some(false), Some(true)));
    assert_eq!((state.boolean(OPTION_NO_TEXT), state.boolean(OPTION_TEXT)), (Some(true), Some(false)));
    assert_eq!((state.boolean(OPTION_NO_RANDOMSEQUENCE), state.boolean(OPTION_RANDOMSEQUENCE)), (Some(true), Some(false)));
    assert_eq!((state.boolean(OPTION_NO_STAGEFILE), state.boolean(OPTION_STAGEFILE)), (Some(false), Some(true)));
    assert_eq!((state.boolean(OPTION_NO_BANNER), state.boolean(OPTION_BANNER)), (Some(true), Some(false)));
    assert_eq!((state.boolean(OPTION_NO_BACKBMP), state.boolean(OPTION_BACKBMP)), (Some(false), Some(true)));
    assert_eq!(state.boolean(OPTION_BPMSTOP), Some(true));
}

#[test]
fn a_tempo_change_is_a_range_and_a_table_song_is_one_that_came_from_a_table() {
    let changing_chart = known_chart();
    let changing = ChartState::Chart(&changing_chart);
    assert_eq!((changing.boolean(OPTION_NO_BPMCHANGE), changing.boolean(OPTION_BPMCHANGE)), (Some(false), Some(true)));

    let steady_chart = ChartMeta { bpm: Some(BpmRange { min: 150, max: 150 }), ..known_chart() };

    let steady = ChartState::Chart(&steady_chart);
    assert_eq!((steady.boolean(OPTION_NO_BPMCHANGE), steady.boolean(OPTION_BPMCHANGE)), (Some(true), Some(false)));

    assert_eq!(changing.boolean(OPTION_TABLE_SONG), Some(true));
    let no_table = ChartMeta { table_name: "", ..known_chart() };
    assert_eq!(ChartState::Chart(&no_table).boolean(OPTION_TABLE_SONG), Some(false));
}

#[test]
fn what_the_player_does_not_know_about_a_chart_is_not_answered() {
    let chart = ChartMeta { difficulty: 2, level: 7, title: "Only", ..ChartMeta::default() };
    let state = ChartState::Chart(&chart);

    assert_eq!(state.integer(NUMBER_PLAYLEVEL), Some(7), "the level is known");
    assert_eq!(text_of(&state, STRING_TITLE).as_deref(), Some("Only"));
    for number in [
        NUMBER_TOTALNOTES,
        NUMBER_MAXBPM,
        NUMBER_MINBPM,
        NUMBER_MAINBPM,
        NUMBER_TOTALNOTE_NORMAL,
        NUMBER_TOTALNOTE_BSS,
        NUMBER_DENSITY_PEAK,
        NUMBER_DENSITY_END_AFTERDOT,
        NUMBER_SONGGAUGE_TOTAL,
        NUMBER_JUDGERANK,
        NUMBER_SONGLENGTH_MINUTE,
        NUMBER_SONGLENGTH_SECOND,
    ] {
        assert_eq!(state.integer(number), None, "number {number}");
    }
    for float in [FLOAT_CHART_PEAKDENSITY, FLOAT_CHART_ENDDENSITY, FLOAT_CHART_AVERAGEDENSITY, FLOAT_CHART_TOTALGAUGE] {
        assert_eq!(state.float(float), None, "float {float}");
    }
    for option in [OPTION_7KEYSONG, OPTION_BGA, OPTION_NO_LN, OPTION_BPMCHANGE, OPTION_JUDGE_NORMAL, OPTION_STAGEFILE, OPTION_NO_BACKBMP, OPTION_BPMSTOP] {
        assert_eq!(state.boolean(option), None, "option {option}");
    }
    assert_eq!(state.boolean(OPTION_DIFFICULTY2), Some(true), "the difficulty is known");
}

#[test]
fn a_cluster_with_nothing_connected_knows_nothing() {
    let state = ChartState::default();

    assert_eq!(state.boolean(OPTION_DIFFICULTY0), None);
    assert_eq!(state.integer(NUMBER_PLAYLEVEL), None);
    assert_eq!(state.float(FLOAT_CHART_TOTALGAUGE), None);
    assert_eq!(state.text(STRING_TITLE), None);
}

#[test]
fn a_slot_with_no_chart_reads_as_the_reference_reads_a_missing_song() {
    let state = ChartState::Empty;

    assert_eq!(state.integer(NUMBER_PLAYLEVEL), Some(INTEGER_ABSENT));
    assert_eq!(state.integer(NUMBER_SONGLENGTH_SECOND), Some(INTEGER_ABSENT));
    assert_eq!(state.float(FLOAT_CHART_PEAKDENSITY), Some(FLOAT_ABSENT));
    assert_eq!(text_of(&state, STRING_TITLE).as_deref(), Some(""));
    assert_eq!(text_of(&state, STRING_TABLE_FULL).as_deref(), Some(""));

    for option in DIFFICULTY_OPTIONS.iter().chain(&MODE_OPTIONS).chain(&JUDGE_OPTIONS) {
        assert_eq!(state.boolean(*option), Some(false), "a song option is off without a song: {option}");
    }
    assert_eq!(state.boolean(OPTION_NO_BGA), Some(false), "even the 'no BGA' option, which asks the song");
    assert_eq!(state.boolean(OPTION_NO_STAGEFILE), Some(true), "while the image options ask what is loaded, which is nothing");
    assert_eq!(state.boolean(OPTION_STAGEFILE), Some(false));
    assert_eq!(state.boolean(OPTION_NO_BANNER), Some(true));
    assert_eq!(state.boolean(OPTION_NO_BACKBMP), Some(true));
    assert_eq!(state.boolean(OPTION_TABLE_SONG), Some(false));
}

#[test]
fn ids_of_other_clusters_are_not_answered() {
    let chart = known_chart();
    for state in [ChartState::Chart(&chart), ChartState::Empty] {
        assert_eq!(state.integer(NUMBER_SCORE), None);
        assert_eq!(state.integer(NUMBER_TIME_YEAR), None);
        assert_eq!(state.boolean(OPTION_NOW_LOADING), None);
        assert_eq!(state.boolean(OPTION_RESULT_CLEAR), None);
        assert_eq!(state.float(FLOAT_LOADING_PROGRESS), None);
        assert_eq!(state.text(STRING_PLAYER), None);
        assert_eq!(state.rate(RATE_LOAD_PROGRESS), None);
    }
}

fn entry() -> SongEntry {
    SongEntry {
        path: PathBuf::from("songs/title.bms"),
        title: "Entry Title".to_owned(),
        subtitle: "-sub-".to_owned(),
        artist: "Entry Artist".to_owned(),
        genre: "Entry Genre".to_owned(),
        maker: String::new(),
        level: " 11 ".to_owned(),
        difficulty: 3,
        init_bpm: 150.0,
        rank: 3,
        total: 0.0,
        mode: Mode::BEAT_14K,
        md5: "abc".to_owned(),
        stagefile: "stage.png".to_owned(),
        banner: String::new(),
        preview: String::new(),
    }
}

#[test]
fn an_entry_of_the_library_gives_what_a_scan_reads_from_its_header() {
    let entry = entry();
    let chart = ChartMeta::of_entry(&entry);
    let state = ChartState::Chart(&chart);

    assert_eq!(text_of(&state, STRING_FULLTITLE).as_deref(), Some("Entry Title -sub-"));
    assert_eq!(state.integer(NUMBER_PLAYLEVEL), Some(11), "the level text is trimmed and read as a number");
    assert_eq!(options_that_are_on(&state, &DIFFICULTY_OPTIONS), [OPTION_DIFFICULTY3]);
    assert_eq!(options_that_are_on(&state, &MODE_OPTIONS), [OPTION_14KEYSONG]);
    assert_eq!(state.integer(NUMBER_JUDGERANK), Some(3));
    assert_eq!(state.boolean(OPTION_JUDGE_EASY), Some(true));
    assert_eq!((state.boolean(OPTION_STAGEFILE), state.boolean(OPTION_BANNER)), (Some(true), Some(false)));
    assert_eq!(state.integer(NUMBER_SONGGAUGE_TOTAL), None, "a header that states no total says nothing of it");
    assert_eq!(state.integer(NUMBER_TOTALNOTES), None, "and a scan has not counted the notes");
    assert_eq!(state.boolean(OPTION_BGA), None);
}

#[test]
fn an_entry_whose_level_is_not_a_number_has_level_zero() {
    let entry = SongEntry { level: "?".to_owned(), ..entry() };

    let chart = ChartMeta::of_entry(&entry);
    assert_eq!(ChartState::Chart(&chart).integer(NUMBER_PLAYLEVEL), Some(0));
}

#[test]
fn measuring_the_notes_adds_what_the_detail_found() {
    let entry = entry();
    let detail = ChartDetail {
        notes: 1200,
        long_notes: 100,
        duration_us: 95_400_000,
        bpm_min: 90.0,
        bpm_max: 199.9,
        density: vec![3, 4],
        peak_density: 18.0,
        avg_density: 12.5,
        end_density: 15.25,
    };
    let chart = ChartMeta::of_entry(&entry).with_detail(&detail);
    let state = ChartState::Chart(&chart);

    assert_eq!(state.integer(NUMBER_TOTALNOTES), Some(1200));
    assert_eq!((state.integer(NUMBER_SONGLENGTH_MINUTE), state.integer(NUMBER_SONGLENGTH_SECOND)), (Some(1), Some(35)));
    assert_eq!((state.integer(NUMBER_MINBPM), state.integer(NUMBER_MAXBPM)), (Some(90), Some(199)));
    assert_eq!(state.boolean(OPTION_BPMCHANGE), Some(true));
    assert_eq!((state.integer(NUMBER_DENSITY_AVERAGE), state.integer(NUMBER_DENSITY_AVERAGE_AFTERDOT)), (Some(12), Some(50)));
    assert_eq!(state.integer(NUMBER_DENSITY_END_AFTERDOT), Some(25));
    assert!(state.integer(NUMBER_SONGGAUGE_TOTAL).is_some_and(|total| total > 0), "a chart that states no total has the one its notes imply");
}

#[test]
fn a_total_the_chart_states_is_kept_when_the_notes_are_measured() {
    let entry = SongEntry { total: 321.5, ..entry() };
    let detail = ChartDetail {
        notes: 10,
        long_notes: 0,
        duration_us: 0,
        bpm_min: 150.0,
        bpm_max: 150.0,
        density: Vec::new(),
        peak_density: 0.0,
        avg_density: 0.0,
        end_density: 0.0,
    };
    let chart = ChartMeta::of_entry(&entry).with_detail(&detail);
    let state = ChartState::Chart(&chart);

    assert_eq!(state.float(FLOAT_CHART_TOTALGAUGE), Some(321.5));
    assert_eq!(state.boolean(OPTION_NO_BPMCHANGE), Some(true));
}

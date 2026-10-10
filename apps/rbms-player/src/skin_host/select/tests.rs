//! The browser cluster answers from the bar under the cursor as the reference's `MusicSelector`
//! properties read it, and answers nothing until the browser lends a frame.

use rbms_config::{Config, SortMode};
use rbms_course::CourseConstraint;
use rbms_judge::{ClearType, clear_type_id};
use rbms_model::Mode;
use rbms_render::skin_render::frame::{BarDistribution, BarKind, LAMP_KINDS, SongBar};
use rbms_skin::lua::local_time;
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};
use rbms_store::scoredb::{BestScore, JudgeCounts};

use super::*;
use crate::skin_host::ir::IrBrowser;
use crate::skin_host::{Cluster, IdSpace, STRING_TARGET_NAME, ScreenHost, clusters_of};
use rbms_skin::dst::DrawStateSource;
use rbms_skin::property::SkinHost;
use rbms_skin::timer::TimerState;

/// The first moment of the last play: late in 2023.
const LAST_PLAYED: i64 = 1_700_000_000;

/// A score of 141 notes: 100 PGREAT, 30 GREAT, 5 GOOD, 2 BAD, 1 POOR and 3 MISS, split between the
/// early and the late side, played 7 times and cleared 5.
fn best(clear: ClearType) -> BestScore {
    BestScore {
        clear: clear_type_id(clear),
        judge: JudgeCounts::from_early_late([40, 10, 2, 1, 1, 1], [60, 20, 3, 1, 0, 2]),
        notes: 141,
        combo: 120,
        minbp: 6,
        playcount: 7,
        clearcount: 5,
        date: LAST_PLAYED,
        ..BestScore::default()
    }
}

/// The browser's frame over `bars` with the cursor on the first.
fn inputs<'a>(bars: &'a [SongBar], config: &'a Config) -> SelectInputs<'a> {
    SelectInputs {
        bars,
        cursor: 0,
        chart: None,
        course: None,
        directory: Vec::new(),
        mode_filter: None,
        sort: SortMode::Title,
        panel: None,
        selected_replay: None,
        config,
        ir: IrBrowser::default(),
    }
}

/// A chart on disk with `best` as the score on it.
fn chart_with<'a>(best: Option<&'a BestScore>) -> ChartInputs<'a> {
    ChartInputs { mode: Mode::BEAT_7K, best, favorite: false, replays_stored: 0 }
}

fn song() -> SongBar {
    SongBar::new(BarKind::Song { exists: true }, "Song")
}

/// The frame of a browser whose cursor is on `bar`.
fn shown_on(bar: SongBar, chart: Option<ChartInputs<'_>>) -> SelectShown {
    let config = Config::default();
    let bars = [bar];
    SelectShown::of(SelectInputs { chart, ..inputs(&bars, &config) })
}

fn state_of(shown: &SelectShown) -> SelectState<'_> {
    SelectState::of(shown)
}

#[test]
fn a_browser_that_lends_no_frame_is_asked_nothing() {
    let state = SelectState::default();

    assert_eq!(state.boolean(OPTION_SONGBAR), None);
    assert_eq!(state.integer(NUMBER_SCORE), None);
    assert_eq!(state.image_index(BUTTON_SORT), None);
    assert_eq!(state.rate(RATE_MUSICSELECT_POSITION), None);
    assert_eq!(state.float(FLOAT_SCORE_RATE), None);
    assert_eq!(state.text(STRING_DIRECTORY), None);
}

#[test]
fn exactly_one_of_the_folder_song_and_course_options_is_on_for_each_kind_of_bar_the_reference_names() {
    let kinds = [
        BarKind::Song { exists: true },
        BarKind::Song { exists: false },
        BarKind::Folder,
        BarKind::Table,
        BarKind::Command,
        BarKind::Search,
        BarKind::Other,
        BarKind::Course { complete: true },
        BarKind::Course { complete: false },
    ];
    for kind in kinds {
        let shown = shown_on(SongBar::new(kind, ""), None);
        let state = state_of(&shown);
        let on = [OPTION_FOLDERBAR, OPTION_SONGBAR, OPTION_GRADEBAR].into_iter().filter(|id| state.boolean(*id) == Some(true)).count();

        assert_eq!(on, 1, "{kind:?}");
    }
    for kind in [BarKind::Executable, BarKind::RandomCourse { complete: true }] {
        let shown = shown_on(SongBar::new(kind, ""), None);
        let state = state_of(&shown);

        assert_eq!([OPTION_FOLDERBAR, OPTION_SONGBAR, OPTION_GRADEBAR].map(|id| state.boolean(id)), [Some(false); 3], "{kind:?} is none of the three");
    }
}

#[test]
fn a_bar_is_playable_when_it_can_be_started_and_random_bars_have_options_of_their_own() {
    let playable = |kind: BarKind| state_of(&shown_on(SongBar::new(kind, ""), None)).boolean(OPTION_PLAYABLEBAR);

    assert_eq!(playable(BarKind::Song { exists: true }), Some(true));
    assert_eq!(playable(BarKind::Song { exists: false }), Some(false), "a chart that is not on disk cannot be started");
    assert_eq!(playable(BarKind::Course { complete: true }), Some(true));
    assert_eq!(playable(BarKind::Course { complete: false }), Some(false));
    assert_eq!(playable(BarKind::RandomCourse { complete: true }), Some(true));
    assert_eq!(playable(BarKind::Executable), Some(true));
    assert_eq!(playable(BarKind::Folder), Some(false));

    let random = shown_on(SongBar::new(BarKind::Executable, ""), None);
    assert_eq!((state_of(&random).boolean(OPTION_RANDOMSELECTBAR), state_of(&random).boolean(OPTION_RANDOMCOURSEBAR)), (Some(true), Some(false)));
    let random_course = shown_on(SongBar::new(BarKind::RandomCourse { complete: false }, ""), None);
    assert_eq!((state_of(&random_course).boolean(OPTION_RANDOMSELECTBAR), state_of(&random_course).boolean(OPTION_RANDOMCOURSEBAR)), (Some(false), Some(true)));
}

#[test]
fn a_list_with_no_bars_has_no_bar_options_on() {
    let config = Config::default();
    let shown = SelectShown::of(inputs(&[], &config));
    let state = state_of(&shown);

    assert_eq!([OPTION_FOLDERBAR, OPTION_SONGBAR, OPTION_GRADEBAR, OPTION_PLAYABLEBAR].map(|id| state.boolean(id)), [Some(false); 4]);
    assert_eq!(state.integer(NUMBER_SCORE), Some(INTEGER_ABSENT));
    assert_eq!(state.rate(RATE_MUSICSELECT_POSITION), Some(0.0));
}

#[test]
fn a_chart_with_no_score_reads_the_sentinels_of_the_reference() {
    let shown = shown_on(song(), Some(chart_with(None)));
    let state = state_of(&shown);

    for id in [NUMBER_SCORE, NUMBER_SCORE2, NUMBER_SCORE3, NUMBER_MAXCOMBO, NUMBER_MAXCOMBO2, NUMBER_MAXCOMBO3, NUMBER_MISSCOUNT, NUMBER_MISSCOUNT2] {
        assert_eq!(state.integer(id), Some(INTEGER_ABSENT), "number {id}");
    }
    for id in (NUMBER_PLAYCOUNT..=NUMBER_POOR_RATE).chain([NUMBER_SCORE_RATE, NUMBER_SCORE_RATE_AFTERDOT, NUMBER_TOTAL_RATE, NUMBER_TOTAL_RATE_AFTERDOT]) {
        assert_eq!(state.integer(id), Some(INTEGER_ABSENT), "number {id}");
    }
    for id in NUMBER_LASTPLAY_TIMESTAMP..=NUMBER_LASTPLAY_SECOND {
        assert_eq!(state.integer(id), Some(INTEGER_ABSENT), "number {id}");
    }
    assert_eq!(state.integer(NUMBER_MAXSCORE), Some(0), "the most a chart can score reads zero rather than absent without a score");
    assert_eq!(state.integer(NUMBER_POINT), Some(0));
    for id in NUMBER_PERFECT..=NUMBER_POOR {
        assert_eq!(state.integer(id), Some(0), "the judgement counts the host asks a state for read zero without a score: number {id}");
    }
    assert_eq!(
        (state.float(FLOAT_SCORE_RATE), state.float(FLOAT_TOTAL_RATE), state.float(FLOAT_PERFECT_RATE)),
        (Some(FLOAT_ABSENT), Some(FLOAT_ABSENT), Some(FLOAT_ABSENT))
    );
    assert_eq!(state.image_index(INDEX_CLEAR), Some(IMAGE_INDEX_ABSENT));
    assert_eq!(state.boolean(OPTION_SELECT_BAR_NOT_PLAYED), Some(true));
    for (id, _) in CLEAR_OPTIONS {
        assert_eq!(state.boolean(id), Some(false), "option {id}");
    }
}

#[test]
fn the_best_score_on_a_chart_is_what_its_numbers_read() {
    let record = best(ClearType::ExHard);
    let shown = shown_on(song(), Some(chart_with(Some(&record))));
    let state = state_of(&shown);
    let number = |id: i32| state.integer(id);

    assert_eq!((number(NUMBER_SCORE), number(NUMBER_SCORE2), number(NUMBER_SCORE3)), (Some(230), Some(230), Some(230)));
    assert_eq!(number(NUMBER_MAXSCORE), Some(282));
    assert_eq!((number(NUMBER_MAXCOMBO), number(NUMBER_MAXCOMBO2), number(NUMBER_MAXCOMBO3)), (Some(120), Some(120), Some(120)));
    assert_eq!((number(NUMBER_MISSCOUNT), number(NUMBER_MISSCOUNT2)), (Some(6), Some(6)));
    assert_eq!((number(NUMBER_PLAYCOUNT), number(NUMBER_CLEARCOUNT), number(NUMBER_FAILCOUNT)), (Some(7), Some(5), Some(2)));
    assert_eq!([NUMBER_PERFECT2, NUMBER_GREAT2, NUMBER_GOOD2, NUMBER_BAD2, NUMBER_POOR2].map(number), [Some(100), Some(30), Some(5), Some(2), Some(1)]);
    assert_eq!([NUMBER_PERFECT, NUMBER_GREAT, NUMBER_GOOD, NUMBER_BAD, NUMBER_POOR].map(number), [Some(100), Some(30), Some(5), Some(2), Some(1)]);
    assert_eq!(
        [NUMBER_PERFECT_RATE, NUMBER_GREAT_RATE, NUMBER_GOOD_RATE, NUMBER_BAD_RATE, NUMBER_POOR_RATE].map(number),
        [Some(70), Some(21), Some(3), Some(1), Some(0)]
    );
    assert_eq!([NUMBER_EARLY_PERFECT, NUMBER_LATE_PERFECT, NUMBER_EARLY_GREAT, NUMBER_LATE_GREAT].map(number), [Some(40), Some(60), Some(10), Some(20)]);
    assert_eq!([NUMBER_MISS, NUMBER_EARLY_MISS, NUMBER_LATE_MISS].map(number), [Some(3), Some(1), Some(2)]);
    assert_eq!((number(NUMBER_TOTALEARLY), number(NUMBER_TOTALLATE)), (Some(10 + 2 + 1 + 1 + 1), Some(20 + 3 + 1 + 2)), "every judgement but the PGREATs");
    assert_eq!((number(NUMBER_COMBOBREAK), number(NUMBER_POOR_PLUS_MISS), number(NUMBER_BAD_PLUS_POOR_PLUS_MISS)), (Some(3), Some(4), Some(6)));
    assert_eq!(number(NUMBER_POINT), Some((150_000 * 100 + 100_000 * 30 + 20_000 * 5) / 141 + 50_000 * 120 / 141), "a seven key chart weighs the combo in");
    assert_eq!((number(NUMBER_SCORE_RATE), number(NUMBER_SCORE_RATE_AFTERDOT)), (Some(81), Some(56)));
    assert_eq!(
        (number(NUMBER_TOTAL_RATE), number(NUMBER_TOTAL_RATE_AFTERDOT), number(NUMBER_SCORE_RATE2), number(NUMBER_SCORE_RATE_AFTERDOT2)),
        (Some(81), Some(56), Some(81), Some(56))
    );
    assert_eq!(number(NUMBER_LASTPLAY_TIMESTAMP), Some(1_700_000_000));
    let clock = local_time(LAST_PLAYED).expect("the C library dates a moment in 2023");
    assert_eq!(
        [NUMBER_LASTPLAY_YEAR, NUMBER_LASTPLAY_MONTH, NUMBER_LASTPLAY_DAY, NUMBER_LASTPLAY_HOUR, NUMBER_LASTPLAY_MINUTE, NUMBER_LASTPLAY_SECOND].map(number),
        [Some(clock.year), Some(clock.month), Some(clock.day), Some(clock.hour), Some(clock.minute), Some(clock.second)]
    );
    assert_eq!(state.float(FLOAT_SCORE_RATE), Some(230.0 / 282.0));
    assert_eq!(state.float(FLOAT_PERFECT_RATE), Some(100.0 / 141.0));
}

#[test]
fn the_rank_of_the_score_is_the_one_option_whose_band_it_falls_in() {
    let record = best(ClearType::Hard);
    let shown = shown_on(song(), Some(chart_with(Some(&record))));
    let state = state_of(&shown);
    let ranks = [OPTION_1P_AAA, OPTION_1P_AA, OPTION_1P_A, OPTION_1P_B, OPTION_1P_C, OPTION_1P_D, OPTION_1P_E, OPTION_1P_F];

    let on = ranks.map(|id| state.boolean(id) == Some(true));
    assert_eq!(on, [false, true, false, false, false, false, false, false], "81.6 percent is an AA");
    assert_eq!(state.boolean(OPTION_NOW_AAA_1P + 1), Some(true));
    assert_eq!(state.boolean(OPTION_RESULT_AAA_1P + 1), Some(true));
    assert_eq!(state.boolean(OPTION_BEST_AAA_1P + 1), Some(false), "the browser never sets a best score to compare with");
    assert_eq!(
        (state.boolean(OPTION_AAA), state.boolean(OPTION_AA), state.boolean(OPTION_A), state.boolean(OPTION_F)),
        (Some(false), Some(true), Some(true), Some(true))
    );

    let none = shown_on(song(), Some(chart_with(None)));
    assert_eq!(ranks.map(|id| state_of(&none).boolean(id)), [Some(false); 8], "a bar with no score is in no rank");
}

#[test]
fn the_lamp_of_the_score_switches_on_its_own_option_only() {
    for (lamp, option) in CLEAR_OPTIONS.map(|(option, lamp)| (lamp, option)) {
        let record = best(lamp);
        let shown = shown_on(song(), Some(chart_with(Some(&record))));
        let state = state_of(&shown);
        let on = CLEAR_OPTIONS.iter().filter(|(id, _)| state.boolean(*id) == Some(true)).map(|(id, _)| *id).collect::<Vec<_>>();

        assert_eq!(on, [option], "{lamp:?}");
        assert_eq!(state.boolean(OPTION_SELECT_BAR_NOT_PLAYED), Some(false), "{lamp:?}");
        assert_eq!(state.image_index(INDEX_CLEAR), Some(i32::from(clear_type_id(lamp))));
    }
    let no_play = best(ClearType::NoPlay);
    let shown = shown_on(song(), Some(chart_with(Some(&no_play))));
    assert_eq!(state_of(&shown).boolean(OPTION_SELECT_BAR_NOT_PLAYED), Some(true), "a record that never cleared anything is not played either");
}

#[test]
fn a_bar_that_is_not_a_chart_or_a_course_is_never_a_bar_that_was_not_played() {
    for kind in [BarKind::Folder, BarKind::Table, BarKind::Executable] {
        assert_eq!(state_of(&shown_on(SongBar::new(kind, ""), None)).boolean(OPTION_SELECT_BAR_NOT_PLAYED), Some(false), "{kind:?}");
    }
    assert_eq!(state_of(&shown_on(SongBar::new(BarKind::Course { complete: true }, ""), None)).boolean(OPTION_SELECT_BAR_NOT_PLAYED), Some(true));
}

#[test]
fn a_course_has_no_score_here_and_a_folder_has_none_ever() {
    let record = best(ClearType::Hard);
    for kind in [BarKind::Folder, BarKind::Course { complete: true }] {
        let shown = shown_on(SongBar::new(kind, ""), Some(chart_with(Some(&record))));
        assert_eq!(state_of(&shown).integer(NUMBER_SCORE), Some(INTEGER_ABSENT), "{kind:?}");
    }
}

#[test]
fn a_folder_reads_how_the_charts_under_it_are_cleared() {
    let lamps = [3, 1, 0, 2, 0, 5, 0, 0, 4, 0, 1];
    let folder = SongBar { distribution: Some(Box::new(BarDistribution { lamps, ..BarDistribution::default() })), ..SongBar::new(BarKind::Folder, "Folder") };
    let shown = shown_on(folder, None);
    let state = state_of(&shown);

    assert_eq!(state.integer(NUMBER_FOLDER_TOTALSONGS), Some(16));
    let counts = (NUMBER_FOLDER_NOPLAY..=NUMBER_FOLDER_MAX).map(|id| state.integer(id)).collect::<Vec<_>>();
    assert_eq!(counts, lamps.map(|count| Some(count as i32)));
    assert_eq!(counts.len(), LAMP_KINDS);

    let uncounted = shown_on(SongBar::new(BarKind::Table, "Table"), None);
    assert_eq!((state_of(&uncounted).integer(NUMBER_FOLDER_TOTALSONGS), state_of(&uncounted).integer(NUMBER_FOLDER_MAX)), (Some(0), Some(0)));

    let chart = shown_on(song(), None);
    assert_eq!(
        (state_of(&chart).integer(NUMBER_FOLDER_TOTALSONGS), state_of(&chart).integer(NUMBER_FOLDER_NOPLAY)),
        (Some(INTEGER_ABSENT), Some(INTEGER_ABSENT))
    );
}

#[test]
fn the_slider_reads_where_the_cursor_stands_in_the_list() {
    let config = Config::default();
    let bars = (0..12).map(|index| SongBar::new(BarKind::Folder, format!("Folder {index}"))).collect::<Vec<_>>();
    let shown = SelectShown::of(SelectInputs { cursor: 3, ..inputs(&bars, &config) });

    assert_eq!(state_of(&shown).rate(RATE_MUSICSELECT_POSITION), Some(0.25));
    assert_eq!(state_of(&shown).float(RATE_MUSICSELECT_POSITION), None, "a rate is asked as a rate, and the host falls back to it for a float");
    let last = SelectShown::of(SelectInputs { cursor: 11, ..inputs(&bars, &config) });
    assert_eq!(state_of(&last).rate(RATE_MUSICSELECT_POSITION), Some(11.0 / 12.0), "the last bar is short of the end, as the reference's is");
}

#[test]
fn the_filters_and_the_order_are_numbered_and_named_as_the_reference_names_them() {
    let config = Config::default();
    let bars = [song()];
    let with = |mode_filter: Option<Mode>, sort: SortMode| SelectShown::of(SelectInputs { mode_filter, sort, ..inputs(&bars, &config) });

    let all = with(None, SortMode::Title);
    assert_eq!((state_of(&all).image_index(BUTTON_MODE), state_of(&all).text(STRING_MODE_FILTER)), (Some(0), Some("ALL".into())));
    assert_eq!((state_of(&all).image_index(INDEX_DIFFICULTY_FILTER), state_of(&all).text(STRING_DIFFICULTY_FILTER)), (Some(0), Some("ALL".into())));
    let modes = [
        (Mode::BEAT_5K, 1, "5KEY"),
        (Mode::BEAT_7K, 2, "7KEY"),
        (Mode::BEAT_10K, 3, "10KEY"),
        (Mode::BEAT_14K, 4, "14KEY"),
        (Mode::POPN_9K, 5, "9KEY"),
        (Mode::KEYBOARD_24K, 6, "24KEY"),
    ];
    for (mode, number, name) in modes {
        let shown = with(Some(mode), SortMode::Title);
        assert_eq!((state_of(&shown).image_index(BUTTON_MODE), state_of(&shown).text(STRING_MODE_FILTER)), (Some(number), Some(name.into())), "{name}");
    }

    let orders = [
        (SortMode::Title, Some(0), "TITLE"),
        (SortMode::Artist, Some(1), "ARTIST"),
        (SortMode::Bpm, Some(2), "BPM"),
        (SortMode::Length, Some(3), "LENGTH"),
        (SortMode::Level, Some(4), "LEVEL"),
        (SortMode::Clear, Some(5), "CLEAR"),
        (SortMode::Score, Some(6), "SCORE"),
        (SortMode::MissCount, Some(7), "MISSCOUNT"),
        (SortMode::Duration, None, "DURATION"),
        (SortMode::LastUpdate, None, "LASTUPDATE"),
        (SortMode::RivalClear, None, "RIVALCOMPARE_CLEAR"),
        (SortMode::RivalScore, None, "RIVALCOMPARE_SCORE"),
        (SortMode::Default, None, ""),
    ];
    for (sort, number, name) in orders {
        let shown = with(None, sort);
        assert_eq!(state_of(&shown).image_index(BUTTON_SORT), Some(number.unwrap_or(SORT_UNNUMBERED)), "{sort:?}");
        assert!(state_of(&shown).image_index(BUTTON_SORT) >= Some(0), "{sort:?} hides the order switch, which takes the press with it");
        assert_eq!(state_of(&shown).text(STRING_SORT), Some(name.into()), "{sort:?}");
    }
}

#[test]
fn the_search_word_reads_empty_and_the_directory_is_the_path_of_the_open_folders() {
    let config = Config::default();
    let bars = [song()];
    let at = |directory: &[&str]| SelectShown::of(SelectInputs { directory: directory.iter().map(ToString::to_string).collect(), ..inputs(&bars, &config) });

    let root = at(&[]);
    assert_eq!((state_of(&root).text(STRING_SEARCHWORD), state_of(&root).text(STRING_DIRECTORY)), (Some("".into()), Some("".into())));
    let deep = at(&["Table", "LV 12"]);
    assert_eq!(state_of(&deep).text(STRING_DIRECTORY), Some("Table > LV 12 > ".into()));
}

#[test]
fn a_folder_names_itself_and_a_chart_leaves_its_title_to_the_chart() {
    let folder = shown_on(SongBar::new(BarKind::Folder, "Pop"), None);
    assert_eq!((state_of(&folder).text(STRING_TITLE), state_of(&folder).text(STRING_FULLTITLE)), (Some("Pop".into()), Some("Pop".into())));
    let chart = shown_on(song(), None);
    assert_eq!((state_of(&chart).text(STRING_TITLE), state_of(&chart).text(STRING_FULLTITLE)), (None, None));
}

#[test]
fn a_course_names_its_stages_and_the_rules_it_plays_under() {
    let config = Config::default();
    let bars = [SongBar::new(BarKind::Course { complete: false }, "Course")];
    let course = CourseShown::new(&[CourseConstraint::Class, CourseConstraint::NoSpeed, CourseConstraint::Hcn], [("Opener", true), ("Missing", false)]);
    let shown = SelectShown::of(SelectInputs { course: Some(course), ..inputs(&bars, &config) });
    let state = state_of(&shown);

    assert_eq!(
        [STRING_COURSE1_TITLE, STRING_COURSE2_TITLE, STRING_COURSE3_TITLE, STRING_COURSE10_TITLE].map(|id| state.text(id)),
        [Some("Opener".into()), Some("(no song) Missing".into()), Some("".into()), Some("".into())]
    );
    let on = CONSTRAINT_OPTIONS.iter().filter(|(id, _)| state.boolean(*id) == Some(true)).map(|(id, _)| *id).collect::<Vec<_>>();
    assert_eq!(on, [OPTION_GRADEBAR_CLASS, OPTION_GRADEBAR_NOSPEED, OPTION_GRADEBAR_HCN]);
    assert_eq!(CONSTRAINT_OPTIONS.len(), CourseConstraint::COUNT, "every constraint a course can have has an option");
}

#[test]
fn only_a_course_is_asked_for_its_stages_and_its_rules() {
    let config = Config::default();
    let bars = [song()];
    let course = CourseShown::new(&[CourseConstraint::Class], [("Opener", true)]);
    let shown = SelectShown::of(SelectInputs { course: Some(course), ..inputs(&bars, &config) });
    let state = state_of(&shown);

    assert_eq!(state.text(STRING_COURSE1_TITLE), Some("".into()));
    assert_eq!(state.boolean(OPTION_GRADEBAR_CLASS), Some(false));
}

#[test]
fn a_stage_that_the_library_does_not_hold_is_marked_as_the_reference_marks_it() {
    assert_eq!(stage_title("Opener", true), "Opener");
    assert_eq!(stage_title("Opener", false), "(no song) Opener");
}

#[test]
fn the_replay_options_ask_the_slot_and_the_saved_option_asks_the_opposite_in_the_browser() {
    let config = Config::default();
    let bars = [song()];
    let with = |replays_stored: usize, selected_replay: Option<usize>| {
        SelectShown::of(SelectInputs { chart: Some(ChartInputs { replays_stored, ..chart_with(None) }), selected_replay, ..inputs(&bars, &config) })
    };

    let stored = with(1, Some(0));
    let state = state_of(&stored);
    assert_eq!([OPTION_REPLAYDATA, OPTION_NO_REPLAYDATA, OPTION_REPLAYDATA_SAVED].map(|id| state.boolean(id)), [Some(true), Some(false), Some(false)]);
    assert_eq!(
        [OPTION_REPLAYDATA2, OPTION_NO_REPLAYDATA2, OPTION_REPLAYDATA2_SAVED].map(|id| state.boolean(id)),
        [Some(false), Some(true), Some(true)],
        "the other slots hold none"
    );
    assert_eq!([OPTION_REPLAYDATA3, OPTION_NO_REPLAYDATA3, OPTION_REPLAYDATA3_SAVED].map(|id| state.boolean(id)), [Some(false), Some(true), Some(true)]);
    assert_eq!([OPTION_REPLAYDATA4, OPTION_NO_REPLAYDATA4, OPTION_REPLAYDATA4_SAVED].map(|id| state.boolean(id)), [Some(false), Some(true), Some(true)]);

    let three = with(3, Some(0));
    let state = state_of(&three);
    assert_eq!(
        [OPTION_REPLAYDATA, OPTION_REPLAYDATA2, OPTION_REPLAYDATA3, OPTION_REPLAYDATA4].map(|id| state.boolean(id)),
        [Some(true), Some(true), Some(true), Some(false)],
        "a slot for each replay saved, the newest first"
    );
    let many = with(usize::MAX, Some(0));
    assert_eq!(state_of(&many).boolean(OPTION_REPLAYDATA4), Some(true));

    let none = with(0, Some(2));
    let state = state_of(&none);
    assert_eq!(
        [OPTION_REPLAYDATA, OPTION_NO_REPLAYDATA, OPTION_REPLAYDATA_SAVED].map(|id| state.boolean(id)),
        [Some(false), Some(true), Some(true)],
        "saved reads as none: the reference's quirk"
    );
    assert_eq!(
        [OPTION_SELECT_REPLAYDATA, OPTION_SELECT_REPLAYDATA2, OPTION_SELECT_REPLAYDATA3, OPTION_SELECT_REPLAYDATA4].map(|id| state.boolean(id)),
        [Some(false), Some(false), Some(true), Some(false)]
    );
}

/// A bar that opens holds no replay and has no slot selected: the reference's selected slot is -1
/// there, which is none of the four (`MusicSelectCommand.RESET_REPLAY`).
#[test]
fn a_bar_that_cannot_hold_a_replay_has_no_replay_option_on_and_no_slot_selected() {
    let folder = shown_on(SongBar::new(BarKind::Folder, ""), None);
    let state = state_of(&folder);

    for [exists, none, saved, selected] in REPLAY_OPTIONS {
        assert_eq!([exists, none, saved, selected].map(|id| state.boolean(id)), [Some(false); 4]);
    }
}

/// A chart with no replay saved has no slot selected either, and one with replays has the slot the
/// browser says.
#[test]
fn the_selected_slot_is_the_one_the_browser_holds_and_none_without_one() {
    let config = Config::default();
    let bars = [song()];
    let with = |replays_stored: usize, selected_replay: Option<usize>| {
        SelectShown::of(SelectInputs { chart: Some(ChartInputs { replays_stored, ..chart_with(None) }), selected_replay, ..inputs(&bars, &config) })
    };
    let selected = [OPTION_SELECT_REPLAYDATA, OPTION_SELECT_REPLAYDATA2, OPTION_SELECT_REPLAYDATA3, OPTION_SELECT_REPLAYDATA4];

    let unplayed = with(0, None);
    assert_eq!(selected.map(|id| state_of(&unplayed).boolean(id)), [Some(false); 4], "a chart with no replay lights the first slot");
    let second = with(2, Some(1));
    assert_eq!(selected.map(|id| state_of(&second).boolean(id)), [Some(false), Some(true), Some(false), Some(false)]);
}

#[test]
fn the_panel_is_left_to_others_until_the_browser_says_which_is_open() {
    let config = Config::default();
    let bars = [song()];
    let panel = |panel: Option<u8>| SelectShown::of(SelectInputs { panel, ..inputs(&bars, &config) });

    let unknown = panel(None);
    assert_eq!([OPTION_PANEL1, OPTION_PANEL2, OPTION_PANEL3].map(|id| state_of(&unknown).boolean(id)), [None; 3]);
    let closed = panel(Some(0));
    assert_eq!([OPTION_PANEL1, OPTION_PANEL2, OPTION_PANEL3].map(|id| state_of(&closed).boolean(id)), [Some(false); 3]);
    let second = panel(Some(2));
    assert_eq!([OPTION_PANEL1, OPTION_PANEL2, OPTION_PANEL3].map(|id| state_of(&second).boolean(id)), [Some(false), Some(true), Some(false)]);
}

#[test]
fn no_rival_is_picked_so_the_target_reads_as_the_reference_reads_a_missing_one() {
    let shown = shown_on(song(), Some(chart_with(None)));
    let state = state_of(&shown);

    assert_eq!((state.boolean(OPTION_NOT_COMPARE_RIVAL), state.boolean(OPTION_COMPARE_RIVAL)), (Some(true), Some(false)));
    assert_eq!(
        [OPTION_1PWIN, OPTION_2PWIN, OPTION_DRAW].map(|id| state.boolean(id)),
        [Some(false), Some(false), Some(true)],
        "nothing against nothing is a draw"
    );
    let record = best(ClearType::Hard);
    let played = shown_on(song(), Some(chart_with(Some(&record))));
    assert_eq!(
        [OPTION_1PWIN, OPTION_2PWIN, OPTION_DRAW].map(|id| state_of(&played).boolean(id)),
        [Some(true), Some(false), Some(false)],
        "anything against the rival's nothing wins"
    );
    assert_eq!(
        [NUMBER_RIVAL_SCORE, NUMBER_TARGET_SCORE, NUMBER_TARGET_SCORE2, NUMBER_HIGHSCORE, NUMBER_HIGHSCORE2, NUMBER_BEST_RATE].map(|id| state.integer(id)),
        [Some(0); 6]
    );
    assert_eq!(
        [NUMBER_TARGET_SCORE_RATE, NUMBER_TARGET_TOTAL_RATE, NUMBER_TARGET_SCORE_RATE2].map(|id| state.integer(id)),
        [Some(100); 3],
        "a rate over no notes is a whole one"
    );
    assert_eq!(
        [NUMBER_TARGET_SCORE_RATE_AFTERDOT, NUMBER_TARGET_TOTAL_RATE_AFTERDOT, NUMBER_TARGET_SCORE_RATE_AFTERDOT2].map(|id| state.integer(id)),
        [Some(0); 3]
    );
    assert_eq!(
        [NUMBER_TARGET_MAXCOMBO, NUMBER_DIFF_MAXCOMBO, NUMBER_TARGET_MISSCOUNT, NUMBER_DIFF_MISSCOUNT].map(|id| state.integer(id)),
        [Some(INTEGER_ABSENT); 4]
    );
    assert_eq!([FLOAT_RIVAL_RATE, FLOAT_TARGET_RATE, FLOAT_TARGET_RATE2].map(|id| state.float(id)), [Some(1.0); 3]);
    assert_eq!(state.float(FLOAT_BEST_RATE), Some(0.0));
    assert_eq!(
        [RATE_BESTSCORE_NOW, RATE_BESTSCORE, RATE_TARGETSCORE_NOW, RATE_TARGETSCORE].map(|id| state.rate(id)),
        [Some(0.0), Some(0.0), Some(0.0), Some(1.0)]
    );
    assert_eq!((state.rate(RATE_SCORE), state.rate(RATE_SCORE_FINAL)), (Some(1.0), Some(1.0)), "and so does the rate of a bar with no score");
    assert_eq!((state.image_index(INDEX_TARGET_CLEAR), state.image_index(BUTTON_FAVORITTE_SONG)), (Some(IMAGE_INDEX_ABSENT), Some(IMAGE_INDEX_ABSENT)));
}

#[test]
fn the_graphs_of_the_score_are_shares_of_the_notes_of_the_chart() {
    let record = best(ClearType::Normal);
    let shown = shown_on(song(), Some(chart_with(Some(&record))));
    let state = state_of(&shown);

    assert_eq!(
        [RATE_PGREAT, RATE_GREAT, RATE_GOOD, RATE_BAD, RATE_POOR].map(|id| state.rate(id)),
        [100.0, 30.0, 5.0, 2.0, 1.0].map(|count| Some(count / 141.0))
    );
    assert_eq!(state.rate(RATE_MAXCOMBO), Some(120.0 / 141.0));
    assert_eq!(state.rate(RATE_EXSCORE), Some(230.0 / 141.0 / 2.0));
    assert_eq!(state.rate(RATE_SCORE), Some(230.0 / 282.0));

    let none = shown_on(song(), Some(chart_with(None)));
    assert_eq!([RATE_PGREAT, RATE_MAXCOMBO, RATE_EXSCORE].map(|id| state_of(&none).rate(id)), [Some(0.0); 3]);
}

#[test]
fn a_star_is_a_chart_s_and_a_folder_has_none() {
    let config = Config::default();
    let bars = [song()];
    let starred = |favorite: bool| SelectShown::of(SelectInputs { chart: Some(ChartInputs { favorite, ..chart_with(None) }), ..inputs(&bars, &config) });

    assert_eq!(state_of(&starred(true)).image_index(BUTTON_FAVORITTE_CHART), Some(1));
    assert_eq!(state_of(&starred(false)).image_index(BUTTON_FAVORITTE_CHART), Some(0));
    assert_eq!(state_of(&shown_on(SongBar::new(BarKind::Folder, ""), None)).image_index(BUTTON_FAVORITTE_CHART), Some(IMAGE_INDEX_ABSENT));
}

#[test]
fn what_the_browser_holds_of_a_chart_follows_the_mode_it_is_played_in() {
    let record = best(ClearType::Normal);
    let in_mode = |mode: Mode| shown_on(song(), Some(ChartInputs { mode, ..chart_with(Some(&record)) }));

    let seven_points = state_of(&in_mode(Mode::BEAT_7K)).integer(NUMBER_POINT).expect("answered");
    let five_points = state_of(&in_mode(Mode::BEAT_5K)).integer(NUMBER_POINT).expect("answered");
    assert_eq!(five_points, (100_000 * 100 + 100_000 * 30 + 50_000 * 5) / 141, "a five key chart weighs a great as much as a perfect");
    assert_ne!(seven_points, five_points);
}

#[test]
fn the_settings_and_the_ranking_are_read_against_the_bar_under_the_cursor() {
    let config = Config::default();
    let board = IrBrowser {
        online: true,
        board: Some(crate::skin_host::ir::IrBoard { access: crate::skin_host::ir::IrAccess::Finished, rank: 3, total: 9, lamps: [0; 11] }),
        offset: 0,
    };
    let on = |kind: BarKind| {
        let bars = [SongBar::new(kind, "")];
        SelectShown::of(SelectInputs { ir: board, ..inputs(&bars, &config) })
    };

    let chart = on(BarKind::Song { exists: true });
    assert!(chart.settings.chart.is_some() && chart.ir.board.is_some());
    let missing = on(BarKind::Song { exists: false });
    assert!(missing.settings.chart.is_some(), "a chart off the disk still has the tempo it would be played at");
    assert!(missing.ir.board.is_none(), "but it is not asked of the service");
    let folder = on(BarKind::Folder);
    assert!(folder.settings.chart.is_none() && folder.ir.board.is_none());
    assert!(folder.ir.online, "whether there is a service does not depend on the bar");
    let incomplete = on(BarKind::Course { complete: false });
    assert!(incomplete.settings.chart.is_none() && incomplete.ir.board.is_none());
    let course = on(BarKind::Course { complete: true });
    assert!(course.settings.chart.is_some() && course.ir.board.is_some());
}

#[test]
fn the_ids_the_browser_answers_are_routed_to_it_and_the_ones_it_handed_over_are_not() {
    for id in [INDEX_DIFFICULTY_FILTER, BUTTON_MODE, BUTTON_SORT] {
        assert_eq!(clusters_of(IdSpace::ImageIndex, id).collect::<Vec<_>>(), [Cluster::Select], "image index {id}");
    }
    for id in [OPTION_IR_NOPLAYER, OPTION_IR_FAILED, OPTION_IR_WAITING, OPTION_IR_BUSY] {
        assert_eq!(clusters_of(IdSpace::Boolean, id).collect::<Vec<_>>(), [Cluster::Ir], "option {id}");
    }
    assert_eq!(clusters_of(IdSpace::Text, STRING_DIRECTORY).collect::<Vec<_>>(), [Cluster::Select]);
    assert_eq!(clusters_of(IdSpace::Boolean, OPTION_REPLAYDATA).collect::<Vec<_>>(), [Cluster::Result, Cluster::Select]);
    assert_eq!(clusters_of(IdSpace::Boolean, OPTION_SELECT_REPLAYDATA2).collect::<Vec<_>>(), [Cluster::Select]);
    assert_eq!(clusters_of(IdSpace::ImageIndex, INDEX_CLEAR).collect::<Vec<_>>(), [Cluster::Result, Cluster::Select]);
    assert_eq!(clusters_of(IdSpace::ImageIndex, BUTTON_FAVORITTE_CHART).collect::<Vec<_>>(), [Cluster::Select, Cluster::Options]);
    assert_eq!(clusters_of(IdSpace::Text, STRING_TARGET_NAME).collect::<Vec<_>>(), [Cluster::Options]);
}

#[test]
fn the_host_answers_the_browser_through_every_cluster_it_lends_a_frame_to() {
    let record = best(ClearType::Hard);
    let config = Config::default();
    let bars = [song(), SongBar::new(BarKind::Folder, "Folder")];
    let shown = SelectShown::of(SelectInputs {
        chart: Some(chart_with(Some(&record))),
        ir: IrBrowser { online: true, board: None, offset: 0 },
        ..inputs(&bars, &config)
    });
    let timers = TimerState::new();
    let mut host = ScreenHost::new(0, &timers);
    host.show_select(&shown);

    assert_eq!(host.integer(NUMBER_SCORE), 230, "the score cluster has no run and the browser's score is read through the fall through");
    assert_eq!(host.boolean(OPTION_SONGBAR), Some(true));
    assert_eq!(host.boolean(-OPTION_SONGBAR), Some(false));
    assert_eq!(host.image_index(BUTTON_GAUGE_1P), 2, "the gauge the player chose is the settings cluster's");
    assert_eq!(host.boolean(OPTION_ONLINE), Some(true), "and whether a service is set up is the ranking cluster's");
    assert_eq!(host.boolean(OPTION_IR_WAITING), Some(true));
    assert_eq!(host.rate(RATE_MUSICSELECT_POSITION), Some(0.0));
    assert_eq!(host.text(STRING_SORT), "TITLE");
    assert_eq!(host.image_index(INDEX_CLEAR), i32::from(clear_type_id(ClearType::Hard)));
    assert_eq!(host.integer(NUMBER_FOLDER_TOTALSONGS), INTEGER_ABSENT, "a chart has no folder to count");
}

/// The ids `m4-select.md` section 6 lists, by what the browser's clusters read them from: each is
/// answered by some cluster of a host that is lent the frame of a played chart, a ranking and the
/// player's settings. An answer may be the absent value of its kind; what it may not be is nobody's.
#[test]
fn every_id_the_browser_skin_reads_that_the_browser_owns_has_a_cluster_that_answers_it() {
    let record = best(ClearType::Hard);
    let config = Config::default();
    let bars = [song()];
    let ranking = crate::ir_ranking::RankingState::Loading;
    let shown = SelectShown::of(SelectInputs {
        chart: Some(chart_with(Some(&record))),
        panel: Some(1),
        ir: IrBrowser::of_ranking(true, Some(&ranking)),
        ..inputs(&bars, &config)
    });
    let timers = TimerState::new();
    let mut host = ScreenHost::new(0, &timers);
    host.show_select(&shown);
    let answered = |space: IdSpace, id: i32| {
        Cluster::ALL.into_iter().any(|cluster| {
            let state = host.cluster(cluster);
            match space {
                IdSpace::Boolean => state.boolean(id).is_some(),
                IdSpace::Integer => state.integer(id).is_some(),
                IdSpace::ImageIndex => state.image_index(id).is_some(),
                IdSpace::Rate => state.rate(id).is_some(),
                IdSpace::Float => state.float(id).is_some(),
                IdSpace::Text => state.text(id).is_some(),
                IdSpace::Offset => state.offset(id).is_some(),
            }
        })
    };

    let options = [
        1, 2, 3, 5, 21, 22, 23, 50, 51, 100, 101, 102, 103, 104, 105, 1100, 1101, 1102, 1103, 1104, 197, 1197, 1200, 1203, 1205, 1206, 1207, 1208, 200, 201,
        202,
    ];
    let more_options = [
        203, 204, 205, 206, 207, 352, 353, 603, 604, 606, 608, 624, 625, 1002, 1003, 1004, 1005, 1006, 1007, 1010, 1011, 1012, 1013, 1014, 1015, 1016, 1017,
        1030, 1031,
    ];
    for id in options.into_iter().chain(more_options) {
        assert!(answered(IdSpace::Boolean, id), "option {id}");
    }
    let numbers =
        [12, 71, 76, 77, 78, 79, 102, 103, 110, 111, 112, 113, 114, 179, 180, 182, 220, 271, 280, 281, 282, 283, 284, 300, 310, 311, 312, 313, 423, 424, 425];
    for id in numbers.into_iter().chain(202..=220).chain(222..=242).chain(320..=330).chain(380..=399) {
        assert!(answered(IdSpace::Integer, id), "number {id}");
    }
    for id in [
        10, 11, 12, 40, 42, 43, 54, 55, 72, 75, 78, 89, 90, 301, 302, 303, 304, 305, 306, 307, 308, 321, 322, 323, 324, 330, 331, 332, 340, 341, 342, 370, 371,
        400,
    ] {
        assert!(answered(IdSpace::ImageIndex, id), "image index {id}");
    }
    for id in [1, 8, 110, 111, 112, 113, 114, 115, 140, 141, 142, 143, 144, 145, 147] {
        assert!(answered(IdSpace::Rate, id), "rate {id}");
    }
    for id in [1, 3, 30, 60, 61, 62, 120, 129, 150, 159, 200, 209, 210, 219, 1000, 1020, 1021] {
        assert!(answered(IdSpace::Text, id), "text {id}");
    }
}

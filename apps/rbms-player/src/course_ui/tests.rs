use std::path::Path;

use rbms_chart::shuffle::NoteOption;
use rbms_config::PlayOptions;
use rbms_course::{Course, CourseChart, CourseConstraint, CourseRun, StageResult, TrophyRule};
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;
use rbms_library::{Library, SongEntry};
use rbms_model::Mode;

use super::{COURSE_DIR_NAME, CourseList, CourseOverrides, RandomLock, SelectTab, constraint_badge, courses_dir, library_index, result_rows, stage_label};

const TEST_MD5_PRESENT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TEST_MD5_ABSENT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const TEST_NOTES: u32 = 1_000;
const TEST_EX: u32 = 1_400;
const TEST_LAMP_HARD: u8 = 6;

fn chart(md5: &str, title: &str) -> CourseChart {
    CourseChart { md5: md5.to_string(), sha256: String::new(), title: title.to_string() }
}

fn course(name: &str, charts: Vec<CourseChart>, constraints: Vec<CourseConstraint>) -> Course {
    Course { name: name.to_string(), charts, constraints, ..Course::default() }
}

fn library_with(md5: &str) -> Library {
    Library::from_songs(vec![SongEntry {
        path: Path::new("/songs/one.bms").to_path_buf(),
        title: "One".to_string(),
        subtitle: String::new(),
        artist: String::new(),
        genre: String::new(),
        maker: String::new(),
        level: "12".to_string(),
        difficulty: 3,
        init_bpm: 150.0,
        rank: 2,
        total: 300.0,
        mode: Mode::BEAT_7K,
        md5: md5.to_string(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }])
}

fn stage(ex: u32, gauge: f32, survived: bool) -> StageResult {
    StageResult {
        ex_score: ex,
        max_ex_score: TEST_NOTES * 2,
        notes: TEST_NOTES,
        counts: [0, 0, 0, 10, 20, 20],
        empty_poor: 0,
        fast: 0,
        slow: 0,
        combo_breaks: 0,
        max_combo: 300,
        combo_at_end: 10,
        gauge_value: gauge,
        clear: if survived { TEST_LAMP_HARD } else { rbms_course::CLEAR_FAILED },
        survived,
    }
}

#[test]
fn the_two_tabs_cycle_and_are_named() {
    assert_eq!(SelectTab::default(), SelectTab::Songs);
    assert_eq!(SelectTab::Songs.next(), SelectTab::Courses);
    assert_eq!(SelectTab::Courses.next(), SelectTab::Songs);
    assert_eq!(SelectTab::Songs.label(), "SONGS");
    assert_eq!(SelectTab::Courses.label(), "COURSE");
}

#[test]
fn courses_live_under_the_config_directory() {
    assert_eq!(courses_dir(Path::new("/home/p/.config/rbms")), Path::new("/home/p/.config/rbms").join(COURSE_DIR_NAME));
}

#[test]
fn a_course_is_playable_only_when_every_stage_resolves() {
    let library = library_with(TEST_MD5_PRESENT);
    let mut list = CourseList::from_courses(vec![
        course("All Present", vec![chart(TEST_MD5_PRESENT, "One")], vec![]),
        course("One Missing", vec![chart(TEST_MD5_PRESENT, "One"), chart(TEST_MD5_ABSENT, "Two")], vec![]),
    ]);
    list.resolve_in_library(&library);

    assert!(list.entries()[0].is_playable());
    assert!(!list.entries()[1].is_playable());
    assert_eq!(list.entries()[1].missing, vec![1], "the second stage is the one the library lacks");
}

#[test]
fn a_chart_known_only_by_sha256_does_not_resolve_yet() {
    let library = library_with(TEST_MD5_PRESENT);
    let sha_only = CourseChart { md5: String::new(), sha256: "ff".repeat(32), title: "One".to_string() };
    assert_eq!(library_index(&library, &sha_only), None);
    assert_eq!(library_index(&library, &chart(TEST_MD5_PRESENT, "One")), Some(0));
    assert_eq!(library_index(&library, &chart(TEST_MD5_ABSENT, "Two")), None);
}

#[test]
fn a_row_reports_the_stage_count_the_badges_and_whether_it_can_start() {
    let library = library_with(TEST_MD5_PRESENT);
    let mut list = CourseList::from_courses(vec![
        course("Grade", vec![chart(TEST_MD5_PRESENT, "One"), chart(TEST_MD5_PRESENT, "Two")], vec![CourseConstraint::Class, CourseConstraint::NoSpeed]),
        Course { release: false, ..course("Draft", vec![chart(TEST_MD5_ABSENT, "One")], vec![]) },
    ]);
    list.resolve_in_library(&library);

    let rows = list.rows();
    assert_eq!(rows[0].title, "Grade");
    assert_eq!(rows[0].detail, "2 STAGES");
    assert_eq!(rows[0].badges, vec!["GRADE", "NO SPEED"]);
    assert!(rows[0].playable);

    assert_eq!(rows[1].detail, "1 STAGES - 1 MISSING");
    assert_eq!(rows[1].badges, vec!["UNRELEASED"]);
    assert!(!rows[1].playable);
}

#[test]
fn every_constraint_has_a_badge_of_its_own() {
    let mut badges: Vec<&str> = CourseConstraint::ALL.iter().map(|c| constraint_badge(*c)).collect();
    badges.sort_unstable();
    let before = badges.len();
    badges.dedup();
    assert_eq!(badges.len(), before, "no two constraints draw the same badge");
    assert!(badges.iter().all(|b| !b.is_empty()));
}

#[test]
fn the_cursor_wraps_at_both_ends_and_an_empty_list_stays_put() {
    let mut list = CourseList::from_courses(vec![
        course("A", vec![chart(TEST_MD5_PRESENT, "One")], vec![]),
        course("B", vec![chart(TEST_MD5_PRESENT, "One")], vec![]),
        course("C", vec![chart(TEST_MD5_PRESENT, "One")], vec![]),
    ]);
    assert_eq!(list.cursor(), 0);
    list.move_cursor(-1);
    assert_eq!(list.cursor(), 2, "moving up from the first row wraps to the last");
    list.move_cursor(1);
    assert_eq!(list.cursor(), 0);
    list.focus(1);
    assert_eq!(list.focused().map(|e| e.course.name.as_str()), Some("B"));
    list.focus(99);
    assert_eq!(list.cursor(), 1, "a row that is not there is ignored");

    let mut empty = CourseList::default();
    empty.move_cursor(1);
    assert_eq!(empty.cursor(), 0);
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert!(empty.focused().is_none());
}

#[test]
fn a_grade_course_plays_the_chart_as_written() {
    let lock = RandomLock::Off;
    for option in NoteOption::ALL {
        assert_eq!(lock.clamp(option), NoteOption::Off, "{option:?}");
    }
    assert!(lock.allows(NoteOption::Off));
    assert!(!lock.allows(NoteOption::Random));
}

#[test]
fn a_mirror_grade_course_keeps_mirror_and_drops_the_rest() {
    let lock = RandomLock::Mirror;
    assert_eq!(lock.clamp(NoteOption::Mirror), NoteOption::Mirror);
    assert_eq!(lock.clamp(NoteOption::Off), NoteOption::Off);
    assert_eq!(lock.clamp(NoteOption::Random), NoteOption::Off);
    assert_eq!(lock.clamp(NoteOption::HRandom), NoteOption::Off);
}

#[test]
fn a_random_grade_course_allows_everything_through_spiral() {
    let lock = RandomLock::Random;
    for option in [NoteOption::Off, NoteOption::Mirror, NoteOption::Random, NoteOption::SRandom, NoteOption::RRandom, NoteOption::Rotate] {
        assert_eq!(lock.clamp(option), option, "{option:?} is inside the reference's id ceiling");
    }
    assert_eq!(lock.clamp(NoteOption::HRandom), NoteOption::Off);
    assert_eq!(lock.clamp(NoteOption::AllScratch), NoteOption::Off);
}

#[test]
fn a_course_without_a_grade_constraint_leaves_the_note_option_alone() {
    for option in NoteOption::ALL {
        assert_eq!(RandomLock::Free.clamp(option), option);
    }
}

#[test]
fn the_gauge_constraints_each_select_their_table() {
    let cases = [
        (CourseConstraint::GaugeLr2, GaugeSetId::Lr2),
        (CourseConstraint::Gauge5Keys, GaugeSetId::FiveKeys),
        (CourseConstraint::Gauge7Keys, GaugeSetId::SevenKeys),
        (CourseConstraint::Gauge9Keys, GaugeSetId::Pms),
        (CourseConstraint::Gauge24Keys, GaugeSetId::Keyboard),
    ];
    for (constraint, set) in cases {
        let overrides = CourseOverrides::of(&course("C", vec![chart(TEST_MD5_PRESENT, "One")], vec![constraint]));
        assert_eq!(overrides.gauge_set, Some(set), "{constraint:?}");
        assert_eq!(overrides.gauge_set_for(Some(GaugeSetId::SevenKeys)), Some(set), "the course wins over the setting");
    }
    let none = CourseOverrides::default();
    assert_eq!(none.gauge_set_for(Some(GaugeSetId::Pms)), Some(GaugeSetId::Pms), "no constraint keeps the setting");
    assert_eq!(none.gauge_set_for(None), None);
}

#[test]
fn the_long_note_constraints_each_force_their_flavour() {
    let cases = [(CourseConstraint::Ln, LnMode::LongNote), (CourseConstraint::Cn, LnMode::ChargeNote), (CourseConstraint::Hcn, LnMode::HellChargeNote)];
    for (constraint, mode) in cases {
        let overrides = CourseOverrides::of(&course("C", vec![chart(TEST_MD5_PRESENT, "One")], vec![constraint]));
        assert_eq!(overrides.ln_mode, Some(mode), "{constraint:?}");
        assert_eq!(overrides.ln_mode_for(LnMode::LongNote), mode);
    }
    assert_eq!(CourseOverrides::default().ln_mode_for(LnMode::HellChargeNote), LnMode::HellChargeNote);
}

#[test]
fn the_judge_constraints_are_recorded_as_not_yet_enforced() {
    let overrides = CourseOverrides::of(&course("C", vec![chart(TEST_MD5_PRESENT, "One")], vec![CourseConstraint::NoGood, CourseConstraint::NoGreat]));
    assert_eq!(overrides.unapplied, vec!["NO GOOD", "NO GREAT"]);
    assert!(CourseOverrides::default().unapplied.is_empty());
}

#[test]
fn a_no_speed_course_pins_the_scroll_and_the_lane_shades() {
    let overrides = CourseOverrides::of(&course("C", vec![chart(TEST_MD5_PRESENT, "One")], vec![CourseConstraint::NoSpeed]));
    assert!(overrides.speed_locked);
    assert!(!overrides.accepts_speed_input());

    let mut play = PlayOptions { hispeed: 4.5, constant_speed: true, cover: 0.3, lift: 0.2, hidden: 0.1, random: NoteOption::HRandom, ..Default::default() };
    play.enable_cover = true;
    play.enable_lift = true;
    play.enable_hidden = true;
    overrides.apply_to(&mut play);

    assert_eq!(play.hispeed, 1.0);
    assert!(!play.constant_speed);
    assert_eq!(play.cover, 0.0);
    assert_eq!(play.lift, 0.0);
    assert_eq!(play.hidden, 0.0);
    assert!(!play.enable_cover && !play.enable_lift && !play.enable_hidden);
    assert_eq!(play.random, NoteOption::HRandom, "a course that says nothing about randomisation leaves it alone");
}

#[test]
fn a_course_that_only_grades_leaves_the_scroll_alone() {
    let overrides = CourseOverrides::of(&course("C", vec![chart(TEST_MD5_PRESENT, "One")], vec![CourseConstraint::Class]));
    let mut play = PlayOptions { hispeed: 4.5, random: NoteOption::Random, ..Default::default() };
    overrides.apply_to(&mut play);
    assert_eq!(play.hispeed, 4.5);
    assert_eq!(play.random, NoteOption::Off);
    assert!(overrides.accepts_speed_input());
}

#[test]
fn the_stage_line_counts_from_one() {
    let mut run = CourseRun::new(course("C", vec![chart(TEST_MD5_PRESENT, "One"), chart(TEST_MD5_PRESENT, "Two")], vec![]), 100.0);
    assert_eq!(stage_label(&run), "STAGE 1 / 2");
    run.advance(&stage(TEST_EX, 80.0, true));
    assert_eq!(stage_label(&run), "STAGE 2 / 2");
}

#[test]
fn the_result_rows_report_the_totals_and_the_stage_a_run_failed_on() {
    let mut run = CourseRun::new(course("C", vec![chart(TEST_MD5_PRESENT, "One"), chart(TEST_MD5_PRESENT, "Two")], vec![]), 100.0);
    run.advance(&stage(TEST_EX, 80.0, true));
    run.advance(&stage(100, 0.0, false));

    let rows = result_rows(&run);
    let labels: Vec<&str> = rows.iter().map(|(label, _)| label.as_str()).collect();
    assert_eq!(labels, vec!["EX SCORE", "SCORE RATE", "MAX COMBO", "MISS COUNT", "GAUGE", "FAILED AT"]);
    assert_eq!(rows[0].1, "1500 / 4000");
    assert_eq!(rows[5].1, "STAGE 2");
}

#[test]
fn a_cleared_run_that_earns_a_trophy_reports_it() {
    let with_trophy = Course {
        trophies: vec![TrophyRule { name: "bronzemedal".to_string(), missrate: 10.0, scorerate: 55.0 }],
        ..course("C", vec![chart(TEST_MD5_PRESENT, "One")], vec![])
    };
    let mut run = CourseRun::new(with_trophy, 100.0);
    run.advance(&stage(TEST_EX, 82.0, true));

    let rows = result_rows(&run);
    assert_eq!(rows.last().map(|(label, value)| (label.as_str(), value.as_str())), Some(("TROPHY", "bronzemedal")));
    assert!(rows.iter().all(|(label, _)| label != "FAILED AT"));
}

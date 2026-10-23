use rbms_skin::lua::LocalTime;

use super::*;
use crate::skin_host::extension::{
    RBMS_NUMBER_FOLDER_LAMP_COUNT_LAST, RBMS_NUMBER_MY_RECORD_LAST, RBMS_NUMBER_SIBLING_CHART_LEVEL_LAST, RBMS_OPTION_SIBLING_CHART_LAST,
    RBMS_RATE_FOLDER_LAMP_LAST, RBMS_TEXT_MY_RECORD_DATE_LAST,
};

/// A record row whose figures are `ex`, `lamp`, `miss`, `level` and whose date is `date`.
fn row(ex: i32, lamp: i32, miss: i32, level: i32, date: &str) -> Option<RecordRow> {
    Some(RecordRow { ex_score: ex, lamp, miss_count: miss, dj_level: level, date: date.to_string() })
}

/// Charts in the first and third slots, ten charts in the folder, and the best and one later play.
fn facts() -> BrowseFacts {
    BrowseFacts {
        siblings: [Some(SiblingChart { level: 3 }), None, Some(SiblingChart { level: 9 }), None, None],
        folder_lamps: [4, 1, 0, 0, 0, 2, 0, 0, 0, 0, 3],
        records: [row(1500, 6, 12, 5, "2023-11-14 22:13"), row(1400, 5, 20, 4, "2023-11-13 08:00"), None, None, None, None],
    }
}

/// The ids of a run of `count` that starts at `first`.
fn ids(first: i32, count: usize) -> impl Iterator<Item = i32> {
    (0..count).map(move |place| first + i32::try_from(place).expect("a place fits"))
}

#[test]
fn a_slot_with_a_chart_reads_on_and_at_its_level_and_an_empty_one_reads_off_and_absent() {
    let facts = facts();

    let on: Vec<_> = ids(RBMS_OPTION_SIBLING_CHART_FIRST, SIBLING_SLOTS).map(|id| facts.boolean(id)).collect();
    assert_eq!(on, [Some(true), Some(false), Some(true), Some(false), Some(false)]);
    let levels: Vec<_> = ids(RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST, SIBLING_SLOTS).map(|id| facts.integer(id)).collect();
    assert_eq!(levels, [Some(3), Some(INTEGER_ABSENT), Some(9), Some(INTEGER_ABSENT), Some(INTEGER_ABSENT)]);
}

#[test]
fn the_charts_of_the_folder_are_counted_by_lamp_and_added_up() {
    let facts = facts();

    let counts: Vec<_> = ids(RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST, LAMP_KINDS).map(|id| facts.integer(id)).collect();
    assert_eq!(counts, [4, 1, 0, 0, 0, 2, 0, 0, 0, 0, 3].map(Some));
    assert_eq!(facts.integer(RBMS_NUMBER_FOLDER_CHART_COUNT), Some(10));
    assert_eq!(facts.folder_chart_count(), 10);
    let shares: Vec<_> = ids(RBMS_RATE_FOLDER_LAMP_FIRST, LAMP_KINDS).map(|id| facts.rate(id)).collect();
    assert_eq!((shares[0], shares[1], shares[5], shares[10]), (Some(0.4), Some(0.1), Some(0.2), Some(0.3)));
    assert_eq!(shares[2], Some(0.0));
}

#[test]
fn a_folder_with_no_chart_holds_none_on_any_lamp_and_every_share_of_it_is_zero() {
    let facts = BrowseFacts::default();

    assert!(ids(RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST, LAMP_KINDS).all(|id| facts.integer(id) == Some(0)));
    assert_eq!(facts.integer(RBMS_NUMBER_FOLDER_CHART_COUNT), Some(0));
    assert!(ids(RBMS_RATE_FOLDER_LAMP_FIRST, LAMP_KINDS).all(|id| facts.rate(id) == Some(0.0)), "no share is a division by nothing");
}

#[test]
fn a_record_row_reads_its_four_figures_and_its_date_and_a_row_with_no_play_reads_absent() {
    let facts = facts();
    let figure = |first: i32, row: usize| facts.integer(first + i32::try_from(row).expect("a row fits"));

    assert_eq!(figure(RBMS_NUMBER_MY_RECORD_EX_FIRST, 0), Some(1500));
    assert_eq!(figure(RBMS_NUMBER_MY_RECORD_LAMP_FIRST, 0), Some(6));
    assert_eq!(figure(RBMS_NUMBER_MY_RECORD_MISS_COUNT_FIRST, 0), Some(12));
    assert_eq!(figure(RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST, 0), Some(5));
    assert_eq!(figure(RBMS_NUMBER_MY_RECORD_EX_FIRST, 1), Some(1400));
    assert_eq!(figure(RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST, 1), Some(4));
    for first in RECORD_FIGURES {
        assert_eq!(figure(first, 2), Some(INTEGER_ABSENT), "figure {first} of a row with no play");
        assert_eq!(figure(first, RECORD_ROWS - 1), Some(INTEGER_ABSENT));
    }
    let date = |row: usize| facts.text(RBMS_TEXT_MY_RECORD_DATE_FIRST + i32::try_from(row).expect("a row fits"));
    assert_eq!(date(0).as_deref(), Some("2023-11-14 22:13"));
    assert_eq!(date(1).as_deref(), Some("2023-11-13 08:00"));
    assert_eq!(date(2).as_deref(), Some(TEXT_ABSENT));
}

#[test]
fn an_id_outside_the_runs_is_nobody_s_answer() {
    let facts = facts();

    assert_eq!(facts.boolean(RBMS_OPTION_SIBLING_CHART_FIRST - 1), None);
    assert_eq!(facts.boolean(RBMS_OPTION_SIBLING_CHART_LAST + 1), None);
    assert_eq!(facts.integer(RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST - 1), None);
    assert_eq!(facts.integer(RBMS_NUMBER_SIBLING_CHART_LEVEL_LAST + 1), None);
    assert_eq!(facts.integer(RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST - 1), None);
    assert_eq!(facts.integer(RBMS_NUMBER_FOLDER_LAMP_COUNT_LAST), Some(3));
    assert_eq!(facts.integer(RBMS_NUMBER_FOLDER_CHART_COUNT + 1), None);
    assert_eq!(facts.integer(RBMS_NUMBER_MY_RECORD_EX_FIRST - 1), None);
    assert_eq!(facts.integer(RBMS_NUMBER_MY_RECORD_LAST + 1), None);
    assert_eq!(facts.rate(RBMS_RATE_FOLDER_LAMP_FIRST - 1), None);
    assert_eq!(facts.rate(RBMS_RATE_FOLDER_LAMP_LAST + 1), None);
    assert_eq!(facts.text(RBMS_TEXT_MY_RECORD_DATE_FIRST - 1), None);
    assert_eq!(facts.text(RBMS_TEXT_MY_RECORD_DATE_LAST + 1), None);
    assert_eq!(facts.integer(i32::MIN), None, "an id far below the runs does not wrap round");
    assert_eq!(facts.boolean(i32::MIN), None);
}

/// A sheet of 100 notes: 60 PGREAT and 20 GREAT split between the early and the late side, which is
/// 140 of the 200 EX there are, a DJ LEVEL of A, and 9 bad, poor and miss.
fn sheet() -> ScoreSheet {
    ScoreSheet { early: [30, 10, 0, 0, 0, 0], late: [30, 10, 0, 0, 0, 0], notes: 100, min_bp: 9, clear: 5, ..ScoreSheet::default() }
}

#[test]
fn a_record_row_is_the_figures_of_a_sheet_and_the_moment_it_was_made() {
    let made = 1_700_000_000;
    let row = RecordRow::of(&sheet(), Some(made));

    assert_eq!((row.ex_score, row.lamp, row.miss_count), (140, 5, 9));
    assert_eq!(row.dj_level, 5, "140 of 200 is 7/10, which is A");
    assert_eq!(Some(row.date.as_str()), local_time(made).map(clock_text).as_deref(), "the moment is read in the machine's own time");
    assert!(!row.date.is_empty());
}

#[test]
fn a_record_row_with_no_moment_to_name_has_no_date() {
    assert_eq!(RecordRow::of(&sheet(), None).date, "");
    assert_eq!(RecordRow::of(&sheet(), Some(0)).date, "", "a play of the epoch itself is one that was never dated");
    assert_eq!(RecordRow::of(&sheet(), Some(-1)).date, "");
}

#[test]
fn a_moment_is_written_to_the_minute_the_way_the_record_list_writes_it() {
    let time = LocalTime { year: 2023, month: 11, day: 4, hour: 2, minute: 7, second: 59 };

    assert_eq!(clock_text(time), "2023-11-04 02:07");
}

#[test]
fn a_sheet_with_no_note_is_the_lowest_level_and_an_ex_of_nothing() {
    let row = RecordRow::of(&ScoreSheet::default(), None);

    assert_eq!((row.ex_score, row.dj_level), (0, 0));
}

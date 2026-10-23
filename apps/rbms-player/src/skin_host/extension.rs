//! The ids this player answers beyond the reference's own: every one of them, and what it means.
//!
//! The host answers the ids the reference declares (`SkinProperty`), and a skin written for the
//! reference never names anything else. A few things this player has are not in the reference at
//! all -- SCRATCH AUTO, KEEP GREEN NUMBER as a switch, the assist grade inside a light-assist lamp,
//! which keys of the browser's table a controller can reach, the NOTES RADAR of a chart, the
//! player's own records of it -- and a skin that wants to show or switch one needs an id for it.
//! They are kept in bands of this player's own, well above everything the reference declares (its
//! highest option is 3035 and its highest event 388) and clear of the bands it leaves to a skin
//! (options 900 to 999, events 1000 to 1999, timers 10000 to 19999). The bands themselves are
//! `rbms_skin::property::extension`'s, because the gates that check an id against the reference
//! live there and in the renderer; this file names the ids inside them:
//!
//! - events [`EXTENSION_EVENT_FIRST`] to [`EXTENSION_EVENT_LAST`], which a skin runs as it runs any
//!   event, by `main_state.event_exec(id)` or a button's `act`. The event space has no gate on what
//!   the reference declares, so an id here reaches the cluster [`super::EVENT_ROUTES`] sends it to.
//! - options [`EXTENSION_OPTION_FIRST`] to [`EXTENSION_OPTION_LAST`], which a skin reads with
//!   `main_state.option(id)` from a function -- a `draw`, a `value`, a `text` -- and only so: the
//!   loader keeps an `op` list to the ids the reference declares, and drops one of these from it.
//!   An option here is on or off, and a negative id reads the negation, as every option does.
//! - numbers, rates, floats and texts [`EXTENSION_VALUE_FIRST`] to [`EXTENSION_VALUE_LAST`], each
//!   space on its own: a skin reads them as it reads the reference's, by `main_state.number`,
//!   `float_number` and `text` or by an object's `ref` or `value`. A float read that the float space
//!   does not answer is asked of the rates, so a number of this band names a float or a rate and
//!   never both.
//! - reference images [`EXTENSION_IMAGE_FIRST`] to [`EXTENSION_IMAGE_LAST`], placed by the negated
//!   id as a destination id, as the reference's own stage file and banner are.
//!
//! What a switch the reference already has an id for reads and runs by that id, not by one of
//! these: the lane cover, the lift and the HIDDEN+ cover are switched by the events 330 to 332 and
//! read by the image indices 330 to 332 as the reference has them. The three are also answered as
//! options here, so a skin can read every switch of a panel the one way.
//!
//! Each band is a range of numbers and nothing else holds them: a number that is in a band and not
//! named below reads as no option, runs as no event and reads as the absent value of its kind. A
//! name below whose value nothing fills yet reads the same way, and a skin draws nothing for it.

use rbms_judge::AssistGrade;
use rbms_model::Mode;
use rbms_render::skin_render::frame::LAMP_KINDS;

pub use rbms_skin::property::extension::{
    EXTENSION_EVENT_FIRST, EXTENSION_EVENT_LAST, EXTENSION_IMAGE_FIRST, EXTENSION_IMAGE_LAST, EXTENSION_OPTION_FIRST, EXTENSION_OPTION_LAST,
    EXTENSION_VALUE_FIRST, EXTENSION_VALUE_LAST, RBMS_IMAGE_NOTES_RADAR,
};

use super::result::snapshot::REPLAY_SLOT_COUNT;
use crate::keyconfig::KEY_INDEX_COUNT;

/// The last id of a run of `count` ids that starts at `first`.
pub const fn run_last(first: i32, count: i32) -> i32 {
    first + count - 1
}

/// Switches SCRATCH AUTO on or off (the PLAY menu's row of that name), whichever way its first
/// argument points.
pub const RBMS_EVENT_SCRATCH_AUTO: i32 = 21_000;

/// Switches KEEP GREEN NUMBER on or off (the PLAY menu's row of that name): whether the green number,
/// rather than the scroll speed, is held from chart to chart.
pub const RBMS_EVENT_KEEP_GREEN_NUMBER: i32 = 21_001;

/// Opens the song browser's ranking mode, or closes it when it is open.
pub const RBMS_EVENT_RANKING_MODE: i32 = 21_002;

/// How many replay slots the browser keeps for a chart, the reference's `BUTTON_REPLAY` to
/// `BUTTON_REPLAY4`.
pub const REPLAY_PATTERN_SLOTS: i32 = REPLAY_SLOT_COUNT as i32;

/// The first of four events kept for playing the chart under the cursor with the note pattern of the
/// replay in slot `1 + (id - first)`. Nothing runs them yet.
pub const RBMS_EVENT_REPLAY_PATTERN_FIRST: i32 = 21_003;

/// The last event of the run that starts at [`RBMS_EVENT_REPLAY_PATTERN_FIRST`].
pub const RBMS_EVENT_REPLAY_PATTERN_LAST: i32 = run_last(RBMS_EVENT_REPLAY_PATTERN_FIRST, REPLAY_PATTERN_SLOTS);

/// How many charts of the song under the browser's cursor a skin can name by slot.
pub const SIBLING_CHART_SLOTS: i32 = 5;

/// The first of five events, one for each sibling chart slot 1 to 5: the event `first + s` moves the
/// cursor to the chart in slot `s + 1` of the song under it.
pub const RBMS_EVENT_SIBLING_CHART_FIRST: i32 = 21_010;

/// The last event of the run that starts at [`RBMS_EVENT_SIBLING_CHART_FIRST`].
pub const RBMS_EVENT_SIBLING_CHART_LAST: i32 = run_last(RBMS_EVENT_SIBLING_CHART_FIRST, SIBLING_CHART_SLOTS);

/// On while SCRATCH AUTO is on, on the song browser.
pub const RBMS_OPTION_SCRATCH_AUTO_ON: i32 = 21_100;

/// On while KEEP GREEN NUMBER is on, on the song browser.
pub const RBMS_OPTION_KEEP_GREEN_NUMBER_ON: i32 = 21_101;

/// On while the lane cover, the lift and the HIDDEN+ cover are switched on, on the song browser:
/// what the image indices 330, 331 and 332 read as 1.
pub const RBMS_OPTION_LANE_COVER_ON: i32 = 21_102;
pub const RBMS_OPTION_LIFT_ON: i32 = 21_103;
pub const RBMS_OPTION_HIDDEN_ON: i32 = 21_104;

/// On while the song browser's ranking mode is open.
pub const RBMS_OPTION_RANKING_MODE_ON: i32 = 21_105;

/// How many grades an assist grade option of each run below names: none, then ASSIST EASY to ASSIST
/// EXHARD ([`AssistGrade::id`]).
pub const ASSIST_GRADE_OPTIONS: i32 = AssistGrade::COUNT as i32;

/// The first of five options, one for each assist grade id 0 to 4, of the best lamp of the chart
/// under the browser's cursor: the option `first + k` is on when that chart holds a best and its
/// grade id is `k`. Off, all five, on a bar with no best. Grade 0 is the grade of every lamp but a
/// light-assist clear on a gauge that grades it, so it is read beside the lamp (image index 370).
pub const RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST: i32 = 21_110;

/// The first of five options, one for each sibling chart slot 1 to 5: the option `first + s` is on
/// while the song under the browser's cursor has a chart in slot `s + 1`.
pub const RBMS_OPTION_SIBLING_CHART_FIRST: i32 = 21_115;

/// The last option of the run that starts at [`RBMS_OPTION_SIBLING_CHART_FIRST`].
pub const RBMS_OPTION_SIBLING_CHART_LAST: i32 = run_last(RBMS_OPTION_SIBLING_CHART_FIRST, SIBLING_CHART_SLOTS);

/// The same five options of the run a result screen shows.
pub const RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST: i32 = 21_120;

/// The same five options of the best the player held on the chart before that run (the lamp image
/// index 371 reads), all off where the chart was never played.
pub const RBMS_OPTION_RESULT_PREVIOUS_ASSIST_GRADE_FIRST: i32 = 21_130;

/// The first of nine options, one for each key index of the browser's key table (0 to 8: the seven
/// keys and the two directions of the turntable): the option `first + i` is on, on the song browser,
/// while no key of the mode whose keys are bound is on index `i`, so no press of it can come. A
/// five-key or ten-key mode has no lane on indices 5 and 6, which are the keys six and seven of an
/// option panel, and a skin that draws those keys reads these to draw them out of reach.
pub const RBMS_OPTION_UNREACHABLE_KEY_FIRST: i32 = 21_140;

/// The last option of the run of nine that starts at [`RBMS_OPTION_UNREACHABLE_KEY_FIRST`].
pub const RBMS_OPTION_UNREACHABLE_KEY_LAST: i32 = RBMS_OPTION_UNREACHABLE_KEY_FIRST + KEY_INDEX_COUNT as i32 - 1;

/// How many options are kept for the language the player's text is shown in, the languages there are
/// and those that may come.
pub const LANGUAGE_OPTIONS: i32 = 10;

/// The first of the language options, on every screen: the option `first + id` is on while the
/// language the interface resolved to is the one of that id (English 0, Korean 1, Japanese 2), and
/// exactly one of the run is on.
pub const RBMS_OPTION_LANGUAGE_FIRST: i32 = 21_150;

/// The last option of the run that starts at [`RBMS_OPTION_LANGUAGE_FIRST`].
pub const RBMS_OPTION_LANGUAGE_LAST: i32 = run_last(RBMS_OPTION_LANGUAGE_FIRST, LANGUAGE_OPTIONS);

/// How many lanes a per-lane value of the chart under the cursor is kept for: the lanes of the widest
/// mode it is given for, fourteen keys and their two turntables, in the mode's own lane order. A lane
/// the chart's mode does not have, and every lane of a nine-key or a twenty-four-key chart, reads as
/// absent.
pub const LANE_NOTE_SLOTS: i32 = Mode::BEAT_14K.key as i32;

/// The first of the numbers of notes on each lane of the chart under the cursor: the number
/// `first + l` is the count on lane `l`.
pub const RBMS_NUMBER_LANE_NOTES_FIRST: i32 = 21_000;

/// The last number of the run that starts at [`RBMS_NUMBER_LANE_NOTES_FIRST`].
pub const RBMS_NUMBER_LANE_NOTES_LAST: i32 = run_last(RBMS_NUMBER_LANE_NOTES_FIRST, LANE_NOTE_SLOTS);

/// How many axes the NOTES RADAR has: NOTES, PEAK, SCRATCH, SOF-LAN, CHARGE and CHORD, in that order,
/// which is clockwise from the top of the hexagon.
pub const NOTES_RADAR_AXES: i32 = 6;

/// The first of the six NOTES RADAR values of the chart under the cursor, as whole numbers rounded
/// down from the shown value, 0 to 200: the number `first + a` is axis `a`.
pub const RBMS_NUMBER_NOTES_RADAR_FIRST: i32 = 21_020;

/// The last number of the run that starts at [`RBMS_NUMBER_NOTES_RADAR_FIRST`].
pub const RBMS_NUMBER_NOTES_RADAR_LAST: i32 = run_last(RBMS_NUMBER_NOTES_RADAR_FIRST, NOTES_RADAR_AXES);

/// The first of the levels of the sibling chart slots 1 to 5: the number `first + s` is the level of
/// the chart in slot `s + 1`.
pub const RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST: i32 = 21_030;

/// The last number of the run that starts at [`RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST`].
pub const RBMS_NUMBER_SIBLING_CHART_LEVEL_LAST: i32 = run_last(RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST, SIBLING_CHART_SLOTS);

/// How many lamps a folder's charts are counted by: the lamp ids 0 to 10.
pub const FOLDER_LAMP_KINDS: i32 = LAMP_KINDS as i32;

/// The first of the numbers of charts in the open folder whose lamp is each lamp id: the number
/// `first + k` counts the charts of lamp id `k`.
pub const RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST: i32 = 21_040;

/// The last number of the run that starts at [`RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST`].
pub const RBMS_NUMBER_FOLDER_LAMP_COUNT_LAST: i32 = run_last(RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST, FOLDER_LAMP_KINDS);

/// How many charts the open folder holds: the lamp counts added up.
pub const RBMS_NUMBER_FOLDER_CHART_COUNT: i32 = 21_051;

/// How many rows the player's own records of the chart under the cursor are shown in: the best, then
/// the five latest runs, newest first.
pub const MY_RECORD_ROWS: i32 = 6;

/// The first of the EX scores of the record rows: the number `first + r` is row `r`.
pub const RBMS_NUMBER_MY_RECORD_EX_FIRST: i32 = 21_060;

/// The first of the lamp ids, 0 to 10, of the record rows.
pub const RBMS_NUMBER_MY_RECORD_LAMP_FIRST: i32 = 21_066;

/// The first of the miss counts of the record rows.
pub const RBMS_NUMBER_MY_RECORD_MISS_COUNT_FIRST: i32 = 21_072;

/// The first of the DJ levels of the record rows, as the index of the level, 0 for F to 7 for AAA.
pub const RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST: i32 = 21_078;

/// The last number of the four runs of record rows.
pub const RBMS_NUMBER_MY_RECORD_LAST: i32 = run_last(RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST, MY_RECORD_ROWS);

/// The first of the six NOTES RADAR values of the chart under the cursor as shown, 0 to 200, in the
/// order of [`RBMS_NUMBER_NOTES_RADAR_FIRST`].
pub const RBMS_FLOAT_NOTES_RADAR_FIRST: i32 = 21_020;

/// The last float of the run that starts at [`RBMS_FLOAT_NOTES_RADAR_FIRST`].
pub const RBMS_FLOAT_NOTES_RADAR_LAST: i32 = run_last(RBMS_FLOAT_NOTES_RADAR_FIRST, NOTES_RADAR_AXES);

/// The first of the shares of each lane of the chart under the cursor: the notes on the lane over the
/// notes on its fullest lane, in the lane order of [`RBMS_NUMBER_LANE_NOTES_FIRST`].
pub const RBMS_RATE_LANE_NOTES_FIRST: i32 = 21_000;

/// The last rate of the run that starts at [`RBMS_RATE_LANE_NOTES_FIRST`].
pub const RBMS_RATE_LANE_NOTES_LAST: i32 = run_last(RBMS_RATE_LANE_NOTES_FIRST, LANE_NOTE_SLOTS);

/// The first of the shares of each lamp id in the open folder: the charts of the lamp over
/// [`RBMS_NUMBER_FOLDER_CHART_COUNT`].
pub const RBMS_RATE_FOLDER_LAMP_FIRST: i32 = 21_040;

/// The last rate of the run that starts at [`RBMS_RATE_FOLDER_LAMP_FIRST`].
pub const RBMS_RATE_FOLDER_LAMP_LAST: i32 = run_last(RBMS_RATE_FOLDER_LAMP_FIRST, FOLDER_LAMP_KINDS);

/// The first of the dates of the record rows, kept in UTC and shown in local time in the date form
/// the browser already shows dates in.
pub const RBMS_TEXT_MY_RECORD_DATE_FIRST: i32 = 21_000;

/// The last text of the run that starts at [`RBMS_TEXT_MY_RECORD_DATE_FIRST`].
pub const RBMS_TEXT_MY_RECORD_DATE_LAST: i32 = run_last(RBMS_TEXT_MY_RECORD_DATE_FIRST, MY_RECORD_ROWS);

/// The last option of a run of five that starts at `first`.
pub const fn assist_grade_options_last(first: i32) -> i32 {
    first + ASSIST_GRADE_OPTIONS - 1
}

/// Which key index the option `id` asks about, or `None` for an id outside the run of
/// [`RBMS_OPTION_UNREACHABLE_KEY_FIRST`].
pub fn unreachable_key_asked(id: i32) -> Option<usize> {
    let place = id.checked_sub(RBMS_OPTION_UNREACHABLE_KEY_FIRST)?;
    usize::try_from(place).ok().filter(|index| *index < KEY_INDEX_COUNT)
}

/// Which assist grade id the option `id` asks about in the run of five that starts at `first`, or
/// `None` for an id outside the run.
pub fn assist_grade_asked(id: i32, first: i32) -> Option<u8> {
    let place = id.checked_sub(first)?;
    if (0..ASSIST_GRADE_OPTIONS).contains(&place) { u8::try_from(place).ok() } else { None }
}

#[cfg(test)]
mod tests {
    use rbms_skin::property::extension::implements;
    use rbms_skin::property::{NameSpace, reference_implements};

    use super::*;

    /// Every event this player keeps for itself, as runs of ids.
    const EVENT_RUNS: [(i32, i32); 5] = [
        (RBMS_EVENT_SCRATCH_AUTO, RBMS_EVENT_SCRATCH_AUTO),
        (RBMS_EVENT_KEEP_GREEN_NUMBER, RBMS_EVENT_KEEP_GREEN_NUMBER),
        (RBMS_EVENT_RANKING_MODE, RBMS_EVENT_RANKING_MODE),
        (RBMS_EVENT_REPLAY_PATTERN_FIRST, RBMS_EVENT_REPLAY_PATTERN_LAST),
        (RBMS_EVENT_SIBLING_CHART_FIRST, RBMS_EVENT_SIBLING_CHART_LAST),
    ];

    /// Every option this player keeps for itself, as runs of ids.
    const OPTION_RUNS: [(i32, i32); 12] = [
        (RBMS_OPTION_SCRATCH_AUTO_ON, RBMS_OPTION_SCRATCH_AUTO_ON),
        (RBMS_OPTION_KEEP_GREEN_NUMBER_ON, RBMS_OPTION_KEEP_GREEN_NUMBER_ON),
        (RBMS_OPTION_LANE_COVER_ON, RBMS_OPTION_LANE_COVER_ON),
        (RBMS_OPTION_LIFT_ON, RBMS_OPTION_LIFT_ON),
        (RBMS_OPTION_HIDDEN_ON, RBMS_OPTION_HIDDEN_ON),
        (RBMS_OPTION_RANKING_MODE_ON, RBMS_OPTION_RANKING_MODE_ON),
        (RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST, assist_grade_options_last(RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST)),
        (RBMS_OPTION_SIBLING_CHART_FIRST, RBMS_OPTION_SIBLING_CHART_LAST),
        (RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST, assist_grade_options_last(RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST)),
        (RBMS_OPTION_RESULT_PREVIOUS_ASSIST_GRADE_FIRST, assist_grade_options_last(RBMS_OPTION_RESULT_PREVIOUS_ASSIST_GRADE_FIRST)),
        (RBMS_OPTION_UNREACHABLE_KEY_FIRST, RBMS_OPTION_UNREACHABLE_KEY_LAST),
        (RBMS_OPTION_LANGUAGE_FIRST, RBMS_OPTION_LANGUAGE_LAST),
    ];

    /// Every number this player keeps for itself, as runs of ids.
    const NUMBER_RUNS: [(i32, i32); 6] = [
        (RBMS_NUMBER_LANE_NOTES_FIRST, RBMS_NUMBER_LANE_NOTES_LAST),
        (RBMS_NUMBER_NOTES_RADAR_FIRST, RBMS_NUMBER_NOTES_RADAR_LAST),
        (RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST, RBMS_NUMBER_SIBLING_CHART_LEVEL_LAST),
        (RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST, RBMS_NUMBER_FOLDER_LAMP_COUNT_LAST),
        (RBMS_NUMBER_FOLDER_CHART_COUNT, RBMS_NUMBER_FOLDER_CHART_COUNT),
        (RBMS_NUMBER_MY_RECORD_EX_FIRST, RBMS_NUMBER_MY_RECORD_LAST),
    ];

    /// Every float this player keeps for itself, as runs of ids.
    const FLOAT_RUNS: [(i32, i32); 1] = [(RBMS_FLOAT_NOTES_RADAR_FIRST, RBMS_FLOAT_NOTES_RADAR_LAST)];

    /// Every rate this player keeps for itself, as runs of ids.
    const RATE_RUNS: [(i32, i32); 2] = [(RBMS_RATE_LANE_NOTES_FIRST, RBMS_RATE_LANE_NOTES_LAST), (RBMS_RATE_FOLDER_LAMP_FIRST, RBMS_RATE_FOLDER_LAMP_LAST)];

    /// Every text this player keeps for itself, as runs of ids.
    const TEXT_RUNS: [(i32, i32); 1] = [(RBMS_TEXT_MY_RECORD_DATE_FIRST, RBMS_TEXT_MY_RECORD_DATE_LAST)];

    /// The highest option and event the reference declares, and the bands it leaves to a skin.
    const REFERENCE_OPTION_MAX: i32 = 3035;
    const REFERENCE_EVENT_MAX: i32 = 388;
    const SKIN_TIMER_LAST: i32 = 19_999;

    /// Every id of `runs`, each run upwards and inside `first..=last`, and no id twice.
    fn ids_inside(runs: &[(i32, i32)], (first, last): (i32, i32), what: &str) -> Vec<i32> {
        let mut ids = Vec::new();
        for (start, end) in runs {
            assert!(start <= end, "{what} run {start} to {end} runs backwards");
            assert!(first <= *start && *end <= last, "{what} run {start} to {end} is outside {first} to {last}");
            ids.extend(*start..=*end);
        }
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "two {what} ids are the same");
        ids
    }

    #[test]
    fn every_id_is_in_its_band_and_no_two_are_the_same() {
        const { assert!(EXTENSION_EVENT_FIRST > SKIN_TIMER_LAST && EXTENSION_EVENT_FIRST > REFERENCE_EVENT_MAX) };
        const { assert!(EXTENSION_OPTION_FIRST > SKIN_TIMER_LAST && EXTENSION_OPTION_FIRST > REFERENCE_OPTION_MAX) };
        const { assert!(EXTENSION_EVENT_LAST < EXTENSION_OPTION_FIRST, "the two bands are apart") };
        ids_inside(&EVENT_RUNS, (EXTENSION_EVENT_FIRST, EXTENSION_EVENT_LAST), "event");
        ids_inside(&OPTION_RUNS, (EXTENSION_OPTION_FIRST, EXTENSION_OPTION_LAST), "option");
        let value_band = (EXTENSION_VALUE_FIRST, EXTENSION_VALUE_LAST);
        ids_inside(&NUMBER_RUNS, value_band, "number");
        ids_inside(&TEXT_RUNS, value_band, "text");
        const { assert!(EXTENSION_IMAGE_FIRST <= RBMS_IMAGE_NOTES_RADAR && RBMS_IMAGE_NOTES_RADAR <= EXTENSION_IMAGE_LAST) };
    }

    /// A float read falls back to the rates, so the two spaces are one here.
    #[test]
    fn no_float_shares_a_number_with_a_rate() {
        let both: Vec<(i32, i32)> = FLOAT_RUNS.iter().chain(&RATE_RUNS).copied().collect();
        ids_inside(&both, (EXTENSION_VALUE_FIRST, EXTENSION_VALUE_LAST), "float or rate");
    }

    #[test]
    fn the_reference_has_no_id_of_these_and_every_gate_lets_them_through() {
        let spaces: [(NameSpace, &[(i32, i32)]); 5] = [
            (NameSpace::Boolean, &OPTION_RUNS),
            (NameSpace::Integer, &NUMBER_RUNS),
            (NameSpace::Float, &FLOAT_RUNS),
            (NameSpace::Rate, &RATE_RUNS),
            (NameSpace::Text, &TEXT_RUNS),
        ];
        for (space, runs) in spaces {
            for id in runs.iter().flat_map(|(first, last)| *first..=*last) {
                assert!(!reference_implements(space, id), "{space:?} {id} is the reference's");
                assert!(implements(space, id), "{space:?} {id} is stopped at a gate");
                if space == NameSpace::Boolean {
                    assert!(implements(space, -id), "the negation of option {id} is stopped at a gate");
                }
            }
        }
    }

    #[test]
    fn every_run_is_as_long_as_what_it_counts() {
        assert_eq!(RBMS_EVENT_REPLAY_PATTERN_LAST - RBMS_EVENT_REPLAY_PATTERN_FIRST + 1, REPLAY_PATTERN_SLOTS);
        assert_eq!(RBMS_NUMBER_LANE_NOTES_LAST - RBMS_NUMBER_LANE_NOTES_FIRST + 1, LANE_NOTE_SLOTS);
        assert_eq!(RBMS_NUMBER_FOLDER_LAMP_COUNT_LAST + 1, RBMS_NUMBER_FOLDER_CHART_COUNT, "the total follows the lamps");
        let record_runs = [
            RBMS_NUMBER_MY_RECORD_EX_FIRST,
            RBMS_NUMBER_MY_RECORD_LAMP_FIRST,
            RBMS_NUMBER_MY_RECORD_MISS_COUNT_FIRST,
            RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST,
            RBMS_NUMBER_MY_RECORD_LAST + 1,
        ];
        for pair in record_runs.windows(2) {
            assert_eq!(pair[1] - pair[0], MY_RECORD_ROWS, "the record runs follow one another, a row each");
        }
    }

    #[test]
    fn an_unreachable_key_option_names_the_key_index_of_its_place_in_its_run() {
        for index in 0..KEY_INDEX_COUNT {
            let id = RBMS_OPTION_UNREACHABLE_KEY_FIRST + i32::try_from(index).expect("a key index fits");
            assert_eq!(unreachable_key_asked(id), Some(index));
        }
        assert_eq!(unreachable_key_asked(RBMS_OPTION_UNREACHABLE_KEY_FIRST - 1), None);
        assert_eq!(unreachable_key_asked(RBMS_OPTION_UNREACHABLE_KEY_LAST + 1), None);
        assert_eq!(unreachable_key_asked(i32::MIN), None, "an id far below the run does not wrap round");
    }

    #[test]
    fn a_grade_option_names_the_grade_of_its_place_in_its_run() {
        let grade_runs = [RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST, RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST, RBMS_OPTION_RESULT_PREVIOUS_ASSIST_GRADE_FIRST];
        for first in grade_runs {
            for grade in AssistGrade::ALL {
                assert_eq!(assist_grade_asked(first + i32::from(grade.id()), first), Some(grade.id()));
            }
            assert_eq!(assist_grade_asked(first - 1, first), None);
            assert_eq!(assist_grade_asked(assist_grade_options_last(first) + 1, first), None);
        }
        assert_eq!(assist_grade_asked(i32::MIN, RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST), None, "an id far below the run does not wrap round");
    }
}

//! The ids this player answers beyond the reference's own: every one of them, and what it means.
//!
//! The host answers the ids the reference declares (`SkinProperty`), and a skin written for the
//! reference never names anything else. A few things this player has are not in the reference at
//! all -- SCRATCH AUTO, KEEP GREEN NUMBER as a switch, the assist grade inside a light-assist lamp,
//! which keys of the browser's table a controller can reach -- and a skin that wants to show or
//! switch one needs an id for it. They are kept in two bands of
//! this player's own, well above everything the reference declares (its highest option is 3035 and
//! its highest event 388) and clear of the bands it leaves to a skin (options 900 to 999, events
//! 1000 to 1999, timers 10000 to 19999):
//!
//! - events [`EXTENSION_EVENT_FIRST`] to [`EXTENSION_EVENT_LAST`], which a skin runs as it runs any
//!   event, by `main_state.event_exec(id)` or a button's `act`. The event space has no gate on what
//!   the reference declares, so an id here reaches the cluster [`super::EVENT_ROUTES`] sends it to.
//! - options [`EXTENSION_OPTION_FIRST`] to [`EXTENSION_OPTION_LAST`], which a skin reads with
//!   `main_state.option(id)` from a function -- a `draw`, a `value`, a `text` -- and only so: the
//!   loader keeps an `op` list to the ids the reference declares, and drops one of these from it.
//!   An option here is on or off, and a negative id reads the negation, as every option does.
//!
//! What a switch the reference already has an id for reads and runs by that id, not by one of
//! these: the lane cover, the lift and the HIDDEN+ cover are switched by the events 330 to 332 and
//! read by the image indices 330 to 332 as the reference has them. The three are also answered as
//! options here, so a skin can read every switch of a panel the one way.
//!
//! Each band is a range of numbers and nothing else holds them: a number that is in a band and not
//! named below reads as no option and runs as no event.

use rbms_judge::AssistGrade;

use crate::keyconfig::KEY_INDEX_COUNT;

/// The first and last of the events this player keeps for itself.
pub const EXTENSION_EVENT_FIRST: i32 = 21_000;
pub const EXTENSION_EVENT_LAST: i32 = 21_099;

/// Switches SCRATCH AUTO on or off (the PLAY menu's row of that name), whichever way its first
/// argument points.
pub const RBMS_EVENT_SCRATCH_AUTO: i32 = 21_000;

/// Switches KEEP GREEN NUMBER on or off (the PLAY menu's row of that name): whether the green number,
/// rather than the scroll speed, is held from chart to chart.
pub const RBMS_EVENT_KEEP_GREEN_NUMBER: i32 = 21_001;

/// The first and last of the options this player keeps for itself.
pub const EXTENSION_OPTION_FIRST: i32 = 21_100;
pub const EXTENSION_OPTION_LAST: i32 = 21_199;

/// On while SCRATCH AUTO is on, on the song browser.
pub const RBMS_OPTION_SCRATCH_AUTO_ON: i32 = 21_100;

/// On while KEEP GREEN NUMBER is on, on the song browser.
pub const RBMS_OPTION_KEEP_GREEN_NUMBER_ON: i32 = 21_101;

/// On while the lane cover, the lift and the HIDDEN+ cover are switched on, on the song browser:
/// what the image indices 330, 331 and 332 read as 1.
pub const RBMS_OPTION_LANE_COVER_ON: i32 = 21_102;
pub const RBMS_OPTION_LIFT_ON: i32 = 21_103;
pub const RBMS_OPTION_HIDDEN_ON: i32 = 21_104;

/// How many grades an assist grade option of each run below names: none, then ASSIST EASY to ASSIST
/// EXHARD ([`AssistGrade::id`]).
pub const ASSIST_GRADE_OPTIONS: i32 = AssistGrade::COUNT as i32;

/// The first of five options, one for each assist grade id 0 to 4, of the best lamp of the chart
/// under the browser's cursor: the option `first + k` is on when that chart holds a best and its
/// grade id is `k`. Off, all five, on a bar with no best. Grade 0 is the grade of every lamp but a
/// light-assist clear on a gauge that grades it, so it is read beside the lamp (image index 370).
pub const RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST: i32 = 21_110;

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
    use super::*;

    /// Every id this player keeps for itself, with the band it has to be in.
    const NAMED_EVENTS: [i32; 2] = [RBMS_EVENT_SCRATCH_AUTO, RBMS_EVENT_KEEP_GREEN_NUMBER];
    const NAMED_SWITCH_OPTIONS: [i32; 5] =
        [RBMS_OPTION_SCRATCH_AUTO_ON, RBMS_OPTION_KEEP_GREEN_NUMBER_ON, RBMS_OPTION_LANE_COVER_ON, RBMS_OPTION_LIFT_ON, RBMS_OPTION_HIDDEN_ON];
    const GRADE_RUNS: [i32; 3] =
        [RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST, RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST, RBMS_OPTION_RESULT_PREVIOUS_ASSIST_GRADE_FIRST];

    /// The highest option and event the reference declares, and the bands it leaves to a skin.
    const REFERENCE_OPTION_MAX: i32 = 3035;
    const REFERENCE_EVENT_MAX: i32 = 388;
    const SKIN_TIMER_LAST: i32 = 19_999;

    #[test]
    fn every_id_is_in_its_band_and_no_two_are_the_same() {
        const { assert!(EXTENSION_EVENT_FIRST > SKIN_TIMER_LAST && EXTENSION_EVENT_FIRST > REFERENCE_EVENT_MAX) };
        const { assert!(EXTENSION_OPTION_FIRST > SKIN_TIMER_LAST && EXTENSION_OPTION_FIRST > REFERENCE_OPTION_MAX) };
        const { assert!(EXTENSION_EVENT_LAST < EXTENSION_OPTION_FIRST, "the two bands are apart") };
        for id in NAMED_EVENTS {
            assert!((EXTENSION_EVENT_FIRST..=EXTENSION_EVENT_LAST).contains(&id), "event {id}");
        }
        let mut options: Vec<i32> = NAMED_SWITCH_OPTIONS.to_vec();
        for first in GRADE_RUNS {
            options.extend(first..=assist_grade_options_last(first));
        }
        options.extend(RBMS_OPTION_UNREACHABLE_KEY_FIRST..=RBMS_OPTION_UNREACHABLE_KEY_LAST);
        for id in &options {
            assert!((EXTENSION_OPTION_FIRST..=EXTENSION_OPTION_LAST).contains(id), "option {id}");
        }
        let count = options.len();
        options.sort_unstable();
        options.dedup();
        assert_eq!(options.len(), count, "two options share an id");
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
        for first in GRADE_RUNS {
            for grade in AssistGrade::ALL {
                assert_eq!(assist_grade_asked(first + i32::from(grade.id()), first), Some(grade.id()));
            }
            assert_eq!(assist_grade_asked(first - 1, first), None);
            assert_eq!(assist_grade_asked(assist_grade_options_last(first) + 1, first), None);
        }
        assert_eq!(assist_grade_asked(i32::MIN, RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST), None, "an id far below the run does not wrap round");
    }
}

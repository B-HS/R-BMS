use serde::{Deserialize, Serialize};

use crate::windows::{GOOD_JUDGE_INDEX, GREAT_JUDGE_INDEX, JudgeWindows};

/// The reference implementation's "unjudged" note state (`Note.getState() == 0`).
pub const UNJUDGED_STATE: u8 = 0;

/// Which of the four timing tables a candidate is judged against — the reference implementation's
/// `JudgeProperty.NoteType`. The note and scratch tables carry five window pairs (the fifth being
/// the 空POOR band); the long-note end tables carry only four.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NoteType {
    Note,
    LongNoteEnd,
    Scratch,
    LongScratchEnd,
}

impl NoteType {
    /// Whether this table has the fifth (空POOR) window pair. Mirrors the reference implementation's
    /// per-table array lengths: 10 longs for note/scratch, 8 for the long-note ends.
    pub fn has_empty_poor_window(self) -> bool {
        matches!(self, NoteType::Note | NoteType::Scratch)
    }
}

/// One candidate note as the selection predicate sees it — the reference implementation's `Note`
/// reduced to the three properties `JudgeAlgorithm.compare` reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NoteRef {
    /// Note time in microseconds (`Note.getMicroTime()`).
    pub time_us: i64,
    /// Judge state; [`UNJUDGED_STATE`] means the note is still unjudged (`Note.getState()`).
    pub state: u8,
    /// Whether the note is a long note, so a caller can pick the matching [`NoteType`].
    pub is_long: bool,
}

/// Candidate-selection policy for a key press — the reference implementation's `JudgeAlgorithm`
/// enum, one pairwise predicate per variant.
///
/// The default is [`JudgeAlgorithm::Combo`], the reference implementation's own
/// (`JudgeAlgorithm.java:42` lists `{Combo, Duration, Lowest}` with `Combo` first). A replay
/// recorded before the algorithm was written down names [`JudgeAlgorithm::Duration`], the only one
/// this engine used to have, and is played back on that. All four variants are selectable, unlike
/// the reference's three.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum JudgeAlgorithm {
    /// Combo first: skip a note that can no longer be taken as GOOD or better.
    #[default]
    Combo,
    /// Smallest timing difference first.
    Duration,
    /// Lowest note first: never replace the earliest candidate.
    Lowest,
    /// Score first: skip a note that can no longer be taken as GREAT or better.
    Score,
}

impl JudgeAlgorithm {
    /// Every variant, in the reference implementation's declaration order.
    pub const ALL: [JudgeAlgorithm; 4] = [JudgeAlgorithm::Combo, JudgeAlgorithm::Duration, JudgeAlgorithm::Lowest, JudgeAlgorithm::Score];

    /// The reference implementation's enum constant name, as persisted in settings.
    pub fn name(self) -> &'static str {
        match self {
            JudgeAlgorithm::Combo => "Combo",
            JudgeAlgorithm::Duration => "Duration",
            JudgeAlgorithm::Lowest => "Lowest",
            JudgeAlgorithm::Score => "Score",
        }
    }

    /// Parse a name produced by [`name`](Self::name); unknown names yield `None`.
    pub fn from_name(name: &str) -> Option<JudgeAlgorithm> {
        JudgeAlgorithm::ALL.into_iter().find(|a| a.name() == name)
    }

    /// Whether the candidate `cand` should replace the current best `best`, given a key press at
    /// `ptime_us` judged against `window` (already the table selected by `note_type`).
    ///
    /// Mirrors `JudgeAlgorithm.compare(t1, t2, ptime, window, type)`: `best` is `t1`, `cand` is
    /// `t2`, and `true` means `t2` is chosen.
    pub fn prefer(self, best: &NoteRef, cand: &NoteRef, ptime_us: i64, window: &JudgeWindows, note_type: NoteType) -> bool {
        match self {
            JudgeAlgorithm::Combo => {
                cand.state == UNJUDGED_STATE
                    && best.time_us < ptime_us + window.get_time(note_type, GOOD_JUDGE_INDEX, false)
                    && cand.time_us <= ptime_us + window.get_time(note_type, GOOD_JUDGE_INDEX, true)
            }
            JudgeAlgorithm::Duration => (best.time_us - ptime_us).abs() > (cand.time_us - ptime_us).abs() && cand.state == UNJUDGED_STATE,
            JudgeAlgorithm::Lowest => false,
            JudgeAlgorithm::Score => {
                cand.state == UNJUDGED_STATE
                    && best.time_us < ptime_us + window.get_time(note_type, GREAT_JUDGE_INDEX, false)
                    && cand.time_us <= ptime_us + window.get_time(note_type, GREAT_JUDGE_INDEX, true)
            }
        }
    }
}

#[cfg(test)]
mod algorithm_tests {
    use super::*;

    const W: JudgeWindows = JudgeWindows::SEVENKEY_NOTE;

    fn unjudged(time_us: i64) -> NoteRef {
        NoteRef { time_us, state: UNJUDGED_STATE, is_long: false }
    }

    fn judged(time_us: i64) -> NoteRef {
        NoteRef { time_us, state: 1, is_long: false }
    }

    #[test]
    fn default_is_combo_like_the_reference() {
        assert_eq!(JudgeAlgorithm::default(), JudgeAlgorithm::Combo);
        assert_eq!(JudgeAlgorithm::ALL[0], JudgeAlgorithm::default(), "JudgeAlgorithm.java:42 lists the default first");
    }

    #[test]
    fn names_round_trip_for_every_variant() {
        for a in JudgeAlgorithm::ALL {
            assert_eq!(JudgeAlgorithm::from_name(a.name()), Some(a), "{a:?}");
        }
        assert_eq!(JudgeAlgorithm::from_name("Nope"), None);
    }

    #[test]
    fn names_match_the_reference_enum_constants() {
        assert_eq!(JudgeAlgorithm::Combo.name(), "Combo");
        assert_eq!(JudgeAlgorithm::Duration.name(), "Duration");
        assert_eq!(JudgeAlgorithm::Lowest.name(), "Lowest");
        assert_eq!(JudgeAlgorithm::Score.name(), "Score");
    }

    #[test]
    fn duration_prefers_the_smaller_absolute_delta() {
        let a = JudgeAlgorithm::Duration;
        assert!(a.prefer(&unjudged(90_000), &unjudged(102_000), 101_000, &W, NoteType::Note), "11ms away loses to 1ms away");
        assert!(!a.prefer(&unjudged(101_000), &unjudged(105_000), 101_000, &W, NoteType::Note), "an exact hit is never replaced");
        assert!(!a.prefer(&unjudged(100_000), &unjudged(102_000), 101_000, &W, NoteType::Note), "an equal distance keeps the first candidate");
    }

    #[test]
    fn duration_never_replaces_with_an_already_judged_note() {
        let a = JudgeAlgorithm::Duration;
        assert!(!a.prefer(&unjudged(150_000), &judged(100_000), 100_000, &W, NoteType::Note));
    }

    #[test]
    fn lowest_always_keeps_the_first_candidate() {
        for best in [unjudged(0), unjudged(500_000), judged(10)] {
            for cand in [unjudged(0), unjudged(1), judged(2)] {
                assert!(!JudgeAlgorithm::Lowest.prefer(&best, &cand, 0, &W, NoteType::Note));
            }
        }
    }

    #[test]
    fn combo_skips_a_best_that_can_no_longer_be_taken_as_good() {
        let a = JudgeAlgorithm::Combo;
        let press = 1_000_000;
        let stale = press + W.gd.0 - 1;
        let reachable = press + W.gd.1;
        assert!(
            a.prefer(&unjudged(stale), &unjudged(reachable), press, &W, NoteType::Note),
            "best is past the GOOD late bound, candidate is inside the early bound"
        );
        assert!(!a.prefer(&unjudged(press + W.gd.0), &unjudged(reachable), press, &W, NoteType::Note), "best still reachable as GOOD, so it is kept");
        assert!(!a.prefer(&unjudged(stale), &unjudged(reachable + 1), press, &W, NoteType::Note), "candidate is beyond the GOOD early bound");
        assert!(!a.prefer(&unjudged(stale), &judged(reachable), press, &W, NoteType::Note), "an already-judged candidate is never chosen");
    }

    #[test]
    fn score_uses_the_great_bounds_instead_of_the_good_bounds() {
        let a = JudgeAlgorithm::Score;
        let press = 1_000_000;
        let stale_for_great = press + W.gr.0 - 1;
        assert!(a.prefer(&unjudged(stale_for_great), &unjudged(press + W.gr.1), press, &W, NoteType::Note));
        assert!(
            !JudgeAlgorithm::Combo.prefer(&unjudged(stale_for_great), &unjudged(press + W.gr.1), press, &W, NoteType::Note),
            "the same pair stays with Combo because GOOD is wider than GREAT"
        );
        assert!(!a.prefer(&unjudged(stale_for_great), &unjudged(press + W.gr.1 + 1), press, &W, NoteType::Note));
    }

    #[test]
    fn note_type_reports_which_tables_carry_the_empty_poor_pair() {
        assert!(NoteType::Note.has_empty_poor_window());
        assert!(NoteType::Scratch.has_empty_poor_window());
        assert!(!NoteType::LongNoteEnd.has_empty_poor_window());
        assert!(!NoteType::LongScratchEnd.has_empty_poor_window());
    }

    #[test]
    fn algorithm_serialises_by_name() {
        for a in JudgeAlgorithm::ALL {
            let text = ron::ser::to_string(&a).unwrap();
            assert_eq!(text, a.name(), "{a:?}");
            assert_eq!(ron::from_str::<JudgeAlgorithm>(&text).unwrap(), a);
        }
    }
    #[test]
    fn algorithm_table_pins() {
        let fixtures = [
            (unjudged(0), unjudged(10_000), "both inside PG, candidate further out"),
            (unjudged(-200_000), unjudged(100_000), "best past the GOOD late bound"),
            (unjudged(-100_000), unjudged(50_000), "best still inside GOOD, candidate nearer"),
            (unjudged(-200_000), unjudged(200_000), "candidate beyond every early bound"),
            (unjudged(-200_000), judged(100_000), "candidate already judged"),
            (unjudged(150_000), unjudged(-10_000), "best far early, candidate nearly on time"),
        ];
        let expected = [
            [false, false, false, false],
            [true, true, false, false],
            [false, true, false, true],
            [false, false, false, false],
            [false, false, false, false],
            [false, true, false, false],
        ];
        for (fixture, (best, cand, what)) in fixtures.iter().enumerate() {
            for (slot, algorithm) in JudgeAlgorithm::ALL.into_iter().enumerate() {
                assert_eq!(algorithm.prefer(best, cand, 0, &W, NoteType::Note), expected[fixture][slot], "{algorithm:?}: {what}");
            }
        }
    }

    #[test]
    fn combo_takes_the_upper_note_once_the_lower_one_is_past_good() {
        let lower = unjudged(-160_000);
        let upper = unjudged(-100_000);
        assert!(JudgeAlgorithm::Combo.prefer(&lower, &upper, 0, &W, NoteType::Note), "the lower note is past the GOOD late bound, so Combo moves up");
        assert!(!JudgeAlgorithm::Lowest.prefer(&lower, &upper, 0, &W, NoteType::Note), "Lowest never moves up");
    }

    #[test]
    fn combo_keeps_a_still_good_lower_note_that_duration_abandons() {
        let lower = unjudged(-100_000);
        let upper = unjudged(50_000);
        assert!(!JudgeAlgorithm::Combo.prefer(&lower, &upper, 0, &W, NoteType::Note), "the lower note is still reachable as GOOD, so Combo keeps it");
        assert!(JudgeAlgorithm::Duration.prefer(&lower, &upper, 0, &W, NoteType::Note), "Duration only looks at the timing distance");
        assert!(JudgeAlgorithm::Score.prefer(&lower, &upper, 0, &W, NoteType::Note), "the lower note is past the GREAT late bound, so Score moves up");
    }

    #[test]
    fn every_algorithm_uses_the_table_the_note_type_selects() {
        let ln = JudgeWindows::SEVENKEY_LN_END;
        let lower = unjudged(-210_000);
        let upper = unjudged(150_000);
        assert!(
            JudgeAlgorithm::Combo.prefer(&lower, &upper, 0, &ln, NoteType::LongNoteEnd),
            "the long-note end GOOD bounds are wider than the note ones, and both notes sit inside them"
        );
        assert!(!JudgeAlgorithm::Combo.prefer(&unjudged(-190_000), &upper, 0, &ln, NoteType::LongNoteEnd), "still inside the long-note end GOOD band");
    }
}

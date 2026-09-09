use serde::{Deserialize, Serialize};

use crate::windows::JudgeWindows;

/// Judge index of GREAT inside a [`JudgeWindows`] row, used by [`JudgeAlgorithm::Score`].
const GREAT_JUDGE_INDEX: usize = 1;

/// Judge index of GOOD inside a [`JudgeWindows`] row, used by [`JudgeAlgorithm::Combo`].
const GOOD_JUDGE_INDEX: usize = 2;

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
/// The default is [`JudgeAlgorithm::Duration`] because that is what this engine has always done
/// (nearest `|Δt|` wins). The reference implementation defaults to [`JudgeAlgorithm::Combo`]; that
/// divergence predates this type and changing it would change judgments, so it is not changed here.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum JudgeAlgorithm {
    /// Combo first: skip a note that can no longer be taken as GOOD or better.
    Combo,
    /// Smallest timing difference first.
    #[default]
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
    fn default_is_duration_because_that_is_the_engines_historic_behaviour() {
        assert_eq!(JudgeAlgorithm::default(), JudgeAlgorithm::Duration);
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
}

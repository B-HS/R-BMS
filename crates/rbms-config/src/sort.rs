//! Song-list ordering: the modes the browser can sort by, the subset one key cycles through, and
//! the vocabulary the setting is stored under.
//!
//! The reference implementation declares twelve sorters and exposes them in two tiers — a default
//! cycle of eight and the full list of twelve (`BarSorter.java:260,262`). This engine keeps that
//! split and adds one of its own, `Default`, which leaves a folder in the order it was scanned.

use serde::{Deserialize, Deserializer, Serializer};

/// How the song list is ordered.
///
/// Ties fall back to the title in every mode, so the order is total and a list never reshuffles
/// between two runs of the same sort.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SortMode {
    /// The order the folder was scanned in, which no other sorter can reproduce.
    #[default]
    Default,
    Title,
    Artist,
    Bpm,
    Length,
    Level,
    Clear,
    Score,
    MissCount,
    /// Total time the chart's notes span, as opposed to the audio length.
    Duration,
    /// When the chart was last played.
    LastUpdate,
    /// A rival's lamp on this chart. Ordered the way every record-reading sort here is: worst
    /// first, with the charts neither player has a record on last.
    RivalClear,
    /// A rival's EX rate on this chart, ordered the same way as [`SortMode::RivalClear`].
    RivalScore,
}

/// How many orderings exist in total.
pub const SORT_MODE_COUNT: usize = 13;

/// How many orderings the browser's sort key steps through.
pub const SORT_CYCLE_COUNT: usize = 9;

/// How many orderings the settings row offers.
pub const SORT_SELECTABLE_COUNT: usize = 11;

impl SortMode {
    /// Every ordering, in declaration order.
    pub const ALL: [SortMode; SORT_MODE_COUNT] = [
        SortMode::Default,
        SortMode::Title,
        SortMode::Artist,
        SortMode::Bpm,
        SortMode::Length,
        SortMode::Level,
        SortMode::Clear,
        SortMode::Score,
        SortMode::MissCount,
        SortMode::Duration,
        SortMode::LastUpdate,
        SortMode::RivalClear,
        SortMode::RivalScore,
    ];

    /// The orderings the browser's sort key steps through: the reference implementation's default
    /// tier (`BarSorter.java:260`) plus this engine's own scan order.
    pub const DEFAULT_CYCLE: [SortMode; SORT_CYCLE_COUNT] = [
        SortMode::Default,
        SortMode::Title,
        SortMode::Artist,
        SortMode::Bpm,
        SortMode::Length,
        SortMode::Level,
        SortMode::Clear,
        SortMode::Score,
        SortMode::MissCount,
    ];

    /// The orderings the SORT row offers: the cycled ones plus the two the key does not reach.
    /// The two rival comparisons are left out until a rival's records are available to compare
    /// against, so the row cannot select an ordering that would silently do nothing.
    pub const SELECTABLE: [SortMode; SORT_SELECTABLE_COUNT] = [
        SortMode::Default,
        SortMode::Title,
        SortMode::Artist,
        SortMode::Bpm,
        SortMode::Length,
        SortMode::Level,
        SortMode::Clear,
        SortMode::Score,
        SortMode::MissCount,
        SortMode::Duration,
        SortMode::LastUpdate,
    ];

    /// Name the browser and the SORT row show.
    pub fn label(self) -> &'static str {
        match self {
            SortMode::Default => "DEFAULT",
            SortMode::Title => "TITLE",
            SortMode::Artist => "ARTIST",
            SortMode::Bpm => "BPM",
            SortMode::Length => "LENGTH",
            SortMode::Level => "LEVEL",
            SortMode::Clear => "CLEAR",
            SortMode::Score => "SCORE",
            SortMode::MissCount => "MISS COUNT",
            SortMode::Duration => "DURATION",
            SortMode::LastUpdate => "LAST UPDATE",
            SortMode::RivalClear => "RIVAL CLEAR",
            SortMode::RivalScore => "RIVAL SCORE",
        }
    }

    /// Separator-free token the ordering is stored under, so a reworded label leaves the file alone.
    pub fn token(self) -> &'static str {
        match self {
            SortMode::Default => "DEFAULT",
            SortMode::Title => "TITLE",
            SortMode::Artist => "ARTIST",
            SortMode::Bpm => "BPM",
            SortMode::Length => "LENGTH",
            SortMode::Level => "LEVEL",
            SortMode::Clear => "CLEAR",
            SortMode::Score => "SCORE",
            SortMode::MissCount => "MISSCOUNT",
            SortMode::Duration => "DURATION",
            SortMode::LastUpdate => "LASTUPDATE",
            SortMode::RivalClear => "RIVALCLEAR",
            SortMode::RivalScore => "RIVALSCORE",
        }
    }

    /// The ordering a stored token names, falling back to the scan order for anything unknown.
    pub fn from_token(token: &str) -> SortMode {
        SortMode::ALL.into_iter().find(|mode| mode.token().eq_ignore_ascii_case(token.trim())).unwrap_or_default()
    }

    /// The next ordering the sort key steps to, wrapping. An ordering outside the cycle (one the
    /// settings row selected) steps to the head of the cycle.
    pub fn next(self) -> SortMode {
        self.step(1)
    }

    /// The previous ordering the sort key steps to, wrapping.
    pub fn prev(self) -> SortMode {
        self.step(-1)
    }

    fn step(self, delta: i32) -> SortMode {
        let Some(at) = SortMode::DEFAULT_CYCLE.iter().position(|mode| *mode == self) else {
            return SortMode::DEFAULT_CYCLE[0];
        };
        let len = SORT_CYCLE_COUNT as i32;
        SortMode::DEFAULT_CYCLE[(at as i32 + delta).rem_euclid(len) as usize]
    }
}

/// Stores the ordering as its token rather than as a Rust variant name, so the file stays readable
/// and a renamed variant does not orphan what is on disk.
pub fn serialize<S: Serializer>(value: &SortMode, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(value.token())
}

/// Reads the ordering back from its token.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SortMode, D::Error> {
    let token = String::deserialize(deserializer)?;
    Ok(SortMode::from_token(&token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ordering_has_its_own_label_and_its_own_token() {
        let mut labels: Vec<&str> = SortMode::ALL.iter().map(|mode| mode.label()).collect();
        let mut tokens: Vec<&str> = SortMode::ALL.iter().map(|mode| mode.token()).collect();
        assert_eq!(labels.len(), SORT_MODE_COUNT);
        labels.sort_unstable();
        labels.dedup();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(labels.len(), SORT_MODE_COUNT, "two orderings share a label");
        assert_eq!(tokens.len(), SORT_MODE_COUNT, "two orderings share a token");
        for mode in SortMode::ALL {
            assert!(!mode.label().is_empty(), "{mode:?} has no label");
            assert!(!mode.token().contains(' '), "{mode:?} stores a token with a separator in it");
        }
    }

    #[test]
    fn a_token_round_trips_through_the_file_vocabulary() {
        for mode in SortMode::ALL {
            assert_eq!(SortMode::from_token(mode.token()), mode);
            assert_eq!(SortMode::from_token(&mode.token().to_ascii_lowercase()), mode, "the token is read case-insensitively");
        }
        assert_eq!(SortMode::from_token("not a sort"), SortMode::Default, "an unknown token falls back to the scan order");
        assert_eq!(SortMode::from_token(""), SortMode::Default);
    }

    #[test]
    fn the_cycled_and_selectable_tiers_are_subsets_of_the_whole_list() {
        for mode in SortMode::DEFAULT_CYCLE {
            assert!(SortMode::SELECTABLE.contains(&mode), "{mode:?} is cycled but cannot be selected");
        }
        for mode in SortMode::SELECTABLE {
            assert!(SortMode::ALL.contains(&mode), "{mode:?} is selectable but is not an ordering");
        }
        assert!(!SortMode::SELECTABLE.contains(&SortMode::RivalClear), "a rival ordering has nothing to compare against yet");
        assert!(!SortMode::SELECTABLE.contains(&SortMode::RivalScore));
    }

    #[test]
    fn the_sort_key_walks_the_whole_cycle_and_comes_back() {
        let mut seen = Vec::new();
        let mut mode = SortMode::DEFAULT_CYCLE[0];
        for _ in 0..SORT_CYCLE_COUNT {
            seen.push(mode);
            mode = mode.next();
        }
        assert_eq!(seen, SortMode::DEFAULT_CYCLE.to_vec());
        assert_eq!(mode, SortMode::DEFAULT_CYCLE[0], "the cycle wraps");
    }

    #[test]
    fn stepping_back_undoes_stepping_forward() {
        for mode in SortMode::DEFAULT_CYCLE {
            assert_eq!(mode.next().prev(), mode, "{mode:?}");
            assert_eq!(mode.prev().next(), mode, "{mode:?}");
        }
        assert_eq!(SortMode::DEFAULT_CYCLE[0].prev(), SortMode::DEFAULT_CYCLE[SORT_CYCLE_COUNT - 1], "the cycle wraps backwards too");
    }

    #[test]
    fn an_ordering_outside_the_cycle_steps_back_into_it() {
        for mode in [SortMode::Duration, SortMode::LastUpdate, SortMode::RivalClear, SortMode::RivalScore] {
            assert_eq!(mode.next(), SortMode::DEFAULT_CYCLE[0], "{mode:?}");
            assert_eq!(mode.prev(), SortMode::DEFAULT_CYCLE[0], "{mode:?}");
        }
    }
}

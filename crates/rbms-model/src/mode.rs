/// A play mode profile. New key modes are added as data (a new `Mode` constant),
/// never by branching engine logic. `channel_assign` maps a raw 18-wide channel
/// index (P1 group 0..8, P2 group 9..17) to a logical lane, `-1` meaning ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub name: &'static str,
    pub key: usize,
    pub player: u8,
    pub scratch: &'static [usize],
    pub channel_assign: &'static [i8; 18],
}

const BEAT7: [i8; 18] = [0, 1, 2, 3, 4, 7, -1, 5, 6, 8, 9, 10, 11, 12, 15, -1, 13, 14];
const BEAT5: [i8; 18] = [0, 1, 2, 3, 4, 5, -1, -1, -1, 6, 7, 8, 9, 10, 11, -1, -1, -1];
const POPN: [i8; 18] = [0, 1, 2, 3, 4, -1, -1, -1, -1, -1, 5, 6, 7, 8, -1, -1, -1, -1];

/// No raw BMS channel maps to a 24-key lane. The reference implementation reaches its
/// `KEYBOARD_24K` mode only from a BMSON `mode_hint` string (`BMSONDecoder`); its BMS decoder never
/// produces it, so the 18-wide channel table is empty rather than invented.
const KEYBOARD24: [i8; 18] = [-1; 18];

impl Mode {
    pub const BEAT_7K: Mode = Mode { name: "BEAT_7K", key: 8, player: 1, scratch: &[7], channel_assign: &BEAT7 };
    pub const BEAT_5K: Mode = Mode { name: "BEAT_5K", key: 6, player: 1, scratch: &[5], channel_assign: &BEAT5 };
    pub const BEAT_10K: Mode = Mode { name: "BEAT_10K", key: 12, player: 2, scratch: &[5, 11], channel_assign: &BEAT5 };
    pub const BEAT_14K: Mode = Mode { name: "BEAT_14K", key: 16, player: 2, scratch: &[7, 15], channel_assign: &BEAT7 };
    pub const POPN_9K: Mode = Mode { name: "POPN_9K", key: 9, player: 1, scratch: &[], channel_assign: &POPN };

    /// The reference implementation's `Mode.KEYBOARD_24K`: 26 lanes, one player, lanes 24 and 25
    /// scratch. The numbers are its enum constructor arguments `(id 25, player 1, key 26,
    /// scratchKey {24, 25})`. Lanes 0..24 are two octaves of a piano keyboard, which is what its
    /// MIDI binding says: key `i` is note `48 + i`, so `i % 12` in `{1, 3, 6, 8, 10}` is a black
    /// key and the rest are white.
    pub const KEYBOARD_24K: Mode = Mode { name: "KEYBOARD_24K", key: 26, player: 1, scratch: &[24, 25], channel_assign: &KEYBOARD24 };

    /// Every mode this build plays: what the key-config screen enumerates, what the browser filters
    /// on, and what a play document's type has to draw for the document to be accepted.
    ///
    /// [`Mode::KEYBOARD_24K`] is here even though no BMS channel scan can pick it, because a bmson
    /// `mode_hint` can: a chart that reaches it needs bindings, a judge row, a gauge set and a
    /// document exactly as the others do.
    pub const ALL: &'static [Mode] = &[Mode::BEAT_7K, Mode::BEAT_5K, Mode::BEAT_10K, Mode::BEAT_14K, Mode::POPN_9K, Mode::KEYBOARD_24K];

    /// Whether a raw BMS channel scan can ever land on this mode, which is false only for the modes
    /// a bmson `mode_hint` is the sole route to. Their channel table is empty, so every lane-cover
    /// invariant stated over the table has to skip them rather than read an empty map as a hole.
    pub fn has_bms_channels(&self) -> bool {
        self.channel_assign.iter().any(|lane| *lane >= 0)
    }

    pub fn is_scratch(&self, lane: usize) -> bool {
        self.scratch.contains(&lane)
    }

    /// Logical lane for a raw 18-wide channel index, or `None` if ignored in this mode.
    /// The table defines the full P1+P2 mapping; `key` bounds the lanes this mode owns,
    /// so a 7K mode (key 8) ignores the P2 lanes 8..15 that a 14K mode (key 16) keeps.
    pub fn lane_of_raw(&self, raw: usize) -> Option<usize> {
        match self.channel_assign.get(raw).copied() {
            Some(l) if l >= 0 && (l as usize) < self.key => Some(l as usize),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_k_ignores_p2_lanes() {
        assert_eq!(Mode::BEAT_7K.lane_of_raw(0), Some(0));
        assert_eq!(Mode::BEAT_7K.lane_of_raw(5), Some(7));
        assert_eq!(Mode::BEAT_7K.lane_of_raw(9), None);
        assert_eq!(Mode::BEAT_7K.lane_of_raw(17), None);
    }

    #[test]
    fn fourteen_k_keeps_p2_lanes() {
        assert_eq!(Mode::BEAT_14K.lane_of_raw(9), Some(8));
        assert_eq!(Mode::BEAT_14K.lane_of_raw(17), Some(14));
    }

    #[test]
    fn beat_7k_lane_of_raw_full_map() {
        let expected: [Option<usize>; 18] =
            [Some(0), Some(1), Some(2), Some(3), Some(4), Some(7), None, Some(5), Some(6), None, None, None, None, None, None, None, None, None];
        for (raw, want) in expected.iter().enumerate() {
            assert_eq!(Mode::BEAT_7K.lane_of_raw(raw), *want, "raw={raw}");
        }
    }

    #[test]
    fn beat_5k_lane_of_raw_full_map() {
        let expected: [Option<usize>; 18] =
            [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5), None, None, None, None, None, None, None, None, None, None, None, None];
        for (raw, want) in expected.iter().enumerate() {
            assert_eq!(Mode::BEAT_5K.lane_of_raw(raw), *want, "raw={raw}");
        }
    }

    #[test]
    fn beat_10k_lane_of_raw_full_map() {
        let expected: [Option<usize>; 18] =
            [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5), None, None, None, Some(6), Some(7), Some(8), Some(9), Some(10), Some(11), None, None, None];
        for (raw, want) in expected.iter().enumerate() {
            assert_eq!(Mode::BEAT_10K.lane_of_raw(raw), *want, "raw={raw}");
        }
    }

    #[test]
    fn beat_14k_lane_of_raw_full_map() {
        let expected: [Option<usize>; 18] = [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(7),
            None,
            Some(5),
            Some(6),
            Some(8),
            Some(9),
            Some(10),
            Some(11),
            Some(12),
            Some(15),
            None,
            Some(13),
            Some(14),
        ];
        for (raw, want) in expected.iter().enumerate() {
            assert_eq!(Mode::BEAT_14K.lane_of_raw(raw), *want, "raw={raw}");
        }
    }

    #[test]
    fn popn_9k_lane_of_raw_full_map() {
        let expected: [Option<usize>; 18] =
            [Some(0), Some(1), Some(2), Some(3), Some(4), None, None, None, None, None, Some(5), Some(6), Some(7), Some(8), None, None, None, None];
        for (raw, want) in expected.iter().enumerate() {
            assert_eq!(Mode::POPN_9K.lane_of_raw(raw), *want, "raw={raw}");
        }
    }

    #[test]
    fn lane_of_raw_out_of_range_is_none_for_all_modes() {
        for mode in Mode::ALL {
            assert_eq!(mode.lane_of_raw(18), None, "{}", mode.name);
            assert_eq!(mode.lane_of_raw(19), None, "{}", mode.name);
            assert_eq!(mode.lane_of_raw(100), None, "{}", mode.name);
            assert_eq!(mode.lane_of_raw(usize::MAX), None, "{}", mode.name);
        }
    }

    #[test]
    fn lane_of_raw_never_exceeds_key() {
        for mode in Mode::ALL {
            for raw in 0..18 {
                if let Some(lane) = mode.lane_of_raw(raw) {
                    assert!(lane < mode.key, "{} raw={raw} lane={lane} key={}", mode.name, mode.key);
                }
            }
        }
    }

    #[test]
    fn lane_of_raw_matches_table_within_key() {
        for mode in Mode::ALL {
            for raw in 0..18 {
                let v = mode.channel_assign[raw];
                let want = if v >= 0 && (v as usize) < mode.key { Some(v as usize) } else { None };
                assert_eq!(mode.lane_of_raw(raw), want, "{} raw={raw}", mode.name);
            }
        }
    }

    #[test]
    fn lane_of_raw_logical_lanes_cover_full_key_range_once() {
        for mode in Mode::ALL.iter().filter(|mode| mode.has_bms_channels()) {
            let mut produced: Vec<usize> = (0..18).filter_map(|raw| mode.lane_of_raw(raw)).collect();
            produced.sort_unstable();
            let expected: Vec<usize> = (0..mode.key).collect();
            assert_eq!(produced, expected, "{}", mode.name);
        }
    }

    #[test]
    fn a_mode_no_bms_channel_reaches_maps_no_raw_channel_at_all() {
        for mode in Mode::ALL.iter().filter(|mode| !mode.has_bms_channels()) {
            assert_eq!(mode.name, Mode::KEYBOARD_24K.name, "only the keyboard mode is bmson-only today");
            assert!((0..18).all(|raw| mode.lane_of_raw(raw).is_none()), "{} maps a raw channel", mode.name);
        }
        assert!(!Mode::KEYBOARD_24K.has_bms_channels());
        for mode in [Mode::BEAT_7K, Mode::BEAT_5K, Mode::BEAT_10K, Mode::BEAT_14K, Mode::POPN_9K] {
            assert!(mode.has_bms_channels(), "{} is reachable from a channel scan", mode.name);
        }
    }

    #[test]
    fn is_scratch_matches_scratch_slice_for_all_modes() {
        for mode in Mode::ALL {
            for lane in 0..(mode.key + 4) {
                assert_eq!(mode.is_scratch(lane), mode.scratch.contains(&lane), "{} lane={lane}", mode.name);
            }
        }
    }

    #[test]
    fn is_scratch_known_values() {
        assert!(Mode::BEAT_7K.is_scratch(7));
        assert!(!Mode::BEAT_7K.is_scratch(0));
        assert!(!Mode::BEAT_7K.is_scratch(6));
        assert!(Mode::BEAT_5K.is_scratch(5));
        assert!(!Mode::BEAT_5K.is_scratch(0));
        assert!(Mode::BEAT_10K.is_scratch(5));
        assert!(Mode::BEAT_10K.is_scratch(11));
        assert!(!Mode::BEAT_10K.is_scratch(0));
        assert!(Mode::BEAT_14K.is_scratch(7));
        assert!(Mode::BEAT_14K.is_scratch(15));
        assert!(!Mode::BEAT_14K.is_scratch(0));
    }

    #[test]
    fn popn_has_no_scratch_lanes() {
        assert!(Mode::POPN_9K.scratch.is_empty());
        for lane in 0..20 {
            assert!(!Mode::POPN_9K.is_scratch(lane), "lane={lane}");
        }
    }

    #[test]
    fn is_scratch_out_of_range_lane_is_false() {
        for mode in Mode::ALL {
            assert!(!mode.is_scratch(usize::MAX), "{}", mode.name);
            assert!(!mode.is_scratch(1000), "{}", mode.name);
        }
    }

    #[test]
    fn scratch_lanes_are_within_key_bounds() {
        for mode in Mode::ALL {
            for &s in mode.scratch {
                assert!(s < mode.key, "{} scratch lane {s} >= key {}", mode.name, mode.key);
            }
        }
    }

    #[test]
    fn scratch_lanes_are_reachable_via_lane_of_raw() {
        for mode in Mode::ALL.iter().filter(|mode| mode.has_bms_channels()) {
            for &s in mode.scratch {
                let reachable = (0..18).any(|raw| mode.lane_of_raw(raw) == Some(s));
                assert!(reachable, "{} scratch lane {s} unreachable", mode.name);
            }
        }
    }

    #[test]
    fn channel_assign_p1_group_values_below_key() {
        for mode in Mode::ALL {
            for raw in 0..9 {
                let v = mode.channel_assign[raw];
                if v != -1 {
                    assert!(v >= 0, "{} raw={raw} negative-but-not-(-1) value {v}", mode.name);
                    assert!((v as usize) < mode.key, "{} P1 raw={raw} value {v} >= key {}", mode.name, mode.key);
                }
            }
        }
    }

    #[test]
    fn channel_assign_no_duplicate_logical_lanes() {
        for mode in Mode::ALL {
            let mut seen: Vec<i8> = Vec::new();
            for &v in mode.channel_assign.iter() {
                if v >= 0 && (v as usize) < mode.key {
                    assert!(!seen.contains(&v), "{} duplicate logical lane {v}", mode.name);
                    seen.push(v);
                }
            }
        }
    }

    #[test]
    fn channel_assign_negative_values_are_exactly_minus_one() {
        for mode in Mode::ALL {
            for &v in mode.channel_assign.iter() {
                assert!(v == -1 || v >= 0, "{} unexpected negative sentinel {v}", mode.name);
            }
        }
    }

    #[test]
    fn all_modes_have_distinct_names() {
        let mut names: Vec<&str> = Mode::ALL.iter().map(|m| m.name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    #[test]
    fn all_constant_has_expected_membership() {
        assert_eq!(Mode::ALL.len(), 6);
        assert!(Mode::ALL.contains(&Mode::BEAT_7K));
        assert!(Mode::ALL.contains(&Mode::BEAT_5K));
        assert!(Mode::ALL.contains(&Mode::BEAT_10K));
        assert!(Mode::ALL.contains(&Mode::BEAT_14K));
        assert!(Mode::ALL.contains(&Mode::POPN_9K));
        assert!(Mode::ALL.contains(&Mode::KEYBOARD_24K));
    }

    #[test]
    fn player_count_is_one_or_two() {
        for mode in Mode::ALL {
            assert!(mode.player == 1 || mode.player == 2, "{} player={}", mode.name, mode.player);
        }
    }

    #[test]
    fn two_player_modes_have_two_scratch_lanes() {
        for mode in Mode::ALL {
            if mode.player == 2 && !mode.scratch.is_empty() {
                assert_eq!(mode.scratch.len(), 2, "{}", mode.name);
            }
        }
    }

    #[test]
    fn mode_equality_is_value_based() {
        assert_eq!(Mode::BEAT_7K, Mode::BEAT_7K);
        assert_ne!(Mode::BEAT_7K, Mode::BEAT_5K);
        assert_ne!(Mode::BEAT_10K, Mode::BEAT_14K);
    }

    #[test]
    fn key_counts_match_mode_names() {
        assert_eq!(Mode::BEAT_7K.key, 8);
        assert_eq!(Mode::BEAT_5K.key, 6);
        assert_eq!(Mode::BEAT_10K.key, 12);
        assert_eq!(Mode::BEAT_14K.key, 16);
        assert_eq!(Mode::POPN_9K.key, 9);
    }

    #[test]
    fn keyboard_24k_matches_the_reference_enum_row() {
        assert_eq!(Mode::KEYBOARD_24K.name, "KEYBOARD_24K");
        assert_eq!(Mode::KEYBOARD_24K.key, 26);
        assert_eq!(Mode::KEYBOARD_24K.player, 1);
        assert_eq!(Mode::KEYBOARD_24K.scratch, &[24, 25]);
    }

    #[test]
    fn keyboard_24k_scratch_lanes_are_the_last_two() {
        for lane in 0..Mode::KEYBOARD_24K.key {
            assert_eq!(Mode::KEYBOARD_24K.is_scratch(lane), lane >= 24, "lane={lane}");
        }
    }

    #[test]
    fn keyboard_24k_has_no_bms_channel_mapping() {
        for raw in 0..18 {
            assert_eq!(Mode::KEYBOARD_24K.lane_of_raw(raw), None, "raw={raw}");
        }
    }

    #[test]
    fn all_carries_the_keyboard_mode_exactly_once() {
        assert_eq!(Mode::ALL.iter().filter(|mode| mode.name == Mode::KEYBOARD_24K.name).count(), 1);
        assert_eq!(Mode::ALL.iter().find(|mode| mode.name == Mode::KEYBOARD_24K.name), Some(&Mode::KEYBOARD_24K));
    }

    #[test]
    fn the_keyboard_lanes_are_two_octaves_of_a_piano() {
        const BLACK_KEYS_IN_AN_OCTAVE: [usize; 5] = [1, 3, 6, 8, 10];
        const SEMITONES_IN_AN_OCTAVE: usize = 12;
        let black = |lane: usize| BLACK_KEYS_IN_AN_OCTAVE.contains(&(lane % SEMITONES_IN_AN_OCTAVE));
        let keys = Mode::KEYBOARD_24K.key - Mode::KEYBOARD_24K.scratch.len();
        assert_eq!(keys, 24, "the keyboard is 24 keys wide once its two scratch lanes are set aside");
        assert_eq!(keys, SEMITONES_IN_AN_OCTAVE * 2, "which is two octaves");
        assert_eq!((0..keys).filter(|lane| black(*lane)).count(), 10, "two octaves carry ten black keys");
        assert_eq!((0..keys).filter(|lane| !black(*lane)).count(), 14, "and fourteen white ones");
    }

    #[test]
    fn lane_of_raw_is_deterministic() {
        for mode in Mode::ALL {
            for raw in 0..20 {
                assert_eq!(mode.lane_of_raw(raw), mode.lane_of_raw(raw), "{} raw={raw}", mode.name);
            }
        }
    }
}

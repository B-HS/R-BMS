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

impl Mode {
    pub const BEAT_7K: Mode = Mode { name: "BEAT_7K", key: 8, player: 1, scratch: &[7], channel_assign: &BEAT7 };
    pub const BEAT_5K: Mode = Mode { name: "BEAT_5K", key: 6, player: 1, scratch: &[5], channel_assign: &BEAT5 };
    pub const BEAT_10K: Mode = Mode { name: "BEAT_10K", key: 12, player: 2, scratch: &[5, 11], channel_assign: &BEAT5 };
    pub const BEAT_14K: Mode = Mode { name: "BEAT_14K", key: 16, player: 2, scratch: &[7, 15], channel_assign: &BEAT7 };
    pub const POPN_9K: Mode = Mode { name: "POPN_9K", key: 9, player: 1, scratch: &[], channel_assign: &POPN };

    pub const ALL: &'static [Mode] = &[Mode::BEAT_7K, Mode::BEAT_5K, Mode::BEAT_10K, Mode::BEAT_14K, Mode::POPN_9K];

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
}

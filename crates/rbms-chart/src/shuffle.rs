use rbms_model::{Mode, Model, NoteKind};

/// Note lane-shuffle option, applied to the chart at load. Deterministic given a seed so a
/// replay reproduces the exact pattern. The scratch lane(s) are never moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteOption {
    Off,
    Mirror,
    Random,
    SRandom,
    RRandom,
    Rotate,
    HRandom,
    AllScratch,
}

impl NoteOption {
    pub const ALL: [NoteOption; 8] = [
        NoteOption::Off,
        NoteOption::Mirror,
        NoteOption::Random,
        NoteOption::SRandom,
        NoteOption::RRandom,
        NoteOption::Rotate,
        NoteOption::HRandom,
        NoteOption::AllScratch,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NoteOption::Off => "OFF",
            NoteOption::Mirror => "MIRROR",
            NoteOption::Random => "RANDOM",
            NoteOption::SRandom => "S-RANDOM",
            NoteOption::RRandom => "R-RANDOM",
            NoteOption::Rotate => "ROTATE",
            NoteOption::HRandom => "H-RANDOM",
            NoteOption::AllScratch => "ALL-SCRATCH",
        }
    }

    pub fn from_str(s: &str) -> NoteOption {
        match s.to_ascii_uppercase().as_str() {
            "MIRROR" => NoteOption::Mirror,
            "RANDOM" => NoteOption::Random,
            "S-RANDOM" | "SRANDOM" => NoteOption::SRandom,
            "R-RANDOM" | "RRANDOM" => NoteOption::RRandom,
            "ROTATE" => NoteOption::Rotate,
            "H-RANDOM" | "HRANDOM" => NoteOption::HRandom,
            "ALL-SCRATCH" | "ALLSCRATCH" | "ALL-SCR" => NoteOption::AllScratch,
            _ => NoteOption::Off,
        }
    }
}

/// H-RANDOM anti-jack window (µs). The reference implementation uses `ceil(15000 / hranThresholdBPM)` ms; at its
/// default 120 BPM that is 125 ms. A lane that fired within this window is avoided for the next
/// note when alternatives remain, so the per-row shuffle stops producing unplayable jacks.
const HRAN_THRESHOLD_US: i64 = 125_000;

/// Scratch-lane anti-jack window (µs) for ALL-SCRATCH — the reference implementation's `SRAN_THRESHOLD` (40 ms). The
/// scratch lane tolerates faster repeats than key lanes, so it uses this tighter window: only when
/// scratch was hit more recently than 40 ms does ALL-SCRATCH spill a note to a key lane instead.
const SCRATCH_THRESHOLD_US: i64 = 40_000;

/// xorshift64 — a tiny deterministic PRNG so shuffles are reproducible from a seed (no rand dep).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

/// Non-scratch lanes belonging to one player side. For SP (player==1) side 0 covers every key
/// lane; for DP each side covers only its own half, so a shuffle never crosses the P1<->P2
/// boundary (that crossing is a separate FLIP option in the reference implementation, not implemented here).
fn side_key_lanes(mode: Mode, side: usize) -> Vec<usize> {
    let players = (mode.player as usize).max(1);
    let per_side = mode.key / players;
    let start = side * per_side;
    (start..start + per_side).filter(|l| !mode.is_scratch(*l)).collect()
}

fn fisher_yates(v: &mut [usize], rng: &mut Rng) {
    for i in (1..v.len()).rev() {
        v.swap(i, rng.below(i + 1));
    }
}

/// Whole-chart lane permutation `perm[old_lane] = new_lane` (identity on scratch lanes). Used
/// for MIRROR/RANDOM/R-RANDOM/ROTATE; S-RANDOM is per-row and handled in [`apply`].
pub fn lane_permutation(option: NoteOption, mode: Mode, seed: u64) -> Vec<usize> {
    let n = mode.key;
    let mut perm: Vec<usize> = (0..n).collect();
    for side in 0..(mode.player as usize).max(1) {
        let lanes = side_key_lanes(mode, side);
        let m = lanes.len();
        if m == 0 {
            continue;
        }
        let mut rng = Rng::new(seed.wrapping_add(side as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let target: Vec<usize> = match option {
            NoteOption::Mirror => lanes.iter().rev().copied().collect(),
            NoteOption::Rotate => (0..m).map(|i| lanes[(i + 1) % m]).collect(),
            NoteOption::RRandom => {
                let r = 1 + rng.below(m.saturating_sub(1).max(1));
                (0..m).map(|i| lanes[(i + r) % m]).collect()
            }
            NoteOption::Random => {
                let mut t = lanes.clone();
                fisher_yates(&mut t, &mut rng);
                t
            }
            // S-RANDOM/H-RANDOM/ALL-SCRATCH are per-row (handled in `apply`), not whole-chart perms.
            NoteOption::Off | NoteOption::SRandom | NoteOption::HRandom | NoteOption::AllScratch => lanes.clone(),
        };
        for (i, &old_lane) in lanes.iter().enumerate() {
            perm[old_lane] = target[i];
        }
    }
    perm
}

/// Apply a note option to the chart in place. Note count is preserved; long notes keep their
/// head and tail in the same (remapped) lane.
pub fn apply(model: &mut Model, option: NoteOption, seed: u64) {
    match option {
        NoteOption::Off => {}
        NoteOption::SRandom => apply_srandom(model, seed),
        NoteOption::HRandom => apply_time_based(model, seed, false, HRAN_THRESHOLD_US),
        NoteOption::AllScratch => apply_time_based(model, seed, true, HRAN_THRESHOLD_US),
        _ => {
            let perm = lane_permutation(option, model.mode, seed);
            apply_perm(model, &perm);
        }
    }
}

fn remap_lanes(slots: &mut Vec<Option<rbms_model::Note>>, map: impl Fn(usize) -> usize) {
    let n = slots.len();
    let old = std::mem::replace(slots, (0..n).map(|_| None).collect());
    for (lane, note) in old.into_iter().enumerate() {
        if let Some(note) = note {
            slots[map(lane)] = Some(note);
        }
    }
}

fn apply_perm(model: &mut Model, perm: &[usize]) {
    let n = perm.len();
    for tl in &mut model.timelines {
        if tl.notes.len() == n {
            remap_lanes(&mut tl.notes, |l| perm[l]);
        }
        if tl.hidden.len() == n {
            remap_lanes(&mut tl.hidden, |l| perm[l]);
        }
    }
}

/// S-RANDOM: a fresh random lane mapping each row, but a lane holding an open long note is
/// pinned to its head's target so head and tail stay aligned.
fn apply_srandom(model: &mut Model, seed: u64) {
    let mode = model.mode;
    let n = mode.key;
    let sides: Vec<Vec<usize>> = (0..(mode.player as usize).max(1)).map(|s| side_key_lanes(mode, s)).collect();
    if sides.iter().all(|s| s.is_empty()) {
        return;
    }
    let mut rng = Rng::new(seed);
    let mut pinned: Vec<Option<usize>> = vec![None; n];

    for tl in &mut model.timelines {
        if tl.notes.len() != n {
            continue;
        }
        let mut map: Vec<Option<usize>> = vec![None; n];
        for lanes in &sides {
            let mut used: Vec<bool> = vec![false; n];
            for &lane in lanes {
                if let Some(dst) = pinned[lane] {
                    map[lane] = Some(dst);
                    used[dst] = true;
                }
            }
            let unmapped: Vec<usize> = lanes.iter().copied().filter(|l| map[*l].is_none()).collect();
            let mut pool: Vec<usize> = lanes.iter().copied().filter(|l| !used[*l]).collect();
            fisher_yates(&mut pool, &mut rng);
            for (i, &lane) in unmapped.iter().enumerate() {
                map[lane] = Some(pool[i]);
            }
        }

        for (lane, slot) in tl.notes.iter().enumerate() {
            match slot.as_ref().map(|n| &n.kind) {
                Some(NoteKind::LongStart { .. }) => pinned[lane] = Some(map[lane].unwrap_or(lane)),
                Some(NoteKind::LongEnd { .. }) => pinned[lane] = None,
                _ => {}
            }
        }
        remap_lanes(&mut tl.notes, |l| map[l].unwrap_or(l));
        if tl.hidden.len() == n {
            remap_lanes(&mut tl.hidden, |l| map[l].unwrap_or(l));
        }
    }
}

/// Per-row, time-aware shuffle shared by H-RANDOM and ALL-SCRATCH. Each row's notes are reassigned
/// to fresh lanes, preferring lanes idle for at least `threshold_us` (anti-jack); an open long note
/// pins its lane until the tail. H-RANDOM (`all_scratch=false`) shuffles only the key lanes and
/// keeps scratch fixed; ALL-SCRATCH (`all_scratch=true`) adds the scratch lane to the pool and fills
/// it first, concentrating notes on scratch. Note count is preserved.
fn apply_time_based(model: &mut Model, seed: u64, all_scratch: bool, threshold_us: i64) {
    let mode = model.mode;
    let n = mode.key;
    let players = (mode.player as usize).max(1);
    let per_side = n / players.max(1);
    if per_side == 0 {
        return;
    }
    let mut rng = Rng::new(seed);
    let mut last_us: Vec<i64> = vec![i64::MIN; n];
    let mut pinned: Vec<Option<usize>> = vec![None; n];

    for tl in &mut model.timelines {
        if tl.notes.len() != n {
            continue;
        }
        let now = tl.time_us;
        let mut map: Vec<Option<usize>> = vec![None; n];

        for side in 0..players {
            let lo = side * per_side;
            let hi = lo + per_side;
            let srcs: Vec<usize> = (lo..hi).filter(|&l| tl.notes[l].is_some()).collect();
            if srcs.is_empty() {
                continue;
            }

            let mut used = vec![false; n];
            let mut free_srcs: Vec<usize> = Vec::new();
            for &s in &srcs {
                match pinned[s] {
                    Some(dst) => {
                        map[s] = Some(dst);
                        used[dst] = true;
                    }
                    None => free_srcs.push(s),
                }
            }
            // H-RANDOM keeps scratch notes on their own lane; only key lanes shuffle.
            if !all_scratch {
                free_srcs.retain(|&s| {
                    if mode.is_scratch(s) {
                        if !used[s] {
                            map[s] = Some(s);
                            used[s] = true;
                        }
                        false
                    } else {
                        true
                    }
                });
            }

            // Target pool: ALL-SCRATCH includes the scratch lane(s) (filled first); H-RANDOM is key
            // lanes only. Fresh lanes (idle >= threshold) come first, randomised within each group.
            let mut pool: Vec<usize> = (lo..hi).filter(|&l| (all_scratch || !mode.is_scratch(l)) && !used[l]).collect();
            fisher_yates(&mut pool, &mut rng);
            // Preference order (lower = picked first): a scratch lane idle past its tight window wins
            // under ALL-SCRATCH, then any lane idle past the key window, then recently-used lanes.
            let rank = |l: usize| -> u8 {
                let scr = all_scratch && mode.is_scratch(l);
                let window = if mode.is_scratch(l) { SCRATCH_THRESHOLD_US } else { threshold_us };
                let fresh = last_us[l].saturating_add(window) <= now;
                match (scr, fresh) {
                    (true, true) => 0,
                    (false, true) => 1,
                    (true, false) => 2,
                    (false, false) => 3,
                }
            };
            pool.sort_by_key(|&l| rank(l));

            let mut pi = 0;
            for &s in &free_srcs {
                while pi < pool.len() && used[pool[pi]] {
                    pi += 1;
                }
                let dst = if pi < pool.len() {
                    let d = pool[pi];
                    pi += 1;
                    d
                } else {
                    s
                };
                map[s] = Some(dst);
                used[dst] = true;
            }
        }

        for (lane, slot) in tl.notes.iter().enumerate() {
            let dst = map[lane].unwrap_or(lane);
            match slot.as_ref().map(|nn| &nn.kind) {
                Some(NoteKind::LongStart { .. }) => pinned[lane] = Some(dst),
                Some(NoteKind::LongEnd { .. }) => pinned[lane] = None,
                _ => {}
            }
            if slot.is_some() {
                last_us[dst] = now;
            }
        }
        remap_lanes(&mut tl.notes, |l| map[l].unwrap_or(l));
        if tl.hidden.len() == n {
            remap_lanes(&mut tl.hidden, |l| map[l].unwrap_or(l));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_model;
    use rbms_parser::parse;

    fn model(bms: &[u8]) -> Model {
        to_model(&parse(bms), Mode::BEAT_7K)
    }

    fn note_count(m: &Model) -> usize {
        m.timelines.iter().flat_map(|t| t.notes.iter()).filter(|n| n.is_some()).count()
    }

    #[test]
    fn mirror_reverses_non_scratch_lanes() {
        let perm = lane_permutation(NoteOption::Mirror, Mode::BEAT_7K, 0);
        assert_eq!(perm[0], 6);
        assert_eq!(perm[6], 0);
        assert_eq!(perm[3], 3);
        assert_eq!(perm[7], 7, "scratch lane unchanged");
    }

    #[test]
    fn random_is_a_permutation_leaving_scratch_fixed() {
        let perm = lane_permutation(NoteOption::Random, Mode::BEAT_7K, 12345);
        let mut sorted = perm.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..8).collect::<Vec<_>>(), "every lane is a target exactly once");
        assert_eq!(perm[7], 7, "scratch fixed");
    }

    #[test]
    fn random_is_deterministic_for_a_seed() {
        assert_eq!(lane_permutation(NoteOption::Random, Mode::BEAT_7K, 99), lane_permutation(NoteOption::Random, Mode::BEAT_7K, 99));
    }

    #[test]
    fn apply_preserves_note_count() {
        let base = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:0101\r\n#00113:0011\r\n#00116:0100\r\n");
        let before = note_count(&base);
        for opt in [
            NoteOption::Mirror,
            NoteOption::Random,
            NoteOption::SRandom,
            NoteOption::RRandom,
            NoteOption::Rotate,
            NoteOption::HRandom,
            NoteOption::AllScratch,
        ] {
            let mut m = base.clone();
            apply(&mut m, opt, 7);
            assert_eq!(note_count(&m), before, "{:?} preserves note count", opt);
        }
    }

    fn scratch_note_count(m: &Model) -> usize {
        m.timelines.iter().filter(|tl| tl.notes.get(7).map(|s| s.is_some()).unwrap_or(false)).count()
    }

    #[test]
    fn all_scratch_concentrates_notes_on_scratch() {
        // Key-only chart (channels 11/13/15 -> lanes 0/2/4, no scratch). ALL-SCRATCH prefers the
        // scratch lane each spaced-out row, so its scratch count rises well above OFF's zero (it is
        // a preference, not a guarantee: scratch jacks closer than the threshold spill to key lanes).
        let chart = b"#BPM 240\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00113:01010101\r\n#00115:01010101\r\n";
        let off = model(chart);
        assert_eq!(scratch_note_count(&off), 0, "the key-only chart has no scratch notes");
        let mut m = off.clone();
        apply(&mut m, NoteOption::AllScratch, 3);
        assert!(scratch_note_count(&m) >= 3, "ALL-SCRATCH concentrates notes toward the scratch lane (got {})", scratch_note_count(&m));
        assert_eq!(note_count(&m), note_count(&off), "note count preserved");
    }

    #[test]
    fn time_based_options_handle_dense_full_rows_without_panic() {
        // Every lane filled on every subdivision (more notes per row than spare lanes after the
        // threshold gate): the shuffle must not panic and must preserve the note count.
        let dense = b"#BPM 300\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00112:01010101\r\n#00113:01010101\r\n#00114:01010101\r\n#00115:01010101\r\n#00118:01010101\r\n#00119:01010101\r\n#00116:01010101\r\n";
        let base = model(dense);
        for opt in [NoteOption::HRandom, NoteOption::AllScratch] {
            let mut m = base.clone();
            apply(&mut m, opt, 13);
            assert_eq!(note_count(&m), note_count(&base), "{:?} preserves note count on a dense full-row chart", opt);
        }
    }

    #[test]
    fn time_based_options_preserve_count_on_dp() {
        let dp = to_model(
            &parse(b"#PLAYER 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01000100\r\n#00121:00110011\r\n#00126:00010001\r\n"),
            Mode::BEAT_14K,
        );
        for opt in [NoteOption::HRandom, NoteOption::AllScratch] {
            let mut m = dp.clone();
            apply(&mut m, opt, 21);
            assert_eq!(note_count(&m), note_count(&dp), "{:?} preserves note count on a 14K DP chart", opt);
        }
    }

    #[test]
    fn time_based_options_are_deterministic() {
        let chart = b"#BPM 200\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00114:00010100\r\n#00116:01000010\r\n";
        for opt in [NoteOption::HRandom, NoteOption::AllScratch] {
            let mut a = model(chart);
            let mut b = model(chart);
            apply(&mut a, opt, 555);
            apply(&mut b, opt, 555);
            let lanes_a: Vec<_> = a.timelines.iter().map(|tl| tl.notes.iter().map(|s| s.is_some()).collect::<Vec<_>>()).collect();
            let lanes_b: Vec<_> = b.timelines.iter().map(|tl| tl.notes.iter().map(|s| s.is_some()).collect::<Vec<_>>()).collect();
            assert_eq!(lanes_a, lanes_b, "{:?} is deterministic for a seed", opt);
        }
    }

    #[test]
    fn h_random_keeps_long_notes_in_one_lane() {
        let mut m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        apply(&mut m, NoteOption::HRandom, 9);
        let mut head = None;
        let mut tail = None;
        for tl in &m.timelines {
            for (lane, slot) in tl.notes.iter().enumerate() {
                match slot.as_ref().map(|nn| &nn.kind) {
                    Some(NoteKind::LongStart { .. }) => head = Some(lane),
                    Some(NoteKind::LongEnd { .. }) => tail = Some(lane),
                    _ => {}
                }
            }
        }
        assert_eq!(head, tail, "H-RANDOM keeps LN head and tail in the same lane");
    }

    #[test]
    fn dp_shuffle_stays_within_player_side() {
        for opt in [NoteOption::Mirror, NoteOption::Random, NoteOption::RRandom, NoteOption::Rotate] {
            let perm = lane_permutation(opt, Mode::BEAT_14K, 777);
            for lane in 0..16 {
                assert_eq!(lane < 8, perm[lane] < 8, "{:?}: lane {lane} crossed the P1/P2 boundary to {}", opt, perm[lane]);
            }
        }
    }

    #[test]
    fn srandom_keeps_long_notes_in_one_lane() {
        let mut m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        apply(&mut m, NoteOption::SRandom, 4);
        let mut head_lane = None;
        let mut tail_lane = None;
        for tl in &m.timelines {
            for (lane, slot) in tl.notes.iter().enumerate() {
                match slot.as_ref().map(|nn| &nn.kind) {
                    Some(NoteKind::LongStart { .. }) => head_lane = Some(lane),
                    Some(NoteKind::LongEnd { .. }) => tail_lane = Some(lane),
                    _ => {}
                }
            }
        }
        assert_eq!(head_lane, tail_lane, "LN head and tail end up in the same lane");
    }

    // -----------------------------------------------------------------------
    // shared helpers
    // -----------------------------------------------------------------------

    fn dp_model(bms: &[u8]) -> Model {
        to_model(&parse(bms), Mode::BEAT_14K)
    }

    /// (total LongStart count, total LongEnd count) across the whole chart.
    fn ln_kind_counts(m: &Model) -> (usize, usize) {
        let mut starts = 0;
        let mut ends = 0;
        for tl in &m.timelines {
            for slot in tl.notes.iter().flatten() {
                match slot.kind {
                    NoteKind::LongStart { .. } => starts += 1,
                    NoteKind::LongEnd { .. } => ends += 1,
                    _ => {}
                }
            }
        }
        (starts, ends)
    }

    /// For single-LN charts: returns true if the ordered head lanes equal the ordered tail lanes.
    fn ln_pairs_aligned(m: &Model) -> bool {
        let mut heads = Vec::new();
        let mut tails = Vec::new();
        for tl in &m.timelines {
            for (lane, slot) in tl.notes.iter().enumerate() {
                match slot.as_ref().map(|n| &n.kind) {
                    Some(NoteKind::LongStart { .. }) => heads.push(lane),
                    Some(NoteKind::LongEnd { .. }) => tails.push(lane),
                    _ => {}
                }
            }
        }
        heads == tails
    }

    /// Number of timelines whose scratch lane (7 in 7K) holds a note.
    fn scratch_rows(m: &Model) -> usize {
        m.timelines.iter().filter(|tl| tl.notes.get(7).map(|s| s.is_some()).unwrap_or(false)).count()
    }

    // -----------------------------------------------------------------------
    // NoteOption::from_str / label round trips
    // -----------------------------------------------------------------------

    #[test]
    fn from_str_round_trips_every_label() {
        for opt in NoteOption::ALL {
            assert_eq!(NoteOption::from_str(opt.label()), opt, "label {} should parse back", opt.label());
        }
    }

    #[test]
    fn from_str_is_case_insensitive_and_accepts_aliases() {
        assert_eq!(NoteOption::from_str("mirror"), NoteOption::Mirror);
        assert_eq!(NoteOption::from_str("S-RANDOM"), NoteOption::SRandom);
        assert_eq!(NoteOption::from_str("srandom"), NoteOption::SRandom);
        assert_eq!(NoteOption::from_str("rrandom"), NoteOption::RRandom);
        assert_eq!(NoteOption::from_str("hrandom"), NoteOption::HRandom);
        assert_eq!(NoteOption::from_str("allscratch"), NoteOption::AllScratch);
        assert_eq!(NoteOption::from_str("ALL-SCR"), NoteOption::AllScratch);
    }

    #[test]
    fn from_str_unknown_is_off() {
        assert_eq!(NoteOption::from_str("nonsense"), NoteOption::Off);
        assert_eq!(NoteOption::from_str(""), NoteOption::Off);
        assert_eq!(NoteOption::from_str("OFF"), NoteOption::Off);
    }

    #[test]
    fn all_array_lists_every_variant_once() {
        assert_eq!(NoteOption::ALL.len(), 8);
        let mut seen = NoteOption::ALL.to_vec();
        seen.dedup();
        assert_eq!(seen.len(), 8, "ALL has no duplicates");
    }

    // -----------------------------------------------------------------------
    // lane_permutation: per-mode structural guarantees
    // -----------------------------------------------------------------------

    #[test]
    fn mirror_fully_reverses_popn_lanes() {
        // POPN has no scratch lane, so MIRROR reverses every one of the 9 lanes.
        let perm = lane_permutation(NoteOption::Mirror, Mode::POPN_9K, 0);
        assert_eq!(perm, vec![8, 7, 6, 5, 4, 3, 2, 1, 0]);
    }

    #[test]
    fn mirror_reverses_5k_keeping_scratch_fixed() {
        let perm = lane_permutation(NoteOption::Mirror, Mode::BEAT_5K, 0);
        assert_eq!(perm, vec![4, 3, 2, 1, 0, 5], "lane 5 (scratch) is fixed");
    }

    #[test]
    fn mirror_is_an_involution_on_key_lanes() {
        // Applying MIRROR's permutation twice returns identity (it is its own inverse).
        let perm = lane_permutation(NoteOption::Mirror, Mode::BEAT_7K, 0);
        for l in 0..8 {
            assert_eq!(perm[perm[l]], l, "mirror twice is identity on lane {l}");
        }
    }

    #[test]
    fn rotate_is_a_derangement_on_key_lanes() {
        // ROTATE shifts every key lane by one, so no key lane maps to itself; only scratch is fixed.
        let perm = lane_permutation(NoteOption::Rotate, Mode::BEAT_7K, 0);
        for l in 0..7 {
            assert_ne!(perm[l], l, "key lane {l} must move under ROTATE");
        }
        assert_eq!(perm[7], 7, "scratch fixed");
    }

    #[test]
    fn every_perm_option_is_a_bijection_leaving_scratch_fixed() {
        for opt in [NoteOption::Mirror, NoteOption::Random, NoteOption::RRandom, NoteOption::Rotate] {
            for &mode in &[Mode::BEAT_7K, Mode::BEAT_5K, Mode::POPN_9K, Mode::BEAT_14K, Mode::BEAT_10K] {
                let perm = lane_permutation(opt, mode, 31337);
                let mut sorted = perm.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..mode.key).collect::<Vec<_>>(), "{:?}/{} is a bijection", opt, mode.name);
                for &s in mode.scratch {
                    assert_eq!(perm[s], s, "{:?}/{}: scratch lane {s} fixed", opt, mode.name);
                }
            }
        }
    }

    #[test]
    fn dp_mirror_mirrors_each_side_independently() {
        // 14K MIRROR reverses lanes 0..6 within P1 and 8..14 within P2, scratch 7 and 15 fixed.
        let perm = lane_permutation(NoteOption::Mirror, Mode::BEAT_14K, 0);
        assert_eq!(perm, vec![6, 5, 4, 3, 2, 1, 0, 7, 14, 13, 12, 11, 10, 9, 8, 15]);
    }

    #[test]
    fn dp_perm_never_crosses_player_boundary() {
        for opt in [NoteOption::Mirror, NoteOption::Random, NoteOption::RRandom, NoteOption::Rotate] {
            let perm = lane_permutation(opt, Mode::BEAT_14K, 4242);
            for lane in 0..16 {
                assert_eq!(lane < 8, perm[lane] < 8, "{:?}: lane {lane} crossed sides to {}", opt, perm[lane]);
            }
        }
    }

    #[test]
    fn off_is_identity_permutation() {
        let perm = lane_permutation(NoteOption::Off, Mode::BEAT_7K, 12345);
        assert_eq!(perm, (0..8).collect::<Vec<_>>(), "OFF leaves every lane in place");
    }

    #[test]
    fn srandom_hrandom_allscratch_yield_identity_in_lane_permutation() {
        // These are per-row options; lane_permutation returns identity for them (the real work is in apply).
        for opt in [NoteOption::SRandom, NoteOption::HRandom, NoteOption::AllScratch] {
            assert_eq!(lane_permutation(opt, Mode::BEAT_7K, 1), (0..8).collect::<Vec<_>>(), "{:?} is identity at the perm level", opt);
        }
    }

    #[test]
    fn lane_permutation_is_deterministic_for_every_random_option() {
        for opt in [NoteOption::Random, NoteOption::RRandom] {
            for seed in [0u64, 1, 7, 9999, u64::MAX] {
                assert_eq!(lane_permutation(opt, Mode::BEAT_7K, seed), lane_permutation(opt, Mode::BEAT_7K, seed), "{:?}/{seed}", opt);
            }
        }
    }

    #[test]
    fn rng_seed_zero_does_not_collapse() {
        // The PRNG remaps a 0 seed to a nonzero constant, so RANDOM at seed 0 is still a real shuffle.
        let perm = lane_permutation(NoteOption::Random, Mode::BEAT_7K, 0);
        let mut sorted = perm.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..8).collect::<Vec<_>>(), "still a bijection at seed 0");
    }

    // -----------------------------------------------------------------------
    // apply: count preservation across EVERY option and several modes
    // -----------------------------------------------------------------------

    #[test]
    fn apply_off_is_a_noop() {
        let base = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:0101\r\n#00113:0011\r\n");
        let lanes_before: Vec<_> = base.timelines.iter().map(|tl| tl.notes.iter().map(|s| s.is_some()).collect::<Vec<_>>()).collect();
        let mut m = base.clone();
        apply(&mut m, NoteOption::Off, 77);
        let lanes_after: Vec<_> = m.timelines.iter().map(|tl| tl.notes.iter().map(|s| s.is_some()).collect::<Vec<_>>()).collect();
        assert_eq!(lanes_before, lanes_after, "OFF must not move any note");
    }

    #[test]
    fn apply_preserves_count_for_every_option_7k() {
        let base = model(b"#BPM 130\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00113:00110011\r\n#00116:01000010\r\n#00118:00010001\r\n");
        let before = note_count(&base);
        assert!(before > 0);
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 123);
            assert_eq!(note_count(&m), before, "{:?} preserves note count (7K)", opt);
        }
    }

    #[test]
    fn apply_preserves_count_for_every_option_5k() {
        let base = to_model(
            &parse(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00113:00110011\r\n#00116:01000010\r\n"),
            Mode::BEAT_5K,
        );
        let before = note_count(&base);
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 88);
            assert_eq!(note_count(&m), before, "{:?} preserves note count (5K)", opt);
        }
    }

    #[test]
    fn apply_preserves_count_for_every_option_popn() {
        // POPN has no scratch; ALL-SCRATCH and H-RANDOM must still preserve the note count.
        // POPN lanes 0-4 come from channels 11-15 (raw 0-4); lanes 5-8 from channels 21-24 (raw 10-13).
        let base = to_model(
            &parse(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:0101\r\n#00112:0011\r\n#00113:0110\r\n#00114:1001\r\n#00115:0101\r\n#00121:1010\r\n#00122:0100\r\n"),
            Mode::POPN_9K,
        );
        let before = note_count(&base);
        assert!(before > 0);
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 55);
            assert_eq!(note_count(&m), before, "{:?} preserves note count (POPN)", opt);
        }
    }

    #[test]
    fn apply_preserves_count_for_every_option_14k() {
        let base = dp_model(b"#PLAYER 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01000100\r\n#00121:00110011\r\n#00126:00010001\r\n#00128:01000010\r\n");
        let before = note_count(&base);
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 2024);
            assert_eq!(note_count(&m), before, "{:?} preserves note count (14K DP)", opt);
        }
    }

    #[test]
    fn apply_preserves_count_for_every_option_10k() {
        let base = to_model(
            &parse(b"#PLAYER 2\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01000100\r\n#00121:00110011\r\n#00126:00010001\r\n"),
            Mode::BEAT_10K,
        );
        let before = note_count(&base);
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 71);
            assert_eq!(note_count(&m), before, "{:?} preserves note count (10K DP)", opt);
        }
    }

    // -----------------------------------------------------------------------
    // apply: scratch lane is never moved by key shuffles
    // -----------------------------------------------------------------------

    #[test]
    fn key_shuffles_keep_scratch_notes_on_scratch_lane() {
        // Chart with scratch notes (ch16 -> lane 7) and key notes. MIRROR/RANDOM/ROTATE/RRANDOM keep
        // the scratch column untouched: every original scratch row still has its scratch note.
        let chart = b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01010101\r\n#00113:00110011\r\n";
        let base = model(chart);
        let scratch_before = scratch_rows(&base);
        assert!(scratch_before > 0);
        for opt in [NoteOption::Mirror, NoteOption::Random, NoteOption::Rotate, NoteOption::RRandom] {
            let mut m = base.clone();
            apply(&mut m, opt, 9);
            assert_eq!(scratch_rows(&m), scratch_before, "{:?} must not move scratch notes", opt);
        }
    }

    #[test]
    fn hrandom_keeps_scratch_notes_on_scratch_lane() {
        // H-RANDOM shuffles only key lanes; the scratch column count is invariant.
        let chart = b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01010101\r\n#00113:00110011\r\n";
        let base = model(chart);
        let scratch_before = scratch_rows(&base);
        let mut m = base.clone();
        apply(&mut m, NoteOption::HRandom, 17);
        assert_eq!(scratch_rows(&m), scratch_before, "H-RANDOM leaves scratch notes in place");
    }

    // -----------------------------------------------------------------------
    // LN integrity under every shuffle
    // -----------------------------------------------------------------------

    #[test]
    fn every_option_keeps_single_ln_head_and_tail_aligned() {
        // One LN spanning a few rows mixed with normal notes; head and tail must remain co-lane.
        let chart = b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00113:00100100\r\n#00112:01000010\r\n";
        for opt in NoteOption::ALL {
            let mut m = model(chart);
            apply(&mut m, opt, 333);
            assert!(ln_pairs_aligned(&m), "{:?} must keep the LN head/tail in one lane", opt);
            assert_eq!(ln_kind_counts(&m), (1, 1), "{:?} preserves the LN head/tail pair", opt);
        }
    }

    #[test]
    fn srandom_preserves_count_with_open_ln_across_dense_rows() {
        // An LN stays open across rows that also carry shuffling normal notes: count is preserved and
        // the pin keeps the head/tail aligned.
        let chart = b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00111:01010101\r\n#00113:01010101\r\n";
        let base = model(chart);
        let mut m = base.clone();
        apply(&mut m, NoteOption::SRandom, 808);
        assert_eq!(note_count(&m), note_count(&base));
        assert!(ln_pairs_aligned(&m), "S-RANDOM keeps the open LN pinned");
    }

    // -----------------------------------------------------------------------
    // determinism for the per-row options
    // -----------------------------------------------------------------------

    #[test]
    fn all_options_are_deterministic_for_a_seed() {
        let chart = b"#BPM 180\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00114:00010100\r\n#00116:01000010\r\n#00151:01000001\r\n";
        for opt in NoteOption::ALL {
            let mut a = model(chart);
            let mut b = model(chart);
            apply(&mut a, opt, 4321);
            apply(&mut b, opt, 4321);
            let pos = |m: &Model| -> Vec<Vec<bool>> {
                m.timelines.iter().map(|tl| tl.notes.iter().map(|s| s.is_some()).collect()).collect()
            };
            assert_eq!(pos(&a), pos(&b), "{:?} must be deterministic for a fixed seed", opt);
        }
    }

    #[test]
    fn different_seeds_can_produce_different_random_layouts() {
        // Sanity: RANDOM is seed-sensitive (not a constant permutation). At least one of a few seeds
        // differs from seed 0 (5040 permutations make an all-collision essentially impossible).
        let p0 = lane_permutation(NoteOption::Random, Mode::BEAT_7K, 0);
        let differs = [1u64, 2, 3, 4, 5].iter().any(|&s| lane_permutation(NoteOption::Random, Mode::BEAT_7K, s) != p0);
        assert!(differs, "RANDOM should vary across seeds");
    }

    // -----------------------------------------------------------------------
    // robustness: dense / degenerate inputs do not panic
    // -----------------------------------------------------------------------

    #[test]
    fn time_based_options_survive_fully_dense_rows() {
        // Every key + scratch lane filled on every subdivision: more notes than fresh lanes after the
        // anti-jack gate. Must not panic and must preserve the count.
        let dense = b"#BPM 300\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00112:01010101\r\n#00113:01010101\r\n#00114:01010101\r\n#00115:01010101\r\n#00118:01010101\r\n#00119:01010101\r\n#00116:01010101\r\n";
        let base = model(dense);
        for opt in [NoteOption::HRandom, NoteOption::AllScratch, NoteOption::SRandom] {
            let mut m = base.clone();
            apply(&mut m, opt, 13);
            assert_eq!(note_count(&m), note_count(&base), "{:?} on fully dense rows", opt);
        }
    }

    #[test]
    fn apply_on_empty_chart_does_not_panic() {
        // No notes at all: every option is a no-op that leaves the (note-free) timelines intact.
        let base = model(b"#BPM 120\r\n#WAV01 a.wav\r\n");
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 1);
            assert_eq!(note_count(&m), 0, "{:?} on an empty chart stays empty", opt);
        }
    }

    #[test]
    fn all_scratch_raises_scratch_population_above_off() {
        // A key-only chart with well-spaced rows: ALL-SCRATCH concentrates notes onto the scratch lane,
        // so its scratch-row count exceeds OFF's zero. (A preference, not a guarantee, hence >= a few.)
        let chart = b"#BPM 240\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00113:01010101\r\n#00115:01010101\r\n";
        let off = model(chart);
        assert_eq!(scratch_rows(&off), 0);
        let mut m = off.clone();
        apply(&mut m, NoteOption::AllScratch, 3);
        assert!(scratch_rows(&m) >= 3, "ALL-SCRATCH should pile notes onto scratch, got {}", scratch_rows(&m));
        assert_eq!(note_count(&m), note_count(&off));
    }

    #[test]
    fn srandom_preserves_count_on_dp() {
        let base = dp_model(b"#PLAYER 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:00010100\r\n#00121:01000010\r\n#00126:00100100\r\n");
        let mut m = base.clone();
        apply(&mut m, NoteOption::SRandom, 909);
        assert_eq!(note_count(&m), note_count(&base), "S-RANDOM preserves count on 14K DP");
    }
}

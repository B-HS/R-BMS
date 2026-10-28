use std::sync::atomic::{AtomicBool, Ordering};

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

    /// Parses a label or alias, case-insensitively. Unknown input maps to `Off`, so this never fails.
    /// Prefer the `FromStr` implementation in new code; this inherent form is kept for existing callers.
    #[allow(clippy::should_implement_trait)]
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

impl std::str::FromStr for NoteOption {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(NoteOption::from_str(s))
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

/// Multiplier a lane shuffle spreads the seed of one player side with: the 64-bit golden ratio.
const SIDE_SEED_MULTIPLIER: u64 = 0x9E3779B97F4A7C15;

/// The generator a whole-chart lane shuffle draws one player side's lanes from. Side `side` of seed
/// `seed` starts where side 0 of seed `seed + side` does, which [`random_seed_for`] leans on.
fn side_rng(seed: u64, side: usize) -> Rng {
    Rng::new(seed.wrapping_add(side as u64).wrapping_mul(SIDE_SEED_MULTIPLIER))
}

/// Non-scratch lanes belonging to one player side. For SP (player==1) side 0 covers every key
/// lane; for DP each side covers only its own half, so a shuffle never crosses the P1<->P2
/// boundary (that crossing is a separate FLIP option in the reference implementation, not implemented here).
pub fn side_key_lanes(mode: Mode, side: usize) -> Vec<usize> {
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
        let mut rng = side_rng(seed, side);
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

/// Lay `model`'s notes out by the whole-chart lane permutation `perm` (`perm[old_lane] = new_lane`),
/// as [`apply`] lays them out by a permutation it draws itself. [`apply`] on RANDOM with a seed
/// [`random_seed_for`] found for `perm` lays the chart out exactly so.
pub fn apply_lane_permutation(model: &mut Model, perm: &[usize]) {
    apply_perm(model, perm);
}

/// The most player sides a mode may have for [`random_seed_for`] to search it.
const SEARCHED_SIDES_MAX: usize = 2;

/// The most key lanes one side may have for [`random_seed_for`] to search it: a nine-key side. A
/// side of more keys has far more layouts than there are seeds below 2^53 (24-key has 24!), so most
/// of its layouts cannot be reached at all.
const SEARCHED_SIDE_KEYS_MAX: usize = 9;

/// How many seeds a one-side mode (5-, 7- and 9-key) is searched through. Every layout of those
/// first comes from a seed at or below 4,775,533 (measured, see
/// `docs/reference/2026-10-29-feature-upgrade/layout-pick.md`), so the search never runs out on one.
pub const RANDOM_SEED_LIMIT_SINGLE: u64 = 1 << 23;

/// How many seeds a ten-key chart is searched through. Every pair of sides RANDOM gives first comes
/// from a seed at or below 1,250,411; an eighth of the pairs comes from no seed, and those are told
/// apart from the rest once the search has been through these.
pub const RANDOM_SEED_LIMIT_TEN_KEY: u64 = 1 << 21;

/// How many seeds a fourteen-key chart is searched through. Every pair of sides first comes from a
/// seed at or below 468,482,176.
pub const RANDOM_SEED_LIMIT_FOURTEEN_KEY: u64 = 1 << 29;

/// The first seed a score server no longer keeps whole (2^53). A found seed is always below it.
const SEED_CEILING: u64 = 1 << 53;

/// How many seeds go by between two looks at the cancel flag.
const CANCEL_CHECK_SEEDS: u64 = 1 << 16;

/// The base the digits of a picked layout's row are written in.
const LAYOUT_DIGIT_RADIX: u32 = 10;

/// Why [`random_seed_for`] answered no seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedSearchError {
    /// The mode is not searched: it has more sides, or more key lanes to a side, than a seed can be
    /// found for (24-key).
    UnsupportedMode,
    /// What was asked for is not shaped as a layout RANDOM gives: not every lane once, a scratch lane
    /// moved, or a lane carried to the other side.
    NotALayout,
    /// No seed up to the limit lays the lanes out so. On ten-key this is a layout RANDOM never gives.
    Unreachable,
    /// The cancel flag was raised before a seed was found.
    Cancelled,
}

/// How many seeds [`random_seed_for`] searches a chart of `mode` through, or `None` for a mode it
/// does not search.
pub fn random_seed_limit(mode: Mode) -> Option<u64> {
    if mode == Mode::BEAT_14K {
        Some(RANDOM_SEED_LIMIT_FOURTEEN_KEY)
    } else if mode == Mode::BEAT_10K {
        Some(RANDOM_SEED_LIMIT_TEN_KEY)
    } else if [Mode::BEAT_5K, Mode::BEAT_7K, Mode::POPN_9K].contains(&mode) {
        Some(RANDOM_SEED_LIMIT_SINGLE)
    } else {
        None
    }
}

/// The first seed, counting up from 1 and no further than `limit`, that RANDOM lays a chart of
/// `mode` out from as `wanted` (`perm[old_lane] = new_lane`, the form [`lane_permutation`] answers
/// in). The search stops when `cancel` is raised.
///
/// A seed is answered only once [`lane_permutation`] has laid it out as `wanted`, so it is the very
/// layout a run started on RANDOM with it plays. It is never zero and always below 2^53.
///
/// Each seed's sides are drawn on the stack, and a side is given up at the first lane that comes out
/// wrong. The second side of a seed is drawn only when the first came out right, so a two-side
/// chart costs about what a one-side chart does per seed.
pub fn random_seed_for(mode: Mode, wanted: &[usize], limit: u64, cancel: &AtomicBool) -> Result<u64, SeedSearchError> {
    let sides = (mode.player as usize).max(1);
    let lanes: Vec<Vec<usize>> = (0..sides).map(|side| side_key_lanes(mode, side)).collect();
    if sides > SEARCHED_SIDES_MAX || lanes.iter().any(|side| side.len() > SEARCHED_SIDE_KEYS_MAX) {
        return Err(SeedSearchError::UnsupportedMode);
    }
    let targets = side_targets(mode, &lanes, wanted).ok_or(SeedSearchError::NotALayout)?;
    for seed in 1..=limit.min(SEED_CEILING - 1) {
        if seed % CANCEL_CHECK_SEEDS == 1 && cancel.load(Ordering::Relaxed) {
            return Err(SeedSearchError::Cancelled);
        }
        let drawn_alike = targets.iter().enumerate().all(|(side, target)| side_comes_out_as(seed, side, target));
        if drawn_alike && lane_permutation(NoteOption::Random, mode, seed) == wanted {
            return Ok(seed);
        }
    }
    Err(SeedSearchError::Unreachable)
}

/// `wanted` as the order each side's shuffle has to leave its key lanes in: for each key lane of a
/// side, the place among the side's key lanes of the lane it is carried to. `None` when `wanted` is
/// not shaped as a layout RANDOM gives.
fn side_targets(mode: Mode, lanes: &[Vec<usize>], wanted: &[usize]) -> Option<Vec<Vec<usize>>> {
    if wanted.len() != mode.key || (0..mode.key).any(|lane| mode.is_scratch(lane) && wanted[lane] != lane) {
        return None;
    }
    lanes
        .iter()
        .map(|side| {
            let target: Vec<usize> = side.iter().map(|lane| side.iter().position(|to| *to == wanted[*lane])).collect::<Option<_>>()?;
            let mut taken = vec![false; side.len()];
            target.iter().all(|place| !std::mem::replace(&mut taken[*place], true)).then_some(target)
        })
        .collect()
}

/// Whether side `side` of `seed` leaves its key lanes in `target`'s order. It is drawn as
/// [`fisher_yates`] draws it, on the stack, and given up at the first place that is wrong: every
/// place from the last down is final once it has been drawn.
fn side_comes_out_as(seed: u64, side: usize, target: &[usize]) -> bool {
    let mut rng = side_rng(seed, side);
    let mut places = [0usize; SEARCHED_SIDE_KEYS_MAX];
    let order = &mut places[..target.len()];
    for (place, slot) in order.iter_mut().enumerate() {
        *slot = place;
    }
    for place in (1..order.len()).rev() {
        order.swap(place, rng.below(place + 1));
        if order[place] != target[place] {
            return false;
        }
    }
    order.first() == target.first()
}

/// Whether `row` names each of `keys` key lanes, counted from 1, exactly once.
pub fn is_layout_row(row: &[u8], keys: usize) -> bool {
    row.len() == keys && (1..=keys).all(|number| row.iter().filter(|n| usize::from(**n) == number).count() == 1)
}

/// The whole-chart lane permutation a picked layout stands for.
///
/// `layout` holds a row for each player side. A row's `i`-th number is the key lane, counted from 1
/// within its side, whose notes are played on the side's `i`-th key lane from the left: the way the
/// reference writes a lane order, so `1234567` is OFF and `7654321` MIRROR. The scratch lanes stay
/// where they are. `None` when a row is not every key lane of its side exactly once.
pub fn permutation_of_layout(mode: Mode, layout: &[Vec<u8>]) -> Option<Vec<usize>> {
    if layout.len() != (mode.player as usize).max(1) {
        return None;
    }
    let mut perm: Vec<usize> = (0..mode.key).collect();
    for (side, row) in layout.iter().enumerate() {
        let lanes = side_key_lanes(mode, side);
        if !is_layout_row(row, lanes.len()) {
            return None;
        }
        for (place, number) in row.iter().enumerate() {
            perm[lanes[usize::from(*number) - 1]] = lanes[place];
        }
    }
    Some(perm)
}

/// The picked layout a whole-chart lane permutation stands for, a row a side: what
/// [`permutation_of_layout`] reads, the other way round. `None` when `perm` does not keep every key
/// lane of a side on that side.
pub fn layout_of_permutation(mode: Mode, perm: &[usize]) -> Option<Vec<Vec<u8>>> {
    if perm.len() != mode.key {
        return None;
    }
    (0..(mode.player as usize).max(1))
        .map(|side| {
            let lanes = side_key_lanes(mode, side);
            let row: Vec<u8> =
                lanes.iter().map(|to| lanes.iter().position(|from| perm[*from] == *to).and_then(|from| u8::try_from(from + 1).ok())).collect::<Option<_>>()?;
            is_layout_row(&row, lanes.len()).then_some(row)
        })
        .collect()
}

/// A row of a picked layout read from its digits (`3517246`), or `None` when they are not each of
/// `keys` key lanes once.
pub fn parse_layout_row(text: &str, keys: usize) -> Option<Vec<u8>> {
    let row: Vec<u8> = text.chars().map(|c| c.to_digit(LAYOUT_DIGIT_RADIX).and_then(|digit| u8::try_from(digit).ok())).collect::<Option<_>>()?;
    is_layout_row(&row, keys).then_some(row)
}

/// A row of a picked layout written as its digits.
pub fn layout_row_text(row: &[u8]) -> String {
    row.iter().map(u8::to_string).collect()
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

            let mut pool: Vec<usize> = (lo..hi).filter(|&l| (all_scratch || !mode.is_scratch(l)) && !used[l]).collect();
            fisher_yates(&mut pool, &mut rng);
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
        for opt in
            [NoteOption::Mirror, NoteOption::Random, NoteOption::SRandom, NoteOption::RRandom, NoteOption::Rotate, NoteOption::HRandom, NoteOption::AllScratch]
        {
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
        let dp = to_model(&parse(b"#PLAYER 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01000100\r\n#00121:00110011\r\n#00126:00010001\r\n"), Mode::BEAT_14K);
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
            for (lane, mapped) in perm.iter().enumerate() {
                assert_eq!(lane < 8, *mapped < 8, "{:?}: lane {lane} crossed the P1/P2 boundary to {}", opt, mapped);
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
    fn from_str_trait_matches_inherent_parse() {
        for label in ["mirror", "S-RANDOM", "ALL-SCR", "nonsense", ""] {
            let via_trait: NoteOption = label.parse().unwrap();
            assert_eq!(via_trait, NoteOption::from_str(label), "FromStr must agree with the inherent parser for {label:?}");
        }
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

    #[test]
    fn mirror_fully_reverses_popn_lanes() {
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
        let perm = lane_permutation(NoteOption::Mirror, Mode::BEAT_7K, 0);
        for l in 0..8 {
            assert_eq!(perm[perm[l]], l, "mirror twice is identity on lane {l}");
        }
    }

    #[test]
    fn rotate_is_a_derangement_on_key_lanes() {
        let perm = lane_permutation(NoteOption::Rotate, Mode::BEAT_7K, 0);
        for (l, mapped) in perm.iter().enumerate().take(7) {
            assert_ne!(*mapped, l, "key lane {l} must move under ROTATE");
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
        let perm = lane_permutation(NoteOption::Mirror, Mode::BEAT_14K, 0);
        assert_eq!(perm, vec![6, 5, 4, 3, 2, 1, 0, 7, 14, 13, 12, 11, 10, 9, 8, 15]);
    }

    #[test]
    fn dp_perm_never_crosses_player_boundary() {
        for opt in [NoteOption::Mirror, NoteOption::Random, NoteOption::RRandom, NoteOption::Rotate] {
            let perm = lane_permutation(opt, Mode::BEAT_14K, 4242);
            for (lane, mapped) in perm.iter().enumerate() {
                assert_eq!(lane < 8, *mapped < 8, "{:?}: lane {lane} crossed sides to {}", opt, mapped);
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
        let perm = lane_permutation(NoteOption::Random, Mode::BEAT_7K, 0);
        let mut sorted = perm.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..8).collect::<Vec<_>>(), "still a bijection at seed 0");
    }

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
        let base = to_model(&parse(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00113:00110011\r\n#00116:01000010\r\n"), Mode::BEAT_5K);
        let before = note_count(&base);
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 88);
            assert_eq!(note_count(&m), before, "{:?} preserves note count (5K)", opt);
        }
    }

    #[test]
    fn apply_preserves_count_for_every_option_popn() {
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
        let base =
            to_model(&parse(b"#PLAYER 2\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01000100\r\n#00121:00110011\r\n#00126:00010001\r\n"), Mode::BEAT_10K);
        let before = note_count(&base);
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 71);
            assert_eq!(note_count(&m), before, "{:?} preserves note count (10K DP)", opt);
        }
    }

    #[test]
    fn key_shuffles_keep_scratch_notes_on_scratch_lane() {
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
        let chart = b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01010101\r\n#00113:00110011\r\n";
        let base = model(chart);
        let scratch_before = scratch_rows(&base);
        let mut m = base.clone();
        apply(&mut m, NoteOption::HRandom, 17);
        assert_eq!(scratch_rows(&m), scratch_before, "H-RANDOM leaves scratch notes in place");
    }

    #[test]
    fn every_option_keeps_single_ln_head_and_tail_aligned() {
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
        let chart = b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00111:01010101\r\n#00113:01010101\r\n";
        let base = model(chart);
        let mut m = base.clone();
        apply(&mut m, NoteOption::SRandom, 808);
        assert_eq!(note_count(&m), note_count(&base));
        assert!(ln_pairs_aligned(&m), "S-RANDOM keeps the open LN pinned");
    }

    #[test]
    fn all_options_are_deterministic_for_a_seed() {
        let chart = b"#BPM 180\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00114:00010100\r\n#00116:01000010\r\n#00151:01000001\r\n";
        for opt in NoteOption::ALL {
            let mut a = model(chart);
            let mut b = model(chart);
            apply(&mut a, opt, 4321);
            apply(&mut b, opt, 4321);
            let pos = |m: &Model| -> Vec<Vec<bool>> { m.timelines.iter().map(|tl| tl.notes.iter().map(|s| s.is_some()).collect()).collect() };
            assert_eq!(pos(&a), pos(&b), "{:?} must be deterministic for a fixed seed", opt);
        }
    }

    #[test]
    fn different_seeds_can_produce_different_random_layouts() {
        let p0 = lane_permutation(NoteOption::Random, Mode::BEAT_7K, 0);
        let differs = [1u64, 2, 3, 4, 5].iter().any(|&s| lane_permutation(NoteOption::Random, Mode::BEAT_7K, s) != p0);
        assert!(differs, "RANDOM should vary across seeds");
    }

    #[test]
    fn time_based_options_survive_fully_dense_rows() {
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
        let base = model(b"#BPM 120\r\n#WAV01 a.wav\r\n");
        for opt in NoteOption::ALL {
            let mut m = base.clone();
            apply(&mut m, opt, 1);
            assert_eq!(note_count(&m), 0, "{:?} on an empty chart stays empty", opt);
        }
    }

    #[test]
    fn all_scratch_raises_scratch_population_above_off() {
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

    /// The seed the golden layouts below are laid out from: a seed as a run of this player draws
    /// one, inside the 53 bits a score server keeps whole.
    const GOLDEN_SEED: u64 = 0x001E_D5A7_C3B9_1F42;

    /// A chart for each mode the golden layouts are of: first every lane alone on a row of its own,
    /// in lane order, which shows where each lane went; then chords; then rows played across a long
    /// note, which a note-by-note arrangement has to keep in its lane.
    const GOLDEN_CHARTS: [(Mode, &[u8]); 3] = [
        (Mode::BEAT_7K, b"#BPM 150\n#WAV01 a.wav\n#00111:0100000000000000\n#00112:0001000000000000\n#00113:0000010000000000\n#00114:0000000100000000\n#00115:0000000001000000\n#00118:0000000000010000\n#00119:0000000000000100\n#00116:0000000000000001\n#00211:01000100\n#00212:00010001\n#00213:01010000\n#00215:00000101\n#00218:01000001\n#00216:01000100\n#00351:0100000000000001\n#00312:0101010100000000\n#00314:0000000001010101\n#00319:0100000001000000\n"),
        (Mode::BEAT_14K, b"#PLAYER 3\n#BPM 150\n#WAV01 a.wav\n#00111:01000000000000000000000000000000\n#00112:00010000000000000000000000000000\n#00113:00000100000000000000000000000000\n#00114:00000001000000000000000000000000\n#00115:00000000010000000000000000000000\n#00118:00000000000100000000000000000000\n#00119:00000000000001000000000000000000\n#00116:00000000000000010000000000000000\n#00121:00000000000000000100000000000000\n#00122:00000000000000000001000000000000\n#00123:00000000000000000000010000000000\n#00124:00000000000000000000000100000000\n#00125:00000000000000000000000001000000\n#00128:00000000000000000000000000010000\n#00129:00000000000000000000000000000100\n#00126:00000000000000000000000000000001\n#00211:01000100\n#00212:00010001\n#00213:01010000\n#00215:00000101\n#00218:01000001\n#00216:01000100\n#00351:0100000000000001\n#00312:0101010100000000\n#00314:0000000001010101\n#00319:0100000001000000\n#00221:00010100\n#00223:01000001\n#00224:00010001\n#00229:01010000\n#00226:00000001\n#00361:0000000100000001\n#00322:0101000001010000\n#00325:0000010100000101\n"),
        (Mode::POPN_9K, b"#BPM 150\n#WAV01 a.wav\n#00111:010000000000000000\n#00112:000100000000000000\n#00113:000001000000000000\n#00114:000000010000000000\n#00115:000000000100000000\n#00122:000000000001000000\n#00123:000000000000010000\n#00124:000000000000000100\n#00125:000000000000000001\n#00211:01000100\n#00212:00010001\n#00213:01010000\n#00215:00000101\n#00223:01000001\n#00225:01000100\n#00314:0101010100000000\n#00322:0000000001010101\n#00351:0100000000000001\n"),
    ];

    /// Where the notes of a chart are once laid out: each row that holds a note, as the lanes it
    /// holds one in, one hexadecimal digit a lane, rows apart by a space.
    fn laid_out(m: &Model) -> String {
        let row = |notes: &[Option<rbms_model::Note>]| {
            notes.iter().enumerate().filter(|(_, note)| note.is_some()).map(|(lane, _)| format!("{lane:x}")).collect::<String>()
        };
        m.timelines.iter().map(|tl| row(&tl.notes)).filter(|lanes| !lanes.is_empty()).collect::<Vec<_>>().join(" ")
    }

    /// Every arrangement on every golden chart, as it lays the chart out from [`GOLDEN_SEED`]: the
    /// mode's name, the arrangement and the notes where they land ([`laid_out`]). A replay states only
    /// its arrangement and its seed, so these are what a replay recorded by any earlier build is
    /// played back on and what a run borrowing its layout is played on.
    const GOLDEN_LAYOUTS: [(&str, NoteOption, &str); 24] = [
        ("BEAT_7K", NoteOption::Off, "0 1 2 3 4 5 6 7 0257 12 047 145 016 1 1 1 36 3 3 03"),
        ("BEAT_7K", NoteOption::Mirror, "6 5 4 3 2 1 0 7 1467 45 267 125 056 5 5 5 03 3 3 36"),
        ("BEAT_7K", NoteOption::Random, "5 4 1 2 0 6 3 7 1567 14 057 046 345 4 4 4 23 2 2 25"),
        ("BEAT_7K", NoteOption::SRandom, "2 1 0 4 2 1 3 7 0167 23 367 235 036 0 5 0 23 2 0 36"),
        ("BEAT_7K", NoteOption::RRandom, "1 2 3 4 5 6 0 7 1367 23 157 256 012 2 2 2 04 4 4 14"),
        ("BEAT_7K", NoteOption::Rotate, "1 2 3 4 5 6 0 7 1367 23 157 256 012 2 2 2 04 4 4 14"),
        ("BEAT_7K", NoteOption::HRandom, "2 2 2 4 3 3 3 7 0127 26 257 026 015 6 1 6 02 5 6 03"),
        ("BEAT_7K", NoteOption::AllScratch, "7 7 7 7 7 7 7 7 2347 57 017 067 057 7 7 7 27 7 7 17"),
        ("BEAT_14K", NoteOption::Off, "0 1 2 3 4 5 6 7 8 9 a b c d e f 0257ae 128be 0478 145abf 0169 19 1c 18c 369 39 3c 038c"),
        ("BEAT_14K", NoteOption::Mirror, "6 5 4 3 2 1 0 7 e d c b a 9 8 f 14678c 458be 267e 125bcf 056d 5d 5a 5ae 03d 3d 3a 36ae"),
        ("BEAT_14K", NoteOption::Random, "5 4 1 2 0 6 3 7 9 d e b c a 8 f 15678e 1489b 0579 046bef 345d 4d 4c 49c 23d 2d 2c 259c"),
        ("BEAT_14K", NoteOption::SRandom, "2 5 2 6 1 2 3 7 a c d 8 b b e f 23479a 129ce 457d 2568ef 145e 5e 3e 29b 128 2a 6d 04bc"),
        ("BEAT_14K", NoteOption::RRandom, "1 2 3 4 5 6 0 7 c d e 8 9 a b f 1367be 238bc 157c 2568ef 012d 2d 29 29c 04d 4d 49 149c"),
        ("BEAT_14K", NoteOption::Rotate, "1 2 3 4 5 6 0 7 9 a b c d e 8 f 13678b 2389c 1579 256bcf 012a 2a 2d 29d 04a 4a 4d 149d"),
        ("BEAT_14K", NoteOption::HRandom, "2 0 2 4 3 6 3 7 a e d e 8 e 9 f 02379d 16abc 2579 345bcf 124c 48 08 08b 45d 2b 59 459b"),
        ("BEAT_14K", NoteOption::AllScratch, "7 7 7 7 7 7 7 7 f f f f f f f f 2367df 3789f 137f 0578cf 147f 7f 7f 7bf 67f 7f 7f 27df"),
        ("POPN_9K", NoteOption::Off, "0 1 2 3 4 5 6 7 8 0268 12 048 146 03 3 3 3 5 5 5 05"),
        ("POPN_9K", NoteOption::Mirror, "8 7 6 5 4 3 2 1 0 0268 67 048 247 58 5 5 5 3 3 3 38"),
        ("POPN_9K", NoteOption::Random, "8 7 5 0 6 4 2 1 3 2358 57 368 267 08 0 0 0 4 4 4 48"),
        ("POPN_9K", NoteOption::SRandom, "6 3 0 5 1 6 5 2 1 0345 25 047 478 12 1 1 7 6 4 7 12"),
        ("POPN_9K", NoteOption::RRandom, "3 4 5 6 7 8 0 1 2 0235 45 237 047 36 6 6 6 8 8 8 38"),
        ("POPN_9K", NoteOption::Rotate, "1 2 3 4 5 6 7 8 0 0137 23 015 257 14 4 4 4 6 6 6 16"),
        ("POPN_9K", NoteOption::HRandom, "2 6 4 6 4 7 4 1 8 0367 46 257 267 24 2 6 1 7 2 1 28"),
        ("POPN_9K", NoteOption::AllScratch, "2 6 4 6 4 7 4 1 8 0367 46 257 267 24 2 6 1 7 2 1 28"),
    ];

    /// The arrangements lay the golden charts out as they always have, for every option and for the
    /// seven-key, fourteen-key and nine-key modes alike. A change here changes the layout of every
    /// replay already recorded, and of every run that borrows one.
    #[test]
    fn every_arrangement_lays_the_golden_charts_out_as_it_always_has() {
        for (mode, chart) in GOLDEN_CHARTS {
            for option in NoteOption::ALL {
                let mut m = to_model(&parse(chart), mode);
                apply(&mut m, option, GOLDEN_SEED);
                let golden =
                    GOLDEN_LAYOUTS.iter().find(|(name, golden_option, _)| *name == mode.name && *golden_option == option).map(|(_, _, layout)| *layout);
                assert_eq!(Some(laid_out(&m).as_str()), golden, "{} {option:?}", mode.name);
            }
        }
    }

    /// How many layouts a five-key side has: 5!.
    const FIVE_KEY_LAYOUTS: usize = 120;

    /// A picked layout of `mode` written a row a side, as the lane permutation it stands for.
    fn picked(mode: Mode, rows: &[&str]) -> Vec<usize> {
        let keys = side_key_lanes(mode, 0).len();
        let layout: Vec<Vec<u8>> = rows.iter().map(|row| parse_layout_row(row, keys).expect("a layout row")).collect();
        permutation_of_layout(mode, &layout).expect("a layout")
    }

    /// The first seed RANDOM lays `wanted` out from, searched as far as the mode is.
    fn seed_of(mode: Mode, wanted: &[usize]) -> Result<u64, SeedSearchError> {
        random_seed_for(mode, wanted, random_seed_limit(mode).expect("a searched mode"), &AtomicBool::new(false))
    }

    /// A layout is written the way the reference writes a lane order: the `i`-th digit is the lane
    /// whose notes land on the `i`-th lane from the left. OFF and MIRROR read as themselves, and
    /// RANDOM's layout from the golden seed is the one the golden chart shows.
    #[test]
    fn a_layout_row_names_the_lane_each_place_plays() {
        assert_eq!(picked(Mode::BEAT_7K, &["1234567"]), lane_permutation(NoteOption::Off, Mode::BEAT_7K, 0));
        assert_eq!(picked(Mode::BEAT_7K, &["7654321"]), lane_permutation(NoteOption::Mirror, Mode::BEAT_7K, 0));
        assert_eq!(picked(Mode::BEAT_7K, &["3517246"]), vec![2, 4, 0, 5, 1, 6, 3, 7]);
        let golden = lane_permutation(NoteOption::Random, Mode::BEAT_7K, GOLDEN_SEED);
        assert_eq!(layout_of_permutation(Mode::BEAT_7K, &golden).map(|rows| layout_row_text(&rows[0])), Some("5347216".to_string()));
        assert_eq!(picked(Mode::BEAT_14K, &["7654321", "7654321"]), lane_permutation(NoteOption::Mirror, Mode::BEAT_14K, 0));
        assert_eq!(picked(Mode::BEAT_14K, &["1234567", "2134567"]), vec![0, 1, 2, 3, 4, 5, 6, 7, 9, 8, 10, 11, 12, 13, 14, 15]);
    }

    /// A row has to be every key lane of its side once: too short, too long, a lane twice, a zero or
    /// a lane past the side, and anything but a digit, are not a layout.
    #[test]
    fn a_layout_row_is_every_lane_of_its_side_once() {
        for text in ["123456", "12345678", "1234566", "0123456", "1234568", "12a4567", ""] {
            assert_eq!(parse_layout_row(text, 7), None, "{text:?}");
        }
        assert_eq!(parse_layout_row("987654321", 9), Some(vec![9, 8, 7, 6, 5, 4, 3, 2, 1]));
        assert_eq!(permutation_of_layout(Mode::BEAT_14K, &[vec![1, 2, 3, 4, 5, 6, 7]]), None, "a fourteen-key layout has two rows");
        assert_eq!(permutation_of_layout(Mode::BEAT_7K, &[vec![1, 1, 3, 4, 5, 6, 7]]), None);
    }

    /// Reading a layout and writing it again gives it back, on every searched mode.
    #[test]
    fn a_layout_survives_the_round_trip_through_its_permutation() {
        for (mode, rows) in [
            (Mode::BEAT_5K, vec!["35142"]),
            (Mode::BEAT_7K, vec!["3517246"]),
            (Mode::POPN_9K, vec!["987654321"]),
            (Mode::BEAT_10K, vec!["21543", "35142"]),
            (Mode::BEAT_14K, vec!["3517246", "7123456"]),
        ] {
            let back = layout_of_permutation(mode, &picked(mode, &rows)).expect("a layout");
            assert_eq!(back.iter().map(|row| layout_row_text(row)).collect::<Vec<_>>(), rows, "{}", mode.name);
        }
    }

    /// The seeds the survey measured for some seven-, five- and nine-key layouts are the ones found,
    /// and each lays the layout out exactly.
    #[test]
    fn the_seed_found_for_a_layout_is_the_first_that_lays_it_out() {
        let known = [
            (Mode::BEAT_7K, "3517246", 4761),
            (Mode::BEAT_7K, "1234567", 24_088),
            (Mode::BEAT_7K, "7654321", 1048),
            (Mode::BEAT_7K, "7123456", 2272),
            (Mode::BEAT_7K, "2345671", 14_563),
            (Mode::BEAT_5K, "12345", 225),
            (Mode::BEAT_5K, "54321", 39),
            (Mode::POPN_9K, "987654321", 590_676),
        ];
        for (mode, row, seed) in known {
            let wanted = picked(mode, &[row]);
            assert_eq!(seed_of(mode, &wanted), Ok(seed), "{} {row}", mode.name);
            assert_eq!(lane_permutation(NoteOption::Random, mode, seed), wanted, "{} {row}", mode.name);
            assert!((1..seed).all(|earlier| lane_permutation(NoteOption::Random, mode, earlier) != wanted), "{} {row} came out earlier", mode.name);
        }
    }

    /// Every five-key layout is found, inside the limit, by a seed that lays it out.
    #[test]
    fn every_five_key_layout_has_a_seed() {
        let mode = Mode::BEAT_5K;
        let mut found = std::collections::HashSet::new();
        for seed in 1..=RANDOM_SEED_LIMIT_SINGLE {
            if found.insert(lane_permutation(NoteOption::Random, mode, seed)) && found.len() == FIVE_KEY_LAYOUTS {
                break;
            }
        }
        assert_eq!(found.len(), FIVE_KEY_LAYOUTS);
        for wanted in found {
            let seed = seed_of(mode, &wanted).expect("a seed");
            assert_eq!(lane_permutation(NoteOption::Random, mode, seed), wanted);
        }
    }

    /// A two-side layout is found as the pair it is: the seed lays both sides out, and it is no later
    /// than a seed known to give the pair.
    #[test]
    fn a_two_side_layout_is_found_side_by_side() {
        for mode in [Mode::BEAT_14K, Mode::BEAT_10K] {
            for known in [1, 2, 37, 1000, 123_457] {
                let wanted = lane_permutation(NoteOption::Random, mode, known);
                let seed = seed_of(mode, &wanted).expect("a seed");
                assert!(seed <= known, "{} {known}: {seed}", mode.name);
                assert_eq!(lane_permutation(NoteOption::Random, mode, seed), wanted, "{} {known}", mode.name);
            }
        }
    }

    /// An eighth of the ten-key pairs comes from no seed, and is reported as such; the search for one
    /// goes through every seed of the limit.
    #[test]
    fn a_ten_key_pair_random_never_gives_is_unreachable() {
        let wanted = picked(Mode::BEAT_10K, &["12345", "51234"]);
        assert_eq!(seed_of(Mode::BEAT_10K, &wanted), Err(SeedSearchError::Unreachable));
        let either_side_alone = [picked(Mode::BEAT_10K, &["12345", "12345"]), picked(Mode::BEAT_10K, &["51234", "51234"])];
        for wanted in either_side_alone {
            assert!(seed_of(Mode::BEAT_10K, &wanted).is_ok());
        }
    }

    /// Twenty-four keys are not searched, and what is not shaped as a layout is not looked for.
    #[test]
    fn a_search_is_refused_what_it_cannot_find() {
        let never = AtomicBool::new(false);
        let identity: Vec<usize> = (0..Mode::KEYBOARD_24K.key).collect();
        assert_eq!(random_seed_limit(Mode::KEYBOARD_24K), None);
        assert_eq!(random_seed_for(Mode::KEYBOARD_24K, &identity, RANDOM_SEED_LIMIT_SINGLE, &never), Err(SeedSearchError::UnsupportedMode));
        let mut scratch_moved = picked(Mode::BEAT_7K, &["1234567"]);
        scratch_moved.swap(0, 7);
        let crossing = {
            let mut perm = picked(Mode::BEAT_14K, &["1234567", "1234567"]);
            perm.swap(0, 8);
            perm
        };
        assert_eq!(random_seed_for(Mode::BEAT_7K, &scratch_moved, RANDOM_SEED_LIMIT_SINGLE, &never), Err(SeedSearchError::NotALayout));
        assert_eq!(random_seed_for(Mode::BEAT_14K, &crossing, RANDOM_SEED_LIMIT_FOURTEEN_KEY, &never), Err(SeedSearchError::NotALayout));
        assert_eq!(random_seed_for(Mode::BEAT_7K, &[0, 0, 2, 3, 4, 5, 6, 7], RANDOM_SEED_LIMIT_SINGLE, &never), Err(SeedSearchError::NotALayout));
        assert_eq!(random_seed_for(Mode::BEAT_7K, &[0, 1, 2], RANDOM_SEED_LIMIT_SINGLE, &never), Err(SeedSearchError::NotALayout));
    }

    /// A raised cancel flag stops the search, and a limit short of the seed leaves it unfound.
    #[test]
    fn a_search_stops_when_cancelled_or_out_of_seeds() {
        let wanted = picked(Mode::BEAT_7K, &["1234567"]);
        assert_eq!(random_seed_for(Mode::BEAT_7K, &wanted, RANDOM_SEED_LIMIT_SINGLE, &AtomicBool::new(true)), Err(SeedSearchError::Cancelled));
        assert_eq!(random_seed_for(Mode::BEAT_7K, &wanted, 24_087, &AtomicBool::new(false)), Err(SeedSearchError::Unreachable));
    }

    /// A chart laid out by a picked layout is the chart RANDOM lays out from the seed found for it.
    #[test]
    fn a_picked_layout_lays_the_chart_out_as_its_seed_does() {
        let (mode, chart) = GOLDEN_CHARTS[0];
        let wanted = picked(mode, &["3517246"]);
        let seed = seed_of(mode, &wanted).expect("a seed");
        let mut by_seed = to_model(&parse(chart), mode);
        apply(&mut by_seed, NoteOption::Random, seed);
        let mut by_layout = to_model(&parse(chart), mode);
        apply_lane_permutation(&mut by_layout, &wanted);
        assert_eq!(laid_out(&by_layout), laid_out(&by_seed));
    }
}

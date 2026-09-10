//! The score axis the result screen reads a run's grade off: the twenty-seven steps every DJ band
//! boundary is a multiple of, and the grade a score on one of those steps is called.
//!
//! The eight bands and the bar they are drawn as live next door in the screen itself; what is here
//! is the finer scale laid over them, which only the result screen uses.

use crate::{Color, Rect, Renderer};

use super::{RANK_BANDS, draw_rank_bar};

/// How many equal steps the score axis is cut into. Every band boundary of [`RANK_BANDS`] is a
/// multiple of three of them, which is what lets one axis carry both the eight bands and the finer
/// grade the suffix in [`dj_rank_label`] names (`BooleanPropertyFactory.java:280-288`).
pub const RATE_STEPS: u32 = 27;

/// First step of each band of [`RANK_BANDS`], with a trailing [`RATE_STEPS`] closing the top one.
/// These are the reference's own band boundaries (`BooleanPropertyFactory.java:280` and the
/// `qualifyRank` indices at `:619-626`), which are `RANK_BOUNDS` in whole steps; the float bounds
/// stay for the widths the rank bar is drawn with, and a test pins that the two agree.
///
/// No table of the twenty-seven boundaries themselves is kept: they are `step / RATE_STEPS` exactly,
/// and a baked copy could only drift away from [`rate_27`].
pub(super) const BAND_FIRST_STEP: [u32; RANK_BANDS.len() + 1] = [0, 6, 9, 12, 15, 18, 21, 24, RATE_STEPS];

/// How many grades one band is split into: the low third, the middle, and the high third.
const BAND_GRADES: usize = 3;

/// The grade each third of each band is shown as, low band and low third first.
///
/// The top third of `AAA` is named for how close it is to a perfect score rather than for its band,
/// because a run one twenty-seventh short of everything is read as "just under MAX" — which is also
/// what the reference calls that step when it is asked for as a target (`RATE_MAX-`,
/// `TargetProperty.java:136-137`).
const BAND_GRADE_LABELS: [[&str; BAND_GRADES]; RANK_BANDS.len()] = [
    ["F-", "F", "F+"],
    ["E-", "E", "E+"],
    ["D-", "D", "D+"],
    ["C-", "C", "C+"],
    ["B-", "B", "B+"],
    ["A-", "A", "A+"],
    ["AA-", "AA", "AA+"],
    ["AAA-", "AAA", "MAX-"],
];

/// The grade a run that took every point there was is shown as.
const PERFECT_GRADE: &str = "MAX";

/// How many whole twenty-sevenths of a chart's ceiling a score reached, `0..=RATE_STEPS`.
///
/// The test is the lower bound `ex / max_ex >= step / RATE_STEPS`, evaluated in integers so a score
/// standing exactly on a boundary lands on its own step rather than one below it.
pub fn rate_27(ex: u32, max_ex: u32) -> u8 {
    if max_ex == 0 {
        return 0;
    }
    let step = u64::from(ex) * u64::from(RATE_STEPS) / u64::from(max_ex);
    step.min(u64::from(RATE_STEPS)) as u8
}

/// The DJ grade of a score: the band [`dj_rank`] puts it in, marked `-`, unmarked or `+` for the
/// third of that band it sits in, and [`PERFECT_GRADE`] for exactly the chart's ceiling.
///
/// The band still comes first, so the grade always agrees with the colour and the bar segment the
/// same score is drawn with.
pub fn dj_rank_label(ex: u32, max_ex: u32) -> &'static str {
    let step = u32::from(rate_27(ex, max_ex));
    if step >= RATE_STEPS {
        return PERFECT_GRADE;
    }
    let band = band_of_step(step);
    let within = step - BAND_FIRST_STEP[band];
    let span = BAND_FIRST_STEP[band + 1] - BAND_FIRST_STEP[band];
    BAND_GRADE_LABELS[band][(within * BAND_GRADES as u32 / span) as usize]
}

/// Which band of [`RANK_BANDS`] a step belongs to. The first band starts at step zero, so every
/// step below [`RATE_STEPS`] has one.
fn band_of_step(step: u32) -> usize {
    BAND_FIRST_STEP[..RANK_BANDS.len()].iter().rposition(|first| step >= *first).unwrap_or(0)
}

/// Width of one of the fine step ticks laid over the rank bar.
const RATE_TICK_W: f32 = 1.0;

/// How far up the bar a fine step tick reaches, as a fraction of its height.
const RATE_TICK_FRACTION: f32 = 0.45;

/// Colour of the fine step ticks: dark and part-transparent, so they read as scale marks over
/// whichever band colour they fall on.
const RATE_TICK_COLOR: Color = Color { r: 10, g: 10, b: 14, a: 120 };

/// [`draw_rank_bar`] with the twenty-seven steps of [`RATE_STEPS`] ticked over it, so a score can be
/// read to the grade [`dj_rank_label`] names rather than only to its band.
///
/// The plain bar is left exactly as it is: the browser's record panel draws it too, and one screen's
/// finer scale is no reason to move the other's pixels.
pub fn draw_rank_bar_stepped<R: Renderer>(r: &mut R, x: f32, y: f32, w: f32, h: f32, ex: u32, max_ex: u32) {
    draw_rank_bar(r, x, y, w, h, ex, max_ex);
    let tick_h = h * RATE_TICK_FRACTION;
    for step in 1..RATE_STEPS {
        let sx = x + w * step as f32 / RATE_STEPS as f32;
        r.fill_rect(Rect::new(sx, y + h - tick_h, RATE_TICK_W, tick_h), RATE_TICK_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::{RANK_BOUNDS, dj_rank};

    /// Every grade the score axis can be read as, low first: the three thirds of each band, then
    /// the perfect score. This is the full label set, so a missing or duplicated entry shows up
    /// here rather than on screen.
    const EVERY_GRADE: [&str; RANK_BANDS.len() * BAND_GRADES + 1] = [
        "F-", "F", "F+", "E-", "E", "E+", "D-", "D", "D+", "C-", "C", "C+", "B-", "B", "B+", "A-", "A", "A+", "AA-", "AA", "AA+", "AAA-", "AAA", "MAX-", "MAX",
    ];

    /// A chart whose ceiling divides evenly by twenty-seven, so every step lands on a whole EX and
    /// the boundaries can be named exactly.
    const STEPPED_MAX_EX: u32 = 27 * 40;

    #[test]
    fn a_rate_step_counts_the_whole_twenty_sevenths_a_score_reached() {
        assert_eq!(rate_27(0, STEPPED_MAX_EX), 0);
        assert_eq!(rate_27(STEPPED_MAX_EX, STEPPED_MAX_EX), RATE_STEPS as u8, "a perfect score reaches every step");
        for step in 0..RATE_STEPS {
            let at = STEPPED_MAX_EX * step / RATE_STEPS;
            assert_eq!(rate_27(at, STEPPED_MAX_EX), step as u8, "standing exactly on step {step}");
            if step > 0 {
                assert_eq!(rate_27(at - 1, STEPPED_MAX_EX), step as u8 - 1, "one point short of step {step}");
            }
        }
    }
    #[test]
    fn a_rate_step_never_falls_as_the_score_climbs_and_never_leaves_the_axis() {
        let max = 1800u32;
        let mut previous = 0u8;
        for ex in 0..=max {
            let step = rate_27(ex, max);
            assert!(step >= previous, "step fell from {previous} to {step} at ex {ex}");
            assert!(u32::from(step) <= RATE_STEPS);
            previous = step;
        }
        assert_eq!(previous, RATE_STEPS as u8);
    }
    #[test]
    fn a_chart_with_no_ceiling_reads_as_the_bottom_step_whatever_the_score() {
        assert_eq!(rate_27(0, 0), 0, "no notes, no divide by zero");
        assert_eq!(rate_27(999, 0), 0);
    }
    /// A score above the ceiling cannot be produced by play, but a stored record from another rule
    /// version can be, and it must not index past the axis.
    #[test]
    fn a_score_over_the_ceiling_stops_at_the_top_step() {
        assert_eq!(rate_27(u32::MAX, 100), RATE_STEPS as u8);
        assert_eq!(dj_rank_label(u32::MAX, 100), PERFECT_GRADE);
    }
    /// The step axis and the band axis are the same axis: whichever way a score is read, it lands in
    /// the same band. This is what lets the grade keep its band's colour.
    #[test]
    fn the_step_axis_and_the_band_axis_agree_everywhere() {
        let max = 1800u32;
        for ex in 0..=max {
            let step = u32::from(rate_27(ex, max));
            let expected = dj_rank(ex, max);
            let band = band_of_step(step.min(RATE_STEPS - 1));
            assert_eq!(band, expected, "ex {ex}/{max} is band {expected} by rate and {band} by step");
        }
    }
    #[test]
    fn every_grade_is_reachable_and_no_two_steps_share_one() {
        let mut seen: Vec<&str> = (0..=RATE_STEPS).map(|step| dj_rank_label(STEPPED_MAX_EX * step / RATE_STEPS, STEPPED_MAX_EX)).collect();
        seen.dedup();
        assert_eq!(seen, EVERY_GRADE.to_vec(), "the twenty-eight steps read out as the whole grade list in order");
        let mut unique = EVERY_GRADE.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), EVERY_GRADE.len(), "two grades share a name");
    }
    /// Every grade but the perfect one starts with its own band's name, which is what keeps the
    /// letter and the colour beside it from disagreeing.
    #[test]
    fn a_grade_carries_the_name_of_the_band_it_is_in() {
        let max = 1800u32;
        for ex in 0..=max {
            let grade = dj_rank_label(ex, max);
            if grade == PERFECT_GRADE || grade == "MAX-" {
                continue;
            }
            let band = RANK_BANDS[dj_rank(ex, max)].0;
            assert!(grade.trim_end_matches(['-', '+']) == band, "grade {grade} sits in band {band} at ex {ex}");
        }
    }
    /// The top slice of the chart is named for how far it is from everything, not for its band: the
    /// last twenty-seventh is `MAX-` and only a whole score is `MAX`.
    #[test]
    fn the_last_step_below_a_perfect_score_is_named_for_the_perfect_score() {
        let max = STEPPED_MAX_EX;
        let last_step = max * (RATE_STEPS - 1) / RATE_STEPS;
        assert_eq!(dj_rank_label(last_step, max), "MAX-");
        assert_eq!(dj_rank_label(max - 1, max), "MAX-");
        assert_eq!(dj_rank_label(max, max), PERFECT_GRADE);
        assert_eq!(dj_rank_label(last_step - 1, max), "AAA");
    }
    #[test]
    fn the_band_step_table_and_the_band_bounds_describe_the_same_boundaries() {
        assert_eq!(BAND_FIRST_STEP.len(), RANK_BOUNDS.len());
        for (at, first) in BAND_FIRST_STEP.iter().enumerate() {
            let from_bounds = RANK_BOUNDS[at] * RATE_STEPS as f32;
            assert!((from_bounds - *first as f32).abs() < 0.001, "band {at} starts at {first} steps but {from_bounds} by rate");
        }
    }
    #[test]
    fn the_stepped_rank_bar_adds_its_ticks_to_the_bar_the_browser_draws() {
        use crate::CpuCanvas;
        let (w, h) = (400u32, 40u32);
        let mut plain = CpuCanvas::new(w, h);
        let mut stepped = CpuCanvas::new(w, h);
        draw_rank_bar(&mut plain, 10.0, 10.0, 380.0, 18.0, 900, 1800);
        draw_rank_bar_stepped(&mut stepped, 10.0, 10.0, 380.0, 18.0, 900, 1800);
        assert_ne!(plain.pixels(), stepped.pixels(), "the fine steps are not drawn");
    }
    #[test]
    fn the_stepped_rank_bar_survives_a_chart_with_no_ceiling() {
        use crate::CpuCanvas;
        let mut canvas = CpuCanvas::new(400, 40);
        draw_rank_bar_stepped(&mut canvas, 10.0, 10.0, 380.0, 18.0, 0, 0);
        draw_rank_bar_stepped(&mut canvas, 10.0, 10.0, 380.0, 18.0, 1800, 1800);
    }
}

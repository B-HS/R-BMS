//! What a run is paced against: the EX score the TARGET row is asking for.
//!
//! The fixed rate targets and the "next rank" walk are the reference implementation's own
//! (`TargetProperty.java:104-152` and `:301-331`); both are pure arithmetic over the chart's note
//! count, so they are settled here and the screens only have to draw the number.
//!
//! The targets that need something outside the chart — the local best, an account's own best, a
//! rival's — are settled here too, against a [`TargetContext`] the caller fills in from whatever has
//! arrived; anything still missing falls back rather than leaving the screen without a target.

use rbms_config::ScoreTarget;

/// How many equal bands the score axis is cut into. Every fixed target and every DJ rank boundary
/// is a multiple of one of these (`TargetProperty.java:117-141`).
pub const SCORE_BANDS: u32 = 27;

/// How much EX one note can be worth, so a chart's ceiling is `total_notes * NOTE_MAX_EX`.
pub const NOTE_MAX_EX: u32 = 2;

/// Lowest band the "next rank" walk starts looking from: anything under it is so far off that the
/// rank above is not a target worth showing (`TargetProperty.java:309`).
pub const NEXT_RANK_FIRST_BAND: u32 = 15;

/// Highest band the walk considers before it gives up and asks for everything.
pub const NEXT_RANK_LAST_BAND: u32 = SCORE_BANDS - 1;

/// A whole percentage, for the fixed targets that are stated as one.
const PERCENT: f64 = 100.0;

/// The fixed rate targets, in the order the TARGET row shows them: the stored id, the name on
/// screen, and the band of [`SCORE_BANDS`] it asks for (`TargetProperty.java:117-141`).
pub const RATE_TARGETS: [(&str, &str, u32); 11] = [
    ("RATE_A-", "RANK A-", 17),
    ("RATE_A", "RANK A", 18),
    ("RATE_A+", "RANK A+", 19),
    ("RATE_AA-", "RANK AA-", 20),
    ("RATE_AA", "RANK AA", 21),
    ("RATE_AA+", "RANK AA+", 22),
    ("RATE_AAA-", "RANK AAA-", 23),
    ("RATE_AAA", "RANK AAA", 24),
    ("RATE_AAA+", "RANK AAA+", 25),
    ("RATE_MAX-", "RANK MAX-", 26),
    ("MAX", "MAX", SCORE_BANDS),
];

/// The highest EX a chart of `total_notes` notes can be played for.
pub fn max_ex(total_notes: u32) -> u32 {
    total_notes * NOTE_MAX_EX
}

/// The EX a rate of `rate` percent of a chart's ceiling asks for.
///
/// The reference rounds up (`TargetProperty.java:104-112` takes the ceiling of
/// `totalNotes * 2 * rate / 100`), so a target is never quietly one point easier than its name.
///
/// This is for a rate stated as a percentage in its own right — the `RATE_<float>` targets. A
/// target named after a band goes through [`band_target_ex`], which never leaves the integers.
pub fn rate_target_ex(total_notes: u32, rate: f64) -> u32 {
    let ceiling = f64::from(max_ex(total_notes));
    (ceiling * rate / PERCENT).ceil().max(0.0) as u32
}

/// The EX a band of [`SCORE_BANDS`] asks for.
///
/// The division is done once, on integers: routing a band through a percentage divides twice, and
/// the second division lands a hair above a whole number wherever the exact answer is one — a chart
/// of fifteen notes would be asked for 21 EX at RANK A where the band is exactly 20. The reference
/// walks the same bands the same way (`TargetProperty.java:319` takes `ceil(max * i / 27)` in one
/// step) and agrees with this for every note count and band.
pub fn band_target_ex(total_notes: u32, band: u32) -> u32 {
    (u64::from(max_ex(total_notes)) * u64::from(band)).div_ceil(u64::from(SCORE_BANDS)) as u32
}

/// The EX of the fixed target stored under `id`, or `None` for an id that is not a fixed one.
pub fn rate_target_by_id(id: &str, total_notes: u32) -> Option<u32> {
    RATE_TARGETS.iter().find(|(stored, _, _)| *stored == id).map(|(_, _, band)| band_target_ex(total_notes, *band))
}

/// The name the fixed target stored under `id` is shown as.
pub fn rate_target_name(id: &str) -> Option<&'static str> {
    RATE_TARGETS.iter().find(|(stored, _, _)| *stored == id).map(|(_, name, _)| *name)
}

/// The EX of the next band up from where the run stands, which is what the "next rank" target asks
/// for (`TargetProperty.java:301-331`).
///
/// The walk starts at [`NEXT_RANK_FIRST_BAND`] and stops at the first band the current score has
/// not reached; a score past every band asks for the chart's ceiling instead, so there is always a
/// target.
pub fn next_rank_ex(now_ex: u32, total_notes: u32) -> u32 {
    for band in NEXT_RANK_FIRST_BAND..=NEXT_RANK_LAST_BAND {
        let target = band_target_ex(total_notes, band);
        if now_ex < target {
            return target;
        }
    }
    max_ex(total_notes)
}

/// What a target can be measured against besides the chart's own ceiling.
///
/// The two scores that come from outside — the account's own best on the score server and a rival's
/// — are `None` until that answer has arrived, which is why every target has a fallback.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetContext {
    /// Playable notes on the chart, which is what sets its ceiling.
    pub total_notes: u32,
    /// The best EX stored locally for this chart, if it has been played before.
    pub local_best_ex: Option<u32>,
    /// The best EX the score server holds for this account on this chart.
    pub ir_best_ex: Option<u32>,
    /// A rival's best on this chart, and which rival it is.
    pub rival: Option<(String, u32)>,
}

/// A target settled on one chart: what to call it, and the EX it asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub name: String,
    pub ex: u32,
}

/// The EX `target` asks for on this chart, or `None` when the score it needs has not arrived.
///
/// The fixed rates and the next rank are arithmetic over the note count and always answer; the
/// three that name somebody's score answer only once that score is known.
pub fn target_ex(target: ScoreTarget, ctx: &TargetContext) -> Option<u32> {
    match target {
        ScoreTarget::RankNext => Some(next_rank_ex(ctx.local_best_ex.unwrap_or(0), ctx.total_notes)),
        ScoreTarget::LocalBest => ctx.local_best_ex,
        ScoreTarget::IrBest => ctx.ir_best_ex,
        ScoreTarget::Rival => ctx.rival.as_ref().map(|(_, ex)| *ex),
        fixed => fixed.rate_id().and_then(|id| rate_target_by_id(id, ctx.total_notes)),
    }
}

/// What `target` is called on screen. A rival target carries the rival's name, since which rival it
/// is is the whole of what it says.
pub fn target_name(target: ScoreTarget, ctx: &TargetContext) -> String {
    match (target, &ctx.rival) {
        (ScoreTarget::Rival, Some((who, _))) => format!("{} {who}", target.label()),
        _ => target.label().to_string(),
    }
}

/// Settle the TARGET row on this chart.
///
/// A target whose score has not arrived — no server, a rival who has not played the chart, a first
/// play with no local record — falls back quietly rather than leaving the screen with nothing to
/// pace against: first to the local best, and then to the chart's ceiling, which every chart has.
/// The name that comes back is the target actually used, so the screen never claims to be showing
/// one it could not resolve.
pub fn resolve(target: ScoreTarget, ctx: &TargetContext) -> ResolvedTarget {
    for candidate in [target, ScoreTarget::LocalBest, ScoreTarget::Max] {
        if let Some(ex) = target_ex(candidate, ctx) {
            return ResolvedTarget { name: target_name(candidate, ctx), ex };
        }
    }
    ResolvedTarget { name: target_name(ScoreTarget::Max, ctx), ex: max_ex(ctx.total_notes) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chart_is_worth_two_points_a_note() {
        assert_eq!(max_ex(0), 0);
        assert_eq!(max_ex(1), 2);
        assert_eq!(max_ex(812), 1624);
    }

    #[test]
    fn every_fixed_target_has_its_own_id_name_and_band() {
        let mut ids: Vec<&str> = RATE_TARGETS.iter().map(|(id, _, _)| *id).collect();
        let mut names: Vec<&str> = RATE_TARGETS.iter().map(|(_, name, _)| *name).collect();
        let count = RATE_TARGETS.len();
        ids.sort_unstable();
        ids.dedup();
        names.sort_unstable();
        names.dedup();
        assert_eq!(ids.len(), count, "two targets share an id");
        assert_eq!(names.len(), count, "two targets share a name");
        for (at, (_, _, band)) in RATE_TARGETS.iter().enumerate() {
            assert!(*band <= SCORE_BANDS, "a target cannot ask for more than the chart holds");
            if at > 0 {
                assert!(*band > RATE_TARGETS[at - 1].2, "the targets have to climb");
            }
        }
        assert_eq!(RATE_TARGETS.last().map(|(id, _, band)| (*id, *band)), Some(("MAX", SCORE_BANDS)));
    }

    /// The reference rounds a target up, so a target is never one point easier than its name.
    #[test]
    fn a_rate_target_rounds_up_rather_than_down() {
        assert_eq!(rate_target_ex(3, 50.0), 3, "half of six is exact");
        assert_eq!(rate_target_ex(3, 51.0), 4, "anything over it costs the next whole point");
        assert_eq!(rate_target_ex(1, 1.0), 1, "the smallest fraction of a point still costs one");
        assert_eq!(rate_target_ex(0, 100.0), 0, "a chart with no notes has nothing to ask for");
        assert_eq!(rate_target_ex(812, 100.0), max_ex(812));
    }

    /// The note counts a band target is checked on: odd and even, and — where a two-step division
    /// goes wrong — the ones whose ceiling divides the band axis exactly. Fifteen notes at band 18
    /// is 20 EX on the nose, and a target computed through a percentage asks for 21.
    const BAND_CHECK_NOTES: [u32; 13] = [1, 2, 3, 15, 27, 30, 81, 162, 189, 811, 812, 813, 900];

    /// What a band asks for, worked out from what a band *means* rather than from the formula under
    /// test: the smallest EX whose share of the chart's ceiling reaches `band` of [`SCORE_BANDS`].
    /// That is what the reference's `ceil(max * band / 27)` (`TargetProperty.java:319`) computes.
    fn expected_band_ex(total_notes: u32, band: u32) -> u32 {
        let ceiling = max_ex(total_notes);
        let wanted = u64::from(total_notes) * u64::from(NOTE_MAX_EX) * u64::from(band);
        (0..=ceiling).find(|ex| u64::from(*ex) * u64::from(SCORE_BANDS) >= wanted).unwrap_or(ceiling)
    }

    /// Every band of one chart, worked out once, for the walks that need the whole table.
    fn expected_bands(total_notes: u32) -> Vec<u32> {
        (0..=SCORE_BANDS).map(|band| expected_band_ex(total_notes, band)).collect()
    }

    /// The ceiling is the same whether the note count is odd or even, which is where a rounding
    /// mistake would show up first.
    #[test]
    fn a_band_target_is_exact_on_both_odd_and_even_note_counts() {
        for total in BAND_CHECK_NOTES {
            let expected = expected_bands(total);
            for band in 1..=SCORE_BANDS {
                assert_eq!(band_target_ex(total, band), expected[band as usize], "{total} notes at band {band}");
            }
            assert_eq!(band_target_ex(total, SCORE_BANDS), max_ex(total), "the top band is the whole chart");
            assert_eq!(band_target_ex(total, 0), 0);
        }
    }

    /// A band whose EX lands on a whole number asks for that number and not the one above it. A
    /// target one point dearer than its name is what a player grinding RANK A sees as a target they
    /// cannot reach on a run that did reach it.
    #[test]
    fn a_band_that_divides_the_chart_exactly_asks_for_exactly_that() {
        for (total, band, exact) in [(15u32, 18u32, 20u32), (27, 18, 36), (30, 18, 40), (813, 18, 1084), (900, 18, 1200), (81, 9, 54)] {
            assert_eq!(band_target_ex(total, band), exact, "{total} notes at band {band}");
        }
        for total in 1..=600u32 {
            for band in 1..=SCORE_BANDS {
                let asked = band_target_ex(total, band);
                assert!(u64::from(asked) * u64::from(SCORE_BANDS) >= u64::from(max_ex(total)) * u64::from(band), "{total}/{band} asks for too little");
                assert!(
                    u64::from(asked.saturating_sub(1)) * u64::from(SCORE_BANDS) < u64::from(max_ex(total)) * u64::from(band) || asked == 0,
                    "{total}/{band} asks for more than the band"
                );
            }
        }
    }

    #[test]
    fn a_fixed_target_is_found_by_the_id_it_is_stored_under() {
        assert_eq!(rate_target_by_id("MAX", 812), Some(1624));
        assert_eq!(rate_target_by_id("RATE_AAA", 812), Some(expected_band_ex(812, 24)));
        assert_eq!(rate_target_name("RATE_AAA"), Some("RANK AAA"));
        assert_eq!(rate_target_by_id("RATE_NOT_A_TARGET", 812), None);
        assert_eq!(rate_target_name(""), None);
    }

    #[test]
    fn the_next_rank_is_the_first_band_the_run_has_not_reached() {
        let total = 813;
        let bands = expected_bands(total);
        assert_eq!(next_rank_ex(0, total), bands[NEXT_RANK_FIRST_BAND as usize], "a run under every band aims at the lowest one");
        for band in NEXT_RANK_FIRST_BAND..NEXT_RANK_LAST_BAND {
            let at_band = bands[band as usize];
            assert_eq!(next_rank_ex(at_band, total), bands[band as usize + 1], "standing exactly on band {band} aims at the next");
            assert_eq!(next_rank_ex(at_band - 1, total), at_band, "one point short of band {band} still aims at it");
        }
    }

    /// Whatever the chart, the next rank is the lowest band the run has not reached — worked out
    /// here from the reference's own arithmetic rather than from the walk being tested.
    #[test]
    fn the_next_rank_is_the_lowest_band_above_the_run_on_every_chart() {
        for total in BAND_CHECK_NOTES {
            let ceiling = max_ex(total);
            let bands = expected_bands(total);
            let walked = &bands[NEXT_RANK_FIRST_BAND as usize..=NEXT_RANK_LAST_BAND as usize];
            for now in 0..=ceiling {
                let expected = walked.iter().copied().find(|ex| now < *ex).unwrap_or(ceiling);
                assert_eq!(next_rank_ex(now, total), expected, "{total} notes, standing at {now}");
            }
        }
    }

    /// A run past the last band still needs something to be paced against, or the screen would have
    /// nothing to show for the best players.
    #[test]
    fn a_run_past_every_band_aims_at_the_whole_chart() {
        let total = 812;
        assert_eq!(next_rank_ex(band_target_ex(total, NEXT_RANK_LAST_BAND), total), max_ex(total));
        assert_eq!(next_rank_ex(max_ex(total), total), max_ex(total));
        assert_eq!(next_rank_ex(u32::MAX, total), max_ex(total));
    }

    #[test]
    fn a_chart_with_no_notes_has_a_target_of_nothing_rather_than_a_panic() {
        assert_eq!(next_rank_ex(0, 0), 0);
        assert_eq!(band_target_ex(0, SCORE_BANDS), 0);
    }

    /// The chart the resolution tests are run on, and the local record standing on it.
    const RESOLVE_NOTES: u32 = 812;

    fn played_context() -> TargetContext {
        TargetContext { total_notes: RESOLVE_NOTES, local_best_ex: Some(1400), ir_best_ex: Some(1500), rival: Some(("friend".into(), 1550)) }
    }

    #[test]
    fn the_fixed_rate_targets_ask_for_their_own_band() {
        let ctx = played_context();
        assert_eq!(target_ex(ScoreTarget::Max, &ctx), Some(max_ex(RESOLVE_NOTES)));
        assert_eq!(target_ex(ScoreTarget::RateA, &ctx), Some(expected_band_ex(RESOLVE_NOTES, 18)));
        assert_eq!(target_ex(ScoreTarget::RateAa, &ctx), Some(expected_band_ex(RESOLVE_NOTES, 21)));
        assert_eq!(target_ex(ScoreTarget::RateAaa, &ctx), Some(expected_band_ex(RESOLVE_NOTES, 24)));
    }

    /// Every rate the TARGET row offers has to be one this table knows, or the row would show a
    /// name the pacer cannot put a number to.
    #[test]
    fn every_rate_the_row_offers_is_a_rate_this_table_holds() {
        let ctx = played_context();
        let mut found = 0;
        for asked in ScoreTarget::ALL {
            let Some(id) = asked.rate_id() else {
                continue;
            };
            found += 1;
            let band = RATE_TARGETS.iter().find(|(stored, _, _)| *stored == id).map(|(_, _, band)| *band);
            let band = band.unwrap_or_else(|| panic!("{asked:?} names a rate this table does not hold: {id}"));
            assert_eq!(target_ex(asked, &ctx), Some(expected_band_ex(RESOLVE_NOTES, band)), "{asked:?}");
        }
        assert_eq!(found, RATE_TARGETS.len(), "the row does not offer every rate the reference names");
    }

    /// The next rank is measured from the record already standing on the chart, not from the run
    /// that has just ended, so it names the same thing before and after a play.
    #[test]
    fn the_next_rank_target_is_measured_from_the_local_record() {
        let ctx = played_context();
        assert_eq!(target_ex(ScoreTarget::RankNext, &ctx), Some(next_rank_ex(1400, RESOLVE_NOTES)));
        let unplayed = TargetContext { local_best_ex: None, ..ctx };
        assert_eq!(target_ex(ScoreTarget::RankNext, &unplayed), Some(next_rank_ex(0, RESOLVE_NOTES)), "a chart with no record is walked from nothing");
    }

    #[test]
    fn the_targets_that_name_a_score_answer_only_once_that_score_is_known() {
        let ctx = played_context();
        assert_eq!(target_ex(ScoreTarget::LocalBest, &ctx), Some(1400));
        assert_eq!(target_ex(ScoreTarget::IrBest, &ctx), Some(1500));
        assert_eq!(target_ex(ScoreTarget::Rival, &ctx), Some(1550));
        let alone = TargetContext { total_notes: RESOLVE_NOTES, ..Default::default() };
        assert_eq!(target_ex(ScoreTarget::LocalBest, &alone), None);
        assert_eq!(target_ex(ScoreTarget::IrBest, &alone), None);
        assert_eq!(target_ex(ScoreTarget::Rival, &alone), None);
    }

    #[test]
    fn a_resolved_target_is_named_for_the_row_that_asked_for_it() {
        let ctx = played_context();
        assert_eq!(resolve(ScoreTarget::RateAaa, &ctx).name, ScoreTarget::RateAaa.label());
        assert_eq!(resolve(ScoreTarget::Rival, &ctx).name, format!("{} friend", ScoreTarget::Rival.label()), "a rival target says which rival");
    }

    /// With no score server the account and rival targets have nothing to ask for; they must fall
    /// back to the local record rather than leave the screen without a target — and say so.
    #[test]
    fn a_target_with_no_answer_falls_back_to_the_local_record() {
        let offline = TargetContext { total_notes: RESOLVE_NOTES, local_best_ex: Some(1400), ..Default::default() };
        for asked in [ScoreTarget::IrBest, ScoreTarget::Rival] {
            let settled = resolve(asked, &offline);
            assert_eq!(settled.ex, 1400, "{asked:?} did not fall back to the local record");
            assert_eq!(settled.name, ScoreTarget::LocalBest.label(), "{asked:?} still claims to be showing what it could not resolve");
        }
    }

    /// A first play on an offline client has neither a server answer nor a record; the chart's own
    /// ceiling is the one target that always exists.
    #[test]
    fn a_target_with_no_answer_and_no_record_falls_back_to_the_whole_chart() {
        let bare = TargetContext { total_notes: RESOLVE_NOTES, ..Default::default() };
        for asked in [ScoreTarget::IrBest, ScoreTarget::Rival, ScoreTarget::LocalBest] {
            let settled = resolve(asked, &bare);
            assert_eq!(settled.ex, max_ex(RESOLVE_NOTES), "{asked:?} did not fall back to the ceiling");
            assert_eq!(settled.name, ScoreTarget::Max.label());
        }
    }

    /// The end of the fallback chain has to answer for every target, or the chain would not end.
    #[test]
    fn the_last_fallback_answers_on_every_chart() {
        for total in [0u32, 1, 812] {
            let ctx = TargetContext { total_notes: total, ..Default::default() };
            assert_eq!(target_ex(ScoreTarget::Max, &ctx), Some(max_ex(total)));
            assert_eq!(resolve(ScoreTarget::Max, &ctx).ex, max_ex(total));
        }
    }

    /// Every target the TARGET row offers settles into something, whatever has and has not arrived.
    #[test]
    fn every_target_the_row_offers_settles_on_every_chart() {
        for ctx in [played_context(), TargetContext { total_notes: RESOLVE_NOTES, ..Default::default() }, TargetContext::default()] {
            for asked in ScoreTarget::ALL {
                let settled = resolve(asked, &ctx);
                assert!(settled.ex <= max_ex(ctx.total_notes.max(settled.ex)), "{asked:?} asked for more than a chart can hold");
                assert!(!settled.name.is_empty(), "{asked:?} settled without a name");
            }
        }
    }
}

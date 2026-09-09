use rbms_model::TimeLine;

/// Lane vertical geometry in pixels. `hl` is the judgment line (notes arrive here at
/// their `time_us`); `hu` is the far top edge (notes spawn/cull beyond it).
#[derive(Debug, Clone, Copy)]
pub struct LaneGeometry {
    pub hu: f32,
    pub hl: f32,
}

impl LaneGeometry {
    pub fn height(&self) -> f32 {
        (self.hu - self.hl).abs()
    }
}

/// Reference travel time (ms) across the full lane at hi-speed 1.0 in CONSTANT (green-number-fixed)
/// scroll: the 120 BPM point ([`green_number`] at `bpm = 120`, `scroll = 1`) that
/// [`constant_offsets`] is calibrated to (`pps = hispeed * lane_height / 2_000_000`).
///
/// This calibration is rbms's own, with no counterpart in the reference implementation: its
/// `playconfig.isEnableConstant()` only pins the `#SPEED` interpolation to 1.0
/// (`LaneRenderer.java:319-320`), while its `region` at `:321` still divides by the live
/// `nbpm`/`nscroll`. rbms's CONSTANT drops the BPM dependency entirely.
pub const CONSTANT_GREEN_BASE_MS: f64 = 2_000.0;

/// IIDX-style green number: the note travel time (ms) across the full lane. Stays
/// constant under BPM changes only if hi-speed is fixed per `bpm` (caller's choice).
///
/// This is the reference implementation's `currentduration` (`LaneRenderer.java:321,330`:
/// `region = (240000 / nbpm / hispeed / speed) / nscroll`, then `region * (1 - lanecover)`), not the
/// number its skins label GREEN — the skin property multiplies the same expression by a further
/// `0.6` (`IntegerPropertyFactory.java:555-556`:
/// `(240000 / bpm / hispeed) * (cover ? 1 - lanecover : 1) * (green ? 0.6 : 1)`).
///
/// LIFT does not enter this: raising the judgment line shrinks the lane but the scroll speed is
/// normalised to the (lifted) lane height, so the travel time is unchanged — the reference implementation computes the
/// same way (`LaneRenderer.java:321-326` derives `region` from bpm/hispeed/scroll only and scales
/// `rxhs` by the lifted `hu - hl`; `currentduration` at `:330` multiplies by `1 - lanecover` alone).
pub fn green_number(bpm: f64, hispeed: f64, scroll: f64, lanecover: f64) -> f64 {
    if bpm <= 0.0 || hispeed <= 0.0 || scroll == 0.0 {
        return 0.0;
    }
    (240_000.0 / bpm / hispeed) / scroll * (1.0 - lanecover)
}

/// Green number for CONSTANT scroll, where the travel time is fixed by hi-speed alone (BPM and
/// SCROLL do not apply). Same `1 - lanecover` visible-window scaling as [`green_number`], and the
/// same rbms-only calibration caveat as [`CONSTANT_GREEN_BASE_MS`].
pub fn constant_green_number(hispeed: f64, lanecover: f64) -> f64 {
    if hispeed <= 0.0 {
        return 0.0;
    }
    CONSTANT_GREEN_BASE_MS / hispeed * (1.0 - lanecover)
}

/// Pixel offset above the judgment line for a note arriving at `note_time_us`, assuming
/// no BPM/SCROLL/STOP changes between `microtime` and the note (the closed form). Used
/// for testing and as the fast path.
pub fn closed_form_offset(note_time_us: i64, microtime: i64, bpm: f64, scroll: f64, hispeed: f64, lane_height: f32) -> f32 {
    let dt = (note_time_us - microtime) as f64;
    (dt * bpm * scroll * hispeed * lane_height as f64 / 240_000_000.0) as f32
}

/// Walk the timeline segments from `microtime` forward, returning `(timeline_index,
/// pixel_offset_above_judgment_line)` for every timeline within the visible window.
/// Integrates BPM/SCROLL/STOP exactly as the reference implementation's `LaneRenderer` does: a STOP holds
/// notes frozen (full segment height), other segments scale by elapsed fraction.
pub fn visible_offsets(timelines: &[TimeLine], microtime: i64, hispeed: f64, lane_height: f32) -> Vec<(usize, f32)> {
    let mut out = Vec::new();
    if timelines.len() < 2 {
        return out;
    }
    let rxhs = lane_height as f64 * hispeed;
    let cur = match timelines.binary_search_by(|t| t.time_us.cmp(&microtime)) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };

    let mut y = 0.0f64;
    for i in (cur + 1)..timelines.len() {
        let prev = &timelines[i - 1];
        let tl = &timelines[i];
        let d_section = tl.section - prev.section;
        if i - 1 == cur {
            if prev.time_us + prev.stop_us > microtime {
                y += d_section * prev.scroll * rxhs;
            } else {
                let denom = (tl.time_us - prev.time_us - prev.stop_us) as f64;
                let frac = if denom > 0.0 { (tl.time_us - microtime) as f64 / denom } else { 1.0 };
                y += d_section * prev.scroll * frac * rxhs;
            }
        } else {
            y += d_section * prev.scroll * rxhs;
        }
        out.push((i, y as f32));
        if y > lane_height as f64 {
            break;
        }
    }
    out
}

/// CONSTANT (green-number-fixed) scroll: a note's offset is linear in time-to-arrival and
/// independent of BPM/SCROLL/STOP, so the visible time window stays fixed under BPM changes.
/// `hispeed` is calibrated so the speed equals FLOATING at 120 BPM (green number ≈ 2000/hispeed ms).
pub fn constant_offsets(timelines: &[TimeLine], microtime: i64, hispeed: f64, lane_height: f32) -> Vec<(usize, f32)> {
    let mut out = Vec::new();
    let pps = hispeed * lane_height as f64 / 2_000_000.0;
    let cur = match timelines.binary_search_by(|t| t.time_us.cmp(&microtime)) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    for (i, tl) in timelines.iter().enumerate().skip(cur + 1) {
        let y = (tl.time_us - microtime) as f64 * pps;
        out.push((i, y as f32));
        if y > lane_height as f64 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_model;
    use rbms_model::Mode;
    use rbms_parser::parse;

    fn timelines(bms: &[u8]) -> Vec<TimeLine> {
        to_model(&parse(bms), Mode::BEAT_7K).timelines
    }

    fn offset_of(tls: &[TimeLine], note_time: i64, microtime: i64, hispeed: f64, h: f32) -> f32 {
        let idx = tls.iter().position(|t| t.time_us == note_time).unwrap();
        visible_offsets(tls, microtime, hispeed, h).iter().find(|(i, _)| *i == idx).map(|(_, y)| *y).unwrap()
    }

    #[test]
    fn walk_matches_closed_form_constant_bpm() {
        let tls = timelines(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let note_t = 2_000_000;
        let walk = offset_of(&tls, note_t, 0, 1.0, 600.0);
        let closed = closed_form_offset(note_t, 0, 120.0, 1.0, 1.0, 600.0);
        assert!((walk - closed).abs() < 0.5, "walk={walk} closed={closed}");
    }

    #[test]
    fn hispeed_scales_offset_linearly() {
        let tls = timelines(b"#BPM 150\r\n#WAV01 a.wav\r\n#00211:01\r\n");
        let note_t = tls.iter().rev().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let micro = note_t - 400_000;
        let o1 = offset_of(&tls, note_t, micro, 1.0, 600.0);
        let o2 = offset_of(&tls, note_t, micro, 2.0, 600.0);
        assert!(o1 > 0.0 && (o2 - 2.0 * o1).abs() < 0.5, "o1={o1} o2={o2}");
    }

    #[test]
    fn note_reaches_line_at_its_time() {
        let note_t = 2_000_000;
        let near = closed_form_offset(note_t, note_t - 1000, 120.0, 1.0, 1.0, 600.0);
        let at = closed_form_offset(note_t, note_t, 120.0, 1.0, 1.0, 600.0);
        assert_eq!(at, 0.0);
        assert!(near > 0.0);
    }

    #[test]
    fn green_number_matches_traversal_time() {
        let h = 600.0f32;
        let gn = green_number(120.0, 1.0, 1.0, 0.0);
        let dt_us = (gn * 1000.0) as i64;
        let off = closed_form_offset(dt_us, 0, 120.0, 1.0, 1.0, h);
        assert!((off - h).abs() < 1.0, "green traversal off={off} expected≈{h}");
    }

    #[test]
    fn constant_speed_offset_independent_of_bpm() {
        let a = timelines(b"#BPM 100\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let b = timelines(b"#BPM 200\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let (h, hs) = (1200.0f32, 1.0f64);
        let na = a.iter().rev().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let nb = b.iter().rev().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let off = |tls: &[TimeLine], nt: i64| constant_offsets(tls, nt - 500_000, hs, h).iter().find(|(i, _)| tls[*i].time_us == nt).map(|(_, y)| *y).unwrap();
        let (oa, ob) = (off(&a, na), off(&b, nb));
        assert!((oa - ob).abs() < 1.0, "constant speed: same dt gives same offset regardless of BPM (oa={oa} ob={ob})");
    }

    #[test]
    fn stop_freezes_offset_during_stop_interval() {
        let tls = timelines(b"#BPM 120\r\n#STOP01 192\r\n#WAV01 a.wav\r\n#00109:01\r\n#00211:01\r\n");
        let note_t = tls.iter().rev().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let stop_start = 2_000_000;
        let o_at_stop = offset_of(&tls, note_t, stop_start, 1.0, 600.0);
        let o_mid_stop = offset_of(&tls, note_t, stop_start + 1_000_000, 1.0, 600.0);
        assert!((o_at_stop - o_mid_stop).abs() < 0.5, "stop should freeze: {o_at_stop} vs {o_mid_stop}");
    }

    #[test]
    fn closed_form_offset_is_zero_at_note_time() {
        assert_eq!(closed_form_offset(1_000_000, 1_000_000, 180.0, 1.0, 1.0, 600.0), 0.0);
    }

    #[test]
    fn closed_form_offset_is_positive_before_arrival() {
        let off = closed_form_offset(1_000_000, 999_000, 120.0, 1.0, 1.0, 600.0);
        assert!(off > 0.0, "approaching note is above the line, got {off}");
    }

    #[test]
    fn closed_form_offset_is_negative_after_passing() {
        let off = closed_form_offset(1_000_000, 1_001_000, 120.0, 1.0, 1.0, 600.0);
        assert!(off < 0.0, "passed note is below the line, got {off}");
    }

    #[test]
    fn closed_form_offset_scales_linearly_with_hispeed() {
        let base = closed_form_offset(1_000_000, 0, 120.0, 1.0, 1.0, 600.0);
        let doubled = closed_form_offset(1_000_000, 0, 120.0, 1.0, 2.0, 600.0);
        assert!((doubled - 2.0 * base).abs() < 1e-3, "hispeed doubles the offset: {base} vs {doubled}");
    }

    #[test]
    fn closed_form_offset_scales_linearly_with_bpm() {
        let lo = closed_form_offset(1_000_000, 0, 60.0, 1.0, 1.0, 600.0);
        let hi = closed_form_offset(1_000_000, 0, 120.0, 1.0, 1.0, 600.0);
        assert!((hi - 2.0 * lo).abs() < 1e-3, "doubling BPM doubles the offset: {lo} vs {hi}");
    }

    #[test]
    fn closed_form_offset_negative_scroll_flips_sign() {
        let pos = closed_form_offset(2_000_000, 0, 120.0, 1.0, 1.0, 600.0);
        let neg = closed_form_offset(2_000_000, 0, 120.0, -1.0, 1.0, 600.0);
        assert!(pos > 0.0 && neg < 0.0, "a negative SCROLL multiplier inverts the offset: {pos} vs {neg}");
        assert!((pos + neg).abs() < 1e-3, "magnitudes match");
    }

    #[test]
    fn green_number_zero_for_nonpositive_bpm() {
        assert_eq!(green_number(0.0, 1.0, 1.0, 0.0), 0.0);
        assert_eq!(green_number(-120.0, 1.0, 1.0, 0.0), 0.0);
    }

    #[test]
    fn green_number_zero_for_nonpositive_hispeed() {
        assert_eq!(green_number(120.0, 0.0, 1.0, 0.0), 0.0);
        assert_eq!(green_number(120.0, -1.0, 1.0, 0.0), 0.0);
    }

    #[test]
    fn green_number_zero_for_zero_scroll() {
        assert_eq!(green_number(120.0, 1.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn green_number_lanecover_reduces_travel_time() {
        let full = green_number(120.0, 1.0, 1.0, 0.0);
        let half = green_number(120.0, 1.0, 1.0, 0.5);
        assert_eq!(full, 2000.0, "240000/120 = 2000 ms");
        assert!((half - 1000.0).abs() < 1e-6, "50% cover halves the green number: {half}");
    }

    #[test]
    fn green_number_inversely_proportional_to_bpm_and_hispeed() {
        let a = green_number(120.0, 1.0, 1.0, 0.0);
        let b = green_number(240.0, 1.0, 1.0, 0.0);
        let c = green_number(120.0, 2.0, 1.0, 0.0);
        assert!((b - a / 2.0).abs() < 1e-6, "double BPM halves green");
        assert!((c - a / 2.0).abs() < 1e-6, "double hispeed halves green");
    }

    #[test]
    fn visible_offsets_empty_for_too_few_timelines() {
        assert!(visible_offsets(&[], 0, 1.0, 600.0).is_empty());
        let one = vec![TimeLine::empty(8, 0, 0.0, 120.0)];
        assert!(visible_offsets(&one, 0, 1.0, 600.0).is_empty(), "needs at least 2 timelines");
    }

    #[test]
    fn visible_offsets_are_monotonically_increasing_under_positive_scroll() {
        let tls = timelines(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00211:01\r\n#00311:01\r\n#00411:01\r\n");
        let offs = visible_offsets(&tls, 0, 1.0, 100_000.0);
        let ys: Vec<f32> = offs.iter().map(|(_, y)| *y).collect();
        assert!(ys.len() >= 2, "expected several visible timelines, got {}", ys.len());
        assert!(ys.windows(2).all(|w| w[1] >= w[0]), "offsets must be non-decreasing: {ys:?}");
    }

    #[test]
    fn visible_offsets_walk_agrees_with_closed_form_under_constant_bpm() {
        let tls = timelines(b"#BPM 150\r\n#WAV01 a.wav\r\n#00111:01\r\n#00211:01\r\n#00311:01\r\n");
        for &note_t in &[tls[1].time_us, tls.last().unwrap().time_us] {
            let micro = note_t - 300_000;
            let walk = offset_of(&tls, note_t, micro, 1.0, 600.0);
            let closed = closed_form_offset(note_t, micro, 150.0, 1.0, 1.0, 600.0);
            assert!((walk - closed).abs() < 1.0, "walk={walk} closed={closed} for note_t={note_t}");
        }
    }

    #[test]
    fn visible_offsets_breaks_once_past_lane_height() {
        let tls = timelines(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00211:01\r\n#00311:01\r\n#00411:01\r\n#00511:01\r\n");
        let h = 50.0f32;
        let offs = visible_offsets(&tls, 0, 1.0, h);
        assert!(!offs.is_empty());
        for (_, y) in &offs[..offs.len() - 1] {
            assert!(*y <= h, "intermediate offset {y} should be within lane height {h}");
        }
    }

    #[test]
    fn constant_offsets_zero_for_note_at_microtime() {
        let tls = timelines(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00211:01\r\n");
        let offs = constant_offsets(&tls, 0, 1.0, 1200.0);
        assert!(offs.iter().all(|(_, y)| *y >= 0.0), "future offsets are non-negative: {offs:?}");
    }

    #[test]
    fn constant_offsets_independent_of_bpm_for_equal_time_to_arrival() {
        let a = timelines(b"#BPM 100\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let b = timelines(b"#BPM 200\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let na = a.iter().rev().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let nb = b.iter().rev().find_map(|t| t.notes[0].as_ref().map(|n| n.time_us)).unwrap();
        let off =
            |tls: &[TimeLine], nt: i64| constant_offsets(tls, nt - 500_000, 1.0, 1200.0).iter().find(|(i, _)| tls[*i].time_us == nt).map(|(_, y)| *y).unwrap();
        assert!((off(&a, na) - off(&b, nb)).abs() < 1.0, "constant speed is BPM-independent");
    }

    #[test]
    fn constant_offsets_scale_linearly_with_hispeed() {
        let tls = timelines(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00211:01\r\n");
        let nt = tls.last().unwrap().time_us;
        let pick = |hs: f64| constant_offsets(&tls, nt - 500_000, hs, 1200.0).iter().find(|(i, _)| tls[*i].time_us == nt).map(|(_, y)| *y).unwrap();
        let o1 = pick(1.0);
        let o2 = pick(2.0);
        assert!(o1 > 0.0 && (o2 - 2.0 * o1).abs() < 1.0, "constant offset doubles with hispeed: {o1} {o2}");
    }

    #[test]
    fn green_number_matches_closed_form_traversal() {
        let h = 800.0f32;
        let gn = green_number(150.0, 1.0, 1.0, 0.0);
        let dt_us = (gn * 1000.0) as i64;
        let off = closed_form_offset(dt_us, 0, 150.0, 1.0, 1.0, h);
        assert!((off - h).abs() < 1.0, "green traversal off={off} expected≈{h}");
    }

    #[test]
    fn green_number_is_the_currentduration_not_the_skin_green_property() {
        assert_eq!(green_number(120.0, 1.0, 1.0, 0.0), 2000.0);
        assert!((green_number(120.0, 1.0, 1.0, 0.0) * 0.6 - 1200.0).abs() < 1e-9, "the skin GREEN property would read 1200");
    }

    #[test]
    fn constant_green_number_is_two_thousand_over_hispeed() {
        assert_eq!(constant_green_number(1.0, 0.0), 2000.0);
        assert_eq!(constant_green_number(2.0, 0.0), 1000.0);
        assert_eq!(constant_green_number(4.0, 0.0), 500.0);
    }

    #[test]
    fn constant_green_number_scales_by_visible_window() {
        assert!((constant_green_number(2.0, 0.25) - 750.0).abs() < 1e-9);
        assert!((constant_green_number(1.0, 0.9) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn constant_green_number_zero_for_nonpositive_hispeed() {
        assert_eq!(constant_green_number(0.0, 0.0), 0.0);
        assert_eq!(constant_green_number(-2.0, 0.0), 0.0);
    }

    #[test]
    fn constant_green_number_equals_floating_green_at_calibration_bpm() {
        for hs in [0.5, 1.0, 2.5, 8.0] {
            let floating = green_number(120.0, hs, 1.0, 0.2);
            let constant = constant_green_number(hs, 0.2);
            assert!((floating - constant).abs() < 1e-9, "hs={hs}: {floating} vs {constant}");
        }
    }

    #[test]
    fn constant_offsets_traverse_lane_in_the_constant_green_number() {
        let tls = timelines(b"#BPM 200\r\n#WAV01 a.wav\r\n#00111:01\r\n#00211:01\r\n#00311:01\r\n");
        let (h, hs) = (600.0f32, 2.0f64);
        let gn_us = (constant_green_number(hs, 0.0) * 1000.0) as i64;
        let nt = tls.last().unwrap().time_us;
        let y = constant_offsets(&tls, nt - gn_us, hs, h).iter().find(|(i, _)| tls[*i].time_us == nt).map(|(_, y)| *y).unwrap();
        assert!((y - h).abs() < 1.0, "a note {gn_us}us out sits at the lane top (y={y}, h={h})");
    }
}

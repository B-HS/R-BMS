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

/// IIDX-style green number: the note travel time (ms) across the full lane. Stays
/// constant under BPM changes only if hi-speed is fixed per `bpm` (caller's choice).
pub fn green_number(bpm: f64, hispeed: f64, scroll: f64, lanecover: f64) -> f64 {
    if bpm <= 0.0 || hispeed <= 0.0 || scroll == 0.0 {
        return 0.0;
    }
    (240_000.0 / bpm / hispeed) / scroll * (1.0 - lanecover)
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
/// Integrates BPM/SCROLL/STOP exactly as beatoraja's `LaneRenderer` does: a STOP holds
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
    for i in (cur + 1)..timelines.len() {
        let y = (timelines[i].time_us - microtime) as f64 * pps;
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
}

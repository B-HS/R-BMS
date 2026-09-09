use rbms_judge::{GaugeKind, JudgeEngine, JudgeProperty, JudgeResult, judgerank_for};
use rbms_model::{Model, NoteKind};

/// A keysound that should fire at `at_us` (autoplay BGM, or a hit note's sound).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayEvent {
    pub wav: i32,
    pub at_us: i64,
}

enum AutoAction {
    Press { wav: i32, ln: bool },
    Release,
}

/// Autoplay key-beam flash for a tapped (non-LN) note, mirroring the reference implementation's
/// `auto_minduration` (80 ms): a tap lights its lane beam for this long, then releases.
const AUTO_BEAM_US: i64 = 80_000;

fn collect_bg(model: &Model) -> Vec<(i64, i32)> {
    let mut v: Vec<(i64, i32)> = model.timelines.iter().flat_map(|tl| tl.bgnotes.iter().map(|n| (n.time_us, n.wav))).collect();
    v.sort_by_key(|e| e.0);
    v
}

/// `(head_time, lane, wav)` for every judgeable note head (Normal + LN start). Used for
/// interactive keysound lookup.
fn collect_note_heads(model: &Model) -> Vec<(i64, usize, i32)> {
    let mut v = Vec::new();
    for tl in &model.timelines {
        for (lane, slot) in tl.notes.iter().enumerate() {
            if let Some(note) = slot {
                match note.kind {
                    NoteKind::Normal | NoteKind::LongStart { .. } => v.push((note.time_us, lane, note.wav)),
                    _ => {}
                }
            }
        }
    }
    v.sort_by_key(|e| e.0);
    v
}

/// Autoplay judge actions, time-sorted: a press for every note head (LN heads too) and a
/// release for every LN end.
fn collect_actions(model: &Model) -> Vec<(i64, usize, AutoAction)> {
    let n = model.mode.key;
    let mut v = Vec::new();
    for lane in 0..n {
        let mut pending_head: Option<(i64, i32)> = None;
        for tl in &model.timelines {
            let Some(note) = &tl.notes[lane] else {
                continue;
            };
            match note.kind {
                NoteKind::Normal => v.push((note.time_us, lane, AutoAction::Press { wav: note.wav, ln: false })),
                NoteKind::LongStart { .. } => pending_head = Some((note.time_us, note.wav)),
                NoteKind::LongEnd { .. } => {
                    if let Some((head_us, wav)) = pending_head.take() {
                        v.push((head_us, lane, AutoAction::Press { wav, ln: true }));
                        v.push((note.time_us, lane, AutoAction::Release));
                    }
                }
                NoteKind::Mine { .. } => {}
            }
        }
    }
    v.sort_by_key(|e| e.0);
    v
}

/// Run a full autoplay pass and return the resulting judge engine. Verification helper
/// (autoplay = all PGREAT, LNs counted once).
pub fn simulate_autoplay(model: &Model) -> JudgeEngine {
    let mut p = Player::new(model.clone(), true);
    let end = p.last_time_us() + 1_000_000;
    p.update(end, |_| {});
    p.into_judge()
}

/// Real-time play driver. Advances over the song clock, emitting keysound events and (in
/// autoplay) feeding perfect judgments including LN releases.
pub struct Player {
    model: Model,
    pub judge: JudgeEngine,
    bg: Vec<(i64, i32)>,
    bg_cursor: usize,
    heads: Vec<(i64, usize, i32)>,
    actions: Vec<(i64, usize, AutoAction)>,
    action_cursor: usize,
    autoplay: bool,
    auto_lanes: Vec<bool>,
    beam_on: Vec<i64>,
    beam_off: Vec<i64>,
    ln_active: Vec<bool>,
    bomb: Vec<(i64, u8)>,
}

impl Player {
    pub fn new(model: Model, autoplay: bool) -> Self {
        let judge = JudgeEngine::from_model_for_mode(&model);
        let bg = collect_bg(&model);
        let heads = collect_note_heads(&model);
        let actions = collect_actions(&model);
        let lanes = model.mode.key;
        let auto_lanes = vec![false; lanes];
        Player {
            model,
            judge,
            bg,
            bg_cursor: 0,
            heads,
            actions,
            action_cursor: 0,
            autoplay,
            auto_lanes,
            beam_on: vec![i64::MIN; lanes],
            beam_off: vec![i64::MIN; lanes],
            ln_active: vec![false; lanes],
            bomb: vec![(i64::MIN, 0); lanes],
        }
    }

    /// Apply a single user JUDGE WIDTH rate (percent; 100 = chart default) to PGREAT/GREAT/GOOD on
    /// both key and scratch lanes. Kept for callers that expose one slider; see
    /// [`set_judge_window_rates`](Self::set_judge_window_rates) for the reference implementation's six-value form.
    pub fn set_judge_rate(&mut self, rate_percent: i32) {
        self.set_judge_window_rates([rate_percent; 3], [rate_percent; 3]);
    }

    /// Reference implementation JUDGE WIDTH: per-tier `[PGREAT, GREAT, GOOD]` percentages for key lanes and for
    /// scratch lanes (`JudgeManager.java:169-174`). The chart's own judgerank (`#RANK`/`#DEFEXRANK`)
    /// is applied first, then these rates — BAD and the 空POOR band never widen, and each tier is
    /// clamped to BAD and to the tier before it.
    pub fn set_judge_window_rates(&mut self, key: [i32; 3], scratch: [i32; 3]) {
        let judgerank = judgerank_for(self.model.meta.rank, self.model.meta.defexrank);
        let prop = JudgeProperty::for_mode(&self.model.mode);
        self.judge.set_windows(prop.note.scaled(judgerank).with_window_rate(key));
        self.judge.set_scratch_windows(prop.scratch.scaled(judgerank).with_window_rate(scratch));
        self.judge.set_ln_end(prop.ln_end.scaled(judgerank).with_window_rate(key));
        self.judge.set_ln_scratch_end(prop.ln_scratch_end.scaled(judgerank).with_window_rate(scratch));
    }

    /// Reference implementation LONGNOTE MARGIN rate (percent of the mode's stock margin).
    pub fn set_longnote_margin_rate(&mut self, rate_percent: i32) {
        self.judge.set_longnote_margin_rate(rate_percent);
    }

    /// Per-lane key-beam press timestamps (µs); `i64::MIN` means the beam is not held. Paired
    /// with [`beam_off`](Self::beam_off) so the renderer can show a hold then a release fade.
    pub fn beam_on(&self) -> &[i64] {
        &self.beam_on
    }

    /// Per-lane key-beam release timestamps (µs); `i64::MIN` means no release fade is pending.
    pub fn beam_off(&self) -> &[i64] {
        &self.beam_off
    }

    /// Per-lane key-bomb state `(hit_us, judge_index)` — set on a note hit (PG/GR/GD/BD, not empty
    /// POOR or miss); `i64::MIN` means no bomb. The renderer fades it over the skin's bomb window.
    pub fn bomb(&self) -> &[(i64, u8)] {
        &self.bomb
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Mark lanes (e.g. an auto-scratch lane) as auto-played: their notes are hit from
    /// the chart even in interactive mode, and key input on them is ignored.
    pub fn set_auto_lanes(&mut self, auto: Vec<bool>) {
        if auto.len() == self.auto_lanes.len() {
            self.auto_lanes = auto;
        }
    }

    pub fn set_gauge(&mut self, kind: GaugeKind) {
        self.judge.set_gauge(kind, self.model.meta.total);
    }

    pub fn into_judge(self) -> JudgeEngine {
        self.judge
    }

    pub fn update<F: FnMut(PlayEvent)>(&mut self, now_us: i64, mut play: F) {
        while self.bg_cursor < self.bg.len() && self.bg[self.bg_cursor].0 <= now_us {
            let (at, wav) = self.bg[self.bg_cursor];
            play(PlayEvent { wav, at_us: at });
            self.bg_cursor += 1;
        }
        while self.action_cursor < self.actions.len() && self.actions[self.action_cursor].0 <= now_us {
            let (at, lane, ref kind) = self.actions[self.action_cursor];
            if self.autoplay || self.auto_lanes.get(lane).copied().unwrap_or(false) {
                match kind {
                    AutoAction::Press { wav, ln } => {
                        play(PlayEvent { wav: *wav, at_us: at });
                        if let Some(jr) = self.judge.press(lane, at) {
                            if (jr.judge as usize) <= 3 {
                                self.bomb[lane] = (at, jr.judge as u8);
                            }
                        }
                        self.beam_on[lane] = at;
                        self.beam_off[lane] = i64::MIN;
                        if *ln {
                            self.ln_active[lane] = true;
                        }
                    }
                    AutoAction::Release => {
                        if let Some(jr) = self.judge.release(lane, at) {
                            if (jr.judge as usize) <= 3 {
                                self.bomb[lane] = (at, jr.judge as u8);
                            }
                        }
                        self.beam_off[lane] = at;
                        self.beam_on[lane] = i64::MIN;
                        self.ln_active[lane] = false;
                    }
                }
            }
            self.action_cursor += 1;
        }
        for lane in 0..self.beam_on.len() {
            let auto = self.autoplay || self.auto_lanes[lane];
            if auto && self.beam_on[lane] != i64::MIN && !self.ln_active[lane] && now_us - self.beam_on[lane] >= AUTO_BEAM_US {
                self.beam_off[lane] = self.beam_on[lane] + AUTO_BEAM_US;
                self.beam_on[lane] = i64::MIN;
            }
        }
        self.judge.update(now_us);
    }

    pub fn press<F: FnMut(PlayEvent)>(&mut self, lane: usize, now_us: i64, mut play: F) -> Option<JudgeResult> {
        if self.auto_lanes.get(lane).copied().unwrap_or(false) {
            return None;
        }
        if lane < self.beam_on.len() {
            self.beam_on[lane] = now_us;
            self.beam_off[lane] = i64::MIN;
        }
        if let Some(wav) = self.nearest_head_wav(lane, now_us) {
            play(PlayEvent { wav, at_us: now_us });
        }
        let res = self.judge.press(lane, now_us);
        if let Some(jr) = &res {
            if (jr.judge as usize) <= 3 && jr.lane < self.bomb.len() {
                self.bomb[jr.lane] = (now_us, jr.judge as u8);
            }
        }
        res
    }

    pub fn release(&mut self, lane: usize, now_us: i64) -> Option<JudgeResult> {
        if lane < self.beam_on.len() && !self.auto_lanes[lane] {
            if self.beam_on[lane] != i64::MIN {
                self.beam_off[lane] = now_us;
            }
            self.beam_on[lane] = i64::MIN;
            self.ln_active[lane] = false;
        }
        let res = self.judge.release(lane, now_us);
        if let Some(jr) = &res {
            if (jr.judge as usize) <= 3 && jr.lane < self.bomb.len() {
                self.bomb[jr.lane] = (now_us, jr.judge as u8);
            }
        }
        res
    }

    fn nearest_head_wav(&self, lane: usize, now_us: i64) -> Option<i32> {
        self.heads.iter().filter(|(_, l, _)| *l == lane).min_by_key(|(t, _, _)| (t - now_us).abs()).map(|(_, _, wav)| *wav)
    }

    pub fn last_time_us(&self) -> i64 {
        self.model.timelines.last().map(|t| t.time_us).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_chart::to_model;
    use rbms_model::Mode;
    use rbms_parser::parse;

    fn model(bms: &[u8]) -> Model {
        to_model(&parse(bms), Mode::BEAT_7K)
    }

    #[test]
    fn autoplay_achieves_perfect_score() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00211:01010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "all PGREAT");
        assert_eq!(engine.ex_score, 2 * n as u32);
        assert_eq!(engine.max_combo, n as u32);
        assert_eq!(engine.counts[5], 0, "no miss");
    }

    #[test]
    fn autoplay_handles_ln_as_single_judge() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        assert_eq!(n, 1, "one LN = one playable note");
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], 1, "LN judged once as PGREAT");
        assert_eq!(engine.counts.iter().sum::<u32>(), 1);
        assert_eq!(engine.max_combo, 1);
    }

    #[test]
    fn player_autoplay_emits_keysounds_and_judges() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00101:01\r\n#00111:0101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let mut p = Player::new(m, true);
        let mut events = Vec::new();
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |e| events.push(e));
        assert!(events.len() >= n, "emitted bg + note keysounds");
        assert_eq!(p.judge.counts[0], n as u32);
    }

    #[test]
    fn auto_lane_judged_without_input() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        let last = p.last_time_us();
        p.update(last + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[0], 1, "scratch lane auto-judged as PGREAT");
        assert!(p.press(7, last, |_| {}).is_none(), "input on auto lane is ignored");
    }

    #[test]
    fn interactive_press_sets_beam_release_clears_it() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        assert_eq!(p.beam_on()[0], i64::MIN, "beam off initially");
        p.press(0, 1_000_000, |_| {});
        assert_eq!(p.beam_on()[0], 1_000_000, "beam lit on press");
        assert_eq!(p.beam_off()[0], i64::MIN);
        p.release(0, 1_050_000);
        assert_eq!(p.beam_on()[0], i64::MIN, "beam cleared on release");
        assert_eq!(p.beam_off()[0], 1_050_000, "release fade timestamp set");
    }

    #[test]
    fn note_hit_lights_key_bomb() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        assert_eq!(p.bomb()[0].0, i64::MIN, "no bomb before any hit");
        let res = p.press(0, 2_000_000, |_| {});
        assert!(res.is_some(), "clean press judges the note");
        assert_eq!(p.bomb()[0].0, 2_000_000, "bomb lit at the hit time");
        assert!((p.bomb()[0].1 as usize) <= 3, "bomb judge index is a hit (PG/GR/GD/BD)");
    }

    #[test]
    fn empty_press_does_not_light_bomb() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.press(0, 100_000, |_| {});
        assert_eq!(p.bomb()[0].0, i64::MIN, "a press far from any note lights no bomb");
    }

    #[test]
    fn autoplay_tap_beam_flashes_then_clears_while_ln_holds() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00112:01\r\n");
        let mut p = Player::new(m, true);
        p.update(2_200_000, |_| {});
        assert_ne!(p.beam_on()[0], i64::MIN, "LN lane beam held through the long note");
        assert_eq!(p.beam_on()[1], i64::MIN, "tapped lane beam auto-cleared after the 80ms flash");
        assert_ne!(p.beam_off()[1], i64::MIN, "tapped lane left a release-fade timestamp");
    }

    #[test]
    fn judge_rate_widens_windows() {
        let m = model(b"#BPM 120\r\n#RANK 2\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut narrow = Player::new(m.clone(), false);
        assert_eq!(narrow.press(0, nt - 17_000, |_| {}).unwrap().judge, rbms_judge::Judge::Great, "17ms off is GREAT at default (#RANK 2 = 75%, PG ±15ms)");
        let mut wide = Player::new(m, false);
        wide.set_judge_rate(200);
        assert_eq!(wide.press(0, nt - 17_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "200% width makes 17ms off a PGREAT");
    }

    #[test]
    fn judge_width_does_not_widen_bad() {
        let m = model(b"#BPM 120\r\n#RANK 2\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m.clone(), false);
        p.set_judge_rate(200);
        assert!(p.press(0, nt + 250_000, |_| {}).is_none(), "250ms late is past the BAD edge, width cannot reach it");
        let mut q = Player::new(m, false);
        assert_eq!(q.press(0, nt + 200_000, |_| {}).unwrap().judge, rbms_judge::Judge::Bad, "200ms late is inside the fixed BAD window at stock width");
    }

    #[test]
    fn judge_width_rates_reach_key_and_scratch_lanes_separately() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n#00116:01\r\n");
        let key_t = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let scr_t = m.timelines.iter().find_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).unwrap();

        let mut stock = Player::new(m.clone(), false);
        assert_eq!(stock.press(0, key_t - 15_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "15ms off is inside the stock key PGREAT");

        let mut p = Player::new(m, false);
        p.set_judge_window_rates([50, 100, 100], [100, 100, 100]);
        assert_eq!(p.press(0, key_t - 15_000, |_| {}).unwrap().judge, rbms_judge::Judge::Great, "50% key rate pulls the key PGREAT in to +-10ms");
        assert_eq!(p.press(7, scr_t - 25_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "the scratch lane kept its own 100% rate");
    }

    #[test]
    fn dangling_longstart_does_not_stick_beam() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000000\r\n");
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |_| {});
        assert_eq!(p.beam_on()[0], i64::MIN, "an unterminated LN (no LongEnd) must not leave the beam stuck on");
    }

    #[test]
    fn interactive_press_and_release_judges_ln() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let times: Vec<i64> = m.timelines.iter().flat_map(|t| t.notes[0].as_ref()).map(|nn| nn.time_us).collect();
        let (head, end) = (times[0], times[1]);
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        let r = p.press(0, head, |e| events.push(e));
        assert_eq!(r.unwrap().judge, rbms_judge::Judge::PerfectGreat, "LN head judged");
        let rr = p.release(0, end);
        assert_eq!(rr.unwrap().judge, rbms_judge::Judge::PerfectGreat, "LN release judged");
        assert_eq!(p.judge.counts[0], 1);
    }

    #[test]
    fn ln_release_window_scales_with_rank() {
        // The LN-end window now follows the chart #RANK (it used to be a fixed 100% constant that
        // ignored rank entirely). A release 100ms before the end is only GOOD at #RANK 1 (HARD,
        // 50% width: GD edge = ±100ms) but a PGREAT at #RANK 3 (NORMAL, 100%: PG edge = ±120ms).
        let ln = |bms: &[u8]| {
            let m = model(bms);
            let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
            (m, t[0], t[1])
        };

        let (m, head, end) = ln(b"#BPM 120\r\n#RANK 1\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let mut hard = Player::new(m, false);
        assert_eq!(hard.press(0, head, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat);
        assert_eq!(hard.release(0, end - 100_000).unwrap().judge, rbms_judge::Judge::Good, "100ms-early LN release is GOOD at #RANK 1");

        let (m2, head2, end2) = ln(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let mut normal = Player::new(m2, false);
        assert_eq!(normal.press(0, head2, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat);
        assert_eq!(normal.release(0, end2 - 100_000).unwrap().judge, rbms_judge::Judge::PerfectGreat, "same release is PGREAT at #RANK 3");
    }

    // ---- helpers -------------------------------------------------------------------------

    /// Run a full autoplay pass and collect every emitted keysound, plus the final engine.
    fn autoplay_collect(m: Model) -> (JudgeEngine, Vec<PlayEvent>) {
        let mut p = Player::new(m, true);
        let mut events = Vec::new();
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |e| events.push(e));
        (p.into_judge(), events)
    }

    // ---- autoplay: invariants over dense charts ------------------------------------------

    #[test]
    fn autoplay_dense_chart_all_pgreat_exscore_2n_no_miss() {
        // A dense multi-lane chart: every playable note must be PGREAT, EX == 2n, full combo, 0 miss.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:0101010101010101\r\n#00112:0101010101010101\r\n#00113:0101010101010101\r\n#00211:0101010101010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "every note PGREAT");
        assert_eq!(engine.ex_score, 2 * n as u32, "EX == 2n for an all-PGREAT clear");
        assert_eq!(engine.max_combo, n as u32, "full combo");
        assert_eq!(engine.counts[5], 0, "no miss");
        assert_eq!(engine.counts.iter().sum::<u32>(), n as u32, "exactly n judgments, LNs not double-counted");
    }

    #[test]
    fn autoplay_ignores_mines_count_and_judges() {
        // Channel D1 (=469) is lane-0 mines. Mines must not be counted nor judged; the lone
        // normal note in lane 1 is the only playable note.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#001D1:01010101\r\n#00112:01\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        assert_eq!(n, 1, "mines are excluded from the playable count");
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], 1, "only the real note is judged (PGREAT)");
        assert_eq!(engine.counts.iter().sum::<u32>(), 1, "mines never produce a judgment");
    }

    #[test]
    fn autoplay_mixed_ln_and_normal_counts_each_once() {
        // One LN (lane 0) + several normal notes (lane 1). count_playable == LN(1) + normals.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00112:01010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "LN + normals all PGREAT");
        assert_eq!(engine.ex_score, 2 * n as u32);
        assert_eq!(engine.max_combo, n as u32);
    }

    #[test]
    fn autoplay_empty_chart_yields_no_judgments() {
        // No playable notes at all: a clean run produces an empty engine (no panics, all zeros).
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00101:01\r\n");
        assert_eq!(rbms_chart::count_playable_notes(&m), 0, "BGM-only chart has no playable notes");
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts, [0; 6]);
        assert_eq!(engine.ex_score, 0);
        assert_eq!(engine.max_combo, 0);
        assert_eq!(engine.total_judged(), 0);
    }

    #[test]
    fn autoplay_is_deterministic_across_runs() {
        // Two identical autoplay passes must produce identical scores (no hidden state/ordering).
        let m = model(b"#BPM 120\r\n#RANK 2\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01010101\r\n#00112:01000001\r\n");
        let a = simulate_autoplay(&m);
        let b = simulate_autoplay(&m);
        assert_eq!(a.counts, b.counts);
        assert_eq!(a.ex_score, b.ex_score);
        assert_eq!(a.max_combo, b.max_combo);
        assert_eq!(a.empty_poor, b.empty_poor);
    }

    #[test]
    fn autoplay_scratch_lane_judged_like_keys() {
        // The scratch lane (channel 16 -> lane 7) is autoplayed the same as key lanes.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00116:01010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "scratch-lane autoplay is all PGREAT");
        assert_eq!(engine.max_combo, n as u32);
    }

    // ---- autoplay keysounds: count & ordering --------------------------------------------

    #[test]
    fn autoplay_emits_bg_and_note_keysounds_time_sorted() {
        // BGM (chan 01) + a note (chan 11): both keysounds fire, and emission is non-decreasing in time.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00101:02\r\n#00111:01\r\n");
        let (_engine, events) = autoplay_collect(m);
        assert!(events.len() >= 2, "a BGM and a note both produce keysounds");
        for w in events.windows(2) {
            assert!(w[0].at_us <= w[1].at_us, "keysounds emitted in non-decreasing time order");
        }
    }

    #[test]
    fn autoplay_ln_emits_exactly_one_keysound() {
        // An LN produces a single press keysound (the head wav); the release emits no sound.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let (_engine, events) = autoplay_collect(m);
        assert_eq!(events.len(), 1, "an LN fires exactly one (head) keysound");
        assert_eq!(events[0].wav, 1, "the head wav is played");
    }

    #[test]
    fn dangling_longstart_emits_no_keysound() {
        // An LN head with no matching LongEnd: collect_actions never pushes a Press, so no sound.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000000\r\n");
        let (engine, events) = autoplay_collect(m);
        assert!(events.is_empty(), "a dangling LongStart must not sound");
        assert_eq!(engine.total_judged(), 0, "and produces no judgment in autoplay");
    }

    // ---- auto_lanes -----------------------------------------------------------------------

    #[test]
    fn auto_lane_ignores_input_on_that_lane_but_judges_from_chart() {
        // Scratch lane (7) set auto: its note is auto-judged, and a press on lane 7 is dropped.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        assert!(p.press(7, nt, |_| {}).is_none(), "press on an auto lane returns None");
        assert!(p.release(7, nt).is_none(), "release on an auto lane returns None");
        assert_eq!(p.judge.counts[0], 0, "input did not score it early");
        p.update(nt + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[0], 1, "the chart auto-judged it as PGREAT");
    }

    #[test]
    fn auto_lane_press_does_not_light_beam() {
        // An ignored press on an auto lane must not light that lane's beam.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        p.press(7, 500_000, |_| {});
        assert_eq!(p.beam_on()[7], i64::MIN, "ignored auto-lane press leaves the beam off");
    }

    #[test]
    fn non_auto_lane_in_interactive_is_not_auto_judged() {
        // With one lane on auto, the OTHER lanes are still fully interactive (not auto-judged).
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        let last = p.last_time_us();
        p.update(last + 1_000_000, |_| {});
        // Lane 7 (auto) was hit as PGREAT; lane 0's note was never pressed, so it was swept to MISS.
        assert_eq!(p.judge.counts[0], 1, "only the auto lane scored");
        assert_eq!(p.judge.counts[4], 1, "the interactive lane's unpressed note became a 見逃し POOR");
    }

    #[test]
    fn set_auto_lanes_wrong_length_is_a_noop() {
        // The length guard: a vector that doesn't match the lane count is ignored entirely.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_auto_lanes(vec![true; 3]); // wrong length (8 expected) -> rejected by the guard
        // No auto-update happens for lane 7 even after passing its time: input is required.
        p.update(nt - 1_000, |_| {});
        assert_eq!(p.judge.counts[0], 0, "no lane became auto, so nothing was auto-judged");
        // And input on lane 7 is still honored (not treated as auto): an on-time press scores it.
        assert!(p.press(7, nt, |_| {}).is_some(), "lane 7 input is honored when the guard rejected the vector");
    }

    #[test]
    fn set_auto_lanes_exact_length_is_applied() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes(vec![false; 8]); // exact length accepted
        // Now make lane 7 auto and confirm it takes effect.
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        let last = p.last_time_us();
        p.update(last + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[0], 1, "exact-length auto vector took effect");
    }

    // ---- interactive: beam / bomb transitions --------------------------------------------

    #[test]
    fn empty_press_lights_beam_but_no_bomb() {
        // A press far from any note: beam lights (input feedback) but no bomb (no hit/empty-poor only).
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.press(0, 100_000, |_| {});
        assert_eq!(p.beam_on()[0], 100_000, "even an empty press lights the beam");
        assert_eq!(p.bomb()[0].0, i64::MIN, "but lights no bomb");
    }

    #[test]
    fn bomb_only_fires_for_judge_index_le_3() {
        // A clean BAD (index 3) lights a bomb; an empty POOR (index 4, far-early press) does not.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        // BAD: a ~250ms-late press (BAD late edge at #RANK 3 = -280ms) is judge index 3.
        let mut bad = Player::new(m.clone(), false);
        let r = bad.press(0, nt + 250_000, |_| {}).unwrap();
        assert_eq!(r.judge, rbms_judge::Judge::Bad);
        assert_eq!(bad.bomb()[0].0, nt + 250_000, "a BAD (index 3) lights a bomb");
        assert_eq!(bad.bomb()[0].1, 3, "bomb judge index is BAD");
        let mut poor = Player::new(m, false);
        let rp = poor.press(0, nt - 300_000, |_| {}).unwrap();
        assert_eq!(rp.judge, rbms_judge::Judge::Miss, "300ms-early is an empty poor (judge code 5)");
        assert_eq!(poor.bomb()[0].0, i64::MIN, "an empty POOR (index 5) lights no bomb");
    }

    #[test]
    fn release_without_held_ln_still_clears_beam() {
        // A release with nothing held (no LN) returns None but still clears the beam/sets fade.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.press(0, 500_000, |_| {});
        assert_eq!(p.beam_on()[0], 500_000);
        let r = p.release(0, 600_000);
        assert!(r.is_none(), "no held LN -> release judges nothing");
        assert_eq!(p.beam_on()[0], i64::MIN, "beam cleared on release");
        assert_eq!(p.beam_off()[0], 600_000, "release fade timestamp set");
    }

    #[test]
    fn release_when_beam_already_off_sets_no_fade() {
        // release() only stamps beam_off if beam_on was lit; an unpaired release leaves beam_off at MIN.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.release(0, 700_000);
        assert_eq!(p.beam_on()[0], i64::MIN);
        assert_eq!(p.beam_off()[0], i64::MIN, "release with no prior press leaves no fade timestamp");
    }

    #[test]
    fn interactive_ln_press_lights_beam_until_release() {
        // The LN head press lights the beam; it stays lit (ln_active) until the release clears it.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
        let (head, end) = (t[0], t[1]);
        let mut p = Player::new(m, false);
        p.press(0, head, |_| {});
        assert_eq!(p.beam_on()[0], head, "LN head lights the beam");
        p.release(0, end);
        assert_eq!(p.beam_on()[0], i64::MIN, "release clears the beam");
        assert_eq!(p.beam_off()[0], end, "release fade timestamp set at the end");
    }

    #[test]
    fn ln_head_press_lights_bomb_for_pgreat() {
        // Pressing an LN head on time is a PGREAT (index 0) -> a bomb is lit at the head.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let head = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).next().unwrap();
        let mut p = Player::new(m, false);
        p.press(0, head, |_| {});
        assert_eq!(p.bomb()[0].0, head, "LN head hit lights a bomb");
        assert_eq!(p.bomb()[0].1, 0, "PGREAT head -> bomb index 0");
    }

    // ---- nearest_head_wav selection -------------------------------------------------------

    #[test]
    fn press_plays_nearest_head_wav_in_lane() {
        // Two notes in lane 0 with different wavs. A press near the second must emit the second's wav.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00111:01000002\r\n");
        let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
        let (first, second) = (t[0], t[1]);
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        // Press much closer to the second note.
        p.press(0, second - 5_000, |e| events.push(e));
        assert_eq!(events.len(), 1, "a press emits exactly one keysound");
        assert_eq!(events[0].wav, 2, "the nearest head's wav (note 2) is chosen");
        // And a press nearer the first picks wav 1.
        let mut p2 = {
            let m2 = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00111:01000002\r\n");
            Player::new(m2, false)
        };
        let mut ev2 = Vec::new();
        p2.press(0, first + 5_000, |e| ev2.push(e));
        assert_eq!(ev2[0].wav, 1, "nearer the first head -> wav 1");
    }

    #[test]
    fn nearest_head_wav_is_lane_scoped() {
        // A note in lane 1 must not be heard when pressing lane 0 (no head in lane 0 -> no sound).
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00112:01\r\n");
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        p.press(0, 2_000_000, |e| events.push(e));
        assert!(events.is_empty(), "pressing an empty lane emits no keysound");
    }

    #[test]
    fn nearest_head_wav_uses_ln_head_for_lookup() {
        // collect_note_heads includes LN starts; pressing the LN lane emits the head wav.
        let m = model(b"#BPM 120\r\n#WAV07 g.wav\r\n#00151:07000007\r\n");
        let head = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).next().unwrap();
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        p.press(0, head, |e| events.push(e));
        assert_eq!(events[0].wav, 7, "LN head wav (07) is selectable via nearest_head_wav");
    }

    // ---- set_judge_rate widening / clamping -----------------------------------------------

    #[test]
    fn judge_rate_zero_clamps_to_min_not_panic() {
        // rate_percent <= 0 is clamped to 1 (rate.max(1)); the windows stay tiny but valid.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_judge_rate(0); // must not panic / divide-by-zero
        // A perfectly-on-time press is still a PGREAT even at the tiniest width.
        assert_eq!(p.press(0, nt, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "on-time press is PGREAT at clamped min width");
    }

    #[test]
    fn judge_rate_widening_reaches_pgreat_edge() {
        // At #RANK 0 (25%) the PG window is ±5ms; a 14ms-early press is a GREAT. Widening to 400%
        // (eff 100%, PG ±20ms) turns the SAME press into a PGREAT.
        let m = model(b"#BPM 120\r\n#RANK 0\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut narrow = Player::new(m.clone(), false);
        assert_eq!(narrow.press(0, nt - 14_000, |_| {}).unwrap().judge, rbms_judge::Judge::Great, "14ms off is GREAT at #RANK 0 (PG ±5ms)");
        let mut wide = Player::new(m, false);
        wide.set_judge_rate(400);
        assert_eq!(wide.press(0, nt - 14_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "400% width reaches the PGREAT edge");
    }

    #[test]
    fn judge_width_cannot_reach_past_the_rank_bad_edge() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut narrow = Player::new(m.clone(), false);
        assert!(narrow.press(0, nt + 300_000, |_| {}).is_none(), "300ms late at 100% is beyond every window");
        let mut wide = Player::new(m, false);
        wide.set_judge_rate(200);
        assert!(wide.press(0, nt + 300_000, |_| {}).is_none(), "200% width still cannot reach past the BAD edge");
    }

    #[test]
    fn judge_rate_does_not_widen_fixed_ms_window() {
        // scaled() fixes the MS window. The far-early empty-POOR boundary (ms.1 = +500ms) does not
        // move with width: a press 600ms early is outside even at 200%.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_judge_rate(200);
        // 600ms early: dm = +600_000 > ms.1(+500_000) -> beyond the candidate gate, no match.
        assert!(p.press(0, nt - 600_000, |_| {}).is_none(), "the fixed MS early edge (+500ms) does not widen with JUDGE WIDTH");
    }

    // ---- update() sweep: MISS / LN finalisation ------------------------------------------

    #[test]
    fn unpressed_note_swept_to_miss_breaks_combo() {
        // An interactive note never pressed: update() past its BAD-late bound sweeps it to MISS.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.update(nt + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[4], 1, "unpressed note swept to 見逃し POOR (reference judge code 4)");
        assert_eq!(p.judge.counts[5], 0, "nothing lands in the 空POOR slot");
        assert_eq!(p.judge.max_combo, 0, "a swept POOR keeps combo at zero");
    }

    #[test]
    fn held_ln_never_released_finalises_via_update_sweep() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
        let (head, end) = (t[0], t[1]);
        let mut p = Player::new(m, false);
        assert_eq!(p.press(0, head, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "LN head PGREAT");
        p.update(end - 1, |_| {});
        assert_eq!(p.judge.total_judged(), 0, "before the end the LN is still held");
        p.update(end + 1_000_000, |_| {});
        assert_eq!(p.judge.total_judged(), 1, "the over-held LN is finalized exactly once");
        assert_eq!(p.judge.counts[0], 1, "finalized with the head PGREAT (lnstartJudge)");
    }

    #[test]
    fn update_is_idempotent_after_full_judge() {
        // Re-running update() past the end of an already fully-judged autoplay must not change counts.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n");
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |_| {});
        let snapshot = p.judge.counts;
        let ex = p.judge.ex_score;
        p.update(end + 5_000_000, |_| {}); // sweep again far past the end
        assert_eq!(p.judge.counts, snapshot, "no extra judgments on a second sweep");
        assert_eq!(p.judge.ex_score, ex);
    }

    #[test]
    fn update_monotonic_combo_progress_in_autoplay() {
        // Driving autoplay in small steps yields a monotonically non-decreasing combo that ends full.
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:0101010101010101\r\n");
        let n = rbms_chart::count_playable_notes(&m) as u32;
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        let mut prev = 0u32;
        let mut t = 0i64;
        while t <= end {
            p.update(t, |_| {});
            assert!(p.judge.combo >= prev, "autoplay combo never drops");
            prev = p.judge.combo;
            t += 100_000;
        }
        assert_eq!(p.judge.combo, n, "autoplay ends on a full combo");
        assert_eq!(p.judge.max_combo, n);
    }

    // ---- last_time_us / construction edges -----------------------------------------------

    #[test]
    fn last_time_us_is_final_timeline_time() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00311:01\r\n");
        let expected = m.timelines.last().map(|t| t.time_us).unwrap();
        let p = Player::new(m, true);
        assert_eq!(p.last_time_us(), expected, "last_time_us mirrors the final timeline's time");
    }

    #[test]
    fn new_player_starts_with_clean_beam_bomb_state() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let p = Player::new(m, false);
        assert_eq!(p.beam_on().len(), 8, "one beam slot per lane (BEAT_7K key=8)");
        assert!(p.beam_on().iter().all(|&v| v == i64::MIN), "all beams initially off");
        assert!(p.beam_off().iter().all(|&v| v == i64::MIN), "no fades pending");
        assert!(p.bomb().iter().all(|&(t, j)| t == i64::MIN && j == 0), "no bombs initially");
    }

    // ---- press on out-of-range lane -------------------------------------------------------

    #[test]
    fn press_out_of_range_lane_returns_none_and_no_bomb() {
        // Lane index beyond the mode's lane count: judge.press returns None (lane lookup fails),
        // and the out-of-range beam/bomb writes are guarded so nothing panics.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        assert!(p.press(99, 1_000_000, |_| {}).is_none(), "press on a non-existent lane judges nothing");
    }

    // ---- bomb does not fire on a swept miss ----------------------------------------------

    #[test]
    fn swept_miss_does_not_light_bomb() {
        // A MISS comes only from update()'s sweep, which never touches the bomb array.
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        let nt = p.last_time_us();
        p.update(nt + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[4], 1, "the note was swept to 見逃し POOR");
        assert_eq!(p.bomb()[0].0, i64::MIN, "a swept POOR lights no bomb");
    }
}

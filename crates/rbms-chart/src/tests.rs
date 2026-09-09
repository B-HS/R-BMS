use super::*;
use rbms_parser::parse;

fn model(bms: &[u8]) -> Model {
    to_model(&parse(bms), Mode::BEAT_7K)
}

fn first_note(m: &Model, lane: usize) -> &Note {
    m.timelines.iter().flat_map(|tl| tl.notes[lane].as_ref()).next().unwrap()
}

fn first_ln_kind(m: &Model, lane: usize) -> LnKind {
    m.timelines
        .iter()
        .flat_map(|tl| tl.notes[lane].as_ref())
        .find_map(|n| match n.kind {
            NoteKind::LongStart { ln } | NoteKind::LongEnd { ln } => Some(ln),
            _ => None,
        })
        .unwrap()
}

#[test]
fn fixed_bpm_absolute_time() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 2_000_000);
}

#[test]
fn measure_index_two_is_two_measures() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 4_000_000);
}

#[test]
fn half_position_is_half_measure() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:0001\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000);
}

#[test]
fn channel_to_lane_7k() {
    let m = model(b"#WAV01 a.wav\r\n#00111:01\r\n#00118:01\r\n#00119:01\r\n#00116:01\r\n#00113:01\r\n");
    let lanes: Vec<usize> = (0..8).filter(|&l| m.timelines.iter().any(|tl| tl.notes[l].is_some())).collect();
    assert_eq!(lanes, vec![0, 2, 5, 6, 7]);
}

#[test]
fn stop_shifts_following_notes_by_one_measure() {
    let with = model(b"#BPM 120\r\n#STOP01 192\r\n#WAV01 a.wav\r\n#00109:01\r\n#00211:01\r\n");
    let without = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    assert_eq!(without.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).next().unwrap().time_us, 4_000_000);
    assert_eq!(first_note(&with, 0).time_us, 6_000_000);
}

#[test]
fn bpm_change_inline_channel_03_hex() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00103:F0\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000);
}

#[test]
fn bpm_ref_channel_08_doubles_speed() {
    let base = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    let changed = model(b"#BPM 120\r\n#BPM01 240\r\n#WAV01 a.wav\r\n#00108:01\r\n#00211:01\r\n");
    assert_eq!(base.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).next().unwrap().time_us, 4_000_000);
    assert_eq!(first_note(&changed, 0).time_us, 3_000_000);
}

#[test]
fn ln_channel_pairs_start_end() {
    let m = model(b"#WAV01 a.wav\r\n#00151:01000001\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert_eq!(kinds.len(), 2);
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }));
    assert!(matches!(kinds[1], NoteKind::LongEnd { .. }));
    assert_eq!(count_playable_notes(&m), 1);
}

#[test]
fn lnmode_defaults_to_ln() {
    let m = model(b"#WAV01 a.wav\r\n#00151:01000001\r\n");
    assert_eq!(first_ln_kind(&m, 0), LnKind::Ln, "no #LNMODE -> plain LN");
}

#[test]
fn lnmode_1_is_explicit_ln() {
    let m = model(b"#LNMODE 1\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
    assert_eq!(first_ln_kind(&m, 0), LnKind::Ln, "#LNMODE 1 -> explicit LN");
}

#[test]
fn lnmode_2_marks_charge_notes() {
    let m = model(b"#LNMODE 2\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
    assert_eq!(first_ln_kind(&m, 0), LnKind::Cn, "#LNMODE 2 -> CN");
}

#[test]
fn lnmode_3_marks_hell_charge_notes() {
    let m = model(b"#LNMODE 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
    assert_eq!(first_ln_kind(&m, 0), LnKind::Hcn, "#LNMODE 3 -> HCN");
}

#[test]
fn lnobj_converts_previous_note() {
    let m = model(b"#LNOBJ ZZ\r\n#WAV01 a.wav\r\n#00111:01\r\n#00211:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }));
    assert!(matches!(kinds[1], NoteKind::LongEnd { .. }));
}

#[test]
fn lnobj_does_not_corrupt_a_mine_in_the_tracked_slot() {
    let m = model(b"#LNOBJ ZZ\r\n#WAVZZ a.wav\r\n#00111:01\r\n#001D1:ZZ\r\n#00211:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(kinds.iter().any(|k| matches!(k, NoteKind::Mine { .. })));
    assert!(!kinds.iter().any(|k| matches!(k, NoteKind::LongStart { .. })));
}

#[test]
fn zero_bpm_does_not_produce_nan_time() {
    let m = model(b"#BPM 0\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    let t = first_note(&m, 0).time_us;
    assert!(t > 0 && t < i64::MAX);
}

#[test]
fn detect_mode_variants() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n#00118:01\r\n"), "x.bme"), Mode::BEAT_7K);
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n#00115:01\r\n#00116:01\r\n"), "x.bms"), Mode::BEAT_5K);
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n#00121:01\r\n"), "x.bme"), Mode::BEAT_14K);
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n"), "x.pms"), Mode::POPN_9K);
}

#[test]
fn p2_channels_ignored_in_7k_no_panic() {
    let m = model(b"#WAV01 a.wav\r\n#00111:01\r\n#00121:01\r\n#00129:01\r\n");
    assert_eq!(count_playable_notes(&m), 1);
}

#[test]
fn bgm_goes_to_bgnotes() {
    let m = model(b"#WAV01 a.wav\r\n#00101:01\r\n");
    assert!(m.timelines.iter().any(|tl| !tl.bgnotes.is_empty()));
    assert_eq!(count_playable_notes(&m), 0);
}

#[test]
fn mine_channel_is_mine_note() {
    let m = model(b"#WAVZZ a.wav\r\n#001D1:ZZ\r\n");
    let has_mine = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).any(|n| matches!(n.kind, NoteKind::Mine { .. }));
    assert!(has_mine);
    assert_eq!(count_playable_notes(&m), 0);
}

#[test]
fn mine_damage_is_the_raw_object_value_in_the_chart_base() {
    let damage = |bms: &[u8]| {
        model(bms)
            .timelines
            .iter()
            .flat_map(|tl| tl.notes[0].as_ref())
            .find_map(|n| match n.kind {
                NoteKind::Mine { damage } => Some(damage),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(damage(b"#001D1:0A\r\n"), 10.0, "base36 0A = 10, beatoraja's stock mine damage");
    assert_eq!(damage(b"#001D1:ZZ\r\n"), 1295.0, "base36 ZZ = 35*36+35");
    assert_eq!(damage(b"#BASE 62\r\n#001D1:zz\r\n"), 3843.0, "base62 zz = 61*62+61");
}

#[test]
fn probe_unterminated_ln_dangling_start() {
    let m = model(b"#WAV01 a.wav\r\n#00151:01\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    eprintln!("PROBE kinds={:?} playable={}", kinds, count_playable_notes(&m));
}


#[test]
fn note_density_bins_normal_notes_per_second() {
    // BPM 120 ⇒ measure 0 spans [0,2)s; four lane-0 notes at 0/0.5/1.0/1.5s ⇒ two per second.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01010101\r\n");
    let d = note_density(&m, 0.0);
    assert!(d.bins.len() >= 3);
    assert_eq!(d.bins.iter().sum::<u32>(), 4);
    assert_eq!(d.bins[0], 2);
    assert_eq!(d.bins[1], 2);
    assert_eq!(d.peak, 2.0);
    assert!(d.avg > 0.0);
    assert!(d.end > 0.0);
}

#[test]
fn note_density_counts_long_note_body_in_each_spanned_second() {
    // One LN held from 0s to ~1.67s ⇒ occupies seconds 0 and 1, so peak stays 1 across the span.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00051:01000001\r\n");
    let d = note_density(&m, 0.0);
    assert!(d.bins.len() >= 3);
    assert_eq!(d.bins[0], 1);
    assert_eq!(d.bins[1], 1);
    assert_eq!(d.peak, 1.0);
}

#[test]
fn note_density_bins_stop_at_last_note_not_trailing_bar_lines() {
    // A note only at t=0 plus a BGM in a much later measure: the per-measure bar-line timelines must
    // not pad the histogram out to the BGM, so the bins stay short (sized to the last *note*).
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01\r\n#00301:01\r\n");
    assert!(m.timelines.last().unwrap().time_us >= 4_000_000, "fixture should carry trailing timelines past the note");
    let d = note_density(&m, 0.0);
    assert!(d.bins.len() <= 3, "bins {} should be sized to the last note (t=0), not the trailing bar lines", d.bins.len());
}

#[test]
fn note_density_dangling_long_note_head_still_counts() {
    // An unterminated LN head must register at head-time (rbms keeps dangling heads, unlike beatoraja's
    // auto-terminating parser), matching count_playable_notes which counts the head.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00051:01\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins[0], 1, "dangling LN head should contribute one note at its second");
    assert_eq!(d.peak, 1.0);
}

// ---------------------------------------------------------------------------
// detect_mode: exhaustive extension / channel combinations
// ---------------------------------------------------------------------------

#[test]
fn detect_mode_pms_extension_is_popn_regardless_of_channels() {
    use super::detect_mode;
    // The .pms short-circuit fires before any channel inspection, even with P2/key6-7 channels present.
    assert_eq!(detect_mode(&parse(b"#00121:01\r\n#00119:01\r\n"), "song.pms"), Mode::POPN_9K);
}

#[test]
fn detect_mode_pms_extension_is_case_insensitive() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n"), "SONG.PMS"), Mode::POPN_9K);
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n"), "SoNg.PmS"), Mode::POPN_9K);
}

#[test]
fn detect_mode_no_extension_defaults_to_5k() {
    use super::detect_mode;
    // No '.' in the filename -> extension is "" -> BMS family, and with no P2 / no key6-7 -> 5K.
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n"), "noextension"), Mode::BEAT_5K);
}

#[test]
fn detect_mode_empty_chart_is_5k() {
    use super::detect_mode;
    // No used channels at all -> p2=false, keys67=false -> 5K (the all-false fallback).
    assert_eq!(detect_mode(&parse(b""), "x.bme"), Mode::BEAT_5K);
}

#[test]
fn detect_mode_key6_channel_18_promotes_to_7k() {
    use super::detect_mode;
    // Channel "18" == raw 44 is in the keys67 set -> 7K even alone.
    assert_eq!(detect_mode(&parse(b"#00118:01\r\n"), "x.bms"), Mode::BEAT_7K);
}

#[test]
fn detect_mode_key7_channel_19_promotes_to_7k() {
    use super::detect_mode;
    // Channel "19" == raw 45 is in the keys67 set -> 7K.
    assert_eq!(detect_mode(&parse(b"#00119:01\r\n"), "x.bms"), Mode::BEAT_7K);
}

#[test]
fn detect_mode_scratch_only_chart_is_5k_not_7k() {
    use super::detect_mode;
    // Channel "16" (scratch, raw 42) is NOT in keys67 -> a chart with only keys 1-5 + scratch is 5K.
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n#00116:01\r\n"), "x.bms"), Mode::BEAT_5K);
}

#[test]
fn detect_mode_p2_low_channel_21_is_14k_even_without_key67() {
    use super::detect_mode;
    // Channel "21" == raw 73 is in the P2 range (73..=81) -> p2=true -> 14K regardless of keys67.
    assert_eq!(detect_mode(&parse(b"#00121:01\r\n"), "x.bms"), Mode::BEAT_14K);
}

#[test]
fn detect_mode_p2_takes_priority_over_keys67() {
    use super::detect_mode;
    // P2 + key6-7 both present: the (true, _) arm wins -> 14K.
    assert_eq!(detect_mode(&parse(b"#00128:01\r\n#00119:01\r\n"), "x.bme"), Mode::BEAT_14K);
}

#[test]
fn detect_mode_ln_mine_hidden_channels_do_not_affect_detection() {
    use super::detect_mode;
    // LN (ch "51"=181), mine (ch "D1"=469), hidden (ch "31"=109) channels are outside the P2/keys67
    // ranges detect_mode inspects, so a chart built only from them detects as the 5K fallback.
    assert_eq!(detect_mode(&parse(b"#00151:0101\r\n#001D1:01\r\n#00131:01\r\n"), "x.bms"), Mode::BEAT_5K);
}

// ---------------------------------------------------------------------------
// assign_times: timing invariants, BPM<=0, STOP, SCROLL, inline+ref BPM
// ---------------------------------------------------------------------------

#[test]
fn first_timeline_time_is_always_zero() {
    let m = model(b"#BPM 200\r\n#WAV01 a.wav\r\n#00311:01\r\n");
    assert_eq!(m.timelines[0].time_us, 0, "the first timeline anchors absolute time at 0");
}

#[test]
fn times_are_monotonically_non_decreasing() {
    // A chart mixing BPM changes, a STOP, and ordinary notes: time must never go backwards.
    let m = model(b"#BPM 120\r\n#BPM01 240\r\n#STOP01 96\r\n#WAV01 a.wav\r\n#00111:01\r\n#00108:01\r\n#00109:01\r\n#00211:01\r\n#00311:01\r\n");
    let times: Vec<i64> = m.timelines.iter().map(|t| t.time_us).collect();
    assert!(times.windows(2).all(|w| w[1] >= w[0]), "timeline times must be non-decreasing: {times:?}");
}

#[test]
fn bpm_zero_header_falls_back_to_130_no_nan() {
    // #BPM 0 -> init_bpm 0 -> assign_times substitutes the 130 BPM default. One measure at 130 BPM.
    let m = model(b"#BPM 0\r\n#WAV01 a.wav\r\n#00111:01\r\n");
    let t = first_note(&m, 0).time_us;
    let expected = (240_000_000.0 / 130.0) as i64;
    assert_eq!(t, expected, "BPM 0 must use the 130 default");
    assert!(t > 0 && t < i64::MAX);
}

#[test]
fn negative_inline_bpm_does_not_corrupt_timing() {
    // An inline BPM whose hex value is huge but a later real BPM still produces finite ordered times.
    // (Channel 03 reads a hex pair; "00" is dropped by the parser, so we use FF = 255.)
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00103:FF\r\n#00211:01\r\n");
    let t = first_note(&m, 0).time_us;
    assert!(t.is_positive() && t < i64::MAX, "inline BPM 255 yields finite time, got {t}");
}

#[test]
fn stop_accumulates_when_two_stops_share_a_position() {
    // STOP01=192 (one 1/192-measure unit per beatoraja => 1 whole measure) + STOP02=96 (half measure)
    // at the SAME position accumulate: at BPM 120 that is 2_000_000 + 1_000_000 = 3_000_000 us of hold.
    // The measure-2 note would sit at 4_000_000 without the hold, so it lands at 7_000_000.
    let m = model(b"#BPM 120\r\n#STOP01 192\r\n#STOP02 96\r\n#WAV01 a.wav\r\n#00109:0102\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 7_000_000, "two STOPs at one position add up");
}

#[test]
fn stop_value_192_holds_exactly_one_measure() {
    // STOP units are 1/192 of a 4/4 measure; STOP 192 == a full measure of freeze.
    let with = model(b"#BPM 120\r\n#STOP01 192\r\n#WAV01 a.wav\r\n#00109:01\r\n#00211:01\r\n");
    let stop_tl = with.timelines.iter().find(|t| t.stop_us > 0).unwrap();
    assert_eq!(stop_tl.stop_us, 2_000_000, "STOP 192 at BPM 120 freezes for one measure (2s)");
}

#[test]
fn negative_stop_is_ignored() {
    // assign_times only applies STOPs with s > 0; a negative STOP must not move later notes.
    let neg = model(b"#BPM 120\r\n#STOP01 -192\r\n#WAV01 a.wav\r\n#00109:01\r\n#00211:01\r\n");
    let without = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    let neg_t = first_note(&neg, 0).time_us;
    let plain_t = without.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).next().unwrap().time_us;
    assert_eq!(neg_t, plain_t, "a negative STOP is dropped, leaving timing unchanged");
    assert!(neg.timelines.iter().all(|t| t.stop_us == 0));
}

#[test]
fn scroll_channel_sets_segment_scroll() {
    // Channel "SC" (=1020) carries SCROLL multipliers; the value applies from that section onward.
    let m = model(b"#BPM 120\r\n#SCROLL01 2.0\r\n#WAV01 a.wav\r\n#001SC:01\r\n#00211:01\r\n");
    let scrolled = m.timelines.iter().find(|t| (t.section - 1.0).abs() < 1e-9).unwrap();
    assert_eq!(scrolled.scroll, 2.0);
    // Scroll persists forward to the next segment until changed.
    assert_eq!(m.timelines.last().unwrap().scroll, 2.0, "scroll carries forward");
}

#[test]
fn scroll_does_not_change_note_times() {
    // SCROLL only affects rendering speed, never the absolute note time produced by assign_times.
    let plain = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    let scrolled = model(b"#BPM 120\r\n#SCROLL01 0.25\r\n#WAV01 a.wav\r\n#001SC:01\r\n#00211:01\r\n");
    let pt = plain.timelines.iter().flat_map(|t| t.notes[0].as_ref()).next().unwrap().time_us;
    let st = first_note(&scrolled, 0).time_us;
    assert_eq!(pt, st, "SCROLL must not affect timing");
}

#[test]
fn negative_scroll_is_preserved_not_clamped() {
    let m = model(b"#BPM 120\r\n#SCROLL01 -1.5\r\n#WAV01 a.wav\r\n#001SC:01\r\n");
    let scrolled = m.timelines.iter().find(|t| t.scroll < 0.0);
    assert!(scrolled.is_some(), "a negative SCROLL value is kept verbatim");
    assert_eq!(scrolled.unwrap().scroll, -1.5);
}

#[test]
fn inline_ch03_bpm_alone_changes_speed() {
    // Channel 03 at measure 1 sets BPM 240 (F0 hex) for measure 1's traversal. measure 0 stays at 120
    // (2s), measure 1 traverses at 240 (1s), so the measure-2 note lands at 3_000_000.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00103:F0\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000, "inline F0 (=240 BPM) speeds up measure 1");
}

#[test]
fn ref_ch08_bpm_alone_changes_speed() {
    // Channel 08 looks the pair up in #BPMxx defs: BPM01=240 from measure 1. Same timing as the inline
    // case: 2s (measure 0 at 120) + 1s (measure 1 at 240) = 3s for the measure-2 note.
    let m = model(b"#BPM 120\r\n#BPM01 240\r\n#WAV01 a.wav\r\n#00108:01\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000, "ref BPM01=240 speeds up measure 1");
}

#[test]
fn inline_and_ref_bpm_same_value_agree() {
    // Both channel 03 (inline) and 08 (ref) at the same position set 240; the segment BPM and the
    // resulting note time match the single-channel cases exactly.
    let m = model(b"#BPM 120\r\n#BPM01 240\r\n#WAV01 a.wav\r\n#00103:F0\r\n#00108:01\r\n#00211:01\r\n");
    let seg = m.timelines.iter().find(|t| (t.section - 1.0).abs() < 1e-9).unwrap();
    assert_eq!(seg.bpm, 240.0);
    assert_eq!(first_note(&m, 0).time_us, 3_000_000);
}

#[test]
fn ref_ch08_to_undefined_bpm_is_dropped() {
    // BPM02 is never defined, so the channel-08 ref finds nothing and is skipped (no BPM change).
    let with = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00108:02\r\n#00211:01\r\n");
    let without = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    let wt = first_note(&with, 0).time_us;
    let ot = without.timelines.iter().flat_map(|t| t.notes[0].as_ref()).next().unwrap().time_us;
    assert_eq!(wt, ot, "an undefined BPM ref leaves timing unchanged");
    assert_eq!(wt, 4_000_000);
}

#[test]
fn section_rate_scales_measure_length() {
    // A 0.5 section rate on measure 0 halves it: measure-1 note = 0.5 measure (1s) + 1 measure (2s) = 3s.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00102:0.5\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000, "section rate 0.5 shortens measure 0");
}

// ---------------------------------------------------------------------------
// LNOBJ conversion and LN-channel pairing
// ---------------------------------------------------------------------------

#[test]
fn lnobj_does_not_convert_an_ln_channel_start() {
    // The slot tracked by last_normal only records *Normal* notes; an LN-channel LongStart never
    // becomes last_normal, so a later LNOBJ note in that lane stays Normal (it doesn't corrupt the LN).
    let m = model(b"#LNOBJ ZZ\r\n#WAVZZ a.wav\r\n#00151:01\r\n#00211:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }), "the LN-channel head is untouched");
    assert!(matches!(kinds[1], NoteKind::Normal), "LNOBJ with no tracked Normal stays Normal, got {:?}", kinds[1]);
}

#[test]
fn lnobj_with_no_previous_note_stays_normal() {
    // The very first note in a lane is the LNOBJ value: there is nothing to convert, so it is Normal.
    let m = model(b"#LNOBJ ZZ\r\n#WAVZZ a.wav\r\n#00111:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], NoteKind::Normal));
}

#[test]
fn lnobj_consumes_the_previous_note_only_once() {
    // Normal, then two LNOBJ notes: the first converts the Normal into a LongStart and itself into a
    // LongEnd; the second has no tracked Normal left (last_normal was taken) and stays Normal.
    let m = model(b"#LNOBJ ZZ\r\n#WAV01 a.wav\r\n#WAVZZ b.wav\r\n#00111:01\r\n#00211:ZZ\r\n#00311:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }));
    assert!(matches!(kinds[1], NoteKind::LongEnd { .. }));
    assert!(matches!(kinds[2], NoteKind::Normal), "the second LNOBJ has no Normal to convert");
    assert_eq!(count_playable_notes(&m), 2, "LongStart + trailing Normal are playable, LongEnd excluded");
}

#[test]
fn lnobj_conversion_preserves_playable_count() {
    // Two notes -> one LN (head+tail). The LongStart is playable, the LongEnd is not => one playable.
    let m = model(b"#LNOBJ ZZ\r\n#WAV01 a.wav\r\n#WAVZZ b.wav\r\n#00111:01\r\n#00211:ZZ\r\n");
    assert_eq!(count_playable_notes(&m), 1);
}

#[test]
fn ln_channel_second_open_after_close_is_a_new_start() {
    // Three markers on the LN channel: open, close, open(dangling). open/close toggle, so the third is
    // a fresh LongStart that stays dangling.
    let m = model(b"#WAV01 a.wav\r\n#00151:010101\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert_eq!(kinds.len(), 3);
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }));
    assert!(matches!(kinds[1], NoteKind::LongEnd { .. }));
    assert!(matches!(kinds[2], NoteKind::LongStart { .. }), "a re-open after close is a new LongStart");
    assert_eq!(count_playable_notes(&m), 2, "two heads are playable, the single end is excluded");
}

#[test]
fn dangling_ln_start_counts_as_one_playable() {
    // An LN head with no matching end is still a playable note (no LongEnd to exclude).
    let m = model(b"#WAV01 a.wav\r\n#00151:01\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }));
    assert_eq!(count_playable_notes(&m), 1);
}

#[test]
fn ln_open_close_is_independent_per_lane() {
    // An open LN in lane 0 must not bleed into lane 2: each lane toggles its own ln_open flag.
    let m = model(b"#WAV01 a.wav\r\n#00151:01\r\n#00153:0101\r\n");
    let lane0: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    let lane2: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[2].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(matches!(lane0[0], NoteKind::LongStart { .. }), "lane 0 dangling head");
    assert!(matches!(lane2[0], NoteKind::LongStart { .. }));
    assert!(matches!(lane2[1], NoteKind::LongEnd { .. }), "lane 2 closes on its own toggle");
}

// ---------------------------------------------------------------------------
// count_playable_notes: exclusions
// ---------------------------------------------------------------------------

#[test]
fn count_playable_excludes_mines_and_long_ends() {
    // One Normal (playable) + one LN pair (head playable, tail excluded) + one Mine (excluded) = 2.
    let m = model(b"#WAVZZ a.wav\r\n#WAV01 a.wav\r\n#00011:01\r\n#00151:0101\r\n#001D3:ZZ\r\n");
    assert_eq!(count_playable_notes(&m), 2);
}

#[test]
fn count_playable_ignores_bgm_and_hidden() {
    // BGM (ch01 -> bgnotes) and hidden (ch31) are never in tl.notes, so they never count as playable.
    let m = model(b"#WAV01 a.wav\r\n#00101:01\r\n#00131:01\r\n");
    assert_eq!(count_playable_notes(&m), 0, "BGM and hidden are not playable lane notes");
}

#[test]
fn count_playable_zero_for_mine_only_chart() {
    let m = model(b"#WAVZZ a.wav\r\n#001D1:ZZ\r\n#001D3:ZZ\r\n");
    assert_eq!(count_playable_notes(&m), 0);
}

// ---------------------------------------------------------------------------
// note_density: formula, bin sizing, defaults, edge inputs
// ---------------------------------------------------------------------------

#[test]
fn note_density_empty_chart_is_all_zero_no_panic() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins.iter().sum::<u32>(), 0);
    assert_eq!(d.peak, 0.0);
    assert_eq!(d.avg, 0.0);
    assert_eq!(d.end, 0.0);
    assert!(d.bins.len() >= 2, "bins always padded to at least 2 (last_us=0 -> +2)");
}

#[test]
fn note_density_single_note_bins_and_scalars() {
    // One note at t=0: last_us=0 -> bins=2. bd = 1/2/4 = 0 so every bin counts toward avg = 1/2 = 0.5.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins, vec![1, 0]);
    assert_eq!(d.peak, 1.0);
    assert_eq!(d.avg, 0.5);
}

#[test]
fn note_density_mines_excluded_from_bins() {
    // Mines never appear in the bin sums (only categories [0..6) excluding mine [6]).
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01\r\n#001D2:01\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins.iter().sum::<u32>(), 1, "only the Normal note contributes to the histogram");
    assert_eq!(d.peak, 1.0);
}

#[test]
fn note_density_long_note_body_fills_every_spanned_second() {
    // LN from 0s to ~1.67s spans seconds 0 and 1: head at 0, body fills second 1, so both bins are 1.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00051:01000001\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins[0], 1, "head registers in its second");
    assert_eq!(d.bins[1], 1, "body fills the next spanned second");
    assert_eq!(d.peak, 1.0);
}

#[test]
fn note_density_negative_total_uses_default_formula() {
    // total_value <= 0 routes to the BMS default-total formula, so a negative total matches a zero one.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01010101\r\n");
    let d0 = note_density(&m, 0.0);
    let dneg = note_density(&m, -42.0);
    assert_eq!(d0.end, dneg.end, "negative total falls back to the default total like zero does");
    assert_eq!(d0.avg, dneg.avg);
    assert_eq!(d0.bins, dneg.bins, "bins never depend on total at all");
}

#[test]
fn note_density_bins_independent_of_total_value() {
    // total_value only feeds the `end` border; the bins histogram is identical across totals.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01010101\r\n");
    let a = note_density(&m, 0.0).bins;
    let b = note_density(&m, 500.0).bins;
    assert_eq!(a, b);
}

#[test]
fn note_density_peak_is_max_bin() {
    // Four notes in second 0, one in second 1 (BPM 240 => measure spans 1s). peak == busiest second.
    let m = model(b"#BPM 240\r\n#WAV01 a.wav\r\n#00011:01010101\r\n#00111:01000000\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.peak, *d.bins.iter().max().unwrap() as f64);
    assert!(d.peak >= 4.0, "the busiest second holds at least the four measure-0 notes, got {}", d.peak);
}

#[test]
fn note_density_bins_sum_equals_playable_normal_plus_ln_body() {
    // Sum over bins counts each Normal once and each LN head + one body-cell per extra spanned second.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01010101\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins.iter().sum::<u32>(), 4, "four plain notes => four histogram entries");
}

// ---------------------------------------------------------------------------
// resource maps & misc to_model invariants
// ---------------------------------------------------------------------------

#[test]
fn wavmap_is_sparse_and_indexed_by_id() {
    // build_resource_map sizes to max id + 1 and indexes each name by its numeric id (gaps are "").
    let m = model(b"#WAV05 e.wav\r\n#WAV01 a.wav\r\n#00011:01\r\n");
    assert_eq!(m.wavmap.len(), 6, "max id 5 -> length 6");
    assert_eq!(m.wavmap[1], "a.wav");
    assert_eq!(m.wavmap[5], "e.wav");
    assert_eq!(m.wavmap[2], "", "unused ids are empty strings");
}

#[test]
fn empty_resource_map_has_one_slot() {
    // No WAV defs -> max_id defaults to 0 -> a single empty slot.
    let m = model(b"#00011:01\r\n");
    assert_eq!(m.wavmap.len(), 1);
    assert_eq!(m.wavmap[0], "");
}

#[test]
fn p2_channels_ignored_in_7k_keep_only_p1_note() {
    // P2 note channels (21/29) map to lanes >= key in 7K and are dropped; only the P1 note survives.
    let m = model(b"#WAV01 a.wav\r\n#00111:01\r\n#00121:01\r\n#00129:01\r\n");
    assert_eq!(count_playable_notes(&m), 1);
}

#[test]
fn bgm_lands_in_bgnotes_not_lane_notes() {
    let m = model(b"#WAV01 a.wav\r\n#00101:01\r\n");
    assert!(m.timelines.iter().any(|tl| !tl.bgnotes.is_empty()), "BGM goes to bgnotes");
    assert_eq!(count_playable_notes(&m), 0);
}

#[test]
fn bgm_notes_get_their_timeline_time() {
    // assign_times stamps bgnotes with their timeline's time. A BGM at measure 1 (BPM 120) -> 2_000_000.
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00101:01\r\n");
    let bg = m.timelines.iter().flat_map(|t| t.bgnotes.iter()).next().unwrap();
    assert_eq!(bg.time_us, 2_000_000);
}



#[test]
fn default_total_matches_bmsplayerrule_formula() {
    assert!((default_total(1000) - 460.909_090_909_090_9).abs() < 1e-9);
    assert_eq!(default_total(100), 260.0);
    assert_eq!(default_total(0), 260.0);
}

#[test]
fn default_total_keyboard_uses_the_higher_floor_and_shifted_numerator() {
    assert!((default_total_keyboard(1000) - 507.0).abs() < 1e-9);
    assert_eq!(default_total_keyboard(0), 300.0);
}

#[test]
fn defexrank_header_reaches_model_meta() {
    let m = model(b"#DEFEXRANK 130\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
    assert_eq!(m.meta.defexrank, Some(130.0));
}

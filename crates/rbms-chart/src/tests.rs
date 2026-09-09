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
fn lnmode_absent_leaves_the_flavour_unstated() {
    let m = model(b"#WAV01 a.wav\r\n#00151:01000001\r\n");
    assert_eq!(first_ln_kind(&m, 0), LnKind::Undefined, "no #LNMODE -> the player's LN MODE decides");
}

#[test]
fn lnmode_0_leaves_the_flavour_unstated() {
    let m = model(b"#LNMODE 0\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
    assert_eq!(first_ln_kind(&m, 0), LnKind::Undefined, "#LNMODE 0 is the header's own unspecified value");
}

#[test]
fn lnmode_outside_the_header_scale_leaves_the_flavour_unstated() {
    for lnmode in ["4", "-1", "99"] {
        let src = format!("#LNMODE {lnmode}\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let m = model(src.as_bytes());
        assert_eq!(first_ln_kind(&m, 0), LnKind::Undefined, "#LNMODE {lnmode}");
    }
}

#[test]
fn player_ln_mode_resolves_only_the_unstated_charts() {
    for default_ln in [LnKind::Ln, LnKind::Cn, LnKind::Hcn] {
        let m = to_model_with_ln_mode(&parse(b"#WAV01 a.wav\r\n#00151:01000001\r\n"), Mode::BEAT_7K, default_ln);
        assert_eq!(first_ln_kind(&m, 0), default_ln, "an unstated chart takes the player's LN MODE");
    }
}

#[test]
fn player_ln_mode_never_overrides_an_explicit_chart_type() {
    for (header, stated) in [("#LNMODE 1\r\n", LnKind::Ln), ("#LNMODE 2\r\n", LnKind::Cn), ("#LNMODE 3\r\n", LnKind::Hcn)] {
        for default_ln in [LnKind::Ln, LnKind::Cn, LnKind::Hcn] {
            let src = format!("{header}#WAV01 a.wav\r\n#00151:01000001\r\n");
            let m = to_model_with_ln_mode(&parse(src.as_bytes()), Mode::BEAT_7K, default_ln);
            assert_eq!(first_ln_kind(&m, 0), stated, "{header} must win over {default_ln:?}");
        }
    }
}

#[test]
fn player_ln_mode_undefined_resolves_nothing() {
    let m = to_model_with_ln_mode(&parse(b"#WAV01 a.wav\r\n#00151:01000001\r\n"), Mode::BEAT_7K, LnKind::Undefined);
    assert_eq!(first_ln_kind(&m, 0), LnKind::Undefined);
}

#[test]
fn a_charge_note_default_makes_an_unstated_long_note_count_twice() {
    let src = parse(b"#WAV01 a.wav\r\n#00151:01000001\r\n");
    assert_eq!(count_playable_notes(&to_model(&src, Mode::BEAT_7K)), 1, "unstated long notes count once");
    assert_eq!(count_playable_notes(&to_model_with_ln_mode(&src, Mode::BEAT_7K, LnKind::Cn)), 2, "a charge note is judged at both ends");
    assert_eq!(count_playable_notes(&to_model_with_ln_mode(&src, Mode::BEAT_7K, LnKind::Hcn)), 2);
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
    assert_eq!(damage(b"#001D1:0A\r\n"), 10.0, "base36 0A = 10, the reference implementation's stock mine damage");
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
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00051:01000001\r\n");
    let d = note_density(&m, 0.0);
    assert!(d.bins.len() >= 3);
    assert_eq!(d.bins[0], 1);
    assert_eq!(d.bins[1], 1);
    assert_eq!(d.peak, 1.0);
}

#[test]
fn note_density_bins_stop_at_last_note_not_trailing_bar_lines() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01\r\n#00301:01\r\n");
    assert!(m.timelines.last().unwrap().time_us >= 4_000_000, "fixture should carry trailing timelines past the note");
    let d = note_density(&m, 0.0);
    assert!(d.bins.len() <= 3, "bins {} should be sized to the last note (t=0), not the trailing bar lines", d.bins.len());
}

#[test]
fn note_density_dangling_long_note_head_still_counts() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00051:01\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins[0], 1, "dangling LN head should contribute one note at its second");
    assert_eq!(d.peak, 1.0);
}

#[test]
fn detect_mode_pms_extension_is_popn_regardless_of_channels() {
    use super::detect_mode;
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
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n"), "noextension"), Mode::BEAT_5K);
}

#[test]
fn detect_mode_empty_chart_is_5k() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b""), "x.bme"), Mode::BEAT_5K);
}

#[test]
fn detect_mode_key6_channel_18_promotes_to_7k() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00118:01\r\n"), "x.bms"), Mode::BEAT_7K);
}

#[test]
fn detect_mode_key7_channel_19_promotes_to_7k() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00119:01\r\n"), "x.bms"), Mode::BEAT_7K);
}

#[test]
fn detect_mode_scratch_only_chart_is_5k_not_7k() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00111:01\r\n#00116:01\r\n"), "x.bms"), Mode::BEAT_5K);
}

#[test]
fn detect_mode_p2_low_channel_21_is_14k_even_without_key67() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00121:01\r\n"), "x.bms"), Mode::BEAT_14K);
}

#[test]
fn detect_mode_p2_takes_priority_over_keys67() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00128:01\r\n#00119:01\r\n"), "x.bme"), Mode::BEAT_14K);
}

#[test]
fn detect_mode_ln_mine_hidden_channels_do_not_affect_detection() {
    use super::detect_mode;
    assert_eq!(detect_mode(&parse(b"#00151:0101\r\n#001D1:01\r\n#00131:01\r\n"), "x.bms"), Mode::BEAT_5K);
}

#[test]
fn first_timeline_time_is_always_zero() {
    let m = model(b"#BPM 200\r\n#WAV01 a.wav\r\n#00311:01\r\n");
    assert_eq!(m.timelines[0].time_us, 0, "the first timeline anchors absolute time at 0");
}

#[test]
fn times_are_monotonically_non_decreasing() {
    let m = model(b"#BPM 120\r\n#BPM01 240\r\n#STOP01 96\r\n#WAV01 a.wav\r\n#00111:01\r\n#00108:01\r\n#00109:01\r\n#00211:01\r\n#00311:01\r\n");
    let times: Vec<i64> = m.timelines.iter().map(|t| t.time_us).collect();
    assert!(times.windows(2).all(|w| w[1] >= w[0]), "timeline times must be non-decreasing: {times:?}");
}

#[test]
fn bpm_zero_header_falls_back_to_130_no_nan() {
    let m = model(b"#BPM 0\r\n#WAV01 a.wav\r\n#00111:01\r\n");
    let t = first_note(&m, 0).time_us;
    let expected = (240_000_000.0 / 130.0) as i64;
    assert_eq!(t, expected, "BPM 0 must use the 130 default");
    assert!(t > 0 && t < i64::MAX);
}

#[test]
fn negative_inline_bpm_does_not_corrupt_timing() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00103:FF\r\n#00211:01\r\n");
    let t = first_note(&m, 0).time_us;
    assert!(t.is_positive() && t < i64::MAX, "inline BPM 255 yields finite time, got {t}");
}

#[test]
fn stop_accumulates_when_two_stops_share_a_position() {
    let m = model(b"#BPM 120\r\n#STOP01 192\r\n#STOP02 96\r\n#WAV01 a.wav\r\n#00109:0102\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 7_000_000, "two STOPs at one position add up");
}

#[test]
fn stop_value_192_holds_exactly_one_measure() {
    let with = model(b"#BPM 120\r\n#STOP01 192\r\n#WAV01 a.wav\r\n#00109:01\r\n#00211:01\r\n");
    let stop_tl = with.timelines.iter().find(|t| t.stop_us > 0).unwrap();
    assert_eq!(stop_tl.stop_us, 2_000_000, "STOP 192 at BPM 120 freezes for one measure (2s)");
}

#[test]
fn negative_stop_is_ignored() {
    let neg = model(b"#BPM 120\r\n#STOP01 -192\r\n#WAV01 a.wav\r\n#00109:01\r\n#00211:01\r\n");
    let without = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    let neg_t = first_note(&neg, 0).time_us;
    let plain_t = without.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).next().unwrap().time_us;
    assert_eq!(neg_t, plain_t, "a negative STOP is dropped, leaving timing unchanged");
    assert!(neg.timelines.iter().all(|t| t.stop_us == 0));
}

#[test]
fn scroll_channel_sets_segment_scroll() {
    let m = model(b"#BPM 120\r\n#SCROLL01 2.0\r\n#WAV01 a.wav\r\n#001SC:01\r\n#00211:01\r\n");
    let scrolled = m.timelines.iter().find(|t| (t.section - 1.0).abs() < 1e-9).unwrap();
    assert_eq!(scrolled.scroll, 2.0);
    assert_eq!(m.timelines.last().unwrap().scroll, 2.0, "scroll carries forward");
}

#[test]
fn scroll_does_not_change_note_times() {
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
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00103:F0\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000, "inline F0 (=240 BPM) speeds up measure 1");
}

#[test]
fn ref_ch08_bpm_alone_changes_speed() {
    let m = model(b"#BPM 120\r\n#BPM01 240\r\n#WAV01 a.wav\r\n#00108:01\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000, "ref BPM01=240 speeds up measure 1");
}

#[test]
fn inline_and_ref_bpm_same_value_agree() {
    let m = model(b"#BPM 120\r\n#BPM01 240\r\n#WAV01 a.wav\r\n#00103:F0\r\n#00108:01\r\n#00211:01\r\n");
    let seg = m.timelines.iter().find(|t| (t.section - 1.0).abs() < 1e-9).unwrap();
    assert_eq!(seg.bpm, 240.0);
    assert_eq!(first_note(&m, 0).time_us, 3_000_000);
}

#[test]
fn ref_ch08_to_undefined_bpm_is_dropped() {
    let with = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00108:02\r\n#00211:01\r\n");
    let without = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00211:01\r\n");
    let wt = first_note(&with, 0).time_us;
    let ot = without.timelines.iter().flat_map(|t| t.notes[0].as_ref()).next().unwrap().time_us;
    assert_eq!(wt, ot, "an undefined BPM ref leaves timing unchanged");
    assert_eq!(wt, 4_000_000);
}

#[test]
fn section_rate_scales_measure_length() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00102:0.5\r\n#00211:01\r\n");
    assert_eq!(first_note(&m, 0).time_us, 3_000_000, "section rate 0.5 shortens measure 0");
}

#[test]
fn lnobj_does_not_convert_an_ln_channel_start() {
    let m = model(b"#LNOBJ ZZ\r\n#WAVZZ a.wav\r\n#00151:01\r\n#00211:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }), "the LN-channel head is untouched");
    assert!(matches!(kinds[1], NoteKind::Normal), "LNOBJ with no tracked Normal stays Normal, got {:?}", kinds[1]);
}

#[test]
fn lnobj_with_no_previous_note_stays_normal() {
    let m = model(b"#LNOBJ ZZ\r\n#WAVZZ a.wav\r\n#00111:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], NoteKind::Normal));
}

#[test]
fn lnobj_consumes_the_previous_note_only_once() {
    let m = model(b"#LNOBJ ZZ\r\n#WAV01 a.wav\r\n#WAVZZ b.wav\r\n#00111:01\r\n#00211:ZZ\r\n#00311:ZZ\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }));
    assert!(matches!(kinds[1], NoteKind::LongEnd { .. }));
    assert!(matches!(kinds[2], NoteKind::Normal), "the second LNOBJ has no Normal to convert");
    assert_eq!(count_playable_notes(&m), 2, "LongStart + trailing Normal are playable, LongEnd excluded");
}

#[test]
fn lnobj_conversion_preserves_playable_count() {
    let m = model(b"#LNOBJ ZZ\r\n#WAV01 a.wav\r\n#WAVZZ b.wav\r\n#00111:01\r\n#00211:ZZ\r\n");
    assert_eq!(count_playable_notes(&m), 1);
}

#[test]
fn ln_channel_second_open_after_close_is_a_new_start() {
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
    let m = model(b"#WAV01 a.wav\r\n#00151:01\r\n");
    let kinds: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], NoteKind::LongStart { .. }));
    assert_eq!(count_playable_notes(&m), 1);
}

#[test]
fn ln_open_close_is_independent_per_lane() {
    let m = model(b"#WAV01 a.wav\r\n#00151:01\r\n#00153:0101\r\n");
    let lane0: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.kind.clone()).collect();
    let lane2: Vec<NoteKind> = m.timelines.iter().flat_map(|tl| tl.notes[2].as_ref()).map(|n| n.kind.clone()).collect();
    assert!(matches!(lane0[0], NoteKind::LongStart { .. }), "lane 0 dangling head");
    assert!(matches!(lane2[0], NoteKind::LongStart { .. }));
    assert!(matches!(lane2[1], NoteKind::LongEnd { .. }), "lane 2 closes on its own toggle");
}

#[test]
fn count_playable_excludes_mines_and_long_ends() {
    let m = model(b"#WAVZZ a.wav\r\n#WAV01 a.wav\r\n#00011:01\r\n#00151:0101\r\n#001D3:ZZ\r\n");
    assert_eq!(count_playable_notes(&m), 2);
}

#[test]
fn count_playable_ignores_bgm_and_hidden() {
    let m = model(b"#WAV01 a.wav\r\n#00101:01\r\n#00131:01\r\n");
    assert_eq!(count_playable_notes(&m), 0, "BGM and hidden are not playable lane notes");
}

#[test]
fn count_playable_zero_for_mine_only_chart() {
    let m = model(b"#WAVZZ a.wav\r\n#001D1:ZZ\r\n#001D3:ZZ\r\n");
    assert_eq!(count_playable_notes(&m), 0);
}

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
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins, vec![1, 0]);
    assert_eq!(d.peak, 1.0);
    assert_eq!(d.avg, 0.5);
}

#[test]
fn note_density_mines_excluded_from_bins() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01\r\n#001D2:01\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins.iter().sum::<u32>(), 1, "only the Normal note contributes to the histogram");
    assert_eq!(d.peak, 1.0);
}

#[test]
fn note_density_long_note_body_fills_every_spanned_second() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00051:01000001\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins[0], 1, "head registers in its second");
    assert_eq!(d.bins[1], 1, "body fills the next spanned second");
    assert_eq!(d.peak, 1.0);
}

#[test]
fn note_density_negative_total_uses_default_formula() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01010101\r\n");
    let d0 = note_density(&m, 0.0);
    let dneg = note_density(&m, -42.0);
    assert_eq!(d0.end, dneg.end, "negative total falls back to the default total like zero does");
    assert_eq!(d0.avg, dneg.avg);
    assert_eq!(d0.bins, dneg.bins, "bins never depend on total at all");
}

#[test]
fn note_density_bins_independent_of_total_value() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01010101\r\n");
    let a = note_density(&m, 0.0).bins;
    let b = note_density(&m, 500.0).bins;
    assert_eq!(a, b);
}

#[test]
fn note_density_peak_is_max_bin() {
    let m = model(b"#BPM 240\r\n#WAV01 a.wav\r\n#00011:01010101\r\n#00111:01000000\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.peak, *d.bins.iter().max().unwrap() as f64);
    assert!(d.peak >= 4.0, "the busiest second holds at least the four measure-0 notes, got {}", d.peak);
}

#[test]
fn note_density_bins_sum_equals_playable_normal_plus_ln_body() {
    let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00011:01010101\r\n");
    let d = note_density(&m, 0.0);
    assert_eq!(d.bins.iter().sum::<u32>(), 4, "four plain notes => four histogram entries");
}

#[test]
fn wavmap_is_sparse_and_indexed_by_id() {
    let m = model(b"#WAV05 e.wav\r\n#WAV01 a.wav\r\n#00011:01\r\n");
    assert_eq!(m.wavmap.len(), 6, "max id 5 -> length 6");
    assert_eq!(m.wavmap[1], "a.wav");
    assert_eq!(m.wavmap[5], "e.wav");
    assert_eq!(m.wavmap[2], "", "unused ids are empty strings");
}

#[test]
fn empty_resource_map_has_one_slot() {
    let m = model(b"#00011:01\r\n");
    assert_eq!(m.wavmap.len(), 1);
    assert_eq!(m.wavmap[0], "");
}

#[test]
fn p2_channels_ignored_in_7k_keep_only_p1_note() {
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

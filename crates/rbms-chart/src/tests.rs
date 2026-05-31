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

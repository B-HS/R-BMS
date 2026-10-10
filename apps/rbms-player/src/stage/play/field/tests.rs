//! A chart made over for a skin's note field, against charts that change tempo, stop and hold.

use rbms_model::{Model, NoteKind};

use super::*;

/// A seven-key chart that opens at 240, halves its tempo in its second measure and stops there, with
/// a note at the start of each of its first three measures.
const TEMPO_CHART: &str = "#PLAYER 1\n#BPM 240\n#BPM01 120\n#STOP01 96\n#WAV01 a.wav\n#00011:01\n#00111:01\n#00108:01\n#00109:01\n#00211:01\n";

/// A seven-key chart with one long note in its first key, a plain note in its second and a mine in
/// its third.
const LONG_CHART: &str = "#PLAYER 1\n#BPM 120\n#LNTYPE 1\n#WAV01 a.wav\n#00151:0101\n#00112:01\n#001D3:01\n";

/// The first key's lane.
const FIRST_KEY: usize = 0;

fn model_of(chart: &str) -> Model {
    let src = rbms_parser::parse_with(chart.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "field.bms");
    rbms_chart::to_model(&src, mode)
}

/// With neither setting on the field is shown the chart itself, so nothing is made over.
#[test]
fn a_chart_is_not_made_over_with_both_settings_off() {
    let model = model_of(TEMPO_CHART);
    assert!(FieldChart::of(&model.timelines, false, false).is_none());
}

/// Under CONSTANT every timeline sits at the section its time alone gives it, at one tempo, with no
/// stop and no scroll rate of its own, so the field moves at one speed through a chart that does
/// not. The notes are the chart's own.
#[test]
fn constant_puts_every_timeline_where_its_time_alone_says() {
    let model = model_of(TEMPO_CHART);
    assert!(model.timelines.iter().any(|line| line.stop_us > 0), "the fixture stops");
    assert!(model.timelines.iter().any(|line| line.bpm != model.init_bpm), "the fixture changes tempo");

    let field = FieldChart::of(&model.timelines, true, false).expect("CONSTANT makes the chart over");
    assert_eq!(field.settings(), (true, false));
    assert_eq!(field.tempo(), Some((CONSTANT_BPM, CONSTANT_SCROLL)));
    assert_eq!(field.timelines().len(), model.timelines.len());
    for (shown, line) in field.timelines().iter().zip(&model.timelines) {
        assert_eq!(shown.time_us, line.time_us);
        assert_eq!(shown.section, line.time_us as f64 / CONSTANT_MEASURE_US, "a timeline at {} us is not where its time puts it", line.time_us);
        assert_eq!((shown.bpm, shown.stop_us, shown.scroll), (CONSTANT_BPM, 0, CONSTANT_SCROLL));
        assert_eq!(shown.section_line, line.section_line);
        let kinds = |line: &TimeLine| line.notes.iter().map(|note| note.as_ref().map(|note| note.kind.clone())).collect::<Vec<_>>();
        assert_eq!(kinds(shown), kinds(line), "CONSTANT changed a note");
    }
}

/// A note crosses a field under CONSTANT in the time the built-in field takes: two seconds at a
/// hi-speed of one, whatever the chart's tempo.
#[test]
fn a_constant_field_is_crossed_in_the_built_in_fields_time() {
    let region_ms = crate::skin_host::play::travel_region_ms(CONSTANT_BPM, 1.0, CONSTANT_SCROLL);
    assert_eq!(region_ms, CONSTANT_GREEN_BASE_MS);
    assert_eq!(region_ms, rbms_chart::scroll::constant_green_number(1.0, 0.0));
}

/// Under LEGACY NOTE the head of a long note is a plain note and its end is gone. Every other note,
/// and where every timeline sits, is the chart's own.
#[test]
fn legacy_note_leaves_the_head_of_a_long_note_as_a_plain_note() {
    let model = model_of(LONG_CHART);
    let kinds = |lines: &[TimeLine], lane: usize| lines.iter().filter_map(|line| line.notes[lane].as_ref().map(|note| note.kind.clone())).collect::<Vec<_>>();
    assert!(matches!(kinds(&model.timelines, FIRST_KEY)[..], [NoteKind::LongStart { .. }, NoteKind::LongEnd { .. }]), "the fixture holds a long note");

    let field = FieldChart::of(&model.timelines, false, true).expect("LEGACY NOTE makes the chart over");
    assert_eq!(field.settings(), (false, true));
    assert_eq!(field.tempo(), None, "a field made over for LEGACY NOTE alone scrolls as its chart does");
    assert_eq!(kinds(field.timelines(), FIRST_KEY), [NoteKind::Normal]);
    let head = |lines: &[TimeLine]| lines.iter().find_map(|line| line.notes[FIRST_KEY].as_ref().map(|note| (line.time_us, note.wav)));
    assert_eq!(head(field.timelines()), head(&model.timelines), "the plain note is not where the long note's head was, or plays another sound");
    for lane in FIRST_KEY + 1..model.mode.key {
        assert_eq!(kinds(field.timelines(), lane), kinds(&model.timelines, lane), "a note that is not a long note changed in lane {lane}");
    }
    for (shown, line) in field.timelines().iter().zip(&model.timelines) {
        assert_eq!((shown.section, shown.bpm, shown.stop_us, shown.scroll), (line.section, line.bpm, line.stop_us, line.scroll));
    }
}

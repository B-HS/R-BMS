use rbms_model::{LnKind, Mode, Note, NoteKind, default_total, default_total_for_mode};

use super::*;

const MINIMAL: &str = include_str!("../../tests/bmson/minimal.bmson");
const LONG_NOTE: &str = include_str!("../../tests/bmson/long-note.bmson");
const BPM_CHANGE: &str = include_str!("../../tests/bmson/bpm-change.bmson");
const STOP: &str = include_str!("../../tests/bmson/stop.bmson");
const CHANNELS: &str = include_str!("../../tests/bmson/channels.bmson");

/// One 4/4 measure at the 120 BPM every fixture starts at.
const MEASURE_US: i64 = 2_000_000;

fn chart(json: &str) -> BmsonChart {
    parse(json.as_bytes()).expect("fixture decodes")
}

fn times(model: &Model) -> Vec<i64> {
    model.timelines.iter().map(|tl| tl.time_us).collect()
}

fn lane_note(model: &Model, index: usize, lane: usize) -> &Note {
    model.timelines[index].notes[lane].as_ref().expect("a note in that lane")
}

/// A chart with one sound channel of `notes`, in `hint` mode, at the fixture tempo and resolution.
fn one_channel(hint: &str, notes: &str) -> BmsonChart {
    chart(&format!(r#"{{"info":{{"init_bpm":120,"resolution":240,"mode_hint":"{hint}"}},"sound_channels":[{{"name":"a.ogg","notes":[{notes}]}}]}}"#))
}

#[test]
fn minimal_info_is_decoded_field_for_field() {
    let c = chart(MINIMAL);
    assert_eq!(c.version, "1.0.0");
    assert_eq!(c.info.title, "Minimal");
    assert_eq!(c.info.subtitle, "sub");
    assert_eq!(c.info.chart_name, "NORMAL");
    assert_eq!(c.info.artist, "rbms");
    assert_eq!(c.info.subartists, vec!["one".to_owned(), "two".to_owned()]);
    assert_eq!(c.info.genre, "test");
    assert_eq!(c.info.mode_hint, "beat-7k");
    assert_eq!(c.info.judge_rank, 100);
    assert_eq!(c.info.total, 100.0);
    assert_eq!(c.info.init_bpm, 120.0);
    assert_eq!(c.info.level, 5);
    assert_eq!(c.info.resolution, 240);
    assert_eq!(c.info.back_image, "back.png");
    assert_eq!(c.info.eyecatch_image, "eye.png");
    assert_eq!(c.info.banner_image, "banner.png");
    assert_eq!(c.info.preview_music, "preview.ogg");
    assert_eq!(c.lines, vec![BarLine { y: 0 }, BarLine { y: 960 }]);
    assert_eq!(c.sound_channels.len(), 1);
    assert_eq!(c.sound_channels[0].name, "kick.wav");
    assert_eq!(c.sound_channels[0].notes.len(), 3);
}

#[test]
fn absent_arrays_decode_empty_and_absent_objects_decode_default() {
    let c = chart(MINIMAL);
    assert!(c.bpm_events.is_empty());
    assert!(c.stop_events.is_empty());
    assert!(c.scroll_events.is_empty());
    assert!(c.key_channels.is_empty());
    assert!(c.mine_channels.is_empty());
    assert_eq!(c.bga, Bga::default());
}

#[test]
fn absent_info_fields_take_the_format_defaults() {
    let c = one_channel("beat-7k", "");
    assert_eq!(c.info.judge_rank, DEFAULT_JUDGE_RANK);
    assert_eq!(c.info.total, 100.0);
    assert_eq!(c.info.resolution, DEFAULT_RESOLUTION);
    assert_eq!(c.info.ln_type, 0);
    assert_eq!(c.info.level, 0);
    assert!(c.info.title.is_empty());
    assert!(c.info.subartists.is_empty());
}

#[test]
fn note_flags_default_to_a_plain_note() {
    let c = one_channel("beat-7k", r#"{"x":1,"y":0}"#);
    assert_eq!(c.sound_channels[0].notes[0], BmsonNote { x: 1, y: 0, l: 0, c: false, t: 0, up: false });
}

#[test]
fn hashes_are_of_the_file_bytes() {
    let c = chart(MINIMAL);
    assert_eq!(c.md5, "ae8ae719abd25371124fd5eb6756fa8a");
    assert_eq!(c.sha256, "e79b84e27e35aecad52b711efa59af084515e93f089677da862a599628c6d348");
    assert_eq!(c.md5, crate::parse(MINIMAL.as_bytes()).md5, "both formats identify a file by the same bytes");
    assert_eq!(c.sha256, crate::parse(MINIMAL.as_bytes()).sha256);
}

#[test]
fn a_byte_order_mark_is_skipped_but_still_hashed() {
    let mut bytes = UTF8_BOM.to_vec();
    bytes.extend_from_slice(MINIMAL.as_bytes());
    let c = parse(&bytes).expect("a BOM does not stop the decode");
    assert_eq!(c.info.title, "Minimal");
    assert_ne!(c.sha256, chart(MINIMAL).sha256, "the BOM is part of the file");
}

#[test]
fn every_mode_hint_maps_to_its_mode_and_the_rest_fall_back() {
    for (hint, mode) in MODE_HINTS {
        assert_eq!(one_channel(hint, "").mode(), mode, "hint {hint}");
    }
    for hint in ["", "popn-5k", "keyboard-24k-double", "beat-9k", "BEAT-7K"] {
        assert_eq!(one_channel(hint, "").mode(), Mode::BEAT_7K, "hint {hint}");
    }
}

#[test]
fn resolution_is_four_quarter_notes_to_a_measure() {
    assert_eq!(chart(MINIMAL).pulses_per_measure(), 960.0);
    assert_eq!(one_channel("beat-7k", "").pulses_per_measure(), 960.0);
    for (resolution, expected) in [(1, 4.0), (48, 192.0), (480, 1920.0), (0, 960.0), (-4, 960.0)] {
        let c = chart(&format!(r#"{{"info":{{"init_bpm":120,"resolution":{resolution}}}}}"#));
        assert_eq!(c.pulses_per_measure(), expected, "resolution {resolution}");
    }
}

#[test]
fn judge_rank_below_five_is_a_rank_index_and_above_it_a_percentage() {
    let rank_of = |judge_rank: i64| chart(&format!(r#"{{"info":{{"init_bpm":120,"judge_rank":{judge_rank}}}}}"#)).rank();
    assert_eq!(rank_of(-1), NORMAL_RANK, "a negative judge_rank leaves the model at NORMAL");
    for index in 0..BMSON_JUDGERANK_MIN {
        assert_eq!(rank_of(index), index as i32, "judge_rank {index} is a #RANK index");
    }
    for percent in [BMSON_JUDGERANK_MIN, 25, 75, 100, 125, 200] {
        assert_eq!(rank_of(percent), PERCENT_FALLBACK_RANK, "judge_rank {percent} is a percentage");
    }
}

#[test]
fn judgerank_percent_is_reported_only_for_a_percentage() {
    let percent_of = |judge_rank: i64| chart(&format!(r#"{{"info":{{"init_bpm":120,"judge_rank":{judge_rank}}}}}"#)).judgerank_percent();
    assert_eq!(percent_of(-1), None);
    assert_eq!(percent_of(0), None);
    assert_eq!(percent_of(4), None);
    assert_eq!(percent_of(5), Some(5));
    assert_eq!(percent_of(100), Some(100));
    assert_eq!(chart(MINIMAL).judgerank_percent(), Some(100));
}

#[test]
fn total_is_a_percentage_of_the_mode_default() {
    let total_of = |total: &str| chart(&format!(r#"{{"info":{{"init_bpm":120,"total":{total}}}}}"#));
    let notes = 1_000;
    let base = default_total(notes);
    assert!(base > 260.0, "a chart this dense is past the floor");
    assert_eq!(total_of("100").total_for_notes(&Mode::BEAT_7K, notes), base);
    assert_eq!(total_of("150").total_for_notes(&Mode::BEAT_7K, notes), 1.5 * base);
    assert_eq!(total_of("80").total_for_notes(&Mode::BEAT_7K, notes), 0.8 * base);
    for absent_or_zero in ["0", "-20"] {
        assert_eq!(total_of(absent_or_zero).total_for_notes(&Mode::BEAT_7K, notes), base, "total {absent_or_zero}");
    }
    assert_eq!(
        total_of("100").total_for_notes(&Mode::KEYBOARD_24K, notes),
        default_total_for_mode(&Mode::KEYBOARD_24K, notes),
        "the percentage scales the mode's own default"
    );
}

#[test]
fn minimal_maps_to_one_timeline_per_pulse() {
    let model = chart(MINIMAL).to_model();
    assert_eq!(model.mode, Mode::BEAT_7K);
    assert_eq!(times(&model), vec![0, MEASURE_US / 4, MEASURE_US / 2, MEASURE_US]);
    assert_eq!(model.timelines.iter().map(|tl| tl.section).collect::<Vec<_>>(), vec![0.0, 0.25, 0.5, 1.0]);
    assert_eq!(model.timelines.iter().map(|tl| tl.bpm).collect::<Vec<_>>(), vec![120.0; 4]);
    assert_eq!(model.timelines.iter().map(|tl| tl.section_line).collect::<Vec<_>>(), vec![true, false, false, true]);
    assert_eq!(model.init_bpm, 120.0);
}

#[test]
fn minimal_places_notes_by_lane_and_sends_a_lane_less_note_to_the_background() {
    let model = chart(MINIMAL).to_model();
    assert_eq!(lane_note(&model, 0, 0).kind, NoteKind::Normal);
    assert_eq!(lane_note(&model, 0, 0).time_us, 0);
    assert_eq!(lane_note(&model, 2, 7).time_us, MEASURE_US / 2, "x 8 is the scratch lane of a 7 key chart");
    assert!(model.timelines[1].notes.iter().all(Option::is_none), "the note without an x is not played");
    assert_eq!(model.timelines[1].bgnotes.len(), 1);
    assert_eq!(model.timelines[1].bgnotes[0].time_us, MEASURE_US / 4);
    assert_eq!(model.wavmap, vec!["kick.wav".to_owned()]);
}

#[test]
fn minimal_carries_the_document_metadata_into_the_model() {
    let model = chart(MINIMAL).to_model();
    assert_eq!(model.meta.title, "Minimal");
    assert_eq!(model.meta.subtitle, "sub NORMAL", "chart_name follows the subtitle");
    assert_eq!(model.meta.artist, "rbms");
    assert_eq!(model.meta.subartist, "one,two");
    assert_eq!(model.meta.genre, "test");
    assert_eq!(model.meta.play_level, "5");
    assert_eq!(model.meta.stagefile, "eye.png");
    assert_eq!(model.meta.rank, PERCENT_FALLBACK_RANK);
    assert_eq!(model.meta.defexrank, None);
    assert_eq!(model.meta.volwav, rbms_model::VOLWAV_DEFAULT_PERCENT);
    assert_eq!(model.meta.total, default_total(2), "two playable notes at 100 %");
    assert_eq!(model.md5, chart(MINIMAL).md5);
}

#[test]
fn subtitle_joins_only_when_both_halves_are_present() {
    let subtitle_of = |subtitle: &str, chart_name: &str| {
        chart(&format!(r#"{{"info":{{"init_bpm":120,"subtitle":"{subtitle}","chart_name":"{chart_name}"}}}}"#)).to_model().meta.subtitle
    };
    assert_eq!(subtitle_of("sub", "NORMAL"), "sub NORMAL");
    assert_eq!(subtitle_of("", "NORMAL"), "NORMAL");
    assert_eq!(subtitle_of("sub", ""), "sub");
    assert_eq!(subtitle_of("", ""), "");
}

#[test]
fn a_long_note_spans_from_its_pulse_to_its_length() {
    let model = chart(LONG_NOTE).to_model();
    assert_eq!(times(&model), vec![0, MEASURE_US / 4, MEASURE_US / 2, MEASURE_US]);
    assert_eq!(lane_note(&model, 0, 1).kind, NoteKind::LongStart { ln: LnKind::Cn });
    assert_eq!(lane_note(&model, 3, 1).kind, NoteKind::LongEnd { ln: LnKind::Cn });
    assert_eq!(lane_note(&model, 3, 1).time_us, MEASURE_US);
    assert!(model.timelines[1].notes[1].is_none(), "nothing is placed between head and tail");
    assert!(model.timelines[2].notes[1].is_none());
}

#[test]
fn a_note_type_overrides_the_chart_wide_long_note_flavour() {
    let model = chart(LONG_NOTE).to_model();
    assert_eq!(lane_note(&model, 0, 3).kind, NoteKind::LongStart { ln: LnKind::Hcn }, "t 3 wins over ln_type 2");
    assert_eq!(lane_note(&model, 1, 3).kind, NoteKind::LongEnd { ln: LnKind::Hcn });
    assert_eq!(lane_note(&model, 0, 2).kind, NoteKind::LongStart { ln: LnKind::Cn }, "no t leaves ln_type 2");
}

#[test]
fn an_unstated_long_note_flavour_is_left_for_the_player_setting() {
    let model = one_channel("beat-7k", r#"{"x":1,"y":0,"l":960}"#).to_model();
    assert_eq!(lane_note(&model, 0, 0).kind, NoteKind::LongStart { ln: LnKind::Undefined });
    assert_eq!(lane_note(&model, 1, 0).kind, NoteKind::LongEnd { ln: LnKind::Undefined });
}

#[test]
fn an_up_note_gives_the_long_note_tail_its_keysound() {
    let model = chart(LONG_NOTE).to_model();
    assert_eq!(model.wavmap, vec!["ln.ogg".to_owned(), "up.ogg".to_owned(), "ln2.ogg".to_owned(), "hcn.ogg".to_owned()]);
    assert_eq!(lane_note(&model, 3, 1).wav, 1, "the up note in a later position closes the open long note");
    assert_eq!(lane_note(&model, 2, 2).wav, 1, "an up note read before its long note is kept until the head arrives");
    assert_eq!(lane_note(&model, 0, 1).wav, 0, "the head keeps its own channel");
    assert_eq!(lane_note(&model, 0, 2).wav, 2);
}

#[test]
fn a_long_note_tail_no_up_note_closes_is_silent() {
    let model = chart(LONG_NOTE).to_model();
    assert_eq!(lane_note(&model, 1, 3).wav, SILENT_WAV);
    assert!(usize::try_from(SILENT_WAV).is_err(), "a silent tail indexes no keysound");
}

#[test]
fn charge_note_tails_count_towards_the_default_total() {
    let model = chart(LONG_NOTE).to_model();
    assert_eq!(model.meta.total, default_total(6), "three heads and three charge or hell charge tails");
}

#[test]
fn a_long_note_tail_stays_out_of_the_note_count_when_the_flavour_is_unstated() {
    let model = one_channel("beat-7k", r#"{"x":1,"y":0,"l":960}"#).to_model();
    assert_eq!(model.meta.total, default_total(1), "only the head is judged until LN MODE resolves the flavour");
}

#[test]
fn a_bpm_event_changes_the_pulse_to_time_rate_from_its_pulse_on() {
    let model = chart(BPM_CHANGE).to_model();
    assert_eq!(times(&model), vec![0, MEASURE_US, MEASURE_US + MEASURE_US / 2]);
    assert_eq!(model.timelines.iter().map(|tl| tl.bpm).collect::<Vec<_>>(), vec![120.0, 240.0, 240.0]);
}

#[test]
fn a_non_positive_bpm_event_is_ignored() {
    let c = chart(
        r#"{"info":{"init_bpm":120,"resolution":240},"bpm_events":[{"y":960,"bpm":0},{"y":1920,"bpm":-60}],
            "sound_channels":[{"name":"a.ogg","notes":[{"x":1,"y":0},{"x":1,"y":960},{"x":1,"y":1920}]}]}"#,
    );
    let model = c.to_model();
    assert_eq!(model.timelines.iter().map(|tl| tl.bpm).collect::<Vec<_>>(), vec![120.0; 3]);
    assert_eq!(times(&model), vec![0, MEASURE_US, 2 * MEASURE_US]);
}

#[test]
fn a_stop_holds_the_chart_for_its_length_in_pulses() {
    let model = chart(STOP).to_model();
    assert_eq!(model.timelines[1].stop_us, MEASURE_US, "960 pulses of 960 is one measure at 120 BPM");
    assert_eq!(times(&model), vec![0, MEASURE_US, 3 * MEASURE_US]);
    assert_eq!(model.timelines[0].stop_us, 0);
    assert_eq!(model.timelines[2].stop_us, 0);
}

#[test]
fn a_stop_is_measured_at_the_tempo_it_stops_at() {
    let c = chart(
        r#"{"info":{"init_bpm":120,"resolution":240},"bpm_events":[{"y":960,"bpm":240}],"stop_events":[{"y":960,"duration":960}],
            "sound_channels":[{"name":"a.ogg","notes":[{"x":1,"y":0},{"x":1,"y":960}]}]}"#,
    );
    let model = c.to_model();
    assert_eq!(model.timelines[1].stop_us, MEASURE_US / 2, "the tempo at the stop is the new one");
}

#[test]
fn a_negative_stop_is_ignored() {
    let c = chart(
        r#"{"info":{"init_bpm":120,"resolution":240},"stop_events":[{"y":960,"duration":-960}],
            "sound_channels":[{"name":"a.ogg","notes":[{"x":1,"y":0},{"x":1,"y":960},{"x":1,"y":1920}]}]}"#,
    );
    let model = c.to_model();
    assert_eq!(model.timelines[1].stop_us, 0);
    assert_eq!(times(&model), vec![0, MEASURE_US, 2 * MEASURE_US]);
}

#[test]
fn a_pulse_becomes_the_time_the_reference_puts_it_at() {
    let c = chart(
        r#"{"info":{"init_bpm":137,"resolution":48},"stop_events":[{"y":192,"duration":96}],
            "sound_channels":[{"name":"a.ogg","notes":[{"x":1,"y":0},{"x":1,"y":192}]}]}"#,
    );
    let model = c.to_model();
    assert_eq!(model.timelines[1].time_us, 1_751_824, "one measure at 137 BPM, truncated once");
    assert_eq!(model.timelines[1].stop_us, 875_912, "half a measure of stop at 137 BPM");
}

#[test]
fn time_does_not_drift_over_a_long_chart() {
    let notes: Vec<String> = (0..1_000).map(|i| format!(r#"{{"x":1,"y":{}}}"#, i * 24)).collect();
    let c = chart(&format!(r#"{{"info":{{"init_bpm":137,"resolution":48}},"sound_channels":[{{"name":"a.ogg","notes":[{}]}}]}}"#, notes.join(",")));
    let model = c.to_model();
    assert_eq!(model.timelines.len(), 1_000);
    assert_eq!(model.timelines[999].time_us, 218_759_124, "999 steps of 24 pulses at 137 BPM");
    assert_ne!(model.timelines[999].time_us, 999 * 218_978, "truncating every step instead would lose a tenth of a microsecond each time");
}

#[test]
fn a_continued_note_plays_the_next_slice_of_its_channel() {
    let model = chart(CHANNELS).to_model();
    let slices: Vec<(i64, i64)> = model.timelines.iter().flat_map(|tl| tl.bgnotes.iter()).map(|n| (n.start_us, n.duration_us)).collect();
    assert_eq!(slices, vec![(0, MEASURE_US / 2), (MEASURE_US / 2, MEASURE_US / 2), (MEASURE_US, 0)]);
}

#[test]
fn a_note_that_starts_a_channel_restarts_its_keysound() {
    let model = one_channel("beat-7k", r#"{"x":1,"y":0},{"x":1,"y":480,"c":true},{"x":1,"y":960}"#).to_model();
    assert_eq!((lane_note(&model, 0, 0).start_us, lane_note(&model, 0, 0).duration_us), (0, MEASURE_US / 2));
    assert_eq!((lane_note(&model, 1, 0).start_us, lane_note(&model, 1, 0).duration_us), (MEASURE_US / 2, 0));
    assert_eq!((lane_note(&model, 2, 0).start_us, lane_note(&model, 2, 0).duration_us), (0, 0), "c false starts the file over");
}

#[test]
fn two_notes_on_one_lane_and_pulse_layer_their_keysounds() {
    let model = chart(CHANNELS).to_model();
    let note = lane_note(&model, 0, 0);
    assert_eq!(note.wav, 1);
    assert_eq!(note.layered.len(), 1, "the second note of the same channel layers onto the first");
    assert_eq!(note.layered[0].wav, 1);
}

#[test]
fn key_and_mine_channels_follow_the_sound_channels_in_the_keysound_list() {
    let model = chart(CHANNELS).to_model();
    assert_eq!(model.wavmap, vec!["loop.ogg".to_owned(), "key.ogg".to_owned(), "hidden.ogg".to_owned(), "mine.ogg".to_owned()]);
    let hidden = model.timelines[1].hidden[1].as_ref().expect("an invisible note");
    assert_eq!(hidden.wav, 2);
    assert_eq!(hidden.time_us, MEASURE_US / 4);
    let mine = lane_note(&model, 1, 2);
    assert_eq!(mine.kind, NoteKind::Mine { damage: 5.5 });
    assert_eq!(mine.wav, 3);
}

#[test]
fn a_mine_is_not_a_playable_note() {
    let model = chart(CHANNELS).to_model();
    assert_eq!(model.meta.total, 1.5 * default_total(1), "one playable note at 150 %");
}

#[test]
fn bga_events_index_the_header_list_and_a_layer_event_is_the_layer_picture() {
    let model = chart(CHANNELS).to_model();
    assert_eq!(model.bgamap, vec!["a.png".to_owned(), "b.png".to_owned()]);
    assert_eq!(model.timelines[0].bga, 1, "id 9 is the second header entry");
    assert_eq!(model.timelines[2].layer, 0);
    assert_eq!(model.timelines[1].bga, -1, "a timeline with no event keeps no picture");
    assert_eq!(model.timelines[3].bga, -1, "poor events have no place in the model");
}

#[test]
fn a_scroll_event_holds_until_the_next_one() {
    let model = chart(CHANNELS).to_model();
    assert_eq!(model.timelines.iter().map(|tl| tl.scroll).collect::<Vec<_>>(), vec![1.0, 1.0, 2.0, 2.0]);
}

#[test]
fn five_key_lanes_put_the_scratch_where_the_mode_does() {
    let model = one_channel("beat-5k", r#"{"x":1,"y":0},{"x":5,"y":240},{"x":8,"y":480},{"x":6,"y":720}"#).to_model();
    assert_eq!(model.mode, Mode::BEAT_5K);
    assert_eq!(lane_note(&model, 0, 0).time_us, 0);
    assert_eq!(lane_note(&model, 1, 4).time_us, MEASURE_US / 4);
    assert_eq!(lane_note(&model, 2, 5).time_us, MEASURE_US / 2, "x 8 is the scratch of a 5 key chart");
    assert!(model.timelines[3].notes.iter().all(Option::is_none), "x 6 has no lane in a 5 key chart");
    assert_eq!(model.timelines[3].bgnotes.len(), 1);
}

#[test]
fn ten_key_lanes_split_the_two_players() {
    let model = one_channel("beat-10k", r#"{"x":1,"y":0},{"x":8,"y":240},{"x":9,"y":480},{"x":16,"y":720},{"x":14,"y":960}"#).to_model();
    assert_eq!(model.mode, Mode::BEAT_10K);
    assert_eq!(lane_note(&model, 0, 0).time_us, 0);
    assert_eq!(lane_note(&model, 1, 5).time_us, MEASURE_US / 4, "x 8 is the first player scratch");
    assert_eq!(lane_note(&model, 2, 6).time_us, MEASURE_US / 2);
    assert_eq!(lane_note(&model, 3, 11).time_us, 3 * MEASURE_US / 4, "x 16 is the second player scratch");
    assert!(model.timelines[4].notes.iter().all(Option::is_none), "x 14 has no lane in a 10 key chart");
}

#[test]
fn fourteen_key_and_popn_lanes_are_the_x_position_itself() {
    let model = one_channel("beat-14k", r#"{"x":1,"y":0},{"x":16,"y":240}"#).to_model();
    assert_eq!(lane_note(&model, 0, 0).time_us, 0);
    assert_eq!(lane_note(&model, 1, 15).time_us, MEASURE_US / 4);

    let popn = one_channel("popn-9k", r#"{"x":1,"y":0},{"x":9,"y":240},{"x":10,"y":480}"#).to_model();
    assert_eq!(popn.mode, Mode::POPN_9K);
    assert_eq!(lane_note(&popn, 0, 0).time_us, 0);
    assert_eq!(lane_note(&popn, 1, 8).time_us, MEASURE_US / 4);
    assert!(popn.timelines[2].notes.iter().all(Option::is_none), "x 10 is past the lanes of a 9 key chart");
}

#[test]
fn a_note_inside_an_open_long_note_is_pushed_to_the_background() {
    let model = one_channel("beat-7k", r#"{"x":1,"y":0,"l":960},{"x":1,"y":480}"#).to_model();
    assert_eq!(lane_note(&model, 0, 0).kind, NoteKind::LongStart { ln: LnKind::Undefined });
    assert!(model.timelines[1].notes[0].is_none(), "the overlapping note is not played");
    assert_eq!(model.timelines[1].bgnotes.len(), 1);
}

#[test]
fn a_long_note_over_an_occupied_lane_is_pushed_to_the_background() {
    let c = chart(
        r#"{"info":{"init_bpm":120,"resolution":240},"sound_channels":[
            {"name":"a.ogg","notes":[{"x":1,"y":480}]},
            {"name":"b.ogg","notes":[{"x":1,"y":0,"l":960}]}]}"#,
    );
    let model = c.to_model();
    assert_eq!(lane_note(&model, 1, 0).kind, NoteKind::Normal, "the note already in the span stays");
    assert_eq!(lane_note(&model, 1, 0).wav, 0);
    assert!(model.timelines[0].notes[0].is_none(), "the long note is not played");
    assert_eq!(model.timelines[0].bgnotes.len(), 1);
    assert!(model.timelines[2].notes[0].is_none(), "and no tail is left behind");
}

#[test]
fn two_long_notes_over_the_same_span_layer_their_keysounds() {
    let c = chart(
        r#"{"info":{"init_bpm":120,"resolution":240,"ln_type":1},"sound_channels":[
            {"name":"a.ogg","notes":[{"x":1,"y":0,"l":960}]},
            {"name":"b.ogg","notes":[{"x":1,"y":0,"l":960}]}]}"#,
    );
    let model = c.to_model();
    let head = lane_note(&model, 0, 0);
    assert_eq!(head.kind, NoteKind::LongStart { ln: LnKind::Ln });
    assert_eq!(head.wav, 0);
    assert_eq!(head.layered.len(), 1);
    assert_eq!(head.layered[0].wav, 1);
    assert_eq!(lane_note(&model, 1, 0).kind, NoteKind::LongEnd { ln: LnKind::Ln });
}

#[test]
fn a_long_note_starting_on_an_occupied_lane_is_dropped() {
    let c = chart(
        r#"{"info":{"init_bpm":120,"resolution":240},"sound_channels":[
            {"name":"a.ogg","notes":[{"x":1,"y":0}]},
            {"name":"b.ogg","notes":[{"x":1,"y":0,"l":960}]}]}"#,
    );
    let model = c.to_model();
    assert_eq!(lane_note(&model, 0, 0).kind, NoteKind::Normal);
    assert!(lane_note(&model, 0, 0).layered.is_empty(), "a long note does not layer onto a plain note");
    assert!(model.timelines[1].notes[0].is_none());
    assert!(model.timelines[0].bgnotes.is_empty());
}

#[test]
fn a_zero_or_negative_length_is_a_plain_note() {
    let model = one_channel("beat-7k", r#"{"x":1,"y":0,"l":0},{"x":2,"y":240,"l":-960}"#).to_model();
    assert_eq!(lane_note(&model, 0, 0).kind, NoteKind::Normal);
    assert_eq!(lane_note(&model, 1, 1).kind, NoteKind::Normal);
}

#[test]
fn notes_are_read_in_pulse_order_whatever_order_the_channel_lists_them_in() {
    let ordered = one_channel("beat-7k", r#"{"x":1,"y":0},{"x":1,"y":480,"c":true}"#).to_model();
    let shuffled = one_channel("beat-7k", r#"{"x":1,"y":480,"c":true},{"x":1,"y":0}"#).to_model();
    assert_eq!(lane_note(&ordered, 0, 0).duration_us, lane_note(&shuffled, 0, 0).duration_us);
    assert_eq!(lane_note(&ordered, 1, 0).start_us, lane_note(&shuffled, 1, 0).start_us);
}

#[test]
fn a_chart_with_nothing_on_it_still_starts_at_pulse_zero() {
    let model = chart(r#"{"info":{"init_bpm":150}}"#).to_model();
    assert_eq!(times(&model), vec![0]);
    assert_eq!(model.timelines[0].bpm, 150.0);
    assert_eq!(model.init_bpm, 150.0);
    assert!(model.wavmap.is_empty());
}

#[test]
fn a_forced_mode_overrides_the_hint() {
    let model = chart(MINIMAL).to_model_in_mode(Mode::BEAT_5K);
    assert_eq!(model.mode, Mode::BEAT_5K);
    assert_eq!(model.timelines[0].notes.len(), Mode::BEAT_5K.key);
    assert_eq!(lane_note(&model, 2, 5).time_us, MEASURE_US / 2, "x 8 follows the forced mode's lane table");
}

#[test]
fn malformed_documents_are_rejected_with_the_reason() {
    assert!(matches!(parse(b"{"), Err(BmsonError::Json(_))), "truncated JSON");
    assert!(matches!(parse(b""), Err(BmsonError::Json(_))), "no JSON at all");
    assert!(matches!(parse(&[0xFF, 0xFE]), Err(BmsonError::Json(_))), "not UTF-8");
    assert!(matches!(parse(b"[]"), Err(BmsonError::NotAnObject)));
    assert!(matches!(parse(b"\"bmson\""), Err(BmsonError::NotAnObject)));
    assert!(matches!(parse(br#"{"version":"1.0.0"}"#), Err(BmsonError::MissingInfo)));
    assert!(matches!(parse(br#"{"info":null}"#), Err(BmsonError::MissingInfo)));
    assert!(matches!(parse(br#"{"info":[]}"#), Err(BmsonError::InvalidField { field: "info" })));
}

#[test]
fn a_chart_without_a_usable_tempo_is_rejected() {
    for bpm in ["0", "-120"] {
        let json = format!(r#"{{"info":{{"init_bpm":{bpm}}}}}"#);
        assert!(matches!(parse(json.as_bytes()), Err(BmsonError::InvalidInitBpm { .. })), "init_bpm {bpm}");
    }
    assert!(matches!(parse(br#"{"info":{"title":"no tempo"}}"#), Err(BmsonError::InvalidInitBpm { bpm: 0.0 })));
}

#[test]
fn a_field_of_the_wrong_json_type_is_rejected_by_name() {
    let cases: [(&str, &str); 8] = [
        (r#"{"info":{"init_bpm":"120"}}"#, "init_bpm"),
        (r#"{"info":{"init_bpm":120,"resolution":"240"}}"#, "resolution"),
        (r#"{"info":{"init_bpm":120,"title":7}}"#, "title"),
        (r#"{"info":{"init_bpm":120,"subartists":[7]}}"#, "subartists"),
        (r#"{"info":{"init_bpm":120},"sound_channels":{}}"#, "sound_channels"),
        (r#"{"info":{"init_bpm":120},"sound_channels":[7]}"#, "sound_channels"),
        (r#"{"info":{"init_bpm":120},"sound_channels":[{"name":"a.ogg","notes":[{"y":"0"}]}]}"#, "y"),
        (r#"{"info":{"init_bpm":120},"sound_channels":[{"name":"a.ogg","notes":[{"y":0,"c":"yes"}]}]}"#, "c"),
    ];
    for (json, field) in cases {
        match parse(json.as_bytes()) {
            Err(BmsonError::InvalidField { field: reported }) => assert_eq!(reported, field, "{json}"),
            other => panic!("{json} should name {field}, got {other:?}"),
        }
    }
}

#[test]
fn a_negative_pulse_is_rejected() {
    let cases = [
        r#"{"info":{"init_bpm":120},"sound_channels":[{"name":"a.ogg","notes":[{"x":1,"y":-1}]}]}"#,
        r#"{"info":{"init_bpm":120},"bpm_events":[{"y":-1,"bpm":150}]}"#,
        r#"{"info":{"init_bpm":120},"stop_events":[{"y":-1,"duration":10}]}"#,
        r#"{"info":{"init_bpm":120},"lines":[{"y":-1}]}"#,
        r#"{"info":{"init_bpm":120},"mine_channels":[{"name":"m.ogg","notes":[{"x":1,"y":-1}]}]}"#,
    ];
    for json in cases {
        assert!(matches!(parse(json.as_bytes()), Err(BmsonError::InvalidField { field: "y" })), "{json}");
    }
}

#[test]
fn unknown_fields_are_ignored() {
    let c = chart(r#"{"info":{"init_bpm":120,"future_field":{"a":[1,2]}},"unknown_top_level":[1],"sound_channels":[]}"#);
    assert_eq!(c.info.init_bpm, 120.0);
    assert!(c.sound_channels.is_empty());
}

#[test]
fn an_integral_float_is_accepted_where_the_format_states_an_integer() {
    let c = chart(r#"{"info":{"init_bpm":120,"resolution":240.0,"level":5.0},"sound_channels":[{"name":"a.ogg","notes":[{"x":1.0,"y":480.0}]}]}"#);
    assert_eq!(c.info.resolution, 240);
    assert_eq!(c.info.level, 5);
    assert_eq!(c.sound_channels[0].notes[0], BmsonNote { x: 1, y: 480, l: 0, c: false, t: 0, up: false });
}

#[test]
fn the_error_reason_reads_back() {
    let json = parse(b"{").expect_err("truncated");
    assert!(std::error::Error::source(&json).is_some(), "the JSON reason is kept");
    assert!(json.to_string().contains("valid JSON"));
    assert!(parse(b"[]").expect_err("array").to_string().contains("object"));
    assert!(parse(br#"{"info":{}}"#).expect_err("no tempo").to_string().contains("init_bpm"));
    assert!(parse(br#"{"info":{"init_bpm":120,"title":1}}"#).expect_err("bad title").to_string().contains("title"));
}

#[test]
fn the_extension_is_the_one_the_scanner_filters_on() {
    assert_eq!(EXTENSION, "bmson");
    assert_eq!(std::path::Path::new("song.bmson").extension().and_then(|e| e.to_str()), Some(EXTENSION));
}

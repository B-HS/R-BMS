//! What a chart's note count means. The song database, the `#TOTAL` fallback and the result screen's
//! denominator count a decoded model with `rbms_chart::count_playable_notes`; the judge engine counts
//! the notes it builds from the same model. The decoder settles overlapping and unpaired long notes
//! the way the reference implementation does, so the two agree on every chart, and an autoplayed run
//! judges each of them once, as a PGREAT.

use rbms_judge::JudgeEngine;
use rbms_judge::ln::LnMode;
use rbms_model::{LnKind, Mode, Model, NoteKind};
use rbms_play::{JudgeSetup, NullSink, PlaySession, SessionClock, SessionOptions};

/// How finely the autoplay run steps the song clock.
const STEP_US: i64 = 16_667;

/// How long past the last note the run keeps stepping, so a note left unjudged would be swept.
const TAIL_US: i64 = 3_000_000;

/// One lane of each way a chart can put something inside or beside a long note, under `#LNOBJ ZZ`:
/// a normal note and a mine in the body (lane 0), a head that is never closed with notes after it
/// (lane 1), a third object that has no partner (lane 2), a head on the previous tail's tick
/// (lane 3) and an lnobj pair inside a long note (lane 4).
const OVERLAPS: &str = "#BPM 150\r\n#TOTAL 300\r\n#LNOBJ ZZ\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#WAVZZ c.wav\r\n\
    #00151:01\r\n#00211:02\r\n#002D1:ZZ\r\n#00351:01\r\n\
    #00152:01\r\n#00212:02\r\n#00312:02\r\n\
    #00153:01\r\n#00253:01\r\n#00353:01\r\n\
    #00154:01\r\n#00254:01\r\n#00254:01\r\n#00354:01\r\n\
    #00155:01\r\n#00215:01\r\n#00315:ZZ\r\n#00455:01\r\n";

/// Plain notes before and after the overlaps in lanes 0 and 2, four in the first and two in the other.
const WITH_NEIGHBOURS: &str = "#00011:0101\r\n#00511:0101\r\n#00013:01\r\n#00513:01\r\n";

/// The notes of [`OVERLAPS`] that survive the decoder: one long note in each of lanes 0, 2, 3 and 4,
/// and the two normal notes lane 1 keeps because nothing closes the head before them.
const LONG_NOTES_KEPT: usize = 4;
const OVERLAP_PLAIN_NOTES_KEPT: usize = 2;
const NEIGHBOUR_NOTES: usize = 6;

fn chart(lnmode: u8) -> Model {
    let text = format!("#LNMODE {lnmode}\r\n{OVERLAPS}{WITH_NEIGHBOURS}");
    rbms_chart::to_model(&rbms_parser::parse(text.as_bytes()), Mode::BEAT_7K)
}

fn autoplay(model: Model, ln_mode: LnMode) -> rbms_play::PlaySummary {
    let mut session = PlaySession::new(model, SessionOptions { autoplay: true, ..SessionOptions::default() });
    session.set_judge_setup(JudgeSetup { ln_mode, ..JudgeSetup::default() });
    let end = session.last_time_us() + TAIL_US;
    let mut now = 0;
    while now <= end {
        session.tick(SessionClock::at(now), &mut NullSink);
        now += STEP_US;
    }
    session.summary()
}

#[test]
fn the_decoder_and_the_judge_engine_count_the_same_notes() {
    for (lnmode, ln_mode) in [(1, LnMode::LongNote), (2, LnMode::ChargeNote), (3, LnMode::HellChargeNote)] {
        let model = chart(lnmode);
        let decoded = rbms_chart::count_playable_notes(&model);
        let engine = JudgeEngine::from_model_for_mode(&model).total_notes() as usize;
        assert_eq!(decoded, engine, "#LNMODE {lnmode}: the decoder counts {decoded} and the judge engine {engine}");
        let played = autoplay(model, ln_mode);
        assert_eq!(played.total_notes as usize, decoded, "#LNMODE {lnmode}: a run judges the notes the decoder counted");
    }
}

#[test]
fn autoplaying_a_chart_with_overlapping_long_notes_is_all_perfect_greats() {
    for (lnmode, ln_mode) in [(1, LnMode::LongNote), (2, LnMode::ChargeNote), (3, LnMode::HellChargeNote)] {
        let played = autoplay(chart(lnmode), ln_mode);
        let notes = played.total_notes;
        assert!(notes > 0);
        assert_eq!(played.counts, [notes, 0, 0, 0, 0, 0], "#LNMODE {lnmode}: every note is a PGREAT and nothing is missed");
        assert_eq!(played.ex_score, 2 * notes, "#LNMODE {lnmode}");
        assert_eq!(played.max_combo, notes, "#LNMODE {lnmode}: no note breaks the combo");
    }
}

#[test]
fn the_overlaps_chart_has_the_notes_the_reference_implementation_keeps() {
    let model = chart(1);
    assert_eq!(rbms_chart::count_playable_notes(&model), LONG_NOTES_KEPT + OVERLAP_PLAIN_NOTES_KEPT + NEIGHBOUR_NOTES);
}

/// How many generated charts the property test decodes.
const GENERATED_CHARTS: u64 = 300;

/// Sample ids a generated object can name; `ZZ` is the `#LNOBJ` of the charts that declare one.
const GENERATED_SAMPLES: [&str; 4] = ["01", "02", "03", "ZZ"];

/// Channels a generated chart writes: four keys, their long-note channels and their mines.
const GENERATED_CHANNELS: [&str; 12] = ["11", "12", "13", "14", "51", "52", "53", "54", "D1", "D2", "D3", "D4"];

/// Slots per line a generated chart picks from.
const GENERATED_SLOTS: [usize; 3] = [2, 4, 8];

/// A small deterministic generator, so a failure names a chart that can be written again.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }
}

/// A chart of up to six measures whose lines put normal notes, long-note objects, `#LNOBJ` objects
/// and mines wherever the generator lands, so every overlap and every unpaired object turns up.
fn generated_chart(rng: &mut Lcg) -> String {
    let mut text = format!("#BPM {}\r\n#TOTAL 300\r\n", 90 + rng.below(100));
    match rng.below(4) {
        0 => {}
        lnmode => text.push_str(&format!("#LNMODE {lnmode}\r\n")),
    }
    if rng.below(2) == 0 {
        text.push_str("#LNOBJ ZZ\r\n");
    }
    for sample in GENERATED_SAMPLES {
        text.push_str(&format!("#WAV{sample} x.wav\r\n"));
    }
    for measure in 1..=1 + rng.below(6) {
        for channel in GENERATED_CHANNELS {
            if rng.below(3) != 0 {
                continue;
            }
            let slots = GENERATED_SLOTS[rng.below(GENERATED_SLOTS.len() as u64) as usize];
            let line: String =
                (0..slots).map(|_| if rng.below(3) == 0 { GENERATED_SAMPLES[rng.below(GENERATED_SAMPLES.len() as u64) as usize] } else { "00" }).collect();
            if line.bytes().any(|byte| byte != b'0') {
                text.push_str(&format!("#{measure:03}{channel}:{line}\r\n"));
            }
        }
    }
    text
}

/// What a lane looks like once the decoder is done: heads and tails alternate and nothing else sits
/// between a head and its tail, with no head left open at the end.
fn lane_is_settled(model: &Model, lane: usize) -> bool {
    let mut open = false;
    for note in model.timelines.iter().filter_map(|tl| tl.notes[lane].as_ref()) {
        match (&note.kind, open) {
            (NoteKind::LongStart { .. }, false) => open = true,
            (NoteKind::LongEnd { .. }, true) => open = false,
            (NoteKind::Normal | NoteKind::Mine { .. }, false) => {}
            _ => return false,
        }
    }
    !open
}

#[test]
fn whatever_a_chart_writes_the_decoded_lanes_are_settled_and_counted_alike() {
    let mut rng = Lcg(1);
    for round in 0..GENERATED_CHARTS {
        let text = generated_chart(&mut rng);
        let mut model = rbms_chart::to_model(&rbms_parser::parse(text.as_bytes()), Mode::BEAT_7K);
        rbms_chart::resolve_long_note_flavour(&mut model, LnKind::Ln);
        assert!((0..model.mode.key).all(|lane| lane_is_settled(&model, lane)), "round {round}: a lane is not settled in\n{text}");
        let decoded = rbms_chart::count_playable_notes(&model);
        let engine = JudgeEngine::from_model_for_mode(&model).total_notes() as usize;
        assert_eq!(decoded, engine, "round {round}: the decoder and the judge engine count different notes in\n{text}");
        let played = autoplay(model, LnMode::default());
        assert_eq!(played.counts, [played.total_notes, 0, 0, 0, 0, 0], "round {round}: autoplay missed a note in\n{text}");
    }
}

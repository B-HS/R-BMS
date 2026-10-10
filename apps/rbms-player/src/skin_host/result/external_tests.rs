//! The result screens of an external skin pack, read through a host that has a finished run on it.
//!
//! Runs only when `RBMS_SKIN_PACK` names a pack with a `result.luaskin` and a `course.luaskin`, and
//! passes silently without it. The pack is read only: whatever a skin writes goes to an overlay in
//! the temporary directory.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rbms_judge::gauge::GaugeIndex;
use rbms_model::Mode;
use rbms_render::result::TargetView;
use rbms_skin::dst::{DrawStateSource, LuaFnId, OffsetSource, SkinOffset};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin_with_host};
use rbms_skin::lua::{LocalTime, LuaFnKind};
use rbms_skin::property::generated::{STRING_COURSE1_TITLE, STRING_COURSE10_TITLE, STRING_IR_NAME, STRING_RANKING1_NAME, STRING_RANKING10_NAME};
use rbms_skin::property::{AudioCommand, FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT, ScoreSlot, ScoreSnapshot, SkinHost, StaticScreen, VolumeBus};
use rbms_skin::timer::TimerState;

use super::snapshot::{FinishedRun, GaugeEnds, PreviousScore, ReplaySlot, ResultInput, ResultSnapshot, TimingFigures};
use crate::skin_host::ResultScene;
use crate::skin_host::ScreenHost;
use crate::skin_host::chart::{ChartContents, ChartMeta, ChartState};
use crate::skin_host::ir::{IrLink, IrPhase};
use crate::skin_host::options::PlayedOptions;
use crate::skin_host::score::standing::{PointFamily, ScoreSheet};
use crate::skin_host::system::SystemState;

const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The entry files of the result screens of a pack.
const RESULT_SCREENS: [&str; 2] = ["result.luaskin", "course.luaskin"];

/// The clock the system cluster is given.
const CLOCK: LocalTime = LocalTime { year: 2026, month: 10, day: 10, hour: 12, minute: 30, second: 45 };

/// The texts a result screen asks for that are empty here on purpose: the names on a ranking this
/// build does not read, the name of the service, and the titles of stages a course of two does not
/// have.
fn empty_on_purpose(asked: &str) -> bool {
    let Some(id) = asked.strip_prefix("text ").and_then(|id| id.parse::<i32>().ok()) else {
        return false;
    };
    (STRING_RANKING1_NAME..=STRING_RANKING10_NAME).contains(&id) || id == STRING_IR_NAME || (STRING_COURSE1_TITLE + 2..=STRING_COURSE10_TITLE).contains(&id)
}

/// The seed every load of this test is pinned with.
const TEST_SEED: u64 = 7;

/// How many frames the functions of a screen are called for.
const FRAMES: usize = 3;

/// Ten notes: five PGREAT, three GREAT, one GOOD and one miss.
fn snapshot() -> ResultSnapshot {
    let mut ends = [None; GaugeIndex::COUNT];
    ends[GaugeIndex::Normal.index()] = Some(72.34);
    ResultSnapshot::of(ResultInput {
        sheet: ScoreSheet { early: [3, 1, 0, 0, 0, 0], late: [2, 2, 1, 0, 0, 1], notes: 10, max_combo: 7, min_bp: 1, clear: 5, family: PointFamily::Beat7 },
        gauge: GaugeEnds { ends },
        timing: TimingFigures { average_ms: 2.5, std_dev_ms: 6.0, center_ms: 150, avg_duration_us: 54_321 },
        previous: PreviousScore { ex_score: 11, max_combo: 9, min_bp: Some(1), clear: 4 },
        target: Some(TargetView { name: "MY BEST".to_string(), ex: 14 }),
        options: PlayedOptions { random: 2, ..PlayedOptions::default() },
        favorite_chart: Some(false),
        course_titles: vec!["Stage 1".to_string(), "Stage 2".to_string()],
        ..ResultInput::default()
    })
}

fn chart() -> ChartMeta<'static> {
    ChartMeta {
        title: "Result Title",
        subtitle: "[SP ANOTHER]",
        genre: "TRANCE",
        artist: "Composer",
        md5: "md5-of-the-chart",
        table_name: "Satellite",
        table_level: "sl",
        level: 9,
        difficulty: 3,
        mode: Some(Mode::BEAT_7K),
        judge: Some(2),
        notes: Some(10),
        total: Some(300.0),
        contents: ChartContents { stagefile: Some(true), ..ChartContents::default() },
        ..ChartMeta::default()
    }
}

/// A host that answers as the one it wraps does and keeps the ids that were asked and came back
/// with nothing, so a run can say which ids the skin reads that no cluster answers.
struct Spy<'h, 'a> {
    inner: &'h ScreenHost<'a>,
    unanswered: RefCell<BTreeSet<String>>,
}

impl Spy<'_, '_> {
    fn note(&self, space: &str, id: i32) {
        self.unanswered.borrow_mut().insert(format!("{space} {id}"));
    }
}

impl OffsetSource for Spy<'_, '_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.inner.offset(id)
    }
}

impl DrawStateSource for Spy<'_, '_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let answer = self.inner.boolean(id);
        if answer.is_none() {
            self.note("option", id.abs());
        }
        answer
    }
}

impl SkinHost for Spy<'_, '_> {
    fn is_static(&self, id: i32) -> bool {
        self.inner.is_static(id)
    }

    fn integer(&self, id: i32) -> i32 {
        let answer = self.inner.integer(id);
        if answer == INTEGER_ABSENT {
            self.note("number", id);
        }
        answer
    }

    fn image_index(&self, id: i32) -> i32 {
        let answer = self.inner.image_index(id);
        if answer == IMAGE_INDEX_ABSENT {
            self.note("image index", id);
        }
        answer
    }

    fn rate(&self, id: i32) -> Option<f32> {
        let answer = self.inner.rate(id);
        if answer.is_none() {
            self.note("rate", id);
        }
        answer
    }

    fn float(&self, id: i32) -> f32 {
        let answer = self.inner.float(id);
        if answer.to_bits() == FLOAT_ABSENT.to_bits() {
            self.note("float", id);
        }
        answer
    }

    fn text(&self, id: i32) -> Cow<'_, str> {
        let answer = self.inner.text(id);
        if answer.is_empty() {
            self.note("text", id);
        }
        answer
    }

    fn timer_us(&self, id: i32) -> i64 {
        self.inner.timer_us(id)
    }

    fn now_us(&self) -> i64 {
        self.inner.now_us()
    }

    fn exec_event(&self, id: i32, arg1: i32, arg2: i32) {
        self.inner.exec_event(id, arg1, arg2);
    }

    fn write_rate(&self, id: i32, value: f32) {
        self.inner.write_rate(id, value);
    }

    fn write_text(&self, id: i32, value: &str) {
        self.inner.write_text(id, value);
    }

    fn audio(&self, command: AudioCommand<'_>) {
        self.inner.audio(command);
    }

    fn key_pressed(&self, code: i32) -> bool {
        self.inner.key_pressed(code)
    }

    fn screen_size(&self) -> (i32, i32) {
        self.inner.screen_size()
    }

    fn gauge(&self) -> f32 {
        self.inner.gauge()
    }

    fn gauge_type(&self) -> i32 {
        self.inner.gauge_type()
    }

    fn judge(&self, judge: i32) -> i32 {
        self.inner.judge(judge)
    }

    fn score(&self, slot: ScoreSlot) -> ScoreSnapshot {
        self.inner.score(slot)
    }

    fn volume(&self, bus: VolumeBus) -> f32 {
        self.inner.volume(bus)
    }

    fn set_volume(&self, bus: VolumeBus, value: f32) {
        self.inner.set_volume(bus, value);
    }
}

#[test]
fn the_result_screens_of_an_external_pack_read_everything_through_the_host_without_a_failure() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let overlay = std::env::temp_dir().join(format!("rbms-result-host-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&overlay);
    let user = SkinUserConfig::default();
    let meta = chart();
    let snapshot = snapshot();
    let timers = TimerState::new();
    let mut host = ScreenHost::new(1_000_000, &timers);
    host.static_screen = Some(StaticScreen::Result);
    host.chart = ChartState::Chart(&meta);
    host.system = SystemState { clock: Some(CLOCK), player_name: Some("guest"), ..SystemState::default() };
    host.show_result(FinishedRun::new(
        &snapshot,
        ResultScene {
            gauge_type: GaugeIndex::Normal.index(),
            replay: [ReplaySlot::Saved, ReplaySlot::Missing, ReplaySlot::Missing, ReplaySlot::Missing],
            ..ResultScene::default()
        },
    ));
    host.ir.link = Some(IrLink::new(false, IrPhase::Offline));

    let spy = Spy { inner: &host, unanswered: RefCell::new(BTreeSet::new()) };
    for name in RESULT_SCREENS {
        spy.unanswered.borrow_mut().clear();
        let entry: &Path = &pack.join(name);
        let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), write_overlay: Some(&overlay), ..SkinLoadOptions::new(&pack, &user, Mode::BEAT_7K) };
        let skin = load_skin_with_host(entry, options, &spy).unwrap_or_else(|error| panic!("{name} should load: {error}"));
        let runtime = skin.runtime().expect("a Lua skin keeps its interpreter");

        let mut called = 0;
        for _ in 0..FRAMES {
            runtime
                .frame(&spy, |bound| {
                    for index in 0..runtime.function_count() {
                        let function = LuaFnId(index as u32);
                        match runtime.kind_of(function) {
                            Some(LuaFnKind::Boolean) => drop(bound.call_boolean(function)),
                            Some(LuaFnKind::Integer) => drop(bound.call_integer(function)),
                            Some(LuaFnKind::Float) => drop(bound.call_float(function)),
                            Some(LuaFnKind::Text) => drop(bound.call_text(function)),
                            Some(LuaFnKind::Timer) => drop(bound.timer(function)),
                            _ => continue,
                        }
                        called += 1;
                    }
                })
                .expect("the host binds");
        }
        for destination in &skin.def.destination {
            for option in destination.op.iter().filter(|option| option.id != 0) {
                let _ = spy.boolean(option.id);
            }
        }
        for value in skin.def.value.iter().filter(|value| value.value.is_none() && value.reference != 0) {
            let _ = spy.integer(value.reference);
        }
        for text in skin.def.text.iter().filter(|text| text.value.is_none() && text.reference != 0) {
            let _ = spy.text(text.reference);
        }
        for image in skin.def.image.iter().filter(|image| image.reference != 0) {
            let _ = spy.image_index(image.reference);
        }
        let diagnostics = runtime.diagnostics();
        println!(
            "{name}: {called} calls over {FRAMES} frames, {} functions, {} swallowed, {} failed",
            runtime.function_count(),
            diagnostics.swallowed.len(),
            diagnostics.function_failures.len()
        );
        for swallowed in &diagnostics.swallowed {
            println!("    pcall caught x{}: {}", swallowed.count, swallowed.message);
        }
        for failure in &diagnostics.function_failures {
            println!("    function {:?} failed x{}: {}", failure.function, failure.count, failure.first_message);
        }
        println!("    asked and not answered: {:?}", spy.unanswered.borrow());
        let unexpected: Vec<_> = spy.unanswered.borrow().iter().filter(|asked| !empty_on_purpose(asked)).cloned().collect();
        assert!(unexpected.is_empty(), "{name}: the skin asked for ids that no cluster answers: {unexpected:?}");
        assert!(diagnostics.function_failures.is_empty(), "{name}: a function of the skin failed against the host");
        assert!(diagnostics.swallowed.is_empty(), "{name}: the skin's own pcall caught an error");
        assert!(called > 0, "{name}: no function was called");
        assert!(host.take_calls().is_empty(), "{name}: reading the host told the game to do something");
    }
    let _ = std::fs::remove_dir_all(&overlay);
}

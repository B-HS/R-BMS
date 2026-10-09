//! The host contract: everything a skin asks the running game, and everything it tells it.
//!
//! A skin never sees the game. It sees [`SkinHost`], which the application implements once per
//! screen and which this crate reaches from three places: the destination interpolator's draw
//! gating, the object values a renderer reads by property id, and the `main_state` module a Lua
//! skin calls every frame. One trait serves all three, so a value can never read differently
//! through a property id than through a script.
//!
//! Every method takes `&self`. The Lua binding shares one host between every function it exposes
//! for the length of a frame, so a host that has to change something in answer to a call -- run an
//! event, move a volume, start a sound -- records it behind its own interior mutability and applies
//! it when the frame is over.
//!
//! [`MapHost`] is the host for everything that is not the game: tests, and the dump tool that loads
//! a skin against a scenario file.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::names::{StaticScreen, static_scope};
use crate::dst::{DrawStateSource, OffsetSource, SkinOffset};
use crate::model::{DEFAULT_SKIN_HEIGHT, DEFAULT_SKIN_WIDTH};
use crate::timer::TIMER_OFF;

/// What [`SkinHost::integer`] answers when the id carries no value: the reference's
/// `Integer.MIN_VALUE`, which a number object reads as "draw nothing" and which a script compares
/// against directly.
pub const INTEGER_ABSENT: i32 = i32::MIN;

/// What [`SkinHost::image_index`] answers when the id selects nothing. Any negative index leaves the
/// image undrawn.
pub const IMAGE_INDEX_ABSENT: i32 = -1;

/// What [`SkinHost::float`] answers when neither id space carries a value.
pub const FLOAT_ABSENT: f32 = 0.0;

/// What [`SkinHost::text`] answers when the id carries no text.
pub const TEXT_ABSENT: &str = "";

/// What [`SkinHost::now_us`] answers before a scene clock exists.
pub const CLOCK_ORIGIN_US: i64 = 0;

/// One of the three volumes `main_state.volume_*` and `main_state.set_volume_*` address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VolumeBus {
    /// `volume_sys`: system and skin sounds.
    System,
    /// `volume_key`: key sounds.
    Key,
    /// `volume_bg`: background sounds.
    Background,
}

/// Whose score `main_state.rate*` and `main_state.exscore*` ask about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScoreSlot {
    /// `rate()` and `exscore()`: the run in progress or just finished.
    Current,
    /// `rate_best()` and `exscore_best()`: the player's own best.
    Best,
    /// `rate_rival()` and `exscore_rival()`: the rival or target it is paced against.
    Rival,
}

/// One score as the `main_state` score functions report it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct ScoreSnapshot {
    /// The score rate, as a share from zero to one.
    pub rate: f32,
    /// The EX score.
    pub exscore: i32,
}

/// One request a skin makes of the sound system through `main_state.audio_*`.
///
/// The path has already been resolved against the skin root and checked to lie inside it, and a
/// volume has already been clamped to the reference's `0.0..=2.0`; the host multiplies it by its own
/// system volume (`SkinAudioLuaApiExporter.play`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand<'a> {
    /// `audio_play` and `audio_loop`.
    Play { path: &'a Path, volume: f32, looped: bool },
    /// `audio_preload`: decode the file without making a sound.
    Preload { path: &'a Path },
    /// `audio_stop`.
    Stop { path: &'a Path },
    /// `audio_dispose`.
    Dispose { path: &'a Path },
}

/// Everything a skin can ask the running game, and everything it can tell it.
///
/// The boolean and offset reads come from the supertraits, so one implementation serves this
/// registry, the draw gating in [`crate::dst`] and the Lua binding in `crate::lua` alike:
///
/// - [`DrawStateSource::boolean`] answers `Option<bool>` for an option id exactly as the skin wrote
///   it. `None` means the host implements no such option; the loader then settles the id against the
///   skin's own customisation options and a script's `main_state.option` reads it as `false`.
/// - [`OffsetSource::offset`] answers `Option<SkinOffset>` for an offset id in `0..200`, in output
///   pixels. `None` is an offset the host does not set, which `main_state.offset` reports as zeros.
///
/// The ids of each kind are separate spaces: 41 is one property as an option, another as a number
/// and a third as a timer. Nothing here fails. An id the host does not know answers the documented
/// "absent" value of its kind, and a command it does not understand is ignored.
pub trait SkinHost: DrawStateSource {
    /// Whether the option under `id` is static on this screen: settled once when the skin is
    /// prepared and never re-read (`BooleanProperty.isStatic`). The loader drops an object whose
    /// static condition is false and forgets the condition when it is true.
    ///
    /// The reference classifies each option as static everywhere, static outside the song browser or
    /// static on the result screens, which is why the host and not a table answers. The default says
    /// "never", which is always safe: the condition is simply evaluated every frame.
    fn is_static(&self, _id: i32) -> bool {
        false
    }

    /// The integer under a `NUMBER_*` id, or [`INTEGER_ABSENT`] when it carries no value.
    fn integer(&self, id: i32) -> i32;

    /// The image index under `id`: which variant an image set shows and what `main_state.event_index`
    /// reports. This is its own id space, separate from [`Self::integer`]
    /// (`IntegerPropertyFactory.getImageIndexProperty`). A negative answer draws nothing.
    fn image_index(&self, _id: i32) -> i32 {
        IMAGE_INDEX_ABSENT
    }

    /// The share under a `RATE_*`, `SLIDER_*` or `BARGRAPH_*` id, or `None` when the host implements
    /// no such rate (`FloatPropertyFactory.getRateProperty`).
    ///
    /// Sliders, graphs and every script field of the reference's `FloatProperty` type read this
    /// space and no other. The value is not clamped here.
    fn rate(&self, _id: i32) -> Option<f32> {
        None
    }

    /// The number under `id`, looked up in the `FLOAT_*` space first and the rate space second, or
    /// [`FLOAT_ABSENT`] when neither answers (`FloatPropertyFactory.getFloatProperty`).
    ///
    /// This is what a `floatvalue` object and `main_state.float_number` read.
    fn float(&self, id: i32) -> f32;

    /// The text under a `STRING_*` id, or [`TEXT_ABSENT`].
    fn text(&self, id: i32) -> Cow<'_, str>;

    /// The microsecond the timer under `id` switched on, or [`TIMER_OFF`] while it is off. A script
    /// receives this as a double.
    fn timer_us(&self, id: i32) -> i64;

    /// The scene clock the frame is being drawn against, in microseconds.
    ///
    /// This is what `main_state.time()` returns. It must be the same clock the frame passes to
    /// [`crate::dst::prepare`], and it does not move within a frame.
    fn now_us(&self) -> i64;

    /// Runs the event under `id` with up to two arguments (`MainState.executeEvent`). An id the host
    /// defines no event for is ignored.
    fn exec_event(&self, _id: i32, _arg1: i32, _arg2: i32) {}

    /// Writes a share back under a rate id, which is what dragging a slider does
    /// (`FloatPropertyFactory.getRateWriter`).
    fn write_rate(&self, _id: i32, _value: f32) {}

    /// Writes text back under a string id, which is what confirming an editable text does
    /// (`StringPropertyFactory.getStringWriter`).
    fn write_text(&self, _id: i32, _value: &str) {}

    /// Carries out one `main_state.audio_*` request.
    fn audio(&self, _command: AudioCommand<'_>) {}

    /// Whether the key with this libGDX key code is held (`Gdx.input.isKeyPressed`). The caller has
    /// already turned a key name into its code and rejected negative ones.
    fn key_pressed(&self, _code: i32) -> bool {
        false
    }

    /// The window size in pixels, for `main_state.screen_width` and `screen_height`.
    fn screen_size(&self) -> (i32, i32) {
        (DEFAULT_SKIN_WIDTH, DEFAULT_SKIN_HEIGHT)
    }

    /// The groove gauge's value while a chart is being played, zero on every other screen
    /// (`main_state.gauge`).
    fn gauge(&self) -> f32 {
        0.0
    }

    /// The groove gauge's type while a chart is being played, zero on every other screen
    /// (`main_state.gauge_type`).
    fn gauge_type(&self) -> i32 {
        0
    }

    /// How many times judgement `judge` was given, early and late together: 0 is PGREAT, 1 GREAT,
    /// 2 GOOD, 3 BAD, 4 POOR, 5 MISS (`main_state.judge`).
    fn judge(&self, _judge: i32) -> i32 {
        0
    }

    /// The score `main_state.rate*` and `main_state.exscore*` report for one slot.
    fn score(&self, _slot: ScoreSlot) -> ScoreSnapshot {
        ScoreSnapshot::default()
    }

    /// One of the three volumes, as `main_state.volume_*` reports it.
    fn volume(&self, _bus: VolumeBus) -> f32 {
        0.0
    }

    /// Sets one of the three volumes. The reference does not range-check the value
    /// (`main_state.set_volume_*`), so neither does the caller.
    fn set_volume(&self, _bus: VolumeBus, _value: f32) {}
}

/// A host that answers nothing.
///
/// Every read returns the absent value of its kind, so a skin can be loaded before a play session or
/// a score exists, and a header can be read with no game behind it at all.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultState;

impl OffsetSource for DefaultState {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for DefaultState {
    fn boolean(&self, _id: i32) -> Option<bool> {
        None
    }
}

impl SkinHost for DefaultState {
    fn integer(&self, _id: i32) -> i32 {
        INTEGER_ABSENT
    }

    fn float(&self, _id: i32) -> f32 {
        FLOAT_ABSENT
    }

    fn text(&self, _id: i32) -> Cow<'_, str> {
        Cow::Borrowed(TEXT_ABSENT)
    }

    fn timer_us(&self, _id: i32) -> i64 {
        TIMER_OFF
    }

    fn now_us(&self) -> i64 {
        CLOCK_ORIGIN_US
    }
}

/// One thing a skin told a [`MapHost`] to do, kept in the order it was asked.
#[derive(Debug, Clone, PartialEq)]
pub enum HostCall {
    /// [`SkinHost::exec_event`].
    Event { id: i32, arg1: i32, arg2: i32 },
    /// [`SkinHost::write_rate`].
    WriteRate { id: i32, value: f32 },
    /// [`SkinHost::write_text`].
    WriteText { id: i32, value: String },
    /// [`AudioCommand::Play`].
    AudioPlay { path: PathBuf, volume: f32, looped: bool },
    /// [`AudioCommand::Preload`].
    AudioPreload { path: PathBuf },
    /// [`AudioCommand::Stop`].
    AudioStop { path: PathBuf },
    /// [`AudioCommand::Dispose`].
    AudioDispose { path: PathBuf },
    /// [`SkinHost::set_volume`].
    SetVolume { bus: VolumeBus, value: f32 },
}

/// A host made of plain maps from id to value, for tests and for the skin dump tool.
///
/// Every table is public and starts empty, so a test fills in the handful of ids it cares about and
/// everything else reads as absent. It deserialises from a scenario file whose keys are the field
/// names below and whose maps are keyed by id, which is how the dump tool describes a screen
/// without running the game. Every command a skin issues is recorded rather than carried out and is
/// read back with [`MapHost::calls`].
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct MapHost {
    /// Option id to its value, keyed by the positive id. A negative read negates the stored value;
    /// an id that is not here answers `None`.
    pub booleans: BTreeMap<i32, bool>,
    /// The positive option ids [`SkinHost::is_static`] reports as static whatever the screen.
    pub static_booleans: BTreeSet<i32>,
    /// The kind of screen the scenario describes. When it is set, [`SkinHost::is_static`] also
    /// reports every option the reference settles once on such a screen ([`static_scope`]).
    pub static_screen: Option<StaticScreen>,
    /// `NUMBER_*` id to its value.
    pub integers: BTreeMap<i32, i32>,
    /// Image index id to its value.
    pub image_indices: BTreeMap<i32, i32>,
    /// Rate id to its share.
    pub rates: BTreeMap<i32, f32>,
    /// `FLOAT_*` id to its value. [`SkinHost::float`] falls back to [`Self::rates`].
    pub floats: BTreeMap<i32, f32>,
    /// `STRING_*` id to its text.
    pub texts: BTreeMap<i32, String>,
    /// Offset id to its value.
    pub offsets: BTreeMap<i32, SkinOffset>,
    /// Timer id to the microsecond it switched on. An id that is not here is off.
    pub timers: BTreeMap<i32, i64>,
    /// The scene clock, in microseconds.
    pub now_us: i64,
    /// The libGDX key codes that are held.
    pub pressed_keys: BTreeSet<i32>,
    /// The window size in pixels, or `None` for the trait's default.
    pub screen: Option<(i32, i32)>,
    /// The groove gauge's value.
    pub gauge: f32,
    /// The groove gauge's type.
    pub gauge_type: i32,
    /// Judgement number to how often it was given.
    pub judges: BTreeMap<i32, i32>,
    /// The score in progress.
    pub score: ScoreSnapshot,
    /// The player's best score.
    pub score_best: ScoreSnapshot,
    /// The rival's score.
    pub score_rival: ScoreSnapshot,
    /// The system volume. A skin's `set_volume_sys` changes it.
    pub volume_system: Cell<f32>,
    /// The key volume.
    pub volume_key: Cell<f32>,
    /// The background volume.
    pub volume_background: Cell<f32>,
    #[serde(skip)]
    calls: RefCell<Vec<HostCall>>,
}

impl MapHost {
    /// A host with every table empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every command issued so far, oldest first.
    pub fn calls(&self) -> Vec<HostCall> {
        self.calls.borrow().clone()
    }

    /// Every command issued so far, forgetting them.
    pub fn take_calls(&self) -> Vec<HostCall> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }

    fn record(&self, call: HostCall) {
        self.calls.borrow_mut().push(call);
    }

    fn volume_cell(&self, bus: VolumeBus) -> &Cell<f32> {
        match bus {
            VolumeBus::System => &self.volume_system,
            VolumeBus::Key => &self.volume_key,
            VolumeBus::Background => &self.volume_background,
        }
    }
}

impl OffsetSource for MapHost {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offsets.get(&id).copied()
    }
}

impl DrawStateSource for MapHost {
    fn boolean(&self, id: i32) -> Option<bool> {
        let value = self.booleans.get(&id.saturating_abs()).copied()?;
        Some(if id < 0 { !value } else { value })
    }
}

impl SkinHost for MapHost {
    fn is_static(&self, id: i32) -> bool {
        self.static_booleans.contains(&id.saturating_abs()) || self.static_screen.is_some_and(|screen| static_scope(id).holds_on(screen))
    }

    fn integer(&self, id: i32) -> i32 {
        self.integers.get(&id).copied().unwrap_or(INTEGER_ABSENT)
    }

    fn image_index(&self, id: i32) -> i32 {
        self.image_indices.get(&id).copied().unwrap_or(IMAGE_INDEX_ABSENT)
    }

    fn rate(&self, id: i32) -> Option<f32> {
        self.rates.get(&id).copied()
    }

    fn float(&self, id: i32) -> f32 {
        self.floats.get(&id).or_else(|| self.rates.get(&id)).copied().unwrap_or(FLOAT_ABSENT)
    }

    fn text(&self, id: i32) -> Cow<'_, str> {
        Cow::Borrowed(self.texts.get(&id).map_or(TEXT_ABSENT, String::as_str))
    }

    fn timer_us(&self, id: i32) -> i64 {
        self.timers.get(&id).copied().unwrap_or(TIMER_OFF)
    }

    fn now_us(&self) -> i64 {
        self.now_us
    }

    fn exec_event(&self, id: i32, arg1: i32, arg2: i32) {
        self.record(HostCall::Event { id, arg1, arg2 });
    }

    fn write_rate(&self, id: i32, value: f32) {
        self.record(HostCall::WriteRate { id, value });
    }

    fn write_text(&self, id: i32, value: &str) {
        self.record(HostCall::WriteText { id, value: value.to_owned() });
    }

    fn audio(&self, command: AudioCommand<'_>) {
        self.record(match command {
            AudioCommand::Play { path, volume, looped } => HostCall::AudioPlay { path: path.to_path_buf(), volume, looped },
            AudioCommand::Preload { path } => HostCall::AudioPreload { path: path.to_path_buf() },
            AudioCommand::Stop { path } => HostCall::AudioStop { path: path.to_path_buf() },
            AudioCommand::Dispose { path } => HostCall::AudioDispose { path: path.to_path_buf() },
        });
    }

    fn key_pressed(&self, code: i32) -> bool {
        self.pressed_keys.contains(&code)
    }

    fn screen_size(&self) -> (i32, i32) {
        self.screen.unwrap_or((DEFAULT_SKIN_WIDTH, DEFAULT_SKIN_HEIGHT))
    }

    fn gauge(&self) -> f32 {
        self.gauge
    }

    fn gauge_type(&self) -> i32 {
        self.gauge_type
    }

    fn judge(&self, judge: i32) -> i32 {
        self.judges.get(&judge).copied().unwrap_or_default()
    }

    fn score(&self, slot: ScoreSlot) -> ScoreSnapshot {
        match slot {
            ScoreSlot::Current => self.score,
            ScoreSlot::Best => self.score_best,
            ScoreSlot::Rival => self.score_rival,
        }
    }

    fn volume(&self, bus: VolumeBus) -> f32 {
        self.volume_cell(bus).get()
    }

    fn set_volume(&self, bus: VolumeBus, value: f32) {
        self.volume_cell(bus).set(value);
        self.record(HostCall::SetVolume { bus, value });
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{AudioCommand, DefaultState, HostCall, INTEGER_ABSENT, MapHost, SkinHost, VolumeBus};
    use crate::dst::DrawStateSource;
    use crate::property::StaticScreen;

    #[test]
    fn a_map_host_negates_a_known_option_and_leaves_an_unknown_one_absent() {
        let mut host = MapHost::new();
        host.booleans.insert(32, true);
        assert_eq!(host.boolean(32), Some(true));
        assert_eq!(host.boolean(-32), Some(false));
        assert_eq!(host.boolean(33), None);
        assert_eq!(host.boolean(-33), None, "an option nobody implements is absent under either sign");
        assert_eq!(DefaultState.boolean(-32), None);
    }

    #[test]
    fn a_map_host_reads_a_scenario_keyed_by_id_and_falls_back_from_float_to_rate() {
        let host: MapHost = serde_json::from_str(r#"{ "integers": { "71": 1234 }, "rates": { "110": 0.5 }, "texts": { "10": "TITLE" }, "now_us": 2000000 }"#)
            .expect("the scenario parses");
        assert_eq!(host.integer(71), 1234);
        assert_eq!(host.integer(72), INTEGER_ABSENT);
        assert_eq!(host.rate(110), Some(0.5));
        assert_eq!(host.float(110), 0.5, "a float read finds a rate when no float answers");
        assert_eq!(host.text(10), "TITLE");
        assert_eq!(host.now_us(), 2_000_000);
    }

    #[test]
    fn a_map_host_holds_still_what_it_lists_and_what_its_screen_settles_once() {
        let mut host: MapHost = serde_json::from_str(r#"{ "static_booleans": [900], "static_screen": "result" }"#).expect("the scenario parses");
        assert!(host.is_static(900));
        assert!(host.is_static(-900));
        assert!(host.is_static(300), "a result rank is settled once on a result screen");
        assert!(host.is_static(-41), "and so is whatever is settled once outside the song browser");
        assert!(!host.is_static(33));

        host.static_screen = Some(StaticScreen::Select);
        assert!(!host.is_static(300));
        assert!(!host.is_static(41));
        assert!(host.is_static(50), "the ranking server's presence is settled once everywhere");

        host.static_screen = None;
        assert!(!host.is_static(50), "without a screen only the listed ids hold still");
        assert!(host.is_static(900));
    }

    #[test]
    fn a_map_host_records_what_a_skin_told_it_to_do() {
        let host = MapHost::new();
        host.exec_event(13, 1, 0);
        host.set_volume(VolumeBus::Key, 0.25);
        host.audio(AudioCommand::Stop { path: Path::new("select.ogg") });
        assert_eq!(host.volume(VolumeBus::Key), 0.25);
        assert_eq!(
            host.take_calls(),
            vec![
                HostCall::Event { id: 13, arg1: 1, arg2: 0 },
                HostCall::SetVolume { bus: VolumeBus::Key, value: 0.25 },
                HostCall::AudioStop { path: "select.ogg".into() },
            ]
        );
        assert!(host.calls().is_empty(), "taking the calls forgets them");
    }
}

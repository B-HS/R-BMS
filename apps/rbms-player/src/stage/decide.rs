//! The DECIDE screen: the scene a skin plays between a chart being picked and its run beginning.
//!
//! It is the reference's `MusicDecide`, and it is nothing but its skin and three times. The scene
//! opens with the decide cue, starts taking input once the skin's `input` time has passed, fades out
//! on its own once its `scene` time has passed, and leaves once the fade has run for the skin's
//! `fadeout` time ([`crate::stage::scene_life`]). Holding one of the keys 1, 3, 5 or 7, or Enter,
//! starts the fade early; Escape, or START and SELECT together, starts it and turns the way out
//! around, back to the browser.
//!
//! Meanwhile the chart loads. It is parsed when it is picked, as the reference parses it, and its
//! keysounds and images decode on the worker pool while the scene plays. A scene that ends with
//! everything in goes straight to the run; one that ends early hands what is outstanding to the
//! LOADING screen, which waits for the rest. Leaving for the browser stops the decodes.
//!
//! The screen exists only when a skin draws it. Without a decide skin a picked chart goes to the
//! LOADING screen as it always did ([`AppShared::decided_song_stage`]), and a skin that turns out
//! not to load ends the scene before it began, with the same result. A chart named on the command
//! line never comes here.
//!
//! The skin's files are read off the frame loop, so the screen is up for a few frames before its
//! skin can be drawn. Those frames are black, and the scene's clock starts on the frame the skin
//! first draws: that frame is the moment the reference's scene begins. The reference has no such
//! frames -- it stands still until the skin is in -- so two rules here are this screen's own. A
//! skin that has been waited for as long as a scene is held for one
//! ([`AppShared::skin_wait_is_spent`]) is given up on like one that could not be read, and the
//! chart goes on to the LOADING screen. And Escape, or START with SELECT, leaves for the browser at
//! once while the screen is still black: there is no scene yet to fade out.
//!
//! One thing differs from the reference on the way back. Its browser is a screen like any other and
//! starts over when a decide scene is cancelled: its skin is read again and its opening plays. Here
//! a chart picked in the browser opens this screen over it, and cancelling puts the browser back as
//! it was left -- the same skin, its timers and its clock where they stood -- because the browser
//! owns more than its skin shows (the filter, the search, the tab) and a cancel should cost none of
//! it. A scene that goes on to the run does end the browser's stay: its skin is let go of with the
//! change of screen and read again after the result.

use std::path::Path;

use rbms_render::{Color, FrameData, ReferenceImages, Renderer};
use rbms_skin::timer::TimerState;
use winit::keyboard::KeyCode;

use crate::app_play::LoadedChart;
use crate::skin_host::overview::ChartOverview;
use crate::skin_screen::DecideDraw;
use crate::stage::loading::ChartAssets;
use crate::stage::scene_life::{ScenePhase, SceneTimes, advance, begin_fadeout, takes_input};
use crate::stage::{Canvas, FrameCtx, KeyInput, LoadingState, Stage, StageHandler, Transition};
use crate::syssound::SystemSound;
use crate::{AppShared, DecodedImage, SKIN_TYPE_DECIDE, SelectView, decode_bga_image};

/// The key indices that end the scene early while one of them is down: keys 1, 3, 5 and 7
/// (`MusicDecide.input`).
const SKIP_KEY_INDICES: [usize; 4] = [0, 2, 4, 6];

/// What the keys and the controller are holding down, as the scene reads them once a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HeldInput {
    /// One of [`SKIP_KEY_INDICES`] is down.
    skip: bool,
    /// START and SELECT are both down.
    start_and_select: bool,
}

impl HeldInput {
    fn read(shared: &AppShared) -> HeldInput {
        HeldInput {
            skip: SKIP_KEY_INDICES.into_iter().any(|index| shared.key_index_pressed(index)),
            start_and_select: shared.start_pressed() && shared.select_pressed(),
        }
    }
}

/// Which way a scene that is over leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    /// On to the run.
    Onward,
    /// Back to the browser.
    Cancelled,
}

/// The scene itself: its times, whether it has begun, and what the player has asked of it.
///
/// It holds no clock and no timers of its own. Both are handed in, so a frame of it is the same
/// whether the application or a test is the one counting the time.
#[derive(Debug, Default)]
struct DecideScene {
    times: SceneTimes,
    begun: bool,
    cancelled: bool,
    /// Enter was pressed and is still down, and nothing has acted on that press yet. The reference
    /// reads the key this way (`KeyBoardInputProcesseor.isKeyPressed`): a press made before the
    /// scene listens is acted on when it does, as long as the key was not let go in between.
    enter: bool,
    /// The same for Escape.
    escape: bool,
}

impl DecideScene {
    /// Begin the scene with the times its skin states. `true` the one time it does: a scene that
    /// has begun is not begun again.
    fn begin(&mut self, times: SceneTimes) -> bool {
        if self.begun {
            return false;
        }
        self.begun = true;
        self.times = times;
        true
    }

    /// Take one keyboard event.
    fn key(&mut self, key: &KeyInput<'_>) {
        let waiting = match key.code {
            KeyCode::Enter | KeyCode::NumpadEnter => &mut self.enter,
            KeyCode::Escape => &mut self.escape,
            _ => return,
        };
        if key.pressed {
            *waiting = true;
        } else if key.released {
            *waiting = false;
        }
    }

    /// Whether the player is asking to go back to the browser right now: Escape pressed and not
    /// yet let go, or START and SELECT both down.
    fn wants_out(&self, held: HeldInput) -> bool {
        self.escape || held.start_and_select
    }

    /// One frame of the scene at `now_us` on its clock: the times are applied first and the keys
    /// after, in the order the reference runs `render` and `input`. Answers the way out once the
    /// scene is over, and nothing before it has begun.
    fn step(&mut self, timers: &mut TimerState, now_us: i64, held: HeldInput) -> Option<Leave> {
        if !self.begun {
            return None;
        }
        if advance(self.times, timers, now_us) == ScenePhase::Over {
            return Some(if self.cancelled { Leave::Cancelled } else { Leave::Onward });
        }
        if !takes_input(timers) {
            return None;
        }
        if held.skip || self.enter {
            begin_fadeout(timers, now_us);
        }
        if self.wants_out(held) {
            self.cancelled = true;
            begin_fadeout(timers, now_us);
        }
        self.enter = false;
        self.escape = false;
        None
    }
}

/// What has become of the skin the scene is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkinStatus {
    /// It is being read, or its files are.
    Reading,
    /// It is compiled and draws the screen.
    Ready,
    /// There is none after all: it could not be read, it was taken away, or it has been waited for
    /// as long as a scene is held for its skin.
    Gone,
}

/// The decide screen's own state: the chart that was picked, the load of it in flight, and the
/// scene being played over both.
pub(crate) struct DecideState {
    /// The library chart to parse when the screen is entered, or `None` for a chart that came
    /// parsed.
    queued: Option<usize>,
    /// Set when the queued chart could not be parsed, which sends the screen straight back.
    failed: bool,
    /// The chart's files as they decode. Taken when the screen leaves.
    assets: Option<Box<ChartAssets>>,
    /// Whether everything the chart named is in.
    loaded: bool,
    chart: ChartOverview,
    /// The course the chart opens, whose name the skin shows in place of the chart's title.
    course: Option<String>,
    /// The difficulty table the chart was picked from and the level it holds there.
    table_name: String,
    table_level: String,
    stagefile: Option<DecodedImage>,
    scene: DecideScene,
    skin: SkinStatus,
}

impl DecideState {
    fn empty(queued: Option<usize>) -> DecideState {
        DecideState {
            queued,
            failed: false,
            assets: None,
            loaded: false,
            chart: ChartOverview::default(),
            course: None,
            table_name: String::new(),
            table_level: String::new(),
            stagefile: None,
            scene: DecideScene::default(),
            skin: SkinStatus::Reading,
        }
    }

    /// The scene for the library chart at `index`, which is parsed when the screen is entered: by
    /// then the browser has let go of the output stream the chart's keysounds are loaded into.
    pub(crate) fn song(index: usize) -> DecideState {
        DecideState::empty(Some(index))
    }

    /// The scene for a chart that has been parsed already, whose path is the one in
    /// [`AppShared::chart_path`].
    pub(crate) fn loaded(shared: &AppShared, loaded: LoadedChart) -> DecideState {
        let mut state = DecideState::empty(None);
        state.take_chart(shared, loaded);
        state
    }

    /// Take a parsed chart in: what the skin shows of it, and the load to wait on.
    ///
    /// The stage image is decoded here and now, as the reference reads it when the chart is picked
    /// (`BMSResource.setModel`), so the skin has it from its first frame.
    fn take_chart(&mut self, shared: &AppShared, mut loaded: LoadedChart) {
        self.chart = std::mem::take(&mut loaded.overview);
        self.course = shared.course_run.as_ref().map(|run| run.course.name.clone());
        (self.table_name, self.table_level) = table_of_the_open_folder(shared);
        self.stagefile = Path::new(&shared.chart_path)
            .parent()
            .filter(|_| !self.chart.stagefile.trim().is_empty())
            .and_then(|folder| decode_bga_image(folder, &self.chart.stagefile));
        self.assets = Some(Box::new(ChartAssets::of(loaded)));
    }

    /// Take in whatever the decodes have finished since the last frame.
    fn poll_load(&mut self, shared: &mut AppShared) {
        if self.loaded {
            return;
        }
        if let Some(assets) = self.assets.as_mut() {
            self.loaded = assets.poll(shared);
        }
    }

    /// How much of the chart's files are in, from nothing (0) to everything (1).
    fn progress(&self) -> f32 {
        let (done, total) = self.assets.as_ref().map_or((0, 0), |assets| assets.progress());
        match (self.loaded, total) {
            (true, _) => 1.0,
            (false, 0) => 0.0,
            (false, total) => done as f32 / total as f32,
        }
    }

    /// Leave for the run. A scene that played with everything in starts it; anything else -- files
    /// still decoding, or a skin that never drew -- goes on to the LOADING screen, which is where a
    /// picked chart goes when there is no decide scene at all.
    fn hand_on(&mut self, shared: &mut AppShared) -> Transition {
        let Some(assets) = self.assets.take() else {
            return self.abandon(shared);
        };
        if self.loaded && self.skin == SkinStatus::Ready {
            return Transition::To(assets.enter(shared));
        }
        Transition::To(Stage::Loading(LoadingState::waiting_on(assets)))
    }

    /// Leave for the browser: stop the decodes, hand the chart's bank back, and forget what the
    /// chart was picked for.
    fn abandon(&mut self, shared: &mut AppShared) -> Transition {
        if let Some(assets) = self.assets.take() {
            assets.stop();
        }
        shared.release_play_audio();
        shared.replay = None;
        shared.practice_requested = false;
        shared.release_practice_chart();
        if shared.course_run.is_some() {
            return crate::end_course(shared);
        }
        Transition::Back
    }

    /// Draw the skin's frame, or answer that it has none to draw yet. A Lua skin still waiting to
    /// be read is read by this call, against the chart handed over here.
    fn draw_skin(&self, shared: &AppShared, canvas: &mut Canvas<'_>) -> bool {
        let stagefile = self.stagefile.as_ref().and_then(|image| canvas.background_texture(image.generation, &image.rgba, image.width, image.height));
        let mut chart = self.chart.meta(stagefile.is_some());
        chart.heading = self.course.as_deref();
        chart.table_name = &self.table_name;
        chart.table_level = &self.table_level;
        let images = ReferenceImages { stagefile, ..ReferenceImages::default() };
        let data = FrameData { series: self.chart.series(), images, ..FrameData::default() };
        shared.draw_decide_skin(canvas, &DecideDraw { chart: &chart, progress: self.progress(), data })
    }
}

/// The difficulty table and the level the browser has open, which is where a chart picked now was
/// picked from; both empty anywhere else in the browser (`MusicSelector.readChart`).
fn table_of_the_open_folder(shared: &AppShared) -> (String, String) {
    let SelectView::TableLevel(table, level) = shared.select_view else {
        return (String::new(), String::new());
    };
    let name = shared.table_names.get(table).cloned().unwrap_or_default();
    let level = shared.table_levels.get(table).and_then(|levels| levels.get(level)).map(|(label, _)| label.clone()).unwrap_or_default();
    (name, level)
}

/// What has become of the decide skin, asked after the frame's own preparation of it.
///
/// A skin still on its way after the scene has waited its limit out for it is not coming as far as
/// this screen is concerned: nothing else is drawn here, so the wait would never end.
fn skin_status(shared: &AppShared) -> SkinStatus {
    if shared.skin_screens.get(SKIN_TYPE_DECIDE).is_some() && !shared.skins.is_waiting(SKIN_TYPE_DECIDE) {
        return SkinStatus::Ready;
    }
    let coming = shared.has_skin_document(SKIN_TYPE_DECIDE) && !shared.skin_wait_is_spent(SKIN_TYPE_DECIDE);
    if coming { SkinStatus::Reading } else { SkinStatus::Gone }
}

impl AppShared {
    /// Whether a skin can draw the decide scene: one is chosen for it, and it has not already been
    /// found unreadable.
    ///
    /// A skin that failed to read is not tried again until the player moves a choice or reloads, so
    /// a broken decide skin costs the first chart picked a moment and no chart after it anything.
    pub(crate) fn has_decide_scene(&self) -> bool {
        let screen = SKIN_TYPE_DECIDE;
        self.skins.document_path(&self.config, screen).is_some()
            && (self.skins.document(screen).is_some() || self.skins.is_waiting(screen) || self.skins.needs_reload_for(&self.config, screen))
    }

    /// The screen a chart picked in the browser goes to: the decide scene when a skin draws one,
    /// and the LOADING screen otherwise.
    pub(crate) fn decided_song_stage(&self, index: usize) -> Stage {
        if self.has_decide_scene() { Stage::Decide(Box::new(DecideState::song(index))) } else { Stage::Loading(LoadingState::song(index)) }
    }

    /// The screen a chart the browser has already parsed goes to, by the same rule.
    pub(crate) fn decided_chart_stage(&mut self, loaded: LoadedChart) -> Stage {
        if self.has_decide_scene() { Stage::Decide(Box::new(DecideState::loaded(self, loaded))) } else { self.enter_loaded_chart(loaded) }
    }
}

impl StageHandler for DecideState {
    /// Parse the chart that was picked, and ask for the skin to be read again: a Lua skin builds
    /// its screen out of the chart it is read against, so one read for the last chart shown here
    /// describes that chart.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        let shared = &mut *ctx.shared;
        shared.skins.request_for(&shared.config, SKIN_TYPE_DECIDE);
        let Some(index) = self.queued.take() else {
            return;
        };
        let Some(path) = shared.library.songs().get(index).map(|entry| entry.path.to_string_lossy().to_string()) else {
            self.failed = true;
            return;
        };
        shared.chart_path = path;
        match shared.load() {
            Some(loaded) => self.take_chart(shared, loaded),
            None => self.failed = true,
        }
    }

    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let shared = &mut *ctx.shared;
        if self.failed {
            return self.abandon(shared);
        }
        self.poll_load(shared);
        match self.skin {
            SkinStatus::Reading if self.scene.wants_out(HeldInput::read(shared)) => self.abandon(shared),
            SkinStatus::Reading => Transition::Stay,
            SkinStatus::Gone => self.hand_on(shared),
            SkinStatus::Ready => {
                let now_us = shared.skin_now_us();
                let held = HeldInput::read(shared);
                match self.scene.step(&mut shared.skin_timers, now_us, held) {
                    None => Transition::Stay,
                    Some(Leave::Onward) => self.hand_on(shared),
                    Some(Leave::Cancelled) => self.abandon(shared),
                }
            }
        }
    }

    fn handle_key(&mut self, _ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        self.scene.key(&key);
        Transition::Stay
    }

    /// The skin's frame, and black until there is one. The frame the skin first draws on is the one
    /// the scene begins on: the clock goes back to zero and the decide cue is played.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let shared = &mut *ctx.shared;
        canvas.clear_bga();
        shared.prepare_skin(canvas, SKIN_TYPE_DECIDE);
        self.skin = skin_status(shared);
        if self.skin == SkinStatus::Ready && self.scene.begin(SceneTimes::of_skin(shared.skins.document(SKIN_TYPE_DECIDE))) {
            shared.start_skin_scene_clock();
            shared.play_system_sound(SystemSound::Decide);
            if let Some(assets) = self.assets.as_mut() {
                assets.mark_announced();
            }
        }
        if !self.draw_skin(shared, canvas) {
            canvas.native().clear(Color::BLACK);
        }
    }
}

#[cfg(test)]
mod tests;

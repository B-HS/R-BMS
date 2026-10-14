//! One frame of a skin document for the skin preview: prepared against sample state, and drawn into
//! a rectangle of the skin menu rather than over the whole target.
//!
//! This is the preview's side of [`AppShared::with_skin_frame`](crate::AppShared), and lives beside
//! it because it is made of the same parts: the inputs of a frame, the host they are gathered into,
//! the one binding of the skin's interpreter a frame is prepared in, and the prepared frame that is
//! drawn afterwards. What differs is what a preview must not do. The host is answered from the
//! sample state the preview brings and from nothing of the player's -- no name, no history, no
//! ranking. What the skin tells the game while it is read, prepared or drawn -- an event, a value
//! written back, a sound -- is dropped rather than kept for the end of the frame, so a previewed skin
//! is silent and changes nothing. And the pointer is not judged against a frame drawn here.
//!
//! The frame is drawn through [`crate::skin_preview::region::with_region`], so the document is laid
//! out for the rectangle it is handed and clipped to it.
//!
//! # The clock
//!
//! A frame here is prepared at the time its caller names rather than at the scene clock's. A preview
//! counts its own time, which stands still for as long as its document is on its way, and every
//! timer its sample state switches is stamped on that same count.
//!
//! # What is measured against the whole window
//!
//! The rectangle is where the document lands and nothing else. A nudge the player made is stored in
//! pixels of the window, and a skin's Lua is told how large the window is: both are answered for the
//! window, so a nudge moves an object by the same share of the screen in the preview as it does on
//! the screen itself.

use std::time::{SystemTime, UNIX_EPOCH};

use rbms_render::{Color, FrameData, Rect, Renderer, SkinFrame};
use rbms_skin::dst::{LuaDrawEval, OffsetSource};

use super::{DecideDraw, PlayDraw, ResultDraw};
use super::{FrameInputs, PREVIEW_FAILURE_NOTE, PreparedDocument, SKIN_VERSION, is_result_screen, static_screen_of, window_size};
use crate::AppShared;
use crate::skin_host::ScreenHost;
use crate::skin_host::chart::ChartState;
use crate::skin_host::ir::{IrLink, IrPhase};
use crate::skin_host::loading::{LoadingScreen, LoadingState};
use crate::skin_host::options::{OptionsState, SettingsView};
use crate::skin_host::result::snapshot::FinishedRun;
use crate::skin_host::select::SelectShown;
use crate::skin_host::system::{CourseStage, SystemState, Volumes};
use crate::skin_preview::region::with_region;
use crate::skin_select::SkinRead;
use crate::stage::Canvas;

/// The name a previewed skin is told the player goes by. The player's own is not read.
const SAMPLE_PLAYER_NAME: &str = "PLAYER";

/// Where and when one frame of a preview is drawn.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PreviewFrame {
    /// Which screen's document is drawn.
    pub(crate) screen: i32,
    /// The preview's own time, in microseconds since its scene began.
    pub(crate) now_us: i64,
    /// Where the frame goes, in the space the built-in screens are laid out in.
    pub(crate) rect: Rect,
}

/// What the song browser's sample brings to one frame of its document.
pub(crate) struct PreviewSelect<'a> {
    /// The sample list as the bars a wheel turns through.
    pub(crate) data: FrameData<'a>,
    /// The sample chart under the cursor, or an empty slot when the bar there is no chart.
    pub(crate) chart: ChartState<'a>,
    /// The browser's frame as the host reads it, made of sample values alone.
    pub(crate) shown: &'a SelectShown,
}

/// The sample state one frame of a preview is drawn from, by the screen it stands in for.
pub(crate) enum PreviewScene<'a> {
    Select(PreviewSelect<'a>),
    Decide(DecideDraw<'a>),
    Play(PlayDraw<'a>),
    /// A finished run, with the course it closed when the screen is the course's.
    Result {
        draw: ResultDraw<'a>,
        course: Option<CourseStage>,
    },
}

/// What became of one frame of a preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreviewDrawn {
    /// The frame is in the rectangle.
    Drawn,
    /// Nothing was drawn: the document is being read, or was read on this very frame, or its files
    /// are still being decoded.
    Waiting,
    /// Nothing was drawn: the document is compiled, and its interpreter could not be bound for the
    /// frame. Why, as the interpreter says it.
    Unbound(String),
}

impl AppShared {
    /// Note that a preview is about to ask for `screen`'s document, on a frame it reads nothing on.
    ///
    /// Asking is what keeps a compiled screen from being let go of when the frame ends. A preview
    /// answers that its document is on its way one frame before it reads it, and a screen that was
    /// compiled for the scene underneath is kept through that frame, so the files the two share are
    /// not decoded a second time.
    pub(crate) fn keep_skin_preview(&mut self, screen: i32) {
        self.skin_screens.mark_prepared(screen);
    }

    /// Begin a scene for a preview of `screen`: every timer off and the scene clock back at zero, as
    /// a scene begins anywhere ([`AppShared::begin_skin_scene`]), and the Lua skin read for that
    /// screen due to be read again, against the preview's sample state.
    ///
    /// What a scene beginning on a screen of its own does besides is left undone. The screens
    /// parked under the menu stay parked: the menu is opened to be closed again, and the screen it
    /// was opened over is drawn on the first frame back with what it was compiled with. A Lua skin
    /// read for any other screen keeps its read, for the same reason. And what the scene under the
    /// menu was sounding goes on sounding, as it does under the settings screen.
    pub(crate) fn begin_skin_preview_scene(&mut self, screen: i32) {
        self.reset_skin_scene();
        self.skins.expire_scripted_for(screen);
    }

    /// Let go of what a preview read for `screen`: a Lua skin read against sample state goes,
    /// interpreter and all, and whatever draws the screen next reads it against its own state. The
    /// screen compiled from it is let go of by the frame that follows, with any other screen whose
    /// read is gone.
    pub(crate) fn end_skin_preview_of(&mut self, screen: i32) {
        self.skins.drop_scripted_for(screen);
    }

    /// [`AppShared::prepare_skin`] for a preview: the same reading and compiling, with a document
    /// that could not be read reported as one nothing is drawn in place of. The preview's own
    /// rectangle says why.
    pub(crate) fn prepare_skin_preview(&mut self, canvas: &mut Canvas<'_>, screen: i32) {
        self.skin_screens.mark_prepared(screen);
        self.read_and_compile_skin(canvas, screen, PREVIEW_FAILURE_NOTE);
        self.hold_scene_while_loading(screen);
    }

    /// Have the screen compiled for `screen` read the offset of the whole screen again on the frame
    /// that follows. A compiled screen settles that offset on the first frame it draws and keeps it,
    /// so this is how a nudge of it is shown without the screen being compiled again.
    pub(crate) fn resettle_skin_preview_whole_offset(&mut self, screen: i32) {
        if let Some(built) = self.skin_screens.built.get_mut(&screen) {
            built.screen.forget_whole_offset();
        }
    }

    /// Whether `screen`'s document is compiled from the read its choices call for: no newer read of
    /// it is waiting or due, so the frame that asks can be drawn and what the document states -- its
    /// times, what it asks of its timers -- is what will be drawn.
    pub(crate) fn skin_preview_is_ready(&self, screen: i32) -> bool {
        self.skin_screens.get(screen).is_some() && !self.skins.is_waiting(screen) && !self.skins.needs_reload_for(&self.config, screen)
    }

    /// How many things a skin has told the game that are waiting for the frame to end, for the
    /// tests that check a previewed skin leaves none.
    #[cfg(test)]
    pub(crate) fn skin_calls_waiting(&self) -> usize {
        self.skin_screens.input.borrow().calls.len()
    }

    /// What the machine says to a previewed skin, and what stands in for the player: the clock, the
    /// volumes and the frame rate are the real ones, the name is a sample and no run was ever
    /// recorded.
    fn preview_system_state(&self, course: Option<CourseStage>) -> SystemState<'_> {
        let unix_seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX));
        let audio = &self.config.audio;
        SystemState {
            clock: rbms_skin::lua::local_time(unix_seconds),
            uptime_ms: Some(self.skin_uptime_ms()),
            fps: Some(self.fps),
            player_name: Some(SAMPLE_PLAYER_NAME),
            version: Some(SKIN_VERSION),
            history: Some(&[]),
            volumes: Some(Volumes { system: audio.system, key: audio.key, background: audio.bg }),
            course,
        }
    }

    /// Makes one frame of `at.screen`'s document from `scene` and draws it into `at.rect`.
    ///
    /// The frame is made the way [`AppShared::with_skin_frame`] makes one -- one host, bound to the
    /// skin's interpreter once, every object prepared inside that binding and drawn after it -- with
    /// the differences the module's own notes list. A Lua skin waiting for this screen is read here,
    /// against the sample state, and that frame draws nothing.
    ///
    /// Nothing is drawn unless the answer is [`PreviewDrawn::Drawn`].
    pub(crate) fn draw_skin_preview(&self, canvas: &mut Canvas<'_>, at: PreviewFrame, scene: PreviewScene<'_>) -> PreviewDrawn {
        let PreviewFrame { screen, now_us, rect } = at;
        let window = canvas.native().size();
        let offsets = self.skin_offsets(screen, window);
        let mut course = None;
        let inputs = match scene {
            PreviewScene::Select(select) => FrameInputs { chart: select.chart, select: Some(select.shown), ..FrameInputs::new(&offsets, select.data, window) },
            PreviewScene::Decide(decide) => FrameInputs {
                chart: ChartState::Chart(decide.chart),
                loading: LoadingState { screen: LoadingScreen::Elsewhere, progress: decide.progress },
                settings: Some(SettingsView::of_config(&self.config)),
                ..FrameInputs::new(&offsets, decide.data, window)
            },
            PreviewScene::Play(play) => FrameInputs {
                chart: ChartState::Chart(play.chart),
                play: Some((play.shown, play.offsets)),
                paced_by: Some(play.paced_by),
                ..FrameInputs::new(&offsets, play.data, window)
            },
            PreviewScene::Result { draw, course: stage } => {
                course = stage;
                FrameInputs {
                    chart: draw.chart.map_or_else(ChartState::default, ChartState::Chart),
                    result: draw.run.map(|run| FinishedRun::new(run, draw.scene)),
                    ..FrameInputs::new(&offsets, draw.data, window)
                }
            }
        };

        let mut host = ScreenHost::new(now_us, &self.skin_timers);
        host.system = self.preview_system_state(course);
        host.chart = inputs.chart;
        host.loading = inputs.loading;
        if let Some(settings) = inputs.settings {
            host.options = OptionsState::of_settings(settings);
        }
        if let Some(finished) = inputs.result {
            host.show_result(finished);
        }
        if let Some(shown) = inputs.select {
            host.show_select(shown);
        }
        if let Some((shown, play_offsets)) = inputs.play {
            host.show_play(shown);
            host.options.paced_by = inputs.paced_by;
            host.play_offsets = Some(play_offsets);
        }
        host.ir.link = is_result_screen(screen).then(|| IrLink::new(false, IrPhase::Offline));
        host.offsets = Some(inputs.offsets as &dyn OffsetSource);
        host.static_screen = Some(static_screen_of(screen));
        host.window = Some(window_size(inputs.window));

        if self.skins.is_waiting(screen) {
            self.skins.read_waiting(&self.config, screen, SkinRead { host: &host, seed: self.skins.seed() });
            drop(host.take_calls());
            return PreviewDrawn::Waiting;
        }
        let Some(compiled) = self.skin_screens.get(screen) else {
            return PreviewDrawn::Waiting;
        };
        let frame = SkinFrame { now_us, timers: &self.skin_timers, state: &host, lua: None, mouse: None, data: inputs.data };
        let prepared = match self.skin_runtime(screen) {
            Some(runtime) => runtime.frame(&host, |bound| compiled.prepare(&SkinFrame { lua: Some(bound as &dyn LuaDrawEval), ..frame })),
            None => Ok(compiled.prepare(&frame)),
        };
        drop(host.take_calls());
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return PreviewDrawn::Unbound(error.to_string()),
        };
        let document = PreparedDocument { screen_type: screen, screen: compiled, frame, prepared, movies: &self.skin_screens.movies };
        with_region(canvas, rect, |region| {
            region.clear(Color::BLACK);
            document.draw(region);
        });
        PreviewDrawn::Drawn
    }
}

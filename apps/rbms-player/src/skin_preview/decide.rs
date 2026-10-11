//! The chart the skin preview shows on the decide screen: the sample chart of the play preview, as
//! the scene between a chart being picked and its run beginning shows a chart.
//!
//! The decide screen is nothing but its skin and three times, and so is this: the scene opens, runs
//! for as long as the skin says, fades for as long as the skin says, and begins again. What a skin
//! reads of the chart is read off the chart itself, as the screen reads its own. How much of the
//! chart's files are in goes from nothing to everything over the scene, so a skin that draws the
//! load has something to draw.

use rbms_model::Mode;
use rbms_render::FrameData;
use rbms_skin::loader::SKIN_TYPE_DECIDE;
use rbms_skin::timer::MICROS_PER_MILLI;

use super::SampleStep;
use crate::AppShared;
use crate::skin_host::overview::ChartOverview;
use crate::skin_screen::DecideDraw;
use crate::skin_screen::preview::{PreviewDrawn, PreviewFrame, PreviewScene};
use crate::stage::Canvas;
use crate::stage::play::skin_preview::sample_model;
use crate::stage::scene_life::{ScenePhase, SceneTimes, advance};

/// The mode of the chart the decide screen is shown with.
const SAMPLE_MODE: Mode = Mode::BEAT_7K;

/// The shortest a previewed decide scene runs before it fades, in milliseconds. A skin that states
/// no length would otherwise be over as it began, every frame.
const SHORTEST_SCENE_MS: i64 = 1_000;

/// How much of a chart is in when none of it is, and when all of it is.
const NOTHING_LOADED: f32 = 0.0;
const ALL_LOADED: f32 = 1.0;

/// The sample chart on the decide screen.
pub(super) struct DecideSample {
    chart: ChartOverview,
    /// The times the scene runs on, taken from the skin on each frame it can be drawn.
    times: SceneTimes,
}

impl DecideSample {
    /// The sample chart as the decide screen reads a chart that was picked. `None` only if the
    /// sample chart cannot be written for the mode, which it can.
    pub(super) fn new() -> Option<DecideSample> {
        Some(DecideSample { chart: ChartOverview::of_model(&sample_model(SAMPLE_MODE)?), times: SceneTimes::default() })
    }

    /// Move the scene on to `now_us`. `ready` is whether the skin that draws it can be drawn. The
    /// scene is over once its fade has run for as long as the skin says.
    pub(super) fn step(&mut self, shared: &mut AppShared, now_us: i64, ready: bool) -> SampleStep {
        if !ready {
            return SampleStep::Running;
        }
        let stated = SceneTimes::of_skin(shared.skins.document(SKIN_TYPE_DECIDE));
        self.times = SceneTimes { scene_ms: stated.scene_ms.max(SHORTEST_SCENE_MS), ..stated };
        match advance(self.times, &mut shared.skin_timers, now_us) {
            ScenePhase::Over => SampleStep::Over,
            ScenePhase::Opening | ScenePhase::Listening | ScenePhase::FadingOut => SampleStep::Running,
        }
    }

    /// How much of the chart's files are in at `now_us`: all of them by the time the scene fades.
    fn progress(&self, now_us: i64) -> f32 {
        let scene_us = self.times.scene_ms.saturating_mul(MICROS_PER_MILLI);
        if scene_us <= 0 {
            return ALL_LOADED;
        }
        (now_us as f32 / scene_us as f32).clamp(NOTHING_LOADED, ALL_LOADED)
    }

    /// Draw one frame of the scene into `at.rect`, from what the decide screen hands its own skin:
    /// the chart, how much of it is in, and the series its graphs plot. The chart has no stage
    /// picture.
    pub(super) fn draw(&self, shared: &AppShared, canvas: &mut Canvas<'_>, at: PreviewFrame) -> PreviewDrawn {
        let chart = self.chart.meta(false);
        let data = FrameData { series: self.chart.series(), ..FrameData::default() };
        shared.draw_skin_preview(canvas, at, PreviewScene::Decide(DecideDraw { chart: &chart, progress: self.progress(at.now_us), data }))
    }
}

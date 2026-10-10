//! The COURSE RESULT screen: what a whole course added up to once its last stage ended.
//!
//! A course is played through the same three screens a single chart is, so this screen exists only
//! for the tally that spans them: the summed EX, the combined miss count, the gauge the run carried
//! out of its last stage, and the trophy that tally earns.
//!
//! Drawn by a skin it is the reference's `CourseResult`, a scene with a length
//! ([`crate::stage::scene_life`]) that runs exactly as the single chart's result scene does
//! ([`crate::stage::result`]): the same scene, the same keys, the same three score timers on its
//! first frame, the same connection timers of the score server. What differs is what the reference
//! differs in. The graph key turns the gauge by the course's own expression, nothing is offered to
//! run again, and the scene always ends in the browser. The course's score is every stage's added together, and its gauge graph is the stages'
//! histories end to end ([`CourseSkinRun`]).
//!
//! Without a skin the screen is the built-in one, as it always was: the rows of
//! [`crate::course_ui::result_rows`], which keep the reference's ordering in one place, and every
//! key acts at once.

mod skin_run;
#[cfg(test)]
mod tests;

use rbms_skin::loader::SKIN_TYPE_COURSE_RESULT;
use winit::keyboard::KeyCode;

use crate::skin_host::ResultScene;
use crate::skin_screen::ResultDraw;
use crate::stage::result::{Scene, SceneInput, SkinStatus, begin_scene_with_skin, has_result_scene, held_key_indices, run_scene_frame};
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::syssound::{SystemSound, course_result_sound};
use crate::{AppShared, CW, Color, Rect, draw_text, draw_text_right, end_course};
use rbms_render::{FrameData, ReferenceImages, Renderer};
use skin_run::CourseSkinRun;

/// Width of the panel the rows are laid out in.
const PANEL_W: f32 = 560.0;

/// Where the first row sits and how far apart the rows are.
const ROW_TOP: f32 = 190.0;
const ROW_PITCH: f32 = 40.0;

/// Height of one row's plate.
const ROW_H: f32 = 32.0;

/// Text scale of the heading, the course name and the rows.
const HEADING_SCALE: f32 = 3.0;
const NAME_SCALE: f32 = 1.6;
const ROW_SCALE: f32 = 1.4;

/// Baselines of the heading, the course name and the hint under the rows.
const HEADING_Y: f32 = 60.0;
const NAME_Y: f32 = 110.0;
const HINT_Y: f32 = 150.0;

/// Inset of a row's label from the plate's left edge, and of its value from the right.
const LABEL_INSET: f32 = 16.0;
const VALUE_INSET: f32 = 16.0;

/// How far a row's text sits below the top of its plate.
const TEXT_DROP: f32 = 8.0;

/// The cues a course result plays, each the course's own when the sound set has it and the single
/// result's otherwise (`CourseResult.prepare`, `CourseResult.render`).
struct Cues {
    clear: SystemSound,
    fail: SystemSound,
    close: SystemSound,
}

impl Cues {
    fn of(shared: &AppShared) -> Cues {
        let pick = |course: SystemSound, chart: SystemSound| if shared.syssound.is_resolved(course) { course } else { chart };
        Cues {
            clear: pick(SystemSound::CourseClear, SystemSound::ResultClear),
            fail: pick(SystemSound::CourseFail, SystemSound::ResultFail),
            close: pick(SystemSound::CourseClose, SystemSound::ResultClose),
        }
    }

    /// Whether the sound set has a closing cue of either kind, which is what silences the clear or
    /// fail cue when the scene ends.
    fn has_closing(shared: &AppShared) -> bool {
        shared.syssound.is_resolved(SystemSound::CourseClose) || shared.syssound.is_resolved(SystemSound::ResultClose)
    }
}

/// The finished course as this screen reports it: the rows, whether it cleared, and the name it ran
/// under. The run itself is dropped when the course ends, so everything drawn here is copied out of
/// it while it is still alive.
pub(crate) struct CourseResultState {
    rows: Vec<(String, String)>,
    cleared: bool,
    course_name: String,
    /// Set once the arrival cue has been raised, so a screen re-entered from a suspended stage does
    /// not sound it twice.
    announced: bool,
    /// Set once the closing cue has been raised.
    closed: bool,
    /// The course as one run, when a skin is chosen to draw it. Without one the screen is the
    /// built-in one and none of this is read.
    skin: Option<Box<CourseSkinRun>>,
    status: SkinStatus,
    scene: Scene,
}

impl CourseResultState {
    /// Read the finished run. When a skin is chosen for the screen, the course is also taken as one
    /// run out of the records its stages left in `shared`.
    pub(crate) fn of(run: &rbms_course::CourseRun, shared: &AppShared) -> CourseResultState {
        let skin = has_result_scene(shared, SKIN_TYPE_COURSE_RESULT).then(|| Box::new(CourseSkinRun::of(run, shared)));
        let mut scene = Scene::of_course();
        if let Some(skin) = &skin {
            scene.gauge_type = skin.finished_gauge();
        }
        CourseResultState {
            rows: crate::course_ui::result_rows(run),
            cleared: run.failed_at.is_none(),
            course_name: run.course.name.clone(),
            announced: false,
            closed: false,
            skin,
            status: SkinStatus::Gone,
            scene,
        }
    }

    /// The rows this screen lists, for the tests that check what the run put on it.
    #[cfg(test)]
    pub(crate) fn rows(&self) -> &[(String, String)] {
        &self.rows
    }

    /// Whether the course cleared, for the same reason.
    #[cfg(test)]
    pub(crate) fn cleared(&self) -> bool {
        self.cleared
    }

    /// Play the arrival cue, once. A skin's scene plays the course's own cue or else the single
    /// result's; the built-in screen has only ever played the course's own.
    fn announce(&mut self, shared: &mut AppShared) {
        if std::mem::replace(&mut self.announced, true) {
            return;
        }
        let cue = match (self.status, self.cleared) {
            (SkinStatus::Ready, true) => Cues::of(shared).clear,
            (SkinStatus::Ready, false) => Cues::of(shared).fail,
            (_, cleared) => course_result_sound(cleared),
        };
        shared.play_system_sound(cue);
    }

    /// Play the closing cue where the fade begins, once, silencing the arrival cue
    /// (`CourseResult.render`, `CourseResult.input`).
    fn close(&mut self, shared: &mut AppShared) {
        if std::mem::replace(&mut self.closed, true) || !Cues::has_closing(shared) {
            return;
        }
        let cues = Cues::of(shared);
        shared.stop_system_sound(cues.clear);
        shared.stop_system_sound(cues.fail);
        shared.play_system_sound(cues.close);
    }

    /// Draw the skin's frame. `false` when there is none to draw yet.
    fn draw_skin(&self, shared: &AppShared, canvas: &mut Canvas<'_>) -> bool {
        let Some(run) = &self.skin else {
            return false;
        };
        let chart = run.chart();
        let gauge_type = self.scene.gauge_type;
        let data = FrameData { gauge: Some(run.gauge(gauge_type)), series: run.series(), images: ReferenceImages::default(), ..FrameData::default() };
        let scene = ResultScene { gauge_type, ..ResultScene::default() };
        let draw = ResultDraw { view: &run.view, extras: &run.extras, cleared: self.cleared, chart: Some(&chart), scene, run: Some(&run.snapshot), data };
        shared.draw_result_skin(canvas, SKIN_TYPE_COURSE_RESULT, &draw)
    }

    /// The built-in screen.
    fn draw_built_in(&self, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let x0 = (CW as f32 - PANEL_W) * 0.5;
        let (heading, colour) = match self.cleared {
            true => ("COURSE CLEAR", Color::GREEN),
            false => ("COURSE FAILED", Color::RED),
        };
        draw_text(canvas, x0, HEADING_Y, HEADING_SCALE, colour, heading);
        draw_text(canvas, x0, NAME_Y, NAME_SCALE, th.text, &self.course_name);
        draw_text(canvas, x0, HINT_Y, 1.2, th.text_muted, "ENTER / ESC  BACK TO SELECT");
        for (i, (label, value)) in self.rows.iter().enumerate() {
            let y = ROW_TOP + i as f32 * ROW_PITCH;
            canvas.fill_rect(Rect::new(x0, y, PANEL_W, ROW_H), th.panel);
            draw_text(canvas, x0 + LABEL_INSET, y + TEXT_DROP, ROW_SCALE, th.text_dim, label);
            draw_text_right(canvas, x0 + PANEL_W - VALUE_INSET, y + TEXT_DROP, ROW_SCALE, th.text, value);
        }
    }
}

impl StageHandler for CourseResultState {
    /// One frame of a skin's scene; the built-in screen has nothing to move. The scene is over once
    /// its fade has run, and the course result always leaves for the browser.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let shared = &mut *ctx.shared;
        if self.status == SkinStatus::Gone {
            return Transition::Stay;
        }
        let input = SceneInput { held: held_key_indices(shared), ..SceneInput::default() };
        let frame = run_scene_frame(shared, &mut self.scene, input);
        if frame.fade_began {
            self.close(shared);
        }
        match frame.leave {
            None => Transition::Stay,
            Some(_) => end_course(shared),
        }
    }

    /// Keys on the course result screen. A skin's scene keeps them until it listens; the built-in
    /// screen leaves on the same keys the single-chart result screen does, landing back in the
    /// browser with the player's own settings restored; and a screen still waiting for its skin
    /// takes Escape alone.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        match self.status {
            SkinStatus::Gone if key.pressed && matches!(key.code, KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter) => end_course(ctx.shared),
            SkinStatus::Reading if key.pressed && key.code == KeyCode::Escape => end_course(ctx.shared),
            SkinStatus::Reading | SkinStatus::Ready => {
                self.scene.key(&key);
                Transition::Stay
            }
            SkinStatus::Gone => Transition::Stay,
        }
    }

    /// The arrival cue is raised here unless a skin's scene is on its way to raise it.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        let shared = &mut *ctx.shared;
        self.status = if self.skin.is_some() && has_result_scene(shared, SKIN_TYPE_COURSE_RESULT) { SkinStatus::Reading } else { SkinStatus::Gone };
        if self.status == SkinStatus::Gone {
            self.announce(shared);
        }
    }

    /// A skin's scene leaves in silence, its cues stopped where the reference stops them
    /// (`CourseResult.shutdown`). The built-in screen plays its closing cue on the way out, as it
    /// always did.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        if !self.scene.begun {
            ctx.shared.play_system_sound(SystemSound::CourseClose);
            return;
        }
        let cues = Cues::of(ctx.shared);
        for sound in [cues.clear, cues.fail, cues.close] {
            ctx.shared.stop_system_sound(sound);
        }
    }

    /// The skin's frame, black while one is on its way, and the built-in screen when there is none.
    /// The frame the skin first draws on is the one the scene begins on: the clock goes back to
    /// zero, the score timers go on and the arrival cue is played.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let shared = &mut *ctx.shared;
        if self.skin.is_none() {
            self.draw_built_in(canvas);
            return;
        }
        let start = begin_scene_with_skin(shared, canvas, SKIN_TYPE_COURSE_RESULT, &mut self.scene);
        self.status = start.status;
        if start.announces() {
            self.announce(shared);
        }
        if self.draw_skin(shared, canvas) {
            return;
        }
        if self.status == SkinStatus::Reading {
            canvas.native().clear(Color::BLACK);
            return;
        }
        self.draw_built_in(canvas);
    }
}

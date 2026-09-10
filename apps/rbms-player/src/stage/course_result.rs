//! The COURSE RESULT screen: what a whole course added up to once its last stage ended.
//!
//! A course is played through the same three screens a single chart is, so this screen exists only
//! for the tally that spans them: the summed EX, the combined miss count, the gauge the run carried
//! out of its last stage, and the trophy that tally earns. The rows themselves are settled by
//! [`crate::course_ui::result_rows`], which is what keeps the reference's ordering in one place.
#![allow(clippy::wildcard_imports)]

use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::syssound::course_result_sound;
use crate::*;

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
}

impl CourseResultState {
    /// Read the finished run.
    pub(crate) fn of(run: &rbms_course::CourseRun) -> CourseResultState {
        CourseResultState {
            rows: crate::course_ui::result_rows(run),
            cleared: run.failed_at.is_none(),
            course_name: run.course.name.clone(),
            announced: false,
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
}

impl StageHandler for CourseResultState {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    /// The same keys the single-chart result screen leaves on, landing back in the browser with the
    /// player's own settings restored.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if !key.pressed {
            return Transition::Stay;
        }
        match key.code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => crate::end_course(ctx.shared),
            _ => Transition::Stay,
        }
    }

    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        if !self.announced {
            self.announced = true;
            ctx.shared.play_system_sound(course_result_sound(self.cleared));
        }
    }

    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        ctx.shared.play_system_sound(SystemSound::CourseClose);
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
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
        let _ = ctx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_course::{Course, CourseChart, CourseRun, StageResult};

    fn course(stages: usize) -> Course {
        let mut course = Course {
            name: "TEST COURSE".to_string(),
            charts: (0..stages).map(|i| CourseChart { md5: format!("{i:032x}"), sha256: String::new(), title: format!("stage {i}") }).collect(),
            ..Course::default()
        };
        assert!(course.validate(), "the fixture course is valid");
        course
    }

    fn cleared_stage() -> StageResult {
        StageResult { ex_score: 100, max_ex_score: 200, notes: 100, gauge_value: 80.0, clear: 5, survived: true, ..StageResult::default() }
    }

    fn failed_stage() -> StageResult {
        StageResult { ex_score: 10, max_ex_score: 200, notes: 100, gauge_value: 0.0, clear: 1, survived: false, ..StageResult::default() }
    }

    #[test]
    fn a_course_that_reached_the_end_reads_as_cleared() {
        let mut run = CourseRun::new(course(2), 100.0);
        run.advance(&cleared_stage());
        run.advance(&cleared_stage());
        let state = CourseResultState::of(&run);
        assert!(state.cleared());
        assert!(state.rows().iter().any(|(label, _)| label == "EX SCORE"));
    }

    #[test]
    fn a_course_that_ended_early_names_the_stage_it_failed_on() {
        let mut run = CourseRun::new(course(3), 100.0);
        run.advance(&cleared_stage());
        run.advance(&failed_stage());
        let state = CourseResultState::of(&run);
        assert!(!state.cleared());
        assert_eq!(state.rows().iter().find(|(label, _)| label == "FAILED AT").map(|(_, value)| value.as_str()), Some("STAGE 2"));
    }
}

//! The RESULT screen: the summary of the run that just ended, plus what the score server said
//! about it.
#![allow(clippy::wildcard_imports)]

use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::*;

/// The result screen's own state: the summary view built when the run ended. The submission status
/// under it lives in [`AppShared`], because the request outlives this screen.
pub(crate) struct ResultState {
    view: ResultView,
}

impl ResultState {
    pub(crate) fn new(view: ResultView) -> ResultState {
        ResultState { view }
    }
}

impl StageHandler for ResultState {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if key.pressed && matches!(key.code, KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter) {
            return ctx.shared.leave_play();
        }
        Transition::Stay
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        canvas.clear_bga();
        render_result_with_palette(canvas, &self.view, &ctx.shared.result_palette);
        for (i, (text, kind)) in ctx.shared.ir_status.lines().iter().enumerate() {
            draw_text(canvas, IR_RESULT_X, IR_RESULT_Y + i as f32 * IR_RESULT_LINE_H, IR_RESULT_SCALE, ir_line_color(*kind), text);
        }
    }
}

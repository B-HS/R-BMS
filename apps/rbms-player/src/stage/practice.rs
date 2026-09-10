//! The PRACTICE panel: the range of a chart to repeat and the terms it is repeated under.
//!
//! The panel is a screen of its own rather than a mode of the browser, because it needs the parsed
//! chart — the range it edits is bounded by the chart's last timeline, and the TOTAL row starts
//! from the chart's own. So the practice key loads the chart exactly as PLAY does and lands here
//! instead, and a slice started from here returns here rather than to the result screen.
//!
//! Nothing on this screen is recorded or submitted: [`crate::practice::practice_block_reason`] is
//! folded into the IR gate, so a practice run updates no best and reaches no score server.
#![allow(clippy::wildcard_imports)]

use crate::practice::{PracticeElement, PracticePanel, practice_path};
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::*;

/// Width of the panel the rows are laid out in.
const PANEL_W: f32 = 560.0;

/// Where the first row sits and how far apart the rows are.
const ROW_TOP: f32 = 150.0;
const ROW_PITCH: f32 = 36.0;

/// Height of one row's plate.
const ROW_H: f32 = 30.0;

/// Text scales of the heading, the hint line and the rows.
const HEADING_SCALE: f32 = 3.0;
const HINT_SCALE: f32 = 1.2;
const ROW_SCALE: f32 = 1.4;

/// Baselines of the heading and the two hint lines.
const HEADING_Y: f32 = 50.0;
const TITLE_Y: f32 = 92.0;
const HINT_Y: f32 = 116.0;

/// Inset of a row's label from the plate's left edge, and of its value from the right.
const LABEL_INSET: f32 = 16.0;
const VALUE_INSET: f32 = 16.0;

/// How far a row's text sits below the top of its plate.
const TEXT_DROP: f32 = 7.0;

/// The PRACTICE screen's own state: the panel being edited, and the chart title it was opened on.
pub(crate) struct PracticeState {
    panel: PracticePanel,
    title: String,
    /// Either shift key is down, which is what makes a left/right step the coarse one.
    turbo: bool,
}

impl PracticeState {
    /// Open the panel on a chart that has already been loaded.
    pub(crate) fn new(panel: PracticePanel, title: String) -> PracticeState {
        PracticeState { panel, title, turbo: false }
    }

    /// The panel this screen edits, for the tests that drive it.
    #[cfg(test)]
    pub(crate) fn panel(&self) -> &PracticePanel {
        &self.panel
    }

    /// Start the slice the panel describes, or stay put when the chart is no longer loaded.
    fn start_slice(&mut self, shared: &mut AppShared) -> Transition {
        match shared.start_practice_slice(&mut self.panel) {
            Some(play) => Transition::Open(Stage::Play(Box::new(play))),
            None => {
                notify(Level::Warn, "practice chart is no longer loaded".to_string());
                Transition::Back
            }
        }
    }
}

impl StageHandler for PracticeState {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    /// Up/down move the cursor, left/right change the focused row (Shift steps faster), Enter
    /// starts the slice and Escape leaves the chart.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if matches!(key.code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.turbo = key.pressed;
            return Transition::Stay;
        }
        if !key.pressed {
            return Transition::Stay;
        }
        let turbo = self.turbo;
        match key.code {
            KeyCode::Escape => Transition::Back,
            KeyCode::Enter | KeyCode::NumpadEnter => self.start_slice(ctx.shared),
            KeyCode::ArrowUp => {
                self.panel.move_cursor(false);
                Transition::Stay
            }
            KeyCode::ArrowDown => {
                self.panel.move_cursor(true);
                Transition::Stay
            }
            KeyCode::ArrowLeft => {
                self.panel.adjust(false, turbo, false);
                Transition::Stay
            }
            KeyCode::ArrowRight => {
                self.panel.adjust(true, turbo, false);
                Transition::Stay
            }
            _ => Transition::Stay,
        }
    }

    /// The panel keeps its values between visits, so what the player set up is written out on the
    /// way out rather than on every keystroke.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.panel.finish();
        ctx.shared.practice_book.put(self.panel.chart_key(), self.panel.property.clone());
        ctx.shared.practice_book.save(&practice_path(&ctx.shared.settings_path));
    }

    /// The panel holds every key it is drawn with, so the option overlay does not open over it.
    fn holds_keys(&self, _ctx: &FrameCtx<'_>) -> bool {
        true
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let x0 = (CW as f32 - PANEL_W) * 0.5;
        draw_text(canvas, x0, HEADING_Y, HEADING_SCALE, th.text, "PRACTICE");
        draw_text(canvas, x0, TITLE_Y, ROW_SCALE, th.text_dim, &self.title);
        draw_text(canvas, x0, HINT_Y, HINT_SCALE, th.text_muted, "UP DOWN ROW   LEFT RIGHT VALUE   ENTER START   ESC BACK");
        let focused = self.panel.focused();
        for (i, element) in PracticeElement::ALL.into_iter().enumerate() {
            let y = ROW_TOP + i as f32 * ROW_PITCH;
            let on = element == focused;
            canvas.fill_rect(Rect::new(x0, y, PANEL_W, ROW_H), if on { th.button } else { th.panel });
            let colour = if on { th.text } else { th.text_dim };
            draw_text(canvas, x0 + LABEL_INSET, y + TEXT_DROP, ROW_SCALE, colour, element.label());
            draw_text_right(canvas, x0 + PANEL_W - VALUE_INSET, y + TEXT_DROP, ROW_SCALE, colour, &self.panel.value_text(element));
        }
        let unapplied = ROW_TOP + PracticeElement::ALL.len() as f32 * ROW_PITCH + ROW_PITCH * 0.5;
        draw_text(canvas, x0, unapplied, HINT_SCALE, Color::YELLOW, "GAUGE VALUE AND FREQUENCY ARE STORED BUT NOT YET APPLIED");
        let _ = ctx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::render_tests::app;

    fn ctx(app: &mut crate::App) -> FrameCtx<'_> {
        FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
    }

    fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    fn state() -> PracticeState {
        let panel = PracticePanel::new("md5".to_string(), MODE, 60_000, None, 300.0);
        PracticeState::new(panel, "a chart".to_string())
    }

    #[test]
    fn the_cursor_walks_every_row_of_the_panel() {
        let mut app = app();
        let mut screen = state();
        let mut seen = vec![screen.panel().focused()];
        for _ in 1..PracticeElement::ALL.len() {
            screen.handle_key(&mut ctx(&mut app), press(KeyCode::ArrowDown));
            seen.push(screen.panel().focused());
        }
        assert_eq!(seen, PracticeElement::ALL.to_vec(), "down steps through the rows in order");
    }

    #[test]
    fn escape_leaves_the_panel_without_starting_a_slice() {
        let mut app = app();
        let mut screen = state();
        assert!(matches!(screen.handle_key(&mut ctx(&mut app), press(KeyCode::Escape)), Transition::Back));
    }

    #[test]
    fn leaving_the_panel_writes_its_values_into_the_book() {
        let mut app = app();
        let mut screen = state();
        screen.handle_key(&mut ctx(&mut app), press(KeyCode::ArrowRight));
        screen.on_exit(&mut ctx(&mut app));
        assert!(app.shared.practice_book.get("md5").is_some(), "the chart's range is remembered");
    }

    /// Starting a slice with no chart loaded must not panic; the panel reports and leaves.
    #[test]
    fn a_slice_started_with_no_chart_loaded_leaves_the_panel() {
        let mut app = app();
        let mut screen = state();
        assert!(matches!(screen.handle_key(&mut ctx(&mut app), press(KeyCode::Enter)), Transition::Back));
    }
}

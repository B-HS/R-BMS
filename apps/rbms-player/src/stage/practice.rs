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

use rbms_render::skin_render::state::PracticeRows;

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
    slice_open: bool,
}

impl PracticeState {
    /// Open the panel on a chart that has already been loaded.
    pub(crate) fn new(panel: PracticePanel, title: String) -> PracticeState {
        PracticeState { panel, title, turbo: false, slice_open: false }
    }

    /// The panel this screen edits, for the tests that drive it.
    #[cfg(test)]
    pub(crate) fn panel(&self) -> &PracticePanel {
        &self.panel
    }

    /// The panel's rows as a play document reads them: a label and a value per row, and which one
    /// the cursor is on.
    fn rows(&self) -> (Vec<&'static str>, Vec<String>, usize) {
        let labels: Vec<&'static str> = PracticeElement::ALL.into_iter().map(PracticeElement::label).collect();
        let values: Vec<String> = PracticeElement::ALL.into_iter().map(|element| self.panel.value_text(element)).collect();
        let focused = PracticeElement::ALL.into_iter().position(|element| element == self.panel.focused()).unwrap_or(0);
        (labels, values, focused)
    }

    /// Draws the panel with the play document of the chart's mode, when that document declares a
    /// practice pane. `false` when it does not, and the panel draws its own rows.
    ///
    /// The document is read and compiled here rather than by the play screen, because a player may
    /// reach this panel without ever having started a chart in this session.
    fn draw_with_document(&self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) -> bool {
        let Some(screen) = rbms_skin::loader::mode_skin_type(ctx.shared.mode) else {
            return false;
        };
        ctx.shared.prepare_skin(canvas, screen);
        let Some((screen, visible)) = ctx.shared.practice_document() else {
            return false;
        };
        let (labels, values, focused) = self.rows();
        let visible = if visible == 0 { labels.len() } else { visible };
        let rows = PracticeRows { labels: &labels, values: &values, focused, visible };
        ctx.shared.draw_practice_skin(canvas, screen, &rows)
    }

    /// Start the slice the panel describes, or stay put when the chart is no longer loaded.
    fn start_slice(&mut self, shared: &mut AppShared) -> Transition {
        match shared.start_practice_slice(&mut self.panel) {
            Some(play) => {
                self.slice_open = true;
                Transition::Open(Stage::Play(Box::new(play)))
            }
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
        if !self.slice_open {
            ctx.shared.release_practice_chart();
        }
    }

    fn on_enter(&mut self, _ctx: &mut FrameCtx<'_>) {
        self.slice_open = false;
    }

    /// The panel holds every key it is drawn with, so the option overlay does not open over it.
    fn holds_keys(&self, _ctx: &FrameCtx<'_>) -> bool {
        true
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        canvas.clear_bga();
        if self.draw_with_document(ctx, canvas) {
            return;
        }
        let th = rbms_render::theme();
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

    /// The panel publishes one label and one value for every row it edits, in the order the cursor
    /// walks them, which is what the document's row objects read.
    #[test]
    fn the_panel_publishes_one_label_and_value_per_row_with_the_cursor_on_the_focused_one() {
        let screen = state();
        let (labels, values, focused) = screen.rows();
        assert_eq!(labels.len(), PracticeElement::ALL.len());
        assert_eq!(values.len(), PracticeElement::ALL.len());
        assert_eq!(labels[0], PracticeElement::ALL[0].label());
        assert_eq!(focused, 0, "a fresh panel starts on its first row");
    }

    /// Moving the cursor moves which row is reported as focused, so a document highlights the row
    /// the keys are editing rather than a fixed one.
    #[test]
    fn the_focused_row_the_panel_reports_follows_the_cursor() {
        let mut app = app();
        let mut screen = state();
        screen.handle_key(&mut ctx(&mut app), press(KeyCode::ArrowDown));
        screen.handle_key(&mut ctx(&mut app), press(KeyCode::ArrowDown));
        let (_, _, focused) = screen.rows();
        assert_eq!(focused, 2);
    }
}

/// The practice panel drawn with the shipped seven-key document, which is the one that declares a
/// practice pane.
#[cfg(test)]
mod document_tests {
    use rbms_model::Mode;
    use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;

    use super::*;
    use crate::stage::render_tests::render_into;
    use crate::stage::render_tests_skin::bundled_app;
    use crate::stage::{HeadlessCanvas, Stage};
    use crate::{SkinConfig, assets};

    /// How many frames a document is allowed to take to be read and compiled.
    const DOCUMENT_LOAD_FRAMES: usize = 240;

    /// The rectangle the seven-key document lays its practice rows out in, measured down from the
    /// head of the screen: the plate runs from document y 120 to 600 of a 720-high document, which
    /// is screen rows 120 to 600, and the rows themselves are inside that.
    const ROWS_PROBE: (u32, u32, u32, u32) = (360, 160, 560, 400);

    /// How light a pixel has to be to count as ink a row put there.
    const INK_TOTAL: u16 = 300;

    /// Whether any pixel of `rect` is light enough to be ink.
    fn has_lit_pixel(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
        let (x, y, w, h) = rect;
        (y..y + h).any(|py| {
            (x..x + w).any(|px| {
                let pixel = pixels.pixel_at(px, py);
                u16::from(pixel.r) + u16::from(pixel.g) + u16::from(pixel.b) >= INK_TOTAL
            })
        })
    }

    /// An app with the bundle installed, on a seven-key chart, with the panel open on it.
    fn practising(tag: &str) -> (App, HeadlessCanvas) {
        let (mut app, settings) = bundled_app(tag);
        app.shared.mode = Mode::BEAT_7K;
        app.shared.skin_cfg =
            SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, Mode::BEAT_7K)).expect("the bundled play layout parses");
        app.shared.rebuild_skin();
        let mut pixels = HeadlessCanvas::new(CW, CH);
        let panel = || {
            let panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, 60_000, None, 300.0);
            Stage::Practice(Box::new(PracticeState::new(panel, "a chart".to_string())))
        };
        for _ in 0..DOCUMENT_LOAD_FRAMES {
            render_into(&mut app, panel(), &mut pixels);
            if app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS) {
                render_into(&mut app, panel(), &mut pixels);
                break;
            }
        }
        assert!(app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS), "the seven-key document never finished compiling");
        (app, pixels)
    }

    /// Saves the frame for the eye to check, when a capture directory was asked for.
    fn save(name: &str, pixels: &HeadlessCanvas) {
        let Some(directory) = std::env::var_os("RBMS_SKIN_CAPTURE_DIR") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("create the capture folder");
        let image = image::RgbaImage::from_fn(CW, CH, |x, y| {
            let pixel = pixels.pixel_at(x, y);
            image::Rgba([pixel.r, pixel.g, pixel.b, pixel.a])
        });
        image.save(directory.join(format!("{name}.png"))).expect("save the capture");
    }

    /// The shipped seven-key document declares a practice pane, so the panel is drawn with it and
    /// the rows land inside the rectangle the document laid out for them.
    #[test]
    fn the_practice_panel_is_drawn_with_the_seven_key_document() {
        let (app, pixels) = practising("v3-practice-7k");
        let (screen, visible) = app.shared.practice_document().expect("the seven-key document declares a practice pane");
        assert_eq!(screen, SKIN_TYPE_PLAY_7KEYS);
        assert_eq!(visible, 12, "the document asks for twelve rows");
        assert!(has_lit_pixel(&pixels, ROWS_PROBE), "the document's practice rows did not reach the screen");
        save("practice-7k", &pixels);
    }

    /// The rows the document draws are gated on the practice screen being open, so a run drawn with
    /// the same document shows none of them.
    #[test]
    fn the_practice_rows_do_not_draw_over_a_run() {
        let (mut app, mut pixels) = practising("v3-practice-not-in-play");
        render_into(&mut app, Stage::Play(Box::new(crate::stage::render_tests::play_state())), &mut pixels);
        let hits = (ROWS_PROBE.1..ROWS_PROBE.1 + ROWS_PROBE.3)
            .flat_map(|y| (ROWS_PROBE.0..ROWS_PROBE.0 + ROWS_PROBE.2).map(move |x| (x, y)))
            .filter(|(x, y)| {
                let pixel = pixels.pixel_at(*x, *y);
                pixel == rbms_render::Color::rgb(7, 9, 18)
            })
            .count();
        assert_eq!(hits, 0, "the practice plate drew over a run");
    }
}

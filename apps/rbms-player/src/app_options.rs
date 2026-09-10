//! The option overlay: the panel the browser puts over itself for the values that change from one
//! chart to the next.
//!
//! It sits above every screen's own key handling rather than inside the browser, because the whole
//! point of it is that the list underneath does not move while it is open. [`options_key`] is
//! offered every key before the screen that is up sees it, and answers whether it took it.
//!
//! What the panel edits is the configuration itself — the same fields the settings screen edits, by
//! way of the same descriptor table — so there is one answer to "what will the next run be played
//! with" rather than two that can disagree. The panel is a shorter route to a handful of those rows,
//! not a second copy of them.

use rbms_config::{AdjustOutcome, SettingId, adjust, descriptor, display_value};
use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_right, text_width, theme};
use winit::keyboard::KeyCode;

use crate::stage::{Canvas, FrameCtx, KeyInput, StageId};
use crate::{AppShared, CH, CW};

/// The rows the panel shows, top to bottom: what a player changes for one chart and then changes
/// back. Everything else stays on the settings screen.
///
/// LANE OPTION is deliberately not here. Its two battle values have to reshape the chart before the
/// run starts, which nothing does yet, and a row that quietly does nothing would read as a chosen
/// option rather than an unbuilt one. The settings screen leaves it out for the same reason
/// (`rbms_config::settings::unbuilt_row`), so neither route offers it until the run applies it.
const OPTION_ROWS: [SettingId; 11] = [
    SettingId::Random,
    SettingId::Gauge,
    SettingId::HiSpeed,
    SettingId::FixHiSpeed,
    SettingId::LaneCover,
    SettingId::Lift,
    SettingId::Hidden,
    SettingId::ScratchSide,
    SettingId::ScratchAuto,
    SettingId::Autoplay,
    SettingId::Target,
];

/// One step to the right on a row.
const ROW_STEP_FORWARD: i32 = 1;

/// One step to the left on a row.
const ROW_STEP_BACK: i32 = -1;

/// Width of the panel.
const PANEL_W: f32 = 420.0;

/// Distance from the right edge of the screen to the panel.
const PANEL_MARGIN_X: f32 = 40.0;

/// Space above and below the rows inside the panel.
const PANEL_PAD_Y: f32 = 18.0;

/// Space either side of a row inside the panel.
const PANEL_PAD_X: f32 = 16.0;

/// Height of the panel's title strip.
const TITLE_H: f32 = 40.0;

/// Height of the panel's keyboard hint strip.
const HINT_H: f32 = 26.0;

/// Vertical distance from one row to the next.
const ROW_PITCH: f32 = 34.0;

/// Height of a row's highlight.
const ROW_H: f32 = 28.0;

/// Text scale of the panel title.
const TITLE_SCALE: f32 = 2.0;

/// Text scale of a row's label and value.
const ROW_SCALE: f32 = 1.6;

/// Text scale of the keyboard hint.
const HINT_SCALE: f32 = 1.1;

/// Opacity of the panel behind its rows, so the list underneath still reads as the thing being
/// played from.
const PANEL_ALPHA: u8 = 235;

/// Title of the panel.
const OPTIONS_TITLE: &str = "OPTIONS";

/// Keyboard hint along the bottom of the panel.
const OPTIONS_HINT: &str = "UP DOWN MOVE   LEFT RIGHT CHANGE   F1 PIN   ESC CLOSE";

/// The key held to keep the panel up, which closes it again when it is let go.
const HOLD_KEY: KeyCode = KeyCode::ShiftLeft;

/// The key that pins the panel open, for a player who would rather not hold one down.
const TOGGLE_KEY: KeyCode = KeyCode::F1;

/// Whether the overlay is open, the row it is on, and whether a key is being held to keep it up.
///
/// The values it edits are not held here — they are read from and written straight back to the
/// configuration, the same fields the settings screen edits, so the two can never disagree about
/// what the next run will be played with.
#[derive(Debug, Default)]
pub(crate) struct OptionsOverlay {
    open: bool,
    /// Set while the panel is up because a key is being held down, as opposed to pinned open.
    held: bool,
    row: usize,
    /// Set once a row has moved, so closing the panel writes the configuration out exactly when
    /// there is something to write.
    dirty: bool,
}

impl OptionsOverlay {
    /// Whether the panel is up.
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// The row the panel is on.
    #[cfg(test)]
    pub(crate) fn row(&self) -> SettingId {
        OPTION_ROWS[self.row]
    }

    fn open_panel(&mut self, held: bool) {
        self.open = true;
        self.held = held;
    }

    fn close_panel(&mut self) {
        self.open = false;
        self.held = false;
    }

    fn move_row(&mut self, delta: i32) {
        let len = OPTION_ROWS.len() as i32;
        self.row = ((self.row as i32 + delta).rem_euclid(len)) as usize;
    }
}

/// Which screens the overlay may be opened over. Only the browser: it is the screen you pick a
/// chart from, and opening it over a running chart would take the keys the run needs.
pub(crate) fn opens_over(stage: StageId) -> bool {
    stage == StageId::Select
}

/// Offer one key to the overlay before the screen that is up sees it, answering whether the overlay
/// took it.
///
/// A closed panel takes only the two keys that open it, so every other key reaches the screen
/// exactly as it did. An open one takes everything: the list underneath must not move while the
/// panel is being read, which is the whole reason the panel is offered keys first.
///
/// `screen_holds_keys` is the screen underneath saying it is taking every key itself — a search box
/// being typed into, a modal, a panel of its own. The key that opens this panel is a shift key, and
/// a shift key held to type a capital letter is not a request for the option panel, so a closed
/// panel stays closed while the screen is in that state.
pub(crate) fn options_key(ctx: &mut FrameCtx<'_>, stage: StageId, screen_holds_keys: bool, key: &KeyInput<'_>) -> bool {
    if !opens_over(stage) {
        close(ctx.shared);
        return false;
    }
    if !ctx.shared.options.is_open() {
        return !screen_holds_keys && open_key(ctx, key);
    }
    if key.released {
        if key.code == HOLD_KEY && ctx.shared.options.held {
            close(ctx.shared);
        }
        return true;
    }
    if !key.pressed {
        return true;
    }
    match key.code {
        KeyCode::Escape | TOGGLE_KEY => close(ctx.shared),
        KeyCode::ArrowUp => ctx.shared.options.move_row(ROW_STEP_BACK),
        KeyCode::ArrowDown => ctx.shared.options.move_row(ROW_STEP_FORWARD),
        KeyCode::ArrowLeft => step(ctx, ROW_STEP_BACK),
        KeyCode::ArrowRight | KeyCode::Enter => step(ctx, ROW_STEP_FORWARD),
        _ => {}
    }
    true
}

/// The two keys that open a closed panel: one held down, one that pins it.
///
/// The held one is taken from the browser rather than passed along, so the browser never sees it go
/// down and never treats it as a modifier. The browser watches both shift keys, so the right one is
/// what still reaches it as one.
fn open_key(ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> bool {
    if !key.pressed {
        return false;
    }
    match key.code {
        HOLD_KEY => ctx.shared.options.open_panel(true),
        TOGGLE_KEY => ctx.shared.options.open_panel(false),
        _ => return false,
    }
    true
}

/// Move the focused row one step and remember that something changed.
///
/// A row that opens a screen or reaches something only the running program can see is left alone:
/// the panel is a shorter route to values, and a screen opened from over the browser would leave the
/// panel up behind it.
fn step(ctx: &mut FrameCtx<'_>, delta: i32) {
    let id = OPTION_ROWS[ctx.shared.options.row];
    if descriptor(id).host_value {
        return;
    }
    if adjust(&mut ctx.shared.config, id, delta) == AdjustOutcome::Changed {
        ctx.shared.options.dirty = true;
    }
}

/// Close the panel, writing the configuration out when a row moved while it was up.
///
/// Called on the way off the browser as well as from the panel's own keys, so a panel left pinned
/// open cannot survive into a running chart and be drawn over the playfield.
pub(crate) fn close(shared: &mut AppShared) {
    if !shared.options.is_open() {
        return;
    }
    let dirty = std::mem::take(&mut shared.options.dirty);
    shared.options.close_panel();
    if dirty {
        shared.rebuild_skin();
        shared.save_settings();
    }
}

/// Draw the panel over the screen that is up.
///
/// Nothing is drawn while the overlay is closed, which is the point: every screen's golden frame is
/// the same with this call in place as it was without it. Nothing is drawn over a screen the panel
/// does not belong to either, so a panel that somehow outlived the browser cannot appear over a
/// running chart.
pub(crate) fn draw(stage: StageId, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
    if !ctx.shared.options.is_open() || !opens_over(stage) {
        return;
    }
    let th = theme();
    let rows = OPTION_ROWS.len() as f32;
    let h = TITLE_H + PANEL_PAD_Y * 2.0 + rows * ROW_PITCH + HINT_H;
    let x = CW as f32 - PANEL_MARGIN_X - PANEL_W;
    let y = (CH as f32 - h) * 0.5;
    canvas.fill_rect(Rect::new(x, y, PANEL_W, h), Color { a: PANEL_ALPHA, ..th.panel });
    draw_text(canvas, x + PANEL_PAD_X, y + PANEL_PAD_Y, TITLE_SCALE, th.text, OPTIONS_TITLE);

    let mut ry = y + TITLE_H + PANEL_PAD_Y;
    for (index, id) in OPTION_ROWS.into_iter().enumerate() {
        let focused = index == ctx.shared.options.row;
        if focused {
            canvas.fill_rect(Rect::new(x + PANEL_PAD_X * 0.5, ry - 4.0, PANEL_W - PANEL_PAD_X, ROW_H), th.row_focus);
        }
        let label_color = if focused { Color::YELLOW } else { th.text_dim };
        draw_text(canvas, x + PANEL_PAD_X, ry, ROW_SCALE, label_color, descriptor(id).label);
        let value = display_value(&ctx.shared.config, id);
        draw_text_right(canvas, x + PANEL_W - PANEL_PAD_X, ry, ROW_SCALE, th.text, &value);
        ry += ROW_PITCH;
    }
    let hint_x = x + (PANEL_W - text_width(OPTIONS_HINT, HINT_SCALE)) * 0.5;
    draw_text(canvas, hint_x, y + h - HINT_H, HINT_SCALE, th.text_muted, OPTIONS_HINT);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{HeadlessCanvas, SelectState, Stage};
    use crate::{App, Config, LaunchOptions};

    fn app() -> App {
        let dir = std::env::temp_dir().join(format!("rbms-app-options-tests-{}", std::process::id()));
        App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron"))
    }

    fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    fn release(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: false, released: true, text: None }
    }

    /// Send one key to the overlay over the browser and report whether it took it.
    fn offer(app: &mut App, key: KeyInput<'_>) -> bool {
        let now = std::time::Instant::now();
        let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
        options_key(&mut ctx, StageId::Select, false, &key)
    }

    #[test]
    fn the_overlay_starts_closed() {
        assert!(!OptionsOverlay::default().is_open());
    }

    /// The overlay belongs to the screen you choose a chart from. Over a running chart it would
    /// take keys the run itself needs, so it is not offered them.
    #[test]
    fn the_overlay_is_only_offered_keys_on_the_browser() {
        for stage in StageId::ALL {
            assert_eq!(opens_over(stage), stage == StageId::Select, "{stage:?}");
        }
    }

    /// A closed panel takes only the keys that open it, so every other key reaches the screen it
    /// always did.
    #[test]
    fn a_closed_overlay_takes_no_key_but_the_two_that_open_it() {
        let mut closed = app();
        for code in [KeyCode::Escape, KeyCode::Enter, KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::F2, KeyCode::F3, KeyCode::KeyO] {
            assert!(!offer(&mut closed, press(code)), "a closed panel took {code:?}");
            assert!(!closed.shared.options.is_open());
        }
        for code in [HOLD_KEY, TOGGLE_KEY] {
            let mut opener = app();
            assert!(offer(&mut opener, press(code)), "{code:?} does not open the panel");
            assert!(opener.shared.options.is_open());
        }
    }

    /// The seam sits in front of the browser's own key handling, so a key it does not take has to
    /// come out the other side and move the list exactly as it did.
    #[test]
    fn a_key_the_overlay_does_not_take_still_reaches_the_screen() {
        let mut app = app();
        app.stage = Stage::Select(Box::new(SelectState::new()));
        app.shared.select_items = vec![
            crate::SelectItem::Folder { label: "one".into(), target: crate::SelectView::AllSongs },
            crate::SelectItem::Folder { label: "two".into(), target: crate::SelectView::AllSongs },
        ];
        app.shared.sel = 0;
        let now = std::time::Instant::now();
        let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
        app.stage.handle_key(&mut ctx, press(KeyCode::ArrowDown));
        assert_eq!(app.shared.sel, 1, "the browser did not see the arrow key");
    }

    /// The list must not move while the panel is up, which is the whole reason the panel is offered
    /// keys before the screen underneath.
    #[test]
    fn an_open_overlay_keeps_the_list_still() {
        let mut app = app();
        app.stage = Stage::Select(Box::new(SelectState::new()));
        app.shared.select_items = vec![
            crate::SelectItem::Folder { label: "one".into(), target: crate::SelectView::AllSongs },
            crate::SelectItem::Folder { label: "two".into(), target: crate::SelectView::AllSongs },
        ];
        app.shared.sel = 0;
        app.shared.options.open_panel(false);
        let now = std::time::Instant::now();
        for code in [KeyCode::ArrowDown, KeyCode::Enter] {
            let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
            app.stage.handle_key(&mut ctx, press(code));
        }
        assert_eq!(app.shared.sel, 0, "the browser moved while the panel was up");
    }

    /// A held panel is up for as long as the key is down and no longer; a pinned one ignores that
    /// key going up, which is what pinning it is for.
    #[test]
    fn a_held_panel_closes_on_release_and_a_pinned_one_does_not() {
        let mut app = app();
        offer(&mut app, press(HOLD_KEY));
        assert!(app.shared.options.is_open());
        assert!(offer(&mut app, release(HOLD_KEY)), "the release belongs to the panel");
        assert!(!app.shared.options.is_open(), "letting the key go closes a held panel");

        offer(&mut app, press(TOGGLE_KEY));
        assert!(app.shared.options.is_open());
        offer(&mut app, release(HOLD_KEY));
        assert!(app.shared.options.is_open(), "a pinned panel is not held by that key");
        offer(&mut app, press(TOGGLE_KEY));
        assert!(!app.shared.options.is_open(), "the same key closes it again");
    }

    /// The held key is taken rather than passed along, so the browser never sees it as a modifier
    /// while the panel is up. The other shift key is untouched and is what still reaches it.
    #[test]
    fn the_held_key_is_taken_from_the_browser_and_the_other_shift_is_not() {
        let mut app = app();
        assert!(offer(&mut app, press(HOLD_KEY)), "the browser must not also see the key that opens the panel");
        close(&mut app.shared);
        assert!(!offer(&mut app, press(KeyCode::ShiftRight)), "the other shift stays the browser's own modifier");
        assert!(!app.shared.options.is_open());
    }

    /// Escape closes the panel rather than reaching the browser, which would take the player out of
    /// the folder they are in.
    #[test]
    fn escape_closes_the_panel_and_goes_no_further() {
        let mut app = app();
        offer(&mut app, press(TOGGLE_KEY));
        assert!(offer(&mut app, press(KeyCode::Escape)), "Escape belongs to the panel while it is up");
        assert!(!app.shared.options.is_open());
    }

    #[test]
    fn the_rows_wrap_at_both_ends() {
        let mut app = app();
        offer(&mut app, press(TOGGLE_KEY));
        assert_eq!(app.shared.options.row(), OPTION_ROWS[0]);
        offer(&mut app, press(KeyCode::ArrowUp));
        assert_eq!(app.shared.options.row(), OPTION_ROWS[OPTION_ROWS.len() - 1], "up from the first row wraps to the last");
        offer(&mut app, press(KeyCode::ArrowDown));
        assert_eq!(app.shared.options.row(), OPTION_ROWS[0], "and back again");
    }

    /// The panel writes the configuration the settings screen reads, so the value it leaves behind
    /// is the one the settings screen then shows.
    #[test]
    fn a_row_moved_on_the_panel_is_the_value_the_settings_screen_shows() {
        let mut app = app();
        offer(&mut app, press(TOGGLE_KEY));
        for _ in 0..OPTION_ROWS.iter().position(|id| *id == SettingId::HiSpeed).expect("the panel shows hi-speed") {
            offer(&mut app, press(KeyCode::ArrowDown));
        }
        let before = app.shared.config.play.hispeed;
        offer(&mut app, press(KeyCode::ArrowRight));
        let after = app.shared.config.play.hispeed;
        assert!(after > before, "the row did not move: {before} to {after}");
        assert_eq!(display_value(&app.shared.config, SettingId::HiSpeed), format!("{after:.2}"), "the panel and the settings screen read the same field");
    }

    /// Every row the panel shows has to be a row the settings screen has, or the two would be
    /// editing different things.
    #[test]
    fn every_row_of_the_panel_is_a_settings_row_that_holds_a_value() {
        for id in OPTION_ROWS {
            let row = descriptor(id);
            assert!(!row.host_value, "{:?} is a row only the running program can fill", id);
            assert!(!matches!(row.kind, rbms_config::SettingKind::Action | rbms_config::SettingKind::FilePick), "{:?} opens something", id);
        }
        let mut seen: Vec<&str> = OPTION_ROWS.into_iter().map(|id| descriptor(id).label).collect();
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "two rows of the panel share a label");
    }

    /// A closed panel draws nothing, so every screen's frame is the same with the overlay wired in
    /// as it was without it.
    #[test]
    fn a_closed_panel_draws_nothing_and_an_open_one_draws() {
        rbms_render::font::use_embedded_fonts_only();
        let mut app = app();
        let now = std::time::Instant::now();
        let mut closed = HeadlessCanvas::new(CW, CH);
        draw(StageId::Select, &mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut Canvas::Headless(&mut closed));
        assert_eq!(closed.quad_count(), 0, "a closed panel drew something");

        app.shared.options.open_panel(false);
        let mut open = HeadlessCanvas::new(CW, CH);
        draw(StageId::Select, &mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut Canvas::Headless(&mut open));
        assert!(open.quad_count() > 0, "an open panel drew nothing");
        assert!(open.painted_pixels() > 0);
    }

    /// Moving the focus has to reach the painted panel, or the highlight is wired to nothing.
    #[test]
    fn the_focused_row_reaches_the_painted_panel() {
        rbms_render::font::use_embedded_fonts_only();
        let mut app = app();
        let now = std::time::Instant::now();
        app.shared.options.open_panel(false);
        let frame = |app: &mut App| {
            let mut canvas = HeadlessCanvas::new(CW, CH);
            draw(StageId::Select, &mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut Canvas::Headless(&mut canvas));
            canvas.pixel_checksum()
        };
        let first = frame(&mut app);
        app.shared.options.move_row(ROW_STEP_FORWARD);
        assert_ne!(first, frame(&mut app), "the focused row does not reach the screen");
    }

    /// Leaving the browser with the panel up must not leave it up over the screen that follows: the
    /// panel takes every key while it is open, and no other screen offers it any.
    #[test]
    fn the_panel_does_not_survive_leaving_the_browser() {
        let mut app = app();
        offer(&mut app, press(TOGGLE_KEY));
        assert!(app.shared.options.is_open());
        let now = std::time::Instant::now();
        let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
        assert!(!options_key(&mut ctx, StageId::Play, false, &press(KeyCode::Escape)), "a running chart keeps its own keys");
        assert!(!app.shared.options.is_open(), "the panel is closed on the way out of the browser");
    }

    /// A chart can be started with the mouse as well as with Enter, and a click that reached the
    /// list through an open panel would leave the panel drawn over the run it started.
    #[test]
    fn a_click_does_not_reach_the_list_through_an_open_panel() {
        let mut app = app();
        app.stage = Stage::Select(Box::new(SelectState::new()));
        app.shared.select_items = vec![
            crate::SelectItem::Folder { label: "one".into(), target: crate::SelectView::AllSongs },
            crate::SelectItem::Folder { label: "two".into(), target: crate::SelectView::AllSongs },
        ];
        app.shared.sel = 0;
        app.shared.hot.push((Rect::new(0.0, 0.0, 100.0, 100.0), crate::Hot::SelectRow(1)));
        let now = std::time::Instant::now();

        app.shared.options.open_panel(false);
        app.stage.handle_mouse(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, (10.0, 10.0));
        assert_eq!(app.shared.sel, 0, "a click reached the list through an open panel");

        close(&mut app.shared);
        app.stage.handle_mouse(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, (10.0, 10.0));
        assert_eq!(app.shared.sel, 1, "the same click has to reach the list once the panel is gone");
    }

    /// Every way off the browser closes the panel, not just the key path: a screen change started by
    /// a click or by a download landing would otherwise carry it into a running chart.
    #[test]
    fn a_stage_change_of_any_kind_closes_the_panel() {
        let mut app = app();
        app.shared.options.open_panel(false);
        app.switch(crate::stage::Transition::To(Stage::Loading(crate::stage::LoadingState::song(0))));
        assert!(!app.shared.options.is_open(), "the panel outlived the browser");
    }

    /// Nothing of the panel reaches a screen it does not belong to, whatever the panel's own flag
    /// says, so a run's frame is the same with the overlay wired in as without it.
    #[test]
    fn the_panel_draws_over_no_screen_but_the_browser() {
        rbms_render::font::use_embedded_fonts_only();
        let mut app = app();
        app.shared.options.open_panel(false);
        let now = std::time::Instant::now();
        for stage in StageId::ALL {
            let mut canvas = HeadlessCanvas::new(CW, CH);
            draw(stage, &mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut Canvas::Headless(&mut canvas));
            assert_eq!(canvas.quad_count() > 0, opens_over(stage), "{stage:?} drew the wrong thing");
        }
    }

    /// The key that opens the panel is a shift key, and a shift key held over a search box is how a
    /// capital letter is typed. A closed panel stays closed while the screen underneath is taking
    /// text, or the letter after it would be eaten by the panel.
    #[test]
    fn the_panel_does_not_open_over_a_screen_that_is_taking_text() {
        let mut app = app();
        let now = std::time::Instant::now();
        for code in [HOLD_KEY, TOGGLE_KEY] {
            let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
            assert!(!options_key(&mut ctx, StageId::Select, true, &press(code)), "{code:?} belongs to what is being typed into");
            assert!(!app.shared.options.is_open());
        }
    }

    /// The browser says which of its own states take every key, and the search box is one of them:
    /// a capital letter typed into it has to reach it rather than open the panel.
    #[test]
    fn a_search_box_being_typed_into_keeps_its_shift_key() {
        let mut app = app();
        app.stage = Stage::Select(Box::new(SelectState::new()));
        app.shared.searching = true;
        let now = std::time::Instant::now();
        app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(HOLD_KEY));
        assert!(!app.shared.options.is_open(), "the panel opened over a search box");
        let typed = KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some("A") };
        app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, typed);
        assert_eq!(app.shared.search, "A", "the letter after the shift key did not reach the search box");
    }
}

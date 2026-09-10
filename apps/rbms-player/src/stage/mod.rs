//! The stages the app can be in and the state each one owns.
//!
//! A stage is one screen of the player: song select, the settings screens, loading, play and
//! result. [`Stage`] is what [`crate::App`] dispatches on; everything that outlives a stage change
//! lives in [`crate::AppShared`] instead. Each screen implements [`StageHandler`], so the frame
//! loop and the window-event handler are a dispatch each rather than one long match per concern.

use std::time::Instant;

use winit::keyboard::KeyCode;

use crate::AppShared;

pub(crate) mod canvas;
pub(crate) mod folders;
pub(crate) mod keyconfig;
pub(crate) mod loading;
pub(crate) mod play;
#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod render_tests_play;
#[cfg(test)]
mod render_tests_result;
#[cfg(test)]
mod render_tests_select;
#[cfg(test)]
mod render_tests_shell;
pub(crate) mod result;
pub(crate) mod select;
pub(crate) mod settings;
pub(crate) mod tables;

pub(crate) use canvas::Canvas;
#[cfg(test)]
pub(crate) use canvas::HeadlessCanvas;
pub(crate) use folders::FoldersState;
pub(crate) use keyconfig::KeyConfigState;
pub(crate) use loading::{KeysoundLoad, LoadingState};
pub(crate) use play::PlayState;
pub(crate) use result::ResultState;
pub(crate) use select::SelectState;
pub(crate) use settings::SettingsState;
pub(crate) use tables::TablesState;

/// Which screen the app is currently running, together with the state that screen owns.
///
/// `Play` and `Select` are boxed: the session with its decoded BGA images, and the browser with its
/// cached scene, would otherwise set the size of every other variant and make each stage change a
/// large memcpy.
pub(crate) enum Stage {
    Select(Box<SelectState>),
    Settings(SettingsState),
    KeyConfig(KeyConfigState),
    Tables(TablesState),
    Folders(FoldersState),
    Loading(LoadingState),
    Play(Box<PlayState>),
    Result(ResultState),
}

/// Which screen a stage is, without its state. Used where only the identity matters — the soak
/// log's stage column, the debug overlay, and the guards that ask "is a chart on screen right
/// now?" — so those stay comparable and constructible without building a whole screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StageId {
    Select,
    Settings,
    KeyConfig,
    Tables,
    Folders,
    Loading,
    Play,
    Result,
}

/// What a screen hands back to [`crate::App`] after an update, a key or a click.
///
/// The whole app changes screens through this one type, so the transition rule — run the leaving
/// screen's `on_exit`, swap, run the arriving screen's `on_enter` — lives in exactly one place.
pub(crate) enum Transition {
    /// Stay on this screen.
    Stay,
    /// Open a screen over this one; [`Transition::Back`] later resumes this one as it is now.
    Open(Stage),
    /// Replace this screen, leaving whatever it was opened over suspended underneath.
    To(Stage),
    /// Resume the screen this one was opened over, or the song browser when there is none.
    Back,
    /// Close the window.
    Quit,
}

/// Everything a screen is handed for one frame, one key or one click: the state that outlives a
/// stage change, plus this frame's clock.
pub(crate) struct FrameCtx<'a> {
    pub shared: &'a mut AppShared,
    pub now: Instant,
    pub dt: f32,
}

/// One keyboard event, already reduced to what the screens actually branch on. `pressed` is a
/// fresh press (auto-repeat excluded) and `released` is the key going back up, so a screen that
/// tracks held keys can tell a repeat from a release.
pub(crate) struct KeyInput<'a> {
    pub code: KeyCode,
    pub pressed: bool,
    pub released: bool,
    pub text: Option<&'a str>,
}

/// One screen of the player.
///
/// `update` runs once per frame before drawing, `draw` paints the screen and records its clickable
/// regions, and the two input methods translate a key or a left-click. `on_enter`/`on_exit` carry
/// the contracts a screen change has to honour — the select preview is torn down on the way out of
/// the browser, for instance, before anything else can touch the audio engine.
pub(crate) trait StageHandler {
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition;
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>);
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition;
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition {
        let _ = (ctx, at);
        Transition::Stay
    }
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        let _ = ctx;
    }
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        let _ = ctx;
    }
    /// The stage-specific lines of the debug overlay. Every screen but PLAY reports the browser.
    fn debug_lines(&self, ctx: &FrameCtx<'_>) -> Vec<String> {
        ctx.shared.browser_debug_lines()
    }

    /// Whether this screen is taking every key itself right now — a search box being typed into, a
    /// modal, a panel of its own. The app-wide overlays step aside while it is, so a shift key held
    /// to type a capital letter is not read as a request for the option panel.
    fn holds_keys(&self, ctx: &FrameCtx<'_>) -> bool {
        let _ = ctx;
        false
    }
}

impl StageId {
    /// Stage label used by the soak log and the debug overlay, kept stable so the CSV stays
    /// machine-readable.
    pub(crate) fn label(self) -> &'static str {
        match self {
            StageId::Select => "Select",
            StageId::Settings => "Settings",
            StageId::KeyConfig => "KeyConfig",
            StageId::Tables => "Tables",
            StageId::Folders => "Folders",
            StageId::Loading => "Loading",
            StageId::Play => "Play",
            StageId::Result => "Result",
        }
    }

    /// Every screen, for the tests that have to cover all of them.
    #[cfg(test)]
    pub(crate) const ALL: [StageId; 8] =
        [StageId::Select, StageId::Settings, StageId::KeyConfig, StageId::Tables, StageId::Folders, StageId::Loading, StageId::Play, StageId::Result];
}

impl Stage {
    pub(crate) fn id(&self) -> StageId {
        match self {
            Stage::Select(_) => StageId::Select,
            Stage::Settings(_) => StageId::Settings,
            Stage::KeyConfig(_) => StageId::KeyConfig,
            Stage::Tables(_) => StageId::Tables,
            Stage::Folders(_) => StageId::Folders,
            Stage::Loading(_) => StageId::Loading,
            Stage::Play(_) => StageId::Play,
            Stage::Result(_) => StageId::Result,
        }
    }

    /// The screen as a [`StageHandler`], so the dispatch below is written once per method.
    fn handler(&mut self) -> &mut dyn StageHandler {
        match self {
            Stage::Select(s) => s.as_mut(),
            Stage::Settings(s) => s,
            Stage::KeyConfig(s) => s,
            Stage::Tables(s) => s,
            Stage::Folders(s) => s,
            Stage::Loading(s) => s,
            Stage::Play(s) => s.as_mut(),
            Stage::Result(s) => s,
        }
    }

    fn view(&self) -> &dyn StageHandler {
        match self {
            Stage::Select(s) => s.as_ref(),
            Stage::Settings(s) => s,
            Stage::KeyConfig(s) => s,
            Stage::Tables(s) => s,
            Stage::Folders(s) => s,
            Stage::Loading(s) => s,
            Stage::Play(s) => s.as_ref(),
            Stage::Result(s) => s,
        }
    }

    pub(crate) fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.handler().update(ctx)
    }

    pub(crate) fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.handler().draw(ctx, canvas);
    }

    /// Route one key to the screen that is up, after the option overlay has had first refusal.
    ///
    /// The overlay is offered every key before the screen underneath sees it, because the whole
    /// point of it is that the list it is drawn over does not move while it is open. A key it does
    /// not take reaches the screen exactly as it would have.
    pub(crate) fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        let holds_keys = self.view().holds_keys(ctx);
        if crate::app_options::options_key(ctx, self.id(), holds_keys, &key) {
            return Transition::Stay;
        }
        self.handler().handle_key(ctx, key)
    }

    /// Route one click to the screen that is up, unless the option overlay is over it.
    ///
    /// An open panel takes the click for the same reason it takes the keys: the list underneath must
    /// not move while the panel is being read, and a click on a row would otherwise start a chart
    /// and leave the panel drawn over the run.
    pub(crate) fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition {
        if ctx.shared.options.is_open() {
            return Transition::Stay;
        }
        self.handler().handle_mouse(ctx, at)
    }

    pub(crate) fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        self.handler().on_enter(ctx);
    }

    pub(crate) fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.handler().on_exit(ctx);
    }

    pub(crate) fn debug_lines(&self, ctx: &FrameCtx<'_>) -> Vec<String> {
        self.view().debug_lines(ctx)
    }
}

impl std::fmt::Debug for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id().label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Upper bound on the enum, so a stage change stays a small move rather than a large memcpy.
    /// Growing a screen's state past this means boxing that variant, not raising the bound.
    const STAGE_SIZE_LIMIT: usize = 192;

    #[test]
    fn the_stage_enum_stays_small_enough_to_move_cheaply() {
        assert!(std::mem::size_of::<Stage>() <= STAGE_SIZE_LIMIT, "Stage is {} bytes, over the {STAGE_SIZE_LIMIT} byte budget", std::mem::size_of::<Stage>());
    }
}

//! Putting the DISPLAY tab's RESOLUTION and WINDOW MODE rows onto the real window: the attributes it
//! opens with, and the steps that bring a window already open to a changed pair of rows.

use std::sync::Arc;

use rbms_config::{DisplayOptions, WindowMode, WindowResolution};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::window::{Fullscreen, Window, WindowAttributes};

const WINDOW_TITLE: &str = "rbms";

/// What the two rows decide about the window.
type WindowRows = (WindowResolution, WindowMode);

/// One thing done to a window to bring it to the rows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WindowChange {
    EnterBorderless,
    LeaveBorderless,
    Resize((u32, u32)),
}

fn rows_of(display: &DisplayOptions) -> WindowRows {
    (display.window_resolution, display.window_mode)
}

fn borderless_or_none(mode: WindowMode) -> Option<Fullscreen> {
    (mode == WindowMode::Borderless).then_some(Fullscreen::Borderless(None))
}

/// The attributes the window is created with, so it opens already in the mode and at the size the
/// rows hold instead of flashing the default first.
pub(crate) fn window_attributes(display: &DisplayOptions) -> WindowAttributes {
    let (width, height) = display.window_resolution.size();
    Window::default_attributes()
        .with_title(WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(width, height))
        .with_fullscreen(borderless_or_none(display.window_mode))
}

/// The steps from a window that matches `applied` to one that matches `wanted`.
///
/// A borderless window takes its monitor's size, so a size chosen while it is up is not requested
/// then; it is requested on the way back to an ordinary window, which is also what puts the window
/// back to the chosen size after a visit to full screen.
fn changes(applied: WindowRows, wanted: WindowRows) -> Vec<WindowChange> {
    let (applied_resolution, applied_mode) = applied;
    let (wanted_resolution, wanted_mode) = wanted;
    match (applied_mode, wanted_mode) {
        (WindowMode::Windowed, WindowMode::Borderless) => vec![WindowChange::EnterBorderless],
        (WindowMode::Borderless, WindowMode::Windowed) => vec![WindowChange::LeaveBorderless, WindowChange::Resize(wanted_resolution.size())],
        (WindowMode::Windowed, WindowMode::Windowed) if applied_resolution != wanted_resolution => vec![WindowChange::Resize(wanted_resolution.size())],
        _ => Vec::new(),
    }
}

/// The open window together with the rows it was last brought to, so a changed row is applied once
/// and a window the user resized by hand is left alone until a row changes.
pub(crate) struct AppWindow {
    window: Arc<Window>,
    applied: WindowRows,
    /// Whether the window is told to hand text typed through an input method over as `Ime` events.
    ime_allowed: bool,
}

impl AppWindow {
    /// Wrap a window that was created from [`window_attributes`] of the same rows.
    pub(crate) fn new(window: Arc<Window>, display: &DisplayOptions) -> Self {
        AppWindow { window, applied: rows_of(display), ime_allowed: false }
    }

    /// Let the window take text through an input method while something is being typed into, and
    /// not otherwise: with it on, a Korean or Japanese input method composes in the window, and with
    /// it off every key reaches the game as a key, which is what a chart being played wants. Told
    /// to the window only when it changes.
    pub(crate) fn sync_ime(&mut self, wanted: bool) {
        if std::mem::replace(&mut self.ime_allowed, wanted) != wanted {
            self.window.set_ime_allowed(wanted);
        }
    }

    /// Bring the window to the rows if they changed since the last call. Returns the new size when
    /// the windowing system applied a resize on the spot, so the renderer can follow without waiting
    /// for the resize event.
    pub(crate) fn sync(&mut self, display: &DisplayOptions) -> Option<PhysicalSize<u32>> {
        let wanted = rows_of(display);
        let mut resized = None;
        for change in changes(self.applied, wanted) {
            match change {
                WindowChange::EnterBorderless => self.window.set_fullscreen(borderless_or_none(WindowMode::Borderless)),
                WindowChange::LeaveBorderless => self.window.set_fullscreen(None),
                WindowChange::Resize((width, height)) => resized = self.window.request_inner_size(LogicalSize::new(width, height)).or(resized),
            }
        }
        self.applied = wanted;
        resized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWED_HD: WindowRows = (WindowResolution::Hd720, WindowMode::Windowed);

    #[test]
    fn rows_that_did_not_change_ask_for_nothing() {
        for rows in [WINDOWED_HD, (WindowResolution::FullHd1080, WindowMode::Borderless)] {
            assert!(changes(rows, rows).is_empty());
        }
    }

    #[test]
    fn a_new_size_resizes_an_ordinary_window() {
        assert_eq!(changes(WINDOWED_HD, (WindowResolution::FullHd1080, WindowMode::Windowed)), vec![WindowChange::Resize((1920, 1080))]);
    }

    #[test]
    fn going_borderless_changes_the_mode_and_leaves_the_size_alone() {
        assert_eq!(changes(WINDOWED_HD, (WindowResolution::QuadHd1440, WindowMode::Borderless)), vec![WindowChange::EnterBorderless]);
    }

    #[test]
    fn a_size_picked_while_borderless_waits_for_the_way_back() {
        let borderless = (WindowResolution::Hd720, WindowMode::Borderless);
        assert!(changes(borderless, (WindowResolution::Hd900, WindowMode::Borderless)).is_empty());
        assert_eq!(
            changes((WindowResolution::Hd900, WindowMode::Borderless), (WindowResolution::Hd900, WindowMode::Windowed)),
            vec![WindowChange::LeaveBorderless, WindowChange::Resize((1600, 900))],
            "leaving full screen restores the chosen size even though the size row did not move"
        );
    }

    #[test]
    fn the_opening_attributes_follow_the_rows() {
        let mut display = DisplayOptions::default();
        assert!(window_attributes(&display).fullscreen.is_none());
        display.window_mode = WindowMode::Borderless;
        assert!(matches!(window_attributes(&display).fullscreen, Some(Fullscreen::Borderless(None))));
    }
}

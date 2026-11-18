//! Putting the DISPLAY tab's RESOLUTION and WINDOW MODE rows onto the real window: the attributes it
//! opens with, and the steps that bring a window already open to a changed pair of rows.
//!
//! FULLSCREEN is exclusive full screen: the monitor the window is on is switched to the video mode
//! nearest the RESOLUTION row. Which monitor that is is known only once the window is open, so a
//! window of that row opens as an ordinary one and is put into its video mode by the first
//! [`AppWindow::sync`]; where no video mode can be taken (Wayland, or a monitor that lists none) it
//! covers the monitor borderless instead and the player is told.
//!
//! Exclusive full screen is always entered from an ordinary window. On macOS a borderless window
//! is the system's full screen, and switching the video mode under it leaves the window at the
//! frame of the old mode, partly off the screen; leaving it first lets the video mode be set before
//! the window goes full screen again, the way an ordinary window enters it.

use std::cmp::Reverse;
use std::sync::Arc;

use rbms_config::{DisplayOptions, WindowMode, WindowResolution};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::monitor::VideoModeHandle;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Fullscreen, Icon, Window, WindowAttributes};

use crate::notify::{Level, notify_text};
use crate::text::notices::NoticeText;

const WINDOW_TITLE: &str = "rbms";

/// The app icon the window shows in its title bar and on the taskbar (Windows, Linux; macOS takes
/// the bundle's `R-BMS.icns` and leaves a window's icon unused). Generated with the rest of the
/// icon files by `assets/icon/tools/generate-icon.py`.
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../../../assets/icon/png/icon-256.png");

/// The app icon decoded for the window, or none if the embedded picture cannot be read, which
/// leaves the platform's default icon rather than keeping the window from opening.
fn window_icon() -> Option<Icon> {
    let rgba = image::load_from_memory(WINDOW_ICON_PNG).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

/// What the two rows decide about the window.
type WindowRows = (WindowResolution, WindowMode);

/// What picking a video mode looks at: width and height in physical pixels, refresh rate in
/// millihertz and bit depth, as [`VideoModeHandle`] reports them.
type VideoModeShape = (u32, u32, u32, u16);

/// One thing done to a window to bring it to the rows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WindowChange {
    EnterBorderless,
    /// Take the monitor in the video mode nearest this size.
    EnterExclusive((u32, u32)),
    /// Back to an ordinary window from either kind of full screen.
    LeaveFullScreen,
    Resize((u32, u32)),
}

fn rows_of(display: &DisplayOptions) -> WindowRows {
    (display.window_resolution, display.window_mode)
}

/// The rows a window created from [`window_attributes`] is in when it opens: FULLSCREEN opens as an
/// ordinary window, since its video mode is picked from the monitor the window turns out to be on.
fn opening_rows(display: &DisplayOptions) -> WindowRows {
    let (resolution, mode) = rows_of(display);
    (resolution, if mode == WindowMode::Fullscreen { WindowMode::Windowed } else { mode })
}

fn borderless_or_none(mode: WindowMode) -> Option<Fullscreen> {
    mode.covers_the_monitor().then_some(Fullscreen::Borderless(None))
}

/// Which of `modes` exclusive full screen takes for a RESOLUTION of `wanted`: one of that size if
/// the monitor has it and the nearest one otherwise, the larger of two equally near, and of one
/// size the highest refresh rate, then the deepest colour. None when there is no mode to take.
fn nearest_video_mode(modes: &[VideoModeShape], wanted: (u32, u32)) -> Option<usize> {
    let (wanted_width, wanted_height) = wanted;
    let preference = |&(width, height, refresh_millihertz, bit_depth): &VideoModeShape| {
        let distance = u64::from(width.abs_diff(wanted_width)) + u64::from(height.abs_diff(wanted_height));
        (distance, Reverse(u64::from(width) * u64::from(height)), Reverse(refresh_millihertz), Reverse(bit_depth))
    };
    (0..modes.len()).min_by_key(|&at| preference(&modes[at]))
}

fn shape_of(mode: &VideoModeHandle) -> VideoModeShape {
    let size = mode.size();
    (size.width, size.height, mode.refresh_rate_millihertz(), mode.bit_depth())
}

/// Whether the window is a Wayland one, where winit does nothing with a video mode request.
fn runs_on_wayland(window: &Window) -> bool {
    window.window_handle().is_ok_and(|handle| matches!(handle.as_raw(), RawWindowHandle::Wayland(_)))
}

/// The video mode of the monitor the window is on that exclusive full screen at `size` takes, or
/// why there is none.
fn exclusive_video_mode(window: &Window, size: (u32, u32)) -> Result<VideoModeHandle, &'static str> {
    if runs_on_wayland(window) {
        return Err("Wayland lets no program change the video mode");
    }
    let monitor = window.current_monitor().or_else(|| window.primary_monitor()).ok_or("the window is on no monitor")?;
    let mut modes: Vec<VideoModeHandle> = monitor.video_modes().collect();
    let shapes: Vec<VideoModeShape> = modes.iter().map(shape_of).collect();
    let at = nearest_video_mode(&shapes, size).ok_or("the monitor lists no video modes")?;
    Ok(modes.swap_remove(at))
}

/// The attributes the window is created with, so it opens already in the mode and at the size the
/// rows hold instead of flashing the default first.
pub(crate) fn window_attributes(display: &DisplayOptions) -> WindowAttributes {
    let (width, height) = display.window_resolution.size();
    Window::default_attributes()
        .with_title(WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(width, height))
        .with_fullscreen(borderless_or_none(opening_rows(display).1))
        .with_window_icon(window_icon())
}

/// The steps from a window that matches `applied` to one that matches `wanted`.
///
/// A borderless window takes its monitor's size, so a size chosen while it is up is not requested
/// then; it is requested on the way back to an ordinary window, which is also what puts the window
/// back to the chosen size after a visit to full screen. Exclusive full screen takes the video mode
/// of the chosen size, so a size chosen while it is up switches the video mode at once; a borderless
/// window is made an ordinary one before it takes the video mode (see the module's notes).
fn changes(applied: WindowRows, wanted: WindowRows) -> Vec<WindowChange> {
    let (applied_resolution, applied_mode) = applied;
    let (wanted_resolution, wanted_mode) = wanted;
    let size = wanted_resolution.size();
    match (applied_mode, wanted_mode) {
        (WindowMode::Windowed, WindowMode::Windowed) if applied_resolution != wanted_resolution => vec![WindowChange::Resize(size)],
        (WindowMode::Borderless | WindowMode::Fullscreen, WindowMode::Windowed) => vec![WindowChange::LeaveFullScreen, WindowChange::Resize(size)],
        (WindowMode::Windowed | WindowMode::Fullscreen, WindowMode::Borderless) => vec![WindowChange::EnterBorderless],
        (WindowMode::Windowed, WindowMode::Fullscreen) => vec![WindowChange::EnterExclusive(size)],
        (WindowMode::Borderless, WindowMode::Fullscreen) => vec![WindowChange::LeaveFullScreen, WindowChange::EnterExclusive(size)],
        (WindowMode::Fullscreen, WindowMode::Fullscreen) if applied_resolution != wanted_resolution => vec![WindowChange::EnterExclusive(size)],
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
    /// Wrap a window that was created from [`window_attributes`] of the same rows. A FULLSCREEN
    /// row is not met yet then, so the first [`AppWindow::sync`] takes the video mode.
    pub(crate) fn new(window: Arc<Window>, display: &DisplayOptions) -> Self {
        AppWindow { window, applied: opening_rows(display), ime_allowed: false }
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
                WindowChange::EnterExclusive(size) => self.enter_exclusive(size),
                WindowChange::LeaveFullScreen => self.window.set_fullscreen(None),
                WindowChange::Resize((width, height)) => resized = self.window.request_inner_size(LogicalSize::new(width, height)).or(resized),
            }
        }
        self.applied = wanted;
        resized
    }

    /// Take the monitor in the video mode nearest `size`, or cover it borderless and say so when no
    /// video mode can be taken. The row stays FULLSCREEN either way, so the attempt is made again
    /// only when a row changes.
    fn enter_exclusive(&self, size: (u32, u32)) {
        match exclusive_video_mode(&self.window, size) {
            Ok(mode) => self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode))),
            Err(reason) => {
                println!("window: exclusive full screen not taken: {reason}");
                notify_text(Level::Warn, NoticeText::FullscreenFellBackToBorderless, &[]);
                self.window.set_fullscreen(borderless_or_none(WindowMode::Borderless));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWED_HD: WindowRows = (WindowResolution::Hd720, WindowMode::Windowed);

    /// The side of the picture the window icon is made from (`assets/icon/png/icon-256.png`).
    const WINDOW_ICON_SIDE: u32 = 256;

    /// The embedded app icon decodes into a window icon, so a window opens with it and not with the
    /// platform's default.
    #[test]
    fn the_embedded_app_icon_becomes_a_window_icon() {
        assert!(window_icon().is_some(), "the embedded icon is not a picture winit takes");
        let picture = image::load_from_memory(WINDOW_ICON_PNG).expect("the embedded icon decodes");
        assert_eq!((picture.width(), picture.height()), (WINDOW_ICON_SIDE, WINDOW_ICON_SIDE));
    }

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
            vec![WindowChange::LeaveFullScreen, WindowChange::Resize((1600, 900))],
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

    /// Every pair of modes, at an unchanged and at a changed size, asks for the steps the table
    /// below names and no others.
    #[test]
    fn every_change_between_the_three_modes_asks_for_its_own_steps() {
        use WindowChange::{EnterBorderless, EnterExclusive, LeaveFullScreen, Resize};
        use WindowMode::{Borderless, Fullscreen, Windowed};
        let (from, to) = (WindowResolution::Hd720, WindowResolution::FullHd1080);
        let size = to.size();
        let table: [(WindowMode, WindowMode, Vec<WindowChange>, Vec<WindowChange>); 9] = [
            (Windowed, Windowed, vec![], vec![Resize(size)]),
            (Windowed, Borderless, vec![EnterBorderless], vec![EnterBorderless]),
            (Windowed, Fullscreen, vec![EnterExclusive(from.size())], vec![EnterExclusive(size)]),
            (Borderless, Windowed, vec![LeaveFullScreen, Resize(from.size())], vec![LeaveFullScreen, Resize(size)]),
            (Borderless, Borderless, vec![], vec![]),
            (Borderless, Fullscreen, vec![LeaveFullScreen, EnterExclusive(from.size())], vec![LeaveFullScreen, EnterExclusive(size)]),
            (Fullscreen, Windowed, vec![LeaveFullScreen, Resize(from.size())], vec![LeaveFullScreen, Resize(size)]),
            (Fullscreen, Borderless, vec![EnterBorderless], vec![EnterBorderless]),
            (Fullscreen, Fullscreen, vec![], vec![EnterExclusive(size)]),
        ];
        assert_eq!(table.len(), WindowMode::ALL.len() * WindowMode::ALL.len(), "a pair of modes has no row");
        for (applied, wanted, same_size, new_size) in table {
            assert_eq!(changes((from, applied), (from, wanted)), same_size, "{applied:?} to {wanted:?}");
            assert_eq!(changes((from, applied), (to, wanted)), new_size, "{applied:?} to {wanted:?} at a new size");
        }
    }

    /// A FULLSCREEN window opens as an ordinary one, since the monitor it is on is not known before
    /// it opens, and the first sync takes the video mode of the chosen size.
    #[test]
    fn exclusive_full_screen_opens_as_a_window_and_takes_its_video_mode_on_the_first_sync() {
        let display = DisplayOptions { window_mode: WindowMode::Fullscreen, window_resolution: WindowResolution::QuadHd1440, ..DisplayOptions::default() };
        assert!(window_attributes(&display).fullscreen.is_none());
        let opened = opening_rows(&display);
        assert_eq!(opened, (WindowResolution::QuadHd1440, WindowMode::Windowed));
        assert_eq!(changes(opened, rows_of(&display)), vec![WindowChange::EnterExclusive((2560, 1440))]);
        for mode in [WindowMode::Windowed, WindowMode::Borderless] {
            let display = DisplayOptions { window_mode: mode, ..display.clone() };
            assert_eq!(opening_rows(&display), rows_of(&display), "{mode:?} opens as it is");
        }
    }

    /// Refresh rates and colour depths of the made-up monitors below, the rates in millihertz.
    const HZ_60: u32 = 60_000;
    const HZ_120: u32 = 120_000;
    const HZ_144: u32 = 144_000;
    const DEPTH_24: u16 = 24;
    const DEPTH_32: u16 = 32;

    #[test]
    fn the_video_mode_of_the_chosen_size_is_taken_at_its_highest_refresh_rate() {
        let modes = [
            (2560, 1440, HZ_144, DEPTH_32),
            (1920, 1080, HZ_60, DEPTH_32),
            (1920, 1080, HZ_144, DEPTH_24),
            (1920, 1080, HZ_144, DEPTH_32),
            (1920, 1080, HZ_120, DEPTH_32),
            (1280, 720, HZ_144, DEPTH_32),
        ];
        assert_eq!(nearest_video_mode(&modes, (1920, 1080)), Some(3), "the highest refresh rate, then the deepest colour");
        assert_eq!(nearest_video_mode(&modes, (1280, 720)), Some(5));
    }

    #[test]
    fn a_size_the_monitor_lacks_takes_the_nearest_one() {
        let laptop = [(2560, 1600, HZ_60, DEPTH_32), (1920, 1200, HZ_120, DEPTH_32), (1440, 900, HZ_120, DEPTH_32), (1280, 800, HZ_60, DEPTH_32)];
        assert_eq!(nearest_video_mode(&laptop, (1920, 1080)), Some(1));
        assert_eq!(nearest_video_mode(&laptop, (3840, 2160)), Some(0), "nothing larger than the largest");
        assert_eq!(nearest_video_mode(&laptop, (1280, 720)), Some(3));
        let equally_near = [(1600, 900, HZ_60, DEPTH_32), (1600, 1100, HZ_60, DEPTH_32)];
        assert_eq!(nearest_video_mode(&equally_near, (1600, 1000)), Some(1), "the larger of two equally near");
    }

    #[test]
    fn a_monitor_that_lists_no_video_modes_has_none_to_take() {
        assert_eq!(nearest_video_mode(&[], (1920, 1080)), None);
    }
}

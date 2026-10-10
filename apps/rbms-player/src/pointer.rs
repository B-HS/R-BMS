//! The mouse as the screens see it: which buttons are down, what a turn of the wheel is worth, and
//! how one event reaches the screen that is up.
//!
//! The window delivers a cursor move, a button edge and a wheel turn as three unrelated events.
//! A screen is handed them as [`PointerInput`] instead: a button edge, a drag (the cursor moving
//! while a button is held) or a scroll, always at the position the cursor was last seen. A screen
//! that has no use for one of them keeps the no-op default of [`StageHandler`].

use winit::event::{MouseButton, MouseScrollDelta};

use crate::stage::{FrameCtx, StageHandler, Transition};

/// How many pixels of touchpad travel count as one line of wheel travel. A touchpad reports its
/// scroll in pixels where a wheel notch reports lines; this is the exchange rate between them.
pub(crate) const PIXELS_PER_SCROLL_LINE: f32 = 40.0;

/// One mouse event, reduced to what the screens branch on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PointerInput {
    /// A button went down (`pressed`) or came back up.
    Button { button: MouseButton, pressed: bool },
    /// The cursor moved while a button was held.
    Drag,
    /// The wheel turned. `lines` is positive when the wheel is rolled towards the player, which is
    /// the direction a list moves on to its next entry (the reference's `amountY`), and negative
    /// when it is rolled away.
    Scroll { lines: f32 },
}

impl PointerInput {
    /// The scroll a window wheel event stands for. The window reports the direction the content
    /// should move, which is the opposite of the direction the wheel was rolled.
    pub(crate) fn from_wheel(delta: MouseScrollDelta) -> PointerInput {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => -y,
            MouseScrollDelta::PixelDelta(travel) => -(travel.y as f32) / PIXELS_PER_SCROLL_LINE,
        };
        PointerInput::Scroll { lines }
    }
}

/// Whether the screens are told about a button. The extra buttons of a mouse (back, forward and
/// anything beyond them) have no meaning to a screen.
pub(crate) const fn is_tracked(button: MouseButton) -> bool {
    matches!(button, MouseButton::Left | MouseButton::Right | MouseButton::Middle)
}

/// The buttons that are down right now, which is what turns a cursor move into a drag.
#[derive(Debug, Default)]
pub(crate) struct HeldButtons(Vec<MouseButton>);

impl HeldButtons {
    /// Record a button edge and report it as the input a screen is handed.
    pub(crate) fn on_button(&mut self, button: MouseButton, pressed: bool) -> PointerInput {
        self.0.retain(|held| *held != button);
        if pressed {
            self.0.push(button);
        }
        PointerInput::Button { button, pressed }
    }

    /// What a cursor move means: a drag while any button is down, nothing otherwise.
    pub(crate) fn on_move(&self) -> Option<PointerInput> {
        (!self.0.is_empty()).then_some(PointerInput::Drag)
    }

    /// Forget every held button, for a window that lost focus and so will never see them released.
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}

/// Hand one event to a screen, unless the option panel is over it or an object of the skin the
/// screen is drawn with took it ([`crate::AppShared::skin_pointer`]).
///
/// An open panel takes the mouse for the same reason it takes the keys: the list underneath must
/// not move while the panel is being read, and a click on a row would otherwise start a chart and
/// leave the panel drawn over the run.
pub(crate) fn route_pointer(handler: &mut dyn StageHandler, ctx: &mut FrameCtx<'_>, at: (f32, f32), input: PointerInput) -> Transition {
    if ctx.shared.options.is_open() || ctx.shared.skin_pointer(at, input) {
        return Transition::Stay;
    }
    match input {
        PointerInput::Button { button, pressed } => handler.handle_mouse(ctx, at, button, pressed),
        PointerInput::Drag => handler.handle_mouse_drag(ctx, at),
        PointerInput::Scroll { lines } => handler.handle_scroll(ctx, lines),
    }
}

#[cfg(test)]
mod tests;

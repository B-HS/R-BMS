//! Pointer input against a drawn skin: which object a press lands on, and which slider a drag moves.
//!
//! A skin takes the pointer the way it draws, by walking its objects, only from the top down
//! (`Skin.mousePressed` and `Skin.mouseDragged`): a press goes to the topmost drawn object that has
//! something to do with it, and a drag goes to sliders alone. Hovering is not an event at all -- a
//! destination's `mouseRect` reads the pointer the frame carries while it is prepared.
//!
//! What an object does with a press is told to the frame's host, through the same calls a script
//! makes ([`rbms_skin::property::SkinHost::exec_event`] and its writers), so input and drawing read
//! and change one state.

use super::{SkinFrame, SkinScreen};

/// Which button a press was made with, numbered as the reference's click table is indexed
/// (`SkinObject.mousePressed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinPointerButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// One thing the pointer did over a skin.
///
/// The position travels with the frame ([`SkinFrame::mouse`]), in document coordinates, because it
/// is the same one the frame's hover gates were prepared against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkinPointer {
    /// A button went down.
    Press(SkinPointerButton),
    /// The pointer moved with a button held.
    Drag,
    /// The wheel turned by this many lines, positive towards the player.
    Scroll(f32),
}

impl SkinScreen {
    /// Offers one pointer event to this screen's objects, as they stood on `frame`, and answers
    /// whether one of them took it.
    ///
    /// No object takes input yet, so this answers false for every event.
    pub fn pointer(&self, _frame: &SkinFrame<'_>, _event: SkinPointer) -> bool {
        false
    }
}

//! An editable `text` object: a run of text the player can type into.
//!
//! The reference keeps one class for both (`SkinText`, whose `editable` flag is what makes
//! `Skin.mousePressed` open an input over it). It is an object of its own here because it is the
//! only text that takes input. For now it draws exactly as the text object it was declared as; what
//! the reference does differently for an editable text -- it stays prepared while it is empty, and
//! it takes the click and the keys that follow -- is this module's to add.

use rbms_skin::dst::SkinRect;
use rbms_skin::model::TextDef;

use super::SkinFrame;
use super::draw::Placement;
use super::text::{TextBody, draw_text, text_body};
use crate::Renderer;
use crate::ctx::RenderCtx;

/// A text object the document marked `editable`.
#[derive(Debug)]
pub(crate) struct TextInputBody {
    /// What is shown while nothing is being typed, which is the text object the document declared.
    text: TextBody,
}

impl TextInputBody {
    /// The text object this was declared as, which is what the prepare stage reads the value of.
    pub(crate) fn shown(&self) -> &TextBody {
        &self.text
    }
}

/// The editable text a text record declares.
pub(crate) fn text_input_body(def: &TextDef, families: &[(String, String)]) -> TextInputBody {
    TextInputBody { text: text_body(def, families) }
}

/// Draws an editable text, which while nothing is being typed is the text object it was declared
/// as.
pub(crate) fn draw_text_input<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TextInputBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    draw_text(ctx, r, place, &body.text, rect, frame)
}

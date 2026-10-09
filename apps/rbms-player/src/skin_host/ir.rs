//! Property cluster H: the internet ranking and rivals.
//!
//! Where the values come from: what the ranking service answered: connection state, this player's
//! rank, the ranking's clear and score spread, the top ten, and the rival's score.
//!
//! What this cluster answers: numbers 179-182, 200-242, 271-289 and 380-399; floats 203-229 and
//! 285-289; options 50-51 and 601-608; strings 1, 120-129 and 1020-1021.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct IrState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for IrState<'_> {}

//! Property cluster E: the player's settings.
//!
//! Where the values come from: what the player chose: gauge, random and double options, hi-speed
//! and its fix, BGA, assists, long note mode, auto-save slots and the other switches a skin shows
//! as a picked image.
//!
//! What this cluster answers: image indices 10-12, 40-43, 54-55, 61-63, 72, 75, 78, 89-90, 301-308,
//! 321-324, 330-332, 340-343, 350-353, 360-361 and 400; numbers 10, 12, 57-59 and 310-313; float
//! 310; rates 17-19; options 60-62.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct OptionsState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for OptionsState<'_> {}

//! Property cluster J: the skin settings screen.
//!
//! Where the values come from: the skin being configured: its name and author, the customise rows
//! on show, which skin type is picked and how far the list is scrolled.
//!
//! What this cluster answers: strings 50-51 and 100-119; rate 7; image indices 170-185 and 386-388.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct SkinConfigState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for SkinConfigState<'_> {}

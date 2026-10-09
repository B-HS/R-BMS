//! Property cluster G: the result of a finished run.
//!
//! Where the values come from: the score a run ended on beside the one it replaced: clear or
//! failed, what was updated, who won, the miss count and the timing average.
//!
//! What this cluster answers: numbers 76, 170-178 and 370-377; options 90-91, 330-336, 352-354 and
//! 1330-1336; image indices 370-371.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct ResultState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for ResultState<'_> {}

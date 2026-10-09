//! Property clusters C and D: the judgement on show and the state of the lanes.
//!
//! Where the values come from: the last judgement of each side and how early or late it was, the
//! per-key judgement images, the lane cover, lift and hidden values and the offsets they move
//! objects by.
//!
//! What these clusters answer: options 241, 261, 361, 1242-1243, 1262-1263, 1362-1363 and 270-273;
//! numbers 14, 314-316, 525-527 and 1312-1327; rates 4 and 5; image indices 500-519 and 1510-1699;
//! offsets 1-5.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What these clusters read from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlayState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for PlayState<'_> {}

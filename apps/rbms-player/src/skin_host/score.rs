//! Property cluster B: the score of the run in progress.
//!
//! Where the values come from: the live score and the scores it is compared with: EX score and
//! rates, judgement counts, the best and the target, and the gauge `main_state` reports.
//!
//! What this cluster answers: numbers 71-72, 74-75, 100-108, 115-116, 121-123, 128, 135-136,
//! 150-158, 170-178, 183-184 and 410-427; floats 85-89, 155, 157, 183, 1102 and 1115; rates
//! 110-115; options 200-207, 220-227, 230-240, 300-307, 320-327, 340-347 and 2241-2246; and what
//! `main_state` asks about the run itself: the gauge, its type, the judgement counts and the three
//! scores.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScoreState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for ScoreState<'_> {}

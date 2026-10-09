//! Property cluster F: the song browser.
//!
//! Where the values come from: the bar under the cursor and the panels over the browser: what kind
//! of bar it is, its lamp, its play counts, its folder's lamp totals, its replays and where the
//! cursor stands in the list.
//!
//! What this cluster answers: options 1-5, 21-23, 100-105, 603-608, 624-625, 1002-1017, 1030-1031,
//! 1100-1104 and 1205-1208; numbers 77-79, 243-249, 300 and 320-330; rates 1 and 8.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct SelectState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for SelectState<'_> {}

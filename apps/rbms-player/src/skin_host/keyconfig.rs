//! Property cluster K: the key configuration screen.
//!
//! Where the values come from: what each key is bound to, for the labels a key configuration skin
//! prints.
//!
//! What this cluster answers: strings 40-49 and 240-283.
//!
//! Nothing is answered yet. Every read falls through to the trait's "not known", so a skin asking
//! for one of these ids reads the absent value of its kind.

use std::marker::PhantomData;

use super::ClusterState;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyConfigState<'a> {
    scene: PhantomData<&'a ()>,
}

impl ClusterState for KeyConfigState<'_> {}

//! Long-note pairing of the BMS decoder, after the reference implementation's `Section`.
//!
//! A long note is a head and a tail in one lane with nothing else between them. A chart does not
//! promise that: a normal note can sit inside a long note's body, a long-note object can be left
//! without its partner, two long notes can touch. The reference implementation settles every such
//! case while it decodes, so a model it hands out has no overlapping note, no head without a tail
//! and no tail without a head, and a note count taken from it means the same to the judge engine
//! and to the song database. This module does the same settling for the model `to_model` builds.
//!
//! `LN` objects of one lane pair up in time order, first as a head, then as its tail
//! (`Section.lambda$makeTimeLines$9`). Around that:
//!
//! - A normal note under the head's tick, or inside the body, leaves the lane and plays as a
//!   background sound; a mine there is dropped.
//! - An object that sits on the tick the lane's previous long note ended on is swallowed together
//!   with its partner, so the earlier long note is the one that stays.
//! - A head the chart never closes is removed when decoding ends.
//!
//! Differences from the reference implementation, none of which a chart that plays well there
//! reaches: it sweeps a body when the tail is read, so a normal note listed after the tail in a later
//! channel line of the same measure survives inside the body there, while here every note before the
//! tail is swept; and a normal note or mine written over a head or tail in the same slot ends the
//! pair here by removing the other half, where the reference keeps the pair link alive.

use rbms_model::{NoteKind, TimeLine};

/// Where a lane stands while its long-note objects are paired in time order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LnPhase {
    /// The next object starts a long note.
    #[default]
    Closed,
    /// A head sits on the timeline at this index and waits for its tail.
    Open(usize),
    /// The head was swallowed, so the next object is its tail and is swallowed too.
    Swallowed,
}

/// The pairing state of one lane.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LnLane {
    pub phase: LnPhase,
    /// The timeline the lane's last long note ended on, by either way of writing one.
    pub last_tail: Option<usize>,
}

/// Move a normal note out of `lane` of `tl` into the background sounds. Anything else in the slot is
/// dropped: a mine inside a long note is not played, and a long-note half is not a sound.
pub(crate) fn move_to_background(tl: &mut TimeLine, lane: usize) {
    if let Some(note) = tl.notes[lane].take()
        && note.kind == NoteKind::Normal
    {
        tl.bgnotes.push(note);
    }
}

/// Make room for a head with sample `head_wav` on the tick `tl`. A normal note already there
/// becomes a background sound unless it is the very sample the head plays, which would sound twice.
pub(crate) fn displace_normal_note(tl: &mut TimeLine, lane: usize, head_wav: i32) {
    match tl.notes[lane].take() {
        Some(note) if note.kind == NoteKind::Normal && note.wav != head_wav => tl.bgnotes.push(note),
        _ => {}
    }
}

/// Clear what lies strictly between the head at `head_idx` and the tail at `tail_idx` in `lane`:
/// normal notes become background sounds, everything else is dropped.
pub(crate) fn clear_body(timelines: &mut [TimeLine], head_idx: usize, tail_idx: usize, lane: usize) {
    for tl in &mut timelines[head_idx + 1..tail_idx] {
        move_to_background(tl, lane);
    }
}

enum Slot {
    Head,
    Tail,
    Plain,
}

/// Enforce the shape of a long note on every lane once all objects are placed: heads and tails
/// alternate, nothing but them is in a lane between a head and its tail, and the head of a long note
/// that was never closed is gone. What the event pass already settled passes through untouched; this
/// catches the rest, such as a tail written over by another object in the same slot.
pub(crate) fn repair(timelines: &mut [TimeLine], lanes: usize) {
    let mut interior = Vec::new();
    for lane in 0..lanes {
        let mut head: Option<usize> = None;
        interior.clear();
        for idx in 0..timelines.len() {
            let slot = match timelines[idx].notes[lane].as_ref().map(|note| &note.kind) {
                None => continue,
                Some(NoteKind::LongStart { .. }) => Slot::Head,
                Some(NoteKind::LongEnd { .. }) => Slot::Tail,
                Some(NoteKind::Normal | NoteKind::Mine { .. }) => Slot::Plain,
            };
            match slot {
                Slot::Head => {
                    if let Some(unclosed) = head.replace(idx) {
                        timelines[unclosed].notes[lane] = None;
                    }
                    interior.clear();
                }
                Slot::Tail if head.take().is_some() => {
                    for inside in interior.drain(..) {
                        move_to_background(&mut timelines[inside], lane);
                    }
                }
                Slot::Tail => timelines[idx].notes[lane] = None,
                Slot::Plain if head.is_some() => interior.push(idx),
                Slot::Plain => {}
            }
        }
        if let Some(unclosed) = head {
            timelines[unclosed].notes[lane] = None;
        }
    }
}

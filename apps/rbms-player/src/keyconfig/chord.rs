//! Two keys held together as one binding.
//!
//! A control of a run or a shortcut may be bound to a single key or to two keys held together, on
//! the keyboard as on a controller. Lanes and the reverse spin of a scratch lane are played, not
//! fired, and stay on a single key.
//!
//! Two keys make the binding the moment both are down, in either order. That press does not also
//! run what its key alone is bound to; a lane on that key is still played, since a note is never
//! eaten by a binding. Read as held -- START, SELECT, the keys that repeat -- the binding is down
//! while both keys are, and a key that is one of two keys held together does not count as down for
//! what it alone is bound to, so a binding the player made wins over START or SELECT held with a
//! key of the key table.
//!
//! Everything here is a function of the keys that are down, so it is tested with no window.

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use winit::keyboard::KeyCode;

use super::{Bindable, HeldKeys, UNNAMED_KEY, key_from_name, key_name};

/// What joins the two keys of a chord in the key config file.
pub const CHORD_JOIN: char = '+';

/// What joins the two keys of a chord on screen.
pub const CHORD_LABEL_JOIN: &str = " + ";

/// What a control or a shortcut is bound to on the keyboard: one key, or two keys held together, in
/// the order they went down when the binding was made. The order is only what is shown; either
/// order makes the chord.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum KeyBind {
    Single(KeyCode),
    Chord(KeyCode, KeyCode),
}

impl KeyBind {
    /// Two different keys held together, or `None` for one key named twice.
    pub fn chord(first: KeyCode, second: KeyCode) -> Option<KeyBind> {
        (first != second).then_some(KeyBind::Chord(first, second))
    }

    /// Read a word of the key config file: a key token, or two joined by [`CHORD_JOIN`]. A word that
    /// names no key, or a chord of one key twice, is `None`.
    pub fn parse(token: &str) -> Option<KeyBind> {
        match token.split_once(CHORD_JOIN) {
            None => key_from_name(token.trim()).map(KeyBind::Single),
            Some((first, second)) => KeyBind::chord(key_from_name(first.trim())?, key_from_name(second.trim())?),
        }
    }

    /// The word the key config file holds for it, which [`KeyBind::parse`] reads back.
    pub fn token(self) -> String {
        match self {
            KeyBind::Single(code) => key_name(code).to_string(),
            KeyBind::Chord(first, second) => format!("{}{CHORD_JOIN}{}", key_name(first), key_name(second)),
        }
    }

    /// What it is called on screen, each key named by `name`.
    pub fn label_with(self, name: impl Fn(KeyCode) -> &'static str) -> String {
        match self {
            KeyBind::Single(code) => name(code).to_string(),
            KeyBind::Chord(first, second) => format!("{}{CHORD_LABEL_JOIN}{}", name(first), name(second)),
        }
    }

    /// What it is called on screen, each key by its word in the key config file.
    pub fn label(self) -> String {
        self.label_with(key_name)
    }

    /// The key, when it is one key.
    pub fn single(self) -> Option<KeyCode> {
        match self {
            KeyBind::Single(code) => Some(code),
            KeyBind::Chord(..) => None,
        }
    }

    /// Its keys, the one of a single key once.
    pub fn keys(self) -> Vec<KeyCode> {
        match self {
            KeyBind::Single(code) => vec![code],
            KeyBind::Chord(first, second) => vec![first, second],
        }
    }

    pub fn contains(self, code: KeyCode) -> bool {
        self.keys().contains(&code)
    }

    /// Whether the two are made by the same keys, a chord's in either order.
    pub fn same_keys(self, other: KeyBind) -> bool {
        match (self, other) {
            (KeyBind::Single(one), KeyBind::Single(two)) => one == two,
            (KeyBind::Chord(a, b), KeyBind::Chord(c, d)) => (a, b) == (c, d) || (a, b) == (d, c),
            _ => false,
        }
    }

    /// Whether the key config file has a word for every key of it.
    pub fn is_named(self) -> bool {
        self.keys().into_iter().all(|code| key_name(code) != UNNAMED_KEY)
    }

    /// Whether every key of it is down.
    pub fn is_held(self, held: &HeldKeys) -> bool {
        self.keys().into_iter().all(|code| held.is_down(code))
    }
}

/// What one key going down or up did to the chords bound on the keyboard: the ones its press made,
/// or the ones its release broke. It lasts for the handling of that one key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordEvent {
    pub code: KeyCode,
    pub pressed: bool,
    pub chords: Vec<Bindable>,
    /// Whether a lane on the key that made the chords is still played. Only a run plays lanes; on
    /// any other screen the key is the chord's alone.
    pub keeps_lanes: bool,
}

impl ChordEvent {
    /// Whether this is the press of `code` that made a chord.
    pub fn made_by(&self, code: KeyCode) -> bool {
        self.pressed && self.code == code
    }
}

/// The chords among `bound` that `code` is a key of and that are held: with `held` counting `code` as
/// down just after its press, the chords the press made, and just before its release, the ones the
/// release breaks.
pub fn held_chords_with(bound: &[(Bindable, KeyBind)], held: &HeldKeys, code: KeyCode) -> Vec<Bindable> {
    bound.iter().filter(|(_, bind)| matches!(bind, KeyBind::Chord(..)) && bind.contains(code) && bind.is_held(held)).map(|(target, _)| *target).collect()
}

/// Every key that is one of two keys held together as a chord among `bound`. Such a key is the
/// chord's while it is held, and does not count as down for what it alone is bound to.
pub fn keys_in_held_chords(bound: &[(Bindable, KeyBind)], held: &HeldKeys) -> HashSet<KeyCode> {
    bound.iter().filter(|(_, bind)| matches!(bind, KeyBind::Chord(..)) && bind.is_held(held)).flat_map(|(_, bind)| bind.keys()).collect()
}

/// What the key editor makes of the keys pressed while it waits for a binding: one key pressed and
/// let go is that key, a second pressed while the first is held is the two held together. It is the
/// same for the keyboard and for a controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChordCapture<T> {
    first: Option<T>,
}

impl<T> Default for ChordCapture<T> {
    fn default() -> Self {
        ChordCapture { first: None }
    }
}

/// What a capture settled on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Captured<T> {
    Single(T),
    Chord(T, T),
}

impl<T: Copy + PartialEq> ChordCapture<T> {
    /// A press: the first is held on to, a second while it is held settles on the two.
    pub fn press(&mut self, pressed: T) -> Option<Captured<T>> {
        match self.first {
            None => {
                self.first = Some(pressed);
                None
            }
            Some(first) if first == pressed => None,
            Some(first) => {
                self.first = None;
                Some(Captured::Chord(first, pressed))
            }
        }
    }

    /// A release: the first let go before a second came settles on it alone. Anything else let go
    /// is something pressed before the capture began.
    pub fn release(&mut self, released: T) -> Option<Captured<T>> {
        if self.first != Some(released) {
            return None;
        }
        self.first = None;
        Some(Captured::Single(released))
    }
}

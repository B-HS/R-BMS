//! The textures the GPU backend holds, under the handles it gave out for them.
//!
//! A texture let go of leaves its slot free for the next one registered, so a screen that uploads
//! a thousand pictures for each chart it plays, and lets them all go when it is done, holds the
//! same slots from one chart to the next rather than a thousand more each time.
//!
//! A handle is never handed out twice, all the same. It carries how many times its slot has been
//! taken as well as which slot it is, and only the handle the slot was last taken under names what
//! is in it. A handle kept past [`TextureSlots::release`] by an owner that did not hear of it names
//! nothing, as it always did, rather than whatever texture took its slot afterwards. A slot that
//! has been taken as many times as a handle can count is not taken again.

use std::collections::HashMap;

use rbms_render::TextureId;

/// Bits of a handle that say which slot it names. The rest count the times the slot was taken.
const SLOT_BITS: u32 = 20;

/// The part of a handle that says which slot it names.
const SLOT_MASK: u32 = (1 << SLOT_BITS) - 1;

/// Slots there can be. The last slot number is never given to a slot, so [`NO_TEXTURE`] names
/// none whatever it counts.
const SLOT_LIMIT: usize = SLOT_MASK as usize;

/// The most times a slot can be taken before the count would run out of bits.
const LAST_TAKING: u32 = u32::MAX >> SLOT_BITS;

/// The handle given out when every slot there can be is held: it names no slot, so it never names
/// a texture.
pub(super) const NO_TEXTURE: TextureId = TextureId(u32::MAX);

/// One slot: the handle it was last taken under, the key it was registered with while it is held,
/// and the texture, which is `None` for a key whose image was refused and for a slot let go of.
struct Slot<T> {
    handle: TextureId,
    key: Option<String>,
    texture: Option<T>,
}

/// The textures a backend holds, each under a handle and a key, with the slots of the ones let go
/// of taken again by the next.
pub(super) struct TextureSlots<T> {
    slots: Vec<Slot<T>>,
    /// Slots let go of that can be taken again, the last let go of first.
    free: Vec<usize>,
    keys: HashMap<String, TextureId>,
}

impl<T> Default for TextureSlots<T> {
    fn default() -> TextureSlots<T> {
        TextureSlots { slots: Vec::new(), free: Vec::new(), keys: HashMap::new() }
    }
}

/// The slot a handle names.
fn slot_of(handle: TextureId) -> usize {
    (handle.0 & SLOT_MASK) as usize
}

/// How many times the slot a handle names had been taken when the handle was given out.
fn taking_of(handle: TextureId) -> u32 {
    handle.0 >> SLOT_BITS
}

/// The handle of `slot` taken for the `taking`th time.
fn handle_of(slot: usize, taking: u32) -> TextureId {
    TextureId((taking << SLOT_BITS) | slot as u32)
}

impl<T> TextureSlots<T> {
    /// The slot `handle` names, while the handle is the one it was last taken under and is held.
    fn held(&self, handle: TextureId) -> Option<&Slot<T>> {
        self.slots.get(slot_of(handle)).filter(|slot| slot.handle == handle && slot.key.is_some())
    }

    /// The texture `handle` names, or `None` for a handle let go of, refused, or never given out.
    pub(super) fn get(&self, handle: TextureId) -> Option<&T> {
        self.held(handle).and_then(|slot| slot.texture.as_ref())
    }

    /// The handle `key` is registered under.
    pub(super) fn keyed(&self, key: &str) -> Option<TextureId> {
        self.keys.get(key).copied()
    }

    /// Puts `texture` under `handle`, which keeps its key; `None` empties it, for an image refused.
    /// A handle that is not held is left alone.
    pub(super) fn set(&mut self, handle: TextureId, texture: Option<T>) {
        if self.held(handle).is_some() {
            self.slots[slot_of(handle)].texture = texture;
        }
    }

    /// Registers `texture` under `key`, which no handle is registered under yet, in a slot let go
    /// of when there is one, and answers its handle: [`NO_TEXTURE`] when every slot there can be
    /// is held.
    pub(super) fn insert(&mut self, key: &str, texture: Option<T>) -> TextureId {
        let handle = match self.free.pop() {
            Some(slot) => handle_of(slot, taking_of(self.slots[slot].handle) + 1),
            None if self.slots.len() < SLOT_LIMIT => {
                let handle = handle_of(self.slots.len(), 0);
                self.slots.push(Slot { handle, key: None, texture: None });
                handle
            }
            None => return NO_TEXTURE,
        };
        self.slots[slot_of(handle)] = Slot { handle, key: Some(key.to_string()), texture };
        self.keys.insert(key.to_string(), handle);
        handle
    }

    /// Lets go of what `handle` names, and of its key, and leaves the slot free to be taken again
    /// under a handle of its own. A handle that is not held is left alone.
    pub(super) fn release(&mut self, handle: TextureId) {
        if self.held(handle).is_none() {
            return;
        }
        let index = slot_of(handle);
        let slot = &mut self.slots[index];
        slot.texture = None;
        if let Some(key) = slot.key.take() {
            self.keys.remove(&key);
        }
        if taking_of(handle) < LAST_TAKING {
            self.free.push(index);
        }
    }

    /// How many slots there are, held or not.
    #[cfg(test)]
    pub(super) fn slot_count(&self) -> usize {
        self.slots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_let_go_of_is_taken_again_under_a_handle_of_its_own() {
        let mut slots = TextureSlots::default();
        let first = slots.insert("a", Some(1));
        assert_eq!(slots.get(first), Some(&1));
        slots.release(first);
        assert_eq!((slots.get(first), slots.keyed("a")), (None, None), "a texture let go of is still named");

        let second = slots.insert("b", Some(2));
        assert_eq!(slots.slot_count(), 1, "the slot let go of was not taken again");
        assert_ne!(second, first, "a handle was handed out twice");
        assert_eq!((slots.get(first), slots.get(second)), (None, Some(&2)), "the old handle names the texture that took its slot");

        slots.release(first);
        assert_eq!(slots.get(second), Some(&2), "letting go of an old handle let go of the texture that took its slot");
        slots.set(first, Some(9));
        assert_eq!(slots.get(second), Some(&2), "an old handle put a texture into the slot");
    }

    #[test]
    fn a_refused_image_keeps_its_key_and_handle_until_it_is_let_go_of() {
        let mut slots: TextureSlots<u8> = TextureSlots::default();
        let refused = slots.insert("big", None);
        assert_eq!((slots.get(refused), slots.keyed("big")), (None, Some(refused)));
        slots.set(refused, Some(3));
        assert_eq!(slots.get(refused), Some(&3), "the key's handle does not take an image that fits");
        slots.release(refused);
        assert_eq!(slots.keyed("big"), None);
    }

    #[test]
    fn a_slot_taken_as_often_as_a_handle_can_count_is_not_taken_again() {
        let mut slots = TextureSlots::default();
        let worn = handle_of(0, LAST_TAKING);
        slots.slots.push(Slot { handle: worn, key: Some("worn".to_string()), texture: Some(1) });
        slots.keys.insert("worn".to_string(), worn);
        slots.release(worn);
        let next = slots.insert("next", Some(2));
        assert_eq!((slot_of(next), slots.slot_count()), (1, 2), "a slot whose count ran out was taken again");
        assert_eq!(slots.get(handle_of(0, 0)), None, "the count wrapped round to a handle that names a texture");
    }

    #[test]
    fn the_handle_given_when_no_slot_is_left_names_nothing() {
        let slots: TextureSlots<u8> = TextureSlots::default();
        assert_eq!(slot_of(NO_TEXTURE), SLOT_LIMIT, "the handle given when no slot is left names a slot there can be");
        assert_eq!(slots.get(NO_TEXTURE), None);
    }
}

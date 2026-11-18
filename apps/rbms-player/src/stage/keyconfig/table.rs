//! What the key editor lists: one line for each thing a key or a controller element can be given,
//! under the heading of the screen that reads it, with the editor's own rows between them.
//!
//! The lanes, the controls and the controller's rows are the ones the application lists for the
//! mode being edited ([`crate::kc_rows`]). There a thing that can be bound on both devices is two
//! rows, one for each; here it is one line with a column for each, so the controller's half of a
//! pair is not a line of its own. The shortcuts follow, group by group.

use rbms_model::Mode;
use rbms_model::translations::Catalog;
use rbms_render::text::language;

use crate::gamepad::{PadBinding, PadConfig};
use crate::keyconfig::{Bindable, Clash, KeyBind, KeyConfig, Shortcut, ShortcutGroup, clash_among, overlap_among};
use crate::text::menus::key_editor::KeyEditorText;
use crate::{KcRow, is_pad_row, kc_rows};

/// The heading of a line that is not a shortcut, in the language of the screens.
fn heading(words: KeyEditorText) -> Line {
    Line::Heading(words.get(language()))
}

/// One line of the key editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Line {
    /// The name of the group the lines under it belong to. The cursor never rests on one.
    Heading(&'static str),
    /// Which mode's lanes are being edited.
    Mode,
    /// The controller the pad column binds against, and the turntable algorithm it is read with.
    PadDevice,
    PadAnalog,
    /// Something that can be given a key, and a controller element when `pad` says the controller
    /// has a cell for it.
    Bind {
        target: Bindable,
        pad: bool,
    },
    /// Puts every key and every controller binding back as shipped.
    RestoreAll,
}

impl Line {
    /// Whether the cursor can rest on the line.
    pub(super) fn takes_the_cursor(self) -> bool {
        !matches!(self, Line::Heading(_))
    }
}

/// The lines of the editor for `mode`, top to bottom.
pub(super) fn lines(mode: Mode) -> Vec<Line> {
    let rows = kc_rows(mode);
    let on_the_pad: Vec<Bindable> = rows
        .iter()
        .filter(|row| is_pad_row(row))
        .filter_map(|row| match row {
            KcRow::PadControl(action) => Some(Bindable::Control(*action)),
            KcRow::PadLane(lane) => Some(Bindable::Lane(*lane)),
            KcRow::PadScratchReverse(lane) => Some(Bindable::ScratchReverse(*lane)),
            _ => None,
        })
        .collect();
    let bind = |target: Bindable| Line::Bind { target, pad: on_the_pad.contains(&target) };
    let mut top = Vec::new();
    let mut lanes = vec![heading(KeyEditorText::Lanes)];
    let mut play = vec![heading(KeyEditorText::Play)];
    let mut controller = vec![heading(KeyEditorText::Controller)];
    for row in rows {
        match row {
            KcRow::ModeSelect => top.push(Line::Mode),
            KcRow::Lane(lane) => lanes.push(bind(Bindable::Lane(lane))),
            KcRow::ScratchReverse(lane) => lanes.push(bind(Bindable::ScratchReverse(lane))),
            KcRow::Control(action) => play.push(bind(Bindable::Control(action))),
            KcRow::PadDevice => controller.push(Line::PadDevice),
            KcRow::PadAnalogMode => controller.push(Line::PadAnalog),
            KcRow::PadControl(_) | KcRow::PadLane(_) | KcRow::PadScratchReverse(_) => {}
        }
    }
    let shortcuts = ShortcutGroup::ALL.into_iter().flat_map(|group| {
        let rows = Shortcut::ALL
            .into_iter()
            .filter(move |shortcut| shortcut.group() == group)
            .map(|shortcut| Line::Bind { target: Bindable::Shortcut(shortcut), pad: shortcut.takes_a_pad() });
        std::iter::once(Line::Heading(group.title())).chain(rows)
    });
    top.into_iter().chain(lanes).chain(play).chain(shortcuts).chain(controller).chain([heading(KeyEditorText::Defaults), Line::RestoreAll]).collect()
}

/// One thing that can be bound, with what it is bound to on the keyboard and, as its first element
/// and the second one held with it, on the controller.
type Row = (Bindable, Option<KeyBind>, Option<PadBinding>, Option<PadBinding>);

/// Everything that can be bound for the mode being edited, with what each is bound to on the
/// keyboard and on the controller, read once for a frame.
pub(super) struct Bindings {
    rows: Vec<Row>,
}

impl Bindings {
    pub(super) fn read(keys: &KeyConfig, mode: Mode) -> Bindings {
        let pad: &PadConfig = &keys.pad;
        let rows = keys.bound_keys(mode).into_iter().map(|(target, key)| (target, key, pad.binding_of(mode, target), pad.chord_of(target))).collect();
        Bindings { rows }
    }

    fn of(&self, target: Bindable) -> Option<&Row> {
        self.rows.iter().find(|(bound, ..)| *bound == target)
    }

    pub(super) fn key_of(&self, target: Bindable) -> Option<KeyBind> {
        self.of(target).and_then(|(_, key, ..)| *key)
    }

    pub(super) fn pad_of(&self, target: Bindable) -> Option<PadBinding> {
        self.of(target).and_then(|(_, _, pad, _)| *pad)
    }

    /// The element the controller binding of `target` is held together with, when it has two.
    pub(super) fn pad_chord_of(&self, target: Bindable) -> Option<PadBinding> {
        self.of(target).and_then(|(.., second)| *second)
    }

    /// Why the key or the two keys `target` has are ones it should not have, if they are.
    pub(super) fn key_clash(&self, target: Bindable) -> Option<Clash> {
        let bind = self.key_of(target)?;
        clash_among(self.rows.iter().map(|(bound, key, ..)| (*bound, *key)), target, bind)
    }

    /// What else the controller element or the two elements `target` has drives at the same
    /// moment, if anything: the same element, or the same two in either order.
    pub(super) fn pad_clash(&self, target: Bindable) -> Option<Bindable> {
        let mine = (self.pad_of(target)?, self.pad_chord_of(target));
        self.rows
            .iter()
            .find(|(other, _, first, second)| *other != target && first.is_some_and(|first| same_elements(mine, (first, *second))) && target.read_with(*other))
            .map(|(other, ..)| *other)
    }

    /// A controller element `target` shares with something read at the same moment, one of them
    /// alone and the other with a second element held ([`overlap_among`]), and what has it.
    pub(super) fn pad_overlap(&self, target: Bindable) -> Option<(PadBinding, Bindable)> {
        let bound: Vec<(Bindable, Option<PadBinding>, Option<PadBinding>)> =
            self.rows.iter().map(|(bound, _, first, second)| (*bound, *first, *second)).collect();
        overlap_among(&bound, target)
    }
}

/// Whether two controller bindings are the same element, or the same two held together in either
/// order.
fn same_elements(one: (PadBinding, Option<PadBinding>), two: (PadBinding, Option<PadBinding>)) -> bool {
    match (one, two) {
        ((a, Some(b)), (c, Some(d))) => (a, b) == (c, d) || (a, b) == (d, c),
        ((a, None), (c, None)) => a == c,
        _ => false,
    }
}

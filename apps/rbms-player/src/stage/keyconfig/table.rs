//! What the key editor lists: one line for each thing a key or a controller element can be given,
//! under the heading of the screen that reads it, with the editor's own rows between them.
//!
//! The lanes, the controls and the controller's rows are the ones the application lists for the
//! mode being edited ([`crate::kc_rows`]). There a thing that can be bound on both devices is two
//! rows, one for each; here it is one line with a column for each, so the controller's half of a
//! pair is not a line of its own. The shortcuts follow, group by group.

use rbms_model::Mode;
use winit::keyboard::KeyCode;

use crate::gamepad::{PadBinding, PadConfig};
use crate::keyconfig::{Bindable, Clash, KeyConfig, Shortcut, ShortcutGroup, clash_among};
use crate::{KcRow, is_pad_row, kc_rows};

/// The headings of the lines that are not shortcuts.
const LANES_HEADING: &str = "LANES";
const PLAY_HEADING: &str = "PLAY";
const CONTROLLER_HEADING: &str = "CONTROLLER";
const DEFAULTS_HEADING: &str = "DEFAULTS";

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
    let mut lanes = vec![Line::Heading(LANES_HEADING)];
    let mut play = vec![Line::Heading(PLAY_HEADING)];
    let mut controller = vec![Line::Heading(CONTROLLER_HEADING)];
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
    top.into_iter().chain(lanes).chain(play).chain(shortcuts).chain(controller).chain([Line::Heading(DEFAULTS_HEADING), Line::RestoreAll]).collect()
}

/// Everything that can be bound for the mode being edited, with what each is bound to on the
/// keyboard and on the controller, read once for a frame.
pub(super) struct Bindings {
    rows: Vec<(Bindable, Option<KeyCode>, Option<PadBinding>)>,
}

impl Bindings {
    pub(super) fn read(keys: &KeyConfig, mode: Mode) -> Bindings {
        let pad: &PadConfig = &keys.pad;
        Bindings { rows: keys.bound_keys(mode).into_iter().map(|(target, key)| (target, key, pad.binding_of(mode, target))).collect() }
    }

    fn of(&self, target: Bindable) -> Option<&(Bindable, Option<KeyCode>, Option<PadBinding>)> {
        self.rows.iter().find(|(bound, _, _)| *bound == target)
    }

    pub(super) fn key_of(&self, target: Bindable) -> Option<KeyCode> {
        self.of(target).and_then(|(_, key, _)| *key)
    }

    pub(super) fn pad_of(&self, target: Bindable) -> Option<PadBinding> {
        self.of(target).and_then(|(_, _, pad)| *pad)
    }

    /// Why the key `target` has is one it should not have, if it is.
    pub(super) fn key_clash(&self, target: Bindable) -> Option<Clash> {
        let code = self.key_of(target)?;
        clash_among(self.rows.iter().map(|(bound, key, _)| (*bound, *key)), target, code)
    }

    /// What else the controller element `target` has drives at the same moment, if anything.
    pub(super) fn pad_clash(&self, target: Bindable) -> Option<Bindable> {
        let binding = self.pad_of(target)?;
        self.rows.iter().find(|(other, _, pad)| *other != target && *pad == Some(binding) && target.read_with(*other)).map(|(other, _, _)| *other)
    }
}

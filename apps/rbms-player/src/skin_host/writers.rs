//! What a skin writes back: the share a dragged slider is set to and the text typed into an
//! editable field.
//!
//! A write arrives while a frame is being drawn or the pointer is being routed, when nothing may
//! change, so the host records each one as it arrives and hands them over when the frame is done
//! ([`super::ScreenHost::take_calls`]). Applying them belongs here: which setting a rate id moves and
//! what a string id is typed into.
//!
//! # Who carries out which write
//!
//! The reference can write seven rates and one string (`FloatPropertyFactory.getRateWriter`,
//! `StringPropertyFactory.getStringWriter`). A write goes to the cluster that owns its id
//! ([`super::WRITE_ROUTES`]); what each of them comes to here:
//!
//! | id | reference | here |
//! | --- | --- | --- |
//! | rate 1, `musicselect_position` | moves the browser's cursor to that share of the list | the browser: [`BrowserWrite::Position`] |
//! | rate 7, `skinselect_position` | scrolls the skin settings screen | nothing: there is no such screen, the skin tab is built in |
//! | rate 8, `ranking_position` | scrolls the ranking list of the browser or the result screen | the result screen, which holds the offset; the browser has no ranking list a skin draws yet |
//! | rates 17, 18, 19 | the system, key and background volume | [`carry_out`], at once |
//! | rate 20, `practice_position` | scrolls the practice items of the play screen | nothing: practice is a screen of its own here |
//! | string 30, `searchword` | `MusicSelector.search` | the browser: [`BrowserWrite::Search`] |
//!
//! A volume belongs to the application rather than to any screen, so it is applied when the frame
//! that made it ends. The cursor and the search belong to the browser's own state, so they wait in
//! the request queue for it ([`browser_write`]) and are applied by it. A write to an id nobody here
//! carries out is dropped without a word, as the reference drops an event no screen defines.
//!
//! # Typing into a text
//!
//! [`TextSession`] is one editable text being typed into: the line, the caret and what an input
//! method is composing. It decides what each key does to it and when it is over, and hands the typed
//! line back as a [`TextWrite`] to be written inside the next frame of the screen it was typed on,
//! where a writer that is a function of the skin can be called.

use rbms_config::{AudioOptions, DEFAULT_BUS_VOLUME, clamp_volume};
use rbms_render::skin_render::{Composition, SkinTextWriter, TextEntry, TextEntryStart};
use rbms_skin::lua::BoundFrame;
use rbms_skin::property::generated::{RATE_BGMVOLUME, RATE_KEYVOLUME, RATE_MASTERVOLUME, RATE_MUSICSELECT_POSITION, STRING_SEARCHWORD};
use rbms_skin::property::{SkinHost, VolumeBus};
use winit::event::Ime;

use super::ClusterRequest;
use crate::AppShared;
use crate::KeyCode;
use crate::stage::KeyInput;
use crate::textedit::{ImeEdit, edit_key};

#[cfg(test)]
mod tests;

/// The volume a rate id sets, or `None` for a rate that is no volume.
pub(crate) fn rate_bus(id: i32) -> Option<VolumeBus> {
    match id {
        RATE_MASTERVOLUME => Some(VolumeBus::System),
        RATE_KEYVOLUME => Some(VolumeBus::Key),
        RATE_BGMVOLUME => Some(VolumeBus::Background),
        _ => None,
    }
}

/// Sets one volume of the audio settings to `value`, and answers whether that moved it.
///
/// The reference stores whatever it is given and pulls it back into range when the settings are next
/// read (`AudioConfig.validate`: a value that is not a number becomes the default and any other is held
/// to `0..=1`); a volume out of range would be heard in the meantime, so it is pulled in here.
pub(crate) fn set_volume(audio: &mut AudioOptions, bus: VolumeBus, value: f32) -> bool {
    let slot = match bus {
        VolumeBus::System => &mut audio.system,
        VolumeBus::Key => &mut audio.key,
        VolumeBus::Background => &mut audio.bg,
    };
    let held = clamp_volume(value, DEFAULT_BUS_VOLUME);
    let moved = *slot != held;
    *slot = held;
    moved
}

/// Carries out a request that needs no screen to be carried out: a volume, from a slider
/// (`FloatPropertyFactory`) or from `main_state.set_volume_*`. Answers whether `request` was one.
///
/// The running stream hears the change at once and the settings are written out once the pointer is
/// let go, so a drag does not write the file at every step ([`AppShared::note_skin_settings_changed`]).
pub(crate) fn carry_out(shared: &mut AppShared, request: &ClusterRequest) -> bool {
    let (bus, value) = match request {
        ClusterRequest::WriteRate { id, value } => match rate_bus(*id) {
            Some(bus) => (bus, *value),
            None => return false,
        },
        ClusterRequest::SetVolume { bus, value } => (*bus, *value),
        _ => return false,
    };
    if set_volume(&mut shared.config.audio, bus, value) {
        shared.apply_audio_gains();
        shared.note_skin_settings_changed();
    }
    true
}

/// A write the song browser carries out for a skin, in the order the skin made them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BrowserWrite {
    /// The share of the list the scrollbar was dragged to (`musicselect_position`).
    Position(f32),
    /// The search word that was confirmed (`searchword`).
    Search(String),
}

/// What `request` asks of the song browser, or `None` when it is no write the browser carries out.
pub(crate) fn browser_write(request: &ClusterRequest) -> Option<BrowserWrite> {
    match request {
        ClusterRequest::WriteRate { id: RATE_MUSICSELECT_POSITION, value } => Some(BrowserWrite::Position(*value)),
        ClusterRequest::WriteText { id: STRING_SEARCHWORD, value } => Some(BrowserWrite::Search(value.clone())),
        _ => None,
    }
}

/// The place that `share` of the way along `extent` things falls on: `extent * share` cut down to a
/// whole number, for a share from nothing up to but not including one (`BarManager.setSelectedPosition`,
/// `AbstractResult.setRankingPosition`). A share outside that, one that is not a number, and a list
/// with nothing in it fall on no place.
///
/// The end of the scrollbar writes exactly one, which is outside the range: dragging all the way down
/// leaves the cursor where it was, as in the reference.
pub(crate) fn scaled_index(share: f32, extent: usize) -> Option<usize> {
    ((0.0..1.0).contains(&share) && extent > 0).then_some((extent as f32 * share) as usize)
}

/// The key modifiers that turn V into a paste: control, and the platform key a Mac pastes with.
const fn is_paste_modifier(code: KeyCode) -> bool {
    matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::SuperLeft | KeyCode::SuperRight)
}

/// What a key did to the text being typed into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Typing {
    /// Still being typed into.
    Going,
    /// Enter: the typing is over and what was typed is to be written.
    Confirmed,
    /// Escape: the typing is over and what was typed is thrown away.
    Cancelled,
}

/// What a text is to be written as, once it is confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextWrite {
    pub(crate) writer: SkinTextWriter,
    pub(crate) text: String,
}

/// One editable text of a skin, being typed into (`SkinTextInput`).
///
/// It starts from what the text showed when it was pressed, with the caret at the end of it. Enter
/// confirms and Escape throws away; a press outside confirms (the application decides that, from where
/// the text is). The reference passes Escape on to the browser, which quits the game; here every key
/// stays with the text for as long as it is typed into.
#[derive(Debug, Clone)]
pub(crate) struct TextSession {
    screen: i32,
    object: usize,
    writer: SkinTextWriter,
    edit: ImeEdit,
    paste_held: bool,
}

impl TextSession {
    /// Starts typing into the editable text that is object `object` of `screen`'s document.
    pub(crate) fn new(screen: i32, object: usize, start: TextEntryStart) -> TextSession {
        TextSession { screen, object, writer: start.writer, edit: ImeEdit::from_text(start.shown), paste_held: false }
    }

    /// The screen whose document the text is on.
    pub(crate) fn screen(&self) -> i32 {
        self.screen
    }

    /// Which object of that screen it is.
    pub(crate) fn object(&self) -> usize {
        self.object
    }

    /// Applies one key. A modifier is noted and does nothing; everything else is the text's, whether
    /// it was pressed, is repeating, or came up.
    pub(crate) fn key(&mut self, key: &KeyInput<'_>) -> Typing {
        if is_paste_modifier(key.code) {
            self.paste_held = !key.released;
            return Typing::Going;
        }
        if key.released {
            return Typing::Going;
        }
        match key.code {
            KeyCode::Enter | KeyCode::NumpadEnter if key.pressed => Typing::Confirmed,
            KeyCode::Escape if key.pressed => Typing::Cancelled,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape => Typing::Going,
            _ => {
                edit_key(self.edit.line_mut(), key, self.paste_held);
                Typing::Going
            }
        }
    }

    /// Follows the input method: what it is composing, and what it commits.
    pub(crate) fn apply_ime(&mut self, ime: &Ime) {
        self.edit.apply_ime(ime);
    }

    /// The text as a frame draws it.
    pub(crate) fn entry(&self) -> TextEntry<'_> {
        TextEntry {
            object: self.object,
            typed: self.edit.line().text(),
            caret: self.edit.line().cursor(),
            composing: self.edit.composing().map(|composing| Composition { text: &composing.text, caret: composing.caret }),
        }
    }

    /// Ends the typing with what has been typed, as the write that confirms it
    /// (`SkinTextInput.commit`). What an input method is still composing is not part of it.
    pub(crate) fn confirm(mut self) -> TextWrite {
        TextWrite { writer: self.writer, text: self.edit.take() }
    }
}

/// Writes a confirmed text: an id is handed to the host, which records it for the end of the frame,
/// and a function of the skin is called with the text inside the frame's binding. A function with no
/// interpreter to call it in is not run (`SkinLuaAccessor.loadStringWriter`).
pub(crate) fn run_text_write(write: &TextWrite, host: &dyn SkinHost, lua: Option<&BoundFrame<'_>>) {
    match (write.writer, lua) {
        (SkinTextWriter::Id(id), _) => host.write_text(id, &write.text),
        (SkinTextWriter::Function(function), Some(lua)) => lua.call_text_writer(function, &write.text),
        (SkinTextWriter::Function(_), None) => {}
    }
}

//! The song search of the browser: a word being typed, and the list a word left on show.
//!
//! A search is in one of three states. It is off. A word is *being typed*
//! ([`AppShared::searching`]): the search box has the keyboard, and the list is narrowed with every
//! letter. Or what a word found is *on show* ([`SelectState::search_is_on_show`]): the typing is
//! over and the list is a list like any other. Either way the search spans the whole library, and
//! leaving it goes back to the folder and the row it was made from.
//!
//! # Where the word is typed
//!
//! There is one search box. On a skin that has a search text of its own -- an editable text that
//! writes the search word ([`skin_search_text`]) -- that text is the box: the search key and a press
//! on the text both start the typing here, and the frame draws what is typed in the text's place and
//! font ([`SelectState::search_entry`]). On the built-in browser and on a skin with no such text the
//! box is the application's, drawn over the list. The word, the keys and the input method are this
//! screen's in both, so the rules below hold whichever box it is.
//!
//! A word some other writer of a skin hands over whole ([`SelectState::search_for`]) is put on show
//! without any typing.
//!
//! # What a word being typed takes
//!
//! Only what is text ([`SelectState::search_key`]): the keys with a character on them -- the letters
//! the lanes, START and SELECT are on and the digits the reference's browser gives a meaning
//! included -- and the keys that edit a line. Enter ends the typing with what it found left on show,
//! and Escape throws the search away. A word with nothing in it is no search, so confirming one is
//! the same as cancelling.
//!
//! Every other key is the list's. The ones that move or reorder it (the two arrows, and the
//! shortcuts [`WhileTyping::UnderTheBox`] names) do so under the box, and the typing goes on. The
//! ones that put something over the list or leave it for another screen ([`WhileTyping::Opens`], and
//! the key that pins the option overlay) end the typing first, with what was typed left on show, and
//! then do what they do on any list. Which key each of them is on is the key configuration's to
//! say, and a shortcut the player has put on a key that types a character or edits the line is not
//! read while a word is typed: there the key is the word's
//! ([`crate::keyconfig::belongs_to_a_typed_line`]). The mouse and the
//! buttons of a skin are not the keyboard's and work as they always do; a key of a controller ends
//! the typing and does what it does, and so does leaving the browser for any other screen.
//!
//! # What a list on show takes
//!
//! Everything a list takes. The one thing a search changes is where going back goes: out of the
//! search, to where it was made from ([`SelectState::select_back`]), as leaving the folder the
//! reference files a search under does (`SearchWordBar`). The key that opens the search takes the
//! word on show up again to be changed.

use rbms_render::skin_render::{Composition, TextEntry};
use rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT;
use rbms_skin::property::generated::STRING_SEARCHWORD;
use winit::event::Ime;

use super::SelectState;
use crate::app_options::options_key;
use crate::keyconfig::{Shortcut, ShortcutScreen, belongs_to_a_typed_line};
use crate::select_tree::{ListPlace, OpenedFolder, SelectFolder};
use crate::stage::{FrameCtx, KeyInput, StageId, Transition};
use crate::textedit::{ImeEdit, edit_key};
use crate::{AppShared, KeyCode, SelectTab};

/// What the search box draws where the next character will go.
const SEARCH_CARET: &str = "_";

/// Roughly how many characters the search box shows at once, so a query too long for it scrolls
/// under the caret instead of hiding the end being edited. The renderer trims to the exact pixel
/// width on top of this; this only decides which stretch of the query it is handed.
const SEARCH_VISIBLE_CHARS: usize = 24;

/// What a shortcut of the browser comes to while a search word is typed, when it is on a key that
/// is not the word's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WhileTyping {
    /// It reorders, filters or marks the list under the box, and the typing goes on: the order, the
    /// star, the mode filter, the long note mode and the replay slot. So does the reload of the
    /// difficulty tables, which runs beside the browser and opens nothing.
    UnderTheBox,
    /// It ends the typing and is then the list's, because what it does is put a panel over the list
    /// or leave it for another screen.
    Opens,
    /// It ends the typing and pins the option overlay, which the browser kept the key from while the
    /// box had the keyboard.
    PinsTheOverlay,
    /// It does nothing: the search is open already, and the keys that are read as held are not
    /// asked about under a box.
    Nothing,
}

impl WhileTyping {
    fn of(shortcut: Shortcut) -> WhileTyping {
        match shortcut {
            Shortcut::Sort
            | Shortcut::RefSort
            | Shortcut::Favourite
            | Shortcut::RefFavourite
            | Shortcut::RefMode
            | Shortcut::RefLnMode
            | Shortcut::RefReplay
            | Shortcut::ReloadTables => WhileTyping::UnderTheBox,
            Shortcut::Filter
            | Shortcut::Practice
            | Shortcut::LayoutPick
            | Shortcut::Records
            | Shortcut::Ranking
            | Shortcut::Settings
            | Shortcut::SkinMenu
            | Shortcut::PlayMenu
            | Shortcut::KeyConfig
            | Shortcut::RefKeyConfig
            | Shortcut::Folders
            | Shortcut::Tables => WhileTyping::Opens,
            Shortcut::OptionsPin => WhileTyping::PinsTheOverlay,
            Shortcut::Search
            | Shortcut::RefSearch
            | Shortcut::OptionsHold
            | Shortcut::RefDetail
            | Shortcut::Retry
            | Shortcut::NextSong
            | Shortcut::ReplayPause
            | Shortcut::ReplayFaster
            | Shortcut::ReplaySlower
            | Shortcut::ReplayForward
            | Shortcut::ReplayBack
            | Shortcut::ReplayPlayLayout => WhileTyping::Nothing,
        }
    }
}

/// Which object of the skin that is drawing the browser is its search text: the editable text that
/// writes the search word. `None` on the built-in browser and on a skin that has none, which is
/// when the search box is the application's.
pub(super) fn skin_search_text(shared: &AppShared) -> Option<usize> {
    shared.skin_text_writing(SKIN_TYPE_MUSIC_SELECT, STRING_SEARCHWORD)
}

impl SelectState {
    /// Whether what a word found is on show: the typing is over and the search is still on.
    pub(super) fn search_is_on_show(&self, shared: &AppShared) -> bool {
        !shared.searching && self.search_is_on(shared)
    }

    /// Whether a search is on at all, being typed or on show: a word is being typed, there is
    /// somewhere to go back to, or a word narrows the list.
    pub(super) fn search_is_on(&self, shared: &AppShared) -> bool {
        shared.searching || self.search_resume.is_some() || !shared.search.trim().is_empty()
    }

    /// Go into the search unless the browser is in one: the search is over songs and spans the whole
    /// library, so the song tab and the flat song list are brought up, and the folder and the row
    /// that were left are remembered for the way back.
    fn enter_search(&mut self, shared: &mut AppShared) {
        self.tab = SelectTab::Songs;
        if self.search_resume.is_some() {
            return;
        }
        self.search_resume = Some(ListPlace::of(shared));
        shared.select_path = vec![OpenedFolder::plain(SelectFolder::AllSongs)];
        shared.sel = 0;
        self.rebuild(shared);
    }

    /// Open the search box (`/`) to type a word into. Over a search that is on show the box opens
    /// on its word, with the caret at the end of it.
    pub(super) fn start_search(&mut self, shared: &mut AppShared) {
        self.enter_search(shared);
        shared.searching = true;
        self.search_edit = ImeEdit::from_text(shared.search.clone());
        self.cached_key = None;
    }

    /// Start typing a word when the skin's search text was pressed since the last frame, as the
    /// search key does. A press on it while a word is being typed leaves the typing as it is.
    pub(super) fn follow_a_press_on_the_skins_search_text(&mut self, shared: &mut AppShared) {
        let Some(text) = skin_search_text(shared) else {
            return;
        };
        if shared.take_skin_text_press(SKIN_TYPE_MUSIC_SELECT, text) && !shared.searching {
            self.start_search(shared);
        }
    }

    /// The word being typed in `edit` as the skin's search text draws it, or `None` when no word is
    /// being typed or the skin has no search text to draw it in.
    pub(super) fn search_entry<'a>(edit: &'a ImeEdit, shared: &AppShared) -> Option<TextEntry<'a>> {
        let object = skin_search_text(shared).filter(|_| shared.searching)?;
        let composing = edit.composing().map(|composing| Composition { text: &composing.text, caret: composing.caret });
        Some(TextEntry { object, typed: edit.line().text(), caret: edit.line().cursor(), composing })
    }

    /// What the window's input method reports while a word is typed: what it is composing is shown
    /// at the caret, and what it commits is typed and narrows the list like any letter.
    pub(super) fn search_ime(&mut self, shared: &mut AppShared, ime: &Ime) {
        if !shared.searching {
            return;
        }
        self.cached_key = None;
        if self.search_edit.apply_ime(ime) {
            self.apply_search(shared);
        }
    }

    /// Leave the search, typed or on show: clear the word and go back to the folder and the row the
    /// search was made from -- a search is a detour, not a way of leaving the folder you were in.
    ///
    /// A difficulty table can arrive, or come back changed, while the search is on. Where the
    /// search was made from is kept as a place that is found again in the list as it is now
    /// ([`ListPlace`]): the bar the cursor was on wherever it stands now, and the folders the
    /// browser was inside by what they are.
    pub(super) fn exit_search(&mut self, shared: &mut AppShared) {
        shared.searching = false;
        self.search_edit = ImeEdit::default();
        shared.search.clear();
        let place = self.search_resume.take();
        shared.select_path = place.as_ref().map_or_else(|| vec![OpenedFolder::plain(SelectFolder::AllSongs)], |place| place.path(shared));
        shared.sel = 0;
        self.rebuild(shared);
        shared.sel = place.map_or(0, |place| place.row(shared));
        self.cached_key = None;
    }

    /// End the typing with what the word found left on show. A word with nothing in it finds
    /// nothing to show, so the search is left instead (`MusicSelector.search` ignores one too).
    /// What an input method was still composing is not part of the word. Nothing is done when no
    /// word is being typed.
    pub(super) fn confirm_search(&mut self, shared: &mut AppShared) {
        if !shared.searching {
            return;
        }
        if shared.search.trim().is_empty() {
            self.exit_search(shared);
            return;
        }
        shared.searching = false;
        self.search_edit = ImeEdit::from_text(shared.search.clone());
        self.cached_key = None;
    }

    /// Put what `word` finds on show without any typing: a word a skin wrote to the browser whole
    /// (`MusicSelector.search`). A word with nothing in it is ignored.
    pub(super) fn search_for(&mut self, shared: &mut AppShared, word: &str) {
        if word.trim().is_empty() {
            return;
        }
        self.enter_search(shared);
        shared.searching = false;
        self.search_edit = ImeEdit::from_text(word);
        self.apply_search(shared);
    }

    /// Drop a search another screen left behind: one that was on when the browser was left, and is
    /// no longer over the flat song list because the browser was moved off it meanwhile. The word
    /// must not go on narrowing folders nobody searched.
    pub(super) fn drop_a_search_left_behind(&mut self, shared: &mut AppShared) {
        if !self.search_is_on(shared) || shared.is_in_the_flat_list() {
            return;
        }
        self.search_resume = None;
        self.search_edit = ImeEdit::default();
        shared.searching = false;
        shared.search.clear();
        self.rebuild(shared);
        self.cached_key = None;
    }

    /// Take what has been typed into the query the list is filtered by.
    fn apply_search(&mut self, shared: &mut AppShared) {
        shared.search = self.search_edit.line().text().to_string();
        shared.sel = 0;
        self.rebuild(shared);
        self.cached_key = None;
    }

    /// The stretch of the query the application's search box draws, with a caret standing where the
    /// next character will go and what an input method is composing in front of it. A query longer
    /// than the box scrolls with the caret, so editing the start of a long one is not done blind.
    pub(super) fn search_display(&self) -> String {
        let (shown, at) = self.search_edit.line().window(SEARCH_VISIBLE_CHARS);
        let split = shown.char_indices().nth(at).map_or(shown.len(), |(byte, _)| byte);
        let composing = self.search_edit.composing().map_or("", |composing| composing.text.as_str());
        format!("{}{composing}{SEARCH_CARET}{}", &shown[..split], &shown[split..])
    }

    /// One key going down while a word is typed. Answers what the key came to when it was the
    /// word's, and `None` for a key that is the list's and goes on to do what it does there.
    ///
    /// Enter confirms and Escape cancels, and the two arrows that move the cursor move it under the
    /// box. A key that types a character or edits the line is the word's, whatever shortcut the key
    /// configuration has put on it. Any other key is looked up there: a shortcut of
    /// [`WhileTyping::UnderTheBox`] is the list's and leaves the typing as it is, one of
    /// [`WhileTyping::Opens`] ends the typing first, and the key that pins the option overlay ends
    /// the typing too and is handed to the overlay. Everything else does nothing.
    pub(super) fn search_key(&mut self, ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> Option<Transition> {
        match key.code {
            KeyCode::Escape => self.exit_search(ctx.shared),
            KeyCode::Enter | KeyCode::NumpadEnter => self.confirm_search(ctx.shared),
            KeyCode::ArrowUp | KeyCode::ArrowDown => return None,
            code if belongs_to_a_typed_line(code) => self.type_into_search(ctx.shared, key),
            code => match ctx.shared.shortcut_for(ShortcutScreen::Browser, code).map(WhileTyping::of) {
                Some(WhileTyping::UnderTheBox) => return None,
                Some(WhileTyping::Opens) => {
                    self.confirm_search(ctx.shared);
                    return None;
                }
                Some(WhileTyping::PinsTheOverlay) => {
                    self.confirm_search(ctx.shared);
                    options_key(ctx, StageId::Select, false, key);
                }
                Some(WhileTyping::Nothing) | None => self.type_into_search(ctx.shared, key),
            },
        }
        Some(Transition::Stay)
    }

    /// Hand one key to the line being typed, and narrow the list by what it left.
    fn type_into_search(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        edit_key(self.search_edit.line_mut(), key, self.ctrl_held);
        self.cached_key = None;
        if self.search_edit.line().text() != shared.search {
            self.apply_search(shared);
        }
    }
}

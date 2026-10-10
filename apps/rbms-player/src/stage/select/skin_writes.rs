//! What a skin writes to the song browser: the share of the list its scrollbar was dragged to, and
//! the search word typed into its search box.
//!
//! The skin's writes arrive in the request queue of the browser's cluster
//! ([`crate::skin_host::writers`] says which write is whose). The browser owns the list and the
//! search, so it carries these out itself; the queue's other requests -- the events its buttons run --
//! are left where they are for whatever answers them.

use super::SelectState;
use crate::skin_host::Cluster;
use crate::skin_host::writers::{BrowserWrite, browser_write, scaled_index};
use crate::textedit::TextEdit;
use crate::{AppShared, SelectTab};

impl SelectState {
    /// Carries out the writes the skin made since the last frame, oldest first.
    pub(super) fn carry_out_skin_writes(&mut self, shared: &mut AppShared) {
        for request in shared.skin_requests().take_if(Cluster::Select, |request| browser_write(request).is_some()) {
            match browser_write(&request) {
                Some(BrowserWrite::Position(share)) => self.move_to_share(shared, share),
                Some(BrowserWrite::Search(word)) => self.search_for(shared, &word),
                None => {}
            }
        }
    }

    /// Puts the cursor on the row that `share` of the way down the list falls on, on whichever tab is
    /// up (`BarManager.setSelectedPosition`). A share outside the list leaves it where it is.
    ///
    /// The cursor is put there without a sound: the reference's writer moves the bar and starts the
    /// bar timer over and plays nothing, where a move by a key plays the sound of one for every bar
    /// (`FloatPropertyFactory`, `musicselect_position`). A drag writes on every frame the mouse
    /// moves, so a sound for each would be a rattle.
    fn move_to_share(&mut self, shared: &mut AppShared, share: f32) {
        match self.tab {
            SelectTab::Songs => {
                let Some(row) = scaled_index(share, shared.select_items.len()) else {
                    return;
                };
                shared.sel = row;
                self.record_modal = None;
            }
            SelectTab::Courses => {
                if let Some(row) = scaled_index(share, self.courses.len()) {
                    self.courses.focus(row);
                }
            }
        }
    }

    /// Searches the library for a word the skin's search box confirmed (`MusicSelector.search`): the
    /// browser's own search, opened if it is not, with the word typed into it. A word with nothing
    /// in it is ignored.
    fn search_for(&mut self, shared: &mut AppShared, word: &str) {
        if word.trim().is_empty() {
            return;
        }
        self.tab = SelectTab::Songs;
        if !shared.searching {
            self.start_search(shared);
        }
        self.search_edit = TextEdit::from_text(word);
        self.apply_search(shared);
    }
}

#[cfg(test)]
mod tests;

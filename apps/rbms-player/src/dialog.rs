//! The three native pickers the app opens, in one place.
//!
//! They used to sit inline in the three screens that opened them, which meant the screen that owns
//! the font row, the one that owns the folder list and the one that owns the table list each had a
//! blocking call to a native dialog in the middle of it. Collecting them here is what lets the
//! blocking be dealt with once.
//!
//! None of them blocks: the panel is begun on the window's own event loop and its answer is waited
//! for off the frame loop, so the screen underneath keeps drawing and animating while it is up. The
//! screen asks [`DialogHandle`] each frame whether an answer has come back.
//!
//! Each one hands back the path the user picked, or nothing if the dialog was dismissed.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

/// File extensions the font picker offers.
const FONT_EXTENSIONS: [&str; 3] = ["ttf", "otf", "ttc"];

/// Title of the font picker.
const FONT_TITLE: &str = "Select UI font";

/// Title of the song-folder picker.
const SONG_FOLDER_TITLE: &str = "Add song folder";

/// Title of the difficulty-table picker.
const TABLE_TITLE: &str = "Select table json";

/// File extension the table picker offers.
const TABLE_EXTENSION: &str = "json";

/// Where an open picker has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DialogState {
    /// Still up: the user has neither picked nor dismissed it.
    Open,
    /// The user picked this.
    Picked(PathBuf),
    /// The user closed it without picking, or it could not be shown at all.
    Dismissed,
}

/// A native picker that is up right now.
///
/// Held by the screen that opened it and asked once a frame. Dropping it does not close the panel —
/// the answer is simply no longer collected — so a screen that is left mid-pick does not leave the
/// user with a window they cannot dismiss.
#[derive(Debug)]
pub(crate) struct DialogHandle {
    answer: Receiver<Option<PathBuf>>,
}

impl DialogHandle {
    /// What the user has said so far. Never blocks.
    pub(crate) fn poll(&self) -> DialogState {
        match self.answer.try_recv() {
            Ok(Some(path)) => DialogState::Picked(path),
            Ok(None) | Err(TryRecvError::Disconnected) => DialogState::Dismissed,
            Err(TryRecvError::Empty) => DialogState::Open,
        }
    }
}

/// Wait for one picker's answer off the frame loop.
///
/// The panel itself is begun on the main thread by the dialog library — it has to be, it is part of
/// the window — and only the wait happens here, which is the part that would otherwise stop the app
/// drawing.
fn open(picker: impl FnOnce() -> Option<rfd::FileHandle> + Send + 'static) -> DialogHandle {
    let (tx, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(picker().map(|picked| picked.path().to_path_buf()));
    });
    DialogHandle { answer }
}

/// Ask for a UI font file.
pub(crate) fn pick_font_file() -> DialogHandle {
    open(|| pollster::block_on(rfd::AsyncFileDialog::new().add_filter("font", &FONT_EXTENSIONS).set_title(FONT_TITLE).pick_file()))
}

/// Ask for a folder to add to the song library.
pub(crate) fn pick_song_folder() -> DialogHandle {
    open(|| pollster::block_on(rfd::AsyncFileDialog::new().set_title(SONG_FOLDER_TITLE).pick_folder()))
}

/// Ask for a difficulty-table json to add to the library.
pub(crate) fn pick_table_file() -> DialogHandle {
    open(|| pollster::block_on(rfd::AsyncFileDialog::new().set_title(TABLE_TITLE).add_filter(TABLE_EXTENSION, &[TABLE_EXTENSION]).pick_file()))
}

/// A handle whose answer a test supplies, so a screen's waiting can be checked without a user in
/// front of a native panel.
#[cfg(test)]
pub(crate) fn handle_for_tests() -> (std::sync::mpsc::Sender<Option<PathBuf>>, DialogHandle) {
    let (tx, answer) = std::sync::mpsc::channel();
    (tx, DialogHandle { answer })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pickers themselves cannot be opened without a user, so what is pinned here is that the
    /// three of them stay distinct and keep offering the file kinds their screens expect — a font
    /// picker that offered json would silently stop finding fonts.
    #[test]
    fn each_picker_keeps_its_own_title() {
        let titles = [FONT_TITLE, SONG_FOLDER_TITLE, TABLE_TITLE];
        for title in titles {
            assert!(!title.is_empty());
        }
        let mut sorted = titles;
        sorted.sort_unstable();
        sorted.iter().reduce(|a, b| {
            assert_ne!(a, b, "two pickers share a title");
            b
        });
    }

    #[test]
    fn the_font_picker_offers_the_faces_the_text_engine_can_load() {
        assert!(FONT_EXTENSIONS.contains(&"ttf"));
        assert!(FONT_EXTENSIONS.contains(&"otf"));
        assert!(FONT_EXTENSIONS.contains(&"ttc"));
        assert!(!FONT_EXTENSIONS.contains(&TABLE_EXTENSION), "a font picker must not offer table files");
    }

    /// A picker nobody has answered yet reads as open, so the screen holding it keeps waiting
    /// rather than treating silence as a refusal.
    #[test]
    fn a_picker_nobody_has_answered_reads_as_still_open() {
        let (_tx, handle) = handle_for_tests();
        assert_eq!(handle.poll(), DialogState::Open);
    }

    #[test]
    fn a_picked_path_comes_back_once_and_the_picker_is_then_closed() {
        let (tx, handle) = handle_for_tests();
        tx.send(Some(PathBuf::from("/songs/new"))).expect("the handle is still held");
        assert_eq!(handle.poll(), DialogState::Picked(PathBuf::from("/songs/new")));
        drop(tx);
        assert_eq!(handle.poll(), DialogState::Dismissed, "there is no second answer to collect");
    }

    #[test]
    fn a_dismissed_picker_and_one_that_could_not_be_shown_read_the_same() {
        let (tx, handle) = handle_for_tests();
        tx.send(None).expect("the handle is still held");
        assert_eq!(handle.poll(), DialogState::Dismissed);

        let (tx, handle) = handle_for_tests();
        drop(tx);
        assert_eq!(handle.poll(), DialogState::Dismissed, "a picker that never opened is not left waiting for ever");
    }
}

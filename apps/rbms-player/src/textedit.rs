//! One editable line of text, shared by every screen that asks for one.
//!
//! Before this the two editors — the settings screen's in-place row editor and the table manager's
//! URL entry — each kept a `String` and appended to the end of it. They now keep one of these
//! instead, so a caret, a paste and a scrolling window are written once rather than twice.
//!
//! The caret is counted in characters rather than bytes, so a line with a Japanese title in it
//! moves one visible character at a time and can never be cut through the middle of one.

use crate::KeyCode;
use crate::notify::{Level, notify};
use crate::stage::KeyInput;

/// An editable line of text with a caret in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TextEdit {
    buf: String,
    /// Characters — not bytes — before the caret.
    cursor: usize,
}

impl TextEdit {
    /// An empty line.
    pub(crate) fn new() -> TextEdit {
        TextEdit::default()
    }

    /// A line pre-filled with `text`, which is what an editor opened on an existing value starts
    /// as, with the caret at the end of it ready to type.
    pub(crate) fn from_text(text: impl Into<String>) -> TextEdit {
        let buf = text.into();
        let cursor = buf.chars().count();
        TextEdit { buf, cursor }
    }

    /// What has been typed.
    pub(crate) fn text(&self) -> &str {
        &self.buf
    }

    /// How many characters are before the caret. What the caret is for is drawing it, which
    /// [`TextEdit::window`] already reports, so outside a test nothing asks for it on its own.
    #[cfg(test)]
    fn cursor(&self) -> usize {
        self.cursor
    }

    /// How many characters the line holds.
    fn len(&self) -> usize {
        self.buf.chars().count()
    }

    /// Take the text out, leaving the editor empty.
    pub(crate) fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.buf)
    }

    /// Type `text` in at the caret, dropping control characters, and leave the caret after it.
    pub(crate) fn insert(&mut self, text: &str) {
        let typed: String = text.chars().filter(|c| !c.is_control()).collect();
        if typed.is_empty() {
            return;
        }
        let at = self.byte_of(self.cursor);
        self.buf.insert_str(at, &typed);
        self.cursor += typed.chars().count();
    }

    /// Delete the character before the caret.
    pub(crate) fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let at = self.byte_of(self.cursor);
        self.buf.remove(at);
    }

    /// Delete the character under the caret, leaving the caret where it is.
    pub(crate) fn delete(&mut self) {
        if self.cursor >= self.len() {
            return;
        }
        let at = self.byte_of(self.cursor);
        self.buf.remove(at);
    }

    /// Move the caret one character left.
    pub(crate) fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move the caret one character right.
    pub(crate) fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    /// Move the caret to the start of the line.
    pub(crate) fn home(&mut self) {
        self.cursor = 0;
    }

    /// Move the caret to the end of the line.
    pub(crate) fn end(&mut self) {
        self.cursor = self.len();
    }

    /// Type in what is on the clipboard. A clipboard that cannot be read — no display server, or
    /// nothing text-shaped on it — is reported rather than silently doing nothing, because from the
    /// keyboard the two look identical.
    pub(crate) fn paste(&mut self) {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
            Ok(text) => self.insert(&text),
            Err(e) => notify(Level::Warn, format!("paste failed: {e}")),
        }
    }

    /// The stretch of the line an entry box `chars` wide shows, and where the caret sits inside it.
    ///
    /// The window follows the caret rather than the end of the line, so editing the middle of a URL
    /// too long to show keeps what is being edited on screen.
    pub(crate) fn window(&self, chars: usize) -> (String, usize) {
        let len = self.len();
        if chars == 0 {
            return (String::new(), 0);
        }
        if len <= chars {
            return (self.buf.clone(), self.cursor);
        }
        let first = self.cursor.saturating_sub(chars).min(len - chars);
        let shown: String = self.buf.chars().skip(first).take(chars).collect();
        (shown, self.cursor - first)
    }

    /// Byte offset of character `index`, or the end of the line.
    fn byte_of(&self, index: usize) -> usize {
        self.buf.char_indices().nth(index).map(|(at, _)| at).unwrap_or(self.buf.len())
    }
}

/// Apply one keystroke to an open editor: the caret keys, the two deletes, a paste, or a typed
/// character. Enter and Escape are the screen's to interpret and never reach here.
///
/// `paste_held` is the screen's own record of whether a paste modifier is down. While it is, a
/// letter is a shortcut rather than a character, so nothing but V is taken.
pub(crate) fn edit_key(edit: &mut TextEdit, key: &KeyInput<'_>, paste_held: bool) {
    match key.code {
        KeyCode::Backspace => edit.backspace(),
        KeyCode::Delete => edit.delete(),
        KeyCode::ArrowLeft => edit.left(),
        KeyCode::ArrowRight => edit.right(),
        KeyCode::Home => edit.home(),
        KeyCode::End => edit.end(),
        KeyCode::KeyV if paste_held => edit.paste(),
        _ if paste_held => {}
        _ => {
            if let Some(text) = key.text {
                edit.insert(text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, text: Option<&'static str>) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text }
    }

    #[test]
    fn a_keystroke_moves_the_caret_and_a_character_lands_at_it() {
        let mut edit = TextEdit::from_text("abc");
        edit_key(&mut edit, &key(KeyCode::Home, None), false);
        edit_key(&mut edit, &key(KeyCode::ArrowRight, None), false);
        edit_key(&mut edit, &key(KeyCode::KeyX, Some("X")), false);
        assert_eq!(edit.text(), "aXbc");
        edit_key(&mut edit, &key(KeyCode::Backspace, None), false);
        assert_eq!(edit.text(), "abc");
        edit_key(&mut edit, &key(KeyCode::Delete, None), false);
        assert_eq!(edit.text(), "ac");
        edit_key(&mut edit, &key(KeyCode::End, None), false);
        assert_eq!(edit.cursor(), 2);
    }

    /// A letter typed with a paste modifier down is a shortcut, not a character, so it must not
    /// land in the line.
    #[test]
    fn a_letter_under_the_paste_modifier_is_not_typed() {
        let mut edit = TextEdit::from_text("url");
        edit_key(&mut edit, &key(KeyCode::KeyA, Some("a")), true);
        assert_eq!(edit.text(), "url");
    }

    #[test]
    fn a_fresh_editor_is_empty() {
        assert_eq!(TextEdit::new().text(), "");
        assert_eq!(TextEdit::new().cursor(), 0);
    }

    #[test]
    fn an_editor_opened_on_a_value_holds_it_with_the_caret_ready_to_type() {
        let edit = TextEdit::from_text("https://ir.example/api");
        assert_eq!(edit.text(), "https://ir.example/api");
        assert_eq!(edit.cursor(), edit.len(), "the caret starts where typing continues the value");
    }

    /// This is exactly what the two editors did before: type at the end, backspace from the end.
    #[test]
    fn typing_and_backspacing_at_the_end_behave_as_they_always_did() {
        let mut edit = TextEdit::new();
        edit.insert("dj");
        edit.insert("x");
        assert_eq!(edit.text(), "djx");
        edit.backspace();
        assert_eq!(edit.text(), "dj");
        edit.backspace();
        edit.backspace();
        assert_eq!(edit.text(), "");
        edit.backspace();
        assert_eq!(edit.text(), "", "backspacing an empty line is not an error");
        assert_eq!(edit.cursor(), 0);
    }

    #[test]
    fn control_characters_are_dropped_rather_than_typed() {
        let mut edit = TextEdit::new();
        edit.insert("a\u{8}b\nc\td");
        assert_eq!(edit.text(), "abcd");
        assert_eq!(edit.cursor(), 4, "the caret counts what was typed, not what was offered");
        edit.insert("\n");
        assert_eq!(edit.text(), "abcd", "an insert of nothing but control characters changes nothing");
    }

    /// A multi-byte character is deleted whole, or backspacing over a title in Japanese would
    /// split it into invalid bytes.
    #[test]
    fn a_multi_byte_character_is_typed_and_deleted_whole() {
        let mut edit = TextEdit::from_text("あい");
        edit.insert("x");
        assert_eq!(edit.text(), "あいx");
        edit.backspace();
        assert_eq!(edit.text(), "あい");
        edit.backspace();
        assert_eq!(edit.text(), "あ");
    }

    #[test]
    fn taking_the_text_leaves_the_editor_empty() {
        let mut edit = TextEdit::from_text("value");
        assert_eq!(edit.take(), "value");
        assert_eq!(edit.text(), "");
        assert_eq!(edit.cursor(), 0);
    }

    #[test]
    fn the_caret_moves_a_character_at_a_time_and_stops_at_both_ends() {
        let mut edit = TextEdit::from_text("abc");
        edit.left();
        assert_eq!(edit.cursor(), 2);
        edit.home();
        assert_eq!(edit.cursor(), 0);
        edit.left();
        assert_eq!(edit.cursor(), 0, "there is nothing left of the start");
        edit.right();
        assert_eq!(edit.cursor(), 1);
        edit.end();
        assert_eq!(edit.cursor(), 3);
        edit.right();
        assert_eq!(edit.cursor(), 3, "there is nothing right of the end");
    }

    #[test]
    fn typing_in_the_middle_puts_the_characters_where_the_caret_is() {
        let mut edit = TextEdit::from_text("http//example");
        for _ in 0..9 {
            edit.left();
        }
        assert_eq!(edit.cursor(), 4);
        edit.insert("s:");
        assert_eq!(edit.text(), "https://example");
        assert_eq!(edit.cursor(), 6, "the caret follows what was typed");
    }

    #[test]
    fn backspace_and_delete_take_the_characters_on_either_side_of_the_caret() {
        let mut edit = TextEdit::from_text("abcd");
        edit.left();
        edit.left();
        assert_eq!(edit.cursor(), 2);
        edit.backspace();
        assert_eq!((edit.text(), edit.cursor()), ("acd", 1));
        edit.delete();
        assert_eq!((edit.text(), edit.cursor()), ("ad", 1), "delete takes the one in front and leaves the caret");
        edit.end();
        edit.delete();
        assert_eq!(edit.text(), "ad", "there is nothing in front of the end to delete");
    }

    /// A caret in the middle of a multi-byte line has to land between characters, not inside one.
    #[test]
    fn the_caret_walks_multi_byte_characters_one_at_a_time() {
        let mut edit = TextEdit::from_text("あxい");
        edit.left();
        edit.backspace();
        assert_eq!(edit.text(), "あい");
        assert_eq!(edit.cursor(), 1);
        edit.insert("y");
        assert_eq!(edit.text(), "あyい");
    }

    #[test]
    fn a_line_that_fits_is_shown_whole_with_its_caret_where_it_is() {
        let mut edit = TextEdit::from_text("short");
        edit.left();
        assert_eq!(edit.window(10), ("short".to_string(), 4));
        assert_eq!(edit.window(0), (String::new(), 0), "an entry box with no room shows nothing");
    }

    /// The window follows the caret, so editing the middle of a URL too long to show keeps the part
    /// being edited on screen instead of jumping to the end of the line.
    #[test]
    fn a_long_line_shows_the_stretch_the_caret_is_in() {
        let mut edit = TextEdit::from_text("0123456789");
        let (shown, caret) = edit.window(4);
        assert_eq!((shown.as_str(), caret), ("6789", 4), "typing at the end shows the end");
        edit.home();
        let (shown, caret) = edit.window(4);
        assert_eq!((shown.as_str(), caret), ("0123", 0), "and the start once the caret is there");
        for _ in 0..6 {
            edit.right();
        }
        let (shown, caret) = edit.window(4);
        assert_eq!(caret, 4);
        assert_eq!(shown, "2345", "the caret stays at the right-hand edge of the window as it walks");
    }

    #[test]
    fn the_window_never_shows_more_than_the_box_holds() {
        let edit = TextEdit::from_text("0123456789");
        for width in 1..=12 {
            let (shown, caret) = edit.window(width);
            assert!(shown.chars().count() <= width, "width {width} showed {shown}");
            assert!(caret <= shown.chars().count(), "width {width} put the caret at {caret} of {shown}");
        }
    }
}

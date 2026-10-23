//! Fixed words of the PLAY, SKIN and PRACTICE menus, the tables, folders and loading screens, the key editor's headings, the course lines and the IR
//! lines of the result screen, by language.
//!
//! One module per screen, each a catalog of its own. A word that is a term every language keeps as it is (a judgement, a gauge, a lamp, an option, a
//! key) is not here: the screen writes it as it is. A word a skin, a chart, a table or the score server supplies is not here either.

pub(crate) mod course;
pub(crate) mod folders;
pub(crate) mod ir_outcome;
pub(crate) mod key_editor;
pub(crate) mod loading;
pub(crate) mod play_menu;
pub(crate) mod practice;
pub(crate) mod preview_menu;
pub(crate) mod record_note;
pub(crate) mod skin_menu;
pub(crate) mod tables;

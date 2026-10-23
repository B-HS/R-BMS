//! Where the song browser is: the folders it has opened, outermost first, each with the bar the
//! cursor was on when it was opened (`BarManager.dir` and `sourcebars`).
//!
//! A folder is named by what it is rather than by where it sits in a list -- a table by its
//! address, a level by its name, one of the player's own folders by its id -- so the same folder
//! is found again after the tables, the folders or the library under it changed. Leaving a folder
//! puts the cursor back on the bar it was opened from, found the same way.

use rbms_model::translations::Catalog;
use rbms_render::skin_render::frame::BarKind;
use rbms_render::text::language;

use crate::tablesrc::{TableLevels, UNLOADED_TABLE};
use crate::text::menus::course::CourseText;
use crate::text::select::tree::TreeText;
use crate::{AppShared, SelectItem};

/// What the reference writes for the folder that holds every chart, in the language the screens are
/// written in.
pub(crate) fn all_songs_title() -> &'static str {
    TreeText::AllSongs.get(language())
}

/// What the folder of the courses a table lists is titled, after the reference's folder of courses
/// (`BarManager.java:207`), in the language the screens are written in.
pub(crate) fn table_courses_title() -> &'static str {
    CourseText::TabCourses.get(language())
}

/// What the folder of the player's own folders is titled, in the language the screens are written in.
pub(crate) fn user_folders_title() -> &'static str {
    TreeText::MyFolders.get(language())
}

/// What a level of a table is titled after when the table names no symbol of its own.
const TABLE_LEVEL_PREFIX: &str = "LV";

/// A folder the browser can open (`DirectoryBar`), named by what it is rather than by where it sits
/// in a list, so the same folder is found again after the tables or the library under it changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelectFolder {
    AllSongs,
    /// Several tables shown under one row of the root (no counterpart in the reference).
    TableGroup(String),
    Table(TableRef),
    /// A level of a table, by the level's own name (`HashBar`).
    TableLevel(TableRef, String),
    /// The courses a table lists, after its levels (the reference's `GradeBar`s inside a `TableBar`).
    TableCourses(TableRef),
    UserFolders,
    /// One of the player's own folders, by its id.
    UserFolder(String),
}

/// A difficulty table of the player's list: its address, and the place it was at when this was
/// made, which is only a hint once the list has changed.
#[derive(Clone, Debug, Eq)]
pub(crate) struct TableRef {
    pub(crate) at: usize,
    pub(crate) location: String,
}

/// One open folder and the bar the cursor was on when it was opened (`BarManager.dir`, `sourcebars`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenedFolder {
    pub(crate) folder: SelectFolder,
    pub(crate) came_from: Option<BarIdentity>,
}

/// What a bar is, for finding it again in a list built afresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BarIdentity {
    /// A chart, owned or missing, by its md5 in lower case.
    Chart(String),
    /// A row that opens a folder.
    Opens(SelectFolder),
    /// A course, by its id ([`rbms_course::Course::hash`]), which tells apart two courses of one
    /// name.
    Course(String),
}

impl PartialEq for TableRef {
    /// Two tables are the same table when they are at the same address. A table named by its place
    /// alone -- a list filled in without the sources it came from -- is the table at that place.
    fn eq(&self, other: &TableRef) -> bool {
        if self.location.is_empty() || other.location.is_empty() { self.at == other.at } else { self.location == other.location }
    }
}

impl TableRef {
    /// The table at `at` of the player's list.
    pub(crate) fn of(shared: &AppShared, at: usize) -> TableRef {
        TableRef { at, location: shared.config.library.tables.get(at).map(|source| source.location.clone()).unwrap_or_default() }
    }

    /// Where this table is in the player's list now: the place it was at while the same address is
    /// still there, else the first place of its address, or `None` when the list has it no more.
    pub(crate) fn resolve(&self, shared: &AppShared) -> Option<usize> {
        let tables = &shared.config.library.tables;
        if tables.get(self.at).is_some_and(|source| source.location == self.location) {
            return Some(self.at);
        }
        if self.location.is_empty() {
            return (self.at < shared.table_levels.len()).then_some(self.at);
        }
        tables.iter().position(|source| source.location == self.location)
    }

    /// This table as it stands now, at the place it is at now.
    fn now(&self, shared: &AppShared) -> Option<TableRef> {
        self.resolve(shared).map(|at| TableRef { at, location: self.location.clone() })
    }

    /// The levels of this table, as matched against the library.
    fn levels<'a>(&self, shared: &'a AppShared) -> Option<&'a TableLevels> {
        self.resolve(shared).and_then(|at| shared.table_levels.get(at))
    }

    /// The owned charts of the level of this table called `level`.
    pub(crate) fn level<'a>(&self, shared: &'a AppShared, level: &str) -> Option<&'a [usize]> {
        self.levels(shared)?.iter().find(|(name, _)| name == level).map(|(_, charts)| charts.as_slice())
    }
}

/// The group the table at `at` of the player's list is shown under, or an empty name for a table
/// shown on a row of its own.
pub(crate) fn table_group(shared: &AppShared, at: usize) -> &str {
    shared.config.library.tables.get(at).map_or("", |source| source.group.as_str())
}

/// What the table at `at` is called in the list.
pub(crate) fn table_name(shared: &AppShared, at: usize) -> String {
    shared.table_names.get(at).cloned().unwrap_or_else(|| UNLOADED_TABLE.to_string())
}

/// What the level `level` of the table at `at` is titled: the symbol the table names put in front
/// of the level (`TableDataAccessor`: `tag + lv`), or `LV` and the level when it names none.
pub(crate) fn level_title(shared: &AppShared, at: usize, level: &str) -> String {
    match shared.table_extras.get(at).and_then(|extras| extras.symbol.as_deref()) {
        Some(symbol) => format!("{symbol}{level}"),
        None => format!("{TABLE_LEVEL_PREFIX} {level}"),
    }
}

impl SelectFolder {
    /// What the folder is titled, without the count its bar adds.
    pub(crate) fn title(&self, shared: &AppShared) -> String {
        match self {
            SelectFolder::AllSongs => all_songs_title().to_string(),
            SelectFolder::TableGroup(name) => name.clone(),
            SelectFolder::Table(table) => table.resolve(shared).map_or_else(|| UNLOADED_TABLE.to_string(), |at| table_name(shared, at)),
            SelectFolder::TableLevel(table, level) => {
                table.resolve(shared).map_or_else(|| format!("{TABLE_LEVEL_PREFIX} {level}"), |at| level_title(shared, at, level))
            }
            SelectFolder::TableCourses(_) => table_courses_title().to_string(),
            SelectFolder::UserFolders => user_folders_title().to_string(),
            SelectFolder::UserFolder(id) => {
                shared.user_folders.shown.iter().find(|folder| &folder.id == id).map(|folder| folder.name.clone()).unwrap_or_default()
            }
        }
    }

    /// Whether the folder is still there to be looked into.
    pub(crate) fn is_there(&self, shared: &AppShared) -> bool {
        match self {
            SelectFolder::AllSongs => true,
            SelectFolder::TableGroup(name) => {
                shared.table_levels.iter().enumerate().any(|(at, levels)| !levels.is_empty() && table_group(shared, at) == name.as_str())
            }
            SelectFolder::Table(table) => table.levels(shared).is_some_and(|levels| !levels.is_empty()),
            SelectFolder::TableLevel(table, level) => table.level(shared, level).is_some(),
            SelectFolder::TableCourses(table) => table.resolve(shared).is_some_and(|at| shared.table_courses(at).next().is_some()),
            SelectFolder::UserFolders => !shared.user_folders.shown.is_empty(),
            SelectFolder::UserFolder(id) => shared.user_folders.shown.iter().any(|folder| &folder.id == id),
        }
    }

    /// Which of the reference's bars a row that opens this folder stands for: a folder of charts
    /// for the flat list, a container of other bars for a group of tables (`ContainerBar`), and a
    /// table for a difficulty table, for a level of one and for its courses (`TableBar`, `HashBar`,
    /// and the `TableBar` the reference's folder of courses is). The player's
    /// own folders are charts picked by hash too, held by one row the way a table holds its levels,
    /// so they are the same two bars.
    pub(crate) fn bar_kind(&self) -> BarKind {
        match self {
            SelectFolder::AllSongs => BarKind::Folder,
            SelectFolder::TableGroup(_) => BarKind::Command,
            SelectFolder::Table(_) | SelectFolder::TableLevel(..) | SelectFolder::TableCourses(_) | SelectFolder::UserFolders | SelectFolder::UserFolder(_) => {
                BarKind::Table
            }
        }
    }

    /// This folder with the tables it names at the places they are at now, or `None` when one of
    /// them has left the list.
    fn now(&self, shared: &AppShared) -> Option<SelectFolder> {
        Some(match self {
            SelectFolder::Table(table) => SelectFolder::Table(table.now(shared)?),
            SelectFolder::TableLevel(table, level) => SelectFolder::TableLevel(table.now(shared)?, level.clone()),
            SelectFolder::TableCourses(table) => SelectFolder::TableCourses(table.now(shared)?),
            folder => folder.clone(),
        })
    }
}

impl OpenedFolder {
    /// `folder` opened with no bar under the cursor to go back to.
    pub(crate) fn plain(folder: SelectFolder) -> OpenedFolder {
        OpenedFolder { folder, came_from: None }
    }
}

impl AppShared {
    /// What the bar of `item` is, for finding it again in a list built afresh. A chart the library
    /// does not hold is the same chart as the one it becomes once it is held, so the cursor stays on
    /// it when it arrives.
    pub(crate) fn identity_of(&self, item: &SelectItem) -> Option<BarIdentity> {
        match item {
            SelectItem::Song(index) => self.library.songs().get(*index).map(|entry| BarIdentity::Chart(entry.md5.to_ascii_lowercase())),
            SelectItem::Folder { target, .. } => Some(BarIdentity::Opens(target.clone())),
            SelectItem::Missing(missing) => self.missing_chart(missing).map(|entry| BarIdentity::Chart(entry.md5.trim().to_ascii_lowercase())),
            SelectItem::Course(course) => self.table_course(course).map(|course| BarIdentity::Course(course.hash())),
        }
    }

    /// The first row of the list whose bar is `identity`.
    pub(crate) fn row_of(&self, identity: &BarIdentity) -> Option<usize> {
        self.select_items.iter().position(|item| match (item, identity) {
            (SelectItem::Song(index), BarIdentity::Chart(md5)) => self.library.songs().get(*index).is_some_and(|entry| entry.md5.eq_ignore_ascii_case(md5)),
            (SelectItem::Missing(missing), BarIdentity::Chart(md5)) => {
                self.missing_chart(missing).is_some_and(|entry| entry.md5.trim().eq_ignore_ascii_case(md5))
            }
            (SelectItem::Folder { target, .. }, BarIdentity::Opens(folder)) => target == folder,
            (SelectItem::Course(course), BarIdentity::Course(hash)) => self.table_course(course).is_some_and(|course| &course.hash() == hash),
            (SelectItem::Song(_) | SelectItem::Missing(_), BarIdentity::Opens(_) | BarIdentity::Course(_))
            | (SelectItem::Folder { .. }, BarIdentity::Chart(_) | BarIdentity::Course(_))
            | (SelectItem::Course(_), BarIdentity::Chart(_) | BarIdentity::Opens(_)) => false,
        })
    }

    /// Whether the browser is at the top of the library, with no folder open.
    pub(crate) fn is_at_root(&self) -> bool {
        self.select_path.is_empty()
    }

    /// Whether the one folder open is the flat list of every chart, which is the list a search runs
    /// over.
    pub(crate) fn is_in_the_flat_list(&self) -> bool {
        matches!(self.select_path.as_slice(), [only] if only.folder == SelectFolder::AllSongs)
    }

    /// The titles of the folders the browser has opened, outermost first.
    pub(crate) fn open_titles(&self) -> Vec<String> {
        self.select_path.iter().map(|open| open.folder.title(self)).collect()
    }

    /// The table, by its place in the list now, and the level by its name, when the folder open
    /// innermost is a level of a table.
    pub(crate) fn innermost_level(&self) -> Option<(usize, String)> {
        let SelectFolder::TableLevel(table, level) = &self.select_path.last()?.folder else {
            return None;
        };
        Some((table.resolve(self)?, level.clone()))
    }
}

/// A place in the song list -- the folders it has opened and the bar its cursor is on -- kept as
/// something that can be found again after the tables, the folders or the library under it have
/// changed.
///
/// Every folder and bar is kept as what it is ([`SelectFolder`], [`BarIdentity`]): a fetch can add
/// a level before the one open or take one away, a table that arrives for the first time is a
/// folder put among the others in the order of the sources, a scan moves every chart after one it
/// found, and one of the player's own folders can be renamed on the site.
///
/// The place is the list's own while a table lands under it ([`crate::tablesrc::apply_finished_fetches`])
/// or a scan is taken in, and it is what the browser keeps of where a search was made from for as
/// long as the search is on: a table that arrives meanwhile must not move what the search goes back
/// to.
pub(crate) struct ListPlace {
    path: Vec<OpenedFolder>,
    /// The bar the cursor was on, or `None` over an empty list.
    focused: Option<BarIdentity>,
    sel: usize,
}

impl ListPlace {
    /// Where the song list is now.
    pub(crate) fn of(shared: &AppShared) -> ListPlace {
        ListPlace { path: shared.select_path.clone(), focused: shared.select_items.get(shared.sel).and_then(|item| shared.identity_of(item)), sel: shared.sel }
    }

    /// The path as the tables and folders stand now: the longest prefix whose every folder is still
    /// there, with each table at the place it is at now. A level the table no longer has leaves the
    /// browser in the table, and a table with no levels left in the folder it was shown from.
    pub(crate) fn path(&self, shared: &AppShared) -> Vec<OpenedFolder> {
        self.path
            .iter()
            .map_while(|open| {
                let folder = open.folder.now(shared).filter(|folder| folder.is_there(shared))?;
                Some(OpenedFolder { folder, came_from: open.came_from.clone() })
            })
            .collect()
    }

    /// The row of the bar the cursor was on, found by what it is; the row it was on when it is gone,
    /// held to the rows there are. The list must have been built over [`ListPlace::path`].
    pub(crate) fn row(&self, shared: &AppShared) -> usize {
        let found = self.focused.as_ref().and_then(|focused| shared.row_of(focused));
        found.unwrap_or(self.sel).min(shared.select_items.len().saturating_sub(1))
    }
}

/// Rebuild the song list where it stood after something under it changed, `place` being where it
/// stood before the change: looking into the same folders, with the cursor on the bar it was on.
/// The list is built through the filter it was last built with, so the cursor is found in the list
/// that is shown.
pub(crate) fn show_the_list_again(shared: &mut AppShared, place: &ListPlace) {
    shared.select_path = place.path(shared);
    shared.rebuild_select_items_with(shared.select_filter.with_favorites_of(&shared.config));
    shared.sel = place.row(shared);
}

#[cfg(test)]
impl AppShared {
    /// Open `folders`, outermost first, with no bar to come back to in any of them.
    pub(crate) fn open_for_tests(&mut self, folders: impl IntoIterator<Item = SelectFolder>) {
        self.select_path = folders.into_iter().map(OpenedFolder::plain).collect();
    }

    /// The folders the browser has open, outermost first.
    pub(crate) fn opened(&self) -> Vec<SelectFolder> {
        self.select_path.iter().map(|open| open.folder.clone()).collect()
    }

    /// The table at `at` of the list, as the folder of its levels.
    pub(crate) fn table_folder(&self, at: usize) -> SelectFolder {
        SelectFolder::Table(TableRef::of(self, at))
    }

    /// The level at `level` of the table at `at`, as the folder of its charts.
    pub(crate) fn level_folder(&self, at: usize, level: usize) -> SelectFolder {
        let name = self.table_levels.get(at).and_then(|levels| levels.get(level)).map(|(name, _)| name.clone()).unwrap_or_default();
        SelectFolder::TableLevel(TableRef::of(self, at), name)
    }

    /// The player's own folder at `at` of the list of them.
    pub(crate) fn user_folder(&self, at: usize) -> SelectFolder {
        SelectFolder::UserFolder(self.user_folders.shown.get(at).map(|folder| folder.id.clone()).unwrap_or_default())
    }
}

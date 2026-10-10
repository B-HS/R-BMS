//! How the browser's row list is built: which folder is open, what is filtered out of it, and what
//! order the charts inside come out in.
//!
//! This is the one place the list is assembled, so the search box, the sort key and the filters all
//! meet here rather than being applied in three different screens. The rows themselves live on
//! [`AppShared`] because every screen returns to the browser and the debug overlay reports them from
//! anywhere.
//!
//! Every ordering here matches the direction the reference implementation sorts in
//! (`BarSorter.java:19-257`), which is ascending on all of them — the un-cleared, the low-scoring
//! and the long-untouched come first, because those are the ones a player is looking for. A chart
//! with no record at all sorts last whatever the ordering, the same way the reference puts a null
//! score behind every real one.
#![allow(clippy::wildcard_imports)]

use std::cmp::Ordering;
use std::collections::HashMap;

use rbms_library::SongEntry;
use rbms_library::songdb::{
    CONTENT_BGA, CONTENT_TEXT, FEATURE_CHARGE_NOTE, FEATURE_HELL_CHARGE_NOTE, FEATURE_LONG_NOTE, FEATURE_RANDOM, FEATURE_STOP_SEQUENCE, FEATURE_UNDEFINED_LN,
    SongDb, SongRow,
};
use rbms_render::skin_render::frame::{LAMP_KINDS, RANK_KINDS};

use crate::skin_host::chart::{BpmRange, ChartContents, ChartMeta};
use crate::*;

/// Level a chart whose `#PLAYLEVEL` is not a number sorts under: after every chart that has one,
/// rather than at the top where an unparsed zero would have put it.
const UNKNOWN_LEVEL: i64 = i64::MAX;

/// What one chart's records say about it, folded once per list rebuild.
///
/// Every field is `None` for a chart with no record, which is what puts it last in the orderings
/// that read them. Folding this once rather than per comparison keeps a sort of a large library one
/// walk of the score book instead of one walk per comparison.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ChartRecords {
    /// Best clear lamp, under the score book's own rules about assisted and stale records.
    lamp: Option<u8>,
    /// Best score rate: EX over the EX ceiling of that run, which is what the reference compares
    /// (`BarSorter.java:159-162` divides the score by its note count).
    rate: Option<f32>,
    /// Fewest combo breaks of any run.
    min_breaks: Option<u32>,
    /// When the chart was last played.
    last_played: Option<i64>,
}

impl ChartRecords {
    /// Fold what the orderings and the clear filter need out of one chart's records.
    fn of(scores: &ScoreBook, md5: &str) -> ChartRecords {
        let records = scores.for_md5(md5);
        ChartRecords {
            lamp: scores.best_clear_for_md5(md5),
            rate: records.iter().filter(|r| r.max_ex > 0).map(|r| r.ex_score as f32 / r.max_ex as f32).max_by(|a, b| a.total_cmp(b)),
            min_breaks: records.iter().map(|r| r.counts[3] + r.counts[4] + r.counts[5]).min(),
            last_played: records.first().map(|r| r.played_at),
        }
    }

    /// Whether the chart has been played at all.
    fn played(&self) -> bool {
        self.last_played.is_some()
    }
}

/// One chart as the orderings and the filters see it: its header facts, the folded comparison keys
/// that would otherwise be recomputed on every comparison, and what its records say about it.
pub(crate) struct SortRow<'a> {
    /// Index into the library's song list, which is what the row list is made of.
    index: usize,
    entry: &'a SongEntry,
    /// Title and artist folded for comparison, so the order does not depend on capitalisation and
    /// a sort folds each string once rather than once per comparison.
    title_key: String,
    artist_key: String,
    level: i64,
    favorite: bool,
    records: ChartRecords,
}

impl<'a> SortRow<'a> {
    /// Fold one chart into what the list needs of it. `records` is left empty when nothing being
    /// applied reads it, so browsing a large library does not walk the score book for nothing.
    fn new(index: usize, entry: &'a SongEntry, records: ChartRecords, favorite: bool) -> SortRow<'a> {
        SortRow {
            index,
            title_key: entry.title.to_lowercase(),
            artist_key: entry.artist.to_lowercase(),
            level: entry.level.trim().parse::<i64>().unwrap_or(UNKNOWN_LEVEL),
            favorite,
            records,
            entry,
        }
    }

    /// Whether the search box's query matches this chart's title, artist or subtitle. An empty
    /// query matches everything.
    fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        self.title_key.contains(query) || self.artist_key.contains(query) || self.entry.subtitle.to_lowercase().contains(query)
    }
}

/// The reference's title ordering: case-insensitive title, and charts that share one ordered by
/// their difficulty slot (`BarSorter.java:30-39`). Every other ordering falls back to it, so the
/// order is total and a list does not reshuffle between two runs of the same sort.
fn by_title(a: &SortRow<'_>, b: &SortRow<'_>) -> Ordering {
    a.title_key.cmp(&b.title_key).then_with(|| a.entry.difficulty.cmp(&b.entry.difficulty))
}

/// Order two values where a chart with nothing recorded sorts last, whichever way round the
/// comparison would otherwise put it (`BarSorter.java:130-138` and every ordering after it return
/// the null side as greater).
fn present_first<T>(a: Option<T>, b: Option<T>, compare: impl FnOnce(T, T) -> Ordering) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => compare(a, b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Whether an ordering reads a chart's records, so a list sorted by title does not fold the score
/// book for every chart in it.
fn reads_records(mode: SortMode) -> bool {
    matches!(mode, SortMode::Clear | SortMode::Score | SortMode::MissCount | SortMode::LastUpdate | SortMode::RivalClear | SortMode::RivalScore)
}

/// Order two charts under one sort mode.
///
/// Three of the reference's twelve orderings need a fact this browser does not hold yet and fall
/// back to the title, which keeps choosing one stable rather than arbitrary: `LENGTH` wants the
/// chart's playing time and `DURATION` its average judge timing (`BarSorter.java:96,201`), neither
/// of which survives a header-only scan or is kept in a record, and the two rival comparisons want
/// a rival's records. `BPM` compares the chart's opening tempo rather than the reference's maximum,
/// which is the one the header states.
///
/// `LEVEL` breaks a tie on the difficulty slot before the title, which is what the reference does
/// (`BarSorter.java:117-119`): a level band reads as the chart's own difficulties in order rather
/// than alphabetically. The title is still the last word, so the order stays total.
pub(crate) fn sort_cmp(mode: SortMode, a: &SortRow<'_>, b: &SortRow<'_>) -> Ordering {
    match mode {
        SortMode::Default | SortMode::Title => by_title(a, b),
        SortMode::Artist => a.artist_key.cmp(&b.artist_key).then_with(|| by_title(a, b)),
        SortMode::Bpm => a.entry.init_bpm.total_cmp(&b.entry.init_bpm).then_with(|| by_title(a, b)),
        SortMode::Level => a.level.cmp(&b.level).then_with(|| a.entry.difficulty.cmp(&b.entry.difficulty)).then_with(|| by_title(a, b)),
        SortMode::Clear => present_first(a.records.lamp, b.records.lamp, |a, b| a.cmp(&b)).then_with(|| by_title(a, b)),
        SortMode::Score => present_first(a.records.rate, b.records.rate, |a, b| a.total_cmp(&b)).then_with(|| by_title(a, b)),
        SortMode::MissCount => present_first(a.records.min_breaks, b.records.min_breaks, |a, b| a.cmp(&b)).then_with(|| by_title(a, b)),
        SortMode::LastUpdate => present_first(a.records.last_played, b.records.last_played, |a, b| a.cmp(&b)).then_with(|| by_title(a, b)),
        SortMode::Length | SortMode::Duration | SortMode::RivalClear | SortMode::RivalScore => by_title(a, b),
    }
}

/// Which lamps a chart may carry to stay in the list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ClearFilter {
    #[default]
    Any,
    /// Charts nothing has been recorded on.
    Unplayed,
    /// Charts not yet cleared, which includes the ones nobody has played — the set a player
    /// grinding a folder is looking for.
    Unclear,
    /// Charts cleared at least once.
    Cleared,
    /// Charts taken to a full combo or better.
    FullCombo,
}

/// Every clear filter, in the order the filter panel steps through them.
pub(crate) const CLEAR_FILTERS: [ClearFilter; 5] =
    [ClearFilter::Any, ClearFilter::Unplayed, ClearFilter::Unclear, ClearFilter::Cleared, ClearFilter::FullCombo];

impl ClearFilter {
    /// Name the filter panel shows.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ClearFilter::Any => "ALL",
            ClearFilter::Unplayed => "NO PLAY",
            ClearFilter::Unclear => "NOT CLEARED",
            ClearFilter::Cleared => "CLEARED",
            ClearFilter::FullCombo => "FULL COMBO",
        }
    }

    /// Whether a chart's records let it through.
    fn keeps(self, records: &ChartRecords) -> bool {
        let lamp = records.lamp.unwrap_or_else(|| clear_type_id(ClearType::NoPlay));
        match self {
            ClearFilter::Any => true,
            ClearFilter::Unplayed => !records.played(),
            ClearFilter::Unclear => lamp < clear_type_id(ClearType::Easy),
            ClearFilter::Cleared => lamp >= clear_type_id(ClearType::Easy),
            ClearFilter::FullCombo => lamp >= clear_type_id(ClearType::FullCombo),
        }
    }
}

/// Which charts of an open folder the browser is showing.
///
/// A default filter keeps everything, so the browser without a filter panel open behaves exactly as
/// it did before there was one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SelectFilter {
    /// Lowest `#PLAYLEVEL` shown, or `None` for no lower bound. A chart that does not state a
    /// readable level is out as soon as either bound is set, since there is nothing to compare.
    pub(crate) level_from: Option<i32>,
    /// Highest `#PLAYLEVEL` shown, or `None` for no upper bound.
    pub(crate) level_to: Option<i32>,
    /// Only charts of this play mode, or `None` for every mode.
    pub(crate) mode: Option<Mode>,
    pub(crate) clear: ClearFilter,
    /// Only the charts that have been starred.
    pub(crate) favorite_only: bool,
}

impl SelectFilter {
    /// The filter the browser applies when nothing has opened the panel: whatever the settings file
    /// remembers, which is the favourites switch alone.
    pub(crate) fn from_config(config: &Config) -> SelectFilter {
        SelectFilter { favorite_only: config.library.favorite_only, ..SelectFilter::default() }
    }

    /// Whether anything is being filtered out at all.
    pub(crate) fn is_active(&self) -> bool {
        *self != SelectFilter::default()
    }

    /// Whether the clear axis is set, which is the only one that needs a chart's records.
    fn reads_records(&self) -> bool {
        self.clear != ClearFilter::Any
    }

    /// Whether this chart stays in the list.
    fn keeps(&self, row: &SortRow<'_>) -> bool {
        if self.favorite_only && !row.favorite {
            return false;
        }
        if let Some(mode) = self.mode
            && row.entry.mode != mode
        {
            return false;
        }
        if (self.level_from.is_some() || self.level_to.is_some()) && row.level == UNKNOWN_LEVEL {
            return false;
        }
        if self.level_from.is_some_and(|from| row.level < i64::from(from)) || self.level_to.is_some_and(|to| row.level > i64::from(to)) {
            return false;
        }
        self.clear.keeps(&row.records)
    }
}

impl AppShared {
    /// Recompute the visible select list for the current `select_view`, filtered by what the
    /// settings file remembers.
    ///
    /// This is what every screen other than the browser calls — a finished scan, a chart returning
    /// from play — so a filter the panel is applying on top of this is re-applied by the browser
    /// itself the next frame it is up.
    pub(crate) fn rebuild_select_items(&mut self) {
        self.rebuild_select_items_with(SelectFilter::from_config(&self.config));
    }

    /// Recompute the visible select list, keeping only the charts `filter` lets through.
    pub(crate) fn rebuild_select_items_with(&mut self, filter: SelectFilter) {
        let songs = self.library.songs();
        self.select_items = match self.select_view {
            SelectView::Root => {
                let shown = self.shown_count(0..songs.len(), filter);
                let mut items = vec![SelectItem::Folder { label: format!("ALL SONGS ({shown})"), target: SelectView::AllSongs }];
                for (ti, levels) in self.table_levels.iter().enumerate() {
                    if levels.is_empty() {
                        continue;
                    }
                    let total: usize = levels.iter().map(|(_, v)| self.shown_count(v.iter().copied(), filter)).sum();
                    let name = self.table_names.get(ti).cloned().unwrap_or_else(|| "TABLE".into());
                    items.push(SelectItem::Folder { label: format!("{name} ({total})"), target: SelectView::TableLevels(ti) });
                }
                items
            }
            SelectView::AllSongs => self.arrange_songs((0..songs.len()).collect(), filter).into_iter().map(SelectItem::Song).collect(),
            SelectView::TableLevels(ti) => self
                .table_levels
                .get(ti)
                .map(|levels| {
                    levels
                        .iter()
                        .enumerate()
                        .map(|(li, (level, songs))| SelectItem::Folder {
                            label: format!("LV {level} ({})", self.shown_count(songs.iter().copied(), filter)),
                            target: SelectView::TableLevel(ti, li),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            SelectView::TableLevel(ti, li) => self
                .table_levels
                .get(ti)
                .and_then(|levels| levels.get(li))
                .map(|(_, songs)| self.arrange_songs(songs.clone(), filter).into_iter().map(SelectItem::Song).collect())
                .unwrap_or_default(),
        };
        if self.sel >= self.select_items.len() {
            self.sel = self.select_items.len().saturating_sub(1);
        }
        self.select_gen = self.select_gen.wrapping_add(1);
    }

    /// Fold one chart into a comparable row, reading its records only when something being applied
    /// looks at them.
    fn sort_row(&self, index: usize, filter: SelectFilter) -> Option<SortRow<'_>> {
        let entry = self.library.songs().get(index)?;
        let wants_records = filter.reads_records() || reads_records(self.config.library.sort);
        let records = if wants_records { ChartRecords::of(&self.scores, &entry.md5) } else { ChartRecords::default() };
        Some(SortRow::new(index, entry, records, self.favorites.contains(&entry.md5)))
    }

    /// How many of these charts the search box and the filter leave, which is what a folder row
    /// counts — a folder that says how many charts it holds has to say how many are reachable.
    ///
    /// With nothing being taken out the answer is the whole folder, and saying so costs nothing:
    /// counting a difficulty table of ten thousand charts is otherwise a walk of all of them every
    /// time the root list is rebuilt.
    fn shown_count(&self, indices: impl ExactSizeIterator<Item = usize>, filter: SelectFilter) -> usize {
        if !filter.is_active() && self.search.trim().is_empty() {
            return indices.len();
        }
        let query = self.search.trim().to_lowercase();
        indices.filter_map(|i| self.sort_row(i, filter)).filter(|row| row.matches_query(&query) && filter.keeps(row)).count()
    }

    /// Keep the charts the search box and the filter allow, and order them by the active
    /// [`SortMode`]. The single place search, filter and sort are applied.
    fn arrange_songs(&self, indices: Vec<usize>, filter: SelectFilter) -> Vec<usize> {
        let query = self.search.trim().to_lowercase();
        let mut rows: Vec<SortRow<'_>> =
            indices.into_iter().filter_map(|i| self.sort_row(i, filter)).filter(|row| row.matches_query(&query) && filter.keeps(row)).collect();
        let sort = self.config.library.sort;
        if sort != SortMode::Default {
            rows.sort_by(|a, b| sort_cmp(sort, a, b));
        }
        rows.into_iter().map(|row| row.index).collect()
    }
}

/// The highest lamp a folder counts a chart under (`DirectoryBar.getLamps`, whose last slot is
/// `Max`).
const TOP_LAMP: usize = LAMP_KINDS - 1;

/// The highest rank a folder counts a chart under, which is also how many steps a full score is cut
/// into (`exscore * 27 / (notes * 2)`).
const TOP_RANK: usize = RANK_KINDS - 1;

/// What the song database holds about a chart that the library's entry leaves out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChartFact {
    /// The chart's `feature` bits (`SongData.getFeature`).
    pub(crate) features: u32,
    /// The chart's `content` bits (`SongData.getContent`).
    pub(crate) content: i32,
    /// When the chart first entered the database, in seconds (`SongData.getAdddate`).
    pub(crate) added_at: i64,
    /// `#BACKBMP` as the chart names it.
    pub(crate) backbmp: String,
    /// The chart's second artist line and its other hash (`SongData.getSubartist`, `getSha256`).
    pub(crate) subartist: String,
    pub(crate) sha256: String,
    /// The slowest and the fastest tempo the chart plays at, in whole BPM (`SongData.getMinbpm`,
    /// `getMaxbpm`).
    pub(crate) min_bpm: i32,
    pub(crate) max_bpm: i32,
    /// How long the chart plays, in milliseconds, and how many notes it has to judge
    /// (`SongData.getLength`, `getNotes`).
    pub(crate) length_ms: i64,
    pub(crate) notes: i32,
}

impl ChartFact {
    /// `chart` with what the song database holds about it: its tempo range, its length and note
    /// count, and which of its extras it has, each as the reference reads it off its `SongData`.
    ///
    /// Which pictures a chart names is left unsaid. The reference's options for them ask whether the
    /// chart that was loaded last brought a picture (`BMSResource.getStagefile`), not whether the
    /// one under the cursor names one, so this is not the place that knows.
    pub(crate) fn describe<'a>(&'a self, chart: ChartMeta<'a>) -> ChartMeta<'a> {
        let has_feature = |bits: i32| self.features & u32::try_from(bits).unwrap_or_default() != 0;
        let any_long_note = FEATURE_UNDEFINED_LN | FEATURE_LONG_NOTE | FEATURE_CHARGE_NOTE | FEATURE_HELL_CHARGE_NOTE;
        ChartMeta {
            subartist: &self.subartist,
            sha256: &self.sha256,
            bpm: Some(BpmRange { min: self.min_bpm, max: self.max_bpm }),
            length_ms: i32::try_from(self.length_ms).ok(),
            notes: Some(self.notes),
            contents: ChartContents {
                bga: Some(self.content & CONTENT_BGA != 0),
                text: Some(self.content & CONTENT_TEXT != 0),
                long_note: Some(has_feature(any_long_note)),
                random_sequence: Some(has_feature(FEATURE_RANDOM)),
                bpm_stop: Some(has_feature(FEATURE_STOP_SEQUENCE)),
                ..chart.contents
            },
            ..chart
        }
    }
}

/// The chart under the cursor as the chart cluster reads it: what the library's entry says, what the
/// song database adds when it was asked (`fact`), and what measuring the notes found once the cursor
/// has rested on it (`detail`).
///
/// The database is what the reference reads all of this from, so its answers are there on the frame
/// the cursor arrives. Without one, the tempo, the length and the note count wait for the
/// measurement, and until then read as not known.
///
/// The tempo most of the notes are played at is something neither holds (`SongInformation.mainbpm`).
/// For a chart that keeps one tempo throughout it is that tempo and is said; for a chart that
/// changes tempo it is left not known rather than guessed.
pub(crate) fn chart_under_cursor<'a>(entry: &'a SongEntry, fact: Option<&'a ChartFact>, detail: Option<&ChartDetail>) -> ChartMeta<'a> {
    let named = ChartMeta::of_entry(entry);
    let named = ChartMeta { contents: ChartContents { stagefile: None, banner: None, ..named.contents }, ..named };
    let described = match fact {
        Some(fact) => fact.describe(named),
        None => named,
    };
    let measured = match detail {
        Some(detail) => described.with_detail(detail),
        None => described,
    };
    ChartMeta { main_bpm: measured.bpm.filter(|bpm| bpm.min == bpm.max).map(|bpm| f64::from(bpm.min)), ..measured }
}

/// The [`ChartFact`]s of one library, by a chart's place in it.
///
/// The library's entries are what the built-in browser draws a row from, and they leave these out.
/// A wheel a skin draws labels a chart by what it is made of and by how new it is, so the browser
/// reads them from the database once for each library it is handed, and only when a skin asks: a
/// browser no skin draws holds [`ChartFacts::unread`] and never touches the database for them.
#[derive(Debug, Default)]
pub(crate) struct ChartFacts {
    /// Which library these belong to: where it keeps its charts and how many it has. A library is
    /// replaced whole, with the old one still held while the new one is built, so the two never
    /// answer alike.
    library: Option<(usize, usize)>,
    /// Whether the database was asked.
    read: bool,
    facts: Vec<Option<ChartFact>>,
}

impl ChartFacts {
    /// What tells one library from the next.
    fn stamp(library: &Library) -> (usize, usize) {
        let songs = library.songs();
        (songs.as_ptr().addr(), songs.len())
    }

    /// Whether these are the facts a browser of `library` needs: they belong to it, and they were
    /// read from the database when `wanted` says something is going to look at them.
    pub(crate) fn serve(&self, library: &Library, wanted: bool) -> bool {
        self.library == Some(Self::stamp(library)) && (self.read || !wanted)
    }

    /// The facts of a library nothing has asked the database about: none for any chart.
    pub(crate) fn unread(library: &Library) -> ChartFacts {
        ChartFacts { library: Some(Self::stamp(library)), read: false, facts: Vec::new() }
    }

    /// Read the facts of every chart of `library` out of `db`. A chart the database does not hold,
    /// and every chart when there is no database, has none.
    pub(crate) fn read(db: Option<&SongDb>, library: &Library) -> ChartFacts {
        let rows = db.and_then(|db| db.all_songs().ok()).unwrap_or_default();
        let by_path: HashMap<&str, &SongRow> = rows.iter().map(|row| (row.path.as_str(), row)).collect();
        let facts = library
            .songs()
            .iter()
            .map(|entry| {
                by_path.get(entry.path.to_string_lossy().as_ref()).map(|row| ChartFact {
                    features: u32::try_from(row.feature).unwrap_or_default(),
                    content: row.content,
                    added_at: row.adddate,
                    backbmp: row.backbmp.clone(),
                    subartist: row.subartist.clone(),
                    sha256: row.sha256.clone(),
                    min_bpm: row.min_bpm,
                    max_bpm: row.max_bpm,
                    length_ms: row.length_ms,
                    notes: row.notes,
                })
            })
            .collect();
        ChartFacts { library: Some(Self::stamp(library)), read: true, facts }
    }

    /// What the database holds about the chart at `index` of the library.
    pub(crate) fn of(&self, index: usize) -> Option<&ChartFact> {
        self.facts.get(index)?.as_ref()
    }
}

/// The file a chart's header names for one of its pictures, beside the chart, or `None` when the
/// header names none.
fn picture_beside(chart: &Path, name: &str) -> Option<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some(chart.parent()?.join(name))
}

/// The pictures one chart names: the two its library entry carries and the one only the database
/// remembers.
pub(crate) fn chart_images(entry: &SongEntry, fact: Option<&ChartFact>) -> BarImages {
    BarImages {
        stagefile: picture_beside(&entry.path, &entry.stagefile),
        banner: picture_beside(&entry.path, &entry.banner),
        backbmp: fact.and_then(|fact| picture_beside(&entry.path, &fact.backbmp)),
    }
}

/// The best score recorded on a chart: the run with the highest EX, and among runs that tie the
/// oldest, which is the run the built-in row has always ranked a chart by.
fn best_score(scores: &ScoreBook, md5: &str) -> Option<BarScore> {
    scores.for_md5(md5).into_iter().max_by_key(|record| record.ex_score).map(|record| BarScore { ex: record.ex_score, max_ex: record.max_ex })
}

/// Which of the reference's bars a row that opens `target` stands for: a folder of charts for the
/// flat list, and a table for a difficulty table and for a level of one.
fn folder_kind(target: SelectView) -> BarKind {
    match target {
        SelectView::Root | SelectView::AllSongs => BarKind::Folder,
        SelectView::TableLevels(_) | SelectView::TableLevel(..) => BarKind::Table,
    }
}

/// The lowest lamp any chart of a folder sits on, or `None` for a folder with no charts counted
/// (`DirectoryBar.getLamp`).
fn folder_lamp(distribution: &BarDistribution) -> Option<u8> {
    distribution.lamps.iter().position(|count| *count > 0).and_then(|lamp| u8::try_from(lamp).ok())
}

impl AppShared {
    /// The list on show as bars: one for every row of [`AppShared::select_items`], in order.
    ///
    /// `facts` is what the song database adds to a chart, and `mode` is the play mode the browser is
    /// filtered to, which is the one filter the reference counts a folder's clears under
    /// (`DirectoryBar.updateFolderStatus`).
    ///
    /// `skinned` says whether a skin is going to read the bars. Only a skin reads how the charts
    /// under a folder are cleared and which pictures a chart names, and the first of those is a walk
    /// of every chart under the folder, so a browser no skin draws is not made to count it.
    pub(crate) fn select_bars(&self, facts: &ChartFacts, mode: Option<Mode>, skinned: bool) -> Vec<SelectBar> {
        self.select_items
            .iter()
            .map(|item| match item {
                SelectItem::Song(index) => self.chart_bar(*index, facts, skinned),
                SelectItem::Folder { label, target } if skinned => self.folder_bar(label, *target, mode),
                SelectItem::Folder { label, target } => SelectBar::new(folder_kind(*target), label.as_str()),
            })
            .collect()
    }

    /// The bar of the chart at `index` of the library (`SongBar`). An index the library does not
    /// hold is a chart that is not on disk, with nothing known about it.
    fn chart_bar(&self, index: usize, facts: &ChartFacts, skinned: bool) -> SelectBar {
        let Some(entry) = self.library.songs().get(index) else {
            return SelectBar::new(BarKind::Song { exists: false }, String::new());
        };
        let fact = facts.of(index);
        SelectBar {
            chart: Some(BarChart {
                index,
                subtitle: entry.subtitle.clone(),
                mode: entry.mode,
                level_text: entry.level.clone(),
                level: entry.level.trim().parse().unwrap_or_default(),
                difficulty: entry.difficulty,
                features: fact.map_or(0, |fact| fact.features),
                added_at: fact.map(|fact| fact.added_at),
                favorite: self.favorites.contains(&entry.md5),
                best: best_score(&self.scores, &entry.md5),
                images: if skinned { chart_images(entry, fact) } else { BarImages::default() },
            }),
            lamp: self.scores.best_clear_for_md5(&entry.md5),
            ..SelectBar::new(BarKind::Song { exists: true }, entry.title.as_str())
        }
    }

    /// The back image of the chart that was loaded last ([`AppShared::chart_path`]), when the library
    /// holds that chart and the song database remembers the image. Nothing for a chart outside the
    /// library, and nothing before any chart was loaded.
    pub(crate) fn loaded_chart_backbmp(&self, facts: &ChartFacts) -> Option<PathBuf> {
        if self.chart_path.is_empty() {
            return None;
        }
        let loaded = Path::new(&self.chart_path);
        let (index, entry) = self.library.songs().iter().enumerate().find(|(_, entry)| entry.path == loaded)?;
        chart_images(entry, facts.of(index)).backbmp
    }

    /// The bar of a row that opens another list.
    ///
    /// The flat list of every chart stands for the reference's folder of charts (`FolderBar`), a
    /// difficulty table for its `TableBar` and a level of one for the `HashBar` under it. The
    /// reference counts the clears under the first and the last and not under a table itself, whose
    /// bars are levels rather than charts.
    fn folder_bar(&self, label: &str, target: SelectView, mode: Option<Mode>) -> SelectBar {
        let distribution = match target {
            SelectView::Root | SelectView::TableLevels(_) => None,
            SelectView::AllSongs => Some(self.distribution(0..self.library.len(), mode)),
            SelectView::TableLevel(table, level) => {
                let charts = self.table_levels.get(table).and_then(|levels| levels.get(level)).map_or(&[][..], |(_, charts)| charts.as_slice());
                Some(self.distribution(charts.iter().copied(), mode))
            }
        };
        SelectBar { lamp: distribution.as_ref().and_then(folder_lamp), distribution, ..SelectBar::new(folder_kind(target), label) }
    }

    /// How the charts at `indices` of the library have been cleared and scored
    /// (`DirectoryBar.updateFolderStatus`): each chart of the mode on show is counted once under the
    /// best lamp it holds and once under the rank of its best score, and a chart with nothing
    /// recorded is counted under no play and under the lowest rank.
    fn distribution(&self, indices: impl Iterator<Item = usize>, mode: Option<Mode>) -> BarDistribution {
        let mut counted = BarDistribution::default();
        for entry in indices.filter_map(|index| self.library.songs().get(index)) {
            if mode.is_some_and(|mode| entry.mode != mode) {
                continue;
            }
            let (lamp, rank) = match best_score(&self.scores, &entry.md5) {
                Some(best) => {
                    let lamp = usize::from(self.scores.best_clear_for_md5(&entry.md5).unwrap_or_default()).min(TOP_LAMP);
                    let rank = if best.max_ex == 0 { 0 } else { (best.ex as usize * TOP_RANK / best.max_ex as usize).min(TOP_RANK) };
                    (lamp, rank)
                }
                None => (0, 0),
            };
            counted.lamps[lamp] += 1;
            counted.ranks[rank] += 1;
        }
        counted
    }
}

/// The course list as bars: one for every course, in order (`GradeBar`).
///
/// A course every chart of which the library holds carries the features of all of them. Its own
/// clear and medal are not kept anywhere in this build, so it has neither.
pub(crate) fn course_bars(courses: &CourseList, library: &Library, facts: &ChartFacts) -> Vec<SelectBar> {
    courses
        .entries()
        .iter()
        .zip(courses.rows())
        .map(|(entry, row)| {
            let complete = entry.is_playable();
            let features = if complete {
                entry
                    .course
                    .charts
                    .iter()
                    .filter_map(|chart| library_index(library, chart))
                    .filter_map(|index| facts.of(index))
                    .fold(0, |all, fact| all | fact.features)
            } else {
                0
            };
            SelectBar {
                course: Some(BarCourse { stages: entry.course.stage_count(), badges: row.badges, features }),
                ..SelectBar::new(BarKind::Course { complete }, row.title)
            }
        })
        .collect()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(crate) fn entry(title: &str, artist: &str, level: &str) -> SongEntry {
        SongEntry {
            path: PathBuf::from(format!("{title}.bms")),
            title: title.to_string(),
            subtitle: String::new(),
            artist: artist.to_string(),
            genre: String::new(),
            maker: String::new(),
            level: level.to_string(),
            difficulty: 0,
            init_bpm: 120.0,
            rank: 2,
            total: 260.0,
            mode: Mode::BEAT_7K,
            md5: format!("md5-{title}"),
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
        }
    }

    pub(crate) fn record(md5: &str, clear: u8, ex: u32, breaks: u32, played_at: i64) -> ScoreRecord {
        ScoreRecord {
            md5: md5.to_string(),
            title: String::new(),
            mode: Mode::BEAT_7K.name.to_string(),
            clear,
            ex_score: ex,
            max_ex: 1000,
            counts: [0, 0, 0, breaks, 0, 0],
            empty_poor: 0,
            max_combo: 0,
            total_notes: 500,
            gauge: String::new(),
            gauge_value: 100.0,
            random: String::new(),
            played_at,
            replay_file: None,
            rule_version: rbms_store::SCORE_RULE_VERSION,
            ln_mode: SCORE_LN_MODE_FROM_CHART.to_string(),
            assisted: false,
        }
    }

    /// A row with nothing recorded on it, which is what the orderings that do not read records see.
    fn row<'a>(entry: &'a SongEntry) -> SortRow<'a> {
        SortRow::new(0, entry, ChartRecords::default(), false)
    }

    fn played_row<'a>(entry: &'a SongEntry, scores: &ScoreBook) -> SortRow<'a> {
        SortRow::new(0, entry, ChartRecords::of(scores, &entry.md5), false)
    }

    fn book(records: Vec<ScoreRecord>) -> ScoreBook {
        ScoreBook::from_records(records)
    }

    /// Sorting a list twice has to give the same answer, whichever mode is chosen, or the browser
    /// would reshuffle under the cursor.
    #[test]
    fn every_ordering_is_total_and_falls_back_to_the_title() {
        let alpha = entry("alpha", "zz", "5");
        let beta = entry("beta", "zz", "5");
        for mode in SortMode::ALL {
            assert_eq!(sort_cmp(mode, &row(&alpha), &row(&beta)), Ordering::Less, "{mode:?} does not break its tie on the title");
            assert_eq!(sort_cmp(mode, &row(&beta), &row(&alpha)), Ordering::Greater, "{mode:?} is not symmetric");
            assert_eq!(sort_cmp(mode, &row(&alpha), &row(&alpha)), Ordering::Equal, "{mode:?} does not order a chart against itself");
        }
    }

    #[test]
    fn the_title_and_artist_orderings_ignore_capitalisation() {
        let upper = entry("APPLE", "ZEBRA", "1");
        let lower = entry("banana", "apple", "1");
        assert_eq!(sort_cmp(SortMode::Title, &row(&upper), &row(&lower)), Ordering::Less);
        assert_eq!(sort_cmp(SortMode::Artist, &row(&lower), &row(&upper)), Ordering::Less);
    }

    /// Two charts under one title are the same song at two difficulties, so the difficulty slot is
    /// what separates them (`BarSorter.java:36`).
    #[test]
    fn charts_that_share_a_title_are_ordered_by_their_difficulty_slot() {
        let mut hyper = entry("song", "a", "8");
        hyper.difficulty = 3;
        let mut another = entry("song", "a", "11");
        another.difficulty = 4;
        assert_eq!(sort_cmp(SortMode::Title, &row(&hyper), &row(&another)), Ordering::Less);
        assert_eq!(sort_cmp(SortMode::Level, &row(&hyper), &row(&another)), Ordering::Less, "a level ordering breaks its own ties the same way");
    }

    /// Charts on one level are the difficulties of that band, so the reference orders them by the
    /// difficulty slot before it ever looks at the title (`BarSorter.java:117-119`).
    #[test]
    fn charts_on_one_level_are_ordered_by_difficulty_before_title() {
        let mut another = entry("another", "a", "12");
        another.difficulty = 4;
        let mut hyper = entry("hyper", "a", "12");
        hyper.difficulty = 3;
        assert_eq!(sort_cmp(SortMode::Level, &row(&hyper), &row(&another)), Ordering::Less, "the lower difficulty slot comes first");
        assert_eq!(sort_cmp(SortMode::Title, &row(&another), &row(&hyper)), Ordering::Less, "a title ordering is unaffected");

        let mut first = entry("zzz", "a", "12");
        first.difficulty = 3;
        let mut second = entry("aaa", "a", "12");
        second.difficulty = 3;
        assert_eq!(sort_cmp(SortMode::Level, &row(&second), &row(&first)), Ordering::Less, "the title is still the last word");
    }

    /// A chart that does not state a playable level sorts after the ones that do, rather than at
    /// the top where an unparsed zero would have put it.
    #[test]
    fn a_chart_with_no_readable_level_sorts_after_the_ones_that_have_one() {
        let twelve = entry("a", "a", "12");
        let unknown = entry("b", "a", "???");
        assert_eq!(row(&unknown).level, UNKNOWN_LEVEL);
        assert_eq!(sort_cmp(SortMode::Level, &row(&twelve), &row(&unknown)), Ordering::Less);
        assert_eq!(sort_cmp(SortMode::Level, &row(&entry("c", "a", "2")), &row(&twelve)), Ordering::Less, "levels compare as numbers, not as text");
    }

    /// The reference sorts on the tempo the chart states (`BarSorter.java:78`), slowest first.
    #[test]
    fn the_tempo_ordering_puts_the_slowest_chart_first() {
        let mut slow = entry("zzz", "a", "5");
        slow.init_bpm = 90.0;
        let mut fast = entry("aaa", "a", "5");
        fast.init_bpm = 220.0;
        assert_eq!(sort_cmp(SortMode::Bpm, &row(&slow), &row(&fast)), Ordering::Less);
        assert_eq!(sort_cmp(SortMode::Bpm, &row(&fast), &row(&slow)), Ordering::Greater);
    }

    /// The reference's record orderings are all ascending with a chart that has no record last
    /// (`BarSorter.java:126-220`): the worst lamp, the lowest rate, the fewest misses and the
    /// longest-untouched come first, which is the order a player grinding a folder wants.
    #[test]
    fn the_record_orderings_are_ascending_and_put_a_chart_with_no_record_last() {
        let low = entry("low", "a", "5");
        let high = entry("high", "a", "5");
        let unplayed = entry("unplayed", "a", "5");
        let scores = book(vec![record(&low.md5, 4, 400, 2, 1_000), record(&high.md5, 7, 900, 30, 9_000)]);
        let low = played_row(&low, &scores);
        let high = played_row(&high, &scores);
        let unplayed = played_row(&unplayed, &scores);
        for mode in [SortMode::Clear, SortMode::Score, SortMode::MissCount, SortMode::LastUpdate] {
            assert_eq!(sort_cmp(mode, &low, &high), Ordering::Less, "{mode:?} is not ascending");
            assert_eq!(sort_cmp(mode, &low, &unplayed), Ordering::Less, "{mode:?} put a chart with no record before one with a record");
            assert_eq!(sort_cmp(mode, &unplayed, &high), Ordering::Greater, "{mode:?}");
        }
    }

    #[test]
    fn the_miss_count_ordering_takes_the_fewest_breaks_of_the_runs_there_are() {
        let clean = entry("clean", "a", "5");
        let messy = entry("messy", "a", "5");
        let scores = book(vec![record(&clean.md5, 5, 900, 30, 1_000), record(&clean.md5, 5, 950, 2, 2_000), record(&messy.md5, 5, 900, 9, 1_000)]);
        assert_eq!(
            sort_cmp(SortMode::MissCount, &played_row(&clean, &scores), &played_row(&messy, &scores)),
            Ordering::Less,
            "the best run of a chart is what it is judged on"
        );
    }

    #[test]
    fn the_last_played_ordering_takes_the_newest_run_of_a_chart() {
        let old = entry("old", "a", "5");
        let new = entry("new", "a", "5");
        let scores = book(vec![record(&old.md5, 5, 900, 3, 1_000), record(&new.md5, 5, 900, 3, 5_000), record(&old.md5, 5, 900, 3, 2_000)]);
        assert_eq!(sort_cmp(SortMode::LastUpdate, &played_row(&old, &scores), &played_row(&new, &scores)), Ordering::Less, "oldest first");
    }

    /// The score ordering compares the rate rather than the raw EX, so a short chart scored well
    /// beats a long chart scored badly (`BarSorter.java:159-162`).
    #[test]
    fn the_score_ordering_compares_the_rate_rather_than_the_raw_score() {
        let short_good = entry("aaa", "a", "5");
        let long_bad = entry("zzz", "a", "5");
        let mut short_record = record(&short_good.md5, 5, 180, 0, 1_000);
        short_record.max_ex = 200;
        let mut long_record = record(&long_bad.md5, 5, 900, 0, 1_000);
        long_record.max_ex = 2_000;
        let scores = book(vec![short_record, long_record]);
        assert_eq!(sort_cmp(SortMode::Score, &played_row(&long_bad, &scores), &played_row(&short_good, &scores)), Ordering::Less, "0.45 before 0.90");
    }

    /// The orderings that need a fact the browser does not hold are stable rather than arbitrary:
    /// they fall back to the title, which is what this pins.
    #[test]
    fn the_orderings_that_need_a_fact_the_browser_lacks_fall_back_to_the_title() {
        let last = entry("zzz", "a", "5");
        let first = entry("aaa", "a", "5");
        for mode in [SortMode::Length, SortMode::Duration, SortMode::RivalClear, SortMode::RivalScore] {
            assert_eq!(sort_cmp(mode, &row(&first), &row(&last)), Ordering::Less, "{mode:?}");
        }
    }

    /// Reading the score book is skipped for the orderings that never look at it, which is what
    /// keeps browsing a large library off the record list.
    #[test]
    fn only_the_record_orderings_ask_for_a_charts_records() {
        for mode in [SortMode::Clear, SortMode::Score, SortMode::MissCount, SortMode::LastUpdate, SortMode::RivalClear, SortMode::RivalScore] {
            assert!(reads_records(mode), "{mode:?} compares records but would be given none");
        }
        for mode in [SortMode::Default, SortMode::Title, SortMode::Artist, SortMode::Bpm, SortMode::Length, SortMode::Level, SortMode::Duration] {
            assert!(!reads_records(mode), "{mode:?} reads the score book for nothing");
        }
    }

    #[test]
    fn a_default_filter_keeps_every_chart() {
        let filter = SelectFilter::default();
        assert!(!filter.is_active());
        assert!(filter.keeps(&row(&entry("a", "a", "12"))));
        assert!(filter.keeps(&row(&entry("b", "b", "???"))), "a chart with no readable level is not filtered out by an unset level bound");
    }

    #[test]
    fn the_level_bounds_keep_only_the_charts_inside_them() {
        let filter = SelectFilter { level_from: Some(10), level_to: Some(12), ..SelectFilter::default() };
        assert!(filter.is_active());
        assert!(!filter.keeps(&row(&entry("a", "a", "9"))));
        assert!(filter.keeps(&row(&entry("b", "a", "10"))));
        assert!(filter.keeps(&row(&entry("c", "a", "12"))));
        assert!(!filter.keeps(&row(&entry("d", "a", "13"))));
        assert!(!filter.keeps(&row(&entry("e", "a", "???"))), "a chart with no readable level cannot be inside a level range");
    }

    #[test]
    fn one_level_bound_leaves_the_other_side_open() {
        let from = SelectFilter { level_from: Some(11), ..SelectFilter::default() };
        assert!(!from.keeps(&row(&entry("a", "a", "10"))));
        assert!(from.keeps(&row(&entry("b", "a", "99"))));
        let to = SelectFilter { level_to: Some(4), ..SelectFilter::default() };
        assert!(to.keeps(&row(&entry("c", "a", "1"))));
        assert!(!to.keeps(&row(&entry("d", "a", "5"))));
    }

    #[test]
    fn the_mode_filter_keeps_only_charts_of_that_mode() {
        let filter = SelectFilter { mode: Some(Mode::BEAT_5K), ..SelectFilter::default() };
        let seven = entry("a", "a", "5");
        let mut five = entry("b", "a", "5");
        five.mode = Mode::BEAT_5K;
        assert!(!filter.keeps(&row(&seven)));
        assert!(filter.keeps(&row(&five)));
    }

    #[test]
    fn the_favourites_filter_keeps_only_the_starred_charts() {
        let filter = SelectFilter { favorite_only: true, ..SelectFilter::default() };
        let starred = entry("a", "a", "5");
        assert!(!filter.keeps(&SortRow::new(0, &starred, ChartRecords::default(), false)));
        assert!(filter.keeps(&SortRow::new(0, &starred, ChartRecords::default(), true)));
    }

    #[test]
    fn the_clear_filter_splits_the_library_by_lamp() {
        let unplayed = entry("unplayed", "a", "5");
        let failed = entry("failed", "a", "5");
        let cleared = entry("cleared", "a", "5");
        let combo = entry("combo", "a", "5");
        let scores = book(vec![
            record(&failed.md5, clear_type_id(ClearType::Failed), 100, 40, 1_000),
            record(&cleared.md5, clear_type_id(ClearType::Normal), 700, 8, 1_000),
            record(&combo.md5, clear_type_id(ClearType::FullCombo), 900, 0, 1_000),
        ]);
        let rows = [
            (ClearFilter::Unplayed, vec!["unplayed"]),
            (ClearFilter::Unclear, vec!["unplayed", "failed"]),
            (ClearFilter::Cleared, vec!["cleared", "combo"]),
            (ClearFilter::FullCombo, vec!["combo"]),
            (ClearFilter::Any, vec!["unplayed", "failed", "cleared", "combo"]),
        ];
        for (clear, expected) in rows {
            let filter = SelectFilter { clear, ..SelectFilter::default() };
            let kept: Vec<&str> =
                [&unplayed, &failed, &cleared, &combo].into_iter().filter(|e| filter.keeps(&played_row(e, &scores))).map(|e| e.title.as_str()).collect();
            assert_eq!(kept, expected, "{clear:?}");
        }
    }

    #[test]
    fn every_clear_filter_has_its_own_name() {
        let mut labels: Vec<&str> = CLEAR_FILTERS.iter().map(|f| f.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), CLEAR_FILTERS.len(), "two clear filters share a name");
        assert!(CLEAR_FILTERS.contains(&ClearFilter::default()));
    }

    #[test]
    fn a_query_matches_the_title_the_artist_or_the_subtitle() {
        let mut chart = entry("Sakura", "composer", "9");
        chart.subtitle = "[ANOTHER]".to_string();
        let chart = row(&chart);
        assert!(chart.matches_query(""));
        assert!(chart.matches_query("saku"), "the title is matched case-insensitively");
        assert!(chart.matches_query("compos"));
        assert!(chart.matches_query("[another]"));
        assert!(!chart.matches_query("nothing"));
    }
}

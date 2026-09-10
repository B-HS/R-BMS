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

use rbms_library::SongEntry;

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
            rule_version: SCORE_RULE_VERSION,
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

//! Song-select IR ranking: fetching one chart's leaderboard off the frame thread, and the bounded
//! cache that keeps a moving selection cursor from firing a request per row.
//!
//! Everything here is data. The panel that draws it lives in [`crate::ir_ranking_view`].

use std::time::{Duration, Instant};

use rbms_ir::dto::DisplayedDan;
use rbms_ir::{ChartId, ChartRankingQuery, ClearLamp, DEFAULT_PAGE, IrError, PlayerId, ScoreRecord, ScoreServer};
use rbms_judge::{AssistGrade, ClearType, clear_type_id};
use rbms_render::Color;
use rbms_store::RecordLamp;

use crate::format::lamp_label_color;
use crate::ir_outcome::short_error;

/// Leaderboard rows asked for in one request. A hundred covers the panel and the ranking mode of
/// most charts in a single answer.
pub(crate) const RANKING_PAGE_LIMIT: u32 = 100;

/// Charts kept in the ranking cache. Large enough to cover scrolling a folder back and forth,
/// small enough that the whole cache is a few kilobytes.
pub(crate) const RANKING_CACHE_CAPACITY: usize = 64;

/// Rivals whose best is fetched for the panel. Each one costs a request, so the list is capped.
pub(crate) const RANKING_MAX_RIVALS: usize = 8;

/// How long a fetched board is served from the cache before the chart is asked for again: ten
/// minutes, as the reference implementation reloads a ranking.
pub(crate) const RANKING_READY_LIFETIME: Duration = Duration::from_secs(600);

/// How long a failed fetch is remembered before the chart is asked for again.
pub(crate) const RANKING_FAILED_RETRY_AFTER: Duration = Duration::from_secs(30);

/// One rendered leaderboard entry: a ranking row, the local player's best, or a rival's best.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RankingRow {
    pub(crate) rank: Option<u32>,
    pub(crate) player: String,
    pub(crate) ex_score: u32,
    /// The row's lamp by id, and the assist grade inside it, which is what the ranking's spread of
    /// lamps counts by.
    pub(crate) record_lamp: RecordLamp,
    /// The lamp as it is named and coloured, its assist grade included (ASSIST EASY to ASSIST
    /// EXHARD for a graded light-assist clear).
    pub(crate) lamp: &'static str,
    pub(crate) lamp_color: Color,
    pub(crate) fast_slow: Option<(u32, u32)>,
    /// The replay linked to the score this row shows, as the server's own row names it.
    pub(crate) replay_id: Option<String>,
    /// When the run was played, in milliseconds since the epoch, as the run itself reported it.
    pub(crate) played_at: i64,
    /// The id the server knows the player by, which `player` falls back to when the name is blank.
    pub(crate) player_id: String,
    /// The dans the player shows beside the name, none from a server that does not send them.
    pub(crate) dans: Vec<DisplayedDan>,
    /// Notes the chart had when the run was scored, 0 when the server did not say.
    pub(crate) total_notes: u32,
    /// Whether this row is the local player's.
    pub(crate) is_you: bool,
    /// Whether this row is a rival's.
    pub(crate) is_rival: bool,
    /// Whether SCRATCH AUTO played the run.
    pub(crate) scratch_auto: bool,
}

/// A page of the leaderboard that is not the one after the pages a board holds: the page the local
/// player's row is on, fetched on its own.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RankingPage {
    /// 1-based page number, [`RANKING_PAGE_LIMIT`] rows to a page.
    pub(crate) page: u32,
    pub(crate) rows: Vec<RankingRow>,
}

/// One chart's panel content: the leaderboard, the local player's row, and one row per rival
/// (`None` when that rival has not played the chart).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RankingBoard {
    /// The leaderboard from the top, `pages` pages of it one after the other.
    pub(crate) rows: Vec<RankingRow>,
    pub(crate) you: Option<RankingRow>,
    pub(crate) rivals: Vec<(String, Option<RankingRow>)>,
    /// How many pages `rows` holds.
    pub(crate) pages: u32,
    /// Whether the server can be asked for a page after those: the last page came back full and the
    /// server pages at all. Ask with [`crate::AppShared::start_ranking_page`] for page `pages + 1`.
    pub(crate) more: bool,
    /// The page the local player's row is on when `rows` does not reach it, so the row can be shown
    /// among its neighbours without fetching everything above it.
    pub(crate) your_page: Option<RankingPage>,
}

impl RankingBoard {
    /// The page the local player's row is on, while that page is neither held in `rows` nor held
    /// apart in `your_page` and the server has it. A tie above the row can leave the row on the page
    /// after the one its rank points at.
    pub(crate) fn page_to_find_you(&self) -> Option<u32> {
        let rank = self.you.as_ref()?.rank?;
        let held = u32::try_from(self.rows.len()).unwrap_or(u32::MAX);
        (self.more && self.your_page.is_none() && rank > held).then(|| (rank - 1) / RANKING_PAGE_LIMIT + DEFAULT_PAGE)
    }

    /// Whether asking for `page` would add rows the board does not hold yet.
    pub(crate) fn wants_page(&self, page: u32) -> bool {
        self.more && page > self.pages && self.your_page.as_ref().is_none_or(|held| held.page != page)
    }
}

/// What the panel knows about a chart right now.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RankingState {
    Loading,
    Ready(Box<RankingBoard>),
    Failed(String),
}

/// A finished background fetch, tagged with the generation it was started in so a result for a row
/// the user already scrolled past can be dropped.
pub(crate) struct RankingFetch {
    pub(crate) generation: u64,
    pub(crate) md5: String,
    pub(crate) state: RankingState,
    /// Whether `state` is the chart's cached board with one more page in it, which is then stored
    /// without starting its lifetime over.
    pub(crate) extends: bool,
}

/// A chart's cached state and when it was stored.
struct RankingEntry {
    md5: String,
    state: RankingState,
    stored_at: Instant,
}

/// Bounded most-recently-used cache of per-chart panel content, keyed by chart md5.
///
/// Ordered oldest-first; a hit or an insert moves the chart to the end, and the oldest entry is
/// evicted once the capacity is exceeded. A finished answer is served for a while only: a board for
/// [`RANKING_READY_LIFETIME`] and a failure for [`RANKING_FAILED_RETRY_AFTER`].
pub(crate) struct RankingCache {
    capacity: usize,
    entries: Vec<RankingEntry>,
    awaiting_score_of: Option<String>,
}

impl RankingCache {
    pub(crate) fn new(capacity: usize) -> RankingCache {
        RankingCache { capacity: capacity.max(1), entries: Vec::new(), awaiting_score_of: None }
    }

    /// Read an entry without changing the eviction order.
    pub(crate) fn peek(&self, md5: &str) -> Option<&RankingState> {
        self.entries.iter().find(|entry| entry.md5 == md5).map(|entry| &entry.state)
    }

    /// Mark a chart as most recently used. Returns whether it was cached at all.
    pub(crate) fn touch(&mut self, md5: &str) -> bool {
        match self.entries.iter().position(|entry| entry.md5 == md5) {
            Some(index) => {
                let entry = self.entries.remove(index);
                self.entries.push(entry);
                true
            }
            None => false,
        }
    }

    /// Store (or replace) a chart's state as the most recently used, evicting the oldest charts
    /// once the capacity is exceeded.
    pub(crate) fn insert(&mut self, md5: String, state: RankingState) {
        self.insert_at(md5, state, Instant::now());
    }

    fn insert_at(&mut self, md5: String, state: RankingState, now: Instant) {
        self.entries.retain(|entry| entry.md5 != md5);
        self.entries.push(RankingEntry { md5, state, stored_at: now });
        while self.entries.len() > self.capacity {
            self.entries.remove(0);
        }
    }

    /// Replace a cached chart's state with the same board grown by a page, keeping the time it was
    /// first stored. Returns whether the chart was cached: one that was forgotten meanwhile is not
    /// brought back, as its board is the one that was found out of date.
    pub(crate) fn extend(&mut self, md5: &str, state: RankingState) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.md5 == md5) else {
            return false;
        };
        let mut entry = self.entries.remove(index);
        entry.state = state;
        self.entries.push(entry);
        true
    }

    /// Forget a chart, so the next focus on it starts a fresh fetch. Used when a fetch is dropped
    /// and its `Loading` placeholder would otherwise read as a cache hit forever, and when the
    /// server has taken a score that the cached board does not show.
    pub(crate) fn remove(&mut self, md5: &str) {
        self.entries.retain(|entry| entry.md5 != md5);
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// Whether the chart is cached with a finished answer that is still good at `now`. A `Loading`
    /// placeholder is *not* a hit: its fetch may have been dropped, and treating it as one would
    /// strand the panel. A board older than [`RANKING_READY_LIFETIME`] and a failure older than
    /// [`RANKING_FAILED_RETRY_AFTER`] are not hits either, so the chart is asked for again.
    pub(crate) fn has_settled_answer(&mut self, md5: &str, now: Instant) -> bool {
        let good = self.entries.iter().find(|entry| entry.md5 == md5).is_some_and(|entry| {
            let age = now.saturating_duration_since(entry.stored_at);
            match entry.state {
                RankingState::Loading => false,
                RankingState::Failed(_) => age < RANKING_FAILED_RETRY_AFTER,
                RankingState::Ready(_) => age < RANKING_READY_LIFETIME,
            }
        });
        good && self.touch(md5)
    }

    /// Whether the chart's cached answer is a failure old enough to be asked for again at `now`.
    pub(crate) fn failure_is_due(&self, md5: &str, now: Instant) -> bool {
        self.entries
            .iter()
            .find(|entry| entry.md5 == md5)
            .is_some_and(|entry| matches!(entry.state, RankingState::Failed(_)) && now.saturating_duration_since(entry.stored_at) >= RANKING_FAILED_RETRY_AFTER)
    }

    /// Note the chart whose score is on its way to the server, so the board cached for it can be
    /// forgotten once the server has taken the score ([`RankingCache::awaited_score_chart`]).
    pub(crate) fn await_score_of(&mut self, md5: &str) {
        self.awaiting_score_of = Some(md5.to_string());
    }

    /// The chart of the score handed to the server last.
    pub(crate) fn awaited_score_chart(&self) -> Option<&str> {
        self.awaiting_score_of.as_deref()
    }

    /// Number of cached charts. Used by the eviction tests.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Cached chart keys, oldest first. Used by the eviction tests.
    #[cfg(test)]
    pub(crate) fn keys(&self) -> Vec<&str> {
        self.entries.iter().map(|entry| entry.md5.as_str()).collect()
    }
}

/// Take a finished fetch if it still belongs to the focused chart. A result from an older
/// generation is dropped so a stale leaderboard can never be shown, and the chart's `Loading`
/// placeholder is cleared with it so the panel refetches instead of showing LOADING forever.
///
/// A board grown by a page keeps the age of the board it grew from; any other result starts its
/// lifetime at `now`.
pub(crate) fn accept_fetch(cache: &mut RankingCache, current_generation: u64, fetch: RankingFetch, now: Instant) -> bool {
    if fetch.generation != current_generation {
        cache.remove(&fetch.md5);
        return false;
    }
    if fetch.extends {
        return cache.extend(&fetch.md5, fetch.state);
    }
    cache.insert_at(fetch.md5, fetch.state, now);
    true
}

/// The engine lamp behind an IR lamp, for the panel's label and colour. This is the inverse of
/// `rbms_ir::mapping::ir_clear`, so a lamp survives a round trip through the IR.
pub(crate) fn clear_type_from_lamp(lamp: ClearLamp) -> ClearType {
    match lamp {
        ClearLamp::NoPlay => ClearType::NoPlay,
        ClearLamp::Failed => ClearType::Failed,
        ClearLamp::AssistEasy => ClearType::AssistEasy,
        ClearLamp::LightAssistEasy => ClearType::LightAssistEasy,
        ClearLamp::Easy => ClearType::Easy,
        ClearLamp::Normal => ClearType::Normal,
        ClearLamp::Hard => ClearType::Hard,
        ClearLamp::ExHard => ClearType::ExHard,
        ClearLamp::FullCombo => ClearType::FullCombo,
        ClearLamp::Perfect => ClearType::Perfect,
        ClearLamp::Max => ClearType::Max,
    }
}

/// Build one row. `fallback_rank` is used when the server did not number the row itself.
///
/// The lamp is named with the assist grade the server sent beside it, so a graded light-assist
/// clear reads as the grade (ASSIST HARD) rather than as LIGHT ASSIST. Only that lamp has a grade;
/// one sent beside any other is no grade.
fn row_from_record(record: &ScoreRecord, fallback_rank: Option<u32>, is_you: bool, is_rival: bool) -> RankingRow {
    let clear = clear_type_from_lamp(record.clear);
    let grade = if clear == ClearType::LightAssistEasy { AssistGrade::from_id(record.assist_grade) } else { AssistGrade::None };
    let (lamp, lamp_color) = lamp_label_color(clear, grade);
    RankingRow {
        rank: record.rank.or(fallback_rank),
        player: if record.player_name.trim().is_empty() { record.player.id.clone() } else { record.player_name.clone() },
        ex_score: record.ex_score,
        record_lamp: RecordLamp { clear: clear_type_id(clear), assist_grade: grade.id() },
        lamp,
        lamp_color,
        fast_slow: record.judge.as_ref().map(|judge| (judge.fast, judge.slow)),
        replay_id: record.replay_id().map(str::to_string),
        played_at: record.played_at,
        player_id: record.player.id.clone(),
        dans: record.dans(),
        total_notes: record.total_notes,
        is_you,
        is_rival,
        scratch_auto: record.is_scratch_auto(),
    }
}

/// Who a leaderboard row can be told to be: the local player and the rivals whose bests the board
/// holds, by the ids the server knows them by.
struct Viewer<'a> {
    you: Option<&'a str>,
    rivals: Vec<&'a str>,
}

impl Viewer<'_> {
    fn rows(&self, records: &[ScoreRecord], first_rank: u32) -> Vec<RankingRow> {
        records
            .iter()
            .zip(first_rank..)
            .map(|(record, rank)| {
                let id = record.player.id.as_str();
                row_from_record(record, Some(rank), self.you == Some(id), self.rivals.contains(&id))
            })
            .collect()
    }
}

/// Whether a page that came back holds as many rows as were asked for, so that another may follow.
fn is_full_page(rows: usize) -> bool {
    u32::try_from(rows).is_ok_and(|rows| rows >= RANKING_PAGE_LIMIT)
}

/// The rank of the first row of `page`, which the rows after it count on from when the server
/// numbers none.
fn rank_by_place(page: u32) -> u32 {
    page.saturating_sub(DEFAULT_PAGE).saturating_mul(RANKING_PAGE_LIMIT).saturating_add(1)
}

/// Assemble the panel content from the responses the fetch collected: the first page of the
/// leaderboard, the local player's best and each rival's best.
pub(crate) fn build_board(ranking: &[ScoreRecord], you: Option<&ScoreRecord>, rivals: &[(String, Option<ScoreRecord>)]) -> RankingBoard {
    let viewer = Viewer { you: you.map(|record| record.player.id.as_str()), rivals: rivals.iter().map(|(id, _)| id.as_str()).collect() };
    RankingBoard {
        rows: viewer.rows(ranking, rank_by_place(DEFAULT_PAGE)),
        you: you.map(|record| row_from_record(record, None, true, false)),
        rivals: rivals.iter().map(|(id, record)| (id.clone(), record.as_ref().map(|r| row_from_record(r, None, false, true)))).collect(),
        pages: DEFAULT_PAGE,
        more: is_full_page(ranking.len()),
        your_page: None,
    }
}

/// `board` with the leaderboard page `records` came from added to it. The page after the ones the
/// board holds joins its rows; any page further down is kept apart as the page of the local player's
/// row. A page the board holds already changes nothing.
fn with_page(mut board: RankingBoard, page: u32, records: &[ScoreRecord]) -> RankingBoard {
    if page <= board.pages {
        return board;
    }
    let rows = {
        let viewer = Viewer { you: board.you.as_ref().map(|row| row.player_id.as_str()), rivals: board.rivals.iter().map(|(id, _)| id.as_str()).collect() };
        viewer.rows(records, rank_by_place(page))
    };
    if page == board.pages + 1 {
        board.rows.extend(rows);
        board.pages = page;
        board.more = is_full_page(records.len());
        if board.your_page.as_ref().is_some_and(|held| held.page == page) {
            board.your_page = None;
        }
    } else {
        board.your_page = Some(RankingPage { page, rows });
    }
    board
}

/// One page of a chart's leaderboard, and whether the server pages at all. A server that leaves
/// [`ScoreServer::chart_ranking_page`] unsupported answers the first page through
/// [`ScoreServer::chart_ranking`] and has no other.
fn fetch_page(server: &dyn ScoreServer, chart: &ChartId, page: u32) -> Result<(Vec<ScoreRecord>, bool), IrError> {
    let query = ChartRankingQuery { limit: RANKING_PAGE_LIMIT, page, ..ChartRankingQuery::default() };
    match server.chart_ranking_page(chart, &query) {
        Ok(records) => Ok((records, true)),
        Err(IrError::Unsupported) if page == DEFAULT_PAGE => server.chart_ranking(chart, RANKING_PAGE_LIMIT).map(|records| (records, false)),
        Err(error) => Err(error),
    }
}

/// Fetch one chart's panel content. Blocking: this is the body of the background worker.
///
/// Only the leaderboard is load-bearing — a server that cannot answer the player's best or a rival's
/// best still produces a panel, just without those extras.
pub(crate) fn fetch_board(server: &dyn ScoreServer, chart: &ChartId, player: &PlayerId, rivals: &[String]) -> RankingState {
    let (ranking, pageable) = match fetch_page(server, chart, DEFAULT_PAGE) {
        Ok(first) => first,
        Err(error) => return RankingState::Failed(short_error(&error)),
    };
    let you = server.player_best(chart, player).ok().flatten();
    let rival_rows: Vec<(String, Option<ScoreRecord>)> = rivals
        .iter()
        .take(RANKING_MAX_RIVALS)
        .map(|id| {
            let best = server.player_best(chart, &PlayerId { id: id.clone() }).ok().flatten();
            (id.clone(), best)
        })
        .collect();
    let mut board = build_board(&ranking, you.as_ref(), &rival_rows);
    board.more &= pageable;
    RankingState::Ready(Box::new(board))
}

/// Fetch one more page of a chart's leaderboard into its board. Blocking: this is the body of the
/// background worker. A failed request is the error, and the caller keeps the board it has.
pub(crate) fn fetch_board_page(server: &dyn ScoreServer, chart: &ChartId, board: RankingBoard, page: u32) -> Result<RankingBoard, IrError> {
    let (records, _) = fetch_page(server, chart, page)?;
    Ok(with_page(board, page, &records))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::format::clear_label_color;
    use rbms_ir::{AuthRequest, AuthResponse, CourseSubmission, JudgeBreakdown, PlayerProfile, ReplayData, ScoreSubmission, ServerInfo, SubmitResponse};
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Mutex;

    const TICK: Duration = Duration::from_millis(1);

    impl Default for RankingRow {
        fn default() -> RankingRow {
            RankingRow {
                rank: None,
                player: String::new(),
                ex_score: 0,
                record_lamp: RecordLamp::default(),
                lamp: "",
                lamp_color: Color::GRAY,
                fast_slow: None,
                replay_id: None,
                played_at: 0,
                player_id: String::new(),
                dans: Vec::new(),
                total_notes: 0,
                is_you: false,
                is_rival: false,
                scratch_auto: false,
            }
        }
    }

    pub(crate) fn record(id: &str, name: &str, ex: u32, rank: Option<u32>) -> ScoreRecord {
        ScoreRecord {
            player: PlayerId { id: id.into() },
            player_name: name.into(),
            clear: ClearLamp::Hard,
            ex_score: ex,
            max_combo: 100,
            minbp: 2,
            rank,
            played_at: 1_700_000_000_000,
            lntype: 0,
            option: 0,
            total_notes: 500,
            judge: None,
            assist_grade: 0,
            extra: HashMap::new(),
        }
    }

    pub(crate) fn record_with_replay(id: &str, name: &str, ex: u32, rank: Option<u32>, replay_id: &str) -> ScoreRecord {
        let mut row = record(id, name, ex, rank);
        row.extra.insert("replay_id".to_string(), json!(replay_id));
        row
    }

    /// `count` numberless rows of players named `{prefix}{n}`.
    fn rows_of(prefix: &str, count: u32) -> Vec<ScoreRecord> {
        (0..count).map(|n| record(&format!("{prefix}{n}"), &format!("Player {prefix}{n}"), 2000 - n, None)).collect()
    }

    /// A [`ScoreServer`] scripted for the panel: the pages of a leaderboard, per-player bests, and
    /// a record of what was asked.
    pub(crate) struct RankingDouble {
        /// The first page, which is also what the unpaged `chart_ranking` answers.
        pub(crate) ranking: Result<Vec<ScoreRecord>, IrError>,
        /// Page 2 on, in order.
        pub(crate) later_pages: Vec<Vec<ScoreRecord>>,
        /// Whether `chart_ranking_page` is supported at all.
        pub(crate) pageable: bool,
        pub(crate) bests: HashMap<String, ScoreRecord>,
        pub(crate) pages_asked: Mutex<Vec<ChartRankingQuery>>,
        pub(crate) unpaged_limits_asked: Mutex<Vec<u32>>,
        pub(crate) bests_asked: Mutex<Vec<String>>,
    }

    impl RankingDouble {
        pub(crate) fn new(ranking: Vec<ScoreRecord>) -> RankingDouble {
            RankingDouble {
                ranking: Ok(ranking),
                later_pages: Vec::new(),
                pageable: true,
                bests: HashMap::new(),
                pages_asked: Mutex::new(Vec::new()),
                unpaged_limits_asked: Mutex::new(Vec::new()),
                bests_asked: Mutex::new(Vec::new()),
            }
        }

        fn first_page(&self) -> Result<Vec<ScoreRecord>, IrError> {
            self.ranking.as_ref().map(Clone::clone).map_err(|e| IrError::Network(e.detail().to_string()))
        }
    }

    impl ScoreServer for RankingDouble {
        fn health(&self) -> Result<ServerInfo, IrError> {
            Err(IrError::Unsupported)
        }
        fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
            Err(IrError::Unsupported)
        }
        fn chart_ranking(&self, _chart: &ChartId, limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
            self.unpaged_limits_asked.lock().expect("the limits").push(limit);
            self.first_page()
        }
        fn chart_ranking_page(&self, _chart: &ChartId, query: &ChartRankingQuery) -> Result<Vec<ScoreRecord>, IrError> {
            if !self.pageable {
                return Err(IrError::Unsupported);
            }
            self.pages_asked.lock().expect("the queries").push(query.clone());
            match query.page.checked_sub(DEFAULT_PAGE + 1) {
                None => self.first_page(),
                Some(later) => Ok(self.later_pages.get(later as usize).cloned().unwrap_or_default()),
            }
        }
        fn player_best(&self, _chart: &ChartId, player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
            self.bests_asked.lock().expect("the players").push(player.id.clone());
            Ok(self.bests.get(&player.id).cloned())
        }
        fn player_profile(&self, _player: &PlayerId) -> Result<PlayerProfile, IrError> {
            Err(IrError::Unsupported)
        }
        fn rivals(&self, _player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
            Err(IrError::Unsupported)
        }
        fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
            Err(IrError::Unsupported)
        }
        fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
            Err(IrError::Unsupported)
        }
        fn login(&self, _req: &AuthRequest) -> Result<AuthResponse, IrError> {
            Err(IrError::Unsupported)
        }
    }

    fn chart() -> ChartId {
        ChartId { md5: "MD5".into(), sha256: "SHA".into() }
    }

    fn who(id: &str) -> PlayerId {
        PlayerId { id: id.into() }
    }

    fn ready() -> RankingState {
        RankingState::Ready(Box::new(build_board(&[], None, &[])))
    }

    /// A board of `rows` numberless leaderboard rows on one page, whose local player stands at
    /// `you_rank`.
    fn board_of(rows: u32, you_rank: Option<u32>, more: bool) -> RankingBoard {
        let you = you_rank.map(|rank| record("me", "Me", 1000, Some(rank)));
        RankingBoard { more, ..build_board(&rows_of("p", rows), you.as_ref(), &[]) }
    }

    #[test]
    fn cache_evicts_the_least_recently_used_chart() {
        let mut cache = RankingCache::new(2);
        cache.insert("a".into(), RankingState::Loading);
        cache.insert("b".into(), RankingState::Loading);
        cache.insert("c".into(), RankingState::Loading);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.keys(), vec!["b", "c"], "the oldest chart is dropped");
        assert!(cache.peek("a").is_none());
    }

    #[test]
    fn touching_a_chart_saves_it_from_the_next_eviction() {
        let mut cache = RankingCache::new(2);
        cache.insert("a".into(), RankingState::Loading);
        cache.insert("b".into(), RankingState::Loading);
        assert!(cache.touch("a"), "a is cached");
        cache.insert("c".into(), RankingState::Loading);
        assert_eq!(cache.keys(), vec!["a", "c"], "b was the least recently used");
        assert!(!cache.touch("zz"), "touching an unknown chart reports a miss");
    }

    #[test]
    fn re_inserting_a_chart_replaces_it_without_growing_the_cache() {
        let mut cache = RankingCache::new(4);
        cache.insert("a".into(), RankingState::Loading);
        cache.insert("a".into(), RankingState::Failed("nope".into()));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.peek("a"), Some(&RankingState::Failed("nope".into())));
    }

    #[test]
    fn clearing_the_cache_drops_settled_and_loading_entries() {
        let mut cache = RankingCache::new(4);
        cache.insert("loading".into(), RankingState::Loading);
        cache.insert("ready".into(), RankingState::Failed("down".into()));

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.peek("loading").is_none());
        assert!(cache.peek("ready").is_none());
    }

    #[test]
    fn a_stale_generation_result_is_ignored() {
        let now = Instant::now();
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        let stale = RankingFetch { generation: 1, md5: "old".into(), state: ready(), extends: false };
        assert!(!accept_fetch(&mut cache, 2, stale, now), "a result for a row the user scrolled past is dropped");
        assert_eq!(cache.len(), 0);

        let fresh = RankingFetch { generation: 2, md5: "new".into(), state: RankingState::Loading, extends: false };
        assert!(accept_fetch(&mut cache, 2, fresh, now));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn dropping_a_stale_result_clears_the_charts_loading_placeholder() {
        let now = Instant::now();
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert("old".into(), RankingState::Loading);
        let stale = RankingFetch { generation: 1, md5: "old".into(), state: ready(), extends: false };
        assert!(!accept_fetch(&mut cache, 2, stale, now));
        assert!(cache.peek("old").is_none(), "the placeholder must go, or the chart never refetches");
    }

    #[test]
    fn a_loading_placeholder_is_not_a_settled_answer_but_a_finished_one_is() {
        let now = Instant::now();
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert_at("pending".into(), RankingState::Loading, now);
        assert!(!cache.has_settled_answer("pending", now), "a pending chart must be refetchable");

        cache.insert_at("done".into(), RankingState::Failed("offline".into()), now);
        assert!(cache.has_settled_answer("done", now), "a finished answer is served from the cache");
        assert!(!cache.has_settled_answer("never-seen", now));
    }

    #[test]
    fn a_board_is_served_for_its_lifetime_and_asked_for_again_after_it() {
        let stored = Instant::now();
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert_at("a".into(), ready(), stored);

        assert!(cache.has_settled_answer("a", stored + RANKING_READY_LIFETIME - TICK), "a board is served until its lifetime is over");
        assert!(!cache.has_settled_answer("a", stored + RANKING_READY_LIFETIME), "an old board is asked for again");
        assert!(cache.peek("a").is_some(), "the old board stays readable until the new one replaces it");
    }

    #[test]
    fn a_failure_is_asked_for_again_after_the_retry_delay() {
        let stored = Instant::now();
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert_at("a".into(), RankingState::Failed("down".into()), stored);

        assert!(cache.has_settled_answer("a", stored + RANKING_FAILED_RETRY_AFTER - TICK));
        assert!(!cache.failure_is_due("a", stored + RANKING_FAILED_RETRY_AFTER - TICK));
        assert!(!cache.has_settled_answer("a", stored + RANKING_FAILED_RETRY_AFTER));
        assert!(cache.failure_is_due("a", stored + RANKING_FAILED_RETRY_AFTER));
    }

    #[test]
    fn only_a_failure_can_be_due_for_a_retry() {
        let stored = Instant::now();
        let long_after = stored + RANKING_READY_LIFETIME * 2;
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert_at("ready".into(), ready(), stored);
        cache.insert_at("loading".into(), RankingState::Loading, stored);
        assert!(!cache.failure_is_due("ready", long_after));
        assert!(!cache.failure_is_due("loading", long_after));
        assert!(!cache.failure_is_due("never-seen", long_after));
    }

    #[test]
    fn a_board_grown_by_a_page_keeps_the_age_of_the_board_it_grew_from() {
        let stored = Instant::now();
        let later = stored + RANKING_READY_LIFETIME - TICK;
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert_at("a".into(), ready(), stored);

        let grown = RankingFetch { generation: 3, md5: "a".into(), state: ready(), extends: true };
        assert!(accept_fetch(&mut cache, 3, grown, later));
        assert!(!cache.has_settled_answer("a", stored + RANKING_READY_LIFETIME), "a page does not start the lifetime over");

        let refetched = RankingFetch { generation: 3, md5: "a".into(), state: ready(), extends: false };
        assert!(accept_fetch(&mut cache, 3, refetched, later));
        assert!(cache.has_settled_answer("a", stored + RANKING_READY_LIFETIME), "a fresh board does");
    }

    #[test]
    fn a_page_for_a_chart_that_was_forgotten_meanwhile_is_not_brought_back() {
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        let grown = RankingFetch { generation: 3, md5: "gone".into(), state: ready(), extends: true };
        assert!(!accept_fetch(&mut cache, 3, grown, Instant::now()));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn the_cache_remembers_the_chart_of_the_score_on_its_way() {
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        assert_eq!(cache.awaited_score_chart(), None);
        cache.await_score_of("first");
        cache.await_score_of("second");
        assert_eq!(cache.awaited_score_chart(), Some("second"), "the score handed over last is the one awaited");
        cache.clear();
        assert_eq!(cache.awaited_score_chart(), Some("second"), "forgetting boards does not forget a score in flight");
    }

    #[test]
    fn board_numbers_unnumbered_rows_and_prefers_the_server_rank() {
        let rows = [record("a", "Alice", 1500, None), record("b", "Bob", 1400, Some(9))];
        let board = build_board(&rows, None, &[]);
        assert_eq!(board.rows[0].rank, Some(1), "an unnumbered row falls back to its position");
        assert_eq!(board.rows[1].rank, Some(9), "the server's own rank wins");
        assert_eq!(board.rows[0].player, "Alice");
        assert_eq!(board.rows[0].lamp, clear_label_color(ClearType::Hard).0);
    }

    /// A light-assist clear the server graded is named and counted by its grade, and a grade beside
    /// any other lamp is no grade at all.
    #[test]
    fn a_row_is_named_by_the_assist_grade_the_server_sent() {
        let graded = ScoreRecord { clear: ClearLamp::LightAssistEasy, assist_grade: AssistGrade::Hard.id(), ..record("a", "Alice", 1500, None) };
        let ungraded = ScoreRecord { clear: ClearLamp::LightAssistEasy, ..record("b", "Bob", 1400, None) };
        let stray = ScoreRecord { assist_grade: AssistGrade::ExHard.id(), ..record("c", "Carol", 1300, None) };
        let board = build_board(&[graded, ungraded, stray], None, &[]);
        let named: Vec<&str> = board.rows.iter().map(|row| row.lamp).collect();
        assert_eq!(named, ["ASSIST HARD", "LIGHT ASSIST", "HARD"]);
        let light_assist = clear_type_id(ClearType::LightAssistEasy);
        assert_eq!(board.rows[0].record_lamp, RecordLamp { clear: light_assist, assist_grade: AssistGrade::Hard.id() });
        assert_eq!(board.rows[1].record_lamp, RecordLamp { clear: light_assist, assist_grade: AssistGrade::None.id() });
        assert_eq!(board.rows[2].record_lamp, RecordLamp { clear: clear_type_id(ClearType::Hard), assist_grade: AssistGrade::None.id() });
        assert_eq!(board.rows[0].lamp_color, lamp_label_color(ClearType::LightAssistEasy, AssistGrade::Hard).1);
    }

    #[test]
    fn a_row_without_a_display_name_falls_back_to_the_player_id() {
        let rows = [record("anon", "  ", 900, None)];
        let board = build_board(&rows, None, &[]);
        assert_eq!(board.rows[0].player, "anon");
        assert_eq!(board.rows[0].player_id, "anon");
    }

    #[test]
    fn fast_slow_is_carried_when_the_server_reports_the_judge_detail() {
        let mut with_judge = record("a", "Alice", 1500, None);
        with_judge.judge = Some(JudgeBreakdown { fast: 12, slow: 8, ..Default::default() });
        let board = build_board(&[with_judge, record("b", "Bob", 1000, None)], None, &[]);
        assert_eq!(board.rows[0].fast_slow, Some((12, 8)));
        assert_eq!(board.rows[1].fast_slow, None, "a ranking-only payload has no timing detail");
    }

    #[test]
    fn a_row_carries_the_replay_its_own_score_is_linked_to() {
        let rows = [record_with_replay("a", "Alice", 1500, None, "rep-a"), record("b", "Bob", 1400, None)];
        let you = record_with_replay("me", "Me", 1200, Some(7), "rep-me");
        let board = build_board(&rows, Some(&you), &[]);
        assert_eq!(board.rows[0].replay_id.as_deref(), Some("rep-a"));
        assert_eq!(board.rows[1].replay_id, None, "a score without a replay names none, whoever else has one");
        assert_eq!(board.you.as_ref().and_then(|row| row.replay_id.as_deref()), Some("rep-me"));
    }

    #[test]
    fn rival_rows_cover_both_played_and_unplayed_rivals() {
        let rivals = [("friend".to_string(), Some(record("friend", "Friend", 1200, None))), ("ghost".to_string(), None)];
        let board = build_board(&[], None, &rivals);
        assert_eq!(board.rivals.len(), 2);
        assert_eq!(board.rivals[0].0, "friend");
        assert_eq!(board.rivals[0].1.as_ref().map(|r| r.ex_score), Some(1200));
        assert_eq!(board.rivals[1].0, "ghost");
        assert!(board.rivals[1].1.is_none(), "a rival who has not played the chart still gets a row");
    }

    #[test]
    fn rows_tell_the_local_player_and_the_rivals_apart() {
        let ranking = [record("me", "Me", 1500, None), record("friend", "Friend", 1400, None), record("other", "Other", 1300, None)];
        let you = record("me", "Me", 1500, Some(1));
        let rivals = [("friend".to_string(), Some(record("friend", "Friend", 1400, Some(2)))), ("ghost".to_string(), None)];
        let board = build_board(&ranking, Some(&you), &rivals);

        let marks: Vec<(&str, bool, bool)> = board.rows.iter().map(|row| (row.player_id.as_str(), row.is_you, row.is_rival)).collect();
        assert_eq!(marks, [("me", true, false), ("friend", false, true), ("other", false, false)]);
        assert!(board.you.as_ref().is_some_and(|row| row.is_you && !row.is_rival));
        assert!(board.rivals[0].1.as_ref().is_some_and(|row| row.is_rival && !row.is_you));
    }

    #[test]
    fn a_player_without_a_best_marks_no_leaderboard_row_as_theirs() {
        let board = build_board(&[record("me", "Me", 1500, None)], None, &[]);
        assert!(!board.rows[0].is_you, "a guest, or a best the server did not answer, names nobody");
    }

    #[test]
    fn a_row_carries_the_dans_the_notes_and_scratch_auto_of_its_score() {
        let mut scored = record("a", "Alice", 1500, None);
        scored.extra.insert("dans".to_string(), json!([{"system": "stella", "name": "Stella", "label": "SL 8"}]));
        scored.extra.insert("scratch_auto".to_string(), json!(true));
        let board = build_board(&[scored, record("b", "Bob", 1400, None)], None, &[]);

        assert_eq!(board.rows[0].dans.len(), 1);
        assert_eq!(board.rows[0].dans[0].label, "SL 8");
        assert_eq!(board.rows[0].total_notes, 500);
        assert!(board.rows[0].scratch_auto);
        assert!(board.rows[1].dans.is_empty(), "a server that predates the dans sends none");
        assert!(!board.rows[1].scratch_auto);
    }

    #[test]
    fn a_row_scored_without_a_note_count_keeps_the_zero() {
        let unknown = ScoreRecord { total_notes: 0, ..record("a", "Alice", 1500, None) };
        let board = build_board(&[unknown], None, &[]);
        assert_eq!(board.rows[0].total_notes, 0, "the reader of the row decides what no count means");
    }

    #[test]
    fn a_full_first_page_may_be_followed_by_another_and_a_short_one_is_the_whole_board() {
        let full = build_board(&rows_of("p", RANKING_PAGE_LIMIT), None, &[]);
        assert_eq!((full.pages, full.more), (1, true));
        let short = build_board(&rows_of("p", RANKING_PAGE_LIMIT - 1), None, &[]);
        assert_eq!((short.pages, short.more), (1, false));
        let empty = build_board(&[], None, &[]);
        assert_eq!((empty.pages, empty.more), (1, false), "an empty chart is a board, with nothing after it");
    }

    #[test]
    fn the_page_to_find_the_local_player_on_follows_their_rank() {
        let page_of = |rank| board_of(RANKING_PAGE_LIMIT, Some(rank), true).page_to_find_you();
        assert_eq!(page_of(RANKING_PAGE_LIMIT), None, "a rank on the first page is in the rows");
        assert_eq!(page_of(RANKING_PAGE_LIMIT + 1), Some(2));
        assert_eq!(page_of(2 * RANKING_PAGE_LIMIT), Some(2));
        assert_eq!(page_of(2 * RANKING_PAGE_LIMIT + 1), Some(3));
        assert_eq!(page_of(250), Some(3));
    }

    #[test]
    fn no_page_is_looked_for_without_a_rank_a_further_page_or_a_page_already_held() {
        assert_eq!(board_of(RANKING_PAGE_LIMIT, None, true).page_to_find_you(), None, "no best, no page");
        assert_eq!(board_of(RANKING_PAGE_LIMIT, Some(250), false).page_to_find_you(), None, "the server has no page after the first");
        let mut held = board_of(RANKING_PAGE_LIMIT, Some(250), true);
        held.your_page = Some(RankingPage { page: 3, rows: Vec::new() });
        assert_eq!(held.page_to_find_you(), None, "the page is held apart already");
        let mut unnumbered = board_of(RANKING_PAGE_LIMIT, Some(250), true);
        unnumbered.you.as_mut().expect("a best").rank = None;
        assert_eq!(unnumbered.page_to_find_you(), None, "a best without a rank points at no page");
    }

    #[test]
    fn a_page_is_wanted_only_when_it_adds_rows() {
        let mut board = board_of(RANKING_PAGE_LIMIT, None, true);
        board.pages = 2;
        assert!(board.wants_page(3));
        assert!(board.wants_page(5));
        assert!(!board.wants_page(2), "the board holds it");
        assert!(!board.wants_page(1));
        assert!(!board.wants_page(0));

        board.your_page = Some(RankingPage { page: 5, rows: Vec::new() });
        assert!(!board.wants_page(5), "it is held apart");
        assert!(board.wants_page(4));

        board.more = false;
        assert!(!board.wants_page(4), "the server has no page to give");
    }

    #[test]
    fn the_next_page_joins_the_rows_and_carries_the_ranks_on() {
        let board = board_of(RANKING_PAGE_LIMIT, None, true);
        let grown = with_page(board, 2, &rows_of("q", 40));

        assert_eq!(grown.rows.len(), 140);
        assert_eq!(grown.rows[RANKING_PAGE_LIMIT as usize].rank, Some(RANKING_PAGE_LIMIT + 1), "the second page counts on from the first");
        assert_eq!(grown.rows[139].rank, Some(140));
        assert_eq!((grown.pages, grown.more), (2, false), "a short page is the last");
        assert!(grown.your_page.is_none());
    }

    #[test]
    fn a_full_next_page_leaves_the_server_a_page_to_give() {
        let grown = with_page(board_of(RANKING_PAGE_LIMIT, None, true), 2, &rows_of("q", RANKING_PAGE_LIMIT));
        assert_eq!((grown.pages, grown.more), (2, true));
    }

    #[test]
    fn a_page_the_server_numbers_keeps_the_servers_numbers() {
        let numbered = [record("q0", "Q0", 900, Some(101)), record("q1", "Q1", 900, Some(101)), record("q2", "Q2", 800, Some(103))];
        let grown = with_page(board_of(RANKING_PAGE_LIMIT, None, true), 2, &numbered);
        let ranks: Vec<Option<u32>> = grown.rows[RANKING_PAGE_LIMIT as usize..].iter().map(|row| row.rank).collect();
        assert_eq!(ranks, [Some(101), Some(101), Some(103)], "a tie shares its rank");
    }

    #[test]
    fn a_page_further_down_is_kept_apart_and_the_rows_stay_as_they_were() {
        let board = board_of(RANKING_PAGE_LIMIT, Some(250), true);
        let grown = with_page(board.clone(), 3, &rows_of("q", RANKING_PAGE_LIMIT));

        assert_eq!(grown.rows, board.rows, "the rows still run from the top without a gap");
        assert_eq!((grown.pages, grown.more), (1, true));
        let held = grown.your_page.expect("the page of the local player is held");
        assert_eq!(held.page, 3);
        assert_eq!(held.rows.len(), RANKING_PAGE_LIMIT as usize);
        assert_eq!(held.rows[0].rank, Some(2 * RANKING_PAGE_LIMIT + 1));
    }

    #[test]
    fn the_rows_reaching_the_page_held_apart_take_its_place() {
        let apart = with_page(board_of(RANKING_PAGE_LIMIT, Some(250), true), 3, &rows_of("q", RANKING_PAGE_LIMIT));
        let second = with_page(apart, 2, &rows_of("r", RANKING_PAGE_LIMIT));
        assert!(second.your_page.is_some(), "page 3 is not in the rows yet");
        assert_eq!(second.pages, 2);

        let third = with_page(second, 3, &rows_of("q", RANKING_PAGE_LIMIT));
        assert!(third.your_page.is_none(), "page 3 is in the rows now");
        assert_eq!((third.pages, third.rows.len()), (3, 3 * RANKING_PAGE_LIMIT as usize));
    }

    #[test]
    fn a_page_the_board_holds_changes_nothing() {
        let board = board_of(RANKING_PAGE_LIMIT, Some(250), true);
        assert_eq!(with_page(board.clone(), 1, &rows_of("q", RANKING_PAGE_LIMIT)), board);
        assert_eq!(with_page(board.clone(), 0, &rows_of("q", RANKING_PAGE_LIMIT)), board);
    }

    #[test]
    fn rows_of_a_later_page_know_the_local_player_and_the_rivals() {
        let you = record("me", "Me", 1500, Some(150));
        let rivals = [("friend".to_string(), Some(record("friend", "Friend", 1400, Some(120))))];
        let board = RankingBoard { more: true, ..build_board(&rows_of("p", RANKING_PAGE_LIMIT), Some(&you), &rivals) };
        let page = [record("friend", "Friend", 1400, Some(120)), record("me", "Me", 1500, Some(150)), record("other", "Other", 1000, Some(160))];

        let grown = with_page(board, 2, &page);

        let marks: Vec<(bool, bool)> = grown.rows[RANKING_PAGE_LIMIT as usize..].iter().map(|row| (row.is_you, row.is_rival)).collect();
        assert_eq!(marks, [(false, true), (true, false), (false, false)]);
    }

    #[test]
    fn fetch_asks_for_the_first_page_of_a_hundred() {
        let server = RankingDouble::new(vec![record("a", "Alice", 1500, None)]);
        let state = fetch_board(&server, &chart(), &who("me"), &[]);
        assert!(matches!(state, RankingState::Ready(_)));
        let asked = server.pages_asked.lock().expect("the queries").clone();
        assert_eq!(asked, [ChartRankingQuery { limit: 100, page: 1, lnmode: None, rival_of: None }]);
        assert!(server.unpaged_limits_asked.lock().expect("the limits").is_empty(), "a server that pages is not asked the unpaged way");
    }

    #[test]
    fn fetch_collects_the_leaderboard_the_player_best_and_the_rivals() {
        let mut server = RankingDouble::new(vec![record_with_replay("a", "Alice", 1500, None, "rep-a")]);
        server.bests.insert("me".into(), record("me", "Me", 1100, Some(4)));
        server.bests.insert("friend".into(), record("friend", "Friend", 1300, Some(2)));

        let state = fetch_board(&server, &chart(), &who("me"), &["friend".into(), "ghost".into()]);
        let RankingState::Ready(board) = state else { panic!("expected a ready board") };
        assert_eq!(board.rows.len(), 1);
        assert_eq!(board.rows[0].replay_id.as_deref(), Some("rep-a"));
        assert_eq!(board.you.as_ref().map(|r| r.ex_score), Some(1100));
        assert_eq!(board.rivals.len(), 2);
        assert_eq!(board.rivals[0].1.as_ref().map(|r| r.ex_score), Some(1300));
        assert!(board.rivals[1].1.is_none());
        assert_eq!((board.pages, board.more), (1, false));
    }

    #[test]
    fn fetch_reports_a_leaderboard_failure_and_asks_for_nothing_else() {
        let mut failing = RankingDouble::new(Vec::new());
        failing.ranking = Err(IrError::Network("down".into()));
        let state = fetch_board(&failing, &chart(), &who("me"), &["friend".into()]);
        let RankingState::Failed(message) = state else { panic!("expected a failure") };
        assert!(message.contains("down"), "{message}");
        assert!(failing.bests_asked.lock().expect("the players").is_empty(), "a board without a leaderboard asks for no best");
    }

    #[test]
    fn fetch_survives_a_player_the_server_has_no_best_for() {
        let server = RankingDouble::new(vec![record("a", "Alice", 1500, None)]);
        let state = fetch_board(&server, &chart(), &who("me"), &[]);
        let RankingState::Ready(board) = state else { panic!("a server without a best still renders") };
        assert!(board.you.is_none(), "an unknown player simply has no YOU record");
    }

    #[test]
    fn a_guest_gets_the_leaderboard_without_a_you_row() {
        let mut server = RankingDouble::new(vec![record("a", "Alice", 1500, None)]);
        server.bests.insert("me".into(), record("me", "Me", 1100, Some(4)));
        let RankingState::Ready(board) = fetch_board(&server, &chart(), &who(rbms_ir::GUEST_PLAYER_ID), &[]) else {
            panic!("a guest still sees the leaderboard")
        };
        assert_eq!(board.rows.len(), 1);
        assert!(board.you.is_none());
        assert!(board.rows.iter().all(|row| !row.is_you));
    }

    #[test]
    fn a_missing_server_fails_the_board_instead_of_pretending_it_has_no_scores() {
        let state = fetch_board(&rbms_ir::NullScoreServer, &chart(), &who("me"), &[]);
        assert!(matches!(state, RankingState::Failed(_)), "{state:?}");
    }

    #[test]
    fn a_server_that_does_not_page_answers_the_first_page_only() {
        let mut server = RankingDouble::new(rows_of("p", RANKING_PAGE_LIMIT));
        server.pageable = false;
        let state = fetch_board(&server, &chart(), &who("me"), &[]);
        let RankingState::Ready(board) = state else { panic!("a server without paging still renders") };
        assert_eq!(board.rows.len(), RANKING_PAGE_LIMIT as usize);
        assert!(!board.more, "there is no second page to ask for");
        assert_eq!(*server.unpaged_limits_asked.lock().expect("the limits"), [RANKING_PAGE_LIMIT]);
    }

    #[test]
    fn fetch_never_asks_for_more_rivals_than_the_cap() {
        let server = RankingDouble::new(Vec::new());
        let many: Vec<String> = (0..RANKING_MAX_RIVALS + 5).map(|i| format!("r{i}")).collect();
        let state = fetch_board(&server, &chart(), &who("me"), &many);
        let RankingState::Ready(board) = state else { panic!("expected a ready board") };
        assert_eq!(board.rivals.len(), RANKING_MAX_RIVALS);
    }

    #[test]
    fn a_player_far_below_the_first_page_is_pointed_at_their_page() {
        let mut server = RankingDouble::new(rows_of("p", RANKING_PAGE_LIMIT));
        server.bests.insert("me".into(), record("me", "Me", 100, Some(250)));
        let RankingState::Ready(board) = fetch_board(&server, &chart(), &who("me"), &[]) else { panic!("expected a ready board") };
        assert_eq!(board.page_to_find_you(), Some(3));
    }

    #[test]
    fn the_next_page_is_fetched_into_the_board() {
        let mut server = RankingDouble::new(rows_of("p", RANKING_PAGE_LIMIT));
        server.later_pages = vec![rows_of("q", 7)];
        let RankingState::Ready(board) = fetch_board(&server, &chart(), &who("me"), &[]) else { panic!("expected a ready board") };

        let grown = fetch_board_page(&server, &chart(), *board, 2).expect("the second page arrives");

        assert_eq!(grown.rows.len(), RANKING_PAGE_LIMIT as usize + 7);
        assert_eq!((grown.pages, grown.more), (2, false));
        let asked = server.pages_asked.lock().expect("the queries").clone();
        assert_eq!(asked.iter().map(|query| (query.limit, query.page)).collect::<Vec<_>>(), [(100, 1), (100, 2)]);
    }

    #[test]
    fn a_page_that_cannot_be_fetched_is_an_error_and_the_caller_keeps_its_board() {
        let mut server = RankingDouble::new(rows_of("p", RANKING_PAGE_LIMIT));
        let RankingState::Ready(board) = fetch_board(&server, &chart(), &who("me"), &[]) else { panic!("expected a ready board") };
        server.pageable = false;

        let result = fetch_board_page(&server, &chart(), (*board).clone(), 2);

        assert!(matches!(result, Err(IrError::Unsupported)), "the second page of an unpaged server is no fallback to the first");
    }

    #[test]
    fn every_ir_lamp_maps_to_an_engine_lamp() {
        let all = [
            ClearLamp::NoPlay,
            ClearLamp::Failed,
            ClearLamp::AssistEasy,
            ClearLamp::LightAssistEasy,
            ClearLamp::Easy,
            ClearLamp::Normal,
            ClearLamp::Hard,
            ClearLamp::ExHard,
            ClearLamp::FullCombo,
            ClearLamp::Perfect,
            ClearLamp::Max,
        ];
        for lamp in all {
            let label = clear_label_color(clear_type_from_lamp(lamp)).0;
            assert!(!label.is_empty(), "{lamp:?} has a label");
        }
        assert_eq!(clear_type_from_lamp(ClearLamp::LightAssistEasy), ClearType::LightAssistEasy);
        assert_eq!(clear_type_from_lamp(ClearLamp::Normal), ClearType::Normal);
    }
}

//! Song-select IR ranking: fetching one chart's leaderboard off the frame thread, and the bounded
//! cache that keeps a moving selection cursor from firing a request per row.
//!
//! Everything here is data. The panel that draws it lives in [`crate::ir_ranking_view`].

use std::collections::HashMap;

use rbms_ir::{ChartId, ChartReplayQuery, ClearLamp, PlayerId, ReplayMeta, ScoreRecord, ScoreServer};
use rbms_judge::ClearType;
use rbms_render::Color;

use crate::format::clear_label_color;
use crate::ir_outcome::short_error;

/// Leaderboard rows requested for the panel. Ten fills the panel without paging.
pub(crate) const RANKING_PANEL_LIMIT: u32 = 10;

/// Charts kept in the ranking cache. Large enough to cover scrolling a folder back and forth,
/// small enough that the whole cache is a few kilobytes.
pub(crate) const RANKING_CACHE_CAPACITY: usize = 64;

/// Rivals whose best is fetched for the panel. Each one costs a request, so the list is capped.
pub(crate) const RANKING_MAX_RIVALS: usize = 8;

/// One rendered leaderboard entry: a ranking row, the local player's best, or a rival's best.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RankingRow {
    pub(crate) rank: Option<u32>,
    pub(crate) player: String,
    pub(crate) ex_score: u32,
    pub(crate) lamp: &'static str,
    pub(crate) lamp_color: Color,
    pub(crate) fast_slow: Option<(u32, u32)>,
    pub(crate) replay_id: Option<String>,
}

/// One chart's panel content: the leaderboard, the local player's row, and one row per rival
/// (`None` when that rival has not played the chart).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RankingBoard {
    pub(crate) rows: Vec<RankingRow>,
    pub(crate) you: Option<RankingRow>,
    pub(crate) rivals: Vec<(String, Option<RankingRow>)>,
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
}

/// Bounded most-recently-used cache of per-chart panel content, keyed by chart md5.
///
/// Ordered oldest-first; a hit or an insert moves the chart to the end, and the oldest entry is
/// evicted once the capacity is exceeded.
pub(crate) struct RankingCache {
    capacity: usize,
    entries: Vec<(String, RankingState)>,
}

impl RankingCache {
    pub(crate) fn new(capacity: usize) -> RankingCache {
        RankingCache { capacity: capacity.max(1), entries: Vec::new() }
    }

    /// Read an entry without changing the eviction order.
    pub(crate) fn peek(&self, md5: &str) -> Option<&RankingState> {
        self.entries.iter().find(|(key, _)| key == md5).map(|(_, state)| state)
    }

    /// Mark a chart as most recently used. Returns whether it was cached at all.
    pub(crate) fn touch(&mut self, md5: &str) -> bool {
        match self.entries.iter().position(|(key, _)| key == md5) {
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
        self.entries.retain(|(key, _)| key != &md5);
        self.entries.push((md5, state));
        while self.entries.len() > self.capacity {
            self.entries.remove(0);
        }
    }

    /// Forget a chart, so the next focus on it starts a fresh fetch. Used when a fetch is dropped
    /// and its `Loading` placeholder would otherwise read as a cache hit forever.
    pub(crate) fn remove(&mut self, md5: &str) {
        self.entries.retain(|(key, _)| key != md5);
    }

    /// Whether the chart is cached with a finished answer. A `Loading` placeholder is *not* a hit:
    /// its fetch may have been dropped, and treating it as one would strand the panel.
    pub(crate) fn has_settled_answer(&mut self, md5: &str) -> bool {
        if matches!(self.peek(md5), Some(RankingState::Loading)) {
            return false;
        }
        self.touch(md5)
    }

    /// Number of cached charts. Used by the eviction tests.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Cached chart keys, oldest first. Used by the eviction tests.
    #[cfg(test)]
    pub(crate) fn keys(&self) -> Vec<&str> {
        self.entries.iter().map(|(key, _)| key.as_str()).collect()
    }
}

/// Take a finished fetch if it still belongs to the focused chart. A result from an older
/// generation is dropped so a stale leaderboard can never be shown, and the chart's `Loading`
/// placeholder is cleared with it so the panel refetches instead of showing LOADING forever.
pub(crate) fn accept_fetch(cache: &mut RankingCache, current_generation: u64, fetch: RankingFetch) -> bool {
    if fetch.generation != current_generation {
        cache.remove(&fetch.md5);
        return false;
    }
    cache.insert(fetch.md5, fetch.state);
    true
}

/// The engine lamp behind an IR lamp, for the panel's label and colour. The IR's
/// `LightAssistEasy` has no engine equivalent and folds onto `AssistEasy`, matching how the local
/// score book reads a stored id 3.
pub(crate) fn clear_type_from_lamp(lamp: ClearLamp) -> ClearType {
    match lamp {
        ClearLamp::NoPlay => ClearType::NoPlay,
        ClearLamp::Failed => ClearType::Failed,
        ClearLamp::AssistEasy | ClearLamp::LightAssistEasy => ClearType::AssistEasy,
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
fn row_from_record(record: &ScoreRecord, fallback_rank: Option<u32>, replays: &HashMap<&str, &str>) -> RankingRow {
    let (lamp, lamp_color) = clear_label_color(clear_type_from_lamp(record.clear));
    RankingRow {
        rank: record.rank.or(fallback_rank),
        player: if record.player_name.trim().is_empty() { record.player.id.clone() } else { record.player_name.clone() },
        ex_score: record.ex_score,
        lamp,
        lamp_color,
        fast_slow: record.judge.as_ref().map(|judge| (judge.fast, judge.slow)),
        replay_id: replays.get(record.player.id.as_str()).map(|id| (*id).to_string()),
    }
}

/// Assemble the panel content from the four responses the fetch collected.
pub(crate) fn build_board(
    ranking: &[ScoreRecord],
    you: Option<&ScoreRecord>,
    rivals: &[(String, Option<ScoreRecord>)],
    replays: &[ReplayMeta],
) -> RankingBoard {
    let mut by_player: HashMap<&str, &str> = HashMap::new();
    for meta in replays {
        by_player.entry(meta.player.id.as_str()).or_insert(meta.id.as_str());
    }
    RankingBoard {
        rows: ranking.iter().enumerate().map(|(index, record)| row_from_record(record, Some(index as u32 + 1), &by_player)).collect(),
        you: you.map(|record| row_from_record(record, None, &by_player)),
        rivals: rivals.iter().map(|(id, record)| (id.clone(), record.as_ref().map(|r| row_from_record(r, None, &by_player)))).collect(),
    }
}

/// Fetch one chart's panel content. Blocking: this is the body of the background worker.
///
/// Only the leaderboard is load-bearing — a server that cannot answer the player's best, a rival's
/// best or the replay listing still produces a panel, just without those extras.
pub(crate) fn fetch_board(server: &dyn ScoreServer, chart: &ChartId, player: &PlayerId, rivals: &[String]) -> RankingState {
    let ranking = match server.chart_ranking(chart, RANKING_PANEL_LIMIT) {
        Ok(rows) => rows,
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
    let replays = server.chart_replays(chart, &ChartReplayQuery { player: None, limit: RANKING_PANEL_LIMIT }).unwrap_or_default();
    RankingState::Ready(Box::new(build_board(&ranking, you.as_ref(), &rival_rows, &replays)))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rbms_ir::{
        AuthRequest, AuthResponse, CourseSubmission, IrError, JudgeBreakdown, PlayerProfile, ReplayData, ScoreSubmission, ServerInfo, SubmitResponse,
    };

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
            extra: HashMap::new(),
        }
    }

    pub(crate) fn replay_meta(id: &str, player: &str) -> ReplayMeta {
        ReplayMeta {
            id: id.into(),
            url: format!("/replays/{id}"),
            player: PlayerId { id: player.into() },
            player_name: player.into(),
            chart_sha256: "sha".into(),
            score_id: None,
            format: "rbms-us-v1".into(),
            mode: "BEAT_7K".into(),
            seed: 1,
            lntype: 0,
            event_count: 3,
            duration_us: 10,
            size: 64,
            client_build_sha256: None,
            created_at: 1_700_000_000_000,
        }
    }

    /// A [`ScoreServer`] scripted for the panel: a leaderboard, per-player bests, and an optional
    /// replay listing.
    pub(crate) struct RankingDouble {
        pub(crate) ranking: Result<Vec<ScoreRecord>, IrError>,
        pub(crate) bests: HashMap<String, ScoreRecord>,
        pub(crate) replays: Result<Vec<ReplayMeta>, IrError>,
    }

    impl RankingDouble {
        pub(crate) fn new(ranking: Vec<ScoreRecord>) -> RankingDouble {
            RankingDouble { ranking: Ok(ranking), bests: HashMap::new(), replays: Ok(Vec::new()) }
        }
    }

    impl ScoreServer for RankingDouble {
        fn health(&self) -> Result<ServerInfo, IrError> {
            Err(IrError::Unsupported)
        }
        fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
            Err(IrError::Unsupported)
        }
        fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
            self.ranking.as_ref().map(Clone::clone).map_err(|e| IrError::Network(e.detail().to_string()))
        }
        fn player_best(&self, _chart: &ChartId, player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
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
        fn chart_replays(&self, _chart: &ChartId, _query: &ChartReplayQuery) -> Result<Vec<ReplayMeta>, IrError> {
            self.replays.as_ref().map(Clone::clone).map_err(|_| IrError::Unsupported)
        }
    }

    fn chart() -> ChartId {
        ChartId { md5: "MD5".into(), sha256: "SHA".into() }
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
    fn a_stale_generation_result_is_ignored() {
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        let stale = RankingFetch { generation: 1, md5: "old".into(), state: RankingState::Ready(Box::new(build_board(&[], None, &[], &[]))) };
        assert!(!accept_fetch(&mut cache, 2, stale), "a result for a row the user scrolled past is dropped");
        assert_eq!(cache.len(), 0);

        let fresh = RankingFetch { generation: 2, md5: "new".into(), state: RankingState::Loading };
        assert!(accept_fetch(&mut cache, 2, fresh));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn dropping_a_stale_result_clears_the_charts_loading_placeholder() {
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert("old".into(), RankingState::Loading);
        let stale = RankingFetch { generation: 1, md5: "old".into(), state: RankingState::Ready(Box::new(build_board(&[], None, &[], &[]))) };
        assert!(!accept_fetch(&mut cache, 2, stale));
        assert!(cache.peek("old").is_none(), "the placeholder must go, or the chart never refetches");
    }

    #[test]
    fn a_loading_placeholder_is_not_a_settled_answer_but_a_finished_one_is() {
        let mut cache = RankingCache::new(RANKING_CACHE_CAPACITY);
        cache.insert("pending".into(), RankingState::Loading);
        assert!(!cache.has_settled_answer("pending"), "a pending chart must be refetchable");

        cache.insert("done".into(), RankingState::Failed("offline".into()));
        assert!(cache.has_settled_answer("done"), "a finished answer is served from the cache");
        assert!(!cache.has_settled_answer("never-seen"));
    }

    #[test]
    fn board_numbers_unnumbered_rows_and_prefers_the_server_rank() {
        let rows = [record("a", "Alice", 1500, None), record("b", "Bob", 1400, Some(9))];
        let board = build_board(&rows, None, &[], &[]);
        assert_eq!(board.rows[0].rank, Some(1), "an unnumbered row falls back to its position");
        assert_eq!(board.rows[1].rank, Some(9), "the server's own rank wins");
        assert_eq!(board.rows[0].player, "Alice");
        assert_eq!(board.rows[0].lamp, clear_label_color(ClearType::Hard).0);
    }

    #[test]
    fn a_row_without_a_display_name_falls_back_to_the_player_id() {
        let rows = [record("anon", "  ", 900, None)];
        let board = build_board(&rows, None, &[], &[]);
        assert_eq!(board.rows[0].player, "anon");
    }

    #[test]
    fn fast_slow_is_carried_when_the_server_reports_the_judge_detail() {
        let mut with_judge = record("a", "Alice", 1500, None);
        with_judge.judge = Some(JudgeBreakdown { fast: 12, slow: 8, ..Default::default() });
        let board = build_board(&[with_judge, record("b", "Bob", 1000, None)], None, &[], &[]);
        assert_eq!(board.rows[0].fast_slow, Some((12, 8)));
        assert_eq!(board.rows[1].fast_slow, None, "a ranking-only payload has no timing detail");
    }

    #[test]
    fn replay_ids_attach_to_the_uploading_player_only() {
        let rows = [record("a", "Alice", 1500, None), record("b", "Bob", 1400, None)];
        let board = build_board(&rows, None, &[], &[replay_meta("rep-a", "a")]);
        assert_eq!(board.rows[0].replay_id.as_deref(), Some("rep-a"));
        assert_eq!(board.rows[1].replay_id, None);
    }

    #[test]
    fn the_newest_replay_per_player_wins_when_several_are_listed() {
        let rows = [record("a", "Alice", 1500, None)];
        let board = build_board(&rows, None, &[], &[replay_meta("rep-first", "a"), replay_meta("rep-second", "a")]);
        assert_eq!(board.rows[0].replay_id.as_deref(), Some("rep-first"), "the listing order decides");
    }

    #[test]
    fn rival_rows_cover_both_played_and_unplayed_rivals() {
        let rivals = [("friend".to_string(), Some(record("friend", "Friend", 1200, None))), ("ghost".to_string(), None)];
        let board = build_board(&[], None, &rivals, &[]);
        assert_eq!(board.rivals.len(), 2);
        assert_eq!(board.rivals[0].0, "friend");
        assert_eq!(board.rivals[0].1.as_ref().map(|r| r.ex_score), Some(1200));
        assert_eq!(board.rivals[1].0, "ghost");
        assert!(board.rivals[1].1.is_none(), "a rival who has not played the chart still gets a row");
    }

    #[test]
    fn fetch_collects_the_leaderboard_the_player_best_and_the_rivals() {
        let mut server = RankingDouble::new(vec![record("a", "Alice", 1500, None)]);
        server.bests.insert("me".into(), record("me", "Me", 1100, Some(4)));
        server.bests.insert("friend".into(), record("friend", "Friend", 1300, Some(2)));
        server.replays = Ok(vec![replay_meta("rep-a", "a")]);

        let state = fetch_board(&server, &chart(), &PlayerId { id: "me".into() }, &["friend".into(), "ghost".into()]);
        let RankingState::Ready(board) = state else { panic!("expected a ready board") };
        assert_eq!(board.rows.len(), 1);
        assert_eq!(board.rows[0].replay_id.as_deref(), Some("rep-a"));
        assert_eq!(board.you.as_ref().map(|r| r.ex_score), Some(1100));
        assert_eq!(board.rivals.len(), 2);
        assert_eq!(board.rivals[0].1.as_ref().map(|r| r.ex_score), Some(1300));
        assert!(board.rivals[1].1.is_none());
    }

    #[test]
    fn fetch_reports_a_leaderboard_failure_but_survives_missing_extras() {
        let failing = RankingDouble { ranking: Err(IrError::Network("down".into())), bests: HashMap::new(), replays: Ok(Vec::new()) };
        let state = fetch_board(&failing, &chart(), &PlayerId { id: "me".into() }, &[]);
        let RankingState::Failed(message) = state else { panic!("expected a failure") };
        assert!(message.contains("down"), "{message}");

        let mut no_replays = RankingDouble::new(vec![record("a", "Alice", 1500, None)]);
        no_replays.replays = Err(IrError::Unsupported);
        let state = fetch_board(&no_replays, &chart(), &PlayerId { id: "me".into() }, &[]);
        let RankingState::Ready(board) = state else { panic!("a server without replay listing still renders") };
        assert_eq!(board.rows[0].replay_id, None);
        assert!(board.you.is_none(), "an unknown player simply has no YOU record");
    }

    #[test]
    fn fetch_never_asks_for_more_rivals_than_the_cap() {
        let server = RankingDouble::new(Vec::new());
        let many: Vec<String> = (0..RANKING_MAX_RIVALS + 5).map(|i| format!("r{i}")).collect();
        let state = fetch_board(&server, &chart(), &PlayerId { id: "me".into() }, &many);
        let RankingState::Ready(board) = state else { panic!("expected a ready board") };
        assert_eq!(board.rivals.len(), RANKING_MAX_RIVALS);
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
        assert_eq!(clear_type_from_lamp(ClearLamp::LightAssistEasy), ClearType::AssistEasy);
        assert_eq!(clear_type_from_lamp(ClearLamp::Normal), ClearType::Normal);
    }
}

/// Default `limit` the server applies to paged score and replay listings.
pub const DEFAULT_PAGE_LIMIT: u32 = 50;

/// Default (1-based) page of a paged listing.
pub const DEFAULT_PAGE: u32 = 1;

/// Largest `limit` the ranking endpoints accept; a bigger value is rejected as invalid.
pub const MAX_RANKING_LIMIT: u32 = 500;

/// Largest `limit` `GET /players/{id}/scores` accepts.
pub const MAX_PLAYER_SCORES_LIMIT: u32 = 100;

/// Largest `limit` `GET /charts/{hash}/replays` accepts.
pub const MAX_REPLAY_LIST_LIMIT: u32 = 200;

/// Query for `GET /charts/{hash}/ranking` and `GET /courses/{hash}/ranking`, covering the paging
/// and filtering the plain [`crate::ScoreServer::chart_ranking`] call leaves at its defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartRankingQuery {
    pub limit: u32,
    pub page: u32,
    /// Restrict to one LN handling (0 = LN, 1 = CN, 2 = HCN); `None` mixes them.
    pub lnmode: Option<i32>,
    /// Restrict to the rivals of this player id, for the rival row of a ranking panel.
    pub rival_of: Option<String>,
}

impl Default for ChartRankingQuery {
    fn default() -> ChartRankingQuery {
        ChartRankingQuery { limit: DEFAULT_PAGE_LIMIT, page: DEFAULT_PAGE, lnmode: None, rival_of: None }
    }
}

impl ChartRankingQuery {
    /// Render as a URL query string, omitting the filters that are unset.
    pub fn to_query_string(&self) -> String {
        let mut query = format!("limit={}&page={}", self.limit, self.page);
        if let Some(lnmode) = self.lnmode {
            query.push_str(&format!("&lnmode={lnmode}"));
        }
        if let Some(rival_of) = &self.rival_of {
            query.push_str(&format!("&rival_of={rival_of}"));
        }
        query
    }
}

/// Query for `GET /players/{id}/scores`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerScoresQuery {
    /// Only scores played at or after this unix-ms instant.
    pub since: Option<i64>,
    /// Only scores of this play mode (`BEAT_7K`, …).
    pub mode: Option<String>,
    pub limit: u32,
    pub page: u32,
}

impl Default for PlayerScoresQuery {
    fn default() -> PlayerScoresQuery {
        PlayerScoresQuery { since: None, mode: None, limit: DEFAULT_PAGE_LIMIT, page: DEFAULT_PAGE }
    }
}

impl PlayerScoresQuery {
    /// Render as a URL query string, omitting the filters that are unset.
    pub fn to_query_string(&self) -> String {
        let mut query = format!("limit={}&page={}", self.limit, self.page);
        if let Some(since) = self.since {
            query.push_str(&format!("&since={since}"));
        }
        if let Some(mode) = &self.mode {
            query.push_str(&format!("&mode={mode}"));
        }
        query
    }
}

/// Query for `GET /charts/{hash}/replays`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartReplayQuery {
    /// Only replays uploaded by this player id.
    pub player: Option<String>,
    pub limit: u32,
}

impl Default for ChartReplayQuery {
    fn default() -> ChartReplayQuery {
        ChartReplayQuery { player: None, limit: DEFAULT_PAGE_LIMIT }
    }
}

impl ChartReplayQuery {
    /// Render as a URL query string, omitting the filter when it is unset.
    pub fn to_query_string(&self) -> String {
        let mut query = format!("limit={}", self.limit);
        if let Some(player) = &self.player {
            query.push_str(&format!("&player={player}"));
        }
        query
    }
}

#[cfg(test)]
mod tests {
    use crate::dto::*;

    #[test]
    fn player_scores_query_defaults_match_the_server() {
        let q = PlayerScoresQuery::default();
        assert_eq!(q.limit, DEFAULT_PAGE_LIMIT);
        assert_eq!(q.page, DEFAULT_PAGE);
        assert_eq!(q.to_query_string(), "limit=50&page=1");
    }

    #[test]
    fn player_scores_query_appends_only_the_set_filters() {
        let mut q = PlayerScoresQuery { limit: 10, page: 2, ..Default::default() };
        assert_eq!(q.to_query_string(), "limit=10&page=2");
        q.since = Some(1700);
        assert_eq!(q.to_query_string(), "limit=10&page=2&since=1700");
        q.mode = Some("BEAT_7K".into());
        assert_eq!(q.to_query_string(), "limit=10&page=2&since=1700&mode=BEAT_7K");
    }

    #[test]
    fn chart_ranking_query_appends_only_the_set_filters() {
        assert_eq!(ChartRankingQuery::default().to_query_string(), "limit=50&page=1");
        let q = ChartRankingQuery { limit: 20, page: 2, lnmode: Some(1), rival_of: Some("gkn".into()) };
        assert_eq!(q.to_query_string(), "limit=20&page=2&lnmode=1&rival_of=gkn");
    }

    #[test]
    fn chart_replay_query_appends_only_the_set_filter() {
        assert_eq!(ChartReplayQuery::default().to_query_string(), "limit=50");
        let q = ChartReplayQuery { player: Some("gkn".into()), limit: 5 };
        assert_eq!(q.to_query_string(), "limit=5&player=gkn");
    }

    #[test]
    fn documented_server_limits_match_the_server_schemas() {
        assert_eq!(MAX_RANKING_LIMIT, 500);
        assert_eq!(MAX_PLAYER_SCORES_LIMIT, 100);
        assert_eq!(MAX_REPLAY_LIST_LIMIT, 200);
    }
}

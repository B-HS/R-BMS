use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dto::{ChartId, ClearLamp, JudgeBreakdown, PlayOptions, PlayerId};

/// A score upload. `api_version` + `extra` keep this forward-compatible as the IR superset grows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSubmission {
    pub api_version: u32,
    pub chart: ChartId,
    pub player: PlayerId,
    pub mode: String,
    pub clear: ClearLamp,
    pub ex_score: u32,
    pub max_ex_score: u32,
    pub judge: JudgeBreakdown,
    pub max_combo: u32,
    pub total_notes: u32,
    /// Notes the run actually resolved (a failed run stops short of `total_notes`). Mirrors the
    /// reference implementation's `IRScoreData.passnotes`; defaults to 0 for a server or a client
    /// that predates the field.
    #[serde(default)]
    pub passnotes: u32,
    pub minbp: u32,
    pub gauge_value: f32,
    pub options: PlayOptions,
    pub played_at: i64,
    pub client: String,
    pub replay_id: Option<String>,
    /// RNG seed the shuffle used (lets a server reproduce the exact lane layout for verification).
    #[serde(default)]
    pub seed: u64,
    /// Reference implementation `JudgeAlgorithm` ("Combo"/"Duration"/"Lowest"/"Score"); empty = client default.
    #[serde(default)]
    pub judge_algorithm: String,
    /// Reference implementation `BMSPlayerRule` the run used; empty = client default.
    #[serde(default)]
    pub rule: String,
    /// Skin identifier the run used (presentation only; recorded for completeness).
    #[serde(default)]
    pub skin: String,
    /// SHA-256 of the running client binary, for build-integrity / ranked eligibility. `None` when
    /// the client could not hash itself (the server decides how to treat an unknown build).
    #[serde(default)]
    pub client_build_sha256: Option<String>,
    /// Client platform as `OS-ARCH` (e.g. `macos-aarch64`), pairing with the build hash.
    #[serde(default)]
    pub client_platform: Option<String>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Lowest `played_at` the server's schema accepts (`z.number().int().min(0)`); a clock that
/// reports a pre-epoch instant is clamped to it instead of drawing a 400.
pub const MIN_PLAYED_AT_MS: i64 = 0;

/// Gauge value substituted when a run reports a non-finite one. `f32::NAN` serialises to JSON
/// `null`, which the server's `z.number()` rejects (a zod default only fills in `undefined`).
pub const FALLBACK_GAUGE_VALUE: f32 = 0.0;

impl ScoreSubmission {
    /// Bring the two fields whose server schema is narrower than the Rust type into range, so a
    /// value that would come back as a 400 `VALIDATION_ERROR` never leaves the client.
    pub fn clamp_to_server_bounds(&mut self) {
        self.played_at = self.played_at.max(MIN_PLAYED_AT_MS);
        if !self.gauge_value.is_finite() {
            self.gauge_value = FALLBACK_GAUGE_VALUE;
        }
    }
}

/// One leaderboard row. The first block is the basic IR ranking payload; the trailing fields are
/// the superset extras a client needs to render reference-style ranking panels (`RankingData`
/// lamp histogram, per-row judge detail, option/LN-type badges). They mirror
/// the reference implementation's `IRScoreData` `lntype` / `notes` / `option` / `epg..lms` and all carry a serde
/// default, so a server that only speaks the basic payload still decodes unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreRecord {
    pub player: PlayerId,
    pub player_name: String,
    pub clear: ClearLamp,
    pub ex_score: u32,
    pub max_combo: u32,
    pub minbp: u32,
    pub rank: Option<u32>,
    pub played_at: i64,
    /// LN handling the run used (reference implementation `IRScoreData.lntype`: 0 = LN, 1 = CN, 2 = HCN).
    #[serde(default)]
    pub lntype: i32,
    /// Raw reference implementation option bitmask the run used (`IRScoreData.option`); 0 = unknown/none.
    #[serde(default)]
    pub option: i64,
    /// Chart note count the row was scored against (reference implementation `IRScoreData.notes`); 0 = unknown.
    #[serde(default)]
    pub total_notes: u32,
    /// Full judge tally for the row when the server exposes it; `None` = ranking-only payload.
    #[serde(default)]
    pub judge: Option<JudgeBreakdown>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A condition the server attached to a submission while deciding whether it counts for ranking.
/// The wire form is the server's `SCORE_FLAG` string; unknown future flags stay as raw strings in
/// [`SubmitResponse::flags`] rather than being dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScoreFlag {
    Autoplay,
    ScratchAuto,
    Assist,
    JudgeWidth,
    TotalOverride,
    UnknownBuild,
    Guest,
}

impl ScoreFlag {
    /// Every flag this client understands, in the server's declaration order.
    pub const ALL: [ScoreFlag; 7] = [
        ScoreFlag::Autoplay,
        ScoreFlag::ScratchAuto,
        ScoreFlag::Assist,
        ScoreFlag::JudgeWidth,
        ScoreFlag::TotalOverride,
        ScoreFlag::UnknownBuild,
        ScoreFlag::Guest,
    ];

    /// The exact string the server sends.
    pub const fn as_wire(self) -> &'static str {
        match self {
            ScoreFlag::Autoplay => "AUTOPLAY",
            ScoreFlag::ScratchAuto => "SCRATCH_AUTO",
            ScoreFlag::Assist => "ASSIST",
            ScoreFlag::JudgeWidth => "JUDGE_WIDTH",
            ScoreFlag::TotalOverride => "TOTAL_OVERRIDE",
            ScoreFlag::UnknownBuild => "UNKNOWN_BUILD",
            ScoreFlag::Guest => "GUEST",
        }
    }

    /// Recognise a wire string, or `None` when the server sent a flag this build predates.
    pub fn from_wire(flag: &str) -> Option<ScoreFlag> {
        ScoreFlag::ALL.into_iter().find(|known| known.as_wire() == flag)
    }

    /// Whether the flag alone keeps a score out of the ranking. `UnknownBuild` does not: the server
    /// only blocks an unrecognised build when it is configured to require a trusted one.
    pub const fn blocks_ranking(self) -> bool {
        !matches!(self, ScoreFlag::UnknownBuild)
    }

    /// Short player-facing reason, for the result screen.
    pub const fn describe(self) -> &'static str {
        match self {
            ScoreFlag::Autoplay => "autoplay",
            ScoreFlag::ScratchAuto => "auto scratch",
            ScoreFlag::Assist => "assist options",
            ScoreFlag::JudgeWidth => "widened judge",
            ScoreFlag::TotalOverride => "TOTAL override",
            ScoreFlag::UnknownBuild => "unrecognised client build",
            ScoreFlag::Guest => "guest submission",
        }
    }
}

impl std::fmt::Display for ScoreFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_wire())
    }
}

/// The server's answer to a score or course submission. Only `accepted`/`rank`/`previous_best`/
/// `message` are guaranteed; everything below carries a serde default so a server that predates a
/// field still decodes, and so the client can be rolled out ahead of the backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubmitResponse {
    pub accepted: bool,
    pub rank: Option<u32>,
    pub previous_best: Option<u32>,
    pub message: Option<String>,
    /// Whether the submission counts towards the leaderboard.
    #[serde(default)]
    pub ranked: bool,
    /// Raw `SCORE_FLAG` strings the server attached; see [`SubmitResponse::unranked_reasons`].
    #[serde(default)]
    pub flags: Vec<String>,
    /// Whether this submission became the player's best on the chart.
    #[serde(default)]
    pub is_new_best: bool,
    /// Server-side id of the stored score, used to link a replay upload to it.
    #[serde(default)]
    pub score_id: Option<String>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl SubmitResponse {
    /// The flags this build recognises, in the order the server sent them.
    pub fn known_flags(&self) -> Vec<ScoreFlag> {
        self.flags.iter().filter_map(|flag| ScoreFlag::from_wire(flag)).collect()
    }

    /// Flags this build does not recognise, kept so the UI can still show them.
    pub fn unknown_flags(&self) -> Vec<&str> {
        self.flags.iter().filter(|flag| ScoreFlag::from_wire(flag).is_none()).map(String::as_str).collect()
    }

    /// The recognised flags that keep the score out of the ranking.
    pub fn unranked_reasons(&self) -> Vec<ScoreFlag> {
        self.known_flags().into_iter().filter(|flag| flag.blocks_ranking()).collect()
    }

    /// True when the server reported this exact flag.
    pub fn has_flag(&self, flag: ScoreFlag) -> bool {
        self.flags.iter().any(|raw| raw == flag.as_wire())
    }

    /// One line naming why the score is unranked, or `None` when nothing blocks it.
    pub fn unranked_summary(&self) -> Option<String> {
        let reasons = self.unranked_reasons();
        if reasons.is_empty() {
            return None;
        }
        Some(reasons.iter().map(|flag| flag.describe()).collect::<Vec<_>>().join(", "))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseSubmission {
    pub api_version: u32,
    pub course_hash: String,
    pub player: PlayerId,
    pub clear: ClearLamp,
    pub ex_score: u32,
    pub judge: JudgeBreakdown,
    pub max_combo: u32,
    pub gauge_value: f32,
    pub charts: Vec<ChartId>,
    pub played_at: i64,
    /// LN handling the course ran under (0 = LN, 1 = CN, 2 = HCN).
    #[serde(default)]
    pub lntype: i32,
    /// Theoretical maximum EX over the whole course; 0 = not reported.
    #[serde(default)]
    pub max_ex_score: u32,
    #[serde(default)]
    pub minbp: u32,
    /// Course trophy the run earned, when the constraint set awards one.
    #[serde(default)]
    pub trophy: Option<String>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CourseSubmission {
    /// The same clamp as [`ScoreSubmission::clamp_to_server_bounds`]: the course schema narrows
    /// `played_at` and `gauge_value` in exactly the same way.
    pub fn clamp_to_server_bounds(&mut self) {
        self.played_at = self.played_at.max(MIN_PLAYED_AT_MS);
        if !self.gauge_value.is_finite() {
            self.gauge_value = FALLBACK_GAUGE_VALUE;
        }
    }
}

#[cfg(test)]
mod tests;

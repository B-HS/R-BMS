use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Identifies a chart. BMS IR keys on MD5; rbms additionally carries SHA-256 (superset).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChartId {
    pub md5: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerId {
    pub id: String,
}

/// Clear lamp — superset of the standard IR lamp set.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClearLamp {
    NoPlay,
    Failed,
    AssistEasy,
    LightAssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    FullCombo,
    Perfect,
    Max,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GaugeType {
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    Hazard,
    Class,
    ExClass,
    ExHardClass,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RandomOption {
    Off,
    Mirror,
    Random,
    RRandom,
    SRandom,
    Spiral,
    HRandom,
    AllScratch,
    Converge,
}

/// Per-judge tally. The `pgreat`..`miss` totals are the basic IR fields; `fast`/`slow`/`combobreak`
/// and the `e*`/`l*` early/late split (beatoraja `IRScoreData`'s 12 fields, `epg`..`lms`), plus
/// `avgjudge` (mean signed timing, µs) and rbms-only `empty_poor`, are the superset extras. The
/// split is additive: `epg + lpg == pgreat`, etc. Every superset field has a serde default so older
/// (split-less) submissions still decode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JudgeBreakdown {
    pub pgreat: u32,
    pub great: u32,
    pub good: u32,
    pub bad: u32,
    pub poor: u32,
    pub miss: u32,
    pub fast: u32,
    pub slow: u32,
    pub combobreak: u32,
    #[serde(default)]
    pub epg: u32,
    #[serde(default)]
    pub lpg: u32,
    #[serde(default)]
    pub egr: u32,
    #[serde(default)]
    pub lgr: u32,
    #[serde(default)]
    pub egd: u32,
    #[serde(default)]
    pub lgd: u32,
    #[serde(default)]
    pub ebd: u32,
    #[serde(default)]
    pub lbd: u32,
    #[serde(default)]
    pub epr: u32,
    #[serde(default)]
    pub lpr: u32,
    #[serde(default)]
    pub ems: u32,
    #[serde(default)]
    pub lms: u32,
    #[serde(default)]
    pub avgjudge: i64,
    #[serde(default)]
    pub empty_poor: u32,
}

/// How a chart was played. The first block is basic IR; the rest is the rbms superset that lets a
/// server evaluate fairness exactly (every modifier that affects difficulty is preserved). `option`
/// keeps beatoraja's raw option bitmask for round-tripping. All superset fields carry a serde
/// default so older submissions still decode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayOptions {
    pub gauge: GaugeType,
    pub random: RandomOption,
    pub random_p2: Option<RandomOption>,
    pub scratch_auto: bool,
    pub lntype: i32,
    pub input_device: String,
    pub assist: Vec<String>,
    #[serde(default)]
    pub option: i64,
    #[serde(default)]
    pub judge_rate: i32,
    #[serde(default)]
    pub offset_ms: i32,
    #[serde(default)]
    pub constant: bool,
    #[serde(default)]
    pub hispeed: f64,
    #[serde(default)]
    pub lift: f32,
    #[serde(default)]
    pub lane_cover: f32,
    #[serde(default)]
    pub total_override: f64,
    #[serde(default)]
    pub autoplay: bool,
    #[serde(default)]
    pub auto_offset: bool,
    #[serde(default)]
    pub scratch_left: bool,
    #[serde(default)]
    pub green_number: f64,
}

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
    pub minbp: u32,
    pub gauge_value: f32,
    pub options: PlayOptions,
    pub played_at: i64,
    pub client: String,
    pub replay_id: Option<String>,
    /// RNG seed the shuffle used (lets a server reproduce the exact lane layout for verification).
    #[serde(default)]
    pub seed: u64,
    /// beatoraja `JudgeAlgorithm` ("Combo"/"Duration"/"Lowest"/"Score"); empty = client default.
    #[serde(default)]
    pub judge_algorithm: String,
    /// beatoraja `BMSPlayerRule` the run used; empty = client default.
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
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResponse {
    pub accepted: bool,
    pub rank: Option<u32>,
    pub previous_best: Option<u32>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub total_plays: u64,
    pub rank_points: f64,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub ranking: bool,
    pub player_best: bool,
    pub rivals: bool,
    pub courses: bool,
    pub replays: bool,
    pub tables: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub ir_compat: String,
    #[serde(default)]
    pub capabilities: ServerCapabilities,
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
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A single µs-resolution input event in a replay: lane press/release at an absolute song time.
/// Lossless (µs), so the server-side ghost/analysis can reproduce timing exactly — unlike a byte
/// stream, this survives re-encoding and is self-describing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayEvent {
    pub t_us: i64,
    pub lane: u32,
    pub press: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayData {
    pub format: String,
    pub events: Vec<ReplayEvent>,
    pub seed: Option<u64>,
}

/// A named settings blob (settings/keyconfig/tables …) for account-side sync. `content` is the raw
/// serialised body (RON/JSON) the client wrote; the server stores it opaquely per `(player, name)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBlob {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub updated_at: i64,
}

/// Register/login request. Superset of beatoraja `IRAccount{id,password,name}` with an optional
/// `email`. `name` is used on register; ignored on login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub id: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Successful auth: a bearer token plus the resolved player identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub player: PlayerId,
    pub name: String,
}

//! The vocabulary the JUDGE tab's rows are stored under: one token per engine value, plus the
//! label each row shows.
//!
//! Every token is uppercase and free of separators so it survives a round trip through the
//! configuration file and the account sync blob, exactly like the gauge token next door. The
//! labels are what the settings screen prints and may carry spaces.

use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::GaugeAutoShift;
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;

use crate::settings::AUTO_VALUE;

/// Names the JUDGE ALGORITHM row shows, one per entry of [`JudgeAlgorithm::ALL`].
pub const JUDGE_ALGORITHM_LABELS: &[&str] = &["COMBO", "DURATION", "LOWEST", "SCORE"];

/// Names the LN MODE row shows, one per entry of [`LnMode::ALL`].
pub const LN_MODE_LABELS: &[&str] = &["LN", "CN", "HCN"];

/// Names the GAUGE AUTO SHIFT row shows, one per entry of [`GaugeAutoShift::ALL`].
pub const GAUGE_AUTO_SHIFT_LABELS: &[&str] = &["NONE", "CONTINUE", "SURVIVAL TO GROOVE", "BEST CLEAR", "SELECT TO UNDER"];

/// The gauge tables the GAUGE SET row offers: the one the chart's mode selects, or the LR2 table
/// the reference reaches only through a course constraint and this engine exposes as a choice.
pub const GAUGE_SET_CYCLE: [Option<GaugeSetId>; 2] = [None, Some(GaugeSetId::Lr2)];

/// Names the GAUGE SET row shows, one per entry of [`GAUGE_SET_CYCLE`].
pub const GAUGE_SET_LABELS: &[&str] = &[AUTO_VALUE, "LR2"];

/// How many targets the TARGET row steps through: the reference's eleven fixed rates
/// (`TargetProperty.java:117-141`) plus the next rank up and the three that name somebody's score.
pub const TARGET_COUNT: usize = 15;

/// What the play screen paces the run against.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScoreTarget {
    /// A theoretical all-PGREAT run.
    Max,
    /// The EX a DJ rank asks for, at each third of each band the reference names
    /// (`TargetProperty.java:117-141`).
    RateAMinus,
    RateA,
    RateAPlus,
    RateAaMinus,
    RateAa,
    RateAaPlus,
    RateAaaMinus,
    #[default]
    RateAaa,
    RateAaaPlus,
    RateMaxMinus,
    /// The EX the next DJ rank up needs.
    RankNext,
    /// The best local record on this chart.
    LocalBest,
    /// The best score the score server holds for this account.
    IrBest,
    /// A rival's score on this chart.
    Rival,
}

impl ScoreTarget {
    /// Every target, in the order the row steps through them.
    pub const ALL: [ScoreTarget; TARGET_COUNT] = [
        ScoreTarget::Max,
        ScoreTarget::RateAMinus,
        ScoreTarget::RateA,
        ScoreTarget::RateAPlus,
        ScoreTarget::RateAaMinus,
        ScoreTarget::RateAa,
        ScoreTarget::RateAaPlus,
        ScoreTarget::RateAaaMinus,
        ScoreTarget::RateAaa,
        ScoreTarget::RateAaaPlus,
        ScoreTarget::RateMaxMinus,
        ScoreTarget::RankNext,
        ScoreTarget::LocalBest,
        ScoreTarget::IrBest,
        ScoreTarget::Rival,
    ];

    /// Name the TARGET row shows.
    pub fn label(self) -> &'static str {
        match self {
            ScoreTarget::Max => "MAX",
            ScoreTarget::RateAMinus => "RATE A-",
            ScoreTarget::RateA => "RATE A",
            ScoreTarget::RateAPlus => "RATE A+",
            ScoreTarget::RateAaMinus => "RATE AA-",
            ScoreTarget::RateAa => "RATE AA",
            ScoreTarget::RateAaPlus => "RATE AA+",
            ScoreTarget::RateAaaMinus => "RATE AAA-",
            ScoreTarget::RateAaa => "RATE AAA",
            ScoreTarget::RateAaaPlus => "RATE AAA+",
            ScoreTarget::RateMaxMinus => "RATE MAX-",
            ScoreTarget::RankNext => "RANK NEXT",
            ScoreTarget::LocalBest => "LOCAL BEST",
            ScoreTarget::IrBest => "IR BEST",
            ScoreTarget::Rival => "RIVAL",
        }
    }

    /// Settings-file token for a target. Separator-free, like every other token here, so it
    /// survives a round trip through the settings file and the account sync blob.
    pub fn token(self) -> &'static str {
        match self {
            ScoreTarget::Max => "MAX",
            ScoreTarget::RateAMinus => "RATEAMINUS",
            ScoreTarget::RateA => "RATEA",
            ScoreTarget::RateAPlus => "RATEAPLUS",
            ScoreTarget::RateAaMinus => "RATEAAMINUS",
            ScoreTarget::RateAa => "RATEAA",
            ScoreTarget::RateAaPlus => "RATEAAPLUS",
            ScoreTarget::RateAaaMinus => "RATEAAAMINUS",
            ScoreTarget::RateAaa => "RATEAAA",
            ScoreTarget::RateAaaPlus => "RATEAAAPLUS",
            ScoreTarget::RateMaxMinus => "RATEMAXMINUS",
            ScoreTarget::RankNext => "RANKNEXT",
            ScoreTarget::LocalBest => "LOCALBEST",
            ScoreTarget::IrBest => "IRBEST",
            ScoreTarget::Rival => "RIVAL",
        }
    }

    /// The id the reference stores this fixed rate under (`TargetProperty.java:117-141`), or `None`
    /// for a target that names a score rather than a rate. It is what the pacer looks the rate up
    /// by, so the two tables cannot drift into naming different bands.
    pub fn rate_id(self) -> Option<&'static str> {
        match self {
            ScoreTarget::Max => Some("MAX"),
            ScoreTarget::RateAMinus => Some("RATE_A-"),
            ScoreTarget::RateA => Some("RATE_A"),
            ScoreTarget::RateAPlus => Some("RATE_A+"),
            ScoreTarget::RateAaMinus => Some("RATE_AA-"),
            ScoreTarget::RateAa => Some("RATE_AA"),
            ScoreTarget::RateAaPlus => Some("RATE_AA+"),
            ScoreTarget::RateAaaMinus => Some("RATE_AAA-"),
            ScoreTarget::RateAaa => Some("RATE_AAA"),
            ScoreTarget::RateAaaPlus => Some("RATE_AAA+"),
            ScoreTarget::RateMaxMinus => Some("RATE_MAX-"),
            ScoreTarget::RankNext | ScoreTarget::LocalBest | ScoreTarget::IrBest | ScoreTarget::Rival => None,
        }
    }
}

/// Names the TARGET row shows, one per entry of [`ScoreTarget::ALL`].
pub const TARGET_LABELS: &[&str] = &[
    "MAX",
    "RATE A-",
    "RATE A",
    "RATE A+",
    "RATE AA-",
    "RATE AA",
    "RATE AA+",
    "RATE AAA-",
    "RATE AAA",
    "RATE AAA+",
    "RATE MAX-",
    "RANK NEXT",
    "LOCAL BEST",
    "IR BEST",
    "RIVAL",
];

/// Parse a persisted target token, defaulting to the shipped target.
pub fn target_from_token(token: &str) -> ScoreTarget {
    let upper = token.to_ascii_uppercase();
    ScoreTarget::ALL.into_iter().find(|t| t.token() == upper).unwrap_or_default()
}

/// Settings-file token for a judge algorithm. The reference implementation's own constant name, so
/// a settings blob reads the same on both.
pub fn algorithm_token(algorithm: JudgeAlgorithm) -> &'static str {
    algorithm.name()
}

/// Parse a persisted algorithm token, defaulting to the engine's own default.
pub fn algorithm_from_token(token: &str) -> JudgeAlgorithm {
    JudgeAlgorithm::ALL.into_iter().find(|a| a.name().eq_ignore_ascii_case(token)).unwrap_or_default()
}

/// Settings-file token for a long-note flavour.
pub fn ln_mode_token(mode: LnMode) -> &'static str {
    match mode {
        LnMode::LongNote => "LN",
        LnMode::ChargeNote => "CN",
        LnMode::HellChargeNote => "HCN",
    }
}

/// Parse a persisted long-note flavour token, defaulting to plain long notes.
pub fn ln_mode_from_token(token: &str) -> LnMode {
    LnMode::ALL.into_iter().find(|m| ln_mode_token(*m).eq_ignore_ascii_case(token)).unwrap_or_default()
}

/// Settings-file token for a gauge auto-shift mode.
pub fn gauge_auto_shift_token(shift: GaugeAutoShift) -> &'static str {
    match shift {
        GaugeAutoShift::None => "NONE",
        GaugeAutoShift::Continue => "CONTINUE",
        GaugeAutoShift::SurvivalToGroove => "SURVIVALTOGROOVE",
        GaugeAutoShift::BestClear => "BESTCLEAR",
        GaugeAutoShift::SelectToUnder => "SELECTTOUNDER",
    }
}

/// Parse a persisted auto-shift token, defaulting to no shifting.
pub fn gauge_auto_shift_from_token(token: &str) -> GaugeAutoShift {
    GaugeAutoShift::ALL.into_iter().find(|s| gauge_auto_shift_token(*s).eq_ignore_ascii_case(token)).unwrap_or_default()
}

/// Settings-file token for a gauge table, where `None` means "the one the chart's mode selects".
pub fn gauge_set_token(set: Option<GaugeSetId>) -> &'static str {
    match set {
        None => AUTO_VALUE,
        Some(set) => set.data_key(),
    }
}

/// Parse a persisted gauge-table token. Anything the table does not name — including the mode keys
/// a chart selects for itself — reads as "follow the chart's mode".
pub fn gauge_set_from_token(token: &str) -> Option<GaugeSetId> {
    GAUGE_SET_CYCLE.into_iter().find(|set| gauge_set_token(*set).eq_ignore_ascii_case(token)).unwrap_or_default()
}

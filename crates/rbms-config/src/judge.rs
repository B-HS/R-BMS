//! The vocabulary the JUDGE tab's rows are stored under: one token per engine value, plus the
//! label each row shows.
//!
//! Every token is uppercase and free of separators so it survives a round trip through the
//! configuration file and the account sync blob, exactly like the gauge token next door. The
//! labels are what the settings screen prints and may carry spaces.

use std::borrow::Cow;

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
/// (`TargetProperty.java:117-141`) plus the next rank up, the rate the player types, and the three
/// that name somebody's score.
pub const TARGET_COUNT: usize = 16;

/// How many targets the TARGET FALLBACK row steps through: the ones that need nothing but the chart
/// (and, for the next rank, the player's own record), so they are always worked out.
pub const TARGET_FALLBACK_COUNT: usize = 13;

/// What the names of the fixed rates start with.
const RATE_PREFIX: &str = "RATE ";

/// What the local record is called where a row has room for no more than a word of it.
const OWN_RECORD_SHORT_NAME: &str = "LOCAL";

/// What the reference calls a share the player typed, before the share (`TargetProperty.java:141`).
const CUSTOM_RATE_NAME: &str = "SCORE RATE";

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
    /// A share of the chart's ceiling the player chose with the TARGET RATE row, which is the
    /// reference's `RATE_<number>` target (`TargetProperty.java:136-146`).
    RateCustom,
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
        ScoreTarget::RateCustom,
        ScoreTarget::LocalBest,
        ScoreTarget::IrBest,
        ScoreTarget::Rival,
    ];

    /// The targets that are always worked out, which are the ones the TARGET FALLBACK row offers: the
    /// reference's eleven fixed rates and the next rank. Every other target names somebody's score
    /// and answers only once that score is known.
    pub const FALLBACKS: [ScoreTarget; TARGET_FALLBACK_COUNT] = [
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
        ScoreTarget::RateCustom,
    ];

    /// Whether the target is worked out from the chart alone, so it can stand in for one that is
    /// not known.
    pub fn is_always_known(self) -> bool {
        ScoreTarget::FALLBACKS.contains(&self)
    }

    /// What is missing when the target cannot be worked out, as the screens say it after the
    /// target's name, or `None` for a target that is always known.
    pub fn missing_score(self) -> Option<&'static str> {
        match self {
            ScoreTarget::LocalBest => Some("NO RECORD"),
            ScoreTarget::IrBest => Some("NO IR SCORE"),
            ScoreTarget::Rival => Some("NO RIVAL SCORE"),
            _ => None,
        }
    }

    /// The target's name as a screen writes it once the share of a custom rate is known:
    /// `SCORE RATE 85%` for RATE CUSTOM at 85, the row's own name for every other target.
    pub fn name_at(self, rate_percent: i32) -> Cow<'static, str> {
        match self {
            ScoreTarget::RateCustom => Cow::Owned(format!("{CUSTOM_RATE_NAME} {rate_percent}%")),
            other => Cow::Borrowed(other.label()),
        }
    }

    /// The target's name without the word the rates share, for a place with little room: `AAA+` for
    /// `RATE AAA+`, and the share itself (`85%`) for RATE CUSTOM.
    pub fn short_name_at(self, rate_percent: i32) -> Cow<'static, str> {
        match self {
            ScoreTarget::RateCustom => Cow::Owned(format!("{rate_percent}%")),
            other => Cow::Borrowed(other.label().strip_prefix(RATE_PREFIX).unwrap_or(other.label())),
        }
    }

    /// The target as a row shows it beside the target it stands in for when its own score is not
    /// there: `LOCAL BEST (NO RECORD: AAA)`. A target that is always known is its name alone.
    pub fn label_with_fallback(self, stand_in: ScoreTarget, rate_percent: i32) -> String {
        match self.missing_score() {
            Some(missing) => format!("{} ({missing}: {})", self.label(), stand_in.short_name_at(rate_percent)),
            None => self.label().to_string(),
        }
    }

    /// The target the player's own record is tried as before `stand_in`, when this one's score is not
    /// there: the account's best on the score server is the player's own best, so the local record
    /// is the same kind of number and comes next. No other target has one.
    pub fn own_record_first(self) -> Option<ScoreTarget> {
        (self == ScoreTarget::IrBest).then_some(ScoreTarget::LocalBest)
    }

    /// The target for a row with little room, beside what stands in for it where its score is not
    /// there, in the order they are tried: `LOCAL BEST > AAA`, `IR BEST > LOCAL > AAA`. A target
    /// that is always known is its name alone.
    pub fn label_then(self, stand_in: ScoreTarget, rate_percent: i32) -> String {
        let stand_in = stand_in.short_name_at(rate_percent);
        match (self.missing_score(), self.own_record_first()) {
            (Some(_), Some(_)) => format!("{} > {OWN_RECORD_SHORT_NAME} > {stand_in}", self.label()),
            (Some(_), None) => format!("{} > {stand_in}", self.label()),
            (None, _) => self.label().to_string(),
        }
    }

    /// What the play menu says under the TARGET row about a target that may have no score, or `None`
    /// for one that is always known: what is missing and what the run is paced against then, the
    /// player's own record first where it is tried first.
    pub fn missing_note(self, stand_in: ScoreTarget, rate_percent: i32) -> Option<String> {
        let stand_in = stand_in.name_at(rate_percent);
        let missing = self.missing_score()?;
        Some(match self.own_record_first() {
            Some(own) => format!("{missing}: YOUR {}, THEN {stand_in}", own.label()),
            None => format!("{missing}: PACED AGAINST {stand_in} INSTEAD"),
        })
    }

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
            ScoreTarget::RateCustom => "RATE CUSTOM",
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
            ScoreTarget::RateCustom => "RATECUSTOM",
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
            ScoreTarget::RankNext | ScoreTarget::RateCustom | ScoreTarget::LocalBest | ScoreTarget::IrBest | ScoreTarget::Rival => None,
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
    "RATE CUSTOM",
    "LOCAL BEST",
    "IR BEST",
    "RIVAL",
];

/// Names the TARGET FALLBACK row shows, one per entry of [`ScoreTarget::FALLBACKS`].
pub const TARGET_FALLBACK_LABELS: &[&str] =
    &["MAX", "RATE A-", "RATE A", "RATE A+", "RATE AA-", "RATE AA", "RATE AA+", "RATE AAA-", "RATE AAA", "RATE AAA+", "RATE MAX-", "RANK NEXT", "RATE CUSTOM"];

/// A target token or label with the separators dropped and the signs spelled out, so a label as the
/// row prints it (`LOCAL BEST`, `RATE AAA+`) reads as the token it is stored under.
fn target_key(text: &str) -> String {
    text.trim()
        .chars()
        .filter_map(|c| match c {
            '+' => Some("PLUS".to_string()),
            '-' => Some("MINUS".to_string()),
            c if c.is_ascii_alphanumeric() => Some(c.to_ascii_uppercase().to_string()),
            _ => None,
        })
        .collect()
}

/// Parse a persisted target token, defaulting to the shipped target.
///
/// The label a row prints reads as its token too: a file edited by hand or written by another build
/// that stored `LOCAL BEST` is the local best, not the shipped target.
pub fn target_from_token(token: &str) -> ScoreTarget {
    let key = target_key(token);
    ScoreTarget::ALL.into_iter().find(|t| t.token() == key).unwrap_or_default()
}

/// Parse a persisted fallback token. Only a target that is always known can stand in for another,
/// so any other reads as the shipped target.
pub fn target_fallback_from_token(token: &str) -> ScoreTarget {
    let target = target_from_token(token);
    if target.is_always_known() { target } else { ScoreTarget::default() }
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

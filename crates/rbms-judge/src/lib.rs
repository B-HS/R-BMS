#![forbid(unsafe_code)]

pub mod algorithm;
pub mod data;
pub mod gauge;
pub mod gauge_tables;
pub mod ln;
pub mod matcher;
pub mod windows;

pub use algorithm::{JudgeAlgorithm, NoteRef, NoteType};
pub use data::{
    GaugeModifier, GaugeParams, GaugeSet, GaugeTables, JudgeDataError, JudgePropertyData, JudgeTables, JudgeWindowsData, builtin_gauge_tables,
    builtin_judge_tables, load_gauge_tables, load_judge_tables,
};
pub use gauge::{ClearType, Gauge, GaugeKind, clear_lamp, clear_type_from_id, clear_type_id};
pub use matcher::{JudgeEngine, JudgeResult};
pub use windows::{JudgeProperty, JudgeWindows, MissCondition, judgerank_for, rank_to_judgerank};

/// A single judgment outcome, in the reference implementation's judge-code order. `Poor` is index 4 (見逃し POOR — a
/// note that went by unhit) and `Miss` is index 5 (空POOR — a press that reached only the MS band
/// and consumed no note). Combo behaviour per index comes from the mode's
/// [`JudgeProperty::combo`](windows::JudgeProperty::combo) table. EX score = 2·PG + 1·GR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judge {
    PerfectGreat,
    Great,
    Good,
    Bad,
    Poor,
    Miss,
}

#[cfg(test)]
mod tests;

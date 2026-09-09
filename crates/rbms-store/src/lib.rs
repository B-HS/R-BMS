//! Persistence for the player's local play history and replays.
//!
//! Owns the on-disk shape of `scores.ron` and `replays/*.ron`, the durable-write helper every
//! other store file goes through, and the judging-rule generation stamped on each record. The
//! crate deliberately depends on serde/RON only: lamps and judgments travel as numeric ids so the
//! judging engine stays out of the persistence layer.

#![forbid(unsafe_code)]

mod atomic;
mod error;
mod replay;
mod score;

#[cfg(test)]
mod tests;

pub use atomic::write_atomic;
pub use error::StoreError;
pub use replay::{Replay, ReplayEvent};
pub use score::{SCORE_RULE_VERSION, ScoreBook, ScoreRecord, is_stale_rule_version};

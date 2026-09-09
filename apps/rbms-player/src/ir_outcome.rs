//! What the IR did with the run just played, rendered as the short lines the result screen shows.
//! Pure formatting over [`rbms_ir::SubmitOutcome`], so every branch is unit-testable without a
//! server.

use rbms_ir::{IrError, SubmitOutcome};
use rbms_render::Color;

/// Where the IR block sits on the result screen: the left column, below the EX-delta lines the
/// rank bar draws, one row per status line.
pub(crate) const IR_RESULT_X: f32 = 44.0;
pub(crate) const IR_RESULT_Y: f32 = 470.0;
pub(crate) const IR_RESULT_LINE_H: f32 = 30.0;
pub(crate) const IR_RESULT_SCALE: f32 = 1.8;

/// Longest server/error text kept in a result line before it is cut with [`TRUNCATION_MARK`].
/// Long enough for a sentence, short enough to stay inside the result panel column.
pub(crate) const IR_LINE_MAX_CHARS: usize = 56;

/// Appended when a message is cut to [`IR_LINE_MAX_CHARS`]. ASCII so it renders in every bundled
/// font.
pub(crate) const TRUNCATION_MARK: &str = "...";

/// How a result line should read: neutral, a win, a caveat, or a failure. The result screen maps
/// these onto theme colours, so the text and the colouring stay in one place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IrLineKind {
    Info,
    Good,
    Warn,
    Bad,
}

/// The IR half of the result screen.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum IrStatus {
    /// No score server configured.
    Off,
    /// The run was never eligible (autoplay, replay playback, widened judge window, scratch assist).
    Skipped(&'static str),
    /// Submitted; waiting on the worker.
    Sending,
    /// The worker reported back.
    Reported(IrReport),
}

impl IrStatus {
    /// Whether a finished submission may overwrite this status.
    ///
    /// Only a run that actually submitted is waiting for a report. A run that was skipped, or that
    /// played with no server configured, must not be relabelled by an *earlier* run's submission
    /// landing late — the result screen would then claim a rank for a score that was never sent.
    pub(crate) fn accepts_report(&self) -> bool {
        matches!(self, IrStatus::Sending)
    }
}

/// The three facts a finished submission can report: what happened to the score, whether it beat
/// the stored best, and what happened to the replay upload that rode along with it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct IrReport {
    pub(crate) score: String,
    pub(crate) new_best: bool,
    pub(crate) replay: Option<String>,
    pub(crate) failed: bool,
}

impl IrStatus {
    /// The lines to draw, top to bottom, each with the tone it should be drawn in.
    pub(crate) fn lines(&self) -> Vec<(String, IrLineKind)> {
        match self {
            IrStatus::Off => vec![("IR: OFF".to_string(), IrLineKind::Info)],
            IrStatus::Skipped(reason) => {
                vec![(format!("IR: SKIPPED ({reason})"), IrLineKind::Info)]
            }
            IrStatus::Sending => vec![("IR: SENDING...".to_string(), IrLineKind::Info)],
            IrStatus::Reported(report) => {
                let score_kind = if report.failed {
                    IrLineKind::Bad
                } else if report.score.starts_with("UNRANKED") {
                    IrLineKind::Warn
                } else {
                    IrLineKind::Info
                };
                let mut out = vec![(report.score.clone(), score_kind)];
                if report.new_best {
                    out.push(("NEW BEST".to_string(), IrLineKind::Good));
                }
                if let Some(replay) = &report.replay {
                    let kind = if replay.starts_with("REPLAY ERROR") { IrLineKind::Bad } else { IrLineKind::Good };
                    out.push((replay.clone(), kind));
                }
                out
            }
        }
    }
}

/// Colour a result line is drawn in.
pub(crate) fn ir_line_color(kind: IrLineKind) -> Color {
    match kind {
        IrLineKind::Info => rbms_render::theme().text_dim,
        IrLineKind::Good => Color::GREEN,
        IrLineKind::Warn => Color::YELLOW,
        IrLineKind::Bad => Color::RED,
    }
}

/// One-line rendering of an [`IrError`] for a status line: its `Display` form, cut to
/// [`IR_LINE_MAX_CHARS`].
pub(crate) fn short_error(error: &IrError) -> String {
    truncate(&error.to_string())
}

/// Cut `text` to [`IR_LINE_MAX_CHARS`] characters (not bytes), marking it when anything was
/// dropped. Whitespace is collapsed so a multi-line server message stays on one row.
pub(crate) fn truncate(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= IR_LINE_MAX_CHARS {
        return flat;
    }
    let head: String = flat.chars().take(IR_LINE_MAX_CHARS).collect();
    format!("{head}{TRUNCATION_MARK}")
}

/// Turn a finished submit worker hand-off into the report the result screen shows.
///
/// The score line is the single most important fact, in priority order: a transport/server failure,
/// a rejection, a submission the server kept out of the ranking (with its reasons), the placement,
/// then a bare acknowledgement. `NEW BEST` and the replay-upload outcome are reported alongside it.
pub(crate) fn format_submit_outcome(outcome: &SubmitOutcome) -> IrReport {
    let (score, failed) = match &outcome.submit {
        Err(error) => (format!("IR ERROR: {}", short_error(error)), true),
        Ok(response) if !response.accepted => {
            let line = match response.message.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
                Some(message) => format!("IR: REJECTED ({})", truncate(message)),
                None => "IR: REJECTED".to_string(),
            };
            (line, true)
        }
        Ok(response) => {
            let line = match response.unranked_summary() {
                Some(reasons) => format!("UNRANKED: {}", truncate(&reasons)),
                None => match response.rank {
                    Some(rank) => format!("IR: RANK #{rank}"),
                    None => "IR: SENT".to_string(),
                },
            };
            (line, false)
        }
    };
    let new_best = matches!(&outcome.submit, Ok(response) if response.accepted && response.is_new_best);
    let replay = outcome.replay.as_ref().map(|result| match result {
        Ok(_) => "REPLAY UPLOADED".to_string(),
        Err(error) => format!("REPLAY ERROR: {}", short_error(error)),
    });
    IrReport { score, new_best, replay, failed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_ir::{ScoreFlag, SubmitResponse};

    fn accepted(rank: Option<u32>) -> SubmitResponse {
        SubmitResponse { accepted: true, rank, ..Default::default() }
    }

    fn reported(outcome: SubmitOutcome) -> Vec<String> {
        IrStatus::Reported(format_submit_outcome(&outcome)).lines().into_iter().map(|(text, _)| text).collect()
    }

    #[test]
    fn each_line_kind_has_its_own_colour() {
        let colors = [IrLineKind::Info, IrLineKind::Good, IrLineKind::Warn, IrLineKind::Bad].map(ir_line_color);
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "status tones are visually distinct");
            }
        }
    }

    #[test]
    fn offline_and_skipped_and_sending_render_their_own_line() {
        assert_eq!(IrStatus::Off.lines(), vec![("IR: OFF".to_string(), IrLineKind::Info)]);
        assert_eq!(IrStatus::Sending.lines(), vec![("IR: SENDING...".to_string(), IrLineKind::Info)]);
        assert_eq!(IrStatus::Skipped("autoplay").lines(), vec![("IR: SKIPPED (autoplay)".to_string(), IrLineKind::Info)]);
    }

    #[test]
    fn accepted_with_a_rank_reports_the_placement() {
        let lines = reported(SubmitOutcome { submit: Ok(accepted(Some(3))), replay: None });
        assert_eq!(lines, vec!["IR: RANK #3".to_string()]);
    }

    #[test]
    fn accepted_without_a_rank_reports_a_bare_acknowledgement() {
        let lines = reported(SubmitOutcome { submit: Ok(accepted(None)), replay: None });
        assert_eq!(lines, vec!["IR: SENT".to_string()]);
    }

    #[test]
    fn a_new_best_adds_its_own_line_under_the_placement() {
        let response = SubmitResponse { accepted: true, rank: Some(1), is_new_best: true, ..Default::default() };
        let lines = reported(SubmitOutcome { submit: Ok(response), replay: None });
        assert_eq!(lines, vec!["IR: RANK #1".to_string(), "NEW BEST".to_string()]);
    }

    #[test]
    fn unranked_flags_replace_the_placement_and_name_every_blocking_reason() {
        let response = SubmitResponse {
            accepted: true,
            rank: Some(9),
            flags: vec![ScoreFlag::Assist.as_wire().to_string(), ScoreFlag::Guest.as_wire().to_string()],
            ..Default::default()
        };
        let report = format_submit_outcome(&SubmitOutcome { submit: Ok(response), replay: None });
        assert!(report.score.starts_with("UNRANKED: "), "{}", report.score);
        assert!(report.score.contains(ScoreFlag::Assist.describe()));
        assert!(report.score.contains(ScoreFlag::Guest.describe()));
        assert!(!report.failed, "an unranked score still reached the server");
        assert_eq!(IrStatus::Reported(report).lines()[0].1, IrLineKind::Warn);
    }

    #[test]
    fn a_non_blocking_flag_leaves_the_placement_alone() {
        let response = SubmitResponse { accepted: true, rank: Some(4), flags: vec![ScoreFlag::UnknownBuild.as_wire().to_string()], ..Default::default() };
        assert!(!ScoreFlag::UnknownBuild.blocks_ranking());
        let lines = reported(SubmitOutcome { submit: Ok(response), replay: None });
        assert_eq!(lines, vec!["IR: RANK #4".to_string()]);
    }

    #[test]
    fn a_rejection_reports_the_server_message_when_there_is_one() {
        let response = SubmitResponse { accepted: false, message: Some("chart unknown".into()), ..Default::default() };
        let report = format_submit_outcome(&SubmitOutcome { submit: Ok(response), replay: None });
        assert_eq!(report.score, "IR: REJECTED (chart unknown)");
        assert!(report.failed);
        assert_eq!(IrStatus::Reported(report).lines()[0].1, IrLineKind::Bad);
    }

    #[test]
    fn a_rejection_without_a_message_is_still_reported() {
        let response = SubmitResponse { accepted: false, message: Some("   ".into()), ..Default::default() };
        let report = format_submit_outcome(&SubmitOutcome { submit: Ok(response), replay: None });
        assert_eq!(report.score, "IR: REJECTED");
    }

    #[test]
    fn a_rejected_submission_never_claims_a_new_best() {
        let response = SubmitResponse { accepted: false, is_new_best: true, ..Default::default() };
        let report = format_submit_outcome(&SubmitOutcome { submit: Ok(response), replay: None });
        assert!(!report.new_best);
    }

    #[test]
    fn a_transport_failure_reports_the_error() {
        let outcome = SubmitOutcome { submit: Err(IrError::Network("connection refused".into())), replay: None };
        let report = format_submit_outcome(&outcome);
        assert!(report.score.starts_with("IR ERROR: "), "{}", report.score);
        assert!(report.score.contains("connection refused"));
        assert!(report.failed);
    }

    #[test]
    fn replay_upload_success_and_failure_each_add_a_line() {
        let uploaded = SubmitOutcome { submit: Ok(accepted(Some(2))), replay: Some(Ok("rep-1".into())) };
        assert_eq!(reported(uploaded), vec!["IR: RANK #2".to_string(), "REPLAY UPLOADED".to_string()]);

        let failed = SubmitOutcome { submit: Ok(accepted(Some(2))), replay: Some(Err(IrError::PayloadTooLarge("too big".into()))) };
        let lines = reported(failed);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("REPLAY ERROR: "), "{}", lines[1]);
    }

    #[test]
    fn no_replay_attempt_adds_no_replay_line() {
        let report = format_submit_outcome(&SubmitOutcome { submit: Ok(accepted(Some(1))), replay: None });
        assert_eq!(report.replay, None);
    }

    #[test]
    fn long_messages_are_flattened_and_truncated() {
        let long = "x".repeat(IR_LINE_MAX_CHARS * 2);
        let cut = truncate(&long);
        assert_eq!(cut.chars().count(), IR_LINE_MAX_CHARS + TRUNCATION_MARK.chars().count());
        assert!(cut.ends_with(TRUNCATION_MARK));
        assert_eq!(truncate("two\n  lines\there"), "two lines here", "whitespace collapses to one row");
        assert_eq!(truncate("short"), "short", "a short message is untouched");
    }

    #[test]
    fn only_a_run_that_is_waiting_on_the_worker_accepts_a_report() {
        assert!(IrStatus::Sending.accepts_report());
        assert!(!IrStatus::Off.accepts_report(), "a run with no server never submitted");
        assert!(!IrStatus::Skipped("autoplay").accepts_report(), "an earlier run's outcome must not relabel a skipped run");
        let report = IrReport { score: "RANK #3".into(), new_best: true, replay: None, failed: false };
        assert!(!IrStatus::Reported(report).accepts_report(), "a landed report is final");
    }
}

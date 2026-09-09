//! How one run is judged: turning the JUDGE settings rows (or a replay's record of them) into the
//! [`JudgeSetup`] a session is configured with, and deciding whether that setup counts as an assist.

use rbms_config::{
    Config, JUDGE_RATE_DEFAULT_PERCENT, LN_MARGIN_DEFAULT_PERCENT, algorithm_from_token, gauge_auto_shift_from_token, gauge_from_name, gauge_set_from_token,
    ln_mode_from_token,
};
use rbms_judge::ln::LnMode;
use rbms_play::JudgeSetup;
use rbms_store::{Replay, ReplayJudge};

/// Whether a run judged by `setup` was judged more leniently than the chart asks, which the
/// reference implementation treats as a custom judge: the run neither scores nor submits
/// (`BMSPlayer.java:208-214`). Narrowing a window is not an assist.
///
/// Taken from the setup the run was actually judged with rather than from the rows as they stand
/// now, so a replay is read by the widths it recorded and a row changed after the run cannot
/// retroactively make it assisted.
pub(crate) fn is_custom_judge(setup: &JudgeSetup) -> bool {
    setup.judge_rate_key.iter().chain(&setup.judge_rate_scratch).any(|rate| *rate > JUDGE_RATE_DEFAULT_PERCENT)
        || setup.longnote_margin_rate > LN_MARGIN_DEFAULT_PERCENT
}

/// The JUDGE settings the configured rows describe, ready to hand to a session.
pub(crate) fn judge_setup_of(config: &Config) -> JudgeSetup {
    JudgeSetup {
        judge_rate_key: config.judge.judge_rate_key,
        judge_rate_scratch: config.judge.judge_rate_scratch,
        longnote_margin_rate: config.judge.longnote_margin_rate,
        algorithm: config.judge.judge_algorithm,
        ln_mode: config.judge.ln_mode,
        gauge_set: config.judge.gauge_set,
        gauge_auto_shift: config.judge.gauge_auto_shift,
        bottom_shiftable_gauge: config.judge.bottom_shiftable_gauge,
    }
}

/// What a recorded run has to be judged by: every JUDGE row it was played under, read back off the
/// replay rather than off the settings as they stand now.
///
/// A replay written before a row was recorded reads as this engine's pre-parity behaviour through
/// [`ReplayJudge`]'s own defaults, so the current settings never leak into a playback — a long-note
/// flavour or a gauge table taken from the settings would change the note count, the EX ceiling and
/// the damage every judgement costs.
pub(crate) fn replay_judge_setup(judge: &ReplayJudge) -> JudgeSetup {
    JudgeSetup {
        judge_rate_key: judge.judge_rate_key,
        judge_rate_scratch: judge.judge_rate_scratch,
        longnote_margin_rate: judge.longnote_margin_rate,
        algorithm: algorithm_from_token(&judge.algorithm),
        ln_mode: ln_mode_from_token(&judge.ln_mode),
        gauge_set: gauge_set_from_token(&judge.gauge_set),
        gauge_auto_shift: gauge_auto_shift_from_token(&judge.gauge_auto_shift),
        bottom_shiftable_gauge: gauge_from_name(&judge.bottom_shiftable_gauge),
    }
}

/// The JUDGE settings this run is judged by: the replay's own when one is being played back, the
/// configured rows otherwise.
pub(crate) fn run_judge_setup(config: &Config, replay: Option<&Replay>) -> JudgeSetup {
    match replay {
        Some(replay) => replay_judge_setup(&replay.judge),
        None => judge_setup_of(config),
    }
}

/// The IR `lntype` a run reports: the chart's own `#LNMODE` when it states one (1 LN, 2 CN, 3 HCN
/// on the header scale), and the LN MODE the run was actually judged under when it does not. The
/// reference records the setting for the same reason (`BMSPlayer.java:893`).
pub(crate) fn run_lntype(chart_lnmode: i32, ln_mode: LnMode) -> i32 {
    match chart_lnmode {
        1 => LnMode::LongNote.id() as i32,
        2 => LnMode::ChargeNote.id() as i32,
        3 => LnMode::HellChargeNote.id() as i32,
        _ => ln_mode.id() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_config::{UNMODIFIED_JUDGE_RATES, algorithm_token, gauge_auto_shift_token, gauge_set_token, gauge_token, ln_mode_token};
    use rbms_judge::GaugeKind;
    use rbms_judge::algorithm::JudgeAlgorithm;
    use rbms_judge::gauge::GaugeAutoShift;
    use rbms_judge::gauge_tables::GaugeSetId;

    fn custom_setup() -> JudgeSetup {
        JudgeSetup {
            judge_rate_key: [140, 120, 90],
            judge_rate_scratch: [80, 150, 100],
            longnote_margin_rate: 150,
            algorithm: JudgeAlgorithm::Lowest,
            ln_mode: LnMode::HellChargeNote,
            gauge_set: Some(GaugeSetId::Lr2),
            gauge_auto_shift: GaugeAutoShift::SelectToUnder,
            bottom_shiftable_gauge: GaugeKind::Easy,
        }
    }

    fn recorded(setup: &JudgeSetup) -> ReplayJudge {
        ReplayJudge {
            algorithm: algorithm_token(setup.algorithm).to_string(),
            judge_rate_key: setup.judge_rate_key,
            judge_rate_scratch: setup.judge_rate_scratch,
            longnote_margin_rate: setup.longnote_margin_rate,
            ln_mode: ln_mode_token(setup.ln_mode).to_string(),
            gauge_set: gauge_set_token(setup.gauge_set).to_string(),
            gauge_auto_shift: gauge_auto_shift_token(setup.gauge_auto_shift).to_string(),
            bottom_shiftable_gauge: gauge_token(setup.bottom_shiftable_gauge).to_string(),
        }
    }

    #[test]
    fn the_shipped_rows_are_not_a_custom_judge() {
        assert!(!is_custom_judge(&judge_setup_of(&Config::default())));
    }

    #[test]
    fn only_widening_makes_a_run_a_custom_judge() {
        let narrowed = JudgeSetup { judge_rate_key: [50, 50, 50], judge_rate_scratch: [50, 50, 50], longnote_margin_rate: 50, ..JudgeSetup::default() };
        assert!(!is_custom_judge(&narrowed), "BMSPlayer.java:208-214 only raises assist for a widened window");
        for widened in [
            JudgeSetup { judge_rate_key: [101, 100, 100], ..JudgeSetup::default() },
            JudgeSetup { judge_rate_scratch: [100, 100, 101], ..JudgeSetup::default() },
            JudgeSetup { longnote_margin_rate: 101, ..JudgeSetup::default() },
        ] {
            assert!(is_custom_judge(&widened), "{widened:?}");
        }
    }

    #[test]
    fn every_judge_row_reaches_the_session_verbatim() {
        let mut config = Config::default();
        let rows = custom_setup();
        config.judge.judge_rate_key = rows.judge_rate_key;
        config.judge.judge_rate_scratch = rows.judge_rate_scratch;
        config.judge.longnote_margin_rate = rows.longnote_margin_rate;
        config.judge.judge_algorithm = rows.algorithm;
        config.judge.ln_mode = rows.ln_mode;
        config.judge.gauge_set = rows.gauge_set;
        config.judge.gauge_auto_shift = rows.gauge_auto_shift;
        config.judge.bottom_shiftable_gauge = rows.bottom_shiftable_gauge;
        assert_eq!(judge_setup_of(&config), rows, "a row edited on the settings screen has to change how the next run is judged");
        assert_eq!(judge_setup_of(&Config::default()), JudgeSetup::default(), "the shipped rows are the engine's own defaults");
    }

    #[test]
    fn a_replay_is_judged_by_every_row_it_recorded_not_by_the_current_settings() {
        let played = custom_setup();
        let replayed = replay_judge_setup(&recorded(&played));
        assert_eq!(replayed, played, "a playback that took any row from the settings would judge a different run");
    }

    #[test]
    fn a_replay_written_before_the_rows_existed_reads_as_the_engines_old_behaviour() {
        let old = replay_judge_setup(&ReplayJudge::default());
        assert_eq!(old.algorithm, JudgeAlgorithm::Duration, "the only algorithm this engine used to have");
        assert_eq!(old.ln_mode, LnMode::LongNote);
        assert_eq!(old.gauge_set, None, "the table the chart's own mode selects");
        assert_eq!(old.gauge_auto_shift, GaugeAutoShift::None);
        assert_eq!(old.judge_rate_key, UNMODIFIED_JUDGE_RATES);
    }

    #[test]
    fn a_run_with_no_replay_takes_the_configured_rows() {
        let mut config = Config::default();
        config.judge.ln_mode = LnMode::ChargeNote;
        assert_eq!(run_judge_setup(&config, None), judge_setup_of(&config));
    }

    #[test]
    fn the_reported_lntype_falls_back_to_the_setting_only_when_the_chart_states_none() {
        for (header, expected) in [(1, 0), (2, 1), (3, 2)] {
            assert_eq!(run_lntype(header, LnMode::HellChargeNote), expected, "#LNMODE {header} wins over the setting");
        }
        for lnmode in LnMode::ALL {
            assert_eq!(run_lntype(0, lnmode), lnmode.id() as i32, "an unstated chart reports what it was judged as");
        }
    }
}

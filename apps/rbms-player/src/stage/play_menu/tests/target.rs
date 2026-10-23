//! The TARGET and TARGET FALLBACK rows of the play menu: where they sit, what each reads, what
//! stepping them does, what puts them back, and how the preview of the play screen follows them.
//!
//! The menu is opened over the pack this repository ships. With [`CAPTURE_DIR_ENV`] naming a folder
//! the menu is saved there as `target-play-menu-*.png`, at the size the screen is looked at.

use rbms_config::{ScoreTarget, TARGET_FALLBACK_LABELS, adjust};

use super::*;

/// The TARGET row stepped with the key a player steps it with, from where it stands to `target`.
fn choose_target(menu: &mut Menu, target: ScoreTarget) {
    menu.focus_setting(SettingId::Target);
    for _ in 0..ScoreTarget::ALL.len() {
        if menu.app.shared.config.judge.target == target {
            return;
        }
        menu.key(KeyCode::ArrowRight);
    }
    panic!("{target:?} is not one of the places the TARGET row steps through");
}

/// The TARGET FALLBACK row stepped the same way.
fn choose_fallback(menu: &mut Menu, fallback: ScoreTarget) {
    menu.focus_setting(SettingId::TargetFallback);
    for _ in 0..ScoreTarget::FALLBACKS.len() {
        if menu.app.shared.config.judge.target_fallback == fallback {
            return;
        }
        menu.key(KeyCode::ArrowRight);
    }
    panic!("{fallback:?} is not one of the places the TARGET FALLBACK row steps through");
}

/// The fallback row is the one under the TARGET row, in the group of the rows that decide how a
/// chart is played, and it offers only targets that are always known.
#[test]
fn the_fallback_row_sits_under_the_target_row_and_offers_only_what_is_always_known() {
    let menu = Menu::over(bare_app("target-rows"));
    let target = menu.at(Row::Setting(SettingId::Target));
    assert_eq!(menu.state.rows[target + 1], Row::Setting(SettingId::TargetFallback), "the fallback does not follow the target");
    assert_eq!(menu.line(Row::Setting(SettingId::TargetFallback)).label, "TARGET FALLBACK");
    assert_eq!(menu.value(SettingId::TargetFallback), "RATE AAA", "a fresh install names the shipped target");
    assert_eq!(rbms_config::cycle_values(SettingId::TargetFallback), TARGET_FALLBACK_LABELS);
    assert!(
        rbms_config::cycle_values(SettingId::TargetFallback).iter().all(|label| ScoreTarget::FALLBACKS.iter().any(|target| target.label() == *label)),
        "the row offers a name that is not an always-known target"
    );
}

/// A target that names a score is written with what stands in for it where there is none, which is
/// what the sample chart under the preview is; one that needs no score is its name alone. The
/// widest of them fit where they are written, whichever fallback is named.
#[test]
fn a_target_that_names_a_score_says_what_stands_in_for_it_and_fits_the_row() {
    let mut menu = Menu::over(bare_app("target-text"));
    choose_target(&mut menu, ScoreTarget::LocalBest);
    assert_eq!(menu.value(SettingId::Target), "LOCAL BEST > AAA");
    let target = Row::Setting(SettingId::Target);
    assert_eq!(menu.state.target_note(&menu.app.shared, target).as_deref(), Some("NO RECORD: PACED AGAINST RATE AAA INSTEAD"));
    choose_fallback(&mut menu, ScoreTarget::RateA);
    assert_eq!(menu.value(SettingId::Target), "LOCAL BEST > A");
    assert_eq!(menu.state.target_note(&menu.app.shared, target).as_deref(), Some("NO RECORD: PACED AGAINST RATE A INSTEAD"));
    choose_target(&mut menu, ScoreTarget::RateAaPlus);
    assert_eq!(menu.value(SettingId::Target), "RATE AA+", "a target that is always known has nothing to fall back from");
    assert_eq!(menu.state.target_note(&menu.app.shared, target), None);
    assert_eq!(menu.state.target_note(&menu.app.shared, Row::Setting(SettingId::TargetFallback)), None, "only the TARGET row has this to say");

    for fallback in ScoreTarget::FALLBACKS {
        menu.app.shared.config.judge.target_fallback = fallback;
        for target in ScoreTarget::ALL {
            menu.app.shared.config.judge.target = target;
            for id in [SettingId::Target, SettingId::TargetFallback] {
                let Line { label, value, .. } = menu.line(Row::Setting(id));
                assert_eq!(fitted_value(&value), value, "{label} reads {value} with {target:?} and {fallback:?}");
                assert!(text_width(&label, LABEL_SCALE) + LABEL_GAP + text_width(&value, VALUE_SCALE) <= row_room(), "{label} with {value}");
            }
        }
    }
}

/// The fallback steps round its twelve targets, is put back to the shipped one by the default key,
/// and moves the preview of the play screen: with LOCAL BEST chosen and no record on the sample
/// chart, the preview is paced against the fallback.
#[test]
fn the_fallback_steps_resets_and_is_what_the_preview_is_paced_against() {
    let mut menu = Menu::over(bare_app("target-steps"));
    choose_target(&mut menu, ScoreTarget::LocalBest);
    let mut seen = Vec::new();
    menu.focus_setting(SettingId::TargetFallback);
    for _ in 0..ScoreTarget::FALLBACKS.len() {
        seen.push(menu.app.shared.config.judge.target_fallback);
        menu.key(KeyCode::ArrowRight);
    }
    let from_the_shipped = ScoreTarget::FALLBACKS.iter().position(|target| *target == ScoreTarget::default()).expect("the shipped fallback is offered");
    let mut in_order = ScoreTarget::FALLBACKS.to_vec();
    in_order.rotate_left(from_the_shipped);
    assert_eq!(seen, in_order, "the row did not walk the targets in order, from the one it began on");
    assert_eq!(menu.app.shared.config.judge.target_fallback, ScoreTarget::RateAaa, "a whole turn comes back to where it began");

    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.app.shared.config.judge.target_fallback, ScoreTarget::RateAaaMinus, "one step back");
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.app.shared.config.judge.target_fallback, ScoreTarget::default(), "the default key did not put the shipped fallback back");

    let paced_by = |menu: &Menu| rbms_config::display_value(&menu.app.shared.config, SettingId::TargetFallback);
    assert_eq!(paced_by(&menu), "RATE AAA");
    assert_eq!(adjust(&mut menu.app.shared.config, SettingId::TargetFallback, 1), rbms_config::AdjustOutcome::Changed);
    assert_eq!(paced_by(&menu), "RATE AAA+");
}

/// RATE CUSTOM is a target and a fallback, and its share is the TARGET RATE row under the fallback:
/// the rows that name it say the share, and stepping the row moves what the preview is paced against.
#[test]
fn the_custom_rate_is_written_with_its_share_and_the_row_under_the_fallback_moves_it() {
    let mut menu = Menu::over(bare_app("target-custom"));
    let fallback = menu.at(Row::Setting(SettingId::TargetFallback));
    assert_eq!(menu.state.rows[fallback + 1], Row::Setting(SettingId::TargetRate), "the share does not follow the fallback");
    assert_eq!(menu.line(Row::Setting(SettingId::TargetRate)).label, "TARGET RATE");
    assert_eq!(menu.value(SettingId::TargetRate), "85%", "the shipped share");

    choose_target(&mut menu, ScoreTarget::LocalBest);
    choose_fallback(&mut menu, ScoreTarget::RateCustom);
    assert_eq!(menu.value(SettingId::TargetFallback), "RATE CUSTOM");
    assert_eq!(menu.value(SettingId::Target), "LOCAL BEST > 85%");
    let target = Row::Setting(SettingId::Target);
    assert_eq!(menu.state.target_note(&menu.app.shared, target).as_deref(), Some("NO RECORD: PACED AGAINST SCORE RATE 85% INSTEAD"));

    menu.focus_setting(SettingId::TargetRate);
    menu.shifted(KeyCode::ArrowLeft);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::TargetRate), "76%", "ten down and one up");
    assert_eq!(menu.value(SettingId::Target), "LOCAL BEST > 76%");
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.value(SettingId::TargetRate), "85%", "the default key did not put the shipped share back");
    choose_target(&mut menu, ScoreTarget::RateCustom);
    assert_eq!(menu.value(SettingId::Target), "RATE CUSTOM", "a target that is always known has nothing to fall back from");
}

/// The rows as they are looked at: LOCAL BEST chosen with the shipped fallback, and the fallback
/// stepped to another rate, each with the cursor on the row it names.
#[test]
fn the_target_rows_can_be_captured() {
    let mut menu = Menu::over(app_with_shipped_pack("target-capture"));
    choose_target(&mut menu, ScoreTarget::LocalBest);
    menu.focus_setting(SettingId::Target);
    assert_eq!(menu.capture_at("target-play-menu-local-best", RUN_SECONDS), PreviewStatus::Drawn);
    choose_fallback(&mut menu, ScoreTarget::RateAaPlus);
    assert_eq!(menu.capture_at("target-play-menu-fallback", RUN_SECONDS), PreviewStatus::Drawn);
    choose_fallback(&mut menu, ScoreTarget::RateCustom);
    menu.focus_setting(SettingId::TargetRate);
    (0..15).for_each(|_| drop(menu.key(KeyCode::ArrowLeft)));
    assert_eq!(menu.capture_at("target-play-menu-custom-rate", RUN_SECONDS), PreviewStatus::Drawn);
    choose_target(&mut menu, ScoreTarget::Rival);
    choose_fallback(&mut menu, ScoreTarget::RankNext);
    assert_eq!(menu.capture_at("target-play-menu-rival-rank-next", RUN_SECONDS), PreviewStatus::Drawn);
    let mut large = Menu::sized(app_with_shipped_pack("target-capture-large"), LARGE);
    choose_target(&mut large, ScoreTarget::IrBest);
    choose_fallback(&mut large, ScoreTarget::RateMaxMinus);
    assert_eq!(large.capture_at("target-play-menu-ir-best", RUN_SECONDS), PreviewStatus::Drawn);
    large.focus_setting(SettingId::Target);
    assert_eq!(large.capture_at("target-play-menu-ir-best-note", RUN_SECONDS), PreviewStatus::Drawn);
    large.focus_setting(SettingId::TargetRate);
    assert_eq!(large.capture_at("target-play-menu-rate-unused", RUN_SECONDS), PreviewStatus::Drawn);
    large.focus_setting(SettingId::ScratchAuto);
    assert_eq!(large.capture_at("target-play-menu-scratch-auto", RUN_SECONDS), PreviewStatus::Drawn);
}

/// The panel under the preview says what a row costs or leaves unused beside what the row is for:
/// SCRATCH AUTO counts no turntable note and lights an assisted lamp, TARGET RATE is not in use while no target is RATE CUSTOM, and
/// IR BEST tries the player's own record before the fallback. Each is said whole on its one line.
#[test]
fn the_panel_says_what_scratch_auto_costs_and_when_the_target_rate_is_unused() {
    let mut menu = Menu::over(bare_app("target-notes"));
    let note = |menu: &Menu, id: SettingId| menu.state.row_note(&menu.app.shared, Row::Setting(id));
    menu.app.shared.config.judge.target = ScoreTarget::LocalBest;
    menu.app.shared.config.judge.target_fallback = ScoreTarget::RateAaa;
    assert_eq!(note(&menu, SettingId::TargetRate).as_deref(), Some(PlayMenuText::TargetRateUnusedNote.get(language())));
    menu.app.shared.config.judge.target_fallback = ScoreTarget::RateCustom;
    assert_eq!(note(&menu, SettingId::TargetRate), None, "the fallback is RATE CUSTOM and uses the rate");
    menu.app.shared.config.judge.target = ScoreTarget::RateCustom;
    menu.app.shared.config.judge.target_fallback = ScoreTarget::Max;
    assert_eq!(note(&menu, SettingId::TargetRate), None, "the target is RATE CUSTOM and uses the rate");
    assert_eq!(note(&menu, SettingId::ScratchAuto).as_deref(), Some(PlayMenuText::ScratchAutoNote.get(language())));
    assert_eq!(note(&menu, SettingId::AutoReplay), None);
    menu.app.shared.config.judge.target = ScoreTarget::IrBest;
    let ir_best = note(&menu, SettingId::Target).expect("IR BEST says what stands in for it");
    assert_eq!(ir_best, "NO IR SCORE: YOUR LOCAL BEST, THEN MAX");

    let room = INFO_RECT.w - INFO_PAD * 2.0;
    for said in [PlayMenuText::TargetRateUnusedNote.get(language()), PlayMenuText::ScratchAutoNote.get(language()), ir_best.as_str()] {
        assert_eq!(fit_text(said, INFO_SCALE, room), said, "the panel cuts {said} short");
    }
}

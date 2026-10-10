//! The ranking cluster answers from how the submission of the run stands, and answers the absent
//! value of its kind for everything the service has not sent.

use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};
use rbms_skin::timer::timer_id;

use super::*;
use crate::ir_outcome::{IrReport, IrStatus};
use crate::skin_host::result::snapshot::ReplaySlot;
use crate::skin_host::{Cluster, IdSpace, ROUTES};

const OFFLINE: IrLink = IrLink { online: false, phase: IrPhase::Offline, rank: 0, previous_rank: 0 };

/// A screen whose ranking holds 120 players and is scrolled to the thirtieth.
const SCENE: ResultScene = ResultScene { gauge_type: 2, replay: [ReplaySlot::Missing; 4], ranking_offset: 30, ranking_total: 120 };

fn report(failed: bool) -> IrReport {
    IrReport { score: "IR: RANK #3".to_string(), new_best: false, replay: None, failed }
}

fn state(link: IrLink) -> IrState<'static> {
    IrState { link: Some(link), scene: Some(SCENE), service_name: "Service", user_name: "guest", target_name: "RIVAL" }
}

#[test]
fn the_phase_follows_the_status_of_the_submission() {
    assert_eq!(IrPhase::from(&IrStatus::Off), IrPhase::Offline);
    assert_eq!(IrPhase::from(&IrStatus::Skipped("autoplay")), IrPhase::Offline, "a run that was never sent reads as offline");
    assert_eq!(IrPhase::from(&IrStatus::Sending), IrPhase::Processing);
    assert_eq!(IrPhase::from(&IrStatus::Reported(report(false))), IrPhase::Finished { succeeded: true });
    assert_eq!(IrPhase::from(&IrStatus::Reported(report(true))), IrPhase::Finished { succeeded: false });
}

#[test]
fn the_connection_timers_are_the_ones_the_phase_has_switched_on() {
    let on = |phase| IrLink::new(true, phase).timers().on().map(|id| id.0).collect::<Vec<_>>();

    assert_eq!(on(IrPhase::Offline), Vec::<i32>::new());
    assert_eq!(on(IrPhase::Processing), [timer_id::IR_CONNECT_BEGIN.0]);
    assert_eq!(on(IrPhase::Finished { succeeded: true }), [timer_id::IR_CONNECT_BEGIN.0, timer_id::IR_CONNECT_SUCCESS.0]);
    assert_eq!(on(IrPhase::Finished { succeeded: false }), [timer_id::IR_CONNECT_BEGIN.0, timer_id::IR_CONNECT_FAIL.0]);
}

#[test]
fn an_offline_run_has_no_ranking_numbers_at_all() {
    let offline = state(OFFLINE);

    for id in [NUMBER_IR_RANK, NUMBER_IR_TOTALPLAYER, NUMBER_IR_PREVRANK, NUMBER_IR_TOTALPLAYER2] {
        assert_eq!(offline.integer(id), Some(INTEGER_ABSENT), "number {id}");
    }
    assert_eq!((offline.boolean(OPTION_OFFLINE), offline.boolean(OPTION_ONLINE)), (Some(true), Some(false)));
}

#[test]
fn a_run_being_sent_has_a_ranking_with_nobody_on_it_yet() {
    let sending = IrState { scene: None, ..state(IrLink::new(true, IrPhase::Processing)) };

    for id in [NUMBER_IR_RANK, NUMBER_IR_TOTALPLAYER, NUMBER_IR_PREVRANK, NUMBER_IR_TOTALPLAYER2] {
        assert_eq!(sending.integer(id), Some(0), "number {id}");
    }
    assert_eq!((sending.boolean(OPTION_OFFLINE), sending.boolean(OPTION_ONLINE)), (Some(false), Some(true)));
}

#[test]
fn a_ranking_the_caller_filled_in_is_read_back() {
    let ranked = state(IrLink { online: true, phase: IrPhase::Finished { succeeded: true }, rank: 3, previous_rank: 5 });

    assert_eq!((ranked.integer(NUMBER_IR_RANK), ranked.integer(NUMBER_IR_PREVRANK)), (Some(3), Some(5)));
    assert_eq!(
        (ranked.integer(NUMBER_IR_TOTALPLAYER), ranked.integer(NUMBER_IR_TOTALPLAYER2)),
        (Some(120), Some(120)),
        "the players on it come from the screen"
    );
}

#[test]
fn the_ranking_s_scroll_is_how_far_down_it_is_over_how_many_players_it_has() {
    let ranked = state(IrLink::new(true, IrPhase::Finished { succeeded: true }));
    assert_eq!(ranked.rate(RATE_RANKING_POSITION), Some(0.25));
    assert_eq!(ranked.rate(RATE_RANKING_POSITION + 1), None);

    let empty = IrState { scene: Some(ResultScene { ranking_offset: 0, ranking_total: 0, ..SCENE }), ..ranked };
    assert_eq!(empty.rate(RATE_RANKING_POSITION), Some(0.0), "a ranking with nobody on it is counted as one place long");
    assert_eq!(IrState::default().rate(RATE_RANKING_POSITION), None);
}

#[test]
fn the_tables_of_the_ranking_have_no_values() {
    let sending = state(IrLink::new(true, IrPhase::Processing));
    let ranked = state(IrLink { online: true, phase: IrPhase::Finished { succeeded: true }, rank: 3, previous_rank: 5 });

    for ir in [state(OFFLINE), sending, ranked] {
        for id in (NUMBER_IR_PLAYER_NOPLAY..=NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE_AFTERDOT).filter(|id| *id != 221) {
            assert_eq!(ir.integer(id), Some(INTEGER_ABSENT), "number {id}");
        }
        for id in (NUMBER_RANKING1_EXSCORE..=NUMBER_RANKING10_CLEAR).chain(NUMBER_RIVAL_PERFECT..=NUMBER_RIVAL_POOR_RATE) {
            assert_eq!(ir.integer(id), Some(INTEGER_ABSENT), "number {id}");
        }
        for id in NUMBER_RANKING1_EXSCORE..=NUMBER_RANKING10_CLEAR {
            assert_eq!(ir.image_index(id), Some(IMAGE_INDEX_ABSENT), "image index {id}");
        }
        for id in [
            FLOAT_IR_PLAYER_NOPLAY_RATE,
            FLOAT_IR_PLAYER_MAX_RATE,
            FLOAT_IR_TOTALCLEARRATE,
            FLOAT_IR_TOTALFULLCOMBORATE,
            FLOAT_RIVAL_PERFECT_RATE,
            FLOAT_RIVAL_POOR_RATE,
        ] {
            assert_eq!(ir.float(id), Some(FLOAT_ABSENT), "float {id}");
        }
    }
}

#[test]
fn the_numbers_the_reference_leaves_without_a_property_are_not_answered() {
    let ir = state(OFFLINE);

    assert_eq!(ir.integer(NUMBER_IR_CLEARRATE), None);
    assert_eq!(ir.integer(NUMBER_IR_TOTALPLAYCOUNT), None);
    assert_eq!(ir.integer(221), None);
    assert_eq!(ir.integer(NUMBER_RIVAL_SCORE), None, "the target's score is the score cluster's");
}

#[test]
fn the_options_of_the_browser_s_ranking_panel_are_off_on_a_result_screen() {
    let ir = state(OFFLINE);

    for id in [OPTION_IR_NOPLAYER, OPTION_IR_FAILED, OPTION_IR_WAITING, OPTION_IR_BUSY] {
        assert_eq!(ir.boolean(id), Some(false), "option {id}");
    }
    for id in [OPTION_IR_LOADING, OPTION_IR_LOADED, OPTION_IR_BANNED, OPTION_IR_ACCESSING] {
        assert_eq!(ir.boolean(id), None, "option {id} has no property in the reference");
    }
}

#[test]
fn the_names_are_the_target_the_service_and_the_player_on_it_and_the_ranking_has_none() {
    let ir = state(OFFLINE);

    assert_eq!(ir.text(STRING_RIVAL).as_deref(), Some("RIVAL"));
    assert_eq!(ir.text(3).as_deref(), Some("RIVAL"), "the target is the rival of a result screen");
    assert_eq!(ir.text(STRING_IR_NAME).as_deref(), Some("Service"));
    assert_eq!(ir.text(STRING_IR_USER_NAME).as_deref(), Some("guest"));
    for id in STRING_RANKING1_NAME..=STRING_RANKING10_NAME {
        assert_eq!(ir.text(id).as_deref(), Some(""), "text {id}");
    }
    assert_eq!(ir.text(STRING_TITLE), None);
}

#[test]
fn without_a_link_nothing_is_answered() {
    let ir = IrState::default();

    assert_eq!(ir.integer(NUMBER_IR_RANK), None);
    assert_eq!(ir.boolean(OPTION_OFFLINE), None);
    assert_eq!(ir.text(STRING_IR_NAME), None);
    assert_eq!(ir.image_index(NUMBER_RANKING1_EXSCORE), None);
}

#[test]
fn every_number_and_option_the_reference_defines_in_a_run_routed_to_this_cluster_is_answered() {
    let ir = state(OFFLINE);

    for route in ROUTES.iter().filter(|route| route.cluster == Cluster::Ir) {
        for id in route.first..=route.last {
            let (name_space, answered) = match route.space {
                IdSpace::Integer => (rbms_skin::property::NameSpace::Integer, ir.integer(id).is_some()),
                IdSpace::Boolean => (rbms_skin::property::NameSpace::Boolean, ir.boolean(id).is_some()),
                IdSpace::Float => (rbms_skin::property::NameSpace::Float, ir.float(id).is_some()),
                IdSpace::Text => (rbms_skin::property::NameSpace::Text, ir.text(id).is_some()),
                IdSpace::ImageIndex => (rbms_skin::property::NameSpace::ImageIndex, ir.image_index(id).is_some()),
                IdSpace::Rate => (rbms_skin::property::NameSpace::Rate, ir.rate(id).is_some()),
                IdSpace::Offset => continue,
            };
            let selector_only_text_or_number = matches!(
                (route.space, id),
                (IdSpace::Integer, NUMBER_RIVAL_SCORE | NUMBER_RIVAL_MAXSCORE..=NUMBER_RIVAL_FAILCOUNT | NUMBER_IR_CLEARRATE | NUMBER_IR_TOTALPLAYCOUNT | 220..=221)
                    | (IdSpace::Boolean, OPTION_IR_LOADING | OPTION_IR_LOADED | OPTION_IR_BANNED | OPTION_IR_ACCESSING)
            );
            if rbms_skin::property::reference_implements(name_space, id) && !selector_only_text_or_number {
                assert!(answered, "{:?} id {id} in {route:?}", route.space);
            }
        }
    }
}

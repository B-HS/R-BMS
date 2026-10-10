//! The ranking cluster answers from how the submission of the run stands, and answers the absent
//! value of its kind for everything the service has not sent.

use rbms_judge::ClearType;
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};
use rbms_skin::timer::timer_id;

use super::*;
use crate::format::clear_label_color;
use crate::ir_outcome::{IrReport, IrStatus};
use crate::ir_ranking::{RankingBoard, RankingRow, RankingState};
use crate::skin_host::result::snapshot::ReplaySlot;
use crate::skin_host::{Cluster, IdSpace, ROUTES};

const OFFLINE: IrLink = IrLink { online: false, phase: IrPhase::Offline, rank: 0, previous_rank: 0 };

/// A screen whose ranking holds 120 players and is scrolled to the thirtieth.
const SCENE: ResultScene = ResultScene { gauge_type: 2, replay: [ReplaySlot::Missing; 4], ranking_offset: 30, ranking_total: 120 };

fn report(failed: bool) -> IrReport {
    IrReport { score: "IR: RANK #3".to_string(), new_best: false, replay: None, failed }
}

fn state(link: IrLink) -> IrState<'static> {
    IrState { link: Some(link), scene: Some(SCENE), service_name: "Service", user_name: "guest", target_name: "RIVAL", ..IrState::default() }
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

fn row(lamp: ClearType, rank: Option<u32>) -> RankingRow {
    let (label, color) = clear_label_color(lamp);
    RankingRow { rank, player: "Player".to_string(), ex_score: 100, lamp: label, lamp_color: color, fast_slow: None, replay_id: None }
}

/// A fetched board of seven scores: three failed, one clear, two hard clears and one max, with the
/// player third.
fn fetched() -> RankingState {
    let rows = [ClearType::Max, ClearType::Hard, ClearType::Hard, ClearType::Normal, ClearType::Failed, ClearType::Failed, ClearType::Failed]
        .into_iter()
        .map(|lamp| row(lamp, None))
        .collect();
    RankingState::Ready(Box::new(RankingBoard { rows, you: Some(row(ClearType::Hard, Some(3))), rivals: Vec::new() }))
}

fn browsing(ranking: Option<&RankingState>) -> IrState<'static> {
    IrState { browser: Some(IrBrowser::of_ranking(true, ranking)), service_name: "Service", user_name: "guest", ..IrState::default() }
}

#[test]
fn a_browser_with_no_service_holds_no_ranking_whatever_the_cache_has() {
    let offline = IrBrowser::of_ranking(false, Some(&fetched()));

    assert_eq!(offline, IrBrowser { online: false, board: None, offset: 0 });
    assert_eq!(IrBrowser::of_ranking(true, None).board, None, "and a service with nothing fetched yet holds none either");
}

#[test]
fn the_ranking_follows_the_fetch_from_being_asked_to_being_answered_or_refused() {
    let access = |state: &RankingState| IrBrowser::of_ranking(true, Some(state)).board.map(|board| board.access);

    assert_eq!(access(&RankingState::Loading), Some(IrAccess::Accessing));
    assert_eq!(access(&RankingState::Failed("timeout".to_string())), Some(IrAccess::Failed));
    assert_eq!(access(&fetched()), Some(IrAccess::Finished));
}

#[test]
fn the_connection_timers_of_the_browser_are_on_only_while_the_ranking_is_in_their_state() {
    let on = |state: Option<&RankingState>| IrBrowser::of_ranking(true, state).timers().on().map(|id| id.0).collect::<Vec<_>>();

    assert_eq!(on(None), Vec::<i32>::new());
    assert_eq!(on(Some(&RankingState::Loading)), [timer_id::IR_CONNECT_BEGIN.0], "the begin timer ends with the fetch, unlike a result screen's");
    assert_eq!(on(Some(&fetched())), [timer_id::IR_CONNECT_SUCCESS.0]);
    assert_eq!(on(Some(&RankingState::Failed(String::new()))), [timer_id::IR_CONNECT_FAIL.0]);
}

#[test]
fn the_options_of_the_browser_follow_the_state_of_the_ranking() {
    let options = |state: Option<&RankingState>| {
        let ir = browsing(state);
        [OPTION_OFFLINE, OPTION_ONLINE, OPTION_IR_NOPLAYER, OPTION_IR_FAILED, OPTION_IR_WAITING, OPTION_IR_BUSY].map(|id| ir.boolean(id))
    };

    assert_eq!(options(None), [Some(false), Some(true), Some(false), Some(false), Some(true), Some(false)], "nothing held is the waiting state");
    assert_eq!(options(Some(&RankingState::Loading)), [Some(false), Some(true), Some(false), Some(false), Some(false), Some(false)]);
    assert_eq!(
        options(Some(&RankingState::Failed(String::new()))),
        [Some(false), Some(true), Some(false), Some(true), Some(false), Some(true)],
        "busy is defined as failed"
    );
    assert_eq!(options(Some(&fetched())), [Some(false), Some(true), Some(false), Some(false), Some(false), Some(false)]);
    let empty = RankingState::Ready(Box::new(RankingBoard { rows: Vec::new(), you: None, rivals: Vec::new() }));
    assert_eq!(browsing(Some(&empty)).boolean(OPTION_IR_NOPLAYER), Some(true), "a ranking that came back with nobody on it");
    let offline = IrState { browser: Some(IrBrowser::of_ranking(false, None)), ..IrState::default() };
    assert_eq!((offline.boolean(OPTION_OFFLINE), offline.boolean(OPTION_ONLINE)), (Some(true), Some(false)));
    assert_eq!(offline.boolean(OPTION_IR_WAITING), Some(true));
}

#[test]
fn the_place_of_the_player_and_the_size_of_the_ranking_are_read_once_it_is_fetched() {
    let fetched = fetched();
    let ir = browsing(Some(&fetched));

    assert_eq!((ir.integer(NUMBER_IR_RANK), ir.integer(NUMBER_IR_TOTALPLAYER), ir.integer(NUMBER_IR_TOTALPLAYER2)), (Some(3), Some(7), Some(7)));
    assert_eq!(ir.integer(NUMBER_IR_PREVRANK), Some(INTEGER_ABSENT), "the browser keeps no earlier place");
    assert_eq!(ir.integer(NUMBER_IR_UPDATE_WAITING_TIME), Some(INTEGER_ABSENT));
    for state in [None, Some(&RankingState::Loading), Some(&RankingState::Failed(String::new()))] {
        let ir = browsing(state);
        assert_eq!((ir.integer(NUMBER_IR_RANK), ir.integer(NUMBER_IR_TOTALPLAYER)), (Some(INTEGER_ABSENT), Some(INTEGER_ABSENT)));
    }
    let unplaced = RankingState::Ready(Box::new(RankingBoard { rows: vec![row(ClearType::Hard, Some(1))], you: None, rivals: Vec::new() }));
    assert_eq!(browsing(Some(&unplaced)).integer(NUMBER_IR_RANK), Some(0), "a player who is not on it holds no place");
}

#[test]
fn the_scores_are_counted_onto_the_lamps_and_read_as_counts_and_shares() {
    let fetched = fetched();
    let ir = browsing(Some(&fetched));

    assert_eq!(
        [NUMBER_IR_PLAYER_NOPLAY, NUMBER_IR_PLAYER_FAILED, NUMBER_IR_PLAYER_NORMAL, NUMBER_IR_PLAYER_HARD, NUMBER_IR_PLAYER_MAX].map(|id| ir.integer(id)),
        [Some(0), Some(3), Some(1), Some(2), Some(1)]
    );
    assert_eq!((ir.integer(NUMBER_IR_PLAYER_FAILED_RATE), ir.integer(NUMBER_IR_PLAYER_FAILED_RATE_AFTERDOT)), (Some(42), Some(8)));
    assert_eq!((ir.integer(NUMBER_IR_PLAYER_HARD_RATE), ir.integer(NUMBER_IR_PLAYER_HARD_RATE_AFTERDOT)), (Some(28), Some(5)));
    assert_eq!(
        (ir.integer(NUMBER_IR_PLAYER_TOTAL_CLEAR), ir.integer(NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE), ir.integer(NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE_AFTERDOT)),
        (Some(4), Some(57), Some(1))
    );
    assert_eq!(
        (
            ir.integer(NUMBER_IR_PLAYER_TOTAL_FULLCOMBO),
            ir.integer(NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE),
            ir.integer(NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE_AFTERDOT)
        ),
        (Some(1), Some(14), Some(2))
    );
    assert_eq!(
        (ir.float(FLOAT_IR_PLAYER_FAILED_RATE), ir.float(FLOAT_IR_TOTALCLEARRATE), ir.float(FLOAT_IR_TOTALFULLCOMBORATE)),
        (Some(3.0 / 7.0), Some(4.0 / 7.0), Some(1.0 / 7.0))
    );
}

#[test]
fn a_ranking_not_fetched_has_no_counts_and_no_shares() {
    let ir = browsing(Some(&RankingState::Loading));

    for id in [
        NUMBER_IR_PLAYER_FAILED,
        NUMBER_IR_PLAYER_FAILED_RATE,
        NUMBER_IR_PLAYER_FAILED_RATE_AFTERDOT,
        NUMBER_IR_PLAYER_TOTAL_CLEAR,
        NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE,
        NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE_AFTERDOT,
    ] {
        assert_eq!(ir.integer(id), Some(INTEGER_ABSENT), "number {id}");
    }
    assert_eq!((ir.float(FLOAT_IR_PLAYER_FAILED_RATE), ir.float(FLOAT_IR_TOTALCLEARRATE)), (Some(FLOAT_ABSENT), Some(FLOAT_ABSENT)));
    let empty = RankingState::Ready(Box::new(RankingBoard { rows: Vec::new(), you: None, rivals: Vec::new() }));
    let ir = browsing(Some(&empty));
    assert_eq!(
        (ir.integer(NUMBER_IR_PLAYER_FAILED), ir.integer(NUMBER_IR_PLAYER_FAILED_RATE), ir.float(FLOAT_IR_PLAYER_FAILED_RATE)),
        (Some(0), Some(INTEGER_ABSENT), Some(FLOAT_ABSENT)),
        "no share of nobody"
    );
}

#[test]
fn the_ranking_list_and_the_rivals_are_not_read_from_the_board() {
    let fetched = fetched();
    let ir = browsing(Some(&fetched));

    for id in NUMBER_RANKING1_EXSCORE..=NUMBER_RANKING10_CLEAR {
        assert_eq!((ir.integer(id), ir.image_index(id)), (Some(INTEGER_ABSENT), Some(IMAGE_INDEX_ABSENT)), "slot id {id}");
    }
    for id in NUMBER_RIVAL_PERFECT..=NUMBER_RIVAL_POOR_RATE {
        assert_eq!(ir.integer(id), Some(INTEGER_ABSENT), "rival number {id}");
    }
    assert_eq!(ir.float(FLOAT_RIVAL_PERFECT_RATE), Some(FLOAT_ABSENT));
    assert_eq!((ir.text(STRING_RANKING1_NAME), ir.text(STRING_RIVAL)), (Some("".into()), Some("".into())));
    assert_eq!((ir.text(STRING_IR_NAME), ir.text(STRING_IR_USER_NAME)), (Some("Service".into()), Some("guest".into())));
}

#[test]
fn the_position_of_the_ranking_list_is_the_offset_over_the_size() {
    let fetched = fetched();
    let ir = IrState { browser: Some(IrBrowser { offset: 2, ..IrBrowser::of_ranking(true, Some(&fetched)) }), ..IrState::default() };
    let none = IrState { browser: Some(IrBrowser { offset: 2, ..IrBrowser::of_ranking(true, None) }), ..IrState::default() };

    assert_eq!(ir.rate(RATE_RANKING_POSITION), Some(2.0 / 7.0));
    assert_eq!(none.rate(RATE_RANKING_POSITION), Some(2.0), "with no ranking the list is one long");
}

//! The ranking cluster answers from how the submission of the run stands, and answers the absent
//! value of its kind for everything the service has not sent.

use rbms_config::{Config, SortMode};
use rbms_judge::{AssistGrade, ClearType, clear_type_id};
use rbms_render::skin_render::frame::{BarKind, SongBar};
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT, SkinHost};
use rbms_skin::timer::{TimerState, timer_id};
use rbms_store::RecordLamp;

use super::*;
use crate::format::lamp_label_color;
use crate::ir_outcome::{IrReport, IrStatus};
use crate::ir_ranking::{RANKING_PAGE_LIMIT, RankingBoard, RankingPage, RankingRow, RankingState};
use crate::skin_host::result::snapshot::ReplaySlot;
use crate::skin_host::select::{SelectInputs, SelectShown};
use crate::skin_host::{Cluster, IdSpace, ROUTES, ScreenHost};

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
    graded_row(lamp, AssistGrade::None, rank)
}

/// A row of a ranking on `lamp` with the assist grade `grade` inside it.
fn graded_row(lamp: ClearType, grade: AssistGrade, rank: Option<u32>) -> RankingRow {
    let (label, color) = lamp_label_color(lamp, grade);
    RankingRow {
        rank,
        player: "Player".to_string(),
        ex_score: 100,
        record_lamp: RecordLamp { clear: clear_type_id(lamp), assist_grade: grade.id() },
        lamp: label,
        lamp_color: color,
        fast_slow: None,
        replay_id: None,
        played_at: 0,
        ..Default::default()
    }
}

/// A graded light-assist clear is named for its grade and still counted on the light-assist lamp,
/// as every row is counted by its lamp's id and not by what it is called.
#[test]
fn a_graded_light_assist_clear_is_counted_on_the_light_assist_lamp() {
    let rows = AssistGrade::ALL.into_iter().map(|grade| graded_row(ClearType::LightAssistEasy, grade, None)).chain([row(ClearType::Easy, None)]).collect();
    let board = RankingState::Ready(Box::new(RankingBoard { rows, you: None, rivals: Vec::new(), ..Default::default() }));
    let browser = browser_of(Some(&board));
    let ir = browsing(&browser);
    assert_eq!(ir.integer(NUMBER_IR_PLAYER_LIGHTASSIST), Some(5), "a row named ASSIST EASY to ASSIST EXHARD fell out of the spread");
    assert_eq!(ir.integer(NUMBER_IR_PLAYER_EASY), Some(1));
    assert_eq!(ir.integer(NUMBER_IR_TOTALPLAYER), Some(6));
}

/// A fetched board of seven scores: three failed, one clear, two hard clears and one max, with the
/// player third.
fn fetched() -> RankingState {
    let rows = [ClearType::Max, ClearType::Hard, ClearType::Hard, ClearType::Normal, ClearType::Failed, ClearType::Failed, ClearType::Failed]
        .into_iter()
        .map(|lamp| row(lamp, None))
        .collect();
    RankingState::Ready(Box::new(RankingBoard { rows, you: Some(row(ClearType::Hard, Some(3))), rivals: Vec::new(), ..Default::default() }))
}

/// The browser of a service that holds `ranking` for the chart under the cursor, with the list
/// scrolled to its top.
fn browser_of(ranking: Option<&RankingState>) -> IrBrowser {
    IrBrowser::of_ranking(true, ranking, 0)
}

fn browsing(browser: &IrBrowser) -> IrState<'_> {
    IrState { browser: Some(browser), service_name: "Service", user_name: "guest", ..IrState::default() }
}

#[test]
fn a_browser_with_no_service_holds_no_ranking_whatever_the_cache_has() {
    let offline = IrBrowser::of_ranking(false, Some(&fetched()), 0);

    assert_eq!(offline, IrBrowser { online: false, board: None, offset: 0 });
    assert_eq!(IrBrowser::of_ranking(true, None, 0).board, None, "and a service with nothing fetched yet holds none either");
}

#[test]
fn the_ranking_follows_the_fetch_from_being_asked_to_being_answered_or_refused() {
    let access = |state: &RankingState| IrBrowser::of_ranking(true, Some(state), 0).board.map(|board| board.access);

    assert_eq!(access(&RankingState::Loading), Some(IrAccess::Accessing));
    assert_eq!(access(&RankingState::Failed("timeout".to_string())), Some(IrAccess::Failed));
    assert_eq!(access(&fetched()), Some(IrAccess::Finished));
}

#[test]
fn the_connection_timers_of_the_browser_are_on_only_while_the_ranking_is_in_their_state() {
    let on = |state: Option<&RankingState>| IrBrowser::of_ranking(true, state, 0).timers().on().map(|id| id.0).collect::<Vec<_>>();

    assert_eq!(on(None), Vec::<i32>::new());
    assert_eq!(on(Some(&RankingState::Loading)), [timer_id::IR_CONNECT_BEGIN.0], "the begin timer ends with the fetch, unlike a result screen's");
    assert_eq!(on(Some(&fetched())), [timer_id::IR_CONNECT_SUCCESS.0]);
    assert_eq!(on(Some(&RankingState::Failed(String::new()))), [timer_id::IR_CONNECT_FAIL.0]);
}

#[test]
fn the_options_of_the_browser_follow_the_state_of_the_ranking() {
    let options = |state: Option<&RankingState>| {
        let browser = browser_of(state);
        let ir = browsing(&browser);
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
    let empty = RankingState::Ready(Box::new(RankingBoard { rows: Vec::new(), you: None, rivals: Vec::new(), ..Default::default() }));
    assert_eq!(browsing(&browser_of(Some(&empty))).boolean(OPTION_IR_NOPLAYER), Some(true), "a ranking that came back with nobody on it");
    let no_service = IrBrowser::of_ranking(false, None, 0);
    let offline = IrState { browser: Some(&no_service), ..IrState::default() };
    assert_eq!((offline.boolean(OPTION_OFFLINE), offline.boolean(OPTION_ONLINE)), (Some(true), Some(false)));
    assert_eq!(offline.boolean(OPTION_IR_WAITING), Some(true));
}

#[test]
fn the_place_of_the_player_and_the_size_of_the_ranking_are_read_once_it_is_fetched() {
    let fetched = fetched();
    let browser = browser_of(Some(&fetched));
    let ir = browsing(&browser);

    assert_eq!((ir.integer(NUMBER_IR_RANK), ir.integer(NUMBER_IR_TOTALPLAYER), ir.integer(NUMBER_IR_TOTALPLAYER2)), (Some(3), Some(7), Some(7)));
    assert_eq!(ir.integer(NUMBER_IR_PREVRANK), Some(INTEGER_ABSENT), "the browser keeps no earlier place");
    assert_eq!(ir.integer(NUMBER_IR_UPDATE_WAITING_TIME), Some(INTEGER_ABSENT));
    for state in [None, Some(&RankingState::Loading), Some(&RankingState::Failed(String::new()))] {
        let browser = browser_of(state);
        let ir = browsing(&browser);
        assert_eq!((ir.integer(NUMBER_IR_RANK), ir.integer(NUMBER_IR_TOTALPLAYER)), (Some(INTEGER_ABSENT), Some(INTEGER_ABSENT)));
    }
    let unplaced =
        RankingState::Ready(Box::new(RankingBoard { rows: vec![row(ClearType::Hard, Some(1))], you: None, rivals: Vec::new(), ..Default::default() }));
    assert_eq!(browsing(&browser_of(Some(&unplaced))).integer(NUMBER_IR_RANK), Some(0), "a player who is not on it holds no place");
}

#[test]
fn the_scores_are_counted_onto_the_lamps_and_read_as_counts_and_shares() {
    let fetched = fetched();
    let browser = browser_of(Some(&fetched));
    let ir = browsing(&browser);

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
    let loading = browser_of(Some(&RankingState::Loading));
    let ir = browsing(&loading);

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
    let empty = RankingState::Ready(Box::new(RankingBoard { rows: Vec::new(), you: None, rivals: Vec::new(), ..Default::default() }));
    let nobody = browser_of(Some(&empty));
    let ir = browsing(&nobody);
    assert_eq!(
        (ir.integer(NUMBER_IR_PLAYER_FAILED), ir.integer(NUMBER_IR_PLAYER_FAILED_RATE), ir.float(FLOAT_IR_PLAYER_FAILED_RATE)),
        (Some(0), Some(INTEGER_ABSENT), Some(FLOAT_ABSENT)),
        "no share of nobody"
    );
}

#[test]
fn the_rivals_are_not_read_from_the_board() {
    let fetched = fetched();
    let browser = browser_of(Some(&fetched));
    let ir = browsing(&browser);

    for id in NUMBER_RIVAL_PERFECT..=NUMBER_RIVAL_POOR_RATE {
        assert_eq!(ir.integer(id), Some(INTEGER_ABSENT), "rival number {id}");
    }
    assert_eq!(ir.float(FLOAT_RIVAL_PERFECT_RATE), Some(FLOAT_ABSENT));
    assert_eq!(ir.text(STRING_RIVAL), Some("".into()));
    assert_eq!((ir.text(STRING_IR_NAME), ir.text(STRING_IR_USER_NAME)), (Some("Service".into()), Some("guest".into())));
}

#[test]
fn the_position_of_the_ranking_list_is_the_offset_over_the_size() {
    let fetched = fetched();
    let scrolled = IrBrowser::of_ranking(true, Some(&fetched), 2);
    let none = IrBrowser { offset: 2, ..IrBrowser::of_ranking(true, None, 0) };
    let ir = IrState { browser: Some(&scrolled), ..IrState::default() };
    let unfetched = IrState { browser: Some(&none), ..IrState::default() };

    assert_eq!(scrolled.offset, 2);
    assert_eq!(ir.rate(RATE_RANKING_POSITION), Some(2.0 / 7.0));
    assert_eq!(unfetched.rate(RATE_RANKING_POSITION), Some(2.0), "with no ranking the list is one long");
}

/// A row of a ranking: `player` with `ex_score` on `lamp`, in the place `rank` when the service
/// numbered it.
fn scored(player: &str, ex_score: u32, lamp: ClearType, rank: Option<u32>) -> RankingRow {
    RankingRow { player: player.to_string(), ex_score, ..row(lamp, rank) }
}

fn ranking_of(rows: Vec<RankingRow>) -> RankingState {
    RankingState::Ready(Box::new(RankingBoard { rows, ..Default::default() }))
}

/// Twelve scores, the best first: player `P0` with 1000 points down to `P11` with 890, one place
/// below the other.
fn twelve() -> RankingState {
    ranking_of((0..12u32).map(|place| scored(&format!("P{place}"), 1000 - place * 10, ClearType::Hard, Some(place + 1))).collect())
}

/// What one slot of the list reads: the EX score, the place, whose it is, the lamp and the name.
type SlotReading = (i32, i32, i32, i32, String);

/// What every slot reads while the list has no score there.
fn nothing() -> SlotReading {
    (INTEGER_ABSENT, INTEGER_ABSENT, IMAGE_INDEX_ABSENT, IMAGE_INDEX_ABSENT, String::new())
}

fn list_of(ir: &IrState<'_>) -> Vec<SlotReading> {
    (0..RANKING_SLOTS as i32)
        .map(|slot| {
            (
                ir.integer(NUMBER_RANKING1_EXSCORE + slot).expect("answered"),
                ir.integer(NUMBER_RANKING1_CLEAR + slot).expect("answered"),
                ir.image_index(NUMBER_RANKING1_EXSCORE + slot).expect("answered"),
                ir.image_index(NUMBER_RANKING1_CLEAR + slot).expect("answered"),
                ir.text(STRING_RANKING1_NAME + slot).expect("answered").into_owned(),
            )
        })
        .collect()
}

#[test]
fn the_list_a_skin_draws_shows_ten_scores_from_the_top_of_the_ranking() {
    let twelve = twelve();
    let browser = browser_of(Some(&twelve));
    let list = list_of(&browsing(&browser));

    let hard = i32::from(clear_type_id(ClearType::Hard));
    assert_eq!(list.len(), 10);
    assert_eq!(list[0], (1000, 1, 0, hard, "P0".to_string()));
    assert_eq!(list[9], (910, 10, 0, hard, "P9".to_string()));
}

#[test]
fn the_list_scrolled_down_starts_at_the_score_it_is_scrolled_to_and_runs_out_with_the_ranking() {
    let twelve = twelve();
    let browser = IrBrowser::of_ranking(true, Some(&twelve), 5);
    let list = list_of(&browsing(&browser));

    assert_eq!((list[0].0, list[0].1, list[0].4.as_str()), (950, 6, "P5"));
    assert_eq!((list[6].0, list[6].1, list[6].4.as_str()), (890, 12, "P11"));
    for (slot, absent) in list.iter().enumerate().skip(7) {
        assert_eq!(absent, &nothing(), "slot {slot} is past the last score");
    }
    let past_the_end = IrBrowser::of_ranking(true, Some(&twelve), 12);
    assert!(list_of(&browsing(&past_the_end)).iter().all(|slot| slot == &nothing()), "nothing is left to show past the last");
    let far_away = IrBrowser::of_ranking(true, Some(&twelve), usize::MAX);
    assert!(list_of(&browsing(&far_away)).iter().all(|slot| slot == &nothing()), "an offset that cannot be added to is past the end too");
}

#[test]
fn a_ranking_not_fetched_has_an_empty_list() {
    for state in [None, Some(&RankingState::Loading), Some(&RankingState::Failed(String::new())), Some(&ranking_of(Vec::new()))] {
        let browser = browser_of(state);
        assert!(list_of(&browsing(&browser)).iter().all(|slot| slot == &nothing()), "{state:?}");
    }
    let no_service = IrBrowser::of_ranking(false, Some(&twelve()), 0);
    assert!(list_of(&browsing(&no_service)).iter().all(|slot| slot == &nothing()), "a browser with no service holds no list");
}

#[test]
fn a_tie_shares_its_place_and_a_score_the_service_did_not_number_takes_its_position() {
    let ranking = ranking_of(vec![
        scored("A", 300, ClearType::Hard, Some(1)),
        scored("B", 300, ClearType::Hard, Some(1)),
        scored("C", 200, ClearType::Hard, Some(3)),
        scored("D", 100, ClearType::Hard, None),
    ]);
    let browser = browser_of(Some(&ranking));
    let places = list_of(&browsing(&browser)).into_iter().take(4).map(|slot| slot.1).collect::<Vec<_>>();

    assert_eq!(places, [1, 1, 3, 4]);
}

#[test]
fn the_player_and_the_rivals_are_told_apart_in_the_list_and_the_player_wins_over_a_rival() {
    let ranking = ranking_of(vec![
        scored("Someone", 400, ClearType::Hard, Some(1)),
        RankingRow { is_rival: true, ..scored("Rival", 300, ClearType::Hard, Some(2)) },
        RankingRow { is_you: true, ..scored("You", 200, ClearType::Hard, Some(3)) },
        RankingRow { is_you: true, is_rival: true, ..scored("Yourself", 100, ClearType::Hard, Some(4)) },
    ]);
    let browser = browser_of(Some(&ranking));
    let kinds = list_of(&browsing(&browser)).into_iter().take(4).map(|slot| slot.2).collect::<Vec<_>>();

    assert_eq!(kinds, [0, 2, 1, 1], "none, a rival, the player, the player");
}

#[test]
fn a_score_is_on_the_lamp_it_was_made_on_and_a_graded_light_assist_clear_is_still_the_light_assist_lamp() {
    let lamps = [ClearType::Failed, ClearType::Normal, ClearType::Max, ClearType::NoPlay];
    let mut rows = lamps.map(|lamp| scored("P", 100, lamp, None)).to_vec();
    rows.push(RankingRow { player: "P".to_string(), ..graded_row(ClearType::LightAssistEasy, AssistGrade::Hard, None) });
    let ranking = ranking_of(rows);
    let browser = browser_of(Some(&ranking));
    let on = list_of(&browsing(&browser)).into_iter().take(5).map(|slot| slot.3).collect::<Vec<_>>();

    assert_eq!(on, [1, 5, 10, 0, 3]);
}

#[test]
fn a_page_found_apart_is_part_of_the_list_where_its_place_in_the_ranking_is() {
    let top = RankingBoard {
        rows: (0..RANKING_PAGE_LIMIT).map(|place| scored("Top", 5000 - place, ClearType::Failed, None)).collect(),
        your_page: Some(RankingPage {
            page: 3,
            rows: (0..5).map(|place| RankingRow { is_you: place == 1, ..scored(&format!("Near{place}"), 900 - place, ClearType::Hard, None) }).collect(),
        }),
        ..Default::default()
    };
    let ranking = RankingState::Ready(Box::new(top));
    let first_of_page = usize::try_from(2 * RANKING_PAGE_LIMIT).expect("a small number");

    let browser = IrBrowser::of_ranking(true, Some(&ranking), first_of_page - 2);
    let list = list_of(&browsing(&browser));
    assert_eq!(list[0], nothing(), "the pages between are not held");
    assert_eq!(list[1], nothing());
    assert_eq!((list[2].0, list[2].1, list[2].4.as_str()), (900, i32::try_from(first_of_page).expect("small") + 1, "Near0"));
    assert_eq!((list[3].2, list[3].4.as_str()), (1, "Near1"), "and the player's own score is the player's");
    assert_eq!((list[6].4.as_str(), &list[7]), ("Near4", &nothing()), "the page is five long");

    let ir = browsing(&browser);
    let received = i32::try_from(RANKING_PAGE_LIMIT).expect("a small number") + 5;
    assert_eq!(
        (ir.integer(NUMBER_IR_TOTALPLAYER), ir.integer(NUMBER_IR_PLAYER_FAILED), ir.integer(NUMBER_IR_PLAYER_HARD)),
        (Some(received), Some(100), Some(5))
    );
}

#[test]
fn the_result_screen_s_list_still_has_no_values() {
    let result = state(IrLink::new(true, IrPhase::Finished { succeeded: true }));

    for slot in 0..RANKING_SLOTS as i32 {
        assert_eq!(result.integer(NUMBER_RANKING1_EXSCORE + slot), Some(INTEGER_ABSENT));
        assert_eq!(result.integer(NUMBER_RANKING1_CLEAR + slot), Some(INTEGER_ABSENT));
        assert_eq!(result.image_index(NUMBER_RANKING1_EXSCORE + slot), Some(IMAGE_INDEX_ABSENT));
        assert_eq!(result.image_index(NUMBER_RANKING1_CLEAR + slot), Some(IMAGE_INDEX_ABSENT));
        assert_eq!(result.text(STRING_RANKING1_NAME + slot).as_deref(), Some(""));
    }
}

#[test]
fn a_host_lent_the_browser_reads_the_list_through_the_ranking_cluster_for_a_chart_and_not_for_a_folder() {
    let config = Config::default();
    let twelve = twelve();
    let ir = IrBrowser::of_ranking(true, Some(&twelve), 3);
    let on = |kind: BarKind| {
        let bars = [SongBar::new(kind, "")];
        SelectShown::of(SelectInputs {
            bars: &bars,
            cursor: 0,
            chart: None,
            course: None,
            directory: Vec::new(),
            mode_filter: None,
            sort: SortMode::Title,
            panel: None,
            selected_replay: None,
            config: &config,
            ir: ir.clone(),
        })
    };
    let timers = TimerState::new();

    let chart = on(BarKind::Song { exists: true });
    let mut host = ScreenHost::new(0, &timers);
    host.show_select(&chart);
    assert_eq!((host.integer(NUMBER_RANKING1_EXSCORE), host.integer(NUMBER_RANKING1_CLEAR)), (970, 4));
    assert_eq!((host.image_index(NUMBER_RANKING1_EXSCORE), host.image_index(NUMBER_RANKING1_CLEAR)), (0, i32::from(clear_type_id(ClearType::Hard))));
    assert_eq!(host.text(STRING_RANKING1_NAME), "P3");
    assert_eq!(host.rate(RATE_RANKING_POSITION), Some(3.0 / 12.0), "the position is the offset over the scores received");
    assert_eq!(
        (host.integer(NUMBER_RANKING10_EXSCORE - 1), host.integer(NUMBER_RANKING10_EXSCORE)),
        (890, INTEGER_ABSENT),
        "the list runs out after the twelfth"
    );

    let folder = on(BarKind::Folder);
    let mut host = ScreenHost::new(0, &timers);
    host.show_select(&folder);
    assert_eq!(host.integer(NUMBER_RANKING1_EXSCORE), INTEGER_ABSENT, "a folder is not asked of the service, so its list is empty");
    assert_eq!(host.text(STRING_RANKING1_NAME), "");
}

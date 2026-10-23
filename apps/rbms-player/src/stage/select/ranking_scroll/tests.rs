//! The list of scores a skin draws from the ranking of the chart under the cursor, and the slider it
//! scrolls by: where a share of the way down lands, when it starts over and when the next page of the
//! ranking is asked for.
//!
//! The score server is a scripted double ([`RankingDouble`]), so nothing here reaches a network.

use std::sync::Arc;

use rbms_render::SongBars;
use rbms_skin::property::SkinHost;
use rbms_skin::property::generated::{NUMBER_RANKING1_EXSCORE, RATE_RANKING_POSITION};
use rbms_skin::timer::TimerState;

use super::super::skinned_tests::{Browser, SHORT_STEP};
use super::*;
use crate::KeyCode;
use crate::ir_ext::MultiIr;
use crate::ir_ranking::tests::{RankingDouble, record};
use crate::ir_ranking::{RANKING_PAGE_LIMIT, RankingPage, RankingRow, build_board};
use crate::skin_host::select::{BrowserLent, SelectShown};
use crate::skin_host::{Cluster, ClusterRequest, ScreenHost};

/// The score server the browsers here read, which nothing ever resolves.
const SERVER_URL: &str = "http://ranking.invalid";

/// The chart the cursor of [`Browser::three_songs`] starts on.
const FIRST_CHART: &str = "md5-alpha";

/// The EX score of the player on the first place of a board [`ranked`] makes; each place after it is
/// worth one point less.
const TOP_EX: u32 = 2000;

/// How many players the short board of these tests has.
const SHORT_BOARD: u32 = 30;

/// A share of the way down a full page that is inside its last ten scores, and one that is not.
const NEAR_THE_END: f32 = 0.97;
const MIDWAY: f32 = 0.5;

/// The EX score the place `place` (counted from nought) holds on a board [`ranked`] makes.
fn ex_at(place: u32) -> i32 {
    i32::try_from(TOP_EX - place).expect("an EX score fits")
}

/// Three charts with a score server and, for the first, a board of `rows` players, already fetched.
fn ranked(tag: &str, rows: u32) -> Browser {
    let mut browser = Browser::three_songs(tag);
    let shared = &mut browser.app.shared;
    shared.config.network.server_url = Some(SERVER_URL.to_owned());
    shared.multi_ir = MultiIr::from_network(&shared.config.network);
    shared.profile_servers = vec![Arc::new(RankingDouble::new(Vec::new()))];
    let records: Vec<_> = (0..rows).map(|n| record(&format!("p{n}"), &format!("Player {n}"), TOP_EX - n, None)).collect();
    shared.ranking_cache.insert(FIRST_CHART.to_owned(), RankingState::Ready(Box::new(build_board(&records, None, &[]))));
    browser.frame_after(SHORT_STEP);
    browser
}

fn write_position(browser: &mut Browser, share: f32) {
    browser.app.shared.skin_requests().push(Cluster::Select, ClusterRequest::WriteRate { id: RATE_RANKING_POSITION, value: share });
    browser.frame_after(SHORT_STEP);
}

/// The frame of the browser as its skin is lent it, with the list scrolled as the browser holds it.
fn lent_frame(browser: &mut Browser) -> SelectShown {
    let cursor = browser.state.cursor(&browser.app.shared);
    let wheel = SongBars::new(browser.state.bars.wheel(0), cursor);
    let lent = BrowserLent { ranking_offset: browser.state.browse.ranking.offset(), ..BrowserLent::default() };
    browser.app.shared.select_shown(Some(&wheel), &lent)
}

/// The EX score a skin reads in the first slot of its list, and where the slider stands, as the host
/// answers them for the frame the browser lends.
fn first_slot_and_slider(browser: &mut Browser) -> (i32, Option<f32>) {
    let shown = lent_frame(browser);
    let timers = TimerState::new();
    let mut host = ScreenHost::new(0, &timers);
    host.show_select(&shown);
    (host.integer(NUMBER_RANKING1_EXSCORE), host.rate(RATE_RANKING_POSITION))
}

#[test]
fn a_chart_other_than_the_one_a_list_was_scrolled_for_is_read_from_its_top() {
    let mut scroll = RankingScroll::default();
    scroll.settle(Some("alpha"));
    scroll.offset = 40;

    scroll.settle(Some("alpha"));
    assert_eq!(scroll.offset(), 40, "the same chart keeps its place");
    scroll.settle(Some("beta"));
    assert_eq!(scroll.offset(), 0);
    scroll.offset = 7;
    scroll.settle(None);
    assert_eq!(scroll.offset(), 0, "a cursor on no chart has no ranking to be down");
    scroll.settle(None);
    assert_eq!(scroll.offset(), 0);
}

#[test]
fn the_slider_scrolls_the_list_by_its_share_of_the_scores_received_and_reads_that_share_back() {
    let mut browser = ranked("scroll-share", SHORT_BOARD);
    assert_eq!(first_slot_and_slider(&mut browser), (ex_at(0), Some(0.0)), "the list starts at the top");

    write_position(&mut browser, MIDWAY);
    assert_eq!(browser.state.browse.ranking.offset(), 15, "half of thirty scores");
    assert_eq!(first_slot_and_slider(&mut browser), (ex_at(15), Some(MIDWAY)), "the sixteenth place is the first on show");

    write_position(&mut browser, 0.99);
    assert_eq!(browser.state.browse.ranking.offset(), 29, "the share is cut down, as the reference cuts it");
    assert_eq!(first_slot_and_slider(&mut browser).0, ex_at(29));
}

#[test]
fn a_share_outside_nought_to_one_leaves_the_list_where_it_is() {
    let mut browser = ranked("scroll-outside", SHORT_BOARD);
    write_position(&mut browser, MIDWAY);

    for share in [1.0, 1.5, -0.1, f32::NAN] {
        write_position(&mut browser, share);
        assert_eq!(browser.state.browse.ranking.offset(), 15, "a share of {share} moved the list");
    }
}

#[test]
fn the_list_starts_again_from_the_top_on_another_chart() {
    let mut browser = ranked("scroll-chart", SHORT_BOARD);
    write_position(&mut browser, MIDWAY);
    assert_eq!(browser.state.browse.ranking.offset(), 15);

    assert_eq!(browser.arrow(KeyCode::ArrowDown), 1);
    assert_eq!(browser.state.browse.ranking.offset(), 0, "the next chart was lent the scroll of the one above it");
    assert_eq!(browser.arrow(KeyCode::ArrowUp), 0);
    assert_eq!(browser.state.browse.ranking.offset(), 0, "and the first chart did not keep its own");
}

#[test]
fn a_chart_with_no_ranking_is_one_place_long_for_the_slider() {
    let mut browser = ranked("scroll-none", SHORT_BOARD);
    assert_eq!(browser.arrow(KeyCode::ArrowDown), 1);
    assert_eq!(browser.app.shared.ranking_list_extent(), 1);

    write_position(&mut browser, 0.9);
    assert_eq!(browser.state.browse.ranking.offset(), 0, "nothing received, nothing to scroll down");
}

#[test]
fn the_course_tab_has_no_ranking_list_to_scroll() {
    let mut browser = ranked("scroll-courses", SHORT_BOARD);
    write_position(&mut browser, MIDWAY);
    assert_eq!(browser.state.browse.ranking.offset(), 15);

    browser.state.tab = SelectTab::Courses;
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.browse.ranking.offset(), 0, "a course is no chart of the song list");
    write_position(&mut browser, MIDWAY);
    assert_eq!(browser.state.browse.ranking.offset(), 0);
}

#[test]
fn the_extent_of_the_list_is_every_score_received_the_page_found_apart_among_them() {
    let mut browser = ranked("scroll-extent", SHORT_BOARD);
    assert_eq!(browser.app.shared.ranking_list_extent(), 30);

    let records: Vec<_> = (0..RANKING_PAGE_LIMIT).map(|n| record(&format!("p{n}"), &format!("Player {n}"), TOP_EX - n, None)).collect();
    let mut board = build_board(&records, None, &[]);
    board.your_page = Some(RankingPage { page: 3, rows: vec![RankingRow::default(); 5] });
    browser.app.shared.ranking_cache.insert(FIRST_CHART.to_owned(), RankingState::Ready(Box::new(board)));
    assert_eq!(browser.app.shared.ranking_list_extent(), 105);

    browser.app.shared.ranking_cache.insert(FIRST_CHART.to_owned(), RankingState::Loading);
    assert_eq!(browser.app.shared.ranking_list_extent(), 1, "a ranking on its way has nothing received yet");
}

#[test]
fn a_list_scrolled_to_the_end_of_the_scores_received_asks_for_the_next_page_and_one_short_of_it_does_not() {
    let mut browser = ranked("scroll-page", RANKING_PAGE_LIMIT);
    assert!(matches!(browser.app.shared.ranking_cache.peek(FIRST_CHART), Some(RankingState::Ready(board)) if board.more), "a full page may be followed");

    write_position(&mut browser, MIDWAY);
    assert!(browser.app.shared.ranking_rx.is_none(), "the middle of the page asked for nothing");
    write_position(&mut browser, 0.0);
    assert!(browser.app.shared.ranking_rx.is_none(), "the top of the page asked for nothing");

    write_position(&mut browser, NEAR_THE_END);
    assert!(browser.app.shared.ranking_rx.is_some(), "the end of the page did not ask for the next one");
}

#[test]
fn a_ranking_that_came_back_short_has_no_next_page_to_ask_for_and_its_last_score_ends_the_list() {
    let mut browser = ranked("scroll-short", SHORT_BOARD);

    write_position(&mut browser, 0.99);
    assert!(browser.app.shared.ranking_rx.is_none(), "a page that came back short is the whole ranking");
    assert_eq!(first_slot_and_slider(&mut browser).0, ex_at(29));
    let board = lent_frame(&mut browser).ir.board.expect("the board is lent");
    assert_eq!(board.slots.iter().filter(|slot| slot.is_some()).count(), 1, "one score is left after the twenty-ninth");
}

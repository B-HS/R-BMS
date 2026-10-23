//! The song-select IR ranking panel: the line list a [`RankingState`] turns into, and the overlay
//! that draws it over the detail column.
//!
//! [`panel_lines`] is the single source of the text on screen, so a headless test asserts on
//! exactly the strings [`render_ranking_panel`] draws.

use rbms_model::translations::Catalog;
use rbms_render::{Color, RANK_BANDS, Rect, Renderer, dj_rank, draw_text, draw_text_right, fit_text, select_layout, text_width, theme};
use winit::keyboard::KeyCode;

use crate::format::fmt_datetime;
use crate::ir_ranking::{RankingBoard, RankingRow, RankingState};
use crate::target::NOTE_MAX_EX;
use crate::text::ranking::RankingText;

/// What a key does while the panel has focus; `None` leaves the key to the select screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PanelAction {
    Close,
    Up,
    Down,
    PlayReplay,
    PreviousProfile,
    NextProfile,
    /// Put the selection on the local player's row, fetching the page it is on when the board does
    /// not hold it yet: SELECT of a controller.
    YourRow,
    /// Fetch the page of the leaderboard after the ones the board holds: START of a controller.
    NextPage,
}

/// The panel's key map.
///
/// It has to stay closed with respect to the select screen's bindings: every key select treats as
/// "open this row" must act here too, or the panel would consume Enter to play a replay while
/// ArrowRight fell through and launched the chart instead.
pub(crate) fn panel_action(code: KeyCode) -> Option<PanelAction> {
    Some(match code {
        KeyCode::KeyI | KeyCode::Escape | KeyCode::ArrowLeft => PanelAction::Close,
        KeyCode::ArrowUp => PanelAction::Up,
        KeyCode::ArrowDown => PanelAction::Down,
        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::ArrowRight => PanelAction::PlayReplay,
        KeyCode::KeyQ => PanelAction::PreviousProfile,
        KeyCode::KeyE => PanelAction::NextProfile,
        _ => return None,
    })
}

/// Panel chrome: the rule along the top and its thickness, title baseline, hint baseline, first
/// row top, the row pitch and height, the side margin of a row, and how far down a row its text
/// stands.
const RULE_H: f32 = 2.0;
const TITLE_Y: f32 = 12.0;
const HINT_Y: f32 = 38.0;
const ROWS_Y: f32 = 62.0;
const ROW_PITCH: f32 = 30.0;
const ROW_H: f32 = 26.0;
const ROW_MARGIN: f32 = 4.0;
const ROW_TEXT_Y: f32 = 5.0;

/// Column offsets inside the panel. The rank and the player start from its left edge; the DJ LEVEL
/// letter, the lamp and the replay tag start from its right edge, and so do the right edges of the
/// EX score and of the date, so the player's room is what the columns to its right leave.
const RANK_X: f32 = 12.0;
const NAME_X: f32 = 58.0;
const LEVEL_FROM_RIGHT: f32 = 306.0;
const SCORE_FROM_RIGHT: f32 = 222.0;
const LAMP_FROM_RIGHT: f32 = 212.0;
const DATE_FROM_RIGHT: f32 = 44.0;
const TAIL_FROM_RIGHT: f32 = 10.0;

/// The gap between the player's name and the DJ LEVEL column, and the narrowest the name is cut to.
const NAME_GAP: f32 = 8.0;
const MIN_NAME_W: f32 = 48.0;

/// The dan badges beside a name: how many a row shows, how a badge is set, and the most room the
/// label of one is given.
const MAX_DAN_BADGES: usize = 2;
const BADGE_SCALE: f32 = 1.0;
const BADGE_PAD_X: f32 = 5.0;
const BADGE_GAP: f32 = 6.0;
const BADGE_H: f32 = 16.0;
const BADGE_TEXT_Y: f32 = 3.0;
const BADGE_LABEL_MAX_W: f32 = 64.0;

/// How a row of the player or of a rival stands out: the width of the rail along its left edge, and
/// how strongly its ground is tinted.
const RAIL_W: f32 = 3.0;
const YOU_TINT_ALPHA: u8 = 36;
const RIVAL_TINT_ALPHA: u8 = 20;

const TITLE_SCALE: f32 = 1.8;
const HINT_SCALE: f32 = 1.1;
const HEADING_SCALE: f32 = 1.3;
const ROW_SCALE: f32 = 1.3;

/// The gap between the chart's name and the hint beside it in the panel's head.
const HEAD_GAP: f32 = 16.0;

/// Panel title.
pub(crate) const PANEL_TITLE: &str = "IR RANKING";

/// Placeholder for a value the server has no row for.
pub(crate) const NO_VALUE: &str = "-";

/// Shown instead of rows while the fetch is in flight.
pub(crate) const LOADING_TEXT: &str = "LOADING...";

/// Shown when the panel is open but nothing is focused, or the chart has no md5.
pub(crate) const NO_CHART_TEXT: &str = "SELECT A CHART";

/// Shown when the leaderboard came back empty.
pub(crate) const NO_SCORES_TEXT: &str = "NO SCORES YET";

/// Shown when no score server is configured, so there is nothing to fetch.
pub(crate) const OFFLINE_TEXT: &str = "IR OFF - SET SERVER URL IN SETTINGS";

/// Tag marking a row whose replay can be downloaded and played.
pub(crate) const REPLAY_TAG: &str = "REP";

/// The label of the line that holds the local player's own best.
pub(crate) const YOU_LABEL: &str = "YOU";

/// What a line of the panel is, which decides how it is set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LineRole {
    /// Words only: a message, the profile in use, or a note on how much of the board is shown.
    Message,
    /// Another player's score.
    Player,
    /// A score of the local player's: a row of the leaderboard, or the line that holds the best.
    You,
    /// A score of a rival's.
    Rival,
}

/// One drawn line of the panel. Every string here is drawn verbatim.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PanelLine {
    pub(crate) role: LineRole,
    pub(crate) rank: String,
    pub(crate) label: String,
    /// The labels of the dans the player shows beside the name, none from a server that sends none.
    pub(crate) dans: Vec<String>,
    /// The DJ LEVEL of the score and the colour it is written in; `None` when the server did not say
    /// how many notes the chart had.
    pub(crate) level: Option<(&'static str, Color)>,
    pub(crate) score: String,
    pub(crate) lamp: &'static str,
    pub(crate) lamp_color: Color,
    /// The day the score was played on, in UTC.
    pub(crate) date: String,
    pub(crate) tail: String,
    pub(crate) replay_id: Option<String>,
}

impl PanelLine {
    /// A line that carries no score: a message, or a player with no record on this chart.
    fn message(text: impl Into<String>) -> PanelLine {
        PanelLine::bare(LineRole::Message, String::new(), text.into(), String::new())
    }

    fn bare(role: LineRole, rank: String, label: String, score: String) -> PanelLine {
        PanelLine {
            role,
            rank,
            label,
            dans: Vec::new(),
            level: None,
            score,
            lamp: "",
            lamp_color: Color::GRAY,
            date: String::new(),
            tail: String::new(),
            replay_id: None,
        }
    }

    fn from_row(label: String, row: &RankingRow) -> PanelLine {
        let role = match (row.is_you, row.is_rival) {
            (true, _) => LineRole::You,
            (false, true) => LineRole::Rival,
            (false, false) => LineRole::Player,
        };
        PanelLine {
            dans: row.dans.iter().map(|dan| dan.label.clone()).collect(),
            level: dj_level(row),
            lamp: row.lamp,
            lamp_color: row.lamp_color,
            date: played_date(row.played_at),
            tail: if row.replay_id.is_some() { REPLAY_TAG.to_string() } else { String::new() },
            replay_id: row.replay_id.clone(),
            ..PanelLine::bare(role, row.rank.map(|rank| format!("#{rank}")).unwrap_or_else(|| NO_VALUE.to_string()), label, row.ex_score.to_string())
        }
    }

    /// Blank row for a player the server has no record for.
    fn empty_for(role: LineRole, label: String) -> PanelLine {
        PanelLine::bare(role, NO_VALUE.to_string(), label, NO_VALUE.to_string())
    }
}

/// The DJ LEVEL a row's score reaches against the most EX its chart gives, or `None` for a row
/// whose chart has no note count to measure against.
fn dj_level(row: &RankingRow) -> Option<(&'static str, Color)> {
    let ceiling = row.total_notes.checked_mul(NOTE_MAX_EX).filter(|ceiling| *ceiling > 0)?;
    Some(RANK_BANDS[dj_rank(row.ex_score, ceiling)])
}

/// The day a score was played on, `YYYY-MM-DD` in UTC, or a dash for a row that reports no time.
fn played_date(played_at: i64) -> String {
    if played_at <= 0 {
        return NO_VALUE.to_string();
    }
    let stamp = fmt_datetime(played_at);
    stamp.split_once(' ').map(|(date, _)| date.to_string()).unwrap_or(stamp)
}

fn board_lines(board: &RankingBoard) -> Vec<PanelLine> {
    let language = rbms_render::text::language();
    let mut lines = Vec::new();
    if board.rows.is_empty() {
        lines.push(PanelLine::message(NO_SCORES_TEXT));
    }
    lines.extend(board.rows.iter().map(|row| PanelLine::from_row(row.player.clone(), row)));
    if board.more {
        lines.push(PanelLine::message(RankingText::TopShownWithMore.fill(language, &[&board.rows.len()])));
    }
    if let Some(held) = &board.your_page
        && let Some(first) = held.rows.first()
    {
        let from = first.rank.map_or_else(|| NO_VALUE.to_string(), |rank| rank.to_string());
        lines.push(PanelLine::message(RankingText::YourPageFrom.fill(language, &[&from])));
        lines.extend(held.rows.iter().map(|row| PanelLine::from_row(row.player.clone(), row)));
    }
    lines.push(match &board.you {
        Some(row) => PanelLine::from_row(YOU_LABEL.to_string(), row),
        None => PanelLine::empty_for(LineRole::You, YOU_LABEL.to_string()),
    });
    for (id, row) in &board.rivals {
        let label = format!("RIVAL {id}");
        lines.push(match row {
            Some(row) => PanelLine::from_row(label, row),
            None => PanelLine::empty_for(LineRole::Rival, label),
        });
    }
    lines
}

/// The panel's only line when the client has no score server configured.
pub(crate) fn offline_lines() -> Vec<PanelLine> {
    vec![PanelLine::message(OFFLINE_TEXT)]
}

pub(crate) fn profile_line(label: &str, position: usize, count: usize) -> PanelLine {
    PanelLine::message(format!("IR {position}/{count}: {label}"))
}

/// The panel's lines for a chart's state. `None` means no chart is focused.
pub(crate) fn panel_lines(state: Option<&RankingState>) -> Vec<PanelLine> {
    match state {
        None => vec![PanelLine::message(NO_CHART_TEXT)],
        Some(RankingState::Loading) => vec![PanelLine::message(LOADING_TEXT)],
        Some(RankingState::Failed(message)) => {
            vec![PanelLine::message(format!("ERROR: {message}"))]
        }
        Some(RankingState::Ready(board)) => board_lines(board),
    }
}

/// How many rows fit in the panel body.
pub(crate) fn visible_rows() -> usize {
    (((select_layout().detail_rect.h - ROWS_Y) / ROW_PITCH).floor() as usize).max(1)
}

/// Index of the first drawn line so `sel` stays visible: the selection is centred until the list
/// end is reached.
pub(crate) fn scroll_start(len: usize, sel: usize) -> usize {
    let visible = visible_rows();
    if len <= visible {
        return 0;
    }
    sel.saturating_sub(visible / 2).min(len - visible)
}

/// Draw the panel over the detail column and return the clickable region of each drawn line,
/// tagged with its index into [`panel_lines`].
pub(crate) fn render_ranking_panel<R: Renderer>(r: &mut R, lines: &[PanelLine], sel: usize, focused: bool, can_switch_profile: bool) -> Vec<(Rect, usize)> {
    render_ranking_panel_at(r, select_layout().detail_rect, None, lines, sel, focused, can_switch_profile)
}

/// [`render_ranking_panel`] in `panel` rather than over the detail column, for a screen that has put
/// its own list there. [`visible_rows`] counts the rows the detail column holds, so a panel meant to
/// show all of them is as tall as that column is.
///
/// `heading` names the chart the board is of; the hint takes its place at the left of the head when
/// there is none.
pub(crate) fn render_ranking_panel_at<R: Renderer>(
    r: &mut R,
    panel: Rect,
    heading: Option<&str>,
    lines: &[PanelLine],
    sel: usize,
    focused: bool,
    can_switch_profile: bool,
) -> Vec<(Rect, usize)> {
    let th = theme();
    let mut hot = Vec::new();
    r.fill_rect(panel, th.panel);
    r.fill_rect(Rect::new(panel.x, panel.y, panel.w, RULE_H), th.divider);
    draw_text(r, panel.x + RANK_X, panel.y + TITLE_Y, TITLE_SCALE, if focused { th.accent } else { th.text }, PANEL_TITLE);
    if let Some(own) = lines.iter().rfind(|line| line.role == LineRole::You) {
        let place = RankingText::YouRank.fill(rbms_render::text::language(), &[&own.rank]);
        draw_text_right(r, panel.x + panel.w - RANK_X, panel.y + TITLE_Y, TITLE_SCALE, th.accent, &place);
    }
    let hint = match (focused, can_switch_profile) {
        (true, true) => "UP DOWN MOVE   Q/E PROFILE   ENTER REPLAY   I CLOSE",
        (true, false) => "UP DOWN MOVE   ENTER REPLAY   I CLOSE",
        (false, _) => "I FOCUS PANEL",
    };
    match heading {
        Some(chart) => {
            draw_text_right(r, panel.x + panel.w - RANK_X, panel.y + HINT_Y, HINT_SCALE, th.text_muted, hint);
            let room = (panel.w - RANK_X * 2.0 - text_width(hint, HINT_SCALE) - HEAD_GAP).max(MIN_NAME_W);
            draw_text(r, panel.x + RANK_X, panel.y + HINT_Y, HEADING_SCALE, th.text_dim, &fit_text(chart, HEADING_SCALE, room));
        }
        None => draw_text(r, panel.x + RANK_X, panel.y + HINT_Y, HINT_SCALE, th.text_muted, hint),
    }

    let start = scroll_start(lines.len(), sel);
    for (slot, index) in (start..(start + visible_rows()).min(lines.len())).enumerate() {
        let line = &lines[index];
        let y = panel.y + ROWS_Y + slot as f32 * ROW_PITCH;
        let rect = Rect::new(panel.x + ROW_MARGIN, y, panel.w - ROW_MARGIN * 2.0, ROW_H);
        let selected = focused && index == sel;
        draw_row_ground(r, rect, line.role, selected);
        match line.role {
            LineRole::Message => {
                let room = panel.w - NAME_X - RANK_X;
                draw_text(
                    r,
                    panel.x + NAME_X,
                    y + ROW_TEXT_Y,
                    ROW_SCALE,
                    if selected { th.text } else { th.text_dim },
                    &fit_text(&line.label, ROW_SCALE, room),
                );
            }
            _ => draw_player_row(r, panel, y + ROW_TEXT_Y, line, selected),
        }
        hot.push((rect, index));
    }
    hot
}

/// What lies behind a row: the focus colour when it is selected, and for a row of the player's or
/// of a rival's a tint of its own and a rail along its left edge.
fn draw_row_ground<R: Renderer>(r: &mut R, rect: Rect, role: LineRole, selected: bool) {
    let th = theme();
    let marks = match role {
        LineRole::You => Some((Color { a: YOU_TINT_ALPHA, ..th.accent }, th.accent)),
        LineRole::Rival => Some((Color { a: RIVAL_TINT_ALPHA, ..th.text }, th.text_dim)),
        LineRole::Message | LineRole::Player => None,
    };
    if selected {
        r.fill_rect(rect, th.row_focus);
    } else if let Some((tint, _)) = marks {
        r.fill_rect(rect, tint);
    }
    if let Some((_, rail)) = marks {
        r.fill_rect(Rect::new(rect.x, rect.y, RAIL_W, rect.h), rail);
    }
}

/// The columns of a row that holds a score. `text_y` is the line its text stands on.
fn draw_player_row<R: Renderer>(r: &mut R, panel: Rect, text_y: f32, line: &PanelLine, selected: bool) {
    let th = theme();
    let right = panel.x + panel.w;
    let body = if selected { th.text } else { th.text_dim };
    let name = match line.role {
        LineRole::You if !selected => th.accent,
        LineRole::Rival if !selected => th.text,
        _ => body,
    };
    draw_text(r, panel.x + RANK_X, text_y, ROW_SCALE, th.text_muted, &line.rank);
    draw_name(r, panel, text_y, line, name);
    if let Some((letter, color)) = line.level {
        draw_text(r, right - LEVEL_FROM_RIGHT, text_y, ROW_SCALE, color, letter);
    }
    draw_text_right(r, right - SCORE_FROM_RIGHT, text_y, ROW_SCALE, body, &line.score);
    if !line.lamp.is_empty() {
        draw_text(r, right - LAMP_FROM_RIGHT, text_y, ROW_SCALE, line.lamp_color, line.lamp);
    }
    draw_text_right(r, right - DATE_FROM_RIGHT, text_y, ROW_SCALE, th.text_muted, &line.date);
    if !line.tail.is_empty() {
        draw_text_right(r, right - TAIL_FROM_RIGHT, text_y, ROW_SCALE, th.text_muted, &line.tail);
    }
}

/// The player's name, cut to the room the columns to its right leave, and the badges of the dans
/// they show after it. A name that would be left too narrow by the badges is given the room
/// instead, and the badges are left out.
fn draw_name<R: Renderer>(r: &mut R, panel: Rect, text_y: f32, line: &PanelLine, color: Color) {
    let th = theme();
    let labels: Vec<String> = line.dans.iter().take(MAX_DAN_BADGES).map(|dan| fit_text(dan, BADGE_SCALE, BADGE_LABEL_MAX_W)).collect();
    let widths: Vec<f32> = labels.iter().map(|label| text_width(label, BADGE_SCALE) + BADGE_PAD_X * 2.0).collect();
    let room = panel.w - NAME_X - LEVEL_FROM_RIGHT - NAME_GAP;
    let badges_room: f32 = widths.iter().map(|width| width + BADGE_GAP).sum();
    let badges_fit = room - badges_room >= MIN_NAME_W;
    let name_room = if badges_fit { room - badges_room } else { room.max(MIN_NAME_W) };
    let name = fit_text(&line.label, ROW_SCALE, name_room);
    draw_text(r, panel.x + NAME_X, text_y, ROW_SCALE, color, &name);
    if !badges_fit {
        return;
    }
    let mut x = panel.x + NAME_X + text_width(&name, ROW_SCALE) + BADGE_GAP;
    let top = text_y - ROW_TEXT_Y + (ROW_H - BADGE_H) * 0.5;
    for (label, width) in labels.iter().zip(&widths) {
        r.fill_rect(Rect::new(x, top, *width, BADGE_H), th.button);
        draw_text(r, x + BADGE_PAD_X, top + BADGE_TEXT_Y, BADGE_SCALE, th.text, label);
        x += width + BADGE_GAP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_ranking::tests::{record, record_with_replay};
    use crate::ir_ranking::{RankingPage, RankingState, build_board};
    use rbms_ir::JudgeBreakdown;
    use rbms_model::Language;
    use rbms_render::CpuCanvas;
    use serde_json::json;

    const CANVAS_W: u32 = 1280;
    const CANVAS_H: u32 = 720;

    fn ready_state() -> RankingState {
        let mut top = record_with_replay("a", "Alice", 1500, None, "rep-a");
        top.judge = Some(JudgeBreakdown { fast: 12, slow: 8, ..Default::default() });
        let rows = [top, record("b", "Bob", 1400, None)];
        let you = record("me", "Me", 1200, Some(7));
        let rivals = [("friend".to_string(), Some(record("friend", "Friend", 1300, Some(3)))), ("ghost".to_string(), None)];
        RankingState::Ready(Box::new(build_board(&rows, Some(&you), &rivals)))
    }

    fn labels(lines: &[PanelLine]) -> Vec<&str> {
        lines.iter().map(|line| line.label.as_str()).collect()
    }

    fn a_row(rank: u32, player: &str, ex_score: u32) -> RankingRow {
        RankingRow { rank: Some(rank), player: player.to_string(), ex_score, lamp: "HARD", ..RankingRow::default() }
    }

    struct Installed;

    impl Installed {
        fn language(language: Language) -> Installed {
            rbms_render::text::set_language(language);
            Installed
        }
    }

    impl Drop for Installed {
        fn drop(&mut self) {
            rbms_render::text::set_language(Language::English);
        }
    }

    #[test]
    fn a_ready_board_lists_the_leaderboard_then_you_then_the_rivals() {
        let state = ready_state();
        let lines = panel_lines(Some(&state));
        assert_eq!(labels(&lines), vec!["Alice", "Bob", "YOU", "RIVAL friend", "RIVAL ghost"]);
        assert_eq!(lines[0].rank, "#1");
        assert_eq!(lines[0].score, "1500");
        assert_eq!(lines[0].tail, REPLAY_TAG, "the replay tag stands alone in the tail");
        assert_eq!(lines[0].replay_id.as_deref(), Some("rep-a"));
        assert_eq!(lines[1].tail, "", "a row without a replay has an empty tail");
        assert_eq!(lines[2].rank, "#7", "the YOU row shows the server's rank");
        assert_eq!(lines[2].score, "1200");
        assert_eq!(lines[4].rank, NO_VALUE, "a rival with no record shows dashes");
        assert_eq!(lines[4].score, NO_VALUE);
    }

    #[test]
    fn a_player_without_a_record_still_gets_a_you_row() {
        let state = RankingState::Ready(Box::new(build_board(&[record("a", "Alice", 1500, None)], None, &[])));
        let lines = panel_lines(Some(&state));
        assert_eq!(labels(&lines), vec!["Alice", "YOU"]);
        assert_eq!(lines[1].rank, NO_VALUE);
        assert_eq!(lines[1].score, NO_VALUE);
        assert_eq!(lines[1].role, LineRole::You);
    }

    #[test]
    fn an_empty_leaderboard_says_so_above_the_you_row() {
        let state = RankingState::Ready(Box::new(build_board(&[], None, &[])));
        assert_eq!(labels(&panel_lines(Some(&state))), vec![NO_SCORES_TEXT, "YOU"]);
    }

    #[test]
    fn loading_failed_and_no_chart_each_render_one_message_line() {
        assert_eq!(labels(&offline_lines()), vec![OFFLINE_TEXT]);
        assert_eq!(labels(&panel_lines(None)), vec![NO_CHART_TEXT]);
        assert_eq!(labels(&panel_lines(Some(&RankingState::Loading))), vec![LOADING_TEXT]);
        let failed = RankingState::Failed("server down".into());
        assert_eq!(labels(&panel_lines(Some(&failed))), vec!["ERROR: server down"]);
        for line in offline_lines().iter().chain(&panel_lines(None)).chain(&panel_lines(Some(&RankingState::Loading))).chain(&panel_lines(Some(&failed))) {
            assert_eq!(line.role, LineRole::Message, "{line:?}");
        }
    }

    #[test]
    fn a_row_carries_the_dans_the_dj_level_and_the_day_it_was_played() {
        let mut scored = record("a", "Alice", 900, None);
        scored.extra.insert(
            "dans".to_string(),
            json!([{"system": "stella", "name": "Stella", "label": "SL 8"}, {"system": "genoside2018_sp", "name": "SP", "label": "二段"}]),
        );
        let others = [record("b", "Bob", 800, None), record("c", "Cy", 400, None), record("d", "Di", 0, None)];
        let rows = [&[scored][..], &others[..]].concat();
        let state = RankingState::Ready(Box::new(build_board(&rows, None, &[])));
        let lines = panel_lines(Some(&state));
        assert_eq!(lines[0].dans, vec!["SL 8".to_string(), "二段".to_string()]);
        assert!(lines[1].dans.is_empty(), "a server that sends no dans leaves the badges out");
        let levels: Vec<_> = lines[..4].iter().map(|line| line.level.map(|(letter, _)| letter)).collect();
        assert_eq!(levels, vec![Some("AAA"), Some("AA"), Some("D"), Some("F")], "500 notes give 1000 EX: 90% AAA, 80% AA, 40% D, none F");
        assert_eq!(lines[0].level.map(|(_, color)| color), Some(RANK_BANDS[7].1), "the letter is written in its band's colour");
        assert_eq!(lines[0].date, "2023-11-14", "1_700_000_000_000 ms is the 14th of November 2023 in UTC");
    }

    #[test]
    fn a_row_without_a_note_count_or_a_time_leaves_the_level_and_the_date_blank() {
        let unknown = RankingRow { total_notes: 0, played_at: 0, ..a_row(1, "Alice", 1500) };
        let line = PanelLine::from_row("Alice".to_string(), &unknown);
        assert_eq!(line.level, None);
        assert_eq!(line.date, NO_VALUE);
        let absurd = RankingRow { total_notes: u32::MAX, ..a_row(1, "Alice", 1500) };
        assert_eq!(PanelLine::from_row("Alice".to_string(), &absurd).level, None, "a note count that cannot be doubled is no count");
    }

    #[test]
    fn the_row_of_the_player_and_of_a_rival_are_told_from_the_rest() {
        let mut me = a_row(2, "Me", 1000);
        me.is_you = true;
        let mut friend = a_row(3, "Friend", 900);
        friend.is_rival = true;
        let board = RankingBoard {
            rows: vec![a_row(1, "Alice", 1100), me.clone(), friend.clone()],
            you: Some(me),
            rivals: vec![("f".into(), Some(friend))],
            ..RankingBoard::default()
        };
        let lines = panel_lines(Some(&RankingState::Ready(Box::new(board))));
        let roles: Vec<_> = lines.iter().map(|line| line.role).collect();
        assert_eq!(roles, vec![LineRole::Player, LineRole::You, LineRole::Rival, LineRole::You, LineRole::Rival]);
        assert_eq!(lines[3].label, YOU_LABEL);
    }

    #[test]
    fn a_board_the_server_has_more_of_says_how_much_it_shows() {
        let board = RankingBoard { rows: vec![a_row(1, "Alice", 1100), a_row(2, "Bob", 1000)], more: true, pages: 1, ..RankingBoard::default() };
        let lines = panel_lines(Some(&RankingState::Ready(Box::new(board.clone()))));
        assert_eq!(labels(&lines), vec!["Alice", "Bob", "TOP 2 SHOWN, MORE ON THE NEXT PAGE", "YOU"]);
        assert_eq!(lines[2].role, LineRole::Message);

        let whole = RankingBoard { more: false, ..board };
        assert_eq!(
            labels(&panel_lines(Some(&RankingState::Ready(Box::new(whole))))),
            vec!["Alice", "Bob", "YOU"],
            "a board that is whole says nothing of pages"
        );
    }

    #[test]
    fn the_page_the_players_row_is_on_is_set_apart_below_the_pages_held() {
        let held = RankingPage { page: 4, rows: vec![a_row(301, "Near", 500), a_row(302, "Me", 499)] };
        let board = RankingBoard { rows: vec![a_row(1, "Alice", 1100)], more: true, pages: 1, your_page: Some(held), ..RankingBoard::default() };
        let lines = panel_lines(Some(&RankingState::Ready(Box::new(board))));
        assert_eq!(labels(&lines), vec!["Alice", "TOP 1 SHOWN, MORE ON THE NEXT PAGE", "YOUR PAGE, FROM RANK 301", "Near", "Me", "YOU"]);
        assert_eq!(lines[3].rank, "#301");
    }

    #[test]
    fn the_notes_on_pages_are_written_in_the_language_the_screens_are_in() {
        let _korean = Installed::language(Language::Korean);
        let board = RankingBoard { rows: vec![a_row(1, "Alice", 1100)], more: true, pages: 1, ..RankingBoard::default() };
        let lines = panel_lines(Some(&RankingState::Ready(Box::new(board))));
        assert_eq!(lines[1].label, "상위 1명을 표시했습니다. 다음 페이지가 더 있습니다");
    }

    #[test]
    fn scrolling_keeps_the_selection_on_screen() {
        let visible = visible_rows();
        assert_eq!(scroll_start(visible, visible - 1), 0, "a list that fits never scrolls");
        let long = visible * 3;
        assert_eq!(scroll_start(long, 0), 0);
        assert_eq!(scroll_start(long, long - 1), long - visible, "the last row is reachable");
        let middle = scroll_start(long, long / 2);
        assert!(middle <= long / 2 && long / 2 < middle + visible, "the selection stays inside the window");
    }

    #[test]
    fn the_panel_renders_headless_without_panicking_and_draws_its_lines() {
        let state = ready_state();
        let lines = panel_lines(Some(&state));
        let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
        let hot = render_ranking_panel(&mut canvas, &lines, 0, true, false);
        let panel = select_layout().detail_rect;
        assert_eq!(hot.len(), lines.len(), "every drawn line is clickable");
        assert!(hot.iter().all(|(rect, _)| rect.x >= panel.x && rect.x + rect.w <= panel.x + panel.w));
        assert_ne!(canvas.signature_hash(8, 8), CpuCanvas::new(CANVAS_W, CANVAS_H).signature_hash(8, 8), "the panel put pixels on the canvas");
    }

    #[test]
    fn every_panel_state_renders_headless() {
        for state in [None, Some(RankingState::Loading), Some(RankingState::Failed("boom".into())), Some(ready_state())] {
            let lines = panel_lines(state.as_ref());
            let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
            let hot = render_ranking_panel(&mut canvas, &lines, lines.len().saturating_sub(1), false, false);
            assert_eq!(hot.len(), lines.len().min(visible_rows()));
        }
    }

    #[test]
    fn a_list_longer_than_the_panel_only_draws_what_fits() {
        let rows: Vec<_> = (0..visible_rows() + 10).map(|i| record(&format!("p{i}"), &format!("Player {i}"), 1000 + i as u32, None)).collect();
        let state = RankingState::Ready(Box::new(build_board(&rows, None, &[])));
        let lines = panel_lines(Some(&state));
        let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
        let hot = render_ranking_panel(&mut canvas, &lines, lines.len() - 1, true, false);
        assert_eq!(hot.len(), visible_rows());
        assert!(hot.iter().any(|(_, index)| *index == lines.len() - 1), "the selected line is one of the drawn rows");
    }

    /// The pixel in the middle of the rail of the `slot`th row of a panel drawn at `panel`.
    fn rail_pixel(canvas: &CpuCanvas, panel: Rect, slot: usize) -> Color {
        let y = panel.y + ROWS_Y + slot as f32 * ROW_PITCH + ROW_H * 0.5;
        canvas.pixel_at((panel.x + ROW_MARGIN + RAIL_W * 0.5) as u32, y as u32)
    }

    #[test]
    fn a_row_of_the_player_and_of_a_rival_has_a_rail_and_the_others_have_none() {
        let mut me = a_row(2, "Me", 1000);
        me.is_you = true;
        let mut friend = a_row(3, "Friend", 900);
        friend.is_rival = true;
        let board = RankingBoard { rows: vec![a_row(1, "Alice", 1100), me, friend], ..RankingBoard::default() };
        let lines = panel_lines(Some(&RankingState::Ready(Box::new(board))));
        let panel = select_layout().detail_rect;
        let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
        render_ranking_panel_at(&mut canvas, panel, None, &lines, 0, true, false);
        let th = theme();
        assert_ne!(rail_pixel(&canvas, panel, 0), th.accent, "the row of another player has no rail");
        assert_eq!(rail_pixel(&canvas, panel, 1), th.accent, "the player's row has the accent rail");
        assert_eq!(rail_pixel(&canvas, panel, 2), th.text_dim, "a rival's row has the dimmer rail");
    }

    /// The panel drawn at the detail column with `heading`, on a fresh canvas.
    fn drawn(lines: &[PanelLine], heading: Option<&str>) -> CpuCanvas {
        let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
        render_ranking_panel_at(&mut canvas, select_layout().detail_rect, heading, lines, 0, true, false);
        canvas
    }

    #[test]
    fn a_dan_is_drawn_as_a_badge_after_the_name() {
        let mut scored = record("a", "Alice", 900, None);
        scored.extra.insert("dans".to_string(), json!([{"system": "stella", "name": "Stella", "label": "SL 8"}]));
        let lines = panel_lines(Some(&RankingState::Ready(Box::new(build_board(&[scored], None, &[])))));
        let panel = select_layout().detail_rect;
        let inside_the_badge = (panel.x + NAME_X + text_width("Alice", ROW_SCALE) + BADGE_GAP + 1.0) as u32;
        let middle_of_the_row = (panel.y + ROWS_Y + ROW_H * 0.5) as u32;
        assert_eq!(drawn(&lines, None).pixel_at(inside_the_badge, middle_of_the_row), theme().button, "the badge's ground follows the name");
        let bare = [PanelLine { dans: Vec::new(), ..lines[0].clone() }];
        assert_ne!(drawn(&bare, None).pixel_at(inside_the_badge, middle_of_the_row), theme().button, "a row without a dan has no badge");
    }

    #[test]
    fn a_chart_named_in_the_head_changes_the_head_and_nothing_below_it() {
        let lines = panel_lines(Some(&ready_state()));
        let panel = select_layout().detail_rect;
        let (plain, headed) = (drawn(&lines, None), drawn(&lines, Some("TITLE OF THE CHART")));
        let first_row = (panel.y + ROWS_Y) as u32;
        let differ_in = |ys: std::ops::Range<u32>| ys.into_iter().any(|y| (0..CANVAS_W).any(|x| headed.pixel_at(x, y) != plain.pixel_at(x, y)));
        assert!(differ_in(panel.y as u32..first_row), "the heading did not change the head");
        assert!(!differ_in(first_row..CANVAS_H), "the heading reached the rows");
    }

    #[test]
    fn a_heading_too_long_for_the_head_is_cut_inside_the_panel() {
        let lines = panel_lines(Some(&ready_state()));
        let panel = select_layout().detail_rect;
        let canvas = drawn(&lines, Some(&"WIDE ".repeat(200)));
        let past_the_panel = (panel.x + panel.w) as u32;
        assert!((0..CANVAS_H).all(|y| (past_the_panel..CANVAS_W).all(|x| canvas.pixel_at(x, y).a == 0)), "the heading ran out of the panel");
    }

    #[test]
    fn the_panel_answers_every_key_the_select_screen_uses_to_open_a_row() {
        for code in [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::ArrowRight] {
            assert_eq!(panel_action(code), Some(PanelAction::PlayReplay), "{code:?} launches the chart if the panel lets it through");
        }
    }

    #[test]
    fn the_panel_closes_on_back_and_moves_on_the_vertical_arrows() {
        for code in [KeyCode::KeyI, KeyCode::Escape, KeyCode::ArrowLeft] {
            assert_eq!(panel_action(code), Some(PanelAction::Close), "{code:?}");
        }
        assert_eq!(panel_action(KeyCode::ArrowUp), Some(PanelAction::Up));
        assert_eq!(panel_action(KeyCode::ArrowDown), Some(PanelAction::Down));
    }

    #[test]
    fn unrelated_keys_fall_through_to_the_select_screen() {
        for code in [KeyCode::Tab, KeyCode::KeyR, KeyCode::Slash, KeyCode::F3] {
            assert_eq!(panel_action(code), None, "{code:?}");
        }
    }
}

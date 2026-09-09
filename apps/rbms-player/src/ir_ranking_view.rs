//! The song-select IR ranking panel: the line list a [`RankingState`] turns into, and the overlay
//! that draws it over the detail column.
//!
//! [`panel_lines`] is the single source of the text on screen, so a headless test asserts on
//! exactly the strings [`render_ranking_panel`] draws.

use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_right, fit_text, theme};
use winit::keyboard::KeyCode;

use crate::ir_ranking::{RankingBoard, RankingRow, RankingState};

/// What a key does while the panel has focus; `None` leaves the key to the select screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PanelAction {
    Close,
    Up,
    Down,
    PlayReplay,
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
        _ => return None,
    })
}

/// The panel covers the detail column of the select screen (`select.rs` `DETAIL_X`/`DETAIL_W`
/// between `TOP` and `BOTTOM`), so the song list stays readable behind it.
pub(crate) const PANEL_X: f32 = 632.0;
pub(crate) const PANEL_Y: f32 = 60.0;
pub(crate) const PANEL_W: f32 = 616.0;
pub(crate) const PANEL_H: f32 = 600.0;

/// Panel chrome: title baseline, hint baseline, first row top, and the row pitch.
const TITLE_Y: f32 = 12.0;
const HINT_Y: f32 = 38.0;
const ROWS_Y: f32 = 62.0;
const ROW_PITCH: f32 = 30.0;
const ROW_H: f32 = 26.0;

/// Column offsets inside the panel, from its left edge (rank, player) and its right edge (score,
/// lamp, tail).
const RANK_X: f32 = 12.0;
const LABEL_X: f32 = 58.0;
const SCORE_RIGHT: f32 = 250.0;
const LAMP_X: f32 = 238.0;
const TAIL_RIGHT: f32 = 10.0;

const TITLE_SCALE: f32 = 1.8;
const HINT_SCALE: f32 = 1.1;
const ROW_SCALE: f32 = 1.3;

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

/// One drawn line of the panel. Every string here is drawn verbatim.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PanelLine {
    pub(crate) rank: String,
    pub(crate) label: String,
    pub(crate) score: String,
    pub(crate) lamp: &'static str,
    pub(crate) lamp_color: Color,
    pub(crate) tail: String,
    pub(crate) replay_id: Option<String>,
}

impl PanelLine {
    /// A line that carries no score: a message, or a player with no record on this chart.
    fn message(text: impl Into<String>) -> PanelLine {
        PanelLine { rank: String::new(), label: text.into(), score: String::new(), lamp: "", lamp_color: Color::GRAY, tail: String::new(), replay_id: None }
    }

    fn from_row(label: String, row: &RankingRow) -> PanelLine {
        let mut tail = match row.fast_slow {
            Some((fast, slow)) => format!("F{fast}/S{slow}"),
            None => String::new(),
        };
        if row.replay_id.is_some() {
            if !tail.is_empty() {
                tail.push_str("  ");
            }
            tail.push_str(REPLAY_TAG);
        }
        PanelLine {
            rank: row.rank.map(|rank| format!("#{rank}")).unwrap_or_else(|| NO_VALUE.to_string()),
            label,
            score: row.ex_score.to_string(),
            lamp: row.lamp,
            lamp_color: row.lamp_color,
            tail,
            replay_id: row.replay_id.clone(),
        }
    }

    /// Blank row for a player the server has no record for.
    fn empty_for(label: String) -> PanelLine {
        PanelLine { rank: NO_VALUE.to_string(), label, score: NO_VALUE.to_string(), lamp: "", lamp_color: Color::GRAY, tail: String::new(), replay_id: None }
    }
}

fn board_lines(board: &RankingBoard) -> Vec<PanelLine> {
    let mut lines = Vec::new();
    if board.rows.is_empty() {
        lines.push(PanelLine::message(NO_SCORES_TEXT));
    }
    for row in &board.rows {
        lines.push(PanelLine::from_row(row.player.clone(), row));
    }
    lines.push(match &board.you {
        Some(row) => PanelLine::from_row("YOU".to_string(), row),
        None => PanelLine::empty_for("YOU".to_string()),
    });
    for (id, row) in &board.rivals {
        let label = format!("RIVAL {id}");
        lines.push(match row {
            Some(row) => PanelLine::from_row(label, row),
            None => PanelLine::empty_for(label),
        });
    }
    lines
}

/// The panel's only line when the client has no score server configured.
pub(crate) fn offline_lines() -> Vec<PanelLine> {
    vec![PanelLine::message(OFFLINE_TEXT)]
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
    (((PANEL_H - ROWS_Y) / ROW_PITCH).floor() as usize).max(1)
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
pub(crate) fn render_ranking_panel<R: Renderer>(r: &mut R, lines: &[PanelLine], sel: usize, focused: bool) -> Vec<(Rect, usize)> {
    let th = theme();
    let mut hot = Vec::new();
    r.fill_rect(Rect::new(PANEL_X, PANEL_Y, PANEL_W, PANEL_H), th.panel);
    r.fill_rect(Rect::new(PANEL_X, PANEL_Y, PANEL_W, 2.0), th.divider);
    draw_text(r, PANEL_X + RANK_X, PANEL_Y + TITLE_Y, TITLE_SCALE, if focused { th.accent } else { th.text }, PANEL_TITLE);
    let hint = if focused { "UP DOWN MOVE   ENTER REPLAY   I CLOSE" } else { "I FOCUS PANEL" };
    draw_text(r, PANEL_X + RANK_X, PANEL_Y + HINT_Y, HINT_SCALE, th.text_muted, hint);

    let start = scroll_start(lines.len(), sel);
    for (slot, index) in (start..(start + visible_rows()).min(lines.len())).enumerate() {
        let line = &lines[index];
        let y = PANEL_Y + ROWS_Y + slot as f32 * ROW_PITCH;
        let rect = Rect::new(PANEL_X + 4.0, y, PANEL_W - 8.0, ROW_H);
        if focused && index == sel {
            r.fill_rect(rect, th.row_focus);
        }
        let text_color = if focused && index == sel { th.text } else { th.text_dim };
        draw_text(r, PANEL_X + RANK_X, y + 5.0, ROW_SCALE, th.text_muted, &line.rank);
        let label_width = PANEL_W - LABEL_X - SCORE_RIGHT - 8.0;
        draw_text(r, PANEL_X + LABEL_X, y + 5.0, ROW_SCALE, text_color, &fit_text(&line.label, ROW_SCALE, label_width));
        draw_text_right(r, PANEL_X + PANEL_W - SCORE_RIGHT, y + 5.0, ROW_SCALE, text_color, &line.score);
        if !line.lamp.is_empty() {
            draw_text(r, PANEL_X + PANEL_W - LAMP_X, y + 5.0, ROW_SCALE, line.lamp_color, line.lamp);
        }
        if !line.tail.is_empty() {
            draw_text_right(r, PANEL_X + PANEL_W - TAIL_RIGHT, y + 5.0, ROW_SCALE, th.text_muted, &line.tail);
        }
        hot.push((rect, index));
    }
    hot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_ranking::tests::{record, replay_meta};
    use crate::ir_ranking::{RankingState, build_board};
    use rbms_ir::JudgeBreakdown;
    use rbms_render::CpuCanvas;

    const CANVAS_W: u32 = 1280;
    const CANVAS_H: u32 = 720;

    fn ready_state() -> RankingState {
        let mut top = record("a", "Alice", 1500, None);
        top.judge = Some(JudgeBreakdown { fast: 12, slow: 8, ..Default::default() });
        let rows = [top, record("b", "Bob", 1400, None)];
        let you = record("me", "Me", 1200, Some(7));
        let rivals = [("friend".to_string(), Some(record("friend", "Friend", 1300, Some(3)))), ("ghost".to_string(), None)];
        RankingState::Ready(Box::new(build_board(&rows, Some(&you), &rivals, &[replay_meta("rep-a", "a")])))
    }

    fn labels(lines: &[PanelLine]) -> Vec<&str> {
        lines.iter().map(|line| line.label.as_str()).collect()
    }

    #[test]
    fn a_ready_board_lists_the_leaderboard_then_you_then_the_rivals() {
        let state = ready_state();
        let lines = panel_lines(Some(&state));
        assert_eq!(labels(&lines), vec!["Alice", "Bob", "YOU", "RIVAL friend", "RIVAL ghost"]);
        assert_eq!(lines[0].rank, "#1");
        assert_eq!(lines[0].score, "1500");
        assert_eq!(lines[0].tail, "F12/S8  REP", "timing detail and the replay tag share the tail");
        assert_eq!(lines[0].replay_id.as_deref(), Some("rep-a"));
        assert_eq!(lines[1].tail, "", "a row with neither detail nor replay has an empty tail");
        assert_eq!(lines[2].rank, "#7", "the YOU row shows the server's rank");
        assert_eq!(lines[2].score, "1200");
        assert_eq!(lines[4].rank, NO_VALUE, "a rival with no record shows dashes");
        assert_eq!(lines[4].score, NO_VALUE);
    }

    #[test]
    fn a_player_without_a_record_still_gets_a_you_row() {
        let state = RankingState::Ready(Box::new(build_board(&[record("a", "Alice", 1500, None)], None, &[], &[])));
        let lines = panel_lines(Some(&state));
        assert_eq!(labels(&lines), vec!["Alice", "YOU"]);
        assert_eq!(lines[1].rank, NO_VALUE);
        assert_eq!(lines[1].score, NO_VALUE);
    }

    #[test]
    fn an_empty_leaderboard_says_so_above_the_you_row() {
        let state = RankingState::Ready(Box::new(build_board(&[], None, &[], &[])));
        assert_eq!(labels(&panel_lines(Some(&state))), vec![NO_SCORES_TEXT, "YOU"]);
    }

    #[test]
    fn loading_failed_and_no_chart_each_render_one_message_line() {
        assert_eq!(labels(&offline_lines()), vec![OFFLINE_TEXT]);
        assert_eq!(labels(&panel_lines(None)), vec![NO_CHART_TEXT]);
        assert_eq!(labels(&panel_lines(Some(&RankingState::Loading))), vec![LOADING_TEXT]);
        let failed = RankingState::Failed("server down".into());
        assert_eq!(labels(&panel_lines(Some(&failed))), vec!["ERROR: server down"]);
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
        let hot = render_ranking_panel(&mut canvas, &lines, 0, true);
        assert_eq!(hot.len(), lines.len(), "every drawn line is clickable");
        assert!(hot.iter().all(|(rect, _)| rect.x >= PANEL_X && rect.x + rect.w <= PANEL_X + PANEL_W));
        assert_ne!(canvas.signature_hash(8, 8), CpuCanvas::new(CANVAS_W, CANVAS_H).signature_hash(8, 8), "the panel put pixels on the canvas");
    }

    #[test]
    fn every_panel_state_renders_headless() {
        for state in [None, Some(RankingState::Loading), Some(RankingState::Failed("boom".into())), Some(ready_state())] {
            let lines = panel_lines(state.as_ref());
            let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
            let hot = render_ranking_panel(&mut canvas, &lines, lines.len().saturating_sub(1), false);
            assert_eq!(hot.len(), lines.len().min(visible_rows()));
        }
    }

    #[test]
    fn a_list_longer_than_the_panel_only_draws_what_fits() {
        let rows: Vec<_> = (0..visible_rows() + 10).map(|i| record(&format!("p{i}"), &format!("Player {i}"), 1000 + i as u32, None)).collect();
        let state = RankingState::Ready(Box::new(build_board(&rows, None, &[], &[])));
        let lines = panel_lines(Some(&state));
        let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
        let hot = render_ranking_panel(&mut canvas, &lines, lines.len() - 1, true);
        assert_eq!(hot.len(), visible_rows());
        assert!(hot.iter().any(|(_, index)| *index == lines.len() - 1), "the selected line is one of the drawn rows");
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

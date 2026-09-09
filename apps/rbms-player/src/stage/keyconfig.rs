//! The key-config editor: the per-mode lane bindings and the in-play controls, rebound one row at
//! a time. The mode being edited is remembered between visits, so it lives in [`AppShared`].
#![allow(clippy::wildcard_imports)]

use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::*;

/// Width of the editor panel.
const PANEL_W: f32 = 760.0;

/// Vertical layout of the binding rows.
const ROW_TOP: f32 = 112.0;
const ROW_H: f32 = 28.0;
const ROW_GAP: f32 = 2.0;

/// How many rows fit on screen at once.
const VISIBLE_ROWS: usize = 17;

/// The editor's own state: the focused row, whether the next key is being captured for it, and
/// whether the last capture was refused as a collision.
#[derive(Default)]
pub(crate) struct KeyConfigState {
    sel: usize,
    capturing: bool,
    warn: bool,
}

impl KeyConfigState {
    pub(crate) fn new() -> KeyConfigState {
        KeyConfigState::default()
    }

    fn cycle_edit_mode(&mut self, shared: &mut AppShared, d: i32) {
        let all = Mode::ALL;
        let cur = all.iter().position(|m| m.key == shared.kc_edit_mode.key).unwrap_or(0);
        shared.kc_edit_mode = all[((cur as i32 + d).rem_euclid(all.len() as i32)) as usize];
        let len = kc_rows(shared.kc_edit_mode).len();
        if self.sel >= len {
            self.sel = len - 1;
        }
    }

    /// Bind the captured key to the focused row unless it collides with another action.
    fn capture(&mut self, shared: &mut AppShared, code: KeyCode) {
        let rows = kc_rows(shared.kc_edit_mode);
        if code != KeyCode::Escape
            && let Some(row) = rows.get(self.sel)
        {
            if shared.binding_collides(shared.kc_edit_mode, row, code) {
                self.warn = true;
                self.capturing = false;
                return;
            }
            match row {
                KcRow::Control(a) => shared.keyconfig.set_control(*a, code),
                KcRow::Lane(lane) => shared.keyconfig.set_lane(shared.kc_edit_mode, *lane, code),
                KcRow::ModeSelect => {}
            }
        }
        self.warn = false;
        self.capturing = false;
    }
}

impl StageHandler for KeyConfigState {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    /// Key-config editor input. In capture mode the next key (except Esc) is bound to the focused
    /// row unless it collides with another action; otherwise navigate, switch edit-mode, start a
    /// rebind, or save and go back to the settings screen.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if !key.pressed {
            return Transition::Stay;
        }
        if self.capturing {
            self.capture(ctx.shared, key.code);
            return Transition::Stay;
        }
        self.warn = false;
        let rows = kc_rows(ctx.shared.kc_edit_mode);
        match key.code {
            KeyCode::Escape => {
                ctx.shared.keyconfig.save(&ctx.shared.keyconfig_path);
                return Transition::Back;
            }
            KeyCode::ArrowUp => self.sel = self.sel.saturating_sub(1),
            KeyCode::ArrowDown => self.sel = (self.sel + 1).min(rows.len().saturating_sub(1)),
            KeyCode::ArrowLeft if matches!(rows.get(self.sel), Some(KcRow::ModeSelect)) => self.cycle_edit_mode(ctx.shared, -1),
            KeyCode::ArrowRight if matches!(rows.get(self.sel), Some(KcRow::ModeSelect)) => self.cycle_edit_mode(ctx.shared, 1),
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if matches!(rows.get(self.sel), Some(KcRow::Control(_)) | Some(KcRow::Lane(_))) {
                    self.capturing = true;
                }
            }
            _ => {}
        }
        Transition::Stay
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let x0 = (CW as f32 - PANEL_W) * 0.5;
        draw_text(canvas, x0, 40.0, 3.0, th.text, "KEY CONFIG");
        let (hint, hint_col) = if self.warn {
            ("KEY ALREADY BOUND - TRY ANOTHER", Color::RED)
        } else if self.capturing {
            ("PRESS A KEY...   ESC CANCEL", Color::YELLOW)
        } else {
            ("UP DOWN MOVE   ENTER REBIND   LEFT RIGHT EDIT-MODE   ESC SAVE/BACK", th.text_muted)
        };
        draw_text(canvas, x0, 82.0, 1.3, hint_col, hint);
        let edit_mode = ctx.shared.kc_edit_mode;
        let dups = ctx.shared.keyconfig.collisions(edit_mode);
        let rows = kc_rows(edit_mode);
        let start = self.sel.saturating_sub(VISIBLE_ROWS / 2).min(rows.len().saturating_sub(VISIBLE_ROWS.min(rows.len())));
        for (i, ridx) in (start..(start + VISIBLE_ROWS).min(rows.len())).enumerate() {
            let y = ROW_TOP + i as f32 * (ROW_H + ROW_GAP);
            let on = ridx == self.sel;
            let (label, raw) = match &rows[ridx] {
                KcRow::ModeSelect => ("EDIT MODE".to_string(), mode_short(edit_mode).to_string()),
                KcRow::Control(a) => (a.label().to_string(), ctx.shared.keyconfig.control_token(*a).to_string()),
                KcRow::Lane(lane) => {
                    let name = if edit_mode.is_scratch(*lane) { format!("SCRATCH {}", lane + 1) } else { format!("LANE {}", lane + 1) };
                    (name, ctx.shared.keyconfig.lane_token(edit_mode, *lane))
                }
            };
            let is_dup = !matches!(&rows[ridx], KcRow::ModeSelect) && key_from_name(&raw).is_some_and(|k| dups.contains(&k));
            canvas.fill_rect(Rect::new(x0, y, PANEL_W, ROW_H), if on { th.button } else { th.panel });
            draw_text(canvas, x0 + 16.0, y + 8.0, 1.6, if on { th.text } else { th.text_dim }, &label);
            let value = if on && self.capturing {
                "?".to_string()
            } else if raw.is_empty() {
                "-".to_string()
            } else {
                raw
            };
            let vcol = if on && self.capturing {
                Color::YELLOW
            } else if is_dup {
                Color::RED
            } else if on {
                th.accent
            } else {
                th.text
            };
            draw_text_right(canvas, x0 + PANEL_W - 16.0, y + 8.0, 1.6, vcol, &value);
        }
    }
}

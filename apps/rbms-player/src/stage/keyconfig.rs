//! The key-config editor: the per-mode lane bindings and the in-play controls, rebound one row at
//! a time. The mode being edited is remembered between visits, so it lives in [`AppShared`].
#![allow(clippy::wildcard_imports)]

use crate::gamepad::{AnalogMode, PadBinding};
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
                KcRow::ScratchReverse(lane) => shared.keyconfig.set_scratch_reverse(shared.kc_edit_mode, *lane, code),
                KcRow::ModeSelect
                | KcRow::PadDevice
                | KcRow::PadAnalogMode
                | KcRow::PadControl(_)
                | KcRow::PadLane(_)
                | KcRow::PadScratchReverse(_) => {}
            }
        }
        self.warn = false;
        self.capturing = false;
    }
}

impl KeyConfigState {
    /// Step the controller a pad row binds against: the first entry is ANY, then every device gilrs
    /// has opened.
    fn cycle_pad_device(&self, shared: &mut AppShared, delta: i32) {
        let mut names: Vec<Option<String>> = vec![None];
        names.extend(shared.pad.as_ref().map(PadState::device_names).unwrap_or_default().into_iter().map(Some));
        let at = names.iter().position(|name| *name == shared.keyconfig.pad.device_name).unwrap_or(0);
        let next = (at as i32 + delta).rem_euclid(names.len() as i32) as usize;
        shared.keyconfig.pad.device_name = names[next].clone();
    }

    /// Step the turntable algorithm the analog scratch rows are read with.
    fn cycle_analog_mode(&self, shared: &mut AppShared, delta: i32) {
        let all = AnalogMode::ALL;
        let at = all.iter().position(|mode| *mode == shared.keyconfig.pad.analog_mode).unwrap_or(0);
        let next = (at as i32 + delta).rem_euclid(all.len() as i32) as usize;
        shared.keyconfig.pad.analog_mode = all[next];
    }

    /// Bind whatever the controller reports next to the focused pad row. Returns once a binding has
    /// landed; until then the capture stays armed and the next frame asks again.
    fn capture_pad(&mut self, ctx: &mut FrameCtx<'_>) {
        let rows = kc_rows(ctx.shared.kc_edit_mode);
        let Some(row) = rows.get(self.sel).filter(|row| is_pad_row(row)) else {
            return;
        };
        let AppShared { pad, keyconfig, kc_edit_mode, .. } = ctx.shared;
        let Some(state) = pad.as_mut() else {
            return;
        };
        let Some(binding) = state.take_capture(&keyconfig.pad) else {
            return;
        };
        let binding = match row {
            KcRow::PadLane(lane) if kc_edit_mode.is_scratch(*lane) && keyconfig.pad.analog_mode != AnalogMode::Off => {
                binding.as_analog_scratch().unwrap_or(binding)
            }
            _ => binding,
        };
        match row {
            KcRow::PadControl(action) => keyconfig.pad.set_control(*action, Some(binding)),
            KcRow::PadLane(lane) => keyconfig.pad.set_lane(*kc_edit_mode, *lane, Some(binding)),
            KcRow::PadScratchReverse(lane) => keyconfig.pad.set_scratch_reverse(*kc_edit_mode, *lane, Some(binding)),
            _ => {}
        }
        self.capturing = false;
    }
}

/// A pad binding as the editor draws it; an unbound row comes back blank, which the row renderer
/// already draws as a dash.
fn pad_token(binding: Option<PadBinding>) -> String {
    binding.map(PadBinding::label).unwrap_or_default()
}

impl StageHandler for KeyConfigState {
    /// A pad row's capture arrives on the frame poll rather than as a window event, so it is
    /// collected here instead of in `handle_key`.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        if self.capturing {
            self.capture_pad(ctx);
        }
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
            let rows = kc_rows(ctx.shared.kc_edit_mode);
            if rows.get(self.sel).is_some_and(is_pad_row) {
                if key.code == KeyCode::Escape {
                    self.capturing = false;
                }
                return Transition::Stay;
            }
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
            KeyCode::ArrowLeft if matches!(rows.get(self.sel), Some(KcRow::PadDevice)) => self.cycle_pad_device(ctx.shared, -1),
            KeyCode::ArrowRight if matches!(rows.get(self.sel), Some(KcRow::PadDevice)) => self.cycle_pad_device(ctx.shared, 1),
            KeyCode::ArrowLeft if matches!(rows.get(self.sel), Some(KcRow::PadAnalogMode)) => self.cycle_analog_mode(ctx.shared, -1),
            KeyCode::ArrowRight if matches!(rows.get(self.sel), Some(KcRow::PadAnalogMode)) => self.cycle_analog_mode(ctx.shared, 1),
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if matches!(rows.get(self.sel), Some(KcRow::Control(_) | KcRow::Lane(_) | KcRow::ScratchReverse(_))) {
                    self.capturing = true;
                }
                if matches!(rows.get(self.sel), Some(KcRow::PadControl(_) | KcRow::PadLane(_) | KcRow::PadScratchReverse(_))) {
                    let AppShared { pad, keyconfig, .. } = ctx.shared;
                    if let Some(state) = pad.as_mut() {
                        state.drain(&keyconfig.pad);
                    }
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
        ctx.shared.prepare_skin(canvas, SKIN_TYPE_KEY_CONFIG);
        let bound: Vec<String> = (0..ctx.shared.kc_edit_mode.key).map(|lane| ctx.shared.keyconfig.lane_token(ctx.shared.kc_edit_mode, lane)).collect();
        if ctx.shared.draw_keyconfig_skin(canvas, &bound) {
            return;
        }
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
        let pad_dups = ctx.shared.keyconfig.pad.collisions(edit_mode);
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
                KcRow::ScratchReverse(lane) => (format!("SCRATCH {} REVERSE", lane + 1), ctx.shared.keyconfig.scratch_reverse_token(edit_mode, *lane)),
                KcRow::PadDevice => ("PAD DEVICE".to_string(), ctx.shared.keyconfig.pad.device_name.clone().unwrap_or_else(|| "ANY".to_string())),
                KcRow::PadAnalogMode => ("PAD ANALOG".to_string(), ctx.shared.keyconfig.pad.analog_mode.label().to_string()),
                KcRow::PadControl(a) => (format!("PAD {}", a.label()), pad_token(ctx.shared.keyconfig.pad.control_binding(*a))),
                KcRow::PadLane(lane) => {
                    let name = if edit_mode.is_scratch(*lane) { format!("PAD SCRATCH {}", lane + 1) } else { format!("PAD LANE {}", lane + 1) };
                    (name, pad_token(ctx.shared.keyconfig.pad.lane_binding(edit_mode, *lane)))
                }
                KcRow::PadScratchReverse(lane) => {
                    (format!("PAD SCRATCH {} REVERSE", lane + 1), pad_token(ctx.shared.keyconfig.pad.scratch_reverse_binding(edit_mode, *lane)))
                }
            };
            let is_dup = match &rows[ridx] {
                KcRow::ModeSelect | KcRow::PadDevice | KcRow::PadAnalogMode => false,
                KcRow::PadControl(a) => ctx.shared.keyconfig.pad.control_binding(*a).is_some_and(|b| pad_dups.contains(&b)),
                KcRow::PadLane(lane) => ctx.shared.keyconfig.pad.lane_binding(edit_mode, *lane).is_some_and(|b| pad_dups.contains(&b)),
                KcRow::PadScratchReverse(lane) => ctx.shared.keyconfig.pad.scratch_reverse_binding(edit_mode, *lane).is_some_and(|b| pad_dups.contains(&b)),
                _ => key_from_name(&raw).is_some_and(|k| dups.contains(&k)),
            };
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

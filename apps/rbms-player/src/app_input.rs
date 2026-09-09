//! `App` methods: lane/control input mapping, settings + skin/font round-trip, replay
//! playback & analysis, and the key-config editor. Split out of `main.rs`; `use crate::*`
//! pulls in the crate-root types/consts/helpers these methods reference.
#![allow(clippy::wildcard_imports)]
use crate::*;

impl App {
    /// Recompute the visible select list for the current `select_view`.
    pub(crate) fn rebuild_select_items(&mut self) {
        let n = self.songs.len();
        self.select_items = match self.select_view {
            SelectView::Root => {
                let mut items = vec![SelectItem::Folder { label: format!("ALL SONGS ({n})"), target: SelectView::AllSongs }];
                for (ti, levels) in self.table_levels.iter().enumerate() {
                    if levels.is_empty() {
                        continue;
                    }
                    let total: usize = levels.iter().map(|(_, v)| v.len()).sum();
                    let name = self.table_names.get(ti).cloned().unwrap_or_else(|| "TABLE".into());
                    items.push(SelectItem::Folder { label: format!("{name} ({total})"), target: SelectView::TableLevels(ti) });
                }
                items
            }
            SelectView::AllSongs => self.arrange_songs((0..n).collect()).into_iter().map(SelectItem::Song).collect(),
            SelectView::TableLevels(ti) => self
                .table_levels
                .get(ti)
                .map(|levels| {
                    levels
                        .iter()
                        .enumerate()
                        .map(|(li, (level, songs))| SelectItem::Folder {
                            label: format!("LV {level} ({})", songs.len()),
                            target: SelectView::TableLevel(ti, li),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            SelectView::TableLevel(ti, li) => self
                .table_levels
                .get(ti)
                .and_then(|levels| levels.get(li))
                .map(|(_, songs)| self.arrange_songs(songs.clone()).into_iter().map(SelectItem::Song).collect())
                .unwrap_or_default(),
        };
        if self.sel >= self.select_items.len() {
            self.sel = self.select_items.len().saturating_sub(1);
        }
        self.select_gen = self.select_gen.wrapping_add(1);
    }

    /// Filter song indices by the search query (title/artist/subtitle substring, case-insensitive)
    /// and order them by the active `SortMode`. The single place search + sort are applied.
    pub(crate) fn arrange_songs(&self, indices: Vec<usize>) -> Vec<usize> {
        let q = self.search.trim().to_lowercase();
        let mut v: Vec<usize> = indices
            .into_iter()
            .filter(|&i| {
                if q.is_empty() {
                    return true;
                }
                let e = &self.songs[i];
                e.title.to_lowercase().contains(&q) || e.artist.to_lowercase().contains(&q) || e.subtitle.to_lowercase().contains(&q)
            })
            .collect();
        let title = |i: usize| self.songs[i].title.to_lowercase();
        match self.sort {
            SortMode::Default => {}
            SortMode::Title => v.sort_by(|&a, &b| title(a).cmp(&title(b))),
            SortMode::Artist => v.sort_by(|&a, &b| self.songs[a].artist.to_lowercase().cmp(&self.songs[b].artist.to_lowercase()).then(title(a).cmp(&title(b)))),
            SortMode::Level => v.sort_by(|&a, &b| {
                let lvl = |i: usize| self.songs[i].level.trim().parse::<i64>().unwrap_or(i64::MAX);
                lvl(a).cmp(&lvl(b)).then(title(a).cmp(&title(b)))
            }),
            SortMode::Clear => v.sort_by(|&a, &b| {
                let clr = |i: usize| self.scores.best_clear_for_md5(&self.songs[i].md5).unwrap_or(0);
                clr(b).cmp(&clr(a)).then(title(a).cmp(&title(b))) // best clear first
            }),
        }
        v
    }

    pub(crate) fn lane_for(&self, code: KeyCode) -> Option<usize> {
        self.active_keys.iter().find(|(k, _)| *k == code).map(|(_, l)| *l)
    }

    /// Which configured in-play control (if any) a key triggers.
    pub(crate) fn control_for(&self, code: KeyCode) -> Option<ControlAction> {
        ControlAction::ALL.into_iter().find(|a| self.keyconfig.control_key(*a) == Some(code))
    }

    /// Apply an in-play control: hi-speed, lane cover (sudden) and lift, clamped to sane ranges.
    pub(crate) fn apply_control(&mut self, action: ControlAction) {
        match action {
            ControlAction::HiSpeedUp => self.config.hispeed = (self.config.hispeed + 0.25).clamp(0.5, 10.0),
            ControlAction::HiSpeedDown => self.config.hispeed = (self.config.hispeed - 0.25).clamp(0.5, 10.0),
            ControlAction::CoverUp => self.config.cover = (self.config.cover + 0.05).clamp(0.0, 0.9),
            ControlAction::CoverDown => self.config.cover = (self.config.cover - 0.05).clamp(0.0, 0.9),
            ControlAction::LiftUp => {
                self.config.lift = (self.config.lift + 0.05).clamp(0.0, 0.9);
                self.rebuild_skin();
            }
            ControlAction::LiftDown => {
                self.config.lift = (self.config.lift - 0.05).clamp(0.0, 0.9);
                self.rebuild_skin();
            }
        }
    }

    pub(crate) fn current_settings(&self) -> PlaySettings {
        PlaySettings {
            hispeed: self.config.hispeed,
            gauge: gauge_token(self.config.gauge).to_string(),
            lift: self.config.lift,
            cover: self.config.cover,
            scratch_left: self.config.scratch_left,
            scratch_auto: self.config.scratch_auto,
            autoplay: self.autoplay,
            random: self.config.random.label().to_string(),
            constant_speed: self.config.constant_speed,
            offset_ms: self.config.offset_ms,
            auto_offset: self.config.auto_offset,
            judge_rate: self.config.judge_rate,
            total_override: self.config.total_override,
            bga: self.config.bga,
            skin: self.config.skin_name.clone(),
            auto_replay: self.config.auto_replay,
            debug: self.config.debug,
            font_path: self.config.font_path.clone(),
            score_graph: self.config.score_graph,
            replay_analysis: self.config.replay_analysis,
            preview: self.config.preview,
            songs_folder: self.config.songs_folder.clone(),
            server_url: self.config.server_url.clone(),
            player_id: self.config.player_id.clone(),
            ir_token: self.session.token().map(str::to_string),
            ir_login_id: self.session.login_id().map(str::to_string),
            ir_email: self.config.ir_email.clone(),
            sync_settings: self.config.sync_settings,
            auto_upload_replay: self.config.auto_upload_replay,
            rivals: self.config.rivals.clone(),
        }
    }

    pub(crate) fn save_settings(&self) {
        self.current_settings().save(&self.settings_path);
    }

    /// Open a native picker for a UI font (TTF/OTF/TTC), load it live as the preferred family,
    /// and persist the path so it is reapplied next launch.
    pub(crate) fn pick_font(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("font", &["ttf", "otf", "ttc"]).set_title("Select UI font").pick_file() {
            match std::fs::read(&path) {
                Ok(bytes) => match rbms_render::load_font(bytes) {
                    Some(family) => {
                        rbms_render::set_ui_family(&family);
                        self.config.font_path = Some(path.to_string_lossy().to_string());
                        self.save_settings();
                        println!("font: {} ({family})", path.display());
                    }
                    None => eprintln!("font load failed (no usable face): {}", path.display()),
                },
                Err(e) => eprintln!("font read failed ({}): {e}", path.display()),
            }
        }
    }

    /// Revert the UI font to the bundled default.
    pub(crate) fn reset_font(&mut self) {
        rbms_render::reset_ui_family();
        self.config.font_path = None;
        self.save_settings();
    }

    /// Edit the active NETWORK text field. Enter commits through `commit_network_edit`, Esc
    /// cancels. Everything but a secret row is trimmed; a password keeps the exact characters
    /// typed. The row the editor was **opened on** decides which field is written, not the row
    /// that happens to be focused at commit time.
    pub(crate) fn settings_text_input(&mut self, code: KeyCode, typed: Option<&str>) {
        match code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let raw = self.text_input.take().unwrap_or_default();
                let value = if self.text_secret { raw } else { raw.trim().to_string() };
                let edited = self.text_edit_row.take();
                self.text_secret = false;
                if let Some(row) = edited {
                    self.commit_network_edit(row, value);
                }
            }
            KeyCode::Escape => self.cancel_text_edit(),
            KeyCode::Backspace => {
                if let Some(b) = self.text_input.as_mut() {
                    b.pop();
                }
            }
            _ => {
                if let (Some(b), Some(t)) = (self.text_input.as_mut(), typed) {
                    b.extend(t.chars().filter(|c| !c.is_control()));
                }
            }
        }
    }

    pub(crate) fn offset_us(&self) -> i64 {
        self.config.offset_ms as i64 * 1000
    }

    /// Feed all recorded inputs whose raw time has been reached, judging them at the recorded
    /// time plus the (replay's) offset — reproducing the original run. `mute` skips keysounds (used
    /// during analysis scrubbing/slow-mo, where the virtual clock would desync audio). Each judged
    /// input's timing delta is captured for the analysis ms-off overlay.
    pub(crate) fn feed_replay(&mut self, song: i64, anchor: i64, mute: bool) {
        let off = self.offset_us();
        loop {
            let ev = match self.replay.as_ref() {
                Some(rp) if self.replay_cursor < rp.events.len() => rp.events[self.replay_cursor],
                _ => break,
            };
            if ev.t > song {
                break;
            }
            self.replay_cursor += 1;
            let res = if ev.press {
                let sound_t = keysound_time_us(ev.t, anchor);
                if let (false, Some(audio)) = (mute, self.audio.as_mut()) {
                    let play = |e: PlayEvent| audio.play(e.wav.max(0) as u32, 1.0, 0.0, 1.0, sound_t);
                    self.player.as_mut().and_then(|p| p.press(ev.lane, ev.t + off, play))
                } else {
                    self.player.as_mut().and_then(|p| p.press(ev.lane, ev.t + off, |_| {}))
                }
            } else {
                self.player.as_mut().and_then(|p| p.release(ev.lane, ev.t + off))
            };
            if let Some(r) = res {
                self.msoff.push((r.lane, r.delta_us, r.judge as u8));
                if self.msoff.len() > 16 {
                    self.msoff.remove(0);
                }
            }
        }
    }

    /// Rebuild the replay's judge state at an arbitrary song time by re-simulating the recorded
    /// inputs from the start up to `target_us`. Used by analysis seek so judging stays exactly
    /// correct (no forward-sweep corruption) when jumping forwards or backwards.
    pub(crate) fn seek_replay(&mut self, target_us: i64) {
        if self.replay.is_none() {
            return;
        }
        let target = target_us.max(0);
        let Some(model) = self.player.as_ref().map(|p| p.model().clone()) else {
            return;
        };
        let off = self.offset_us();
        let mut p = Player::new(model, false);
        p.set_gauge(self.config.gauge);
        p.set_judge_rate(self.config.judge_rate);
        if self.config.scratch_auto {
            let auto: Vec<bool> = (0..self.mode.key).map(|l| self.mode.is_scratch(l)).collect();
            p.set_auto_lanes(auto);
        }
        let mut cursor = 0;
        if let Some(rp) = &self.replay {
            for ev in &rp.events {
                if ev.t > target {
                    break;
                }
                if ev.press {
                    p.press(ev.lane, ev.t + off, |_| {});
                } else {
                    p.release(ev.lane, ev.t + off);
                }
                cursor += 1;
            }
        }
        p.update(target, |_| {});
        self.player = Some(p);
        self.replay_cursor = cursor;
        self.analysis_us = target;
        self.analysis_manual = true;
        self.msoff.clear();
    }

    /// Handle an analysis-mode playback key (only while a replay analysis is active): pause/resume,
    /// playback rate, and ±2 s seek. Returns whether the key was an analysis control.
    pub(crate) fn analysis_key(&mut self, code: KeyCode) -> bool {
        if !self.analysis {
            return false;
        }
        match code {
            KeyCode::Space => {
                self.analysis_manual = true;
                self.analysis_paused = !self.analysis_paused;
            }
            KeyCode::Equal => {
                self.analysis_manual = true;
                self.analysis_rate = (self.analysis_rate + 0.25).min(4.0);
            }
            KeyCode::Minus => {
                self.analysis_manual = true;
                self.analysis_rate = (self.analysis_rate - 0.25).max(0.25);
            }
            KeyCode::PageUp => self.seek_replay(self.analysis_us + 2_000_000),
            KeyCode::PageDown => self.seek_replay(self.analysis_us - 2_000_000),
            _ => return false,
        }
        true
    }

    /// Rebuild the resolved skin from the loaded base config plus the live scratch-side/lift.
    pub(crate) fn rebuild_skin(&mut self) {
        let mut cfg = self.skin_cfg.clone();
        cfg.scratch_left = self.config.scratch_left;
        cfg.lift = self.config.lift;
        self.result_palette = ResultPalette::from_skin(&cfg);
        self.skin = Skin::build(&cfg, self.mode, CW as f32, CH as f32);
    }

    pub(crate) fn enter_keyconfig(&mut self) {
        self.kc_sel = 0;
        self.kc_capturing = false;
        self.stage = Stage::KeyConfig;
    }

    pub(crate) fn cycle_edit_mode(&mut self, d: i32) {
        let all = Mode::ALL;
        let cur = all.iter().position(|m| m.key == self.kc_edit_mode.key).unwrap_or(0);
        self.kc_edit_mode = all[((cur as i32 + d).rem_euclid(all.len() as i32)) as usize];
        let len = kc_rows(self.kc_edit_mode).len();
        if self.kc_sel >= len {
            self.kc_sel = len - 1;
        }
    }

    /// Whether binding `code` to `row` (for `mode`) would collide with another action and so
    /// silently break it (controls are resolved before lanes in play, so a shared key would
    /// shadow the lane). Rebinding a row to its own current key is not a collision.
    pub(crate) fn binding_collides(&self, mode: Mode, row: &KcRow, code: KeyCode) -> bool {
        match row {
            KcRow::Lane(lane) => self.control_for(code).is_some() || self.keyconfig.lane_keys(mode).iter().any(|(k, l)| *k == code && l != lane),
            KcRow::Control(action) => {
                self.keyconfig.lane_keys(mode).iter().any(|(k, _)| *k == code)
                    || ControlAction::ALL.into_iter().any(|a| a != *action && self.keyconfig.control_key(a) == Some(code))
            }
            KcRow::ModeSelect => false,
        }
    }

    /// Key-config editor input. In capture mode the next key (except Esc) is bound to the
    /// focused row unless it collides with another action; otherwise navigate, switch
    /// edit-mode, start a rebind, or save+exit.
    pub(crate) fn keyconfig_input(&mut self, code: KeyCode) {
        let rows = kc_rows(self.kc_edit_mode);
        if self.kc_capturing {
            if code != KeyCode::Escape {
                if let Some(row) = rows.get(self.kc_sel) {
                    if self.binding_collides(self.kc_edit_mode, row, code) {
                        self.kc_warn = true;
                        self.kc_capturing = false;
                        return;
                    }
                    match row {
                        KcRow::Control(a) => self.keyconfig.set_control(*a, code),
                        KcRow::Lane(lane) => self.keyconfig.set_lane(self.kc_edit_mode, *lane, code),
                        KcRow::ModeSelect => {}
                    }
                }
            }
            self.kc_warn = false;
            self.kc_capturing = false;
            return;
        }
        self.kc_warn = false;
        match code {
            KeyCode::Escape => {
                self.keyconfig.save(&self.keyconfig_path);
                self.stage = Stage::Settings;
            }
            KeyCode::ArrowUp => self.kc_sel = self.kc_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.kc_sel = (self.kc_sel + 1).min(rows.len().saturating_sub(1)),
            KeyCode::ArrowLeft if matches!(rows.get(self.kc_sel), Some(KcRow::ModeSelect)) => self.cycle_edit_mode(-1),
            KeyCode::ArrowRight if matches!(rows.get(self.kc_sel), Some(KcRow::ModeSelect)) => self.cycle_edit_mode(1),
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if matches!(rows.get(self.kc_sel), Some(KcRow::Control(_)) | Some(KcRow::Lane(_))) {
                    self.kc_capturing = true;
                }
            }
            _ => {}
        }
    }
}

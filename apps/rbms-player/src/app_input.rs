//! `App` methods: lane/control input mapping, settings + skin/font round-trip, replay
//! playback & analysis, and the key-config editor. Split out of `main.rs`; `use crate::*`
//! pulls in the crate-root types/consts/helpers these methods reference.
#![allow(clippy::wildcard_imports)]

use crate::*;

impl AppShared {
    /// Recompute the visible select list for the current `select_view`.
    pub(crate) fn rebuild_select_items(&mut self) {
        let songs = self.library.songs();
        let n = songs.len();
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
        let songs = self.library.songs();
        let q = self.search.trim().to_lowercase();
        let mut v: Vec<usize> = indices
            .into_iter()
            .filter(|&i| {
                if q.is_empty() {
                    return true;
                }
                let e = &songs[i];
                e.title.to_lowercase().contains(&q) || e.artist.to_lowercase().contains(&q) || e.subtitle.to_lowercase().contains(&q)
            })
            .collect();
        let title = |i: usize| songs[i].title.to_lowercase();
        match self.sort {
            SortMode::Default => {}
            SortMode::Title => v.sort_by_key(|&a| title(a)),
            SortMode::Artist => v.sort_by(|&a, &b| songs[a].artist.to_lowercase().cmp(&songs[b].artist.to_lowercase()).then(title(a).cmp(&title(b)))),
            SortMode::Level => v.sort_by(|&a, &b| {
                let lvl = |i: usize| songs[i].level.trim().parse::<i64>().unwrap_or(i64::MAX);
                lvl(a).cmp(&lvl(b)).then(title(a).cmp(&title(b)))
            }),
            SortMode::Clear => v.sort_by(|&a, &b| {
                let clr = |i: usize| self.scores.best_clear_for_md5(&songs[i].md5).unwrap_or(0);
                clr(b).cmp(&clr(a)).then(title(a).cmp(&title(b)))
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
            ControlAction::HiSpeedUp => self.config.play.hispeed = (self.config.play.hispeed + HISPEED_STEP).clamp(HISPEED_MIN, HISPEED_MAX),
            ControlAction::HiSpeedDown => self.config.play.hispeed = (self.config.play.hispeed - HISPEED_STEP).clamp(HISPEED_MIN, HISPEED_MAX),
            ControlAction::CoverUp => self.config.play.cover = (self.config.play.cover + LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX),
            ControlAction::CoverDown => self.config.play.cover = (self.config.play.cover - LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX),
            ControlAction::LiftUp => {
                self.config.play.lift = (self.config.play.lift + LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
                self.rebuild_skin();
            }
            ControlAction::LiftDown => {
                self.config.play.lift = (self.config.play.lift - LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
                self.rebuild_skin();
            }
        }
    }

    /// Write the whole configuration out.
    ///
    /// The live [`AccountSession`] — not the configuration — owns the credential while the app
    /// runs, so it is folded back in here on the way to disk. Every other value the settings
    /// screen edits is already in `config`.
    pub(crate) fn save_settings(&mut self) {
        self.config.network.ir_token = self.session.token().map(str::to_string);
        self.config.network.ir_login_id = self.session.login_id().map(str::to_string);
        save_config(&self.config, &self.settings_path);
    }

    /// Open a native picker for a UI font (TTF/OTF/TTC), load it live as the preferred family,
    /// and persist the path so it is reapplied next launch.
    pub(crate) fn pick_font(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("font", &["ttf", "otf", "ttc"]).set_title("Select UI font").pick_file() {
            match std::fs::read(&path) {
                Ok(bytes) => match rbms_render::load_font(bytes) {
                    Some(family) => {
                        rbms_render::set_ui_family(&family);
                        self.config.display.font_path = Some(path.to_string_lossy().to_string());
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
        self.config.display.font_path = None;
        self.save_settings();
    }

    pub(crate) fn offset_us(&self) -> i64 {
        self.config.judge.offset_ms as i64 * 1000
    }

    /// Rebuild the resolved skin from the loaded base config plus the live scratch-side/lift.
    pub(crate) fn rebuild_skin(&mut self) {
        let mut cfg = self.skin_cfg.clone();
        cfg.scratch_left = self.config.play.scratch_left;
        cfg.lift = self.config.play.lift;
        self.result_palette = ResultPalette::from_skin(&cfg);
        self.skin = Skin::build(&cfg, self.mode, CW as f32, CH as f32);
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

    /// md5 of the currently focused chart, if a song (not a folder) is focused.
    pub(crate) fn focused_md5(&self) -> Option<String> {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => self.library.songs().get(*si).map(|e| e.md5.clone()),
            _ => None,
        }
    }

    /// Index into `songs` of the focused select row, if it is a chart (not a folder).
    pub(crate) fn focused_song_index(&self) -> Option<usize> {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => Some(*si),
            _ => None,
        }
    }

    /// Map a local score record into the renderer's record-row view (owned data, no borrow escapes).
    /// `trend` carries the EX delta versus the next-older play when the score graph is enabled.
    pub(crate) fn record_row_view(&self, r: &ScoreRecord, older: Option<&ScoreRecord>) -> RecordRowView {
        let (label, color) = clear_label_color(clear_type_from_id(r.clear));
        let trend = older.filter(|_| self.config.display.score_graph).map(|o| ex_delta_label(r.ex_score as i64 - o.ex_score as i64));
        let when = format!("{}{}", fmt_datetime(r.played_at), rule_version_mark(r.rule_version));
        RecordRowView { when, lamp: color, lamp_label: label, ex: r.ex_score, max_ex: r.max_ex, bp: r.counts[3] + r.counts[4] + r.counts[5], trend }
    }
}

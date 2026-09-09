//! `App` methods for the SELECT screen: the song library, focused-chart detail, hover
//! preview, record modal, difficulty tables, folder scanning, and loading transitions.
//! Split out of `main.rs` (see `app_input` for the import note).
#![allow(clippy::wildcard_imports)]
use crate::*;

impl App {
    /// Enter the focused select item: descend into a folder, or start a chart.
    pub(crate) fn select_enter(&mut self) {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(i)) => {
                let i = *i;
                self.begin_loading(Loading::Song(i));
            }
            Some(SelectItem::Folder { target, .. }) => {
                self.select_view = *target;
                self.sel = 0;
                self.rebuild_select_items();
            }
            None => {}
        }
    }

    /// Open the search box (`/`). Search spans the whole library, so switch to the flat song list.
    pub(crate) fn start_search(&mut self) {
        self.searching = true;
        self.search.clear();
        self.select_view = SelectView::AllSongs;
        self.sel = 0;
        self.rebuild_select_items();
    }

    /// Close the search box (Esc) and clear the filter.
    pub(crate) fn exit_search(&mut self) {
        self.searching = false;
        self.search.clear();
        self.sel = 0;
        self.rebuild_select_items();
    }

    /// Re-filter after the query changed (every keystroke), keeping the focus at the top.
    pub(crate) fn apply_search(&mut self) {
        self.sel = 0;
        self.rebuild_select_items();
    }

    /// Cycle the song-list sort order (F3).
    pub(crate) fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.sel = 0;
        self.rebuild_select_items();
    }

    /// md5 of the currently focused chart, if a song (not a folder) is focused.
    pub(crate) fn focused_md5(&self) -> Option<String> {
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => self.songs.get(*si).map(|e| e.md5.clone()),
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
        let trend = older.filter(|_| self.config.score_graph).map(|o| ex_delta_label(r.ex_score as i64 - o.ex_score as i64));
        let when = format!("{}{}", fmt_datetime(r.played_at), rule_version_mark(r.rule_version));
        RecordRowView { when, lamp: color, lamp_label: label, ex: r.ex_score, max_ex: r.max_ex, bp: r.counts[3] + r.counts[4] + r.counts[5], trend }
    }

    /// Whether a second Esc/Left right now would quit: the arming press must still be inside
    /// [`crate::ROOT_ESC_CONFIRM`]. Read by both the guide line and the scene cache key, so the
    /// "press again" prompt disappears exactly when the window closes.
    pub(crate) fn esc_quit_armed(&self) -> bool {
        esc_confirms_quit(self.esc_quit_at, Instant::now())
    }

    pub(crate) fn select_key(&self) -> SelectKey {
        (self.select_gen, self.sel, self.record_modal, self.scores.records.len(), self.config.score_graph, self.esc_quit_armed())
    }

    /// Rebuild the cached select scene only when [`select_key`] changes. `frame()` runs a continuous
    /// redraw loop, so rebuilding every frame would re-walk + re-allocate the whole song list. Kept as a
    /// `&mut self` step (separate from borrowing the field) so the cached scene can be read disjointly
    /// from the GPU during render.
    pub(crate) fn refresh_select_cache(&mut self) {
        let key = self.select_key();
        if self.cached_select_key != Some(key) || self.cached_select.is_none() {
            let scene = self.build_select_view();
            self.cached_select = Some(scene);
            self.cached_select_key = Some(key);
        }
    }

    /// Assemble the backend-agnostic [`SelectScene`] from the current select state. Built before the
    /// GPU borrow so the render pass can stay a pure data → pixels call (the cover texture is uploaded
    /// separately into [`cover_rect`]).
    pub(crate) fn build_select_view(&self) -> SelectScene {
        let rows: Vec<SelectRow> = self
            .select_items
            .iter()
            .map(|item| match item {
                SelectItem::Folder { label, .. } => SelectRow {
                    folder: true,
                    title: label.clone(),
                    mode_short: "",
                    mode_color: Color::GRAY,
                    level: String::new(),
                    difficulty_color: Color::GRAY,
                    lamp: Color::rgb(44, 44, 54),
                    folder_count: None,
                },
                SelectItem::Song(si) => {
                    let e = &self.songs[*si];
                    let lamp = self.scores.best_clear_for_md5(&e.md5).map(|c| clear_label_color(clear_type_from_id(c)).1).unwrap_or(Color::rgb(44, 44, 54));
                    SelectRow {
                        folder: false,
                        title: e.title.clone(),
                        mode_short: mode_short(e.mode),
                        mode_color: mode_color(e.mode),
                        level: e.level.clone(),
                        difficulty_color: difficulty_color(e.difficulty),
                        lamp,
                        folder_count: None,
                    }
                }
            })
            .collect();

        let header = match self.select_view {
            SelectView::Root => self.config.songs_folder.as_deref().and_then(|p| Path::new(p).file_name()).and_then(|n| n.to_str()).unwrap_or("ROOT").to_string(),
            SelectView::AllSongs => "ALL SONGS".into(),
            SelectView::TableLevels(ti) => self.table_names.get(ti).cloned().unwrap_or_default(),
            SelectView::TableLevel(ti, li) => {
                let table = self.table_names.get(ti).cloned().unwrap_or_default();
                let level = self.table_levels.get(ti).and_then(|ls| ls.get(li)).map(|(l, _)| format!("LV {l}")).unwrap_or_default();
                format!("{table} / {level}")
            }
        };

        let detail = match self.select_items.get(self.sel) {
            Some(SelectItem::Song(si)) => {
                let e = &self.songs[*si];
                let d = self.focused_detail.as_ref();
                let bpm = match d {
                    Some(d) if (d.bpm_max - d.bpm_min).abs() >= 1.0 => format!("{}\u{2013}{}", d.bpm_min.round() as i32, d.bpm_max.round() as i32),
                    Some(d) => format!("{}", d.bpm_min.round() as i32),
                    None => format!("{}", e.init_bpm.round() as i32),
                };
                let notes = d.map(|d| if d.long_notes > 0 { format!("{} ({}LN)", d.notes, d.long_notes) } else { d.notes.to_string() }).unwrap_or_else(|| "\u{2026}".into());
                let length = d.map(|d| fmt_duration(d.duration_us)).unwrap_or_else(|| "\u{2026}".into());
                let total = if e.total > 0.0 { format!("{}", e.total.round() as i32) } else { "AUTO".into() };
                let genre_maker = match (e.genre.trim(), e.maker.trim()) {
                    ("", "") => String::new(),
                    ("", m) => m.to_string(),
                    (g, "") => g.to_string(),
                    (g, m) => format!("{g}  \u{00B7}  {m}"),
                };
                let density = d.filter(|d| !d.density.is_empty()).map(|d| DensityView { bins: d.density.clone(), peak: d.peak_density, avg: d.avg_density, end: d.end_density });

                let recs = self.scores.for_md5(&e.md5);
                let plays = recs.len();
                let clears = recs.iter().filter(|r| r.clear >= 2).count();
                let best = recs.iter().max_by(|a, b| a.clear.cmp(&b.clear).then(a.ex_score.cmp(&b.ex_score)));
                let rank_bar = best.filter(|_| self.config.score_graph).map(|b| (b.ex_score, b.max_ex));
                let best = best.map(|b| self.record_row_view(b, None));
                let recent = recs.iter().enumerate().map(|(ri, r)| self.record_row_view(r, recs.get(ri + 1).copied())).collect();

                SelectDetail::Song(Box::new(DetailView {
                    accent: mode_color(e.mode),
                    title: e.title.clone(),
                    subtitle: e.subtitle.clone(),
                    artist: e.artist.clone(),
                    genre_maker,
                    mode_short: mode_short(e.mode),
                    mode_color: mode_color(e.mode),
                    level: e.level.clone(),
                    difficulty_color: difficulty_color(e.difficulty),
                    difficulty_name: difficulty_name(e.difficulty),
                    cover: if self.cover_rgba.is_some() { CoverState::Present } else { CoverState::None },
                    stats: vec![
                        StatCell { label: "BPM", value: bpm },
                        StatCell { label: "DENSITY", value: d.map(|d| format!("{}/s", d.avg_density.round() as i32)).unwrap_or_else(|| "\u{2026}".into()) },
                        StatCell { label: "NOTES", value: notes },
                        StatCell { label: "JUDGE", value: rank_label(e.rank) },
                        StatCell { label: "LENGTH", value: length },
                        StatCell { label: "TOTAL", value: total },
                    ],
                    density,
                    records: RecordsView { plays, clears, best, rank_bar, recent },
                }))
            }
            Some(SelectItem::Folder { label, .. }) => SelectDetail::Folder { label: label.clone(), count: 0 },
            None => SelectDetail::Empty,
        };

        let modal = self.record_modal.and_then(|ri| {
            let si = match self.select_items.get(self.sel) {
                Some(SelectItem::Song(si)) => *si,
                _ => return None,
            };
            let e = &self.songs[si];
            let recs = self.scores.for_md5(&e.md5);
            let r = recs.get(ri)?;
            let (clear_label, clear_color) = clear_label_color(clear_type_from_id(r.clear));
            let (rank, rank_color) = if self.config.score_graph {
                let (name, col) = RANK_BANDS[dj_rank(r.ex_score, r.max_ex)];
                (Some(name), col)
            } else {
                (None, Color::WHITE)
            };
            Some(SelectModal {
                title: e.title.clone(),
                clear_label,
                clear_color,
                when: fmt_datetime(r.played_at),
                sub: format!("{}   {}   GAUGE {}{}", r.mode, r.random, r.gauge, rule_version_note(r.rule_version)),
                counts: r.counts,
                ex: r.ex_score,
                max_ex: r.max_ex,
                rank,
                rank_color,
                max_combo: r.max_combo,
                total_notes: r.total_notes,
                bp: r.counts[3] + r.counts[4] + r.counts[5],
                empty_poor: r.empty_poor,
                gauge_value: r.gauge_value.round() as i32,
                index: ri,
                total: recs.len(),
                has_replay: r.replay_file.is_some(),
            })
        });

        let guide = if self.esc_quit_armed() {
            "PRESS ESC AGAIN TO QUIT"
        } else if self.searching {
            "TYPE TO FILTER   BACKSPACE EDIT   ENTER OPEN   ESC CLEAR"
        } else {
            "\u{2191}\u{2193} MOVE   ENTER OPEN   ESC BACK"
        };
        // Onboarding/empty hint shown when the list has no rows: a first-run "add a folder" CTA, an
        // empty-search note, or a generic "no charts" message.
        let empty_hint = if !rows.is_empty() {
            None
        } else if self.searching {
            Some(("NO RESULTS", "Try a different search  \u{00B7}  Esc to clear"))
        } else if self.folders.is_empty() {
            Some(("WELCOME TO rbms", "Add your music folder \u{2014} press O or click FOLDERS below"))
        } else {
            Some(("NO CHARTS", "Press O to manage folders  \u{00B7}  T for difficulty tables"))
        };
        SelectScene {
            rows,
            sel: self.sel,
            header,
            guide,
            detail,
            modal,
            score_graph: self.config.score_graph,
            search: self.searching.then(|| self.search.clone()),
            sort: self.sort.label(),
            empty_hint,
        }
    }

    /// (Re)compute the focused chart's heavy detail (notes/LN/length/BPM range) only when the focus
    /// moves to a different song — so scrolling the list parses at most one chart per moved row.
    pub(crate) fn refresh_focused_detail(&mut self) {
        let si = self.focused_song_index();
        if si != self.focus_settle_si {
            self.focus_settle_si = si;
            self.focus_settle_at = Instant::now();
        }
        if si == self.focused_detail_si || self.focus_settle_at.elapsed() < FOCUS_DETAIL_DEBOUNCE {
            return;
        }
        self.focused_detail_si = si;
        self.focused_detail = si.and_then(|i| self.songs.get(i)).and_then(|e| compute_chart_detail(&e.path, e.mode));
        // Decode the cover (#STAGEFILE, then #BANNER) for the detail panel; the texture is uploaded
        // into the single BGA slot during the Select render. One decode per moved row, like the detail.
        self.cover_rgba = si.and_then(|i| self.songs.get(i)).and_then(|e| {
            let dir = e.path.parent()?;
            [&e.stagefile, &e.banner].into_iter().filter(|n| !n.trim().is_empty()).find_map(|n| decode_bga_256(dir, n))
        });
    }

    /// Drive the song-select hover preview once the focus settles (debounced so fast scrolling
    /// doesn't load every row): if the chart defines a `#PREVIEW` clip, decode + loop that file;
    /// otherwise build an autoplay preview of the chart itself (background-decode its keysounds, then
    /// replay the autoplay timeline, looped). The preview audio engine is separate from the play
    /// engine (which only exists during Play) and recreated per settled chart so its bank is clean.
    pub(crate) fn update_preview(&mut self) {
        if !self.config.preview {
            if self.preview_audio.is_some() {
                self.stop_preview();
            }
            return;
        }
        let cur = self.focused_song_index();
        if cur != self.preview_target {
            self.preview_target = cur;
            self.preview_target_at = Instant::now();
            if self.preview_si.is_some() {
                self.reset_preview_playback();
            }
        }
        if self.preview_si != cur {
            if let Some(si) = cur {
                if self.preview_target_at.elapsed() >= PREVIEW_DEBOUNCE {
                    self.start_preview(si);
                }
            }
            return;
        }
        // Autoplay preview load: drain the background channel (the schedule, then decoded keysounds);
        // the channel disconnecting means the load is done, so anchor the clock to begin playback at
        // the first event.
        if self.preview_prep_rx.is_some() {
            let mut done = false;
            loop {
                match self.preview_prep_rx.as_ref().unwrap().try_recv() {
                    Ok(PreviewMsg::Schedule { sched, start_us, end_us }) => {
                        self.preview_sched = sched;
                        self.preview_start_us = start_us;
                        self.preview_end_us = end_us;
                        self.preview_cursor = 0;
                    }
                    Ok(PreviewMsg::Keysound(id, dec)) => {
                        if let Some(eng) = self.preview_audio.as_mut() {
                            eng.insert_decoded(id, dec);
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
            if done {
                self.preview_prep_rx = None;
                if self.preview_sched.is_empty() {
                    // Nothing to play (no autoplay events) — drop the idle engine instead of leaving
                    // its cpal stream open until the next focus. The song stays settled (no retry).
                    self.preview_audio = None;
                } else {
                    if let Some(eng) = self.preview_audio.as_ref() {
                        self.preview_anchor = eng.clock_us() - self.preview_start_us;
                    }
                    self.preview_cursor = 0;
                }
            }
            return;
        }
        // Autoplay preview playback: fire every scheduled keysound whose time has passed FIRST (so the
        // final event is never skipped), then loop once the silent tail elapses by re-anchoring the clock.
        if !self.preview_sched.is_empty() {
            if let Some(eng) = self.preview_audio.as_mut() {
                let song = eng.clock_us() - self.preview_anchor;
                while self.preview_cursor < self.preview_sched.len() && self.preview_sched[self.preview_cursor].0 <= song {
                    let (at, wav) = self.preview_sched[self.preview_cursor];
                    eng.play(wav, PREVIEW_GAIN, 0.0, 1.0, at + self.preview_anchor);
                    self.preview_cursor += 1;
                }
                if song >= self.preview_end_us {
                    self.preview_anchor += self.preview_end_us - self.preview_start_us;
                    self.preview_cursor = 0;
                }
            }
            return;
        }
        // File preview (#PREVIEW): loop by re-triggering at each clip boundary. (The mixer stops the
        // same key on a new play, so scheduling ahead would cut the current clip; re-triggering after
        // it naturally ends is clean.)
        if self.preview_loop_us > 0 {
            if let Some(eng) = self.preview_audio.as_mut() {
                while eng.clock_us() >= self.preview_next_us {
                    eng.play(PREVIEW_ID, PREVIEW_GAIN, 0.0, 1.0, self.preview_next_us);
                    self.preview_next_us += self.preview_loop_us;
                }
            }
        }
    }

    /// Begin the preview for the focused chart: its `#PREVIEW` clip if defined, otherwise an autoplay
    /// preview of the chart. Marks the song as settled even when nothing can be played, so the
    /// debounce doesn't retry every frame.
    pub(crate) fn start_preview(&mut self, si: usize) {
        let dbg = self.config.debug;
        self.preview_si = Some(si);
        self.preview_loop_us = 0;
        self.preview_sched.clear();
        self.preview_cursor = 0;
        let Some(has_file) = self.songs.get(si).map(|e| !e.preview.trim().is_empty()) else {
            if dbg {
                eprintln!("[preview] song index {si} out of range");
            }
            return;
        };
        if !has_file {
            self.start_autoplay_preview(si, dbg);
            return;
        }
        let e = &self.songs[si];
        let title = e.title.clone();
        let Some(dir) = e.path.parent() else {
            if dbg {
                eprintln!("[preview] no parent dir for {}", e.path.display());
            }
            return;
        };
        let Some((path, _)) = resolve_file(dir, &e.preview, &["wav", "ogg", "flac", "mp3"]) else {
            if dbg {
                eprintln!("[preview] #PREVIEW '{}' not found under {}", e.preview, dir.display());
            }
            return;
        };
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(err) => {
                if dbg {
                    eprintln!("[preview] read failed {}: {err}", path.display());
                }
                return;
            }
        };
        let ext = path.extension().and_then(|x| x.to_str()).map(str::to_owned);
        let engine = match AudioEngine::new() {
            Ok(eng) => eng,
            Err(err) => {
                if dbg {
                    eprintln!("[preview] AudioEngine::new failed (no preview output stream): {err}");
                }
                return;
            }
        };
        self.preview_audio = Some(engine);
        let Some(eng) = self.preview_audio.as_mut() else { return };
        if let Err(err) = eng.load(PREVIEW_ID, bytes, ext.as_deref()) {
            if dbg {
                eprintln!("[preview] decode failed {}: {err}", path.display());
            }
            return;
        }
        let dur = eng.sample_duration_us(PREVIEW_ID).unwrap_or(0);
        let now = eng.clock_us();
        eng.play(PREVIEW_ID, PREVIEW_GAIN, 0.0, 1.0, now);
        self.preview_loop_us = dur;
        self.preview_next_us = now + dur.max(1);
        if dbg {
            eprintln!("[preview] playing '{title}' ({}) dur_us={dur} clock_us={now}", path.display());
        }
    }

    /// Start an autoplay preview for a chart with no `#PREVIEW` file. All heavy work — parse, autoplay
    /// schedule extraction, and keysound decode — runs on a background coordinator thread so the
    /// select screen never stutters on focus-settle; [`update_preview`] drains the results and begins
    /// playback once the load finishes. A fresh cancel flag lets a later focus abandon this load.
    pub(crate) fn start_autoplay_preview(&mut self, si: usize, dbg: bool) {
        let Some(e) = self.songs.get(si) else { return };
        let path = e.path.clone();
        let title = e.title.clone();
        let engine = match AudioEngine::new() {
            Ok(eng) => eng,
            Err(err) => {
                if dbg {
                    eprintln!("[preview] AudioEngine::new failed (no preview output stream): {err}");
                }
                return;
            }
        };
        self.preview_audio = Some(engine);
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.preview_cancel = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel::<PreviewMsg>();
        self.preview_prep_rx = Some(rx);
        if dbg {
            eprintln!("[preview] '{title}' autoplay: loading in background");
        }
        std::thread::spawn(move || {
            use std::sync::atomic::Ordering;
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let Ok(bytes) = std::fs::read(&path) else { return };
            let Some(dir) = path.parent().map(Path::to_path_buf) else { return };
            let src = rbms_parser::parse_with(&bytes, Default::default());
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let chart_name = path.to_string_lossy();
            let mode = rbms_chart::detect_mode(&src, &chart_name);
            let model = to_model(&src, mode);
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let jobs = keysound_jobs(&model.wavmap, &dir);
            // The autoplay keysound timeline, from a one-shot autoplay pass (judging discarded).
            let mut sched: Vec<(i64, u32)> = Vec::new();
            {
                let mut p = Player::new(model, true);
                let end = p.last_time_us() + 1_000_000;
                p.update(end, |ev| {
                    if ev.wav >= 0 {
                        sched.push((ev.at_us, ev.wav as u32));
                    }
                });
            }
            sched.sort_by_key(|s| s.0);
            if sched.is_empty() || cancel.load(Ordering::Relaxed) {
                return;
            }
            let start_us = sched.first().map(|s| s.0).unwrap_or(0);
            let end_us = sched.last().map(|s| s.0).unwrap_or(0) + PREVIEW_LOOP_TAIL_US;
            if tx.send(PreviewMsg::Schedule { sched, start_us, end_us }).is_err() {
                return;
            }
            // Reuse the shared decode pool, forwarding each decoded keysound to the preview channel.
            let (kx, _progress, _total) = spawn_keysound_decode(jobs, cancel);
            for (id, dec) in kx {
                if tx.send(PreviewMsg::Keysound(id, dec)).is_err() {
                    break;
                }
            }
        });
    }

    /// Stop and clear the active preview playback: signal any in-flight autoplay-preview load to stop
    /// (parse + decode workers check this cancel flag), drop the engine to release its cpal stream and
    /// ringing voices, and clear all playback state. Leaves `preview_target` so the debounce state
    /// stays owned by the caller.
    pub(crate) fn reset_preview_playback(&mut self) {
        self.preview_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        self.preview_audio = None;
        self.preview_si = None;
        self.preview_loop_us = 0;
        self.preview_next_us = 0;
        self.preview_sched.clear();
        self.preview_cursor = 0;
        self.preview_anchor = 0;
        self.preview_start_us = 0;
        self.preview_end_us = 0;
        self.preview_prep_rx = None;
    }

    /// Tear down the preview entirely (used when leaving Select / entering Play); recreated next time
    /// a preview is needed in select.
    pub(crate) fn stop_preview(&mut self) {
        self.reset_preview_playback();
        self.preview_target = None;
    }

    /// Open the record-detail modal for the focused chart (newest record first), if it has any.
    pub(crate) fn open_record_modal(&mut self) {
        if let Some(md5) = self.focused_md5() {
            if !self.scores.for_md5(&md5).is_empty() {
                self.record_modal = Some(0);
            }
        }
    }

    /// Move the record-detail modal selection within the focused chart's records (clamped).
    pub(crate) fn record_modal_nav(&mut self, d: i32) {
        let Some(ri) = self.record_modal else { return };
        let n = self.focused_md5().map(|m| self.scores.for_md5(&m).len()).unwrap_or(0);
        if n == 0 {
            self.record_modal = None;
            return;
        }
        self.record_modal = Some((ri as i32 + d).clamp(0, n as i32 - 1) as usize);
    }

    /// Load and start the replay attached to the record currently shown in the modal.
    pub(crate) fn play_record_replay(&mut self) {
        let Some(ri) = self.record_modal else { return };
        let Some(md5) = self.focused_md5() else { return };
        let file = self.scores.for_md5(&md5).get(ri).and_then(|r| r.replay_file.clone());
        let Some(file) = file else { return };
        let dir = self.settings_path.parent().map(|d| d.join("replays")).unwrap_or_else(|| PathBuf::from("replays"));
        match Replay::load(&dir.join(&file)) {
            Ok(rp) => {
                self.record_modal = None;
                self.chart_path = rp.chart_path.clone();
                self.replay = Some(rp);
                self.replay_cursor = 0;
                self.result = None;
                self.loading_drawn = false;
                self.pending = None;
                if self.load() { self.after_load(); } else { self.stage = Stage::Select; }
            }
            Err(e) => {
                eprintln!("replay load failed: {e}");
                self.record_modal = None;
            }
        }
    }

    /// Hit-test the last frame's clickable regions against the cursor and dispatch a left-click.
    /// Regions are tested topmost-first (later pushes draw on top).
    pub(crate) fn handle_click(&mut self) {
        let (cx, cy) = self.cursor;
        let hit = self
            .hot
            .iter()
            .rev()
            .find(|(r, _)| cx >= r.x && cx <= r.x + r.w && cy >= r.y && cy <= r.y + r.h)
            .map(|(_, h)| *h);
        match hit {
            Some(Hot::SelectRow(idx)) => {
                if self.sel == idx {
                    self.select_enter();
                } else {
                    self.sel = idx;
                    self.record_modal = None;
                    self.print_selection();
                }
            }
            Some(Hot::RecordRow(ri)) => self.record_modal = Some(ri),
            Some(Hot::SettingTab(ti)) => {
                self.set_tab = ti.min(SETTING_TABS.len() - 1);
                self.set_sel = 0;
            }
            Some(Hot::SettingRow(i)) => {
                let items = SETTING_TABS[self.set_tab].1;
                if let Some(&g) = items.get(i) {
                    self.set_sel = i;
                    if g == SETTING_KEYCONFIG {
                        self.enter_keyconfig();
                    } else {
                        self.adjust_setting(g, 1);
                    }
                }
            }
            Some(Hot::ModalReplay) => self.play_record_replay(),
            Some(Hot::ModalClose) => self.record_modal = None,
            // Bottom navigation buttons — clickable equivalents of the keyboard shortcuts.
            Some(Hot::NavSearch) => {
                if self.searching {
                    self.exit_search();
                } else {
                    self.start_search();
                }
            }
            Some(Hot::NavSort) => self.cycle_sort(),
            Some(Hot::NavFolders) => self.open_folders(),
            Some(Hot::NavTables) => self.open_tables(),
            Some(Hot::NavRecords) => self.open_record_modal(),
            Some(Hot::NavSettings) => self.stage = Stage::Settings,
            // A click that misses every region closes an open modal (click-outside-to-dismiss).
            None => self.record_modal = None,
        }
    }

    /// Apply a finished background scan: swap in the merged library/tables and return to the
    /// (rebuilt) select screen.
    pub(crate) fn apply_scan(&mut self, out: ScanOutcome) {
        self.songs = out.songs;
        self.table_names = out.names;
        self.table_levels = out.levels;
        self.select_view = SelectView::Root;
        self.sel = 0;
        self.rebuild_select_items();
        self.scan_rx = None;
        self.pending = None;
        self.stage = Stage::Select;
        println!("scanned {} charts", self.songs.len());
        self.print_selection();
    }

    pub(crate) fn open_tables(&mut self) {
        self.tables_sel = 0;
        self.text_input = None;
        self.stage = Stage::Tables;
    }

    pub(crate) fn open_folders(&mut self) {
        self.folders_sel = 0;
        self.stage = Stage::Folders;
    }

    pub(crate) fn folders_row_count(&self) -> usize {
        self.folders.len() + 1 // the folders + a trailing "+ ADD FOLDER" row
    }

    /// Pick a folder and add it to the library list (deduped, persisted). The merged rescan happens
    /// when the user leaves the Folders screen, so several folders can be added in one visit.
    pub(crate) fn add_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title("Add song folder").pick_folder() {
            let path = dir.to_string_lossy().to_string();
            if !self.folders.iter().any(|f| f == &path) {
                self.folders.push(path);
                FolderList { folders: self.folders.clone() }.save(&self.folders_path);
            }
        }
    }

    pub(crate) fn remove_folder(&mut self, idx: usize) {
        if idx < self.folders.len() {
            self.folders.remove(idx);
            FolderList { folders: self.folders.clone() }.save(&self.folders_path);
            self.folders_sel = self.folders_sel.min(self.folders_row_count().saturating_sub(1));
        }
    }

    /// Rescan every library folder off-thread and merge into one song list (then re-match tables),
    /// landing back on Select. Used after the folder list changes.
    pub(crate) fn rescan_all_folders(&mut self) {
        let dirs = self.folders.clone();
        let sources = self.table_sources.clone();
        self.scan_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = self.scan_count.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let songs = scan_folders(&dirs, &count);
            let (names, levels) = fetch_and_match(&sources, &songs);
            let _ = tx.send(ScanOutcome { songs, names, levels });
        });
        self.scan_rx = Some(rx);
        self.pending = Some(Loading::Scan);
        self.loading_drawn = false;
        self.stage = Stage::Loading;
    }

    pub(crate) fn folders_input(&mut self, code: KeyCode) {
        let n = self.folders_row_count();
        let add_row = self.folders.len();
        match code {
            // Re-merge the (possibly changed) library on the way out.
            KeyCode::Escape => self.rescan_all_folders(),
            KeyCode::ArrowUp => self.folders_sel = self.folders_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.folders_sel = (self.folders_sel + 1).min(n.saturating_sub(1)),
            KeyCode::KeyD | KeyCode::Delete => {
                if self.folders_sel < self.folders.len() {
                    self.remove_folder(self.folders_sel);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if self.folders_sel == add_row {
                    self.add_folder_dialog();
                }
            }
            _ => {}
        }
    }

    /// Rows in the table-manager: each source, then the two add actions.
    pub(crate) fn tables_row_count(&self) -> usize {
        self.table_sources.len() + 2
    }

    pub(crate) fn add_table_source(&mut self, src: TableSource) {
        let (name, levels) = load_and_match(&src, &self.songs);
        self.table_sources.push(src);
        self.table_names.push(name);
        self.table_levels.push(levels);
        TableList { tables: self.table_sources.clone() }.save(&self.tables_path);
    }

    pub(crate) fn add_table_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title("Select table json").add_filter("json", &["json"]).pick_file() {
            let location = path.to_string_lossy().to_string();
            if self.table_sources.iter().any(|t| t.location == location) {
                eprintln!("table already added: {location}");
                return;
            }
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("table").to_string();
            self.add_table_source(TableSource { name, location });
        }
    }

    pub(crate) fn remove_table_source(&mut self, idx: usize) {
        if idx < self.table_sources.len() {
            self.table_sources.remove(idx);
            if idx < self.table_names.len() {
                self.table_names.remove(idx);
            }
            if idx < self.table_levels.len() {
                self.table_levels.remove(idx);
            }
            TableList { tables: self.table_sources.clone() }.save(&self.tables_path);
            self.tables_sel = self.tables_sel.min((self.table_sources.len() + 2).saturating_sub(1));
        }
    }

    /// Table-manager input. In URL-text mode, type the URL (Enter adds, Esc cancels); otherwise
    /// navigate, add (URL/file), remove (D), or leave (Esc — rebuilds the browse list).
    pub(crate) fn tables_input(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, typed: Option<&str>) {
        if self.text_input.is_some() {
            match code {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let url = self.text_input.take().unwrap_or_default().trim().to_string();
                    if !url.is_empty() {
                        if self.table_sources.iter().any(|t| t.location == url) {
                            eprintln!("table already added: {url}");
                        } else {
                            self.add_table_source(TableSource { name: String::new(), location: url });
                        }
                    }
                }
                KeyCode::Escape => self.text_input = None,
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
            return;
        }
        let n = self.tables_row_count();
        let add_url = self.table_sources.len();
        let add_file = self.table_sources.len() + 1;
        match code {
            KeyCode::Escape => {
                self.select_view = SelectView::Root;
                self.sel = 0;
                self.rebuild_select_items();
                self.stage = Stage::Select;
                let _ = event_loop;
            }
            KeyCode::ArrowUp => self.tables_sel = self.tables_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.tables_sel = (self.tables_sel + 1).min(n.saturating_sub(1)),
            KeyCode::KeyD | KeyCode::Delete => {
                if self.tables_sel < self.table_sources.len() {
                    self.remove_table_source(self.tables_sel);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if self.tables_sel == add_url {
                    self.text_input = Some(String::new());
                } else if self.tables_sel == add_file {
                    self.add_table_file();
                }
            }
            _ => {}
        }
    }

    /// Esc in the select screen: one level up, or — at the root — arm the quit confirmation and only
    /// exit on a second Esc within [`ROOT_ESC_CONFIRM`]. An empty library behaves the same, so a
    /// first-run window can't be closed by a stray keypress.
    pub(crate) fn select_escape(&mut self, event_loop: &ActiveEventLoop) {
        if self.select_view != SelectView::Root {
            self.esc_quit_at = None;
            self.select_back(event_loop);
            return;
        }
        if self.esc_quit_armed() {
            event_loop.exit();
        } else {
            self.esc_quit_at = Some(Instant::now());
        }
    }

    /// Go up one select level; at the root, quit.
    pub(crate) fn select_back(&mut self, event_loop: &ActiveEventLoop) {
        match self.select_view {
            SelectView::Root => event_loop.exit(),
            SelectView::AllSongs | SelectView::TableLevels(_) => {
                self.select_view = SelectView::Root;
                self.sel = 0;
                self.rebuild_select_items();
            }
            SelectView::TableLevel(ti, _) => {
                self.select_view = SelectView::TableLevels(ti);
                self.sel = 0;
                self.rebuild_select_items();
            }
        }
    }

    /// Queue a loading task and switch to the loading screen; the actual (blocking) work happens
    /// one frame later in `finish_loading`, so a LOADING frame is presented first.
    pub(crate) fn begin_loading(&mut self, task: Loading) {
        self.pending = Some(task);
        self.loading_drawn = false;
        self.stage = Stage::Loading;
    }

    /// Perform the queued chart load (one frame after the LOADING screen is shown) then enter Play —
    /// `load()` (re)bases the song clock at its end, so playback starts at 0 only after everything is
    /// loaded. Folder scans don't reach here (they run on a background thread, polled in `frame`).
    pub(crate) fn finish_loading(&mut self) {
        match self.pending.take() {
            Some(Loading::Song(idx)) => match self.songs.get(idx) {
                Some(entry) => {
                    self.chart_path = entry.path.to_string_lossy().to_string();
                    self.result = None;
                    if self.load() { self.after_load(); } else { self.stage = Stage::Select; }
                }
                None => self.stage = Stage::Select,
            },
            _ => self.stage = Stage::Select,
        }
    }

    pub(crate) fn to_select_or_exit(&mut self, event_loop: &ActiveEventLoop) {
        if self.stage == Stage::Play {
            self.save_settings();
        }
        if self.songs.is_empty() {
            event_loop.exit();
        } else {
            self.audio = None;
            self.player = None;
            self.result = None;
            self.replay = None;
            self.replay_cursor = 0;
            self.record_modal = None;
            // Clear analysis state so a stale overlay/virtual-clock can't leak into the next play.
            self.analysis = false;
            self.analysis_manual = false;
            self.analysis_paused = false;
            self.analysis_rate = 1.0;
            self.analysis_us = 0;
            self.msoff.clear();
            self.stage = Stage::Select;
            self.rebuild_select_items();
            self.print_selection();
        }
    }

    pub(crate) fn setting_line(&self, i: usize) -> (&'static str, String) {
        let on = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
        match i {
            0 => ("AUTOPLAY", on(self.autoplay)),
            1 => ("HI-SPEED", format!("{:.2}", self.config.hispeed)),
            2 => ("SPEED FIX", if self.config.constant_speed { "CONSTANT".to_string() } else { "FLOATING".to_string() }),
            3 => ("RANDOM", self.config.random.label().to_string()),
            4 => ("GAUGE", gauge_name(self.config.gauge).to_string()),
            5 => ("LIFT", format!("{}%", (self.config.lift * 100.0).round() as i32)),
            6 => ("LANE COVER", format!("{}%", (self.config.cover * 100.0).round() as i32)),
            7 => ("SCRATCH SIDE", if self.config.scratch_left { "LEFT".to_string() } else { "RIGHT".to_string() }),
            8 => ("SCRATCH AUTO", on(self.config.scratch_auto)),
            9 => ("JUDGE OFFSET", format!("{:+} MS", self.config.offset_ms)),
            10 => ("BGA", on(self.config.bga)),
            11 => ("KEY CONFIG", ">".to_string()),
            12 => ("JUDGE WIDTH", format!("{}%", self.config.judge_rate)),
            13 => ("TOTAL", if self.config.total_override > 0.0 { format!("{}", self.config.total_override.round() as i32) } else { "AUTO".to_string() }),
            14 => ("SKIN", if self.config.skin_path.is_some() { "CUSTOM".to_string() } else { self.config.skin_name.clone() }),
            15 => ("AUTO CAL", on(self.config.auto_offset)),
            16 => ("AUTO REPLAY", on(self.config.auto_replay)),
            17 => ("DEBUG MODE", on(self.config.debug)),
            18 => ("FONT", if self.config.font_path.is_some() { "CUSTOM".to_string() } else { "DEFAULT".to_string() }),
            19 => ("SCORE GRAPH", on(self.config.score_graph)),
            20 => ("REPLAY ANALYSIS", on(self.config.replay_analysis)),
            21 => ("PREVIEW", on(self.config.preview)),
            22 => ("SERVER URL", self.config.server_url.clone().unwrap_or_else(|| "(none)".to_string())),
            23 => ("PLAYER ID", self.config.player_id.clone()),
            _ => ("", String::new()),
        }
    }

    pub(crate) fn adjust_setting(&mut self, global: usize, d: i32) {
        match global {
            0 => self.autoplay = !self.autoplay,
            1 => self.config.hispeed = (self.config.hispeed + d as f64 * 0.25).clamp(0.5, 10.0),
            2 => self.config.constant_speed = !self.config.constant_speed,
            3 => {
                let idx = NoteOption::ALL.iter().position(|o| *o == self.config.random).unwrap_or(0);
                self.config.random = NoteOption::ALL[((idx as i32 + d).rem_euclid(NoteOption::ALL.len() as i32)) as usize];
            }
            4 => {
                let idx = GAUGE_CYCLE.iter().position(|g| *g == self.config.gauge).unwrap_or(2);
                self.config.gauge = GAUGE_CYCLE[((idx as i32 + d).rem_euclid(GAUGE_CYCLE.len() as i32)) as usize];
            }
            5 => self.config.lift = (self.config.lift + d as f32 * 0.05).clamp(0.0, 0.9),
            6 => self.config.cover = (self.config.cover + d as f32 * 0.05).clamp(0.0, 0.9),
            7 => self.config.scratch_left = !self.config.scratch_left,
            8 => self.config.scratch_auto = !self.config.scratch_auto,
            9 => self.config.offset_ms = (self.config.offset_ms + d * 5).clamp(-200, 200),
            10 => self.config.bga = !self.config.bga,
            12 => self.config.judge_rate = (self.config.judge_rate + d * 5).clamp(50, 200),
            13 => self.config.total_override = (self.config.total_override + d as f64 * 10.0).max(0.0),
            15 => self.config.auto_offset = !self.config.auto_offset,
            16 => self.config.auto_replay = !self.config.auto_replay,
            17 => self.config.debug = !self.config.debug,
            19 => self.config.score_graph = !self.config.score_graph,
            20 => {
                self.config.replay_analysis = !self.config.replay_analysis;
                // Re-evaluate an in-progress replay so toggling the setting takes effect immediately.
                if self.stage == Stage::Play {
                    self.analysis = self.replay.is_some() && self.config.replay_analysis;
                }
            }
            21 => self.config.preview = !self.config.preview,
            18 => {
                if d < 0 {
                    self.reset_font();
                } else {
                    self.pick_font();
                }
            }
            14 => {
                self.config.skin_path = None;
                self.config.skin_name = if self.config.skin_name.eq_ignore_ascii_case("WIDE") { "NORMAL".into() } else { "WIDE".into() };
            }
            _ => {}
        }
    }

}

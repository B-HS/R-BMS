//! `App` methods for PLAY: chart load, the per-frame play loop, the song clock, and the
//! result screen + score submission. Split out of `main.rs` (see `app_input` for the import note).
#![allow(clippy::wildcard_imports)]
use crate::ir_session::submission_player_id;
use crate::*;

impl App {
    /// Load the current `chart_path` and set up the player. Returns `false` (after logging)
    /// if the chart file can't be read, so callers can recover instead of panicking.
    pub(crate) fn load(&mut self) -> bool {
        // Drop the select preview engine before creating the Play engine, so the two cpal output
        // streams never coexist — covers every play-entry path (the direct replay launch bypasses the
        // Loading stage where frame() would otherwise tear it down).
        self.stop_preview();
        let bytes = match std::fs::read(&self.chart_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("chart not found: {} ({e})", self.chart_path);
                return false;
            }
        };
        let src = rbms_parser::parse_with(&bytes, Default::default());
        let mode = rbms_chart::detect_mode(&src, &self.chart_path);
        self.chart_lntype = ir_lntype(src.headers.lnmode);
        let mut model = to_model(&src, mode);
        self.recording.clear();
        self.replay_cursor = 0;
        self.cal_sum_us = 0;
        self.cal_count = 0;
        let (random, seed) = match &self.replay {
            Some(rp) => {
                if !rp.md5.is_empty() && rp.md5 != src.md5 {
                    eprintln!("warning: chart md5 mismatch (replay {} vs file {}); replay may desync", rp.md5, src.md5);
                }
                self.config.offset_ms = rp.offset_ms;
                self.config.scratch_auto = rp.scratch_auto;
                self.config.gauge = gauge_from_name(&rp.gauge);
                let random = NoteOption::from_str(&rp.random);
                self.config.random = random;
                (random, rp.seed)
            }
            None => (self.config.random, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)),
        };
        self.seed = seed;
        rbms_chart::shuffle::apply(&mut model, random, seed);
        if model.meta.total <= 0.0 {
            model.meta.total = default_total(rbms_chart::count_playable_notes(&model));
        }
        self.ks_cancel = Arc::new(AtomicBool::new(false));
        self.audio_dead_at.set(None);
        if self.config.total_override > 0.0 {
            model.meta.total = self.config.total_override;
        }
        self.mode = mode;
        self.active_keys = self.config.keys_override.clone().unwrap_or_else(|| self.keyconfig.lane_keys(mode));
        self.skin_cfg = match &self.config.skin_path {
            Some(p) => SkinConfig::load(p).unwrap_or_else(|e| {
                eprintln!("skin load failed ({e}), using bundled");
                bundled_skin(&self.config.skin_name)
            }),
            None => bundled_skin(&self.config.skin_name),
        };
        self.rebuild_skin();
        let dir = Path::new(&self.chart_path).parent().unwrap_or(Path::new(".")).to_path_buf();

        match AudioEngine::new() {
            Ok(audio) => {
                // Decode keysounds on BACKGROUND threads (qualia has 650+; serial decode was ~6s and
                // froze the UI). `frame()` drains them and draws a progress bar, and `start_play()`
                // captures the song clock once they're all in. (Shared with the autoplay preview.)
                let jobs = keysound_jobs(&model.wavmap, &dir);
                let total = jobs.len();
                println!("device {} Hz — decoding {total} keysounds...", audio.out_rate());
                self.audio = Some(audio);
                if total == 0 {
                    self.ks_rx = None;
                    self.ks_total = 0;
                } else {
                    let (rx, progress, _) = spawn_keysound_decode(jobs, self.ks_cancel.clone());
                    self.ks_rx = Some(rx);
                    self.ks_progress = progress;
                    self.ks_total = total;
                }
            }
            Err(e) => {
                println!("audio unavailable ({e}) — visual only");
                self.ks_rx = None;
                self.ks_total = 0;
            }
        }

        let status = if self.autoplay {
            "AUTOPLAY".to_string()
        } else {
            let mut keys = self.active_keys.clone();
            keys.sort_by_key(|(_, lane)| *lane);
            let hint = keys
                .iter()
                .map(|(code, lane)| {
                    let name = key_name(*code);
                    if mode.is_scratch(*lane) { format!("{name}=SC") } else { name.to_string() }
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("interactive ({hint})")
        };
        println!("playing '{}' [{}] ({} notes) — {}", model.meta.title, mode.name, rbms_chart::count_playable_notes(&model), status);
        self.bga_images.clear();
        if self.config.bga && self.skin.bga.is_some() {
            for (id, name) in model.bgamap.iter().enumerate() {
                if name.is_empty() {
                    continue;
                }
                if let Some(rgba) = decode_bga_256(&dir, name) {
                    self.bga_images.insert(id as i32, rgba);
                }
            }
        }
        self.bga_events = model.timelines.iter().filter(|tl| tl.bga >= 0).map(|tl| (tl.time_us, tl.bga)).collect();
        self.bga_events.sort_by_key(|e| e.0);
        self.bga_cursor = 0;
        self.cur_bga = -1;
        println!("loaded {} BGA images", self.bga_images.len());

        // NB: the song-clock anchor is captured later, in `start_play()` (once keysounds finish
        // decoding), so the time spent loading does not count against the song position.
        // During replay playback the recorded input stream drives judging, so the player must not
        // also autoplay (that would double-hit every note).
        let mut player = Player::new(model, self.autoplay && self.replay.is_none());
        player.set_gauge(self.config.gauge);
        player.set_judge_rate(self.config.judge_rate);
        if self.config.scratch_auto {
            let auto: Vec<bool> = (0..mode.key).map(|l| mode.is_scratch(l)).collect();
            player.set_auto_lanes(auto);
        }
        self.player = Some(player);
        // Replay analysis starts following the live clock with sound; it only switches to the
        // virtual clock once the user takes manual control.
        self.analysis = self.replay.is_some() && self.config.replay_analysis;
        self.analysis_manual = false;
        self.analysis_paused = false;
        self.analysis_rate = 1.0;
        self.analysis_us = 0;
        self.msoff.clear();
        true
    }

    /// Transition after a successful `load()`: if keysounds are still decoding stay on the Loading
    /// screen (its progress bar + `poll_keysound_load` drive the rest), otherwise start play now.
    pub(crate) fn after_load(&mut self) {
        if self.ks_rx.is_some() {
            self.stage = Stage::Loading;
        } else {
            self.start_play();
        }
    }

    /// Begin actual play: capture the song-clock anchor (so position starts at ~0) and enter the Play
    /// stage. Called once keysounds finish decoding (or immediately when there are none).
    pub(crate) fn start_play(&mut self) {
        self.clock = Instant::now();
        self.anchor_us = self.audio.as_ref().map(|a| a.clock_us()).unwrap_or(0);
        self.audio_dead_at.set(None);
        self.stage = Stage::Play;
    }

    /// Drain decoded keysounds into the audio bank; once every worker has reported in, start play.
    /// Drives the determinate loading bar (`ks_progress` / `ks_total`).
    pub(crate) fn poll_keysound_load(&mut self) {
        if self.ks_rx.is_none() {
            return;
        }
        if let (Some(rx), Some(audio)) = (self.ks_rx.as_ref(), self.audio.as_mut()) {
            while let Ok((id, dec)) = rx.try_recv() {
                audio.insert_decoded(id, dec);
            }
        }
        if self.ks_progress.load(std::sync::atomic::Ordering::Relaxed) >= self.ks_total {
            // All workers done. A sample may have been sent just before its progress tick, so do a
            // final drain before dropping the receiver, then start play.
            if let (Some(rx), Some(audio)) = (self.ks_rx.as_ref(), self.audio.as_mut()) {
                while let Ok((id, dec)) = rx.try_recv() {
                    audio.insert_decoded(id, dec);
                }
            }
            self.ks_rx = None;
            println!("keysounds ready ({} decoded)", self.audio.as_ref().map(|a| a.loaded()).unwrap_or(0));
            self.start_play();
        }
    }

    /// Current song position. Normally the audio output clock (sample-accurate); if that stream has
    /// died (device unplugged / driver error) it stops advancing, so fall back to the wall clock,
    /// resuming from the last position the audio clock reported instead of freezing the chart.
    pub(crate) fn song_us(&self) -> i64 {
        let Some(audio) = self.audio.as_ref() else {
            return self.clock.elapsed().as_micros() as i64;
        };
        if audio.is_alive() {
            self.audio_dead_at.set(None);
            return audio.clock_us() - self.anchor_us;
        }
        let (last, at) = match self.audio_dead_at.get() {
            Some(v) => v,
            None => {
                let v = (audio.clock_us() - self.anchor_us, Instant::now());
                self.audio_dead_at.set(Some(v));
                eprintln!("audio stream stopped — falling back to the wall clock");
                v
            }
        };
        resumed_clock_us(last, at.elapsed().as_micros() as i64)
    }

    pub(crate) fn enter_result(&mut self) {
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let j = &player.judge;
        let lamp = j.clear_lamp();
        let (label, color) = clear_label_color(lamp);
        println!(
            "RESULT [{label}]  EX {}/{}  combo {}/{}  gauge {:.1}%  PG/GR/GD/BD/POOR/MISS {:?}  empty-poor {}",
            j.ex_score,
            j.total_notes() * 2,
            j.max_combo,
            j.total_notes(),
            j.gauge.value(),
            j.counts,
            j.empty_poor,
        );
        // Deltas compare against the records that existed BEFORE this play (this run's record is
        // pushed further down), so prev = newest stored play, best = max stored EX.
        let history = self.scores.for_md5(&player.model().md5);
        let prev_ex = history.first().map(|r| r.ex_score);
        let prev_best_ex = history.iter().map(|r| r.ex_score).max();
        self.result = Some(ResultView {
            title: player.model().meta.title.chars().take(48).collect(),
            counts: j.counts,
            ex_score: j.ex_score,
            max_score: j.total_notes() * 2,
            max_combo: j.max_combo,
            total_notes: j.total_notes(),
            fast: j.fast,
            slow: j.slow,
            gauge: j.gauge.value(),
            clear_label: label,
            clear_color: color,
            prev_best_ex,
            prev_ex,
            show_graph: self.config.score_graph,
        });

        let model = player.model();
        let assist = assist_flags(self.config.scratch_auto, self.config.judge_rate);
        let c = j.counts;
        let played_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        let sub = ScoreSubmission {
            api_version: API_VERSION,
            chart: ChartId { md5: model.md5.clone(), sha256: model.sha256.clone() },
            player: PlayerId { id: submission_player_id(&self.session, &self.config.player_id) },
            mode: self.mode.name.to_string(),
            clear: ir_clear(lamp),
            ex_score: j.ex_score,
            max_ex_score: j.total_notes() * 2,
            judge: JudgeBreakdown {
                pgreat: c[0],
                great: c[1],
                good: c[2],
                bad: c[3],
                poor: c[4],
                miss: c[5],
                fast: j.fast,
                slow: j.slow,
                combobreak: combo_breaks(&self.mode, c),
                epg: j.early[0],
                lpg: j.late[0],
                egr: j.early[1],
                lgr: j.late[1],
                egd: j.early[2],
                lgd: j.late[2],
                ebd: j.early[3],
                lbd: j.late[3],
                epr: j.early[4],
                lpr: j.late[4],
                ems: j.early[5],
                lms: j.late[5],
                avgjudge: j.avg_judge_us(),
                empty_poor: j.empty_poor,
            },
            max_combo: j.max_combo,
            total_notes: j.total_notes(),
            passnotes: j.total_judged(),
            minbp: c[3] + c[4] + c[5],
            gauge_value: j.gauge.value(),
            options: PlayOptions {
                gauge: ir_gauge(self.config.gauge),
                random: ir_random(self.config.random),
                random_p2: None,
                scratch_auto: self.config.scratch_auto,
                lntype: self.chart_lntype,
                input_device: "keyboard".into(),
                assist,
                option: 0,
                judge_rate: self.config.judge_rate,
                offset_ms: self.config.offset_ms,
                constant: self.config.constant_speed,
                hispeed: self.config.hispeed,
                lift: self.config.lift,
                lane_cover: self.config.cover,
                total_override: self.config.total_override,
                autoplay: self.autoplay,
                auto_offset: self.config.auto_offset,
                scratch_left: self.config.scratch_left,
                green_number: green_number_for(self.config.constant_speed, model.init_bpm, self.config.hispeed, 1.0, self.config.cover),
            },
            played_at,
            client: concat!("rbms/", env!("CARGO_PKG_VERSION")).into(),
            replay_id: None,
            seed: self.seed,
            judge_algorithm: "Combo".into(),
            rule: String::new(),
            skin: self.config.skin_name.clone(),
            client_build_sha256: self.build_sha256.clone(),
            client_platform: Some(client_platform()),
            extra: Default::default(),
        };
        let scores_count = updates_score(self.autoplay, self.replay.is_some(), self.config.judge_rate, self.config.scratch_auto);
        let block_reason = ir_submission_block_reason(self.autoplay, self.replay.is_some(), self.config.judge_rate, self.config.scratch_auto);

        let played_ms = played_at;
        let mut replay_file: Option<String> = None;
        let mut recorded_replay: Option<Replay> = None;
        if self.config.auto_replay && self.replay.is_none() && !self.autoplay && !self.recording.is_empty() {
            let stem: String = model.md5.chars().take(8).collect();
            let dir = self.settings_path.parent().map(|d| d.join("replays")).unwrap_or_else(|| PathBuf::from("replays"));
            let name = format!("{stem}-{played_ms}.ron");
            let rp = Replay {
                chart_path: self.chart_path.clone(),
                md5: model.md5.clone(),
                mode: self.mode.name.to_string(),
                random: self.config.random.label().to_string(),
                seed: self.seed,
                offset_ms: self.config.offset_ms,
                scratch_auto: self.config.scratch_auto,
                gauge: gauge_token(self.config.gauge).to_string(),
                events: self.recording.clone(),
            };
            rp.save(&dir.join(&name));
            replay_file = Some(name);
            recorded_replay = Some(rp);
        }

        // Persist a local play record (independent of the score server) for every real
        // interactive play, so history/replays survive offline. autoplay/replay runs are excluded.
        if self.replay.is_none() && !self.autoplay {
            let record = ScoreRecord {
                md5: model.md5.clone(),
                title: model.meta.title.clone(),
                mode: self.mode.name.to_string(),
                clear: clear_type_id(lamp),
                ex_score: j.ex_score,
                max_ex: j.total_notes() * 2,
                counts: j.counts,
                empty_poor: j.empty_poor,
                max_combo: j.max_combo,
                total_notes: j.total_notes(),
                gauge: gauge_token(self.config.gauge).to_string(),
                gauge_value: j.gauge.value(),
                random: self.config.random.label().to_string(),
                played_at: played_ms,
                replay_file,
                rule_version: SCORE_RULE_VERSION,
                assisted: !scores_count,
            };
            self.scores.push(record);
            self.scores.save(&self.scores_path);
        }

        if self.config.auto_offset && !self.autoplay && self.replay.is_none() && self.cal_count >= 20 {
            let mean_us = self.cal_sum_us / self.cal_count as i64;
            let new_offset = calibrated_offset(self.config.offset_ms, mean_us);
            println!("auto-cal: avg {:+} ms over {} hits → judge offset {} ms (was {})", mean_us / 1000, self.cal_count, new_offset, self.config.offset_ms);
            self.config.offset_ms = new_offset;
        }

        match block_reason {
            Some(reason) => {
                println!("score not submitted: {reason}");
                self.submit_rx = None;
                self.ir_status = IrStatus::Skipped(reason);
            }
            None if self.config.server_url.is_none() => {
                self.submit_rx = None;
                self.ir_status = IrStatus::Off;
            }
            None => {
                let chart = sub.chart.clone();
                let replay = recorded_replay.as_ref().and_then(|rp| self.replay_upload_payload(rp, &chart));
                self.spawn_score_submit(sub, replay);
            }
        }

        self.save_settings();
        self.stage = Stage::Result;
    }

    pub(crate) fn frame(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        if dt > 0.0 {
            let inst = 1.0 / dt;
            self.fps = if self.fps <= 0.0 { inst } else { self.fps * 0.9 + inst * 0.1 };
        }
        self.frame_count = self.frame_count.wrapping_add(1);
        self.poll_network();
        self.poll_ir_jobs();
        if self.config.debug && self.frame_count % 15 == 0 {
            if let Some(u) = memory_stats::memory_stats() {
                self.ram_mb = u.physical_mem as f32 / (1024.0 * 1024.0);
            }
        }
        if self.stage == Stage::Loading {
            if self.scan_rx.is_some() {
                // A background folder scan is running; apply it the frame it finishes, otherwise keep
                // animating the LOADING screen.
                match self.scan_rx.as_ref().unwrap().try_recv() {
                    Ok(out) => self.apply_scan(out),
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.scan_rx = None;
                        self.pending = None;
                        self.stage = Stage::Select;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
            } else if self.ks_rx.is_some() {
                // Keysounds are decoding on background threads — drain them and (when done) start play.
                self.poll_keysound_load();
            } else if self.loading_drawn {
                self.finish_loading();
            }
        }
        if self.stage == Stage::Select {
            self.refresh_focused_detail();
            self.update_preview();
            self.update_ranking();
        } else if self.preview_audio.is_some() {
            self.stop_preview();
        }
        // In manual analysis the displayed song time comes from the virtual clock (pausable,
        // rate-scaled); otherwise it follows the real (audio) clock, and analysis mirrors it so a
        // first manual control resumes from the live position. Manual analysis mutes keysounds.
        let manual = self.stage == Stage::Play && self.analysis && self.analysis_manual;
        let song = if manual {
            if !self.analysis_paused {
                self.analysis_us = (self.analysis_us + (dt as f64 * 1_000_000.0 * self.analysis_rate) as i64).max(0);
            }
            self.analysis_us
        } else {
            let s = self.song_us();
            if self.stage == Stage::Play && self.analysis {
                self.analysis_us = s;
            }
            s
        };
        let anchor = self.anchor_us;

        if self.stage == Stage::Play {
            if self.replay.is_some() {
                self.feed_replay(song, anchor, manual);
            }
            if let (false, Some(audio)) = (manual, self.audio.as_mut()) {
                let play = |e: PlayEvent| audio.play(e.wav.max(0) as u32, 1.0, 0.0, 1.0, e.at_us + anchor);
                if let Some(player) = self.player.as_mut() {
                    player.update(song, play);
                }
            } else if let Some(player) = self.player.as_mut() {
                player.update(song, |_| {});
            }
            if let Some(player) = self.player.as_ref() {
                // Analysis stays on the field at the end (the player scrubs); only a non-analysis
                // run auto-advances to the result screen.
                if !self.analysis && player.judge.total_notes() > 0 && song > player.last_time_us() + 2_000_000 {
                    self.enter_result();
                }
            }
        }

        let settings_scene = (self.stage == Stage::Settings).then(|| self.settings_scene());
        let ranking_lines = if self.stage == Stage::Select && self.ranking_open { self.ranking_lines() } else { Vec::new() };
        let ir_lines = if self.stage == Stage::Result { self.ir_status.lines() } else { Vec::new() };
        // Refresh the (cached) select scene before the GPU borrow, then read it as a disjoint immutable
        // field so it coexists with the mutable `self.gpu` borrow during render.
        if self.stage == Stage::Select {
            self.refresh_select_cache();
        }
        let select_scene = if self.stage == Stage::Select { self.cached_select.as_ref() } else { None };

        if let Some(gpu) = self.gpu.as_mut() {
            self.hot.clear();
            match self.stage {
                Stage::Loading => {
                    let th = rbms_render::theme();
                    gpu.clear_bga();
                    gpu.clear(th.bg);
                    let scanning = self.scan_rx.is_some();
                    let (heading, sub): (&str, String) = match &self.pending {
                        Some(Loading::Song(i)) => ("LOADING", self.songs.get(*i).map(|e| e.title.chars().take(30).collect()).unwrap_or_default()),
                        Some(Loading::Scan) => ("SCANNING", format!("{} folder(s)", self.folders.len())),
                        None => ("LOADING", String::new()),
                    };
                    let cx = CW as f32 * 0.5;
                    let cy = CH as f32 * 0.5;
                    let dots = ".".repeat((self.frame_count / 12 % 4) as usize);
                    draw_text_centered(gpu, cx, cy - 36.0, 3.0, th.text, &format!("{heading}{dots}"));
                    if !sub.is_empty() {
                        draw_text_centered(gpu, cx, cy + 14.0, 1.6, th.text_dim, &sub);
                    }
                    let (bw, bh) = (360.0, 8.0);
                    let bx = cx - bw * 0.5;
                    let by = cy + 56.0;
                    if self.ks_rx.is_some() {
                        // Determinate keysound-decode progress.
                        let done = self.ks_progress.load(std::sync::atomic::Ordering::Relaxed).min(self.ks_total);
                        let frac = if self.ks_total > 0 { done as f32 / self.ks_total as f32 } else { 1.0 };
                        gpu.fill_rect(Rect::new(bx, by, bw, bh), Color::rgb(28, 28, 40));
                        gpu.fill_rect(Rect::new(bx, by, bw * frac, bh), th.good);
                        draw_text_centered(gpu, cx, by + 16.0, 1.2, th.text_dim, &format!("{done} / {} keysounds", self.ks_total));
                    } else if scanning {
                        // Indeterminate sweeping bar (ping-pong) so a long background scan reads as
                        // working, not frozen (a folder scan has no total to count toward), plus a live
                        // count of charts found so far.
                        let seg = 96.0;
                        gpu.fill_rect(Rect::new(bx, by, bw, bh), Color::rgb(28, 28, 40));
                        let phase = (self.frame_count % 120) as f32 / 60.0;
                        let t = if phase <= 1.0 { phase } else { 2.0 - phase };
                        gpu.fill_rect(Rect::new(bx + t * (bw - seg), by, seg, bh), th.good);
                        let found = self.scan_count.load(std::sync::atomic::Ordering::Relaxed);
                        draw_text_centered(gpu, cx, by + 16.0, 1.2, th.text_dim, &format!("{found} charts found"));
                    }
                    self.loading_drawn = true;
                }
                Stage::Select => {
                    let view = select_scene.unwrap();
                    // Upload the focused cover into the single BGA slot (drawn behind the panel quads);
                    // the renderer leaves the cover square unfilled so the texture shows through.
                    match (&self.cover_rgba, &view.detail) {
                        (Some(rgba), SelectDetail::Song(_)) => gpu.set_bga(rgba, cover_rect()),
                        _ => gpu.clear_bga(),
                    }
                    let hot = render_select(gpu, view);
                    self.hot.extend(hot.into_iter().map(|(rect, h)| {
                        let mapped = match h {
                            SelectHot::Row(i) => Hot::SelectRow(i),
                            SelectHot::Record(i) => Hot::RecordRow(i),
                            SelectHot::ModalReplay => Hot::ModalReplay,
                            SelectHot::ModalClose => Hot::ModalClose,
                            SelectHot::Search => Hot::NavSearch,
                            SelectHot::Sort => Hot::NavSort,
                            SelectHot::Folders => Hot::NavFolders,
                            SelectHot::Tables => Hot::NavTables,
                            SelectHot::Records => Hot::NavRecords,
                            SelectHot::Settings => Hot::NavSettings,
                        };
                        (rect, mapped)
                    }));
                    if self.ranking_open {
                        let hot = render_ranking_panel(gpu, &ranking_lines, self.ranking_sel, true);
                        self.hot.extend(hot.into_iter().map(|(rect, index)| (rect, Hot::RankingRow(index))));
                    }
                }
                Stage::Play => {
                    while self.bga_cursor < self.bga_events.len() && self.bga_events[self.bga_cursor].0 <= song {
                        self.cur_bga = self.bga_events[self.bga_cursor].1;
                        self.bga_cursor += 1;
                    }
                    match (self.config.bga, self.skin.bga, self.bga_images.get(&self.cur_bga)) {
                        (true, Some(rect), Some(img)) => gpu.set_bga(img, rect),
                        _ => gpu.clear_bga(),
                    }
                    if let Some(player) = self.player.as_ref() {
                        render_playfield(
                            gpu,
                            &player.model().timelines,
                            song,
                            self.config.hispeed,
                            &self.skin,
                            player.beam_on(),
                            player.beam_off(),
                            self.config.constant_speed,
                        );
                        render_lane_cover(gpu, &self.skin, self.config.cover);
                        render_key_bomb(gpu, &self.skin, player.bomb(), song);
                        let j = &player.judge;
                        // Green number = note travel time (ms) for the scroll speed shown right now. In
                        // CONSTANT mode it is fixed (2000/hi-speed at the calibration BPM); in FLOATING
                        // it tracks the BPM of the timeline segment under the current play time.
                        let tls = &player.model().timelines;
                        let seg = tls.binary_search_by(|t| t.time_us.cmp(&song)).unwrap_or_else(|i| i.saturating_sub(1));
                        let (bpm, scroll) = tls.get(seg).map(|t| (t.bpm, t.scroll)).unwrap_or((player.model().init_bpm, 1.0));
                        let green = green_number_for(self.config.constant_speed, bpm, self.config.hispeed, scroll, self.config.cover);
                        let best_ex = self.scores.best_ex_for_md5(&player.model().md5);
                        let hud = HudView {
                            combo: j.combo,
                            last_judge: j.last_judge.map(|x| x as u8),
                            last_fast: j.last_fast,
                            fast: j.fast,
                            slow: j.slow,
                            counts: j.counts,
                            ex_score: j.ex_score,
                            gauge: j.gauge.value(),
                            green_number: green,
                            max_ex: j.total_notes() * 2,
                            best_ex,
                        };
                        render_hud(gpu, &self.skin, &hud);

                        // Replay analysis overlay: a playback bar, the current rate/paused state and
                        // time, plus the recent per-note timing errors (ms early = cyan +, late = orange -).
                        if self.analysis {
                            let total = player.last_time_us().max(1);
                            let prog = (song as f32 / total as f32).clamp(0.0, 1.0);
                            let bx = 40.0;
                            let bw = CW as f32 - 80.0;
                            let by = CH as f32 - 14.0;
                            gpu.fill_rect(Rect::new(0.0, CH as f32 - 74.0, CW as f32, 74.0), Color { r: 0, g: 0, b: 0, a: 170 });
                            gpu.fill_rect(Rect::new(bx, by, bw, 6.0), Color::rgb(40, 40, 52));
                            gpu.fill_rect(Rect::new(bx, by, bw * prog, 6.0), Color::rgb(90, 200, 230));
                            gpu.fill_rect(Rect::new((bx + bw * prog - 1.5).max(bx), by - 3.0, 3.0, 12.0), Color::WHITE);
                            let state = if self.analysis_paused { "PAUSED".to_string() } else { format!("{:.2}x", self.analysis_rate) };
                            draw_text(
                                gpu,
                                bx,
                                CH as f32 - 66.0,
                                1.4,
                                Color::YELLOW,
                                &format!("ANALYSIS  {state}   {:.1} / {:.1} S", song as f32 / 1e6, total as f32 / 1e6),
                            );
                            draw_text_right(gpu, bx + bw, CH as f32 - 66.0, 1.1, Color::GRAY, "SPACE PAUSE  -/+ SPEED  PGUP/PGDN SEEK  ESC EXIT");
                            let start = self.msoff.len().saturating_sub(14);
                            let mut mx = bx;
                            for &(_, delta_us, _) in &self.msoff[start..] {
                                let col = if delta_us > 0 {
                                    Color::rgb(90, 210, 230)
                                } else if delta_us < 0 {
                                    Color::ORANGE
                                } else {
                                    Color::WHITE
                                };
                                let txt = format!("{:+}", delta_us / 1000);
                                draw_text(gpu, mx, CH as f32 - 44.0, 1.3, col, &txt);
                                mx += text_width(&txt, 1.3) + 12.0;
                            }
                        }
                    }
                }
                Stage::Settings => {
                    gpu.clear_bga();
                    if let Some(scene) = settings_scene.as_ref() {
                        let hot = render_settings(gpu, scene);
                        self.hot.extend(hot.into_iter().map(|(rect, h)| {
                            let mapped = match h {
                                SettingsHot::Tab(i) => Hot::SettingTab(i),
                                SettingsHot::Row(i) => Hot::SettingRow(i),
                                SettingsHot::RivalRow(i) => Hot::RivalRow(i),
                            };
                            (rect, mapped)
                        }));
                    }
                }
                Stage::KeyConfig => {
                    let th = rbms_render::theme();
                    gpu.clear_bga();
                    gpu.clear(th.bg);
                    const PANEL_W: f32 = 760.0;
                    let x0 = (CW as f32 - PANEL_W) * 0.5;
                    draw_text(gpu, x0, 40.0, 3.0, th.text, "KEY CONFIG");
                    let (hint, hint_col) = if self.kc_warn {
                        ("KEY ALREADY BOUND - TRY ANOTHER", Color::RED)
                    } else if self.kc_capturing {
                        ("PRESS A KEY...   ESC CANCEL", Color::YELLOW)
                    } else {
                        ("UP DOWN MOVE   ENTER REBIND   LEFT RIGHT EDIT-MODE   ESC SAVE/BACK", th.text_muted)
                    };
                    draw_text(gpu, x0, 82.0, 1.3, hint_col, hint);
                    let dups = self.keyconfig.collisions(self.kc_edit_mode);
                    let rows = kc_rows(self.kc_edit_mode);
                    let row_h = 28.0;
                    let visible = 17usize;
                    let start = self.kc_sel.saturating_sub(visible / 2).min(rows.len().saturating_sub(visible.min(rows.len())));
                    for (i, ridx) in (start..(start + visible).min(rows.len())).enumerate() {
                        let y = 112.0 + i as f32 * (row_h + 2.0);
                        let on = ridx == self.kc_sel;
                        let (label, raw) = match &rows[ridx] {
                            KcRow::ModeSelect => ("EDIT MODE".to_string(), mode_short(self.kc_edit_mode).to_string()),
                            KcRow::Control(a) => (a.label().to_string(), self.keyconfig.control_token(*a).to_string()),
                            KcRow::Lane(lane) => {
                                let name = if self.kc_edit_mode.is_scratch(*lane) { format!("SCRATCH {}", lane + 1) } else { format!("LANE {}", lane + 1) };
                                (name, self.keyconfig.lane_token(self.kc_edit_mode, *lane))
                            }
                        };
                        let is_dup = !matches!(&rows[ridx], KcRow::ModeSelect) && key_from_name(&raw).is_some_and(|k| dups.contains(&k));
                        gpu.fill_rect(Rect::new(x0, y, PANEL_W, row_h), if on { th.button } else { th.panel });
                        draw_text(gpu, x0 + 16.0, y + 8.0, 1.6, if on { th.text } else { th.text_dim }, &label);
                        let value = if on && self.kc_capturing {
                            "?".to_string()
                        } else if raw.is_empty() {
                            "-".to_string()
                        } else {
                            raw
                        };
                        let vcol = if on && self.kc_capturing {
                            Color::YELLOW
                        } else if is_dup {
                            Color::RED
                        } else if on {
                            th.accent
                        } else {
                            th.text
                        };
                        draw_text_right(gpu, x0 + PANEL_W - 16.0, y + 8.0, 1.6, vcol, &value);
                    }
                }
                Stage::Tables => {
                    let th = rbms_render::theme();
                    gpu.clear_bga();
                    gpu.clear(th.bg);
                    const PANEL_W: f32 = 900.0;
                    let x0 = (CW as f32 - PANEL_W) * 0.5;
                    draw_text(gpu, x0, 40.0, 3.0, th.text, "DIFFICULTY TABLES");
                    if let Some(buf) = self.text_input.as_ref() {
                        draw_text(gpu, x0, 84.0, 1.4, Color::YELLOW, "TYPE TABLE URL  -  ENTER ADD  ESC CANCEL");
                        gpu.fill_rect(Rect::new(x0, 116.0, PANEL_W, 40.0), th.button_active);
                        let shown: String = buf.chars().rev().take(70).collect::<Vec<_>>().into_iter().rev().collect();
                        draw_text(gpu, x0 + 14.0, 128.0, 1.6, th.text, &format!("{shown}_"));
                    } else {
                        draw_text(gpu, x0, 84.0, 1.3, th.text_muted, "UP DOWN MOVE   ENTER SELECT   D REMOVE   ESC BACK");
                        let add_url = self.table_sources.len();
                        let row_count = self.table_sources.len() + 2;
                        for i in 0..row_count {
                            let y = 124.0 + i as f32 * 44.0;
                            let on = i == self.tables_sel;
                            let label = if i < self.table_sources.len() {
                                let s = &self.table_sources[i];
                                let nm = if s.name.is_ascii() && !s.name.trim().is_empty() { s.name.clone() } else { String::new() };
                                let loc: String = s.location.chars().take(64).collect();
                                if nm.is_empty() { loc } else { format!("{nm}  -  {loc}") }
                            } else if i == add_url {
                                "+ ADD TABLE (URL)".to_string()
                            } else {
                                "+ ADD TABLE (FILE)".to_string()
                            };
                            gpu.fill_rect(Rect::new(x0, y, PANEL_W, 36.0), if on { th.button } else { th.panel });
                            let col = if i >= self.table_sources.len() {
                                Color::GREEN
                            } else if on {
                                th.text
                            } else {
                                th.text_dim
                            };
                            draw_text(gpu, x0 + 16.0, y + 10.0, 1.5, col, &label);
                        }
                    }
                }
                Stage::Folders => {
                    let th = rbms_render::theme();
                    gpu.clear_bga();
                    gpu.clear(th.bg);
                    const PANEL_W: f32 = 980.0;
                    let x0 = (CW as f32 - PANEL_W) * 0.5;
                    draw_text(gpu, x0, 40.0, 3.0, th.text, "SONG FOLDERS");
                    draw_text(gpu, x0, 84.0, 1.3, th.text_muted, "UP DOWN MOVE   ENTER ADD FOLDER   D REMOVE   ESC SAVE/RESCAN");
                    let add_row = self.folders.len();
                    for i in 0..self.folders.len() + 1 {
                        let y = 124.0 + i as f32 * 44.0;
                        let on = i == self.folders_sel;
                        let label = if i < self.folders.len() {
                            let f = &self.folders[i];
                            // Show the tail of long paths so the folder name stays visible.
                            if f.chars().count() > 78 {
                                format!("…{}", f.chars().rev().take(76).collect::<Vec<_>>().into_iter().rev().collect::<String>())
                            } else {
                                f.clone()
                            }
                        } else {
                            "+ ADD FOLDER".to_string()
                        };
                        gpu.fill_rect(Rect::new(x0, y, PANEL_W, 36.0), if on { th.button } else { th.panel });
                        let col = if i == add_row {
                            Color::GREEN
                        } else if on {
                            th.text
                        } else {
                            th.text_dim
                        };
                        draw_text(gpu, x0 + 16.0, y + 10.0, 1.4, col, &label);
                    }
                    if self.folders.is_empty() {
                        draw_text(gpu, x0, 124.0 + 44.0 * 2.0, 1.2, th.text_muted, "No folders yet — ENTER on \"+ ADD FOLDER\" to pick one.");
                    }
                }
                Stage::Result => {
                    gpu.clear_bga();
                    if let Some(view) = self.result.as_ref() {
                        render_result_with_palette(gpu, view, &self.result_palette);
                    }
                    for (i, (text, kind)) in ir_lines.iter().enumerate() {
                        draw_text(gpu, IR_RESULT_X, IR_RESULT_Y + i as f32 * IR_RESULT_LINE_H, IR_RESULT_SCALE, ir_line_color(*kind), text);
                    }
                }
            }
            if self.config.server_url.is_some() {
                let connected = self.server_connected.load(Ordering::Relaxed);
                gpu.fill_rect(Rect::new(CW as f32 - 22.0, 10.0, 10.0, 10.0), if connected { Color::GREEN } else { Color::RED });
            }
            if self.config.debug {
                let frame_ms = if self.fps > 0.0 { 1000.0 / self.fps } else { 0.0 };
                let mut lines = vec![
                    "DEBUG".to_string(),
                    format!("FPS {:.0}  ({:.1} MS)", self.fps, frame_ms),
                    format!("RAM {:.1} MB", self.ram_mb),
                    format!("QUADS {}  STAGE {:?}", gpu.quad_count(), self.stage),
                ];
                if self.stage == Stage::Play {
                    if let Some(p) = self.player.as_ref() {
                        let j = &p.judge;
                        lines.push(format!("TIME {:.2} / {:.2} S", song as f32 / 1e6, p.last_time_us() as f32 / 1e6));
                        lines.push(format!("NOTES {} / {}", j.total_judged(), j.total_notes()));
                        lines.push(format!("COMBO {}  MAX {}", j.combo, j.max_combo));
                        lines.push(format!("EX {}  GAUGE {:.1}%", j.ex_score, j.gauge.value()));
                        lines.push(format!("FAST {}  SLOW {}  EPOOR {}", j.fast, j.slow, j.empty_poor));
                    }
                    lines.push(format!("HISPEED {:.2}  OFFSET {:+}MS", self.config.hispeed, self.config.offset_ms));
                    let clk = self.audio.as_ref().map(|a| a.clock_us()).unwrap_or(0);
                    lines.push(format!("AUDIO {} US  ANCHOR {}", clk, anchor));
                    if let Some(a) = self.audio.as_ref() {
                        lines.push(format!(
                            "STREAM {}  DROP {}  REALLOC {}",
                            if a.is_alive() { "ALIVE" } else { "DEAD" },
                            a.dropped_commands(),
                            a.scratch_reallocations()
                        ));
                    }
                } else {
                    lines.push(format!("SEL {} / {}", self.sel + 1, self.select_items.len()));
                    lines.push(format!("SCORES {}  SONGS {}", self.scores.records.len(), self.songs.len()));
                    lines.push(format!("CURSOR {:.0} {:.0}", self.cursor.0, self.cursor.1));
                }
                let (layouts, runs) = rbms_render::cache_stats();
                lines.push(format!("FONT {}/{}  RUNS {}/{}", layouts, rbms_render::LAYOUT_CACHE_LIMIT, runs, rbms_render::RUN_CACHE_LIMIT));
                let lh = 16.0;
                let ph = lines.len() as f32 * lh + 12.0;
                gpu.fill_rect(Rect::new(6.0, 6.0, 320.0, ph), Color { r: 0, g: 0, b: 0, a: 180 });
                for (i, l) in lines.iter().enumerate() {
                    let col = if i == 0 { Color::YELLOW } else { Color::rgb(120, 240, 140) };
                    draw_text(gpu, 14.0, 12.0 + i as f32 * lh, 1.2, col, l);
                }
            }
            gpu.render();
        }
    }

    pub(crate) fn print_selection(&self) {
        let total = self.select_items.len();
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(i)) => {
                if let Some(e) = self.songs.get(*i) {
                    println!("[{}/{}] {} [{}]", self.sel + 1, total, e.title, e.mode.name);
                }
            }
            Some(SelectItem::Folder { label, .. }) => {
                println!("[{}/{}] {label}/", self.sel + 1, total)
            }
            None => {}
        }
    }
}

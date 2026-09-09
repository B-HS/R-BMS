//! `App` methods for PLAY: chart load, the per-frame play loop, the song clock, and the
//! result screen + score submission. Split out of `main.rs` (see `app_input` for the import note).
#![allow(clippy::wildcard_imports)]
use crate::ir_session::submission_player_id;
use crate::*;

/// How often the resident-set reading behind the debug overlay and the soak log is refreshed.
const RAM_SAMPLE_FRAMES: u64 = 15;

/// Width of the debug overlay panel, sized for its widest audio telemetry line.
const DEBUG_PANEL_W: f32 = 380.0;

/// How much of the output device's name the debug overlay shows before it is cut.
const DEVICE_NAME_OVERLAY_CHARS: usize = 20;

/// Stage label used by the soak log, kept stable so the CSV stays machine-readable.
fn stage_name(stage: &Stage) -> &'static str {
    match stage {
        Stage::Select => "Select",
        Stage::Settings => "Settings",
        Stage::KeyConfig => "KeyConfig",
        Stage::Tables => "Tables",
        Stage::Folders => "Folders",
        Stage::Loading => "Loading",
        Stage::Play => "Play",
        Stage::Result => "Result",
    }
}

/// Song position from a raw audio-clock reading: rebased on the play anchor, then clamped against
/// the previous reading so an interpolated clock can never step backwards inside a frame.
pub(crate) fn song_position_us(audio_clock_us: i64, anchor_us: i64, previous_us: i64) -> i64 {
    rbms_audio::monotonic_us(previous_us, audio_clock_us - anchor_us)
}

/// Shortest horizon the scheduler is allowed to book against, so a frame that measured as almost
/// instant does not shrink it to nothing.
const MIN_SCHEDULE_POLL_US: i64 = 1_000;

/// Longest horizon a frame-time spike may push the scheduler out to. Past this the sound is better
/// late than booked a visible fraction of a second early.
const MAX_SCHEDULE_POLL_US: i64 = 50_000;

/// How much of the previous frame's horizon survives into the next one, as a fraction. A peak hold
/// that decays keeps one long frame from being forgotten immediately and from inflating the horizon
/// for the rest of the song.
const SCHEDULE_POLL_DECAY_NUMERATOR: i64 = 15;
const SCHEDULE_POLL_DECAY_DENOMINATOR: i64 = 16;

/// The stretch of song the next scheduling pass has to cover. Sounds are booked once per frame, so
/// an onset falling just past this frame's horizon waits for the following frame and is already that
/// much late by the time it is queued; the horizon therefore has to include the frame period, not
/// just the device lead.
pub(crate) fn schedule_poll_interval_us(previous_us: i64, frame_us: i64) -> i64 {
    let decayed = previous_us.clamp(0, MAX_SCHEDULE_POLL_US) * SCHEDULE_POLL_DECAY_NUMERATOR / SCHEDULE_POLL_DECAY_DENOMINATOR;
    frame_us.max(decayed).clamp(MIN_SCHEDULE_POLL_US, MAX_SCHEDULE_POLL_US)
}

/// Position the sound scheduler books against: the device lead the engine reports plus this loop's
/// own polling interval. The manual analysis clock is virtual, has no device behind it, and must
/// stay on the position being scrubbed to.
pub(crate) fn schedule_position_us(song_us: i64, lookahead_us: i64, poll_interval_us: i64, manual: bool) -> i64 {
    if manual { song_us } else { song_us + lookahead_us + poll_interval_us }
}

/// Id namespaces released from the shared engine on a stage change. Entering Play drops both the
/// previous chart's bank and whatever the select preview left behind; leaving Play drops only the
/// chart, so the engine's memory stays bounded even though the stream itself never closes.
pub(crate) fn released_namespaces(entering_play: bool) -> &'static [IdNamespace] {
    if entering_play { &[IdNamespace::PREVIEW, IdNamespace::PLAY] } else { &[IdNamespace::PLAY] }
}

/// When a pending audio-settings change should be applied. The deadline is pushed back for as long
/// as the rows keep moving, so walking one row through several values reopens the stream once at the
/// end instead of once per step; once nothing is pending the armed deadline stands.
pub(crate) fn audio_reopen_deadline(pending: bool, armed: Option<Instant>, now: Instant) -> Option<Instant> {
    if pending { Some(now + AUDIO_REOPEN_DEBOUNCE) } else { armed }
}

/// Whether a chart owns the shared stream right now, in which case a reopen has to wait. Closing
/// the engine throws its keysound bank away: a new engine starts empty, the decode workers have
/// already reported the samples they handed to the old one, and the chart gain would fall back to
/// neutral. Loading counts too — that is exactly when the bank is being filled.
pub(crate) fn audio_reopen_blocked(stage: &Stage, chart_loaded: bool, keysounds_decoding: bool) -> bool {
    matches!(stage, Stage::Play | Stage::Loading) || chart_loaded || keysounds_decoding
}

/// The open attempts a reopen makes, in order: the newly requested settings, then the last ones
/// that actually worked, then the built-in defaults. Repeats are dropped so a configuration that
/// just failed is not retried.
pub(crate) fn reopen_attempts(requested: AudioOptions, last_ok: Option<AudioOptions>) -> Vec<AudioOptions> {
    let mut attempts = vec![requested];
    for candidate in last_ok.into_iter().chain(std::iter::once(AudioOptions::default())) {
        if !attempts.contains(&candidate) {
            attempts.push(candidate);
        }
    }
    attempts
}

impl App {
    /// Load the current `chart_path` and set up the player. Returns `false` (after logging) if the
    /// chart file can't be read, so callers can recover instead of panicking. Keysounds decode on
    /// background threads — a big chart has 650+ of them and serial decode froze the UI for seconds
    /// — so `frame` drains them behind the progress bar and `start_play` captures the song clock
    /// once they are all in. The shared output stream stays open across stages: entering Play only
    /// releases the namespaces the previous chart and the select preview held.
    pub(crate) fn load(&mut self) -> bool {
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

        self.audio_failed = false;
        self.ensure_audio();
        self.ks_rx = None;
        self.ks_total = 0;
        let chart_gain = rbms_chart::chart_gain(model.meta.volwav);
        self.chart_gain = chart_gain;
        if let Some(audio) = self.audio.as_mut() {
            for ns in released_namespaces(true) {
                audio.clear_namespace(*ns);
            }
            audio.set_chart_gain(chart_gain);
            let jobs = keysound_jobs(&model.wavmap, &dir);
            let total = jobs.len();
            println!("device {} Hz — decoding {total} keysounds (chart gain {chart_gain:.2})...", audio.out_rate());
            if total > 0 {
                let (rx, progress, _) = spawn_keysound_decode(jobs, self.ks_cancel.clone());
                self.ks_rx = Some(rx);
                self.ks_progress = progress;
                self.ks_total = total;
            }
        } else {
            println!("audio unavailable — visual only");
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

    /// Open the one output stream the whole app shares, unless it is already open or the last
    /// attempt failed. Play, Result and the select preview all use it, so nothing else opens or
    /// closes a cpal stream.
    pub(crate) fn ensure_audio(&mut self) {
        if self.audio.is_some() || self.audio_failed {
            return;
        }
        let options = self.audio_options();
        self.open_audio_with(&options);
    }

    fn open_audio_with(&mut self, options: &AudioOptions) {
        match AudioEngine::open(options) {
            Ok((audio, report)) => {
                println!("audio: {} — {} Hz, {} ch", report.device_name, report.sample_rate, report.channels);
                for note in &report.notes {
                    println!("audio: {note}");
                }
                self.audio_max_voices = options.max_voices;
                self.audio_opened_with = Some(options.clone());
                self.audio_report = Some(report);
                self.audio = Some(audio);
                self.audio_failed = false;
                self.audio_dead_at.set(None);
                self.apply_audio_gains();
                if let Some(engine) = self.audio.as_mut() {
                    engine.set_chart_gain(self.chart_gain);
                }
            }
            Err(e) => {
                eprintln!("audio unavailable ({e}) — visual only");
                self.audio_report = None;
                self.audio_failed = true;
            }
        }
    }

    /// Reopen the shared stream for changed audio settings, falling back to the settings that last
    /// worked and then to the defaults. The app keeps running silently if none of them open. Any
    /// preview is dropped first: its anchors are on the old stream's clock, and `update_preview`
    /// rebuilds it against the new one on the next frame.
    pub(crate) fn reopen_audio(&mut self) {
        self.reset_preview_playback();
        self.audio = None;
        self.audio_report = None;
        self.audio_dead_at.set(None);
        for options in reopen_attempts(self.audio_options(), self.audio_opened_with.clone()) {
            self.audio_failed = false;
            self.open_audio_with(&options);
            if self.audio.is_some() {
                return;
            }
            eprintln!("audio: reopen failed — trying the previous device settings");
        }
        self.audio_failed = true;
    }

    /// Apply a pending audio-settings change once it has rested for [`AUDIO_REOPEN_DEBOUNCE`].
    /// The deadline is pushed back on every frame the change is still moving, so walking a row
    /// through several values reopens the stream once at the end instead of once per step. Never
    /// while a chart owns the stream: the reopen waits until the player is back on the song list.
    pub(crate) fn poll_audio_reopen(&mut self, now: Instant) {
        self.audio_reopen_at = audio_reopen_deadline(self.audio_reopen_pending(), self.audio_reopen_at, now);
        self.clear_audio_reopen_pending();
        let Some(due) = self.audio_reopen_at else {
            return;
        };
        if now < due || audio_reopen_blocked(&self.stage, self.player.is_some(), self.ks_rx.is_some()) {
            return;
        }
        self.audio_reopen_at = None;
        self.reopen_audio();
    }

    /// Release everything the loaded chart owns in the shared stream without closing it: drop its
    /// keysound bank, silence what it left ringing, and restore the neutral chart gain.
    pub(crate) fn release_play_audio(&mut self) {
        self.song_us_last.set(0);
        self.chart_gain = rbms_chart::chart_gain(rbms_chart::VOLWAV_DEFAULT_PERCENT);
        if let Some(audio) = self.audio.as_mut() {
            for ns in released_namespaces(false) {
                audio.clear_namespace(*ns);
            }
            audio.set_chart_gain(rbms_chart::chart_gain(rbms_chart::VOLWAV_DEFAULT_PERCENT));
        }
    }

    /// Begin actual play: capture the song-clock anchor (so position starts at ~0) and enter the Play
    /// stage. Called once keysounds finish decoding (or immediately when there are none).
    pub(crate) fn start_play(&mut self) {
        self.clock = Instant::now();
        self.audio_dead_at.set(None);
        self.anchor_us = self.audio_clock_us();
        self.song_us_last.set(0);
        self.timing.clear();
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

    /// Where the shared stream is audible right now on the engine's own absolute axis, and the
    /// head start a scheduled sound needs, taken from one engine snapshot so the two cannot come
    /// from different callbacks. The position is interpolated between callbacks, so it does not
    /// step once per buffer. If the stream has died (device unplugged / driver error) the audio
    /// clock stops advancing, so it continues on the wall clock from the last position that clock
    /// reported and nothing is scheduled. Play and the select preview both rebase the position by
    /// their own anchor, which is why the fallback covers both.
    pub(crate) fn audio_clocks(&self) -> (i64, i64) {
        let now = Instant::now();
        let Some(audio) = self.audio.as_ref() else {
            return (self.clock.elapsed().as_micros() as i64, 0);
        };
        if audio.is_alive() {
            self.audio_dead_at.set(None);
            let clocks = audio.clocks(now);
            return (clocks.audible_us, clocks.lookahead_us);
        }
        let (last, at) = match self.audio_dead_at.get() {
            Some(v) => v,
            None => {
                let v = (audio.audible_us(now), now);
                self.audio_dead_at.set(Some(v));
                eprintln!("audio stream stopped — falling back to the wall clock");
                v
            }
        };
        (resumed_clock_us(last, now.saturating_duration_since(at).as_micros() as i64), 0)
    }

    pub(crate) fn audio_clock_us(&self) -> i64 {
        self.audio_clocks().0
    }

    /// The head start a scheduled sound needs so the mixer sees its command before the callback
    /// that has to render it: the device lead plus the buffer that callback renders. Zero without a
    /// live stream, where nothing is being scheduled anyway.
    pub(crate) fn audio_lookahead_us(&self) -> i64 {
        self.audio_clocks().1
    }

    /// Current song position on the audible axis — what the player is hearing — clamped so it can
    /// never step backwards, together with the engine's scheduling lead read from the same snapshot.
    /// Judgement, note rendering and replay recording use the position only; sound scheduling adds
    /// the lead and this loop's polling interval on top.
    pub(crate) fn song_and_lookahead_us(&self) -> (i64, i64) {
        let (audible, lookahead) = self.audio_clocks();
        let value = song_position_us(audible, self.anchor_us, self.song_us_last.get());
        self.song_us_last.set(value);
        (value, lookahead)
    }

    pub(crate) fn song_us(&self) -> i64 {
        self.song_and_lookahead_us().0
    }

    /// Record one input on both clocks so the overlay and the CSV dump can compare the interpolated
    /// reading against the quantised one, stamped with the wall clock it was taken at.
    pub(crate) fn push_timing_sample(&mut self, audible_us: i64, judge_delta_us: Option<i64>) {
        let Some(audio) = self.audio.as_ref() else {
            return;
        };
        let snapshot = audio.snapshot();
        let sample = TimingSample {
            wall_us: self.clock.elapsed().as_micros() as i64,
            input_at_us: audible_us,
            quantized_us: audio.clock_us() - self.anchor_us,
            judge_delta_us,
            frames_at_start: snapshot.frames_at_callback_start,
            buffer_frames: snapshot.buffer_frames,
        };
        self.timing.push(sample);
    }

    /// Feed the clock fit one reading per frame. The soak gate is the scatter of the interpolated
    /// clock around a straight line in wall-clock time, and an unattended autoplay run produces no
    /// key presses at all, so the readings cannot come from the input path.
    pub(crate) fn push_clock_sample(&mut self, song_us: i64) {
        let wall_us = self.clock.elapsed().as_micros() as i64;
        self.timing.push_clock(wall_us, song_us);
    }

    /// Dump every retained timing sample to the path from `--timing-csv` or `RBMS_TIMING_CSV`.
    pub(crate) fn dump_timing_csv(&self) {
        let configured = self.config.timing_csv.clone().or_else(|| std::env::var(TIMING_CSV_ENV).ok());
        let Some(path) = env_path(configured) else {
            return;
        };
        if self.timing.is_empty() {
            return;
        }
        match self.timing.write_csv(&path) {
            Ok(()) => println!("timing: {} samples -> {}", self.timing.len(), path.display()),
            Err(e) => eprintln!("timing csv write failed ({}): {e}", path.display()),
        }
    }

    /// Append one soak row, reporting a write failure only the first time it happens.
    fn write_soak_row(&mut self, now: Instant) {
        let stats = self.timing.stats();
        let mix = self.audio.as_ref().map(AudioEngine::mix_stats).unwrap_or_default();
        let snapshot = SoakSnapshot {
            stage: stage_name(&self.stage),
            rss_mb: self.ram_mb,
            active_voices: mix.active_voices,
            underruns: self.audio.as_ref().map(AudioEngine::underruns).unwrap_or(0),
            drops: self.audio.as_ref().map(AudioEngine::dropped_commands).unwrap_or(0),
            steals: mix.steals,
            hard_steals: mix.hard_steals,
            late: mix.late_schedules,
            ts_fallbacks: self.audio.as_ref().map(AudioEngine::timestamp_fallbacks).unwrap_or(0),
            retire_overflows: self.audio.as_ref().map(AudioEngine::retire_overflows).unwrap_or(0),
            interp_sd_us: stats.interp_residual_sd_us,
            judge_sd_us: stats.judge_delta_sd_us,
        };
        if let Err(e) = self.soak.write_row(now, &snapshot)
            && !self.soak_failed
        {
            self.soak_failed = true;
            eprintln!("soak log write failed: {e}");
        }
    }

    /// The audio and timing lines of the debug overlay. Built before the GPU is borrowed for
    /// rendering, so it can read the whole app state.
    fn debug_audio_lines(&self, anchor_us: i64) -> Vec<String> {
        let mut lines = Vec::new();
        if self.stage == Stage::Play {
            lines.push(format!("AUDIO {} US  ANCHOR {}  LOOKAHEAD {:.2} MS", self.audio_clock_us(), anchor_us, us_to_millis(self.audio_lookahead_us() as f64)));
        }
        if let Some(audio) = self.audio.as_ref() {
            let mix = audio.mix_stats();
            lines.push(format!(
                "STREAM {}  DROP {}  REALLOC {}  UNDERRUN~{}  RETIRE-OF {}",
                if audio.is_alive() { "ALIVE" } else { "DEAD" },
                audio.dropped_commands(),
                audio.scratch_reallocations(),
                audio.underruns(),
                audio.retire_overflows()
            ));
            lines.push(format!(
                "VOICES {}/{}  STEAL {}/{}  LATE {}  TS-FB {}",
                mix.active_voices,
                self.audio_max_voices,
                mix.steals,
                mix.hard_steals,
                mix.late_schedules,
                audio.timestamp_fallbacks()
            ));
        }
        if let Some(report) = self.audio_report.as_ref() {
            let name: String = report.device_name.chars().take(DEVICE_NAME_OVERLAY_CHARS).collect();
            lines.push(format!("DEVICE {name}  {} HZ  CH {}  FB {}", report.sample_rate, report.channels, report.fallback_step));
        }
        if self.stage == Stage::Play {
            let t = self.timing.stats();
            lines.push(format!(
                "JUDGE n={}  d(mean) {:+.2}MS  sd {:.2}MS  p95 {:.2}MS",
                t.n,
                us_to_millis(t.judge_delta_mean_us),
                us_to_millis(t.judge_delta_sd_us),
                us_to_millis(t.judge_delta_p95_abs_us as f64)
            ));
            lines.push(format!(
                "CLOCK n={}  fit sd {:.3}MS  quantised gap {:+.2}MS",
                t.clock_n,
                us_to_millis(t.interp_residual_sd_us),
                us_to_millis(t.interp_minus_quantized_mean_us)
            ));
        }
        lines
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
        self.dump_timing_csv();
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
        self.soak.record_frame(dt);
        if let Some(audio) = self.audio.as_mut() {
            audio.collect_retired();
        }
        self.schedule_poll_us = schedule_poll_interval_us(self.schedule_poll_us, (dt as f64 * 1_000_000.0) as i64);
        self.poll_network();
        self.poll_ir_jobs();
        self.poll_audio_reopen(now);
        if (self.config.debug || self.soak.enabled())
            && self.frame_count.is_multiple_of(RAM_SAMPLE_FRAMES)
            && let Some(u) = memory_stats::memory_stats()
        {
            self.ram_mb = u.physical_mem as f32 / (1024.0 * 1024.0);
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
        } else if self.preview_active() {
            self.stop_preview();
        }
        // In manual analysis the displayed song time comes from the virtual clock (pausable,
        // rate-scaled); otherwise it follows the real (audio) clock, and analysis mirrors it so a
        // first manual control resumes from the live position. Manual analysis mutes keysounds.
        let manual = self.stage == Stage::Play && self.analysis && self.analysis_manual;
        let (song, lookahead) = if manual {
            if !self.analysis_paused {
                self.analysis_us = (self.analysis_us + (dt as f64 * 1_000_000.0 * self.analysis_rate) as i64).max(0);
            }
            (self.analysis_us, 0)
        } else {
            let (s, lookahead) = self.song_and_lookahead_us();
            if self.stage == Stage::Play && self.analysis {
                self.analysis_us = s;
            }
            (s, lookahead)
        };
        let anchor = self.anchor_us;

        if self.stage == Stage::Play {
            self.push_clock_sample(song);
            if self.replay.is_some() {
                self.feed_replay(song, manual);
            }
            let sched = schedule_position_us(song, lookahead, self.schedule_poll_us, manual);
            if let (false, Some(audio), Some(player)) = (manual, self.audio.as_mut(), self.player.as_mut()) {
                player.update_schedule(sched, |e: PlayEvent| {
                    let bus = match e.source {
                        PlaySource::Bgm => Bus::Bg,
                        PlaySource::Key => Bus::Key,
                    };
                    audio.play_on(bus, e.wav.max(0) as u32, KEYSOUND_GAIN, KEYSOUND_PAN, KEYSOUND_PITCH, e.at_us + anchor);
                });
            } else if let Some(player) = self.player.as_mut() {
                player.update_schedule(sched, |_| {});
            }
            if let Some(player) = self.player.as_mut() {
                player.update_judge(song);
            }
            if let Some(player) = self.player.as_ref() {
                // Analysis stays on the field at the end (the player scrubs); only a non-analysis
                // run auto-advances to the result screen.
                if !self.analysis && player.judge.total_notes() > 0 && song > player.last_time_us() + 2_000_000 {
                    self.enter_result();
                }
            }
        }

        if self.soak.due(now) {
            self.write_soak_row(now);
        }
        let debug_audio_lines = if self.config.debug { self.debug_audio_lines(anchor) } else { Vec::new() };

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
                } else {
                    lines.push(format!("SEL {} / {}", self.sel + 1, self.select_items.len()));
                    lines.push(format!("SCORES {}  SONGS {}", self.scores.records.len(), self.songs.len()));
                    lines.push(format!("CURSOR {:.0} {:.0}", self.cursor.0, self.cursor.1));
                }
                lines.extend(debug_audio_lines);
                let (layouts, runs) = rbms_render::cache_stats();
                lines.push(format!("FONT {}/{}  RUNS {}/{}", layouts, rbms_render::LAYOUT_CACHE_LIMIT, runs, rbms_render::RUN_CACHE_LIMIT));
                let lh = 16.0;
                let ph = lines.len() as f32 * lh + 12.0;
                gpu.fill_rect(Rect::new(6.0, 6.0, DEBUG_PANEL_W, ph), Color { r: 0, g: 0, b: 0, a: 180 });
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

#[cfg(test)]
mod tests {
    use super::*;

    const BUFFER_US_512_AT_48K: i64 = 10_687;

    #[test]
    fn the_song_clock_is_the_audio_clock_rebased_on_the_anchor() {
        assert_eq!(song_position_us(1_500_000, 500_000, 0), 1_000_000);
        assert_eq!(song_position_us(500_000, 500_000, 0), 0);
    }

    #[test]
    fn the_song_clock_never_steps_backwards() {
        let anchor = 500_000;
        let first = song_position_us(1_000_000, anchor, 0);
        let second = song_position_us(999_000, anchor, first);
        assert_eq!(first, 500_000);
        assert_eq!(second, first, "a clock that reads earlier than the last reading is held");
        assert_eq!(song_position_us(1_200_000, anchor, second), 700_000);
    }

    #[test]
    fn an_audio_clock_behind_the_anchor_is_clamped_to_the_start() {
        assert_eq!(song_position_us(499_000, 500_000, 0), 0);
    }

    #[test]
    fn scheduling_leads_the_audible_axis_by_the_device_lead_and_the_frame_period() {
        let song = 4_000_000;
        let poll = 16_667;
        assert_eq!(schedule_position_us(song, BUFFER_US_512_AT_48K, poll, false), song + BUFFER_US_512_AT_48K + poll);
        assert!(
            schedule_position_us(song, BUFFER_US_512_AT_48K, poll, false) > song + BUFFER_US_512_AT_48K,
            "an onset booked one frame from now must still clear the device lead"
        );
    }

    #[test]
    fn the_manual_analysis_clock_gets_no_lookahead() {
        assert_eq!(schedule_position_us(4_000_000, BUFFER_US_512_AT_48K, 16_667, true), 4_000_000);
    }

    #[test]
    fn a_dead_stream_reports_no_lookahead_so_only_the_poll_interval_remains() {
        assert_eq!(schedule_position_us(4_000_000, 0, 0, false), 4_000_000);
    }

    #[test]
    fn the_poll_horizon_holds_a_frame_spike_and_then_decays() {
        assert_eq!(schedule_poll_interval_us(0, 8_000), 8_000);
        assert_eq!(schedule_poll_interval_us(0, 0), MIN_SCHEDULE_POLL_US, "a zero frame time still leaves a horizon");
        assert_eq!(schedule_poll_interval_us(0, 10_000_000), MAX_SCHEDULE_POLL_US, "a stall does not book a second of sound early");

        let spike = schedule_poll_interval_us(8_000, 40_000);
        assert_eq!(spike, 40_000);
        let next = schedule_poll_interval_us(spike, 8_000);
        assert!(next < spike && next > 8_000, "the spike decays instead of vanishing or sticking, got {next}");

        let mut held = spike;
        for _ in 0..200 {
            held = schedule_poll_interval_us(held, 8_000);
        }
        assert_eq!(held, 8_000, "a settled frame time wins in the end");
    }

    #[test]
    fn a_row_that_keeps_moving_pushes_the_reopen_back_instead_of_firing_mid_edit() {
        let start = Instant::now();
        let first = audio_reopen_deadline(true, None, start).expect("a change arms the debounce");
        assert_eq!(first, start + AUDIO_REOPEN_DEBOUNCE);

        let later = start + AUDIO_REOPEN_DEBOUNCE / 2;
        let second = audio_reopen_deadline(true, Some(first), later).expect("a second change rearms it");
        assert!(second > first, "the deadline must follow the last change, not the first");
        assert_eq!(second, later + AUDIO_REOPEN_DEBOUNCE);

        let settled = later + Duration::from_millis(1);
        assert_eq!(audio_reopen_deadline(false, Some(second), settled), Some(second), "an idle frame leaves the deadline alone");
        assert_eq!(audio_reopen_deadline(false, None, settled), None, "no change, nothing armed");
    }

    #[test]
    fn reopening_the_stream_waits_while_a_chart_owns_it() {
        for (stage, blocked) in [(Stage::Play, true), (Stage::Loading, true), (Stage::Select, false), (Stage::Settings, false), (Stage::Result, false)] {
            assert_eq!(audio_reopen_blocked(&stage, false, false), blocked, "stage {stage:?}");
        }
        assert!(audio_reopen_blocked(&Stage::Select, true, false), "a chart is still loaded");
        assert!(audio_reopen_blocked(&Stage::Select, false, true), "keysounds are still decoding");
    }

    #[test]
    fn entering_play_releases_the_preview_and_the_previous_chart() {
        assert_eq!(released_namespaces(true), &[IdNamespace::PREVIEW, IdNamespace::PLAY]);
    }

    #[test]
    fn leaving_play_releases_only_the_chart_so_a_preview_can_keep_sounding() {
        assert_eq!(released_namespaces(false), &[IdNamespace::PLAY]);
    }

    #[test]
    fn the_released_namespaces_do_not_overlap() {
        for id in [IdNamespace::PLAY.base, IdNamespace::PLAY.base + IdNamespace::PLAY.len - 1] {
            assert!(IdNamespace::PLAY.contains(id));
            assert!(!IdNamespace::PREVIEW.contains(id));
        }
        for id in [IdNamespace::PREVIEW.base, IdNamespace::PREVIEW.base + IdNamespace::PREVIEW.len - 1] {
            assert!(IdNamespace::PREVIEW.contains(id));
            assert!(!IdNamespace::PLAY.contains(id));
        }
    }

    #[test]
    fn a_reopen_falls_back_to_the_last_working_settings_then_the_defaults() {
        let requested = AudioOptions { device_name: Some("Studio".into()), sample_rate: Some(96_000), buffer_frames: Some(128), max_voices: 256 };
        let last_ok = AudioOptions { device_name: Some("Built-in".into()), sample_rate: None, buffer_frames: None, max_voices: 512 };
        let attempts = reopen_attempts(requested.clone(), Some(last_ok.clone()));
        assert_eq!(attempts, vec![requested, last_ok, AudioOptions::default()]);
    }

    #[test]
    fn a_reopen_never_retries_the_same_settings_twice() {
        assert_eq!(reopen_attempts(AudioOptions::default(), Some(AudioOptions::default())), vec![AudioOptions::default()]);
        let requested = AudioOptions { buffer_frames: Some(256), ..AudioOptions::default() };
        assert_eq!(reopen_attempts(requested.clone(), None), vec![requested, AudioOptions::default()]);
    }

    #[test]
    fn every_stage_has_a_distinct_soak_label() {
        let stages = [Stage::Select, Stage::Settings, Stage::KeyConfig, Stage::Tables, Stage::Folders, Stage::Loading, Stage::Play, Stage::Result];
        let mut names: Vec<&str> = stages.iter().map(stage_name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
        assert_eq!(stage_name(&Stage::Play), "Play");
    }
}

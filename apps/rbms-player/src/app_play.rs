//! `AppShared` methods behind the play path: loading a chart, the shared output stream, the song
//! clock and the timing instrumentation. The PLAY screen itself lives in `stage::play`.
#![allow(clippy::wildcard_imports)]
use crate::assets::{bga_jobs, spawn_bga_decode};
use crate::stage::loading::BgaLoad;
use crate::stage::{KeysoundLoad, LoadingState, PlayState, Stage, StageId, Transition};
use crate::*;

/// How much of the output device's name the debug overlay shows before it is cut.
const DEVICE_NAME_OVERLAY_CHARS: usize = 20;

/// A parsed chart, ready to play as soon as the files it names have been decoded.
///
/// The background images are not part of it yet: they are decoded on the worker pool like the
/// keysounds are, and the screen is built once they have arrived. Holding the session apart from
/// them is what lets the decode happen off the frame loop at all.
pub(crate) struct PendingChart {
    session: PlaySession,
    lntype: i32,
    ln_mode_key: String,
}

impl PendingChart {
    /// Turn the parsed chart into the screen that plays it, now that its images are in.
    pub(crate) fn into_play(self, bga: std::collections::HashMap<i32, Vec<u8>>) -> PlayState {
        PlayState::new(self.session, bga, self.lntype, self.ln_mode_key)
    }
}

/// A chart that has just been parsed, plus whatever is still being decoded for it. Either decode is
/// `None` when there was nothing of that kind to wait for.
pub(crate) struct LoadedChart {
    pub(crate) chart: PendingChart,
    pub(crate) bga: Option<BgaLoad>,
    pub(crate) keysounds: Option<KeysoundLoad>,
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

/// Screens that hold the loaded chart's keysound bank in the shared stream.
///
/// LOADING is where the bank is filled, PLAY sounds from it, and RESULT still holds it — the bank
/// is only handed back by [`AppShared::release_play_audio`], which runs when the run is left, not
/// when the result screen opens.
pub(crate) fn stage_owns_chart_audio(stage: StageId) -> bool {
    matches!(stage, StageId::Play | StageId::Loading | StageId::Result)
}

/// Whether a chart owns the shared stream right now, in which case a reopen has to wait. Closing
/// the engine throws its keysound bank away: a new engine starts empty, the decode workers have
/// already reported the samples they handed to the old one, and the chart gain would fall back to
/// neutral.
pub(crate) fn audio_reopen_blocked(stage: StageId, keysounds_decoding: bool) -> bool {
    stage_owns_chart_audio(stage) || keysounds_decoding
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

impl AppShared {
    /// Load the current `chart_path` and set up the player. Returns `false` (after logging) if the
    /// chart file can't be read, so callers can recover instead of panicking. Keysounds decode on
    /// background threads — a big chart has 650+ of them and serial decode froze the UI for seconds
    /// — so `frame` drains them behind the progress bar and `start_play` captures the song clock
    /// once they are all in. The shared output stream stays open across stages: entering Play only
    /// releases the namespaces the previous chart and the select preview held.
    pub(crate) fn load(&mut self) -> Option<LoadedChart> {
        let bytes = match std::fs::read(&self.chart_path) {
            Ok(b) => b,
            Err(e) => {
                notify(Level::Error, format!("chart not found: {} ({e})", self.chart_path));
                return None;
            }
        };
        let src = rbms_parser::parse_with(&bytes, Default::default());
        let mode = rbms_chart::detect_mode(&src, &self.chart_path);
        let (random, seed) = match &self.replay {
            Some(rp) => {
                if !rp.md5.is_empty() && rp.md5 != src.md5 {
                    notify(Level::Warn, format!("warning: chart md5 mismatch (replay {} vs file {}); replay may desync", rp.md5, src.md5));
                }
                self.config.judge.offset_ms = rp.offset_ms;
                self.config.play.scratch_auto = rp.scratch_auto;
                self.config.play.gauge = gauge_from_name(&rp.gauge);
                let random = NoteOption::from_str(&rp.random);
                self.config.play.random = random;
                (random, rp.seed)
            }
            None => (self.config.play.random, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)),
        };
        let judge_setup = run_judge_setup(&self.config, self.replay.as_ref());
        let mut model = to_model(&src, mode);
        let ln_mode_decides_flavour = rbms_chart::contains_undefined_long_note(&model);
        rbms_chart::resolve_long_note_flavour(&mut model, judge_setup.ln_mode.resolve());
        let lntype = run_lntype(src.headers.lnmode, judge_setup.ln_mode);
        rbms_chart::shuffle::apply(&mut model, random, seed);
        if model.meta.total <= 0.0 {
            model.meta.total = default_total_for_mode(&mode, rbms_chart::count_playable_notes(&model));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.audio_dead_at.set(None);
        if self.config.play.total_override > 0.0 {
            model.meta.total = self.config.play.total_override;
        }
        self.mode = mode;
        self.active_keys = self.launch.keys_override.clone().unwrap_or_else(|| self.keyconfig.lane_keys(mode));
        self.active_reverse_keys = match self.launch.keys_override {
            Some(_) => Vec::new(),
            None => self.keyconfig.scratch_reverse_keys(mode),
        };
        self.skin_cfg = match &self.launch.skin_path {
            Some(p) => SkinConfig::load(p).unwrap_or_else(|e| {
                notify(Level::Warn, format!("skin load failed ({e}), using bundled"));
                bundled_skin(&self.config.display.skin)
            }),
            None => bundled_skin(&self.config.display.skin),
        };
        self.rebuild_skin();
        let dir = Path::new(&self.chart_path).parent().unwrap_or(Path::new(".")).to_path_buf();

        self.audio_failed = false;
        self.ensure_audio();
        let mut keysounds: Option<KeysoundLoad> = None;
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
                let (rx, progress, _) = spawn_keysound_decode(jobs, cancel.clone());
                keysounds = Some(KeysoundLoad { rx, progress, cancel, total });
            }
        } else {
            println!("audio unavailable — visual only");
        }

        let status = if self.config.play.autoplay {
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
        let bga_cancel = Arc::new(AtomicBool::new(false));
        let mut bga: Option<BgaLoad> = None;
        if self.config.display.bga && self.skin.bga.is_some() {
            let jobs = bga_jobs(&model.bgamap, &dir);
            let total = jobs.len();
            if total > 0 {
                println!("decoding {total} BGA images...");
                let (rx, progress, _) = spawn_bga_decode(jobs, bga_cancel.clone());
                bga = Some(BgaLoad { rx, progress, cancel: bga_cancel, total });
            }
        }

        let auto_lanes: Vec<bool> = if self.config.play.scratch_auto { (0..mode.key).map(|lane| mode.is_scratch(lane)).collect() } else { Vec::new() };
        let options = SessionOptions {
            autoplay: self.config.play.autoplay && self.replay.is_none(),
            gauge: self.config.play.gauge,
            judge_offset_us: self.offset_us(),
            auto_lanes,
            seed,
            analysis: self.replay.is_some() && self.config.display.replay_analysis,
            auto_calibration: self.config.judge.auto_offset,
            replay: self.replay.clone(),
            ..SessionOptions::default()
        };
        let mut session = PlaySession::new(model, options);
        session.set_judge_setup(judge_setup);
        let ln_mode_key = match ln_mode_decides_flavour {
            true => ln_mode_token(judge_setup.ln_mode).to_string(),
            false => SCORE_LN_MODE_FROM_CHART.to_string(),
        };
        Some(LoadedChart { chart: PendingChart { session, lntype, ln_mode_key }, bga, keysounds })
    }

    /// Enter a chart that has just been parsed: keep the LOADING screen up while its files decode,
    /// or start play right away when there is nothing left to wait for.
    pub(crate) fn enter_loaded_chart(&mut self, loaded: LoadedChart) -> Stage {
        if loaded.bga.is_none() && loaded.keysounds.is_none() {
            self.start_play();
            return Stage::Play(Box::new(loaded.chart.into_play(std::collections::HashMap::new())));
        }
        Stage::Loading(LoadingState::assets(loaded))
    }

    /// Leave a run: back to the song browser, or out of the app when there is no library to return
    /// to (a chart or replay launched straight from the command line).
    pub(crate) fn leave_play(&mut self) -> Transition {
        if self.library.is_empty() {
            return Transition::Quit;
        }
        self.release_play_audio();
        self.replay = None;
        self.rebuild_select_items();
        self.print_selection();
        Transition::Back
    }

    /// Hit-test the last frame's clickable regions against a cursor position. Regions are tested
    /// topmost-first (later pushes draw on top).
    pub(crate) fn hit_test(&self, at: (f32, f32)) -> Option<Hot> {
        let (cx, cy) = at;
        self.hot.iter().rev().find(|(r, _)| cx >= r.x && cx <= r.x + r.w && cy >= r.y && cy <= r.y + r.h).map(|(_, h)| *h)
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
                notify(Level::Warn, format!("audio unavailable ({e}) — visual only"));
                self.audio_report = None;
                self.audio_failed = true;
            }
        }
    }

    /// Reopen the shared stream for changed audio settings, falling back to the settings that last
    /// worked and then to the defaults. The app keeps running silently if none of them open.
    /// Dropping the old engine takes the select preview's samples and anchors with it; the browser
    /// rebuilds them against the new stream the next time it is up.
    pub(crate) fn reopen_audio(&mut self) {
        self.audio = None;
        self.audio_report = None;
        self.audio_dead_at.set(None);
        for options in reopen_attempts(self.audio_options(), self.audio_opened_with.clone()) {
            self.audio_failed = false;
            self.open_audio_with(&options);
            if self.audio.is_some() {
                return;
            }
            notify(Level::Warn, "audio: reopen failed — trying the previous device settings");
        }
        self.audio_failed = true;
    }

    /// Apply a pending audio-settings change once it has rested for [`AUDIO_REOPEN_DEBOUNCE`].
    /// The deadline is pushed back on every frame the change is still moving, so walking a row
    /// through several values reopens the stream once at the end instead of once per step. Never
    /// while a chart owns the stream: the reopen waits until the player is back on the song list.
    pub(crate) fn poll_audio_reopen(&mut self, now: Instant, stage: StageId, keysounds_decoding: bool) {
        self.audio_reopen_at = audio_reopen_deadline(self.audio_reopen_pending(), self.audio_reopen_at, now);
        self.clear_audio_reopen_pending();
        let Some(due) = self.audio_reopen_at else {
            return;
        };
        if now < due || audio_reopen_blocked(stage, keysounds_decoding) {
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

    /// Begin actual play: capture the song-clock anchor, so the position starts at ~0. Called once
    /// keysounds finish decoding (or immediately when there are none).
    pub(crate) fn start_play(&mut self) {
        self.clock = Instant::now();
        self.audio_dead_at.set(None);
        self.anchor_us = self.audio_clock_us();
        self.song_us_last.set(0);
        self.timing.clear();
    }

    /// Read the audio clock once per frame outside play, so a stream that dies while the player is
    /// sitting on a menu is noticed there rather than only when the next chart starts.
    pub(crate) fn poll_audio_clock(&self) {
        let _ = self.song_and_lookahead_us();
    }

    /// The browser lines of the debug overlay, shown on every screen but PLAY.
    pub(crate) fn browser_debug_lines(&self) -> Vec<String> {
        vec![
            format!("SEL {} / {}", self.sel + 1, self.select_items.len()),
            format!("SCORES {}  SONGS {}", self.scores.records().len(), self.library.len()),
            format!("CURSOR {:.0} {:.0}", self.cursor.0, self.cursor.1),
        ]
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
                notify(Level::Warn, "audio stream stopped — falling back to the wall clock");
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
        let configured = self.launch.timing_csv.clone().or_else(|| std::env::var(TIMING_CSV_ENV).ok());
        let Some(path) = env_path(configured) else {
            return;
        };
        if self.timing.is_empty() {
            return;
        }
        match self.timing.write_csv(&path) {
            Ok(()) => println!("timing: {} samples -> {}", self.timing.len(), path.display()),
            Err(e) => notify(Level::Error, format!("timing csv write failed ({}): {e}", path.display())),
        }
    }

    /// Append one soak row, reporting a write failure only the first time it happens.
    pub(crate) fn write_soak_row(&mut self, now: Instant, stage: StageId) {
        let stats = self.timing.stats();
        let mix = self.audio.as_ref().map(AudioEngine::mix_stats).unwrap_or_default();
        let snapshot = SoakSnapshot {
            stage: stage.label(),
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
            notify(Level::Error, format!("soak log write failed: {e}"));
        }
    }

    /// The audio and timing lines of the debug overlay. Built before the GPU is borrowed for
    /// rendering, so it can read the whole app state.
    pub(crate) fn debug_audio_lines(&self, anchor_us: i64, in_play: bool) -> Vec<String> {
        let mut lines = Vec::new();
        if in_play {
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
        if in_play {
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

    pub(crate) fn print_selection(&self) {
        let total = self.select_items.len();
        match self.select_items.get(self.sel) {
            Some(SelectItem::Song(i)) => {
                if let Some(e) = self.library.songs().get(*i) {
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

/// A parsed chart with nothing in it, for the screens that have to hold one while its files are
/// still being decoded.
#[cfg(test)]
pub(crate) fn pending_chart_for_tests() -> PendingChart {
    let src = rbms_parser::parse_with(b"#PLAYER 1\n#TITLE pending\n#BPM 120\n", Default::default());
    let mode = rbms_chart::detect_mode(&src, "pending.bms");
    let model = to_model(&src, mode);
    PendingChart { session: PlaySession::new(model, SessionOptions::default()), lntype: 0, ln_mode_key: SCORE_LN_MODE_FROM_CHART.to_string() }
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
        for (stage, blocked) in [(StageId::Play, true), (StageId::Loading, true), (StageId::Result, true), (StageId::Select, false), (StageId::Settings, false)]
        {
            assert_eq!(audio_reopen_blocked(stage, false), blocked, "stage {stage:?}");
        }
        assert!(audio_reopen_blocked(StageId::Select, true), "keysounds are still decoding");
    }

    #[test]
    fn the_result_screen_still_owns_the_chart_bank_so_a_reopen_waits_for_it() {
        assert!(stage_owns_chart_audio(StageId::Result), "the bank is only handed back when the run is left");
        assert!(audio_reopen_blocked(StageId::Result, false), "reopening here would throw the chart's keysounds away");
        for stage in StageId::ALL {
            assert_eq!(audio_reopen_blocked(stage, false), stage_owns_chart_audio(stage), "stage {stage:?}");
        }
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
        let mut names: Vec<&str> = StageId::ALL.iter().map(|s| s.label()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
        assert_eq!(StageId::Play.label(), "Play");
    }
}

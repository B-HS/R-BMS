//! The song browser's hover preview: which chart is sounding, how a newly focused row becomes the
//! one to load, and how the two kinds of preview — a `#PREVIEW` clip, or an autoplay pass over the
//! chart itself — are started, looped and torn down.
//!
//! Preview sounds live in their own id namespace inside the shared output engine, so a chart load
//! never disturbs them and leaving the browser clears exactly what the preview owned.
#![allow(clippy::wildcard_imports)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

#[cfg(test)]
use rbms_config::DEFAULT_PREVIEW_FADE_MS;

use crate::app_play::schedule_position_us;
use crate::stage::select::SelectState;
use crate::*;

/// Where the preview's level is on its way in or out.
///
/// A voice's gain is fixed when the sound is booked, so a clip already playing cannot be faded by
/// changing it. The background bus can be, and while the browser is up the preview is the only
/// thing on that bus — so the fade is a ramp on the bus, put back where the settings file has it as
/// soon as the preview is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fade {
    /// Nothing is sounding and the background bus is the settings file's to set.
    Idle,
    In,
    Out,
}

/// The hover preview's playback state: which chart is sounding, the debounce that decides when a
/// newly focused chart becomes the one to load, and the autoplay-preview timeline being replayed.
pub(super) struct PreviewState {
    /// The chart currently sounding, once its load has finished.
    si: Option<usize>,
    /// The chart the focus has moved to, and when it arrived there.
    target: Option<usize>,
    target_at: Instant,
    /// File-preview loop: the clip length and when the next repeat is booked for.
    loop_us: i64,
    next_us: i64,
    /// Autoplay-preview keysound timeline `(at_us, wav)` for the focused chart when it defines no
    /// `#PREVIEW` file — extracted once from a throwaway autoplay `Player`, then replayed against
    /// the engine clock and looped. Empty in file-preview mode.
    sched: Vec<(i64, u32)>,
    cursor: usize,
    anchor: i64,
    start_us: i64,
    end_us: i64,
    /// Background autoplay-preview load channel: the throwaway-`Player` schedule, then decoded
    /// keysounds; `None` once the load finishes (channel disconnected) and playback has begun.
    prep_rx: Option<Receiver<PreviewMsg>>,
    /// Cooperative cancel for the in-flight autoplay-preview load (parse + decode workers), flipped
    /// when the focus moves on so abandoned work stops instead of running to completion.
    cancel: std::sync::Arc<AtomicBool>,
    /// Which way the preview's level is moving, and when it started moving that way.
    fade: Fade,
    fade_at: Instant,
}

impl Default for PreviewState {
    fn default() -> PreviewState {
        PreviewState {
            si: None,
            target: None,
            target_at: Instant::now(),
            loop_us: 0,
            next_us: 0,
            sched: Vec::new(),
            cursor: 0,
            anchor: 0,
            start_us: 0,
            end_us: 0,
            prep_rx: None,
            cancel: std::sync::Arc::new(AtomicBool::new(false)),
            fade: Fade::Idle,
            fade_at: Instant::now(),
        }
    }
}

/// How far through a fade of `span` the preview is, as a level between 0 and 1. A fade of no length
/// is already over, which is what a `PREVIEW FADE` of zero asks for.
fn fade_level(fade: Fade, elapsed: Duration, span: Duration) -> f32 {
    let done = if span.is_zero() { 1.0 } else { (elapsed.as_secs_f32() / span.as_secs_f32()).clamp(0.0, 1.0) };
    match fade {
        Fade::Idle => 0.0,
        Fade::In => done,
        Fade::Out => 1.0 - done,
    }
}

impl SelectState {
    /// Whether a select preview is loading or sounding. The shared output stream is not evidence of
    /// that any more, since it stays open for the whole app lifetime.
    pub(super) fn preview_active(&self) -> bool {
        self.preview.si.is_some() || self.preview.prep_rx.is_some()
    }

    /// Sample id of preview keysound `wav` inside the preview namespace, clamped so a chart with an
    /// absurd `#WAV` index cannot spill into a neighbouring namespace.
    pub(crate) fn preview_sample_id(wav: u32) -> u32 {
        IdNamespace::PREVIEW.base + wav.min(IdNamespace::PREVIEW.len - 1)
    }

    /// The clock the preview books sounds against: the audible position plus the engine's device
    /// lead and this loop's polling interval, on the shared engine's absolute axis. Shares the
    /// dead-stream wall-clock fallback with play, so a device that disappears stops the preview
    /// freezing instead of leaving it silent forever.
    pub(super) fn preview_sched_us(shared: &AppShared) -> i64 {
        let (audible, lookahead) = shared.audio_clocks();
        schedule_position_us(audible, lookahead, shared.schedule_poll_us, false)
    }

    /// Drop the preview's samples and silence its voices, leaving the shared stream open.
    pub(super) fn clear_preview_audio(shared: &mut AppShared) {
        if let Some(audio) = shared.audio.as_mut() {
            audio.clear_namespace(IdNamespace::PREVIEW);
        }
    }

    /// How long a preview takes to fade in, and to fade back out again.
    fn fade_span(shared: &AppShared) -> Duration {
        Duration::from_millis(u64::from(shared.config.library.preview_fade_ms))
    }

    /// Begin fading the preview out, leaving it sounding until the fade is over. The focus moving
    /// on is what asks for this, so scrolling past a row lets go of its preview instead of cutting
    /// it off mid-note.
    fn begin_fade_out(&mut self, now: Instant) {
        if self.preview.fade == Fade::Out {
            return;
        }
        self.preview.fade = Fade::Out;
        self.preview.fade_at = now;
    }

    /// Begin fading the preview in from silence, which is what a chart that has just begun playing
    /// asks for.
    fn begin_fade_in(&mut self, now: Instant) {
        self.preview.fade = Fade::In;
        self.preview.fade_at = now;
    }

    /// Move the fade on by one frame: push the preview's level onto the background bus, and once a
    /// fade-out has run its course tear the preview down — which is what lets the chart the focus
    /// has moved to become the one that plays.
    fn pump_fade(&mut self, shared: &mut AppShared, now: Instant) {
        if self.preview.fade == Fade::Idle {
            return;
        }
        let span = SelectState::fade_span(shared);
        let elapsed = now.saturating_duration_since(self.preview.fade_at);
        if self.preview.fade == Fade::Out && elapsed >= span {
            self.reset_preview_playback(shared);
            return;
        }
        let level = fade_level(self.preview.fade, elapsed, span);
        let bg = shared.config.audio.bg;
        if let Some(audio) = shared.audio.as_mut() {
            audio.set_bus_gain(Bus::Bg, bg * level);
        }
    }

    /// Put the background bus back where the settings file has it, which is what a preview that is
    /// no longer sounding owes every other screen.
    fn release_bus(shared: &mut AppShared) {
        let bg = shared.config.audio.bg;
        if let Some(audio) = shared.audio.as_mut() {
            audio.set_bus_gain(Bus::Bg, bg);
        }
    }

    /// Drive the song-select hover preview once the focus settles (debounced so fast scrolling
    /// doesn't load every row): if the chart defines a `#PREVIEW` clip, decode + loop that file;
    /// otherwise build an autoplay preview of the chart itself (background-decode its keysounds, then
    /// replay the autoplay timeline, looped). Preview sounds live in their own id namespace inside
    /// the shared engine, which is cleared per settled chart so the bank stays clean. Autoplay
    /// events are booked before the end-of-loop check, so the final one is never skipped; the loop
    /// then restarts by re-anchoring once the silent tail has elapsed.
    ///
    /// An autoplay load drains its background channel here — the schedule first, then the decoded
    /// keysounds; the channel disconnecting is what says the load is done, and the clock is anchored
    /// so playback begins at the first event. A `#PREVIEW` clip instead loops by re-triggering at
    /// each clip boundary: the mixer stops the same key on a new play, so booking the next clip
    /// ahead would cut the current one short.
    pub(super) fn update_preview(&mut self, shared: &mut AppShared, now: Instant) {
        if !shared.config.library.preview {
            if self.preview_active() {
                self.stop_preview(shared);
            }
            return;
        }
        let cur = shared.focused_song_index();
        if cur != self.preview.target {
            self.preview.target = cur;
            self.preview.target_at = now;
            if self.preview.si.is_some() {
                self.begin_fade_out(now);
            }
        }
        self.pump_fade(shared, now);
        if self.preview.si != cur {
            if let Some(si) = cur
                && self.preview.fade != Fade::Out
                && self.preview.target_at.elapsed() >= PREVIEW_DEBOUNCE
            {
                self.start_preview(shared, si);
            }
            return;
        }
        if self.preview.prep_rx.is_some() {
            let mut done = false;
            loop {
                match self.preview.prep_rx.as_ref().unwrap().try_recv() {
                    Ok(PreviewMsg::Clip(dec)) => self.begin_clip_playback(shared, dec, now),
                    Ok(PreviewMsg::Schedule { sched, start_us, end_us }) => {
                        self.preview.sched = sched;
                        self.preview.start_us = start_us;
                        self.preview.end_us = end_us;
                        self.preview.cursor = 0;
                    }
                    Ok(PreviewMsg::Keysound(id, dec)) => {
                        if let Some(eng) = shared.audio.as_mut() {
                            eng.insert_decoded(SelectState::preview_sample_id(id), dec);
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
            if done {
                self.preview.prep_rx = None;
                if !self.preview.sched.is_empty() {
                    self.preview.anchor = SelectState::preview_sched_us(shared) - self.preview.start_us;
                    self.preview.cursor = 0;
                    self.begin_fade_in(now);
                } else if self.preview.loop_us == 0 {
                    SelectState::clear_preview_audio(shared);
                }
            }
            return;
        }
        if !self.preview.sched.is_empty() {
            let song = SelectState::preview_sched_us(shared) - self.preview.anchor;
            let volume = shared.config.library.preview_volume;
            if let Some(eng) = shared.audio.as_mut() {
                while self.preview.cursor < self.preview.sched.len() && self.preview.sched[self.preview.cursor].0 <= song {
                    let (at, wav) = self.preview.sched[self.preview.cursor];
                    eng.play_on(Bus::Bg, SelectState::preview_sample_id(wav), volume, KEYSOUND_PAN, KEYSOUND_PITCH, at + self.preview.anchor);
                    self.preview.cursor += 1;
                }
            }
            if song >= self.preview.end_us {
                self.preview.anchor += self.preview.end_us - self.preview.start_us;
                self.preview.cursor = 0;
            }
            return;
        }
        if self.preview.loop_us > 0 {
            let sched = SelectState::preview_sched_us(shared);
            let volume = shared.config.library.preview_volume;
            if let Some(eng) = shared.audio.as_mut() {
                while sched >= self.preview.next_us {
                    eng.play_on(Bus::Bg, PREVIEW_ID, volume, KEYSOUND_PAN, KEYSOUND_PITCH, self.preview.next_us);
                    self.preview.next_us += self.preview.loop_us;
                }
            }
        }
    }

    /// Begin the preview for the focused chart: its `#PREVIEW` clip if defined, otherwise an autoplay
    /// preview of the chart. Marks the song as settled even when nothing can be played, so the
    /// debounce doesn't retry every frame.
    pub(super) fn start_preview(&mut self, shared: &mut AppShared, si: usize) {
        let dbg = shared.config.display.debug;
        self.preview.si = Some(si);
        self.preview.loop_us = 0;
        self.preview.sched.clear();
        self.preview.cursor = 0;
        let Some(has_file) = shared.library.songs().get(si).map(|e| !e.preview.trim().is_empty()) else {
            if dbg {
                notify(Level::Warn, format!("[preview] song index {si} out of range"));
            }
            return;
        };
        if !has_file {
            self.start_autoplay_preview(shared, si, dbg);
            return;
        }
        let e = &shared.library.songs()[si];
        let title = e.title.clone();
        let Some(dir) = e.path.parent() else {
            if dbg {
                notify(Level::Warn, format!("[preview] no parent dir for {}", e.path.display()));
            }
            return;
        };
        let Some((path, _)) = resolve_file(dir, &e.preview, &["wav", "ogg", "flac", "mp3"]) else {
            if dbg {
                notify(Level::Warn, format!("[preview] #PREVIEW '{}' not found under {}", e.preview, dir.display()));
            }
            return;
        };
        let ext = path.extension().and_then(|x| x.to_str()).map(str::to_owned);
        shared.ensure_audio();
        if shared.audio.is_none() {
            if dbg {
                notify(Level::Warn, "[preview] no output stream — preview skipped");
            }
            return;
        }
        SelectState::clear_preview_audio(shared);
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        self.preview.cancel = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel::<PreviewMsg>();
        self.preview.prep_rx = Some(rx);
        if dbg {
            notify(Level::Info, format!("[preview] '{title}' clip: reading {} in background", path.display()));
        }
        std::thread::spawn(move || {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(err) => {
                    notify(Level::Warn, format!("[preview] read failed {}: {err}", path.display()));
                    return;
                }
            };
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            match rbms_audio::decode_bytes(bytes, ext.as_deref()) {
                Ok(dec) => {
                    let _ = tx.send(PreviewMsg::Clip(dec));
                }
                Err(err) => notify(Level::Warn, format!("[preview] decode failed {}: {err}", path.display())),
            }
        });
    }

    /// Start the `#PREVIEW` clip that has just finished decoding, and set the repeat it loops on.
    ///
    /// The clip re-triggers at each boundary rather than being booked ahead: the mixer stops the
    /// same key on a new play, so a queued repeat would cut the one sounding short.
    fn begin_clip_playback(&mut self, shared: &mut AppShared, dec: rbms_audio::DecodedAudio, now: Instant) {
        let start_at = SelectState::preview_sched_us(shared);
        let volume = shared.config.library.preview_volume;
        let Some(eng) = shared.audio.as_mut() else {
            return;
        };
        eng.insert_decoded(PREVIEW_ID, dec);
        let dur = eng.sample_duration_us(PREVIEW_ID).unwrap_or(0);
        eng.play_on(Bus::Bg, PREVIEW_ID, volume, KEYSOUND_PAN, KEYSOUND_PITCH, start_at);
        self.preview.loop_us = dur;
        self.preview.next_us = start_at + dur.max(PREVIEW_MIN_LOOP_US);
        self.begin_fade_in(now);
    }

    /// Start an autoplay preview for a chart with no `#PREVIEW` file. All heavy work — parse, autoplay
    /// schedule extraction, and keysound decode — runs on a background coordinator thread so the
    /// select screen never stutters on focus-settle; `update_preview` drains the results and begins
    /// playback once the load finishes. A fresh cancel flag lets a later focus abandon this load.
    ///
    /// The keysound timeline comes from a one-shot autoplay pass whose judging is discarded, and
    /// the decode reuses the shared worker pool, forwarding each sample to the preview channel.
    pub(super) fn start_autoplay_preview(&mut self, shared: &mut AppShared, si: usize, dbg: bool) {
        let Some(e) = shared.library.songs().get(si) else { return };
        let path = e.path.clone();
        let title = e.title.clone();
        shared.ensure_audio();
        if shared.audio.is_none() {
            if dbg {
                notify(Level::Warn, "[preview] no output stream — preview skipped");
            }
            return;
        }
        SelectState::clear_preview_audio(shared);
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        self.preview.cancel = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel::<PreviewMsg>();
        self.preview.prep_rx = Some(rx);
        if dbg {
            notify(Level::Info, format!("[preview] '{title}' autoplay: loading in background"));
        }
        std::thread::spawn(move || {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                return;
            };
            let Some(dir) = path.parent().map(Path::to_path_buf) else {
                return;
            };
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
            let (kx, _progress, _total) = spawn_keysound_decode(jobs, cancel);
            for (id, dec) in kx {
                if tx.send(PreviewMsg::Keysound(id, dec)).is_err() {
                    break;
                }
            }
        });
    }

    /// Stop and clear the active preview playback: signal any in-flight autoplay-preview load to stop
    /// (parse + decode workers check this cancel flag), release the preview namespace so its samples
    /// and ringing voices go away, hand the background bus back to the settings file, and clear all
    /// playback state. Leaves `target` so the debounce state stays owned by the caller.
    pub(super) fn reset_preview_playback(&mut self, shared: &mut AppShared) {
        self.preview.cancel.store(true, Ordering::Relaxed);
        SelectState::clear_preview_audio(shared);
        self.preview.fade = Fade::Idle;
        SelectState::release_bus(shared);
        self.preview.si = None;
        self.preview.loop_us = 0;
        self.preview.next_us = 0;
        self.preview.sched.clear();
        self.preview.cursor = 0;
        self.preview.anchor = 0;
        self.preview.start_us = 0;
        self.preview.end_us = 0;
        self.preview.prep_rx = None;
    }

    /// Tear down the preview entirely (used when leaving Select / entering Play); recreated next time
    /// a preview is needed in select.
    pub(super) fn stop_preview(&mut self, shared: &mut AppShared) {
        self.reset_preview_playback(shared);
        self.preview.target = None;
    }
}

/// A `PREVIEW FADE` longer than the settle delay, for the test that pins the two against each other.
#[cfg(test)]
const LONG_FADE_MS: u32 = 1000;

#[cfg(test)]
mod tests {
    use super::*;

    const SPAN: Duration = Duration::from_millis(200);

    #[test]
    fn a_fade_in_rises_from_silence_to_the_full_level() {
        assert_eq!(fade_level(Fade::In, Duration::ZERO, SPAN), 0.0);
        assert!((fade_level(Fade::In, SPAN / 2, SPAN) - 0.5).abs() < 1e-6);
        assert_eq!(fade_level(Fade::In, SPAN, SPAN), 1.0);
        assert_eq!(fade_level(Fade::In, SPAN * 4, SPAN), 1.0, "a fade that has run over stays at the full level");
    }

    #[test]
    fn a_fade_out_falls_from_the_full_level_to_silence() {
        assert_eq!(fade_level(Fade::Out, Duration::ZERO, SPAN), 1.0);
        assert!((fade_level(Fade::Out, SPAN / 4, SPAN) - 0.75).abs() < 1e-6);
        assert_eq!(fade_level(Fade::Out, SPAN, SPAN), 0.0);
        assert_eq!(fade_level(Fade::Out, SPAN * 4, SPAN), 0.0, "a fade that has run over stays silent");
    }

    /// A `PREVIEW FADE` of zero is what asks for the behaviour the browser had before there were
    /// fades: the preview is at its full level on the first frame and gone on the frame it stops.
    #[test]
    fn a_fade_of_no_length_is_already_over() {
        assert_eq!(fade_level(Fade::In, Duration::ZERO, Duration::ZERO), 1.0);
        assert_eq!(fade_level(Fade::Out, Duration::ZERO, Duration::ZERO), 0.0);
    }

    #[test]
    fn nothing_sounding_is_silent_whatever_the_span() {
        assert_eq!(fade_level(Fade::Idle, Duration::ZERO, SPAN), 0.0);
        assert_eq!(fade_level(Fade::Idle, SPAN, Duration::ZERO), 0.0);
    }

    /// The fade is as long as the settings row says, so a row moved to zero turns fading off.
    #[test]
    fn the_fade_is_as_long_as_the_setting_asks_for() {
        let mut app = crate::stage::select::tests::app();
        assert_eq!(SelectState::fade_span(&app.shared), Duration::from_millis(u64::from(DEFAULT_PREVIEW_FADE_MS)));
        app.shared.config.library.preview_fade_ms = 0;
        assert!(SelectState::fade_span(&app.shared).is_zero());
    }

    /// A fade set longer than the settle delay must not be cut off by the chart the focus moved to:
    /// the row being let go of is left to finish, which is what fading it out was for.
    #[test]
    fn a_fade_longer_than_the_settle_delay_is_left_to_finish() {
        let mut app = crate::stage::select::tests::app();
        app.shared.config.library.preview_fade_ms = LONG_FADE_MS;
        assert!(SelectState::fade_span(&app.shared) > PREVIEW_DEBOUNCE, "the fixture does not test what it says it does");

        let mut state = SelectState::new();
        state.preview.si = Some(0);
        state.preview.fade = Fade::Out;
        state.preview.fade_at = Instant::now();
        state.preview.target = Some(1);
        state.preview.target_at = Instant::now() - PREVIEW_DEBOUNCE * 2;
        state.update_preview(&mut app.shared, Instant::now());
        assert_eq!(state.preview.si, Some(0), "the chart being faded out was cut off by the next one");
        assert_eq!(state.preview.fade, Fade::Out, "the fade was abandoned part way through");
    }

    /// Once the fade has run its course the row that was let go of is torn down, and the chart the
    /// focus has moved to becomes the one that plays.
    #[test]
    fn a_finished_fade_lets_the_next_chart_take_over() {
        let mut app = crate::stage::select::tests::app();
        app.shared.config.library.preview_fade_ms = 0;
        let mut state = SelectState::new();
        state.preview.si = Some(0);
        state.preview.fade = Fade::Out;
        state.preview.fade_at = Instant::now();
        state.update_preview(&mut app.shared, Instant::now());
        assert_eq!(state.preview.fade, Fade::Idle, "a fade of no length is over on the frame it starts");
        assert_eq!(state.preview.si, None, "the chart that was let go of is still held");
    }
}

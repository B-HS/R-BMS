//! The song browser's hover preview: which chart is sounding, how a newly focused row becomes the
//! one to load, and how the two kinds of preview — a `#PREVIEW` clip, or an autoplay pass over the
//! chart itself — are started, looped and torn down.
//!
//! Preview sounds live in their own id namespace inside the shared output engine, so a chart load
//! never disturbs them and leaving the browser clears exactly what the preview owned.
#![allow(clippy::wildcard_imports)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::app_play::schedule_position_us;
use crate::stage::select::SelectState;
use crate::*;

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
        }
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
                self.reset_preview_playback(shared);
            }
        }
        if self.preview.si != cur {
            if let Some(si) = cur
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
                if self.preview.sched.is_empty() {
                    SelectState::clear_preview_audio(shared);
                } else {
                    self.preview.anchor = SelectState::preview_sched_us(shared) - self.preview.start_us;
                    self.preview.cursor = 0;
                }
            }
            return;
        }
        if !self.preview.sched.is_empty() {
            let song = SelectState::preview_sched_us(shared) - self.preview.anchor;
            if let Some(eng) = shared.audio.as_mut() {
                while self.preview.cursor < self.preview.sched.len() && self.preview.sched[self.preview.cursor].0 <= song {
                    let (at, wav) = self.preview.sched[self.preview.cursor];
                    eng.play_on(Bus::Bg, SelectState::preview_sample_id(wav), PREVIEW_GAIN, KEYSOUND_PAN, KEYSOUND_PITCH, at + self.preview.anchor);
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
            if let Some(eng) = shared.audio.as_mut() {
                while sched >= self.preview.next_us {
                    eng.play_on(Bus::Bg, PREVIEW_ID, PREVIEW_GAIN, KEYSOUND_PAN, KEYSOUND_PITCH, self.preview.next_us);
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
                eprintln!("[preview] song index {si} out of range");
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
        shared.ensure_audio();
        SelectState::clear_preview_audio(shared);
        let start_at = SelectState::preview_sched_us(shared);
        let Some(eng) = shared.audio.as_mut() else {
            if dbg {
                eprintln!("[preview] no output stream — preview skipped");
            }
            return;
        };
        if let Err(err) = eng.load(PREVIEW_ID, bytes, ext.as_deref()) {
            if dbg {
                eprintln!("[preview] decode failed {}: {err}", path.display());
            }
            return;
        }
        let dur = eng.sample_duration_us(PREVIEW_ID).unwrap_or(0);
        eng.play_on(Bus::Bg, PREVIEW_ID, PREVIEW_GAIN, KEYSOUND_PAN, KEYSOUND_PITCH, start_at);
        self.preview.loop_us = dur;
        self.preview.next_us = start_at + dur.max(PREVIEW_MIN_LOOP_US);
        if dbg {
            eprintln!("[preview] playing '{title}' ({}) dur_us={dur} start_us={start_at}", path.display());
        }
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
                eprintln!("[preview] no output stream — preview skipped");
            }
            return;
        }
        SelectState::clear_preview_audio(shared);
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        self.preview.cancel = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel::<PreviewMsg>();
        self.preview.prep_rx = Some(rx);
        if dbg {
            eprintln!("[preview] '{title}' autoplay: loading in background");
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
    /// and ringing voices go away, and clear all playback state. Leaves `target` so the debounce
    /// state stays owned by the caller.
    pub(super) fn reset_preview_playback(&mut self, shared: &mut AppShared) {
        self.preview.cancel.store(true, Ordering::Relaxed);
        SelectState::clear_preview_audio(shared);
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

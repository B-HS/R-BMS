//! The song browser's hover preview: which chart is sounding, how a newly focused row becomes the
//! one to load, and how the two kinds of preview — a `#PREVIEW` clip, or an autoplay pass over the
//! chart itself — are started, looped and torn down.
//!
//! Preview sounds live in their own id namespace inside the shared output engine, so a chart load
//! never disturbs them and leaving the browser clears exactly what the preview owned.
//!
//! # The browser's music and the preview
//!
//! A set whose `select` sound is music has it looped for as long as the browser is up
//! ([`crate::syssound`]). The reference runs the two as one switch (`PreviewMusicProcessor`): the
//! music plays until a song's preview starts, is turned down to nothing under it, and is turned back
//! up when the preview is stopped. rbms keeps that and decides the cases the reference does not meet,
//! because here every chart has a preview, an autoplay pass over its keysounds when it names no
//! `#PREVIEW` clip:
//!
//! - The music is turned down when a preview begins to sound ([`Fade::In`]), over the preview's own
//!   fade, and is turned up when it begins to leave ([`Fade::Out`], which the focus moving on starts),
//!   over the same fade. The two cross: as the row being left fades out, the music comes in.
//! - While the focus has settled nowhere, which is the debounce ([`PREVIEW_DEBOUNCE`]) and the load
//!   that follows it, no preview sounds and the music is up. Scrolling through a list therefore
//!   brings the music in between rows, and a row that is stayed on takes it down again once its
//!   preview is ready (the reference does the same for a row with a clip, and keeps the music up
//!   throughout for a row with none).
//! - A focus that is not a chart, a library with previews switched off, and a chart whose preview
//!   cannot be played all leave the music up.
//! - The music keeps running under a preview; it is not restarted when the preview goes.
//!
//! # Which volumes a preview answers to
//!
//! The master volume scales everything. A `#PREVIEW` clip is one mixed recording and sounds on the
//! background bus. An autoplay pass sounds each of the chart's events on the bus it has in play: the
//! accompaniment on the background bus and the keysound of each note on the key bus
//! ([`crate::play_sink::bus_for`]), so the three sliders of the default skin's volume tab (master,
//! key, BGM) all turn a preview down.
#![allow(clippy::wildcard_imports)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

#[cfg(test)]
use rbms_config::DEFAULT_PREVIEW_FADE_MS;

use rbms_play::PlaySource;

use crate::app_play::schedule_position_us;
use crate::play_sink::bus_for;
use crate::stage::select::SelectState;
use crate::*;

/// Where the preview's level is on its way in or out.
///
/// A voice's gain is fixed when the sound is booked, so a clip already playing cannot be faded by
/// changing it. The buses can be, and while the browser is up the preview is the only thing on the
/// background and key buses — so the fade is a ramp on both, put back where the settings file has
/// them as soon as the preview is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fade {
    /// Nothing is sounding and the background and key buses are the settings file's to set.
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
    /// Autoplay-preview timeline `(at_us, wav, source)` for the focused chart when it defines no
    /// `#PREVIEW` file — extracted once from a throwaway autoplay `Player`, then replayed against
    /// the engine clock and looped, each sound on the bus its source has in play. Empty in
    /// file-preview mode.
    sched: Vec<(i64, u32, PlaySource)>,
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

/// Whether the select music is turned down: while a preview is sounding and not on its way out.
fn select_bgm_is_held(fade: Fade) -> bool {
    fade == Fade::In
}

/// The level at which the preview's buses are the settings file's own gains.
const PREVIEW_FULL_LEVEL: f32 = 1.0;

/// How far past the chart's last event the autoplay pass runs, so a keysound booked there is caught.
const AUTOPLAY_PASS_TAIL_US: i64 = 1_000_000;

/// The gains the buses a preview sounds on are to be at while it is at `level`: the settings file's
/// gain for each, scaled by the level. The master gain is not here; it scales the buses' sum and is
/// the settings file's to set.
fn preview_bus_gains(audio: &rbms_config::AudioOptions, level: f32) -> [(Bus, f32); 2] {
    [(bus_for(PlaySource::Key), audio.key * level), (bus_for(PlaySource::Bgm), audio.bg * level)]
}

/// Every event the chart plays by itself, time-sorted, as an autoplay pass over it gives them: when,
/// which keysound, and where it came from. A throwaway `Player` is run to the end and its judging is
/// discarded.
fn autoplay_preview_schedule(model: rbms_model::Model) -> Vec<(i64, u32, PlaySource)> {
    let mut sched = Vec::new();
    let mut p = Player::new(model, true);
    let end = p.last_time_us() + AUTOPLAY_PASS_TAIL_US;
    p.update(end, |ev| {
        if ev.wav >= 0 {
            sched.push((ev.at_us, ev.wav as u32, ev.source));
        }
    });
    sched.sort_by_key(|s| s.0);
    sched
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

    /// Move the fade on by one frame: push the preview's level onto the background and key buses, and
    /// once a fade-out has run its course tear the preview down — which is what lets the chart the
    /// focus has moved to become the one that plays.
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
        SelectState::put_buses_at(shared, level);
    }

    /// Put the buses a preview sounds on at `level` of the settings file's gains.
    fn put_buses_at(shared: &mut AppShared, level: f32) {
        if let Some(audio) = shared.audio.as_mut() {
            for (bus, gain) in preview_bus_gains(&shared.config.audio, level) {
                audio.set_bus_gain(bus, gain);
            }
        }
    }

    /// Put the background and key buses back where the settings file has them, which is what a
    /// preview that is no longer sounding owes every other screen.
    fn release_bus(shared: &mut AppShared) {
        SelectState::put_buses_at(shared, PREVIEW_FULL_LEVEL);
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
        self.drive_preview(shared, now);
        let hold = select_bgm_is_held(self.preview.fade);
        shared.drive_select_bgm(true, hold, SelectState::fade_span(shared));
    }

    /// Stop the browser's music, which is what leaving the browser asks for. Nothing happens when it
    /// was not running, so a set whose `select` is a cue keeps it sounding on the way out.
    pub(super) fn end_select_bgm(shared: &mut AppShared) {
        shared.drive_select_bgm(false, false, Duration::ZERO);
    }

    /// One frame of the preview itself: the debounce, the load, the playback and the fades.
    fn drive_preview(&mut self, shared: &mut AppShared, now: Instant) {
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
                    let (at, wav, source) = self.preview.sched[self.preview.cursor];
                    eng.play_on(bus_for(source), SelectState::preview_sample_id(wav), volume, KEYSOUND_PAN, KEYSOUND_PITCH, at + self.preview.anchor);
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
            let sched = autoplay_preview_schedule(model);
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
    /// and ringing voices go away, hand the background and key buses back to the settings file, and
    /// clear all playback state. Leaves `target` so the debounce state stays owned by the caller.
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

    /// The music is down only while a preview is sounding: not before one starts (the debounce and the
    /// load), not once it has begun to leave, and not when there is none.
    #[test]
    fn the_select_music_is_held_only_while_a_preview_is_sounding() {
        assert!(select_bgm_is_held(Fade::In));
        assert!(!select_bgm_is_held(Fade::Out), "a preview that is leaving lets the music back in");
        assert!(!select_bgm_is_held(Fade::Idle), "with no preview the music plays");
    }

    /// Following a preview through its life gives the sequence the module documents: music, then
    /// held when it sounds, then up again as the focus moves, then held for the next one.
    #[test]
    fn scrolling_past_a_row_brings_the_music_in_between_previews() {
        use crate::syssound::{BgmStep, SelectBgm};
        const ENGINE: u64 = 1;
        let mut bgm = SelectBgm::default();
        let mut steps = Vec::new();
        for fade in [Fade::Idle, Fade::Idle, Fade::In, Fade::In, Fade::Out, Fade::Idle, Fade::Idle, Fade::In] {
            steps.push(bgm.step(true, Some(ENGINE), select_bgm_is_held(fade)));
        }
        assert_eq!(
            steps,
            [BgmStep::Start, BgmStep::Idle, BgmStep::Level(0.0), BgmStep::Idle, BgmStep::Level(1.0), BgmStep::Idle, BgmStep::Idle, BgmStep::Level(0.0)]
        );
    }

    const SAMPLE_RATE_HZ: u32 = 48_000;
    const MIXER_VOICES: usize = 8;
    const SAMPLE_FRAMES: usize = 24_000;
    const SETTLED_FRAMES: usize = 12_000;
    const SAMPLE_LEVEL: f32 = 0.5;
    const OUTPUT_CHANNELS: usize = 2;
    const SILENT: f32 = 0.0;

    /// The BMS of a chart with one accompaniment sound and one note, both on lane 1 of the first measure.
    const ONE_BGM_AND_ONE_NOTE: &str = "#PLAYER 1\n#BPM 120\n#WAV01 bgm.wav\n#WAV02 note.wav\n#00101:01\n#00111:02\n";

    /// What a preview sounding on `bus` at `gain` comes out as at the end of the mixer, with the master
    /// gain the settings hold: the same chain the engine runs, which is the bus gain, the master gain and
    /// the sound itself.
    fn heard_on(audio: &rbms_config::AudioOptions, bus: Bus, gain: f32) -> f32 {
        use std::sync::Arc;
        let mut mixer = rbms_audio::Mixer::new(SAMPLE_RATE_HZ, OUTPUT_CHANNELS as u16, MIXER_VOICES);
        mixer.apply(rbms_audio::Command::MasterGain(audio.master));
        mixer.apply(rbms_audio::Command::BusGain { bus, gain });
        let sample = Arc::new(rbms_audio::SampleData { pcm: vec![SAMPLE_LEVEL; SAMPLE_FRAMES].into(), channels: 1, rate: SAMPLE_RATE_HZ });
        mixer.apply(rbms_audio::Command::Play { sample, gain: 1.0, pan: -1.0, pitch: 1.0, key: 0, at_frame: 0, bus });
        let mut out = vec![0.0f32; SETTLED_FRAMES * OUTPUT_CHANNELS];
        mixer.mix(&mut out);
        out[out.len() - OUTPUT_CHANNELS]
    }

    /// Both buses a preview sounds on are faded together and neither is the system bus, which belongs
    /// to the sounds of the interface.
    #[test]
    fn a_fade_is_put_on_the_key_and_the_background_bus_alike() {
        let mut audio = rbms_config::AudioOptions::default();
        (audio.key, audio.bg, audio.system) = (0.4, 0.6, 0.8);

        assert_eq!(preview_bus_gains(&audio, 0.5), [(Bus::Key, 0.2), (Bus::Bg, 0.3)], "half way through a fade each bus is at half its gain");
        assert_eq!(preview_bus_gains(&audio, 0.0), [(Bus::Key, 0.0), (Bus::Bg, 0.0)], "the start of a fade-in is silent on both");
        assert_eq!(
            preview_bus_gains(&audio, PREVIEW_FULL_LEVEL),
            [(Bus::Key, audio.key), (Bus::Bg, audio.bg)],
            "releasing the buses hands both back at the settings file's gains"
        );
    }

    /// An autoplay preview sounds the chart's accompaniment on the background bus and the keysound of its
    /// notes on the key bus, as play does, so the KEY slider is heard in the browser as well as BGM.
    #[test]
    fn an_autoplay_preview_keeps_the_source_of_each_event_and_sounds_it_on_that_bus() {
        let src = rbms_parser::parse_with(ONE_BGM_AND_ONE_NOTE.as_bytes(), Default::default());
        let sched = autoplay_preview_schedule(rbms_chart::to_model(&src, Mode::BEAT_7K));

        let buses = sched.iter().map(|&(_, wav, source)| (wav, bus_for(source))).collect::<Vec<_>>();
        assert_eq!(buses.len(), 2, "one accompaniment sound and one note: {buses:?}");
        assert!(buses.contains(&(1, Bus::Bg)), "the accompaniment is not on the background bus: {buses:?}");
        assert!(buses.contains(&(2, Bus::Key)), "the note's keysound is not on the key bus: {buses:?}");
    }

    /// What dragging the MASTER slider to nothing did to a preview before it wrote the master volume:
    /// it set the system volume, and a preview, which sounds on no system bus, stayed as loud as it was.
    /// Now it is silent on both of its buses, and the other two sliders each silence their own bus only.
    #[test]
    fn the_master_key_and_bgm_sliders_each_turn_a_preview_down() {
        use crate::skin_host::writers::{VolumeTarget, rate_volume, set_volume};
        use rbms_skin::property::VolumeBus;
        use rbms_skin::property::generated::{RATE_BGMVOLUME, RATE_KEYVOLUME, RATE_MASTERVOLUME};

        let defaults = rbms_config::AudioOptions::default();
        for (bus, gain) in preview_bus_gains(&defaults, PREVIEW_FULL_LEVEL) {
            assert!(heard_on(&defaults, bus, gain) > SILENT, "the fixture's preview is not audible on {bus:?}");
        }

        let heard_with_slider_at_nothing = |rate: i32| {
            let mut audio = defaults.clone();
            let target = rate_volume(rate).expect("a volume rate");
            assert!(set_volume(&mut audio, target, 0.0));
            preview_bus_gains(&audio, PREVIEW_FULL_LEVEL).map(|(bus, gain)| (bus, heard_on(&audio, bus, gain)))
        };
        assert_eq!(rate_volume(RATE_MASTERVOLUME), Some(VolumeTarget::Master));
        assert_eq!(rate_volume(RATE_KEYVOLUME), Some(VolumeTarget::Bus(VolumeBus::Key)));
        assert_eq!(rate_volume(RATE_BGMVOLUME), Some(VolumeTarget::Bus(VolumeBus::Background)));

        for (bus, level) in heard_with_slider_at_nothing(RATE_MASTERVOLUME) {
            assert_eq!(level, SILENT, "the master slider at nothing left the preview audible on {bus:?}");
        }
        for (bus, level) in heard_with_slider_at_nothing(RATE_KEYVOLUME) {
            assert_eq!(level == SILENT, bus == Bus::Key, "the key slider at nothing must silence the key bus and only it ({bus:?} -> {level})");
        }
        for (bus, level) in heard_with_slider_at_nothing(RATE_BGMVOLUME) {
            assert_eq!(level == SILENT, bus == Bus::Bg, "the BGM slider at nothing must silence the background bus and only it ({bus:?} -> {level})");
        }
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

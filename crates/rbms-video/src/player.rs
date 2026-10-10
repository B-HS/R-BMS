//! Playing a movie against a clock somebody else keeps.
//!
//! A [`VideoPlayer`] owns one decoder and one thread. The thread decodes ahead into a queue of a
//! few frames and stops when the queue is full; whoever draws says what time it is
//! ([`VideoPlayer::advance`]) and is handed the frame that is due, without ever waiting on a
//! decode. So the clock is the caller's entirely: a clock that stands still takes no frames, the
//! queue stays full and the thread sleeps, and the movie stands still with it.
//!
//! A movie is played round and round. The thread goes back to the first frame when it reaches the
//! last and carries on, with every frame's time moved on by the movie's length for each pass
//! already made, so the caller's clock just keeps counting.
//!
//! The clock need not run evenly, or forwards. Wherever it is said to be, the frame on show
//! becomes the one due then, and what that costs is kept from growing with how far the clock went:
//!
//! - A clock that jumps ahead by more than a pass is taken by arithmetic to the pass it landed in,
//!   since every pass is the same frames; nothing of the passes in between is decoded.
//! - Inside a pass the decoder is asked to go to wherever nearest before the time it can start
//!   from ([`VideoDecoder::seek`]), and the frames from there to the time are decoded without being
//!   turned into pixels: they have to be decoded for the ones after them to come out right, but
//!   only a frame that can still be shown costs a colour conversion.
//! - A clock that goes back to before the frame on show throws away what was decoded ahead and
//!   starts from the time it went back to in the same way.
//!
//! While the thread catches up the frame that was on show stays there, and nobody waits.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::{VideoDecoder, VideoError};

/// How many decoded frames a player keeps ahead of the one on show. At 1920 by 1080 a frame is
/// eight megabytes, so this is also what a playing movie costs in memory beyond its decoder.
pub const DEFAULT_QUEUE_FRAMES: usize = 3;

/// The name the decoding thread goes by.
const THREAD_NAME: &str = "rbms-video";

/// How many pixel buffers are kept to be written into again beyond the ones in the queue: the one
/// on show and the one being decoded into.
const SPARE_BUFFERS_BEYOND_QUEUE: usize = 2;

/// What is said of a movie whose decoder panicked.
const PANICKED: &str = "the decoder panicked";

/// One decoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    /// When the frame is shown, in microseconds since the movie was started, every pass over the
    /// movie counted.
    pub time_us: i64,
    pub width: u32,
    pub height: u32,
    /// The frame's pixels as RGBA, top row first.
    pub rgba: Vec<u8>,
}

/// What the thread and the caller share.
#[derive(Debug, Default)]
struct State {
    /// Decoded frames nobody has taken yet, oldest first.
    queue: VecDeque<VideoFrame>,
    /// Pixel buffers of frames that were replaced, to be decoded into again.
    spare: Vec<Vec<u8>>,
    /// The time the caller last said it was.
    target_us: i64,
    /// Counts the times what was decoded ahead was thrown away: the movie was started over, or
    /// its clock went back. The thread works on one count at a time, throws away what it decoded
    /// for an older one and finds its place again from `target_us`.
    generation: u64,
    /// Whether the player is being dropped.
    stop: bool,
    /// Why the thread gave up, once it has.
    failure: Option<String>,
    /// How many samples the decoder has said came to nothing.
    lost_samples: u64,
    /// Whether the thread has returned.
    exited: bool,
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    /// Signalled when the thread has something to do: room in the queue, a restart or a stop.
    for_worker: Condvar,
    /// Signalled when the caller may have something to take: a frame, or the thread's end.
    for_caller: Condvar,
    capacity: usize,
}

impl Shared {
    /// The shared state. A lock that was poisoned is taken anyway: the state is counters and
    /// buffers, all of which are still what they say they are.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Keeps a pixel buffer to be decoded into again, unless enough are kept already.
    fn recycle(&self, state: &mut State, buffer: Vec<u8>) {
        if state.spare.len() < self.capacity + SPARE_BUFFERS_BEYOND_QUEUE {
            state.spare.push(buffer);
        }
    }
}

/// Says the decoding thread has returned when it is dropped, however the thread came to return:
/// a caller waiting on a frame is woken to find there is none coming, also when the decoder
/// panicked, which is then what the movie is said to have stopped for.
struct ThreadEnd<'a>(&'a Shared);

impl Drop for ThreadEnd<'_> {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        if std::thread::panicking() && state.failure.is_none() {
            state.failure = Some(PANICKED.into());
        }
        state.exited = true;
        drop(state);
        self.0.for_caller.notify_all();
    }
}

/// The decoding thread: keeps the queue full until told to stop, and says why if it gives up. The
/// decoder is gone by the time the thread is said to have returned.
fn decode_ahead(shared: &Shared, decoder: Box<dyn VideoDecoder>) {
    let _end = ThreadEnd(shared);
    if let Err(error) = decode_passes(shared, decoder) {
        shared.lock().failure = Some(error.to_string());
    }
}

/// The body of the decoding thread. `Ok` when the player was dropped.
fn decode_passes(shared: &Shared, mut decoder: Box<dyn VideoDecoder>) -> Result<(), VideoError> {
    let info = decoder.info();
    let (interval_us, pass_us) = (info.frame_interval_us(), info.duration_us.max(1));
    let mut generation = 0u64;
    let mut pass_start_us = 0i64;
    let mut frames_this_pass = 0u64;
    let mut is_whole_pass = true;
    loop {
        let (target_us, was_thrown_away, mut buffer) = {
            let mut state = shared.lock();
            while !state.stop && state.generation == generation && state.queue.len() >= shared.capacity {
                state = shared.for_worker.wait(state).unwrap_or_else(PoisonError::into_inner);
            }
            if state.stop {
                return Ok(());
            }
            let was_thrown_away = std::mem::replace(&mut generation, state.generation) != state.generation;
            (state.target_us, was_thrown_away, state.spare.pop().unwrap_or_default())
        };

        let is_in_another_pass = was_thrown_away || target_us >= pass_start_us.saturating_add(pass_us);
        if is_in_another_pass {
            pass_start_us = target_us.div_euclid(pass_us) * pass_us;
        }
        let into_pass_us = target_us - pass_start_us;
        let is_behind = |decoder: &dyn VideoDecoder| decoder.next_time_us().is_some_and(|at_us| at_us.saturating_add(interval_us) <= into_pass_us);
        if (is_in_another_pass || is_behind(decoder.as_ref())) && decoder.seek(into_pass_us)? {
            is_whole_pass = false;
        }

        let superseded = is_behind(decoder.as_ref());
        let decoded = decoder.next_frame(if superseded { None } else { Some(&mut buffer) });
        let mut state = shared.lock();
        state.lost_samples = decoder.lost_samples();
        match decoded? {
            Some(at_us) => {
                frames_this_pass += 1;
                if superseded || state.generation != generation {
                    shared.recycle(&mut state, buffer);
                    continue;
                }
                state.queue.push_back(VideoFrame { time_us: pass_start_us + at_us, width: info.width, height: info.height, rgba: buffer });
                shared.for_caller.notify_all();
            }
            None => {
                shared.recycle(&mut state, buffer);
                drop(state);
                if is_whole_pass && frames_this_pass == 0 {
                    return Err(VideoError::Decode("the movie has no frame that decodes".into()));
                }
                decoder.rewind()?;
                pass_start_us += pass_us;
                (frames_this_pass, is_whole_pass) = (0, true);
            }
        }
    }
}

/// One movie being played: a decoder, the thread driving it, and the frame that is on show.
#[derive(Debug)]
pub struct VideoPlayer {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    /// The frame that is on show: the newest one whose time has come.
    current: Option<VideoFrame>,
}

impl VideoPlayer {
    /// Starts playing `decoder` from wherever it is, keeping [`DEFAULT_QUEUE_FRAMES`] frames ahead.
    pub fn spawn(decoder: Box<dyn VideoDecoder>) -> Result<VideoPlayer, VideoError> {
        VideoPlayer::with_queue(decoder, DEFAULT_QUEUE_FRAMES)
    }

    /// [`VideoPlayer::spawn`], keeping `frames` frames ahead; at least one is kept.
    pub fn with_queue(decoder: Box<dyn VideoDecoder>, frames: usize) -> Result<VideoPlayer, VideoError> {
        let shared = Arc::new(Shared { state: Mutex::new(State::default()), for_worker: Condvar::new(), for_caller: Condvar::new(), capacity: frames.max(1) });
        let for_thread = Arc::clone(&shared);
        let worker = std::thread::Builder::new().name(THREAD_NAME.into()).spawn(move || decode_ahead(&for_thread, decoder)).map_err(VideoError::io)?;
        Ok(VideoPlayer { shared, worker: Some(worker), current: None })
    }

    /// The frame on show, when there is one: none before the first has been decoded, and none
    /// from when the clock went back to before the frame that was on show until the one due then
    /// has been decoded.
    pub fn frame(&self) -> Option<&VideoFrame> {
        self.current.as_ref()
    }

    /// Throws away the frame on show and everything decoded ahead of it, and has the thread find
    /// its place again from the time last said.
    fn throw_away(&mut self, state: &mut State) {
        state.generation += 1;
        let stale: Vec<VideoFrame> = state.queue.drain(..).chain(self.current.take()).collect();
        for frame in stale {
            self.shared.recycle(state, frame.rgba);
        }
        self.shared.for_worker.notify_all();
    }

    /// Takes every queued frame that is due at `time_us` and keeps the newest as the frame on
    /// show, answering whether that changed it.
    ///
    /// A time before the one last said, with the frame on show not due yet at it, is the clock
    /// having gone back: nothing that was decoded is of use then, and the thread starts again from
    /// there. A thread that has returned is left its frames; it decodes nothing more for any time.
    fn take_due(&mut self, state: &mut State, time_us: i64) -> bool {
        let time_us = time_us.max(0);
        let went_back = time_us < std::mem::replace(&mut state.target_us, time_us);
        if went_back && !state.exited && self.current.as_ref().is_none_or(|frame| frame.time_us > time_us) {
            self.throw_away(state);
        }
        let mut changed = false;
        while state.queue.front().is_some_and(|frame| frame.time_us <= time_us) {
            if let Some(replaced) = state.queue.pop_front().and_then(|frame| self.current.replace(frame)) {
                self.shared.recycle(state, replaced.rgba);
            }
            changed = true;
        }
        if changed {
            self.shared.for_worker.notify_all();
        }
        changed
    }

    /// Says it is `time_us` microseconds since the movie was started, and answers whether that
    /// changed the frame on show. A time before the movie was started is taken as its start.
    ///
    /// Never waits: a frame that is due and not decoded yet is simply not on show yet, and the one
    /// before it stays. A time that has not moved on changes nothing, which is how a clock that
    /// stands still holds the movie still. A time far from the last one said, ahead or back, is
    /// caught up with by the thread, and until it has been the answer is that nothing changed.
    pub fn advance(&mut self, time_us: i64) -> bool {
        let shared = Arc::clone(&self.shared);
        let mut state = shared.lock();
        self.take_due(&mut state, time_us)
    }

    /// [`VideoPlayer::advance`], waiting up to `patience` for the frame that is due at `time_us` to
    /// have been decoded. For a caller that draws one frame and has to have the right one: a
    /// capture, a test.
    ///
    /// The wait ends when a frame for a later time is waiting behind the one on show, when the
    /// thread has given up, or when `patience` runs out.
    pub fn advance_blocking(&mut self, time_us: i64, patience: Duration) -> bool {
        let deadline = Instant::now() + patience;
        let shared = Arc::clone(&self.shared);
        let mut state = shared.lock();
        let mut changed = false;
        loop {
            changed |= self.take_due(&mut state, time_us);
            if state.exited || !state.queue.is_empty() {
                return changed;
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()).filter(|left| !left.is_zero()) else {
                return changed;
            };
            state = shared.for_caller.wait_timeout(state, left).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    /// Starts the movie over: what was decoded is thrown away, nothing is on show, and the next
    /// time said is measured from the first frame again.
    pub fn restart(&mut self) {
        let shared = Arc::clone(&self.shared);
        let mut state = shared.lock();
        state.target_us = 0;
        self.throw_away(&mut state);
    }

    /// Why the movie stopped playing, once it has: the decoder gave up on it, or panicked.
    pub fn failure(&self) -> Option<String> {
        self.shared.lock().failure.clone()
    }

    /// How many of the movie's samples the decoder has found damaged and left out so far. The
    /// movie goes on playing around them: the frame before a damaged stretch stays on show until
    /// the first frame after it is due.
    pub fn lost_samples(&self) -> u64 {
        self.shared.lock().lost_samples
    }

    /// Whether the movie has nothing more to show: the decoding thread is gone and every frame it
    /// decoded has been taken. A thread that gave up leaves the frames it had decoded to be shown
    /// when they are due, so this turns true only once the last of them is on show.
    pub fn has_ended(&self) -> bool {
        let state = self.shared.lock();
        state.exited && state.queue.is_empty()
    }
}

impl Drop for VideoPlayer {
    /// Stops the thread and waits for it, so nothing of the movie outlives its player: not the
    /// thread, not the decoder, not the file it holds open.
    fn drop(&mut self) {
        self.shared.lock().stop = true;
        self.shared.for_worker.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests;

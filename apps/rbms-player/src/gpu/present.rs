//! Waiting for the display on a thread of its own, so the thread the window's events reach is never
//! the one the display holds up.
//!
//! A surface hands out the image the next frame is drawn into only once the display has room for
//! it, which under vsync is up to a whole refresh away. The window's events reach the app on the
//! thread that runs its event loop, and they reach it only between the handlers that thread runs. A
//! frame that waited for its image there held every key that came in meanwhile, and a key is judged
//! at the instant it reaches the app (test-debt 3.53). Here a worker asks for the image and waits
//! for it; the event loop is woken once it has come ([`Presenter::start`]'s `wake`), draws into it,
//! submits it and gives it back to the worker to present. Between frames the event loop waits on
//! nothing but the window, so a key reaches it as it comes in.
//!
//! This is the arrangement every rhythm game that judges input precisely settles on: input is
//! never read by a thread the display holds up (LR2 polls input while it waits for vblank,
//! beatoraja and Etterna read it on threads of their own, osu!framework draws on another thread).
//! See `docs/acknowledge/2026-10-15-adr-keyboard-input-timing.md`.
//!
//! An image is asked for once at a time and taken by the event loop before the next is asked for,
//! so the worker never holds more than one. The event loop never waits for the worker: a surface
//! that has to be reconfigured while an image is being asked for is reconfigured once that image
//! has come and been let go of ([`Presenter::waiting`]), and the worker is waited for, a bounded
//! while, only when the app is shutting down ([`Presenter::stop`]).
//!
//! While an image is being asked for, the event loop does not submit to the queue, poll the device
//! or reconfigure the surface. wgpu holds the device's fence for reading for as long as it waits
//! for the image (wgpu-core 29.0.4 `present.rs:165-175`), and a submit takes it for writing
//! (`device/queue.rs`), so any of them would wait for the display on the event loop again, which is
//! the wait this thread exists to take off it. The only submit is the frame's own, made after its
//! image has come ([`super::Gpu::render`]).
//!
//! A worker that panics wakes the event loop on its way out, so the failure reaches the event loop
//! at once, as a panic on it ([`Presenter::take`], [`Presenter::request`]), rather than leaving a
//! window that never draws again.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use crate::notify::{Level, notify};

/// The name the worker thread shows in a debugger or a crash report.
const PRESENT_THREAD: &str = "rbms-present";

/// The longest wgpu waits for an image before it gives up on one (wgpu-core 29.0.4 `present.rs`,
/// `FRAME_TIMEOUT_MS`): as long as an app shutting down waits for the worker to let go of the
/// surface ([`Presenter::stop`]).
pub(crate) const STOP_PATIENCE: Duration = Duration::from_secs(1);

/// A surface the images of frames come from and go back to.
pub(crate) trait Swapchain: Send + Sync + 'static {
    /// What asking for an image gives: the image, or why there is none.
    type Acquired: Send + 'static;
    /// An image drawn into, ready to be shown.
    type Frame: Send + 'static;

    /// Wait for the next image the display has room for.
    fn acquire(&self) -> Self::Acquired;

    /// Show an image whose drawing has been submitted.
    fn present(&self, frame: Self::Frame);
}

impl Swapchain for wgpu::Surface<'static> {
    type Acquired = wgpu::CurrentSurfaceTexture;
    type Frame = wgpu::SurfaceTexture;

    fn acquire(&self) -> wgpu::CurrentSurfaceTexture {
        self.get_current_texture()
    }

    fn present(&self, frame: wgpu::SurfaceTexture) {
        frame.present();
    }
}

/// What the event loop asks the worker to do, in order.
enum Job<F> {
    Acquire,
    Present(F),
    Stop,
}

/// The worker's way out, however it goes: dropped last on it, after everything it held of the
/// surface, it wakes the event loop and tells [`Presenter::stop`] the worker is done.
struct Leaving<W: Fn()> {
    wake: W,
    _done: Sender<()>,
}

impl<W: Fn()> Drop for Leaving<W> {
    fn drop(&mut self) {
        (self.wake)();
    }
}

/// The worker, as the event loop sees it.
///
/// Dropping it ends the worker once it has finished what it is doing, without waiting for that: an
/// image asked for of a window that is going away may never come, and a worker still waiting for one
/// must not hold up the app's exit.
pub(crate) struct Presenter<S: Swapchain> {
    jobs: Sender<Job<S::Frame>>,
    acquired: Receiver<S::Acquired>,
    done: Receiver<()>,
    /// Whether an image has been asked for and not yet taken.
    asked: bool,
}

impl<S: Swapchain> Presenter<S> {
    /// Start the worker on `chain`. `wake` is called on the worker each time an image asked for has
    /// come, to have the event loop take it ([`Presenter::take`]), and once more as the worker ends.
    pub(crate) fn start(chain: Arc<S>, wake: impl Fn() + Send + 'static) -> std::io::Result<Presenter<S>> {
        let (jobs, inbox) = mpsc::channel::<Job<S::Frame>>();
        let (outbox, acquired) = mpsc::channel();
        let (done_tx, done) = mpsc::channel();
        thread::Builder::new().name(PRESENT_THREAD.to_string()).spawn(move || {
            let leaving = Leaving { wake, _done: done_tx };
            let outbox = outbox;
            let chain = chain;
            for job in inbox {
                match job {
                    Job::Acquire => {
                        if outbox.send(chain.acquire()).is_err() {
                            return;
                        }
                        (leaving.wake)();
                    }
                    Job::Present(frame) => chain.present(frame),
                    Job::Stop => return,
                }
            }
        })?;
        Ok(Presenter { jobs, acquired, done, asked: false })
    }

    /// Ask for the next image, unless one is already asked for and not taken. Returns at once.
    ///
    /// # Panics
    ///
    /// As [`Presenter::take`].
    pub(crate) fn request(&mut self) {
        if self.asked {
            return;
        }
        if self.jobs.send(Job::Acquire).is_err() {
            panic!("the {PRESENT_THREAD} thread stopped");
        }
        self.asked = true;
    }

    /// Whether an image has been asked for and not yet taken: the surface may not be reconfigured
    /// until it has been ([`Presenter::take`]).
    pub(crate) fn waiting(&self) -> bool {
        self.asked
    }

    /// The image asked for, if it has come. Returns at once.
    ///
    /// # Panics
    ///
    /// If the worker stopped without answering, which only a panic on it does: the app cannot draw
    /// another frame, and a panic is what the same failure on the event loop's thread was.
    pub(crate) fn take(&mut self) -> Option<S::Acquired> {
        if !self.asked {
            return None;
        }
        match self.acquired.try_recv() {
            Ok(acquired) => {
                self.asked = false;
                Some(acquired)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("the {PRESENT_THREAD} thread stopped"),
        }
    }

    /// Have the worker show `frame`, whose drawing has been submitted. Returns at once.
    pub(crate) fn present(&self, frame: S::Frame) {
        self.jobs.send(Job::Present(frame)).ok();
    }

    /// End the worker once it has done what it was asked, and wait for it to, for at most
    /// `patience`: what the app does as it shuts down, so the surface and the window it shows on are
    /// let go of last on the event loop's thread, which on macOS is the only one that may. A worker
    /// still waiting for an image past `patience` is left to the process's exit. Asks for nothing
    /// after this.
    pub(crate) fn stop(&mut self, patience: Duration) {
        self.jobs.send(Job::Stop).ok();
        self.asked = false;
        if let Err(RecvTimeoutError::Timeout) = self.done.recv_timeout(patience) {
            notify(Level::Warn, format!("the {PRESENT_THREAD} thread did not finish within {patience:?} of the app closing"));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;

    /// How long a test waits for something the worker does at once before calling it a failure: far
    /// longer than anything here takes, so a slow machine does not fail it, and short enough that a
    /// wait that would never end fails the test rather than hanging it.
    const PATIENCE: Duration = Duration::from_secs(10);

    /// A surface whose every image comes only when the test lets it: the display a test holds up for
    /// as long as it likes. It numbers the images it hands out and notes each one it shows.
    struct HeldDisplay {
        vblanks: Mutex<Receiver<()>>,
        handed: Mutex<u32>,
        shown: Sender<u32>,
    }

    impl Swapchain for HeldDisplay {
        type Acquired = u32;
        type Frame = u32;

        fn acquire(&self) -> u32 {
            self.vblanks.lock().expect("the display's lock").recv().ok();
            let mut handed = self.handed.lock().expect("the display's lock");
            *handed += 1;
            *handed
        }

        fn present(&self, frame: u32) {
            self.shown.send(frame).ok();
        }
    }

    /// A presenter on a [`HeldDisplay`], with the levers a test pulls: the vblank that lets the
    /// next image come, the wakes the event loop is sent, and the images shown.
    struct Rig {
        presenter: Presenter<HeldDisplay>,
        display: Arc<HeldDisplay>,
        vblank: Sender<()>,
        woken: Receiver<()>,
        shown: Receiver<u32>,
    }

    fn rig() -> Rig {
        let (vblank, vblanks) = mpsc::channel();
        let (shown_tx, shown) = mpsc::channel();
        let (wake, woken) = mpsc::channel();
        let display = Arc::new(HeldDisplay { vblanks: Mutex::new(vblanks), handed: Mutex::new(0), shown: shown_tx });
        let presenter = Presenter::start(display.clone(), move || {
            wake.send(()).ok();
        })
        .expect("the worker starts");
        Rig { presenter, display, vblank, woken, shown }
    }

    /// The event loop's side of a frame: wait to be woken, then take the image. A wake that finds
    /// nothing to take sends the loop back to waiting, as the app does.
    fn next_image(rig: &mut Rig) -> u32 {
        loop {
            rig.woken.recv_timeout(PATIENCE).expect("the event loop was woken");
            if let Some(image) = rig.presenter.take() {
                return image;
            }
        }
    }

    /// The point of the worker: asking for the next image returns while the display is still being
    /// waited for, so whatever the event loop is sent meanwhile -- a key -- is handled as it comes and
    /// stamped with the instant it came, not the instant the display let go.
    #[test]
    fn asking_for_an_image_does_not_wait_for_the_display() {
        let mut rig = rig();
        let (asked, returned) = mpsc::channel();
        let mut presenter = rig.presenter;
        let asking = thread::spawn(move || {
            presenter.request();
            asked.send(()).ok();
            presenter
        });
        returned.recv_timeout(PATIENCE).expect("asking for an image returned while the display was held");
        rig.presenter = asking.join().expect("the asking thread");

        assert!(rig.presenter.take().is_none(), "the image came before the display let it");
        rig.vblank.send(()).expect("the display is there");
        assert_eq!(next_image(&mut rig), 1, "the image comes once the display lets it");
    }

    /// A frame's image goes back to the worker to be shown, and the next is asked for after it: one
    /// image out at a time, shown in the order they were drawn.
    #[test]
    fn images_are_handed_out_one_at_a_time_and_shown_in_order() {
        let mut rig = rig();
        for expected in 1..=3 {
            rig.presenter.request();
            rig.presenter.request();
            rig.vblank.send(()).expect("the display is there");
            let image = next_image(&mut rig);
            assert_eq!(image, expected, "a second request while one is out asked for nothing");
            rig.presenter.present(image);
            assert_eq!(rig.shown.recv_timeout(PATIENCE).expect("the image was shown"), expected);
        }
        assert!(rig.presenter.take().is_none(), "nothing is out once every image was taken");
    }

    /// Whether the surface may be reconfigured now: not while an image is being asked for, and again
    /// once it has been taken, so the event loop never has to wait for one to let it go.
    #[test]
    fn an_image_asked_for_is_waited_on_until_it_is_taken() {
        let mut rig = rig();
        assert!(!rig.presenter.waiting(), "nothing is asked for before the first request");
        rig.presenter.request();
        assert!(rig.presenter.waiting(), "an image is out while the display holds it");
        rig.vblank.send(()).expect("the display is there");
        assert_eq!(next_image(&mut rig), 1, "the image the display let go of");
        assert!(!rig.presenter.waiting(), "and no longer once it is taken");
        rig.presenter.request();
        rig.vblank.send(()).expect("the display is there");
        assert_eq!(next_image(&mut rig), 2, "the next request is a fresh image");
    }

    /// An app shutting down has the worker let go of the surface before it does, so the surface is
    /// let go of last on the event loop's thread.
    #[test]
    fn stopping_waits_for_the_worker_to_let_go_of_the_surface() {
        let mut rig = rig();
        rig.presenter.request();
        rig.vblank.send(()).expect("the display is there");
        let image = next_image(&mut rig);
        rig.presenter.present(image);
        rig.presenter.stop(PATIENCE);
        assert_eq!(Arc::strong_count(&rig.display), 1, "the worker let go of the surface before stop returned");
    }

    /// A display that never answers holds up an app shutting down for no longer than the patience
    /// it is given.
    #[test]
    fn stopping_gives_up_on_a_display_that_never_answers() {
        let mut rig = rig();
        rig.presenter.request();
        let (stopped, done) = mpsc::channel();
        let mut presenter = rig.presenter;
        thread::spawn(move || {
            presenter.stop(Duration::ZERO);
            stopped.send(()).ok();
        });
        done.recv_timeout(PATIENCE).expect("stop returned with the display still holding the image");
        drop(rig.vblank);
    }

    /// A surface whose every image is shown by panicking: a worker that dies.
    struct BrokenDisplay;

    impl Swapchain for BrokenDisplay {
        type Acquired = u32;
        type Frame = u32;

        fn acquire(&self) -> u32 {
            0
        }

        fn present(&self, _: u32) {
            panic!("the display broke");
        }
    }

    /// A worker that panics wakes the event loop on its way out, and the next thing the event loop
    /// asks of it panics there, rather than the window silently never drawing again.
    #[test]
    fn a_worker_that_panics_wakes_the_event_loop_and_fails_it() {
        let (wake, woken) = mpsc::channel();
        let mut presenter = Presenter::start(Arc::new(BrokenDisplay), move || {
            wake.send(()).ok();
        })
        .expect("the worker starts");
        presenter.request();
        woken.recv_timeout(PATIENCE).expect("the event loop was woken for the image");
        let image = presenter.take().expect("the image came");
        presenter.present(image);
        woken.recv_timeout(PATIENCE).expect("the event loop was woken by the worker dying");
        let asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| presenter.request()));
        assert!(asked.is_err(), "asking a dead worker for an image fails the event loop");
    }

    /// An app closing while the display has not answered does not wait for it.
    #[test]
    fn dropping_the_presenter_does_not_wait_for_a_display_that_never_answers() {
        let mut rig = rig();
        rig.presenter.request();
        let (dropped, done) = mpsc::channel();
        let presenter = rig.presenter;
        thread::spawn(move || {
            drop(presenter);
            dropped.send(()).ok();
        });
        done.recv_timeout(PATIENCE).expect("the presenter was dropped with an image still asked for");
        drop(rig.vblank);
    }
}

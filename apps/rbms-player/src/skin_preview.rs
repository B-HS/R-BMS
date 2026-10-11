//! The skin preview: one screen's skin document, drawn with sample data inside a rectangle of
//! another screen.
//!
//! The skin menu ([`crate::stage::SkinMenuState`]) lists the skin settings on its left and shows
//! what they do on its right. This module is the right half. [`SkinPreview`] is the whole of what
//! the menu sees of it: the menu says when a frame begins, where the preview goes and when a setting
//! moved, and is told what is in the rectangle -- a frame of the skin, or why there is none
//! ([`PreviewStatus`]).
//!
//! # What is previewed
//!
//! One of two things, which the preview is made for and keeps ([`PreviewTarget`]).
//!
//! The skin menu's preview ([`SkinPreview::new`]) shows the screen whose skin the menu is
//! configuring, which is the one stored value the SCREEN row has always moved
//! (`config.skin.screen`). The preview reads it rather than being handed it, so the rows on the left
//! and the frame on the right cannot be about two different screens.
//!
//! The play menu's preview ([`SkinPreview::of_play`]) shows the play screen and the tempo the menu
//! names ([`PlayTarget`]), whatever `config.skin.screen` holds, and is told when the menu names
//! another ([`SkinPreview::show_play`]). Neither the mode nor the tempo is a setting: they choose the
//! sample the play options are shown on, and nothing of them is stored. The sample chart of such a
//! preview holds its one tempo throughout, so the figures the menu writes beside it
//! ([`PlayTarget::readout`]) are the ones the run in the rectangle is drawn at.
//!
//! # What the preview owns, and what it borrows
//!
//! A preview is a scene like any other screen's. It is drawn with the application's own skin
//! machinery -- the library that reads documents ([`crate::skin_select::SkinLibrary`]), the compiled
//! screens and their textures and movies (`crate::skin_screen`), the timer table and the scene clock
//! -- under the scene the skin menu was opened into. It owns only what a real screen would have
//! owned: the sample state the document is answered from, and the moment of it that is on show.
//!
//! The settings themselves are not the preview's. The menu changes them through the same calls the
//! settings screen always made, asks the library to read the document again where a change calls for
//! it, and then says that something moved ([`SkinPreview::settings_changed`]). What moved is read
//! back from where it is stored: the library says whether a document was read again, and an offset
//! is read out of the stored choices by every frame.
//!
//! # Calls, in order
//!
//! | When | Call | Who |
//! | --- | --- | --- |
//! | once a frame, from the menu's `update` | [`SkinPreview::update`] | menu |
//! | once a frame, from the menu's `draw`, after its own clear | [`SkinPreview::draw`] | menu |
//! | after a setting was changed and the library was told | [`SkinPreview::settings_changed`] | menu |
//! | when the play menu names another mode or tempo | [`SkinPreview::show_play`] | play menu |
//! | from the menu's `on_exit` | [`SkinPreview::release`] | menu |
//!
//! `update` and `draw` may be called on a preview that was released: it begins again.
//!
//! # The scene and its clock
//!
//! A preview counts its own time: the seconds its `update` is handed, added up for as long as a
//! frame of the skin was drawn and not otherwise. So the scene stands at its first moment until its
//! document can be drawn, stands still again while a setting has the document read again, and never
//! moves faster than the frames that show it. The scene's timers are stamped on that count and a
//! frame is prepared at it; the application's own scene clock is put where the count is, for
//! whatever else reads it.
//!
//! A scene that has a length -- a chart that ends, a result that fades -- begins again when it is
//! over, as a scene: its timers are cleared and a Lua skin is read again, as it is each time the
//! screen it draws is entered.
//!
//! The play menu's preview is the exception, because it is looked at while a value is being moved
//! and a scene that began again would take the notes away for as long as its skin takes to come
//! back. Its sample run has no end: a moment after the chart's last note a run of it is begun afresh
//! under the same scene ([`SampleEnding::BeginsAgain`]). Another tempo named by the menu and another
//! gauge named by the settings are taken up the same way, part of the way through the chart, so the
//! frame after either is a frame of the same scene with notes in it.
//!
//! # Waiting in the dark
//!
//! A document that is read again while a frame of it was on show -- its scene began again, or an
//! option of it moved -- is a few frames away from being drawn again. Writing that it is loading for
//! those frames would flash a panel over what the player is looking at, so the menu is told to leave
//! the rectangle dark instead ([`SkinPreview::waits_in_the_dark`]), which is what a scene that ended
//! faded to. A wait that goes on is said after all.
//!
//! # The screen under the menu
//!
//! The menu is opened over another screen and closed again, and what that screen was compiled with
//! is parked while it is. A preview begins its scenes without disturbing that
//! ([`AppShared::begin_skin_preview_scene`]), so the screen gone back to is drawn on the first frame
//! back. The one exception is that screen's own skin being previewed: a Lua skin is then read
//! against the sample state, and is read again by its own screen once the preview has let go of it.
//!
//! # Nothing of the player's own
//!
//! The sample state is invented: no chart, score or name is read from the library, the score book or
//! the account. Nothing a previewed skin asks to hear is sounded, and nothing it tells the game --
//! an event, a value written back -- is carried out.

pub(crate) mod region;
pub(crate) mod target;

mod decide;

use rbms_config::Config;
use rbms_render::Rect;
use rbms_skin::dst::{OffsetSource, SkinOffset};
use rbms_skin::loader::{SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_RESULT, is_supported_skin_type};
use rbms_skin::property::generated::OFFSET_ALL;

use crate::AppShared;
use crate::skin_screen::preview::{PreviewDrawn, PreviewFrame};
use crate::stage::play::skin_preview::{PlaySample, SampleEnding, SampleTempo};
use crate::stage::result::skin_preview::{CourseSample, ResultSample};
use crate::stage::select::skin_preview::SelectSample;
use crate::stage::{Canvas, FrameCtx};
use decide::DecideSample;
pub(crate) use target::{PlayTarget, PreviewTarget};

/// What the preview says while a document is on its way.
const LOADING_MESSAGE: &str = "LOADING THE SKIN";

/// What the preview says of a screen no document is chosen for.
const NO_DOCUMENT_MESSAGE: &str = "NO SKIN DOCUMENT FOR THIS SCREEN";

/// What the preview adds about such a screen: what draws it instead.
const NO_DOCUMENT_DETAIL: &str = "THE BUILT-IN SCREEN DRAWS IT";

/// What the preview says of a screen this build draws no skin for.
const UNSUPPORTED_MESSAGE: &str = "THIS SCREEN IS NOT DRAWN BY A SKIN";

/// What the preview adds about such a screen.
const UNSUPPORTED_DETAIL: &str = "ITS DOCUMENT IS LISTED BUT THE BUILT-IN SCREEN DRAWS IT";

/// What the preview says of a screen it has no sample state for.
const NO_PREVIEW_MESSAGE: &str = "NO PREVIEW FOR THIS SCREEN";

/// What the preview adds about such a screen.
const NO_PREVIEW_DETAIL: &str = "THE SETTINGS ON THE LEFT STILL APPLY TO IT";

/// What the preview says of a document that could not be read.
const FAILED_MESSAGE: &str = "THE SKIN DID NOT LOAD";

/// How long a wait for a document that was on show is left dark before it is said, in seconds of
/// the frames that went by.
const QUIET_WAIT_SECONDS: f32 = 0.5;

/// Why the preview's rectangle holds no frame of a skin and none is on its way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewAbsence {
    /// No document is chosen for the screen, by hand or by the pack in effect: the built-in screen
    /// draws it. This is also a play screen's answer in a run started with a layout for the built-in
    /// note field.
    NoDocument,
    /// This build draws no skin for the screen type, whatever document declares it.
    UnsupportedScreen,
    /// The screen is drawn by a skin, but the preview has no sample state to draw it from.
    NoPreview,
}

/// What is in the preview's rectangle after a [`SkinPreview::draw`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreviewStatus {
    /// The document is being read, or was read and is having its files decoded. Nothing was drawn.
    Loading,
    /// A frame of the document was drawn into the rectangle.
    Drawn,
    /// Nothing was drawn and nothing is coming until a setting moves.
    Absent(PreviewAbsence),
    /// No document is chosen for the screen, and the pack in effect may be why: its folder is not
    /// on the disk, or a Lua skin in it could not be read far enough to say which screen it draws.
    /// What is wrong, as the library says it ([`crate::skin_select::SkinLibrary::pack_trouble`]).
    /// Nothing was drawn.
    PackTrouble(String),
    /// The chosen document could not be read: why, on one line that begins with the file that
    /// failed. Nothing was drawn.
    Failed(String),
}

impl PreviewStatus {
    /// Whether a frame of the skin is in the rectangle, which is when the menu draws nothing of its
    /// own over it.
    pub(crate) fn is_drawn(&self) -> bool {
        *self == PreviewStatus::Drawn
    }

    /// What the menu writes in the rectangle in place of a frame: one line that says what happened.
    /// `None` when a frame was drawn.
    pub(crate) fn message(&self) -> Option<&'static str> {
        match self {
            PreviewStatus::Drawn => None,
            PreviewStatus::Loading => Some(LOADING_MESSAGE),
            PreviewStatus::Absent(PreviewAbsence::NoDocument) => Some(NO_DOCUMENT_MESSAGE),
            PreviewStatus::Absent(PreviewAbsence::UnsupportedScreen) => Some(UNSUPPORTED_MESSAGE),
            PreviewStatus::Absent(PreviewAbsence::NoPreview) => Some(NO_PREVIEW_MESSAGE),
            PreviewStatus::PackTrouble(_) => Some(NO_DOCUMENT_MESSAGE),
            PreviewStatus::Failed(_) => Some(FAILED_MESSAGE),
        }
    }

    /// What the menu writes under [`PreviewStatus::message`]: what follows from it, or the reason a
    /// document did not load. `None` when there is nothing to add.
    pub(crate) fn detail(&self) -> Option<&str> {
        match self {
            PreviewStatus::Drawn | PreviewStatus::Loading => None,
            PreviewStatus::Absent(PreviewAbsence::NoDocument) => Some(NO_DOCUMENT_DETAIL),
            PreviewStatus::Absent(PreviewAbsence::UnsupportedScreen) => Some(UNSUPPORTED_DETAIL),
            PreviewStatus::Absent(PreviewAbsence::NoPreview) => Some(NO_PREVIEW_DETAIL),
            PreviewStatus::PackTrouble(reason) | PreviewStatus::Failed(reason) => Some(reason),
        }
    }
}

/// The longest one frame moves a preview's scene on by, in seconds. A frame that took longer -- the
/// one a skin was read on -- moves it by this much and no more, so a sample run is never handed a
/// stretch of its chart whole.
const LONGEST_STEP_SECONDS: f32 = 0.1;

const MICROS_PER_SECOND: f64 = 1_000_000.0;

/// Where a scene's own time begins.
const SCENE_START_US: i64 = 0;

/// What a step of a sample's scene came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SampleStep {
    /// The scene goes on.
    Running,
    /// The scene has run its length: the preview begins it again.
    Over,
}

/// The sample state one screen is previewed from.
enum Sample {
    Select(Box<SelectSample>),
    Decide(Box<DecideSample>),
    Play(Box<PlaySample>),
    Result(Box<ResultSample>),
    CourseResult(Box<CourseSample>),
}

impl Sample {
    /// The sample state of `screen` at its first moment, or `None` for a screen the preview has
    /// none for: the key configuration, and any play screen no sample chart is written for. A play
    /// screen's sample chart is written at `tempo` and its run ends as `ending` says; no other
    /// screen's reads either.
    fn of(screen: i32, config: &Config, tempo: SampleTempo, ending: SampleEnding) -> Option<Sample> {
        match screen {
            SKIN_TYPE_MUSIC_SELECT => Some(Sample::Select(Box::new(SelectSample::new()))),
            SKIN_TYPE_DECIDE => DecideSample::new().map(|sample| Sample::Decide(Box::new(sample))),
            SKIN_TYPE_RESULT => ResultSample::new(config).map(|sample| Sample::Result(Box::new(sample))),
            SKIN_TYPE_COURSE_RESULT => CourseSample::new(config).map(|sample| Sample::CourseResult(Box::new(sample))),
            play => PlaySample::at_tempo(play, config, tempo).map(|sample| Sample::Play(Box::new(sample.ending(ending)))),
        }
    }

    /// Have the sample go on at `tempo` under the scene as it stands. `false` when it cannot and a
    /// scene has to be begun for the tempo: a run that closes, and every screen that is no play
    /// screen.
    fn show_tempo(&mut self, tempo: SampleTempo) -> bool {
        match self {
            Sample::Play(sample) => sample.show_tempo(tempo),
            Sample::Select(_) | Sample::Decide(_) | Sample::Result(_) | Sample::CourseResult(_) => false,
        }
    }

    /// Move the sample's scene on to `now_us`, switching the scene's timers as its screen would.
    fn step(&mut self, shared: &mut AppShared, now_us: i64, ready: bool) -> SampleStep {
        match self {
            Sample::Select(sample) => sample.step(shared, now_us, ready),
            Sample::Decide(sample) => sample.step(shared, now_us, ready),
            Sample::Play(sample) => sample.step(shared, now_us, ready),
            Sample::Result(sample) => sample.step(shared, now_us, ready),
            Sample::CourseResult(sample) => sample.step(shared, now_us, ready),
        }
    }

    /// Draw one frame of the sample's screen.
    fn draw(&mut self, shared: &AppShared, canvas: &mut Canvas<'_>, at: PreviewFrame) -> PreviewDrawn {
        match self {
            Sample::Select(sample) => sample.draw(shared, canvas, at),
            Sample::Decide(sample) => sample.draw(shared, canvas, at),
            Sample::Play(sample) => sample.draw(shared, canvas, at),
            Sample::Result(sample) => sample.draw(shared, canvas, at),
            Sample::CourseResult(sample) => sample.draw(shared, canvas, at),
        }
    }
}

/// The offset of the whole screen stored for the document `screen` is drawn with, which a compiled
/// screen reads on the first frame it draws and keeps.
fn stored_whole_offset(shared: &AppShared, screen: i32) -> Option<SkinOffset> {
    let document = shared.skins.document_path(&shared.config, screen)?;
    shared.config.skin.customisation(document)?.offset(OFFSET_ALL)
}

/// How the document of the previewed screen stands with the library, as far as a preview has to do
/// something about it: which read of it is held, whether a newer read has been asked for and is
/// waiting, and whether a choice moved that a read has yet to take in.
///
/// A preview keeps the one it last saw. A setting that leaves it as it was -- a nudge of an offset,
/// which every frame reads where it is stored -- asks nothing of the preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DocumentState {
    build: Option<u64>,
    waiting: bool,
    stale: bool,
}

impl DocumentState {
    fn of(shared: &AppShared, screen: i32) -> DocumentState {
        let skins = &shared.skins;
        DocumentState { build: skins.build_of(screen), waiting: skins.is_waiting(screen), stale: skins.needs_reload_for(&shared.config, screen) }
    }
}

/// A reason as one line a screen can write: its first line, with any character a line cannot be
/// written with made a space.
fn one_line(reason: &str) -> String {
    let line = reason.lines().next().unwrap_or_default();
    line.chars().map(|ch| if ch.is_control() { ' ' } else { ch }).collect::<String>().trim().to_owned()
}

/// The preview of one screen's skin. See the module's own notes for who calls what, and when.
pub(crate) struct SkinPreview {
    /// What the preview shows: the screen the skin menu is configuring, or the play screen and the
    /// tempo the play menu names. It outlives a [`SkinPreview::release`].
    target: PreviewTarget,
    /// The screen the preview was last brought up to date for, or `None` before its first
    /// [`SkinPreview::update`] and after a [`SkinPreview::release`].
    screen: Option<i32>,
    /// The tempo the sample chart of the scene on show was written at.
    tempo: SampleTempo,
    /// What the last [`SkinPreview::draw`] left in its rectangle.
    status: PreviewStatus,
    /// The sample state the screen is drawn from, when the preview has one for it and the screen
    /// has a document to draw.
    sample: Option<Sample>,
    /// The scene's own time, in microseconds.
    now_us: i64,
    /// The document the scene was begun for, as the library named it then.
    document: Option<String>,
    /// How the document stood when the preview last looked: on its last frame, or when it was last
    /// told a setting moved.
    seen: Option<DocumentState>,
    /// The offset of the whole screen the compiled screen settled on, once it has drawn a frame.
    whole: Option<Option<SkinOffset>>,
    /// Whether the document's being on its way has been answered for one frame since it was last
    /// asked for. The frame after is the one that reads it.
    announced: bool,
    /// Whether a frame of the document the scene was begun for has been in the rectangle, and
    /// nothing has been said in its place since.
    on_show: bool,
    /// How long the preview has been answering that its document is on its way, in seconds of the
    /// frames it was brought up to date for.
    waited_seconds: f32,
}

impl Default for SkinPreview {
    fn default() -> SkinPreview {
        SkinPreview::new()
    }
}

impl SkinPreview {
    /// A preview of the screen the skin menu is configuring ([`PreviewTarget::Configured`]), which
    /// its first [`SkinPreview::update`] reads from the settings, as every one after it does.
    pub(crate) fn new() -> SkinPreview {
        SkinPreview::of(PreviewTarget::Configured)
    }

    /// A preview of the play screen and the tempo `target` names ([`PreviewTarget::Play`]). It
    /// never reads `config.skin.screen`, and shows `target` until it is handed another
    /// ([`SkinPreview::show_play`]).
    pub(crate) fn of_play(target: PlayTarget) -> SkinPreview {
        SkinPreview::of(PreviewTarget::Play(target))
    }

    /// A preview of `target` that has read nothing yet.
    fn of(target: PreviewTarget) -> SkinPreview {
        SkinPreview {
            target,
            screen: None,
            tempo: SampleTempo::Shipped,
            status: PreviewStatus::Loading,
            sample: None,
            now_us: SCENE_START_US,
            document: None,
            seen: None,
            whole: None,
            announced: false,
            on_show: false,
            waited_seconds: 0.0,
        }
    }

    /// What the preview shows.
    pub(crate) fn target(&self) -> PreviewTarget {
        self.target
    }

    /// Show another play screen, or the same one at another tempo.
    ///
    /// The preview is of `target` from the [`SkinPreview::update`] that follows. A screen that is
    /// not the one on show begins a scene of its own there, and what was read for the screen shown
    /// before is let go of as it is when the skin menu's SCREEN row moves. Another tempo for the
    /// screen on show does not: the sample run is begun afresh on a chart written at it, as far
    /// through as the run had come, under the scene as it stands, so the frame that follows is drawn
    /// and nothing is read again. Naming the target that is on show does nothing.
    ///
    /// A preview made for the skin menu becomes a preview of `target` and stops following
    /// `config.skin.screen`.
    pub(crate) fn show_play(&mut self, target: PlayTarget) {
        self.target = PreviewTarget::Play(target);
    }

    /// The screen the preview is to show on this frame, and the tempo its sample chart is written
    /// at.
    fn wanted(&self, config: &Config) -> (i32, SampleTempo) {
        match self.target {
            PreviewTarget::Configured => (config.skin.screen, SampleTempo::Shipped),
            PreviewTarget::Play(play) => (play.screen, SampleTempo::Steady(play.bpm)),
        }
    }

    /// How the sample run of a play screen ends: the skin menu's closes, so every state a play skin
    /// draws comes round, and the play menu's begins again, so notes keep coming down while a value
    /// is moved.
    fn ending(&self) -> SampleEnding {
        match self.target {
            PreviewTarget::Configured => SampleEnding::Closes,
            PreviewTarget::Play(_) => SampleEnding::BeginsAgain,
        }
    }

    /// Whether the scene on show goes on at `tempo`: it is the tempo the scene has, or the sample
    /// takes it up where it stands ([`Sample::show_tempo`]). A screen with nothing to draw has no
    /// chart to be at a tempo, and goes on saying so.
    fn takes_tempo(&mut self, tempo: SampleTempo) -> bool {
        let taken = self.tempo == tempo || self.sample.as_mut().is_none_or(|sample| sample.show_tempo(tempo));
        if taken {
            self.tempo = tempo;
        }
        taken
    }

    /// Begin the scene of `screen` at its first moment: every timer off, the clocks at nothing, the
    /// Lua skin read for the screen due to be read again, and the sample state made afresh with a
    /// play screen's chart written at `tempo`. A screen with no document, or one this build draws no
    /// skin for, is given no sample state.
    ///
    /// What was read for the screen previewed before, when it was another, is let go of. A screen
    /// that stays compiled into the new scene -- a document that is only data is not read again --
    /// settles the offset of the whole screen afresh on the scene's first frame, as a screen compiled
    /// for the scene would.
    fn begin_scene(&mut self, shared: &mut AppShared, screen: i32, tempo: SampleTempo) {
        if let Some(previous) = self.screen.filter(|previous| *previous != screen) {
            shared.end_skin_preview_of(previous);
        }
        shared.begin_skin_preview_scene(screen);
        shared.resettle_skin_preview_whole_offset(screen);
        self.screen = Some(screen);
        self.tempo = tempo;
        self.now_us = SCENE_START_US;
        self.status = PreviewStatus::Loading;
        self.document = shared.skins.document_path(&shared.config, screen).map(str::to_owned);
        self.seen = None;
        self.whole = None;
        let drawable = self.document.is_some() && is_supported_skin_type(screen);
        self.sample = if drawable { Sample::of(screen, &shared.config, tempo, self.ending()) } else { None };
        shared.set_skin_scene_clock(self.now_us);
    }

    /// Whether the document the scene was begun for is still the one its screen is drawn with.
    fn document_stands(&self, shared: &AppShared, screen: i32) -> bool {
        self.document.as_deref() == shared.skins.document_path(&shared.config, screen)
    }

    /// Bring the preview up to this frame, before it is drawn.
    ///
    /// The screen previewed is the one the preview's target names: `config.skin.screen`, or the
    /// play screen the play menu chose ([`PreviewTarget`]). When it is not the one the preview was
    /// last brought up to date for, the preview begins again for it: a new scene, with the sample
    /// state at its first moment, and whatever was compiled or playing for the screen before is let
    /// go of by the frame that follows. So it does when the screen is drawn with another document
    /// than the one its scene was begun for. Otherwise the sample state moves on by `ctx.dt`
    /// seconds; a scene that is waiting for its document does not move. Another tempo named by the
    /// play menu is handed to the sample run, which takes it up under the scene as it stands
    /// ([`SkinPreview::takes_tempo`]).
    ///
    /// Called once a frame from the menu's `update`, whether or not anything changed.
    pub(crate) fn update(&mut self, ctx: &mut FrameCtx<'_>) {
        let shared = &mut *ctx.shared;
        let (screen, tempo) = self.wanted(&shared.config);
        let step = ctx.dt.clamp(0.0, LONGEST_STEP_SECONDS);
        self.waited_seconds = if self.status == PreviewStatus::Loading { self.waited_seconds + step } else { 0.0 };
        if self.screen != Some(screen) || !self.document_stands(shared, screen) || !self.takes_tempo(tempo) {
            self.announced = false;
            self.on_show = false;
            self.begin_scene(shared, screen, tempo);
        } else if self.status.is_drawn() {
            self.now_us += (f64::from(step) * MICROS_PER_SECOND) as i64;
        }
        shared.set_skin_scene_clock(self.now_us);
        let ready = shared.skin_preview_is_ready(screen);
        let over = self.sample.as_mut().is_some_and(|sample| sample.step(shared, self.now_us, ready) == SampleStep::Over);
        if over {
            self.begin_scene(shared, screen, tempo);
        }
    }

    /// Draw one frame of the previewed screen into `rect` and answer what is there.
    ///
    /// `rect` is in the fixed space the built-in screens are laid out in
    /// ([`crate::stage::canvas::UI_SIZE`]). The frame is drawn on the target's own pixels, laid out
    /// for the rectangle as it lands there and clipped to it ([`region::with_region`]): nothing
    /// outside `rect` is drawn on, and what the menu drew there before is not disturbed. Keeping
    /// `rect` the shape of the screen is the menu's business; the document is stretched onto
    /// whatever it is handed.
    ///
    /// When the answer is not [`PreviewStatus::Drawn`] nothing was drawn at all, and the rectangle
    /// is the menu's to fill and write the status in ([`PreviewStatus::message`]).
    ///
    /// Called once a frame from the menu's `draw`, after the menu has cleared the frame. Reading and
    /// compiling the document happens here, a step a frame, so the first frames of a document answer
    /// [`PreviewStatus::Loading`]. A document that fails does so once: the answer stays
    /// [`PreviewStatus::Failed`] until a setting moves.
    pub(crate) fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>, rect: Rect) -> &PreviewStatus {
        let shared = &mut *ctx.shared;
        let screen = match self.screen {
            Some(screen) => screen,
            None => {
                let (screen, tempo) = self.wanted(&shared.config);
                self.on_show = false;
                self.begin_scene(shared, screen, tempo);
                screen
            }
        };
        self.status = self.frame(shared, canvas, screen, rect);
        self.on_show = match self.status {
            PreviewStatus::Drawn => true,
            PreviewStatus::Loading => self.on_show,
            PreviewStatus::Absent(_) | PreviewStatus::PackTrouble(_) | PreviewStatus::Failed(_) => false,
        };
        &self.status
    }

    /// One frame of `screen`: what a preview with nothing to draw says of it, read from what is
    /// chosen for the screen with no document read to say it, or else the next step of its document
    /// towards being drawn and, once it is compiled, the frame itself.
    ///
    /// The first frame after a document was asked for reads nothing and answers that it is on its
    /// way, so the menu has said so by the time the frame that reads it -- which for a Lua skin runs
    /// the whole skin -- holds the frame loop up. The screen compiled for the scene underneath is
    /// kept through that frame ([`AppShared::keep_skin_preview`]).
    ///
    /// The reason of a document that failed is the first line of the one the LOADED row shows, cut
    /// down to what there is room for in the rectangle ([`SkinPreview::failure`]).
    fn frame(&mut self, shared: &mut AppShared, canvas: &mut Canvas<'_>, screen: i32, rect: Rect) -> PreviewStatus {
        let status = self.step_document(shared, canvas, screen, rect);
        self.seen = Some(DocumentState::of(shared, screen));
        status
    }

    /// The frame itself, for [`SkinPreview::frame`], which notes how the document stands afterwards.
    fn step_document(&mut self, shared: &mut AppShared, canvas: &mut Canvas<'_>, screen: i32, rect: Rect) -> PreviewStatus {
        if shared.skins.document_path(&shared.config, screen).is_none() {
            return match shared.skins.pack_trouble(&shared.config, screen) {
                Some(trouble) => PreviewStatus::PackTrouble(trouble),
                None => PreviewStatus::Absent(PreviewAbsence::NoDocument),
            };
        }
        if !is_supported_skin_type(screen) {
            return PreviewStatus::Absent(PreviewAbsence::UnsupportedScreen);
        }
        let Some(sample) = self.sample.as_mut() else {
            return SkinPreview::failure(shared, screen).unwrap_or(PreviewStatus::Absent(PreviewAbsence::NoPreview));
        };
        if !std::mem::replace(&mut self.announced, true) {
            shared.keep_skin_preview(screen);
            return PreviewStatus::Loading;
        }
        let whole = stored_whole_offset(shared, screen);
        if self.whole.is_some_and(|settled| settled != whole) {
            shared.resettle_skin_preview_whole_offset(screen);
        }
        shared.prepare_skin_preview(canvas, screen);
        if let Some(failed) = SkinPreview::failure(shared, screen) {
            return failed;
        }
        match sample.draw(shared, canvas, PreviewFrame { screen, now_us: self.now_us, rect }) {
            PreviewDrawn::Drawn => {
                self.whole = Some(whole);
                PreviewStatus::Drawn
            }
            PreviewDrawn::Waiting => PreviewStatus::Loading,
            PreviewDrawn::Unbound(reason) => PreviewStatus::Failed(one_line(&reason)),
        }
    }

    /// What is said of `screen`'s document when it was read and could not be: the reason on one
    /// line, beginning with the file that failed and with the folder it is in left out, so the part
    /// that says what is wrong is not what a narrow place cuts off. The LOADED row has the line
    /// whole. `None` for a document that did not fail.
    fn failure(shared: &AppShared, screen: i32) -> Option<PreviewStatus> {
        shared.skins.failure_line(&shared.config, screen).map(PreviewStatus::Failed)
    }

    /// Whether the rectangle is to be left dark rather than written in: the preview is waiting for a
    /// document that was on show a moment ago -- its scene began again, or a choice had it read
    /// again -- and has not been waiting long. A panel saying so would be up for a frame or two,
    /// over what was a frame of the skin and is about to be one again.
    pub(crate) fn waits_in_the_dark(&self) -> bool {
        self.status == PreviewStatus::Loading && self.on_show && self.waited_seconds < QUIET_WAIT_SECONDS
    }

    /// What the last [`SkinPreview::draw`] left in its rectangle.
    pub(crate) fn status(&self) -> &PreviewStatus {
        &self.status
    }

    /// Take in that the menu changed a setting: any row of its list, the skin menu's screen
    /// included. The play menu calls it after any of its rows moved, a play option as much as a skin
    /// option of the previewed document: the option is read again like any other, and a play option
    /// asks nothing here because the sample run reads the play options where they are stored on
    /// every frame. A mode or a tempo of the play menu is not a setting and is not told here
    /// ([`SkinPreview::show_play`]).
    ///
    /// The menu calls this after it has changed the stored choice and, where the change calls for
    /// the document to be read again -- an option, a file slot, the document, the pack folder, a
    /// reload, a reset -- after it has asked the library to (`AppShared::reload_skin` and the calls
    /// beside it). The preview does not write settings and does not ask the library for what the
    /// menu already asked for; it reads back what moved.
    ///
    /// A nudge of an offset is drawn with from the frame that follows, with nothing read again: the
    /// offset of the whole screen too, which a compiled screen settles once and is told to settle
    /// again. A document that is read again answers [`PreviewStatus::Loading`] until it is compiled,
    /// and its scene carries on from where it stood. A screen that is drawn with another document,
    /// or another screen altogether, begins its scene with the next [`SkinPreview::update`].
    ///
    /// What moved is told by how the document stands now against how it stood when the preview last
    /// looked ([`DocumentState`]), so being told twice of one change, or of a change that asks
    /// nothing of the preview, does nothing.
    pub(crate) fn settings_changed(&mut self, shared: &mut AppShared) {
        let wanted = self.wanted(&shared.config).0;
        let Some(screen) = self.screen.filter(|screen| *screen == wanted) else {
            return;
        };
        if !self.document_stands(shared, screen) {
            return;
        }
        let stands = DocumentState::of(shared, screen);
        if self.seen.replace(stands) != Some(stands) {
            self.announced = self.on_show;
            self.waited_seconds = 0.0;
            self.status = PreviewStatus::Loading;
        }
    }

    /// Let go of everything the preview holds: its sample state and the interpreter of a Lua skin
    /// read for it at once, and the compiled screen, its textures and its movies by the frame that
    /// follows, as for any screen that is left. The skin is read again by the screen it really
    /// draws, against that screen's own state.
    ///
    /// Called from the menu's `on_exit`. Calling it twice, or on a preview that never drew, does
    /// nothing more.
    pub(crate) fn release(&mut self, shared: &mut AppShared) {
        if let Some(screen) = self.screen.take() {
            shared.end_skin_preview_of(screen);
        }
        *self = SkinPreview::of(self.target);
    }

    /// The sample run on show, when the screen previewed is a play screen.
    #[cfg(test)]
    fn play_sample(&self) -> Option<&PlaySample> {
        match self.sample.as_ref()? {
            Sample::Play(sample) => Some(sample),
            _ => None,
        }
    }

    /// The sample list on show, when the screen previewed is the song browser.
    #[cfg(test)]
    fn select_sample(&self) -> Option<&SelectSample> {
        match self.sample.as_ref()? {
            Sample::Select(sample) => Some(sample),
            _ => None,
        }
    }

    /// The scene's own time, in microseconds.
    #[cfg(test)]
    fn scene_us(&self) -> i64 {
        self.now_us
    }
}

#[cfg(test)]
mod tests;

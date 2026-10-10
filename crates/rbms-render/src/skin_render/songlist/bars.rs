//! The browser's bars as a song wheel reads them, and the rules that turn one bar into the pieces a
//! slot draws.
//!
//! Everything here is the reference's `BarRenderer` with the game taken out of it: a bar is what the
//! reference's `Bar` subclasses answer when the renderer asks them, written down once by the browser
//! so that drawing a frame asks nothing. Which image a bar is cut from, which of the eleven texts
//! names it, which lamp, level, trophy and labels ride on it, where the ring puts it and how far a
//! scroll has carried it are all decided here, in the reference's own order and with its own
//! arithmetic.

/// How many slots the reference's wheel has room for (`SkinBar.BAR_COUNT`, `BarRenderer.barlength`).
/// A document that declares more has the rest ignored.
pub(crate) const BAR_SLOTS: usize = 60;

/// How many texts a wheel carries, one per kind of bar title (`SkinBar.BARTEXT_COUNT`).
pub(crate) const BAR_TEXTS: usize = 11;

/// How many level numbers a wheel carries, one per difficulty (`SkinBar.BARLEVEL_COUNT`).
pub(crate) const BAR_LEVELS: usize = 7;

/// How many lamps each of a wheel's three lamp lists carries (`SkinBar.BARLAMP_COUNT`).
pub(crate) const BAR_LAMPS: usize = 11;

/// How many trophies a wheel carries (`SkinBar.BARTROPHY_COUNT`).
pub(crate) const BAR_TROPHIES: usize = 3;

/// How many chart labels a wheel carries (`SkinBar.BARLABEL_COUNT`).
pub(crate) const BAR_LABELS: usize = 5;

/// How many clear lamps a folder's charts are counted into, from no play up to max
/// (`DirectoryBar.getLamps`).
pub const LAMP_KINDS: usize = 11;

/// How many score ranks a folder's charts are counted into (`DirectoryBar.getRanks`).
pub const RANK_KINDS: usize = 28;

/// The image of a wheel's bar set a chart that is on disk is cut from.
const IMAGE_SONG: usize = 0;

/// The image a folder is cut from.
const IMAGE_FOLDER: usize = 1;

/// The image a table, a hash folder, a random pick and a complete random course are cut from.
const IMAGE_TABLE: usize = 2;

/// The image a course whose charts are all on disk is cut from.
const IMAGE_COURSE: usize = 3;

/// The image a chart that is not on disk, and a course missing one, is cut from.
const IMAGE_MISSING: usize = 4;

/// The image a command and a container are cut from.
const IMAGE_COMMAND: usize = 5;

/// The image a search is cut from.
const IMAGE_SEARCH: usize = 6;

/// The text every bar falls back to (`SkinBar.BARTEXT_NORMAL`).
const TEXT_NORMAL: usize = 0;

/// The text a newly added bar falls back to (`SkinBar.BARTEXT_NEW`).
const TEXT_NEW: usize = 1;

/// The texts of a chart, as it has been there a while and as it was added within a day
/// (`BARTEXT_SONG_NORMAL`, `BARTEXT_SONG_NEW`).
const TEXT_SONG: (usize, usize) = (2, 3);

/// The same two for a folder (`BARTEXT_FOLDER_NORMAL`, `BARTEXT_FOLDER_NEW`).
const TEXT_FOLDER: (usize, usize) = (4, 5);

/// The first bar image whose text is its own number moved up by [`TEXT_PAST_IMAGE`].
const FIRST_IMAGE_WITH_ITS_OWN_TEXT: usize = IMAGE_TABLE;

/// How far past its image's number the text of such a bar is (`songstatus += 4`).
const TEXT_PAST_IMAGE: usize = 4;

/// The label drawn for a chart with long notes, and the one a chart falls back to when the label of
/// its own long note kind is missing.
pub(crate) const LABEL_LONG_NOTE: usize = 0;

/// The label drawn for a chart that picks its notes at random.
pub(crate) const LABEL_RANDOM: usize = 1;

/// The label drawn for a chart with mines.
pub(crate) const LABEL_MINE: usize = 2;

/// The labels drawn for a chart with charge notes and for one with hell charge notes.
const LABEL_CHARGE_NOTE: usize = 3;
const LABEL_HELL_CHARGE_NOTE: usize = 4;

/// The label of each long note kind, by the number the player's long note mode gives it: long note,
/// charge note, hell charge note (`lnindex = {0, 3, 4}`).
const LONG_NOTE_LABELS: [usize; 3] = [LABEL_LONG_NOTE, LABEL_CHARGE_NOTE, LABEL_HELL_CHARGE_NOTE];

/// The long note kinds, as the player's mode and a chart's features both number them.
const LONG_NOTE: i32 = 0;
const CHARGE_NOTE: i32 = 1;
const HELL_CHARGE_NOTE: i32 = 2;

/// How many times its own length the ring's arithmetic adds to stay positive
/// (`currentsongs.length * 100`).
const RING_TURNS: i64 = 100;

/// How long one slot's travel takes when a held key first moves the wheel, in milliseconds
/// (`Config.scrolldurationlow`).
pub const SCROLL_DURATION_LOW_MS: i32 = 300;

/// How long it takes each time the held key repeats (`Config.scrolldurationhigh`).
pub const SCROLL_DURATION_HIGH_MS: i32 = 50;

/// How long one notch of the wheel takes to travel when nothing is queued behind it
/// (`BarRenderer.input`, the `120`).
const WHEEL_NOTCH_MS: i32 = 120;

/// The most notches the wheel queues in either direction.
const WHEEL_QUEUE: i32 = 2;

/// What a bar of the browser's list is, as the reference tells its `Bar` classes apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarKind {
    /// A chart (`SongBar`). `exists` is whether its file is on disk (`existsSong`).
    Song { exists: bool },
    /// A folder of the library (`FolderBar`).
    Folder,
    /// A difficulty table or a folder of charts picked by hash (`TableBar`, `HashBar`).
    Table,
    /// A bar that runs something rather than opening, such as a random pick (`ExecutableBar`).
    Executable,
    /// A course (`GradeBar`). `complete` is whether every chart of it is on disk (`existsAllSongs`).
    Course { complete: bool },
    /// A course drawn at random (`RandomCourseBar`), with the same `complete`.
    RandomCourse { complete: bool },
    /// A command or a container of bars (`CommandBar`, `ContainerBar`).
    Command,
    /// What a search found (`SearchWordBar`).
    Search,
    /// Any other bar, which the reference's wheel does not draw at all (`SameFolderBar`).
    Other,
}

impl BarKind {
    /// Which image of the wheel's bar set this bar is cut from, or `None` for a bar that is not
    /// drawn (`BarRenderer.prepare`, `ba.value`).
    pub const fn image_index(self) -> Option<usize> {
        match self {
            BarKind::Table | BarKind::Executable | BarKind::RandomCourse { complete: true } => Some(IMAGE_TABLE),
            BarKind::Course { complete: true } => Some(IMAGE_COURSE),
            BarKind::Course { complete: false } | BarKind::RandomCourse { complete: false } | BarKind::Song { exists: false } => Some(IMAGE_MISSING),
            BarKind::Folder => Some(IMAGE_FOLDER),
            BarKind::Song { exists: true } => Some(IMAGE_SONG),
            BarKind::Search => Some(IMAGE_SEARCH),
            BarKind::Command => Some(IMAGE_COMMAND),
            BarKind::Other => None,
        }
    }

    /// Whether the bar opens onto other bars (`DirectoryBar`), which is what carries a distribution
    /// of clears and what a press opens rather than plays.
    pub const fn is_directory(self) -> bool {
        matches!(self, BarKind::Folder | BarKind::Table | BarKind::Command | BarKind::Search | BarKind::Other)
    }
}

/// The medal a course has been cleared to (`GradeBar.getTrophy`, matched by name against
/// `bronzemedal`, `silvermedal` and `goldmedal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarTrophy {
    Bronze,
    Silver,
    Gold,
}

impl BarTrophy {
    /// Which of the wheel's trophies this is.
    pub(crate) const fn index(self) -> usize {
        match self {
            BarTrophy::Bronze => 0,
            BarTrophy::Silver => 1,
            BarTrophy::Gold => 2,
        }
    }
}

/// How the charts under a folder have been cleared and scored (`DirectoryBar.getLamps`,
/// `getRanks`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarDistribution {
    /// How many charts sit on each clear lamp, from no play at index zero to max at ten.
    pub lamps: [u32; LAMP_KINDS],
    /// How many charts sit on each score rank: `exscore * 27 / (notes * 2)`, held to 27, and zero
    /// for a chart with no score.
    pub ranks: [u32; RANK_KINDS],
}

impl Default for BarDistribution {
    fn default() -> Self {
        BarDistribution { lamps: [0; LAMP_KINDS], ranks: [0; RANK_KINDS] }
    }
}

/// One bar of the browser's list, with everything the wheel asks a bar for.
#[derive(Debug, Clone, PartialEq)]
pub struct SongBar {
    pub kind: BarKind,
    /// What the bar is called (`Bar.getTitle`).
    pub title: String,
    /// Whether a chart or a folder was added within the last day: its record is there and the
    /// clock, in seconds, is no later than its `adddate` plus a day. No other kind of bar reads it.
    pub is_new: bool,
    /// A chart's level (`SongData.getLevel`), drawn on a chart that is on disk.
    pub level: i32,
    /// A chart's difficulty (`SongData.getDifficulty`): zero to six pick the level number of that
    /// difficulty and anything else picks the first.
    pub difficulty: i32,
    /// The clear lamp the player holds on the bar (`Bar.getLamp(true)`), zero for no play up to ten
    /// for max. A lamp the wheel has no image under draws nothing.
    pub lamp: i32,
    /// The clear lamp the rival holds on it (`Bar.getLamp(false)`), drawn only while a rival is
    /// picked ([`SongBars::rival`]).
    pub rival_lamp: i32,
    /// The medal of a course, when it has one.
    pub trophy: Option<BarTrophy>,
    /// What the chart is made of, as the reference's `SongData.getFeature` bits: the
    /// `FEATURE_` constants of this type or-ed together. A course carries the bits of all its
    /// charts. Read on a chart that is on disk and on a course that is complete.
    pub features: u32,
    /// How the charts under a bar that opens have been cleared, when that has been counted. One
    /// that has not been counted draws no graph, as a folder whose counts are all zero draws none.
    pub distribution: Option<Box<BarDistribution>>,
}

impl SongBar {
    /// A chart whose long notes take the kind the player's mode says (`FEATURE_UNDEFINEDLN`).
    pub const FEATURE_UNDEFINED_LN: u32 = 1;
    /// A chart with mines (`FEATURE_MINENOTE`).
    pub const FEATURE_MINE_NOTE: u32 = 2;
    /// A chart that picks its notes at random (`FEATURE_RANDOM`).
    pub const FEATURE_RANDOM: u32 = 4;
    /// A chart with long notes (`FEATURE_LONGNOTE`).
    pub const FEATURE_LONG_NOTE: u32 = 8;
    /// A chart with charge notes (`FEATURE_CHARGENOTE`).
    pub const FEATURE_CHARGE_NOTE: u32 = 16;
    /// A chart with hell charge notes (`FEATURE_HELLCHARGENOTE`).
    pub const FEATURE_HELL_CHARGE_NOTE: u32 = 32;

    /// A bar of `kind` called `title`, with nothing played on it and nothing else known about it.
    pub fn new(kind: BarKind, title: impl Into<String>) -> SongBar {
        SongBar { kind, title: title.into(), is_new: false, level: 0, difficulty: 0, lamp: 0, rival_lamp: 0, trophy: None, features: 0, distribution: None }
    }

    /// The features the wheel reads off this bar: a chart's own when it is on disk, a course's when
    /// every chart of it is, and none for anything else (`BarRenderer.render`, `flag`).
    fn drawn_features(&self) -> u32 {
        match self.kind {
            BarKind::Song { exists: true } | BarKind::Course { complete: true } => self.features,
            _ => 0,
        }
    }

    /// Which of the wheel's texts names this bar, given the image it is cut from and which texts
    /// the document declared (`BarRenderer.prepare`, `songstatus`).
    ///
    /// A chart and a folder pick between two texts by whether they are new and fall back to the two
    /// general ones; every other bar has one text of its own and falls back to the first.
    pub(crate) fn text_index(&self, image: usize, declared: impl Fn(usize) -> bool) -> usize {
        if image >= FIRST_IMAGE_WITH_ITS_OWN_TEXT {
            let own = image + TEXT_PAST_IMAGE;
            return if declared(own) { own } else { TEXT_NORMAL };
        }
        let (settled, new) = if image == IMAGE_SONG { TEXT_SONG } else { TEXT_FOLDER };
        match (self.is_new, declared(new), declared(settled)) {
            (true, true, _) => new,
            (true, false, _) => TEXT_NEW,
            (false, _, true) => settled,
            (false, _, false) => TEXT_NORMAL,
        }
    }

    /// Which level number a chart that is on disk draws its level with, or `None` for any other
    /// bar (`BarRenderer.render`, the level pass).
    pub(crate) fn level_index(&self) -> Option<usize> {
        if self.kind != (BarKind::Song { exists: true }) {
            return None;
        }
        Some(usize::try_from(self.difficulty).ok().filter(|difficulty| *difficulty < BAR_LEVELS).unwrap_or_default())
    }

    /// Which trophy a course draws, or `None` for a bar that is not a course or has no medal.
    pub(crate) fn trophy_index(&self) -> Option<usize> {
        matches!(self.kind, BarKind::Course { .. }).then_some(self.trophy).flatten().map(BarTrophy::index)
    }

    /// The label of the long note kind this bar's charts carry, or `None` when they carry none
    /// (`BarRenderer.render`, `ln`).
    ///
    /// A chart that leaves the kind open takes the player's mode, and a chart that names kinds takes
    /// the hardest of them. A mode the reference has no label under draws none.
    pub(crate) fn long_note_label(&self, ln_mode: i32) -> Option<usize> {
        let features = self.drawn_features();
        let has = |feature: u32| features & feature != 0;
        let mut kind = has(Self::FEATURE_UNDEFINED_LN).then_some(ln_mode);
        for (feature, named) in
            [(Self::FEATURE_LONG_NOTE, LONG_NOTE), (Self::FEATURE_CHARGE_NOTE, CHARGE_NOTE), (Self::FEATURE_HELL_CHARGE_NOTE, HELL_CHARGE_NOTE)]
        {
            if has(feature) {
                kind = Some(kind.map_or(named, |open| open.max(named)));
            }
        }
        let kind = kind.filter(|kind| *kind >= LONG_NOTE)?;
        LONG_NOTE_LABELS.get(usize::try_from(kind).ok()?).copied()
    }

    /// Whether the bar's charts carry mines.
    pub(crate) fn has_mines(&self) -> bool {
        self.drawn_features() & Self::FEATURE_MINE_NOTE != 0
    }

    /// Whether the bar's charts pick their notes at random.
    pub(crate) fn has_random(&self) -> bool {
        self.drawn_features() & Self::FEATURE_RANDOM != 0
    }
}

/// How far the wheel is through sliding its bars one slot along (`BarRenderer.duration` and
/// `angle`, with the clock they are read against).
///
/// The slide runs on the wall clock rather than on the scene's, and is drawn from the far end: the
/// cursor moves at once, and each bar is drawn part of the way back towards the slot it left.
/// [`BarScroll::default`] is a wheel at rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BarScroll {
    /// The wall clock millisecond the slide ends at, or zero when there is none.
    pub duration_ms: i64,
    /// How long one slot's travel takes, in milliseconds, signed by the way the cursor moved:
    /// positive towards the next bar, negative towards the one before.
    pub angle: i32,
    /// The wall clock this frame is drawn at, in the same milliseconds
    /// (`System.currentTimeMillis`).
    pub now_ms: i64,
}

impl BarScroll {
    /// How much of a slot's travel is still to go, or `None` when nothing is sliding
    /// (`BarRenderer.prepare`, `angleLerp`). It starts at one and falls to nothing; a wheel with a
    /// second notch queued starts at two.
    pub(crate) fn remaining(self) -> Option<f32> {
        if self.duration_ms == 0 || self.duration_ms <= self.now_ms {
            return None;
        }
        let left = if self.angle < 0 { self.now_ms - self.duration_ms } else { self.duration_ms - self.now_ms };
        Some(left as f32 / self.angle as f32)
    }

    /// The slot whose place a bar in `slot` is sliding in from, or `None` at the first slot when the
    /// cursor moved back.
    pub(crate) fn neighbour(self, slot: usize) -> Option<usize> {
        if self.angle >= 0 { slot.checked_add(1) } else { slot.checked_sub(1) }
    }
}

/// Which way a held key is moving the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarHold {
    /// No key is held.
    #[default]
    None,
    /// Towards the next bar, which is the bar below the cursor.
    Next,
    /// Towards the bar before.
    Previous,
}

/// The state behind [`BarScroll`]: when the slide under way ends, which way it goes and whether a
/// held key is driving it (`BarRenderer.input`).
///
/// It is the reference's own bookkeeping, kept beside the interpolation that reads it so that the
/// two cannot drift apart: the wheel queues at most two notches and travels faster with two, and a
/// held key moves one bar, waits [`SCROLL_DURATION_LOW_MS`], and then repeats every
/// [`SCROLL_DURATION_HIGH_MS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BarScroller {
    duration_ms: i64,
    angle: i32,
    key_input: bool,
}

impl BarScroller {
    /// Takes one frame's input and answers how many bars the cursor moves by: positive towards the
    /// next bar, negative towards the one before.
    ///
    /// `wheel` is how many notches the wheel turned towards the next bar since the last frame. A
    /// held key replaces whatever the wheel asked for on the frame it moves the cursor, as it does
    /// in the reference.
    pub fn input(&mut self, wheel: i32, hold: BarHold, now_ms: i64) -> i32 {
        let mut moved = wheel;
        if moved != 0 {
            let queued = if self.angle == 0 { 0 } else { (self.duration_ms - now_ms).max(0) as i32 / self.angle };
            let queued = (queued + moved).clamp(-WHEEL_QUEUE, WHEEL_QUEUE);
            if queued == 0 {
                self.angle = 0;
                self.duration_ms = now_ms;
            } else {
                let travel = WHEEL_NOTCH_MS / queued / queued;
                self.angle = travel / queued;
                self.duration_ms = now_ms + i64::from(travel);
            }
        }

        let towards = match hold {
            BarHold::Next => Some(1),
            BarHold::Previous => Some(-1),
            BarHold::None => None,
        };
        match towards {
            Some(step) => {
                if self.duration_ms == 0 {
                    self.key_input = true;
                    moved = step;
                    self.duration_ms = now_ms + i64::from(SCROLL_DURATION_LOW_MS);
                    self.angle = step * SCROLL_DURATION_LOW_MS;
                }
                if now_ms > self.duration_ms && self.key_input {
                    self.duration_ms = now_ms + i64::from(SCROLL_DURATION_HIGH_MS);
                    moved = step;
                    self.angle = step * SCROLL_DURATION_HIGH_MS;
                }
            }
            None => self.key_input = false,
        }
        if now_ms > self.duration_ms && !self.key_input {
            self.duration_ms = 0;
        }
        moved
    }

    /// Lets a slide that has ended go, for the frames on which the wheel takes no input because a
    /// panel is open over it (`BarRenderer.resetInput`).
    pub fn reset_input(&mut self, now_ms: i64) {
        if now_ms > self.duration_ms {
            self.duration_ms = 0;
        }
    }

    /// The slide as a frame drawn at `now_ms` reads it.
    pub fn at(&self, now_ms: i64) -> BarScroll {
        BarScroll { duration_ms: self.duration_ms, angle: self.angle, now_ms }
    }
}

/// The browser's bars: the list a song wheel turns through, where the cursor is in it, and the few
/// settings the reference's wheel reads beside the bars themselves.
///
/// A wheel needs the bars rather than one number, and a property id answers scalars, so they arrive
/// beside the scalar source instead.
#[derive(Debug, Clone, Copy)]
pub struct SongBars<'a> {
    /// Every bar of the list on show, in order (`BarManager.currentsongs`). The wheel reads the few
    /// around the cursor and goes round the ends, so a list shorter than the wheel repeats.
    pub bars: &'a [SongBar],
    /// The bar under the cursor (`BarManager.selectedindex`).
    pub selected: usize,
    /// Whether a rival is picked, which draws each bar's lamp as the player's over the rival's
    /// (`MusicSelector.getRival`).
    pub rival: bool,
    /// Whether the charts under a folder are counted by clear, which is what a folder's graph is
    /// drawn from (`Config.isFolderlamp`).
    pub folder_lamps: bool,
    /// The long note kind a chart that leaves it open is played with: zero long note, one charge
    /// note, two hell charge note (`PlayerConfig.getLnmode`).
    pub ln_mode: i32,
    /// How far the wheel is through sliding its bars along.
    pub scroll: BarScroll,
    /// Whether the application's option panel is open over the browser.
    pub options_open: bool,
}

impl<'a> SongBars<'a> {
    /// The list `bars` with the cursor on `selected`: no rival, folders counted, long notes played
    /// as long notes, the wheel at rest and no panel open.
    pub fn new(bars: &'a [SongBar], selected: usize) -> SongBars<'a> {
        SongBars { bars, selected, rival: false, folder_lamps: true, ln_mode: LONG_NOTE, scroll: BarScroll::default(), options_open: false }
    }

    /// Which bar lands on `slot` of a wheel whose cursor is on slot `center`, or `None` for an empty
    /// list (`(selectedindex + length * 100 + i - center) % length`).
    pub fn bar_on(&self, slot: usize, center: i32) -> Option<usize> {
        let len = i64::try_from(self.bars.len()).ok().filter(|len| *len > 0)?;
        let position = i64::try_from(self.selected).ok()? + len * RING_TURNS + i64::try_from(slot).ok()? - i64::from(center);
        usize::try_from(position.rem_euclid(len)).ok()
    }
}

//! Which picture a chart's background shows at a given moment (`BGAProcessor.prepareBGA` and the
//! choice `drawBGA` makes from what it left behind).
//!
//! The chart changes its background at points of its own timeline: a new picture, a new layer, a
//! layer taken away, and a picture to flash when the player misses. [`BgaPlayhead`] walks those
//! points as the play clock passes them and remembers what is showing. It speaks in picture numbers
//! (`#BMPxx`); the textures behind them are [`BgaTextures`](super::BgaTextures)'.
//!
//! The walk is the reference's walk, quirks included. A point is passed once, when the clock first
//! reaches it. A clock that is before the start of play (negative) passes nothing and shows black.
//! The miss layer starts when the game tells the playhead a miss happened, shows for as long as the
//! player's miss layer duration says, and is chosen by how far through that time the clock is.

use rbms_model::TimeLine;

/// What `BGAProcessor` keeps for the picture or layer that is showing and for a chart that has
/// none: `playingbgaid = -1`.
const NO_PICTURE: i32 = -1;

/// What a chart's picture or layer number says to take the picture or layer away: `bga == -2`.
const PICTURE_OFF: i32 = -2;

/// What a chart's picture or layer number says to leave it as it is.
const PICTURE_KEEP: i32 = -1;

/// The picture number of a step of a miss layer that shows nothing (`Integer.MIN_VALUE`).
pub const MISS_LAYER_NONE: i32 = i32::MIN;

/// The time a miss layer shows for unless the player chose another (`PlayerConfig.misslayerDuration`).
pub const DEFAULT_MISS_LAYER_DURATION_MS: i64 = 500;

/// The microseconds in a millisecond, which is how a chart's time becomes the reference's.
const MICROS_PER_MILLI: i64 = 1000;

/// One point of a chart's timeline at which its background changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgaEvent {
    /// When the clock passes it, in whole milliseconds (`TimeLine.getTime()`).
    pub time_ms: i64,
    /// The picture to show from here: a picture number, `-1` to leave the one showing, `-2` to
    /// take it away.
    pub base: i32,
    /// The layer to show from here, by the same convention.
    pub layer: i32,
    /// The pictures of the miss layer this point installs, in the order they are shown through the
    /// miss layer's time, or `None` for a point that installs none. A step of
    /// [`MISS_LAYER_NONE`] shows nothing.
    pub miss: Option<Vec<i32>>,
}

impl BgaEvent {
    /// The event a timeline of the model makes, or `None` for one that changes nothing in the
    /// background.
    ///
    /// The model carries a picture and a layer per timeline and no miss layer, so the event it makes
    /// has none.
    pub fn of_timeline(timeline: &TimeLine) -> Option<BgaEvent> {
        (timeline.bga != PICTURE_KEEP || timeline.layer != PICTURE_KEEP).then_some(BgaEvent {
            time_ms: timeline.time_us / MICROS_PER_MILLI,
            base: timeline.bga,
            layer: timeline.layer,
            miss: None,
        })
    }
}

/// What the background shows at the moment the playhead is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgaPick {
    /// Play has not started: black.
    Blank,
    /// The miss layer is showing: the picture of its current step, or none for a step that shows
    /// nothing.
    Miss(Option<i32>),
    /// The chart is playing: its picture and its layer, by picture number.
    Playing { base: Option<i32>, layer: Option<i32> },
}

/// Walks a chart's background events as the play clock passes them.
#[derive(Debug, Clone, Default)]
pub struct BgaPlayhead {
    events: Vec<BgaEvent>,
    /// Where the next walk starts (`BGAProcessor.pos`).
    cursor: usize,
    /// The clock the last walk ended at (`BGAProcessor.time`); negative before play.
    clock_ms: i64,
    base: i32,
    layer: i32,
    /// The steps of the miss layer the walk last passed, when it has passed one.
    miss: Option<Vec<i32>>,
    /// When the last miss happened, or `0` for none yet (`misslayertime`).
    missed_at_ms: i64,
    /// How long the last miss shows its layer for (`getMisslayerduration`).
    miss_duration_ms: i64,
}

impl BgaPlayhead {
    /// A playhead over the background events of a chart, in time order.
    pub fn new(events: Vec<BgaEvent>) -> BgaPlayhead {
        BgaPlayhead { events, base: NO_PICTURE, layer: NO_PICTURE, ..BgaPlayhead::default() }
    }

    /// A playhead over a model's timelines, which are in time order.
    pub fn of_chart(timelines: &[TimeLine]) -> BgaPlayhead {
        BgaPlayhead::new(timelines.iter().filter_map(BgaEvent::of_timeline).collect())
    }

    /// Back to the start of the chart, showing nothing and with no miss layer (`BGAProcessor.prepare`,
    /// which the reference runs as play becomes ready).
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.clock_ms = 0;
        self.base = NO_PICTURE;
        self.layer = NO_PICTURE;
        self.miss = None;
        self.missed_at_ms = 0;
    }

    /// Passes every event the play clock has reached since the last call (`prepareBGA`). A negative
    /// clock is play that has not started: nothing is passed and the background reads as black.
    pub fn prepare(&mut self, now_ms: i64) {
        if now_ms < 0 {
            self.clock_ms = -1;
            return;
        }
        for index in self.cursor..self.events.len() {
            let event = &self.events[index];
            if event.time_ms > now_ms {
                break;
            }
            if event.time_ms <= self.clock_ms {
                self.cursor += 1;
                continue;
            }
            match event.base {
                PICTURE_OFF => self.base = NO_PICTURE,
                number if number >= 0 => self.base = number,
                _ => {}
            }
            match event.layer {
                PICTURE_OFF => self.layer = NO_PICTURE,
                number if number >= 0 => self.layer = number,
                _ => {}
            }
            if let Some(steps) = &event.miss {
                self.miss = Some(steps.clone());
            }
        }
        self.clock_ms = now_ms;
    }

    /// Starts the miss layer at `at_ms` for `duration_ms` (`setMisslayerTme`). The game calls it
    /// when a judgement leaves the combo at nothing. A miss at the very start of the clock, `0`,
    /// reads as no miss, as the reference reads it.
    pub fn start_miss(&mut self, at_ms: i64, duration_ms: i64) {
        self.missed_at_ms = at_ms;
        self.miss_duration_ms = duration_ms;
    }

    /// What the background shows now (`drawBGA`).
    pub fn pick(&self) -> BgaPick {
        if self.clock_ms < 0 {
            return BgaPick::Blank;
        }
        let into = self.clock_ms - self.missed_at_ms;
        if let Some(steps) = &self.miss
            && self.missed_at_ms != 0
            && (0..self.miss_duration_ms).contains(&into)
        {
            let last = i64::try_from(steps.len().saturating_sub(1)).unwrap_or(i64::MAX);
            let step = usize::try_from(last * into / self.miss_duration_ms).ok().and_then(|index| steps.get(index));
            return BgaPick::Miss(step.copied().filter(|number| *number != MISS_LAYER_NONE));
        }
        BgaPick::Playing { base: (self.base >= 0).then_some(self.base), layer: (self.layer >= 0).then_some(self.layer) }
    }
}

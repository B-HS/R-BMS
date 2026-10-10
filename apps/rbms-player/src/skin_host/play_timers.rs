//! The play screen's timer driver: what switches a play skin's timers on and off as a run goes,
//! and the offsets the game itself sets on that screen.
//!
//! A timer is not read from a cluster. It lives in the scene's timer table, which the host reads
//! from directly, and something has to switch each one at the moment the reference does. That is
//! [`PlayTimerDriver`]: the play screen tells it what happened ([`SceneEvent`]) and it switches the
//! timers the reference switches for that, on the same terms -- restarted or left running, switched
//! off or never switched off.
//!
//! The driver decides nothing about the run. When the screen leaves its loading state, when a run
//! fails and when the last note has gone by are the play screen's own decisions, and so is every
//! judgement; each arrives here as an event. What the driver keeps is the little the reference's own
//! timer code keeps between frames: whether the judge is running, whether the key beams are stopped,
//! which way each turntable was last turned, how far each has turned, where the rhythm timer stands
//! and which bar line comes next.
//!
//! | The play screen (`BMSPlayer`) | Event |
//! | --- | --- |
//! | every frame, once `input` has passed | [`SceneEvent::InputOpen`] |
//! | every frame of `STATE_PRELOAD` and `STATE_PRACTICE` | [`SceneEvent::Standby`] |
//! | every frame of `STATE_PRELOAD` with the chart preview enabled | [`SceneEvent::Preview`] |
//! | `STATE_PRELOAD` or `STATE_PRACTICE` to `STATE_READY` | [`SceneEvent::Ready`] |
//! | `STATE_READY` to `STATE_PLAY` | [`SceneEvent::Started`] |
//! | every frame of `STATE_PLAY` | [`SceneEvent::Playing`] |
//! | every frame of `STATE_PLAY` past the last note | [`SceneEvent::LastNotePassed`] |
//! | to `STATE_FAILED` | [`SceneEvent::Failed`] |
//! | `STATE_PLAY` to `STATE_FINISHED` because the chart ran out | [`SceneEvent::MusicEnded`] |
//! | the fade out beginning, whichever way | [`SceneEvent::Fadeout`] |
//! | every frame of `STATE_FAILED` and `STATE_FINISHED` | [`SceneEvent::JudgeStopped`] |
//! | every frame of the screen, whatever its state (`KeyInputProccessor.input`) | [`SceneEvent::Input`] |
//! | the judge taking a key going down (`inputKeyOn`) | [`SceneEvent::Pressed`] |
//! | a judgement (`JudgeManager.updateMicro`, `BMSPlayer.update`) | [`SceneEvent::Judged`] |
//! | a lane's long note being held or let go | [`SceneEvent::LongNote`] |
//! | a lane's hell charge note going by | [`SceneEvent::Hcn`] |
//!
//! [`SceneEvent::Seeked`] has no row: the reference cannot move a run about, and it is this player's
//! own rule for the replay analysis that a jump switches every lane's timers off.
//!
//! The reference's practice screen coming back after a run (`BMSPlayer.java:526-540`) has no event.
//! This player sets a practice slice up on a screen of its own and plays every slice on a play
//! screen entered afresh, with every timer off, so no run is ever cleared away from under a skin.
//!
//! The offsets are the five the reference writes itself during play. The two turntable angles are
//! worked out here, from the same frames as the key timers. The lift, the lane cover and the hidden
//! cover belong to the note field, so [`PlayOffsets`] carries whatever the play screen worked out
//! for them and answers with that.

use rbms_judge::Judge;
use rbms_judge::matcher::ScratchDir;
use rbms_model::{Mode, Model, TimeLine};
use rbms_skin::dst::{OffsetSource, SkinOffset};
use rbms_skin::loader::{LoadedSkin, PlayTimings};
use rbms_skin::model::SkinDef;
use rbms_skin::property::generated::{OFFSET_HIDDEN_COVER, OFFSET_LANECOVER, OFFSET_LIFT, OFFSET_SCRATCHANGLE_1P};
use rbms_skin::timer::{MICROS_PER_MILLI, TimerId, TimerState, timer_id};

use super::score::standing::ScoreStanding;

/// The timer the chart preview runs on, which the reference numbers without declaring a constant
/// for it (`BMSPlayer.java:482-503`, `LaneRenderer.java:300-301`).
pub const CHART_PREVIEW: TimerId = TimerId(141);

/// How far ahead of the first note the chart preview starts, in microseconds
/// (`BMSPlayer.java:495`).
const CHART_PREVIEW_LEAD_US: i64 = 1_000_000;

/// How many sides the reference numbers lane timers for (`SkinPropertyMapper`: `player < 2`).
const LANE_TIMER_SIDES: usize = 2;

/// The first key of a side that is numbered in the second, wider band of lane timer ids, and the
/// first key that has no id at all (`SkinPropertyMapper`: `key < 10`, `key < 100`).
const HIGH_BAND_FIRST_KEY: usize = 10;
const LANE_TIMER_KEY_LIMIT: usize = 100;

/// How far apart the two sides' ids are in the low band and in the high one.
const LOW_BAND_SIDE_STRIDE: usize = 10;
const HIGH_BAND_SIDE_STRIDE: usize = 100;

/// The judgement pop-up timer and the combo timer of each judge region, first to third
/// (`JudgeManager.JUDGE_TIMER`, `COMBO_TIMER`).
const JUDGE_TIMERS: [TimerId; 3] = [timer_id::JUDGE_1P, timer_id::JUDGE_2P, timer_id::JUDGE_3P];
const COMBO_TIMERS: [TimerId; 3] = [timer_id::COMBO_1P, timer_id::COMBO_2P, timer_id::COMBO_3P];

/// From how many judge regions on a judgement switches the other regions' combo timers off
/// (`JudgeManager.java:680`).
const EXCLUSIVE_COMBO_REGIONS: usize = 3;

/// The rank steps whose reaching switches `SCORE_A`, `SCORE_AA` and `SCORE_AAA` on
/// (`BMSPlayer.java:1033-1035`).
const RANK_STEP_A: usize = 18;
const RANK_STEP_AA: usize = 21;
const RANK_STEP_AAA: usize = 24;

/// How many turntables a mode can have, which is how many angle offsets the reference writes.
const SCRATCH_SLOTS: usize = 2;

/// One whole turn of a turntable in the reference's own units, and how many of them make a degree
/// (`KeyInputProccessor.java:95-104`).
const SCRATCH_TURN: i64 = 2160;
const SCRATCH_UNITS_PER_DEGREE: i64 = 6;

/// How many times faster than the idle spin a held turntable key turns it.
const SCRATCH_KEY_GAIN: i64 = 2;

/// The play speed and the practice frequency a run has when nothing changes them, in percent, and
/// the scale the rhythm timer's formula is written in (`RhythmTimerProcessor.java:64`).
const FULL_PERCENT: i32 = 100;
const RHYTHM_PERCENT: f64 = 100.0;

/// The seconds in the minute a tempo is counted over (`RhythmTimerProcessor.java:64`).
const SECONDS_PER_MINUTE: f64 = 60.0;

/// How many motions a pop'n character has a cycle time for (`PomyuCharaProcessor.PMcharaTime`).
pub const PM_MOTION_COUNT: usize = 8;

/// The cycle time of a motion nothing has set, in milliseconds, which is also the shortest one the
/// reference accepts (`PomyuCharaProcessor.setPMcharaTime`).
const PM_MOTION_CYCLE_FLOOR_MS: i64 = 1;

/// How long after a neutral cycle comes round a character may leave it, in milliseconds
/// (`PomyuCharaProcessor.java:42`).
const PM_NEUTRAL_WINDOW_MS: i64 = 17;

/// The last judgement a pop'n character's first side cheers, and the one it merely approves of
/// (`PomyuCharaProcessor.PMcharaJudge`, which is the judgement's code plus one).
const PM_GREAT_LIMIT: i32 = 2;
const PM_GOOD: i32 = 3;

/// The motions a pop'n character returns to neutral from (`PomyuCharaProcessor.java:59-70`): the
/// first side's and the second side's, the second side's own neutral timer sitting between them.
const PM_FIRST_SIDE_MOTIONS: [TimerId; 4] = [timer_id::PM_CHARA_1P_FEVER, timer_id::PM_CHARA_1P_GREAT, timer_id::PM_CHARA_1P_GOOD, timer_id::PM_CHARA_1P_BAD];
const PM_SECOND_SIDE_MOTIONS: [TimerId; 2] = [timer_id::PM_CHARA_2P_GREAT, timer_id::PM_CHARA_2P_BAD];

/// The pop'n character timers a run's end switches off (`BMSPlayer.java:682-685`): every one from
/// the first side's neutral to the second side's bad, and the dance.
const PM_TIMERS: [TimerId; 9] = [
    timer_id::PM_CHARA_1P_NEUTRAL,
    timer_id::PM_CHARA_1P_FEVER,
    timer_id::PM_CHARA_1P_GREAT,
    timer_id::PM_CHARA_1P_GOOD,
    timer_id::PM_CHARA_1P_BAD,
    timer_id::PM_CHARA_2P_NEUTRAL,
    timer_id::PM_CHARA_2P_GREAT,
    timer_id::PM_CHARA_2P_BAD,
    timer_id::PM_CHARA_DANCE,
];

/// The skin key numbers of each mode's lanes, in the chart's lane order
/// (`LaneProperty.laneToSkinOffset`): zero is a turntable and one and up are the keys of a side.
const BEAT_5K_KEYS: [usize; 6] = [1, 2, 3, 4, 5, 0];
const BEAT_7K_KEYS: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 0];
const BEAT_10K_KEYS: [usize; 12] = [1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5, 0];
const BEAT_14K_KEYS: [usize; 16] = [1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0];
const POPN_9K_KEYS: [usize; 9] = counted_from_one();
const KEYBOARD_24K_KEYS: [usize; 26] = counted_from_one();

/// Which table numbers the lanes of which mode. A mode that is not here is numbered as seven keys.
const MODE_KEYS: [(Mode, &[usize]); 5] = [
    (Mode::BEAT_5K, &BEAT_5K_KEYS),
    (Mode::BEAT_10K, &BEAT_10K_KEYS),
    (Mode::BEAT_14K, &BEAT_14K_KEYS),
    (Mode::POPN_9K, &POPN_9K_KEYS),
    (Mode::KEYBOARD_24K, &KEYBOARD_24K_KEYS),
];

/// The key numbers of a mode none of whose lanes is a turntable: one for the first lane and up.
const fn counted_from_one<const LANES: usize>() -> [usize; LANES] {
    let mut keys = [0; LANES];
    let mut lane = 0;
    while lane < LANES {
        keys[lane] = lane + 1;
        lane += 1;
    }
    keys
}

/// Where one lane of the chart sits in the numbering a skin's lane timers and judge values carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneSlot {
    /// Which side the lane is on, zero or one (`LaneProperty.laneToPlayer`).
    pub player: usize,
    /// Zero for a turntable, one and up for the keys of its side (`laneToSkinOffset`).
    pub key: usize,
    /// Which of the mode's turntables the lane is, for a lane that is one (`laneToScratch`).
    pub scratch: Option<usize>,
}

/// The slot of every lane of `mode`, in the chart's lane order (`LaneProperty`).
///
/// The numbering is the reference's table for the mode and nothing else: where a lane is drawn has
/// no say in it. The 24-key mode has no turntable in this table, whatever the chart model calls its
/// last two lanes, and a mode the reference has no table for is numbered as seven keys are, which
/// is what the reference's own fallback does.
pub fn lane_slots(mode: Mode) -> Vec<LaneSlot> {
    let keys = MODE_KEYS.iter().find(|(known, _)| *known == mode).map_or(BEAT_7K_KEYS.as_slice(), |(_, keys)| *keys);
    let lanes_per_side = (mode.key / usize::from(mode.player).max(1)).max(1);
    let mut scratches = 0;
    keys.iter()
        .enumerate()
        .map(|(lane, &key)| {
            let scratch = (key == 0).then_some(scratches);
            scratches += usize::from(key == 0);
            LaneSlot { player: lane / lanes_per_side, key, scratch }
        })
        .collect()
}

/// One kind of timer every lane has (`SkinPropertyMapper`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneTimer {
    /// A judgement good enough for a bomb (`bombTimerId`).
    Bomb,
    /// A long note being held (`holdTimerId`).
    Hold,
    /// The lane going down (`keyOnTimerId`).
    KeyOn,
    /// The lane coming back up (`keyOffTimerId`).
    KeyOff,
    /// A hell charge note going by while it is held (`hcnActiveTimerId`).
    HcnActive,
    /// A hell charge note going by while it is not (`hcnDamageTimerId`).
    HcnDamage,
}

impl LaneTimer {
    /// Every kind, for whatever has to reach all of a lane's timers.
    pub const ALL: [LaneTimer; 6] = [LaneTimer::Bomb, LaneTimer::Hold, LaneTimer::KeyOn, LaneTimer::KeyOff, LaneTimer::HcnActive, LaneTimer::HcnDamage];

    /// The id of the first side's turntable in the low band, and of its tenth key in the high one.
    const fn bases(self) -> (TimerId, TimerId) {
        match self {
            LaneTimer::Bomb => (timer_id::BOMB_1P_SCRATCH, timer_id::BOMB_1P_KEY10),
            LaneTimer::Hold => (timer_id::HOLD_1P_SCRATCH, timer_id::HOLD_1P_KEY10),
            LaneTimer::KeyOn => (timer_id::KEYON_1P_SCRATCH, timer_id::KEYON_1P_KEY10),
            LaneTimer::KeyOff => (timer_id::KEYOFF_1P_SCRATCH, timer_id::KEYOFF_1P_KEY10),
            LaneTimer::HcnActive => (timer_id::HCN_ACTIVE_1P_SCRATCH, timer_id::HCN_ACTIVE_1P_KEY10),
            LaneTimer::HcnDamage => (timer_id::HCN_DAMAGE_1P_SCRATCH, timer_id::HCN_DAMAGE_1P_KEY10),
        }
    }

    /// The timer of this kind for key `key` of side `player`, or `None` where the reference has no
    /// id: a third side, or a key past the ninety-ninth.
    ///
    /// Keys nought to nine follow the first side's turntable ten apart per side, and keys ten and
    /// up follow its tenth key a hundred apart per side, exactly as the reference writes it.
    pub fn id(self, player: usize, key: usize) -> Option<TimerId> {
        if player >= LANE_TIMER_SIDES || key >= LANE_TIMER_KEY_LIMIT {
            return None;
        }
        let (low, high) = self.bases();
        let offset = if key < HIGH_BAND_FIRST_KEY { key + player * LOW_BAND_SIDE_STRIDE } else { key - HIGH_BAND_FIRST_KEY + player * HIGH_BAND_SIDE_STRIDE };
        let base = if key < HIGH_BAND_FIRST_KEY { low } else { high };
        i32::try_from(offset).ok().map(|offset| TimerId(base.get() + offset))
    }

    /// The timer of this kind for one lane.
    pub fn of(self, slot: LaneSlot) -> Option<TimerId> {
        self.id(slot.player, slot.key)
    }
}

/// How many judge regions a play skin has (`PlaySkin.judgeregion`): one more than the highest
/// `index` among the judge objects a destination places, and none when no destination places one.
///
/// A region is a run of lanes that shares one judgement pop-up, so this is what decides which of
/// the judge and combo timers a judgement switches.
pub fn judge_regions(def: &SkinDef) -> usize {
    let regions = def
        .destination
        .iter()
        .filter_map(|placed| def.judge.iter().find(|judge| judge.id == placed.id))
        .fold(0, |regions, judge| if judge.index >= regions { judge.index + 1 } else { regions });
    usize::try_from(regions).unwrap_or_default()
}

/// The region a judgement in `lane` belongs to: the lane's place in the chart over how many lanes
/// make a region (`JudgeManager.java:677`), both divisions whole.
///
/// `None` where the reference's own arithmetic has no answer: a skin with no judge object, more
/// regions than lanes, or a region past the third, which there is no timer for.
pub fn judge_region(lane: usize, lane_count: usize, regions: usize) -> Option<usize> {
    let lanes_per_region = lane_count.checked_div(regions)?;
    lane.checked_div(lanes_per_region).filter(|region| *region < JUDGE_TIMERS.len())
}

/// When each bar line of a chart is reached, in microseconds and in order
/// (`RhythmTimerProcessor`'s `sectiontimes`).
pub fn section_times_us(timelines: &[TimeLine]) -> Vec<i64> {
    timelines.iter().filter(|line| line.section_line).map(|line| line.time_us).collect()
}

/// When the first note of a chart is reached, in microseconds, or zero for a chart with none
/// (`BMSPlayer.java:488-494`).
pub fn first_note_us(timelines: &[TimeLine]) -> i64 {
    timelines.iter().find(|line| line.notes.iter().any(Option::is_some)).map_or(0, |line| line.time_us)
}

/// What the driver is told once, before the run: the chart's shape and what the skin asks for.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaySetup {
    /// The mode of the chart as it is played, which decides how its lanes are numbered.
    pub mode: Mode,
    /// Whether the game plays the chart by itself. Its key beams are then lit from the frame's key
    /// levels and not by the judge (`KeyInputProccessor.java:78`). A replay is not this.
    pub autoplay: bool,
    /// The worst judgement that still fires a lane's bomb, as the reference numbers judgements
    /// (`PlaySkin.judgetimer`).
    pub judge_timer: i32,
    /// How many judge regions the skin has ([`judge_regions`]).
    pub judge_regions: usize,
    /// When each bar line is reached ([`section_times_us`]), in the chart's time as it was written.
    pub sections_us: Vec<i64>,
    /// When the first note is reached ([`first_note_us`]).
    pub first_note_us: i64,
    /// The practice frequency in percent, a hundred outside practice. The reference weighs its bar
    /// lines by `100 / freq`, a whole division, because its table is the unscaled chart's
    /// (`RhythmTimerProcessor.java:67`). Leave it at a hundred when [`Self::sections_us`] is already
    /// in the time the run is played in.
    pub practice_freq: i32,
    /// How long one cycle of each pop'n character motion lasts, in milliseconds
    /// (`PomyuCharaProcessor.PMcharaTime`): neutral, fever, great, good and bad of the first side,
    /// then neutral, great and bad of the second.
    pub pm_motion_cycles_ms: [i64; PM_MOTION_COUNT],
}

impl PlaySetup {
    /// The setup of a chart of `mode` with everything else as the reference has it when nothing
    /// says otherwise: played by hand, bombs for the two best judgements, no judge region, no bar
    /// line.
    pub fn new(mode: Mode) -> PlaySetup {
        PlaySetup {
            mode,
            autoplay: false,
            judge_timer: PlayTimings::default().judgetimer,
            judge_regions: 0,
            sections_us: Vec::new(),
            first_note_us: 0,
            practice_freq: FULL_PERCENT,
            pm_motion_cycles_ms: [PM_MOTION_CYCLE_FLOOR_MS; PM_MOTION_COUNT],
        }
    }

    /// The setup of one run: `model` as it is played, drawn by `skin` when there is one.
    pub fn of(model: &Model, skin: Option<&LoadedSkin>, autoplay: bool) -> PlaySetup {
        PlaySetup {
            autoplay,
            judge_timer: skin.map_or_else(|| PlayTimings::default().judgetimer, |skin| skin.play.judgetimer),
            judge_regions: skin.map_or(0, |skin| judge_regions(&skin.def)),
            sections_us: section_times_us(&model.timelines),
            first_note_us: first_note_us(&model.timelines),
            ..PlaySetup::new(model.mode)
        }
    }
}

/// Which of a lane's keys are down this frame (`LaneProperty.laneToKey`).
///
/// A key lane has one key, which is `forward`. A turntable has two: `forward` is the first the
/// reference lists for it and `backward` the second, the same two [`ScratchDir`] tells apart. A key
/// the game presses for the player while it plays by itself counts as down
/// (`JudgeManager.auto_presstime`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LaneKeys {
    pub forward: bool,
    pub backward: bool,
}

impl LaneKeys {
    /// A lane with nothing down.
    pub const UP: LaneKeys = LaneKeys { forward: false, backward: false };

    /// A lane with the one key in `dir` down.
    pub const fn down(dir: ScratchDir) -> LaneKeys {
        LaneKeys { forward: matches!(dir, ScratchDir::Forward), backward: matches!(dir, ScratchDir::Backward) }
    }

    /// Whether the key in `dir` is down.
    const fn is_down(self, dir: ScratchDir) -> bool {
        match dir {
            ScratchDir::Forward => self.forward,
            ScratchDir::Backward => self.backward,
        }
    }
}

/// What a run has reached as of a judgement, which is what `BMSPlayer.update` switches the full
/// combo timer and the five score timers by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reached {
    /// Every note has gone by and the combo is as long as the chart.
    pub full_combo: bool,
    /// The score over the whole chart has secured rank A, AA and AAA (`qualifyRank(18)`, `(21)`
    /// and `(24)`).
    pub rank_a: bool,
    pub rank_aa: bool,
    pub rank_aaa: bool,
    /// The EX score is at least the player's best.
    pub best: bool,
    /// The EX score is at least the target's.
    pub target: bool,
}

impl Reached {
    /// What `standing` has reached after `past_notes` of the chart's `total_notes` have gone by
    /// with a combo of `combo` (`BMSPlayer.java:1028-1037`).
    pub fn of(standing: &ScoreStanding, past_notes: u32, total_notes: u32, combo: u32) -> Reached {
        Reached {
            full_combo: past_notes == total_notes && past_notes == combo,
            rank_a: standing.secured(RANK_STEP_A),
            rank_aa: standing.secured(RANK_STEP_AA),
            rank_aaa: standing.secured(RANK_STEP_AAA),
            best: standing.now_ex >= standing.best_score,
            target: standing.now_ex >= standing.rival_score,
        }
    }
}

/// One judgement as the judge reports it to the screen (`JudgeManager.updateMicro`).
///
/// A judgement the reference drops before it reaches the screen is not one: a second POOR on a note
/// that already has a judgement, under the rule that counts one miss a note
/// (`JudgeManager.java:646-648`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Judgement {
    /// The chart lane it was given in.
    pub lane: usize,
    pub judge: Judge,
    /// How early the input was, in microseconds; a late one is negative (`mfast`). No timer reads
    /// it: it rides along for whatever else the play screen feeds from the same judgement.
    pub fast_us: i64,
    /// What the run has reached with this judgement counted.
    pub reached: Reached,
}

/// A judgement's code in the reference's numbering, PGREAT as zero to the empty POOR as five.
const fn judge_code(judge: Judge) -> i32 {
    match judge {
        Judge::PerfectGreat => 0,
        Judge::Great => 1,
        Judge::Good => 2,
        Judge::Bad => 3,
        Judge::Poor => 4,
        Judge::Miss => 5,
    }
}

/// Where a lane stands with a hell charge note (`JudgeManager.java:306-338`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HcnPass {
    /// None is going by, or the one that is has not been judged yet.
    #[default]
    None,
    /// One is going by and its gauge is rising: the lane is held.
    Active,
    /// One is going by and its gauge is falling: the lane is not held.
    Damage,
}

/// One frame of `STATE_PLAY` (`BMSPlayer.java:620-691`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayingFrame {
    /// The chart time the frame is drawn at, in microseconds: what the play timer has run for.
    pub chart_us: i64,
    /// The tempo at that time (`LaneRenderer.getNowBPM`).
    pub bpm: f64,
    /// How fast the chart is running against the scene clock, in percent
    /// (`BMSPlayer.getPlaySpeed`): a hundred as written, nought while it stands still.
    pub play_speed: i32,
    /// Whether the gauge is full (`Gauge.isMax`).
    pub gauge_max: bool,
    /// How many notes have gone by (`BMSPlayer.getPastNotes`).
    pub past_notes: u32,
}

/// One thing that happened on the play screen. The module's own table says when each is sent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneEvent<'a> {
    /// The skin's `input` time has passed: `STARTINPUT` goes on and stays (`BMSPlayer.java:470-472`).
    InputOpen,
    /// The screen is waiting to start: a pop'n character stands in neutral
    /// (`BMSPlayer.java:518-521`, `:539-542`).
    Standby,
    /// The chart preview's frame. Holding START or SELECT on the frame starts the preview one second
    /// ahead of the first note, and a frame without either ends it (`BMSPlayer.java:481-497`).
    Preview { pressed: bool },
    /// The run is ready: `READY` goes on, for good, and a chart preview ends
    /// (`BMSPlayer.java:501-515`, `:591`).
    Ready,
    /// The run starts `start_offset_us` into the chart: `PLAY` and `RHYTHM` are set that far back
    /// and the judge starts (`BMSPlayer.java:607-616`).
    Started { start_offset_us: i64 },
    /// One frame of the run.
    Playing(PlayingFrame),
    /// The play timer is past the last note: `ENDOFNOTE_1P` goes on and stays
    /// (`BMSPlayer.java:688-690`).
    LastNotePassed,
    /// The run failed, by its gauge or by the player giving up: `FAILED` goes on
    /// (`BMSPlayer.java:656`, `:963`).
    Failed,
    /// The chart ran out: `MUSIC_END` goes on and a pop'n character stops (`BMSPlayer.java:679-687`).
    MusicEnded,
    /// The fade out begins. It keeps the moment it first began (`BMSPlayer.java:748-750`, and
    /// `stopPlay`, which never reaches a fade that has begun).
    Fadeout,
    /// The judge is stopped and the key beams with it (`KeyInputProccessor.stopJudge`). Stopping a
    /// judge that is not running does nothing.
    JudgeStopped,
    /// The run jumped to `chart_us`. This player's own rule, for the replay analysis: every lane's
    /// timers go off, and the next bar line is the first one after where the run landed.
    Seeked { chart_us: i64 },
    /// The keys of every lane this frame, in the chart's lane order (`KeyInputProccessor.input`).
    Input(&'a [LaneKeys]),
    /// The judge took a key going down in `lane` (`KeyInputProccessor.inputKeyOn`).
    Pressed { lane: usize },
    /// A judgement.
    Judged(Judgement),
    /// Whether `lane` has a long note held: one being pressed, or a hell charge note going by with
    /// its gauge rising (`JudgeManager.java:632`).
    LongNote { lane: usize, held: bool },
    /// Where `lane` stands with a hell charge note.
    Hcn { lane: usize, pass: HcnPass },
}

/// The offsets the game itself sets during play.
///
/// The two turntable angles are the driver's ([`PlayTimerDriver::offsets`]). The other three are
/// whatever the play screen worked out from the note field, and one it has not worked out is left
/// to whoever else answers for it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayOffsets {
    /// How far each turntable has turned, in degrees, for the turntables the mode has
    /// (`OFFSET_SCRATCHANGLE_1P` and `_2P`).
    pub scratch_angle: [Option<f32>; SCRATCH_SLOTS],
    /// The lift, the lane cover and the hidden cover.
    pub field: FieldOffsets,
}

/// The three offsets the note field writes as it is drawn (`LaneRenderer.java:332-346`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FieldOffsets {
    /// `OFFSET_LIFT`: how far the judge line is lifted.
    pub lift: Option<SkinOffset>,
    /// `OFFSET_LANECOVER`: how far down the lane cover has come.
    pub lane_cover: Option<SkinOffset>,
    /// `OFFSET_HIDDEN_COVER`: how far up the hidden cover reaches, or the alpha that hides it.
    pub hidden_cover: Option<SkinOffset>,
}

impl OffsetSource for PlayOffsets {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        match id {
            OFFSET_LIFT => self.field.lift,
            OFFSET_LANECOVER => self.field.lane_cover,
            OFFSET_HIDDEN_COVER => self.field.hidden_cover,
            _ => {
                let turntable = usize::try_from(id - OFFSET_SCRATCHANGLE_1P).ok()?;
                let angle = (*self.scratch_angle.get(turntable)?)?;
                Some(SkinOffset { r: angle, ..SkinOffset::default() })
            }
        }
    }
}

/// Switches a play skin's timers as the play screen reports what happens.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayTimerDriver {
    lanes: Vec<LaneSlot>,
    /// The lane each turntable is, by turntable.
    scratch_lanes: [Option<usize>; SCRATCH_SLOTS],
    autoplay: bool,
    judge_timer: i32,
    judge_regions: usize,
    sections_us: Vec<i64>,
    first_note_us: i64,
    practice_freq: i32,
    pm_motion_cycles_ms: [i64; PM_MOTION_COUNT],
    /// Whether the judge is running (`KeyInputProccessor.isJudgeStarted`).
    judging: bool,
    /// Whether the key beams are stopped (`keyBeamStop`).
    key_beams_stopped: bool,
    /// The key each turntable was last seen turned with (`scratchKey`).
    scratch_key: [Option<ScratchDir>; SCRATCH_SLOTS],
    /// How far each turntable has turned, in [`SCRATCH_TURN`]ths of a turn (`scratch`).
    scratch: [i64; SCRATCH_SLOTS],
    /// The millisecond of the last frame of keys (`KeyInputProccessor.prevtime`).
    input_ms: Option<i64>,
    /// The microsecond of the last frame of the run (`BMSPlayer.prevtime`).
    playing_us: Option<i64>,
    /// Where the rhythm timer stands (`RhythmTimerProcessor.rhythmtimer`).
    rhythm_us: i64,
    /// How many bar lines are behind the run (`sections`).
    sections_passed: usize,
    /// How many notes had gone by when each side's pop'n character last went neutral
    /// (`PMcharaLastnotes`).
    pm_last_notes: [u32; LANE_TIMER_SIDES],
    /// The last judgement's code plus one, nought before any (`PMcharaJudge`).
    pm_judge: i32,
}

impl PlayTimerDriver {
    /// A driver for one run, with nothing having happened yet.
    pub fn new(setup: PlaySetup) -> PlayTimerDriver {
        let lanes = lane_slots(setup.mode);
        let scratch_lanes = std::array::from_fn(|turntable| lanes.iter().position(|slot| slot.scratch == Some(turntable)));
        PlayTimerDriver {
            lanes,
            scratch_lanes,
            autoplay: setup.autoplay,
            judge_timer: setup.judge_timer,
            judge_regions: setup.judge_regions,
            sections_us: setup.sections_us,
            first_note_us: setup.first_note_us,
            practice_freq: setup.practice_freq,
            pm_motion_cycles_ms: setup.pm_motion_cycles_ms.map(|cycle| cycle.max(PM_MOTION_CYCLE_FLOOR_MS)),
            judging: false,
            key_beams_stopped: false,
            scratch_key: [None; SCRATCH_SLOTS],
            scratch: [0; SCRATCH_SLOTS],
            input_ms: None,
            playing_us: None,
            rhythm_us: 0,
            sections_passed: 0,
            pm_last_notes: [0; LANE_TIMER_SIDES],
            pm_judge: 0,
        }
    }

    /// The slot of every lane, in the chart's lane order.
    pub fn lanes(&self) -> &[LaneSlot] {
        &self.lanes
    }

    /// The offsets the game sets this frame: the turntable angles as they stand, with `field` as
    /// the play screen worked it out.
    ///
    /// An angle is whole degrees, a sixth of the turn counter cut down
    /// (`KeyInputProccessor.java:104`). A mode has one for each turntable it has and none for the
    /// rest, which the reference never writes either.
    pub fn offsets(&self, field: FieldOffsets) -> PlayOffsets {
        let scratch_angle = std::array::from_fn(|turntable| self.scratch_lanes[turntable].map(|_| (self.scratch[turntable] / SCRATCH_UNITS_PER_DEGREE) as f32));
        PlayOffsets { scratch_angle, field }
    }

    /// Switches the timers `event` switches, as of `now_us` on the scene clock.
    pub fn apply(&mut self, timers: &mut TimerState, now_us: i64, event: SceneEvent<'_>) {
        match event {
            SceneEvent::InputOpen => timers.switch(timer_id::STARTINPUT, true, now_us),
            SceneEvent::Standby => stand_in_neutral(timers, now_us),
            SceneEvent::Preview { pressed } => self.preview(timers, now_us, pressed),
            SceneEvent::Ready => {
                timers.off(CHART_PREVIEW);
                timers.set_on(timer_id::READY, now_us);
                self.pm_last_notes = [0; LANE_TIMER_SIDES];
                self.pm_judge = 0;
            }
            SceneEvent::Started { start_offset_us } => {
                timers.set_on(timer_id::PLAY, now_us - start_offset_us);
                timers.set_on(timer_id::RHYTHM, now_us - start_offset_us);
                self.judging = true;
                self.playing_us = Some(now_us);
            }
            SceneEvent::Playing(frame) => self.playing(timers, now_us, frame),
            SceneEvent::LastNotePassed => timers.switch(timer_id::ENDOFNOTE_1P, true, now_us),
            SceneEvent::Failed => timers.set_on(timer_id::FAILED, now_us),
            SceneEvent::MusicEnded => {
                timers.set_on(timer_id::MUSIC_END, now_us);
                PM_TIMERS.into_iter().for_each(|motion| timers.off(motion));
            }
            SceneEvent::Fadeout => timers.switch(timer_id::FADEOUT, true, now_us),
            SceneEvent::JudgeStopped => {
                self.key_beams_stopped |= self.judging;
                self.judging = false;
            }
            SceneEvent::Seeked { chart_us } => self.seeked(timers, chart_us),
            SceneEvent::Input(keys) => self.input(timers, now_us, keys),
            SceneEvent::Pressed { lane } => self.pressed(timers, now_us, lane),
            SceneEvent::Judged(judgement) => self.judged(timers, now_us, judgement),
            SceneEvent::LongNote { lane, held } => {
                if let Some(hold) = self.judged_lane_timer(LaneTimer::Hold, lane) {
                    timers.switch(hold, held, now_us);
                }
            }
            SceneEvent::Hcn { lane, pass } => self.hcn(timers, now_us, lane, pass),
        }
    }

    /// The timer of one kind for `lane`, for something only a running judge reports. A judge that
    /// is not running reports nothing, so nothing is switched for it.
    fn judged_lane_timer(&self, kind: LaneTimer, lane: usize) -> Option<TimerId> {
        self.lanes.get(lane).filter(|_| self.judging).and_then(|slot| kind.of(*slot))
    }

    /// The chart preview's frame (`BMSPlayer.java:481-497`).
    fn preview(&self, timers: &mut TimerState, now_us: i64, pressed: bool) {
        match (timers.is_on(CHART_PREVIEW), pressed) {
            (true, false) => timers.off(CHART_PREVIEW),
            (false, true) => timers.set_on(CHART_PREVIEW, now_us - self.first_note_us + CHART_PREVIEW_LEAD_US),
            _ => {}
        }
    }

    /// One frame of the run: the play timer, the rhythm timer, the gauge and a pop'n character, in
    /// the reference's order.
    ///
    /// The reference moves the play timer by what the scene clock ran ahead of the chart this frame
    /// (`deltatime * (100 - playspeed) / 100`), which keeps the timer's elapsed time equal to the
    /// chart time. The chart time here comes from the audio clock, so the timer is set from it
    /// outright, which is the same thing without the two clocks' drift.
    fn playing(&mut self, timers: &mut TimerState, now_us: i64, frame: PlayingFrame) {
        let delta_us = now_us - self.playing_us.unwrap_or(now_us);
        self.playing_us = Some(now_us);
        timers.set_on(timer_id::PLAY, now_us - frame.chart_us);

        let slowed = RHYTHM_PERCENT - frame.bpm * f64::from(frame.play_speed) / SECONDS_PER_MINUTE;
        self.rhythm_us = (self.rhythm_us as f64 + delta_us as f64 * slowed / RHYTHM_PERCENT) as i64;
        timers.set_on(timer_id::RHYTHM, self.rhythm_us);
        if self.section_us(self.sections_passed).is_some_and(|section_us| section_us <= frame.chart_us) {
            self.sections_passed += 1;
            timers.set_on(timer_id::RHYTHM, now_us);
            self.rhythm_us = now_us;
        }

        timers.switch(timer_id::GAUGE_MAX_1P, frame.gauge_max, now_us);
        self.pm_chara(timers, now_us, frame);
    }

    /// When one bar line is reached on the play timer, weighed by the practice frequency the way the
    /// reference weighs it (`sectiontimes[sections] * (100 / freq)`, the division whole). `None` past
    /// the last bar line.
    fn section_us(&self, index: usize) -> Option<i64> {
        let weight = FULL_PERCENT.checked_div(self.practice_freq)?;
        self.sections_us.get(index).map(|section_us| section_us * i64::from(weight))
    }

    /// A pop'n character's frame (`PomyuCharaProcessor.updateTimer`).
    ///
    /// A side leaves neutral only as a neutral cycle comes round, and only when a note has gone by
    /// since it last went neutral; it goes back once the motion it left for has run a cycle.
    fn pm_chara(&mut self, timers: &mut TimerState, now_us: i64, frame: PlayingFrame) {
        let ran_ms = |timers: &TimerState, motion: TimerId| timers.is_on(motion).then(|| (now_us - timers.value_us(motion)) / MICROS_PER_MILLI);
        let cycle_ms = |motion: TimerId| {
            let index = usize::try_from(motion.get() - timer_id::PM_CHARA_1P_NEUTRAL.get()).unwrap_or_default();
            self.pm_motion_cycles_ms.get(index).copied().unwrap_or(PM_MOTION_CYCLE_FLOOR_MS)
        };
        let comes_round = |timers: &TimerState, neutral: TimerId| {
            let cycle = cycle_ms(neutral);
            ran_ms(timers, neutral).is_some_and(|ran| ran >= cycle && ran % cycle < PM_NEUTRAL_WINDOW_MS)
        };

        let [first_last_notes, second_last_notes] = self.pm_last_notes;
        if comes_round(timers, timer_id::PM_CHARA_1P_NEUTRAL) && first_last_notes != frame.past_notes && self.pm_judge > 0 {
            let motion = match self.pm_judge {
                judge if judge <= PM_GREAT_LIMIT && frame.gauge_max => timer_id::PM_CHARA_1P_FEVER,
                judge if judge <= PM_GREAT_LIMIT => timer_id::PM_CHARA_1P_GREAT,
                PM_GOOD => timer_id::PM_CHARA_1P_GOOD,
                _ => timer_id::PM_CHARA_1P_BAD,
            };
            timers.set_on(motion, now_us);
            timers.off(timer_id::PM_CHARA_1P_NEUTRAL);
        }
        if comes_round(timers, timer_id::PM_CHARA_2P_NEUTRAL) && second_last_notes != frame.past_notes && self.pm_judge > 0 {
            let motion = if self.pm_judge <= PM_GOOD { timer_id::PM_CHARA_2P_BAD } else { timer_id::PM_CHARA_2P_GREAT };
            timers.set_on(motion, now_us);
            timers.off(timer_id::PM_CHARA_2P_NEUTRAL);
        }

        let sides = [(PM_FIRST_SIDE_MOTIONS.as_slice(), timer_id::PM_CHARA_1P_NEUTRAL), (PM_SECOND_SIDE_MOTIONS.as_slice(), timer_id::PM_CHARA_2P_NEUTRAL)];
        for (side, (motions, neutral)) in sides.into_iter().enumerate() {
            for &motion in motions {
                if ran_ms(timers, motion).is_some_and(|ran| ran >= cycle_ms(motion)) {
                    timers.set_on(neutral, now_us);
                    self.pm_last_notes[side] = frame.past_notes;
                    timers.off(motion);
                }
            }
        }
        timers.switch(timer_id::PM_CHARA_DANCE, true, now_us);
    }

    /// The run jumping to another place in the chart.
    fn seeked(&mut self, timers: &mut TimerState, chart_us: i64) {
        for slot in &self.lanes {
            LaneTimer::ALL.into_iter().filter_map(|kind| kind.of(*slot)).for_each(|timer| timers.off(timer));
        }
        self.sections_passed = (0..self.sections_us.len()).take_while(|index| self.section_us(*index).is_some_and(|section_us| section_us <= chart_us)).count();
    }

    /// One frame of keys: the key beams of every lane, then the turntables
    /// (`KeyInputProccessor.input`).
    ///
    /// A lane that is down lights its beam here only before the judge runs or while the game plays
    /// by itself; otherwise the judge lights it ([`SceneEvent::Pressed`]). A lane that is up puts
    /// its beam out here always. A turntable that changes hands relights its beam, and one turned
    /// with both keys at once changes hands every frame.
    fn input(&mut self, timers: &mut TimerState, now_us: i64, keys: &[LaneKeys]) {
        let lit_here = !self.judging || self.autoplay;
        for (lane, slot) in self.lanes.iter().enumerate() {
            let held = keys.get(lane).copied().unwrap_or_default();
            let mut pressed = false;
            let mut changed_hands = false;
            for dir in lane_keys(*slot).iter().copied().filter(|dir| !self.key_beams_stopped && held.is_down(*dir)) {
                pressed = true;
                if let Some(last) = slot.scratch.and_then(|turntable| self.scratch_key.get_mut(turntable))
                    && *last != Some(dir)
                {
                    changed_hands = true;
                    *last = Some(dir);
                }
            }
            let (Some(key_on), Some(key_off)) = (LaneTimer::KeyOn.of(*slot), LaneTimer::KeyOff.of(*slot)) else {
                continue;
            };
            if pressed {
                if lit_here && (!timers.is_on(key_on) || changed_hands) {
                    timers.set_on(key_on, now_us);
                    timers.off(key_off);
                }
            } else if timers.is_on(key_on) {
                timers.set_on(key_off, now_us);
                timers.off(key_on);
            }
        }
        self.turn_turntables(now_us, keys);
    }

    /// Turns every turntable by the time since the last frame of keys
    /// (`KeyInputProccessor.java:92-108`).
    ///
    /// The time is whole milliseconds. Left alone, the first turntable turns backwards by it and the
    /// second forwards; the second key adds twice the time and the first takes twice the time away,
    /// the second key winning when both are down. The stopped key beams do not stop a turntable.
    fn turn_turntables(&mut self, now_us: i64, keys: &[LaneKeys]) {
        let now_ms = now_us / MICROS_PER_MILLI;
        let Some(delta_ms) = self.input_ms.replace(now_ms).map(|last_ms| now_ms - last_ms) else {
            return;
        };
        for (turntable, lane) in self.scratch_lanes.into_iter().enumerate() {
            let Some(held) = lane.map(|lane| keys.get(lane).copied().unwrap_or_default()) else {
                continue;
            };
            let idle = if turntable % LANE_TIMER_SIDES == 0 { SCRATCH_TURN - delta_ms } else { delta_ms };
            let turned = match (held.backward, held.forward) {
                (true, _) => delta_ms * SCRATCH_KEY_GAIN,
                (false, true) => SCRATCH_TURN - delta_ms * SCRATCH_KEY_GAIN,
                (false, false) => 0,
            };
            self.scratch[turntable] = (self.scratch[turntable] + idle + turned) % SCRATCH_TURN;
        }
    }

    /// The judge taking a key going down (`KeyInputProccessor.inputKeyOn`): the lane's beam lights
    /// unless it is already lit, and a turntable's is relit every time.
    fn pressed(&mut self, timers: &mut TimerState, now_us: i64, lane: usize) {
        let Some(slot) = self.lanes.get(lane).copied().filter(|_| self.judging && !self.key_beams_stopped) else {
            return;
        };
        let (Some(key_on), Some(key_off)) = (LaneTimer::KeyOn.of(slot), LaneTimer::KeyOff.of(slot)) else {
            return;
        };
        if !timers.is_on(key_on) || slot.scratch.is_some() {
            timers.set_on(key_on, now_us);
            timers.off(key_off);
        }
    }

    /// A judgement (`JudgeManager.java:673-692`, `BMSPlayer.java:1017-1040`).
    ///
    /// The lane's bomb is restarted for a judgement at least as good as the skin asks for, and is
    /// never switched off. The lane's region restarts its judge and combo timers whatever the
    /// judgement was, and with three regions the other two lose their combo timers. Then the full
    /// combo and the score timers follow what the run has reached.
    fn judged(&mut self, timers: &mut TimerState, now_us: i64, judgement: Judgement) {
        if !self.judging {
            return;
        }
        let code = judge_code(judgement.judge);
        if code <= self.judge_timer
            && let Some(bomb) = self.lanes.get(judgement.lane).and_then(|slot| LaneTimer::Bomb.of(*slot))
        {
            timers.set_on(bomb, now_us);
        }
        if let Some(region) = judge_region(judgement.lane, self.lanes.len(), self.judge_regions) {
            timers.set_on(JUDGE_TIMERS[region], now_us);
            if self.judge_regions >= EXCLUSIVE_COMBO_REGIONS {
                COMBO_TIMERS.into_iter().enumerate().filter(|(other, _)| *other != region).for_each(|(_, combo)| timers.off(combo));
            }
            timers.set_on(COMBO_TIMERS[region], now_us);
        }

        let reached = judgement.reached;
        let switched = [
            (timer_id::FULLCOMBO_1P, reached.full_combo),
            (timer_id::SCORE_A, reached.rank_a),
            (timer_id::SCORE_AA, reached.rank_aa),
            (timer_id::SCORE_AAA, reached.rank_aaa),
            (timer_id::SCORE_BEST, reached.best),
            (timer_id::SCORE_TARGET, reached.target),
        ];
        for (timer, on) in switched {
            timers.switch(timer, on, now_us);
        }
        self.pm_judge = code + 1;
    }

    /// A lane's hell charge note timers (`JudgeManager.java:306-338`): the one for where the lane
    /// stands is switched on and left running, and the other is switched off.
    fn hcn(&mut self, timers: &mut TimerState, now_us: i64, lane: usize, pass: HcnPass) {
        let (Some(active), Some(damage)) = (self.judged_lane_timer(LaneTimer::HcnActive, lane), self.judged_lane_timer(LaneTimer::HcnDamage, lane)) else {
            return;
        };
        timers.switch(active, pass == HcnPass::Active, now_us);
        timers.switch(damage, pass == HcnPass::Damage, now_us);
    }
}

/// The keys a lane has, in the order the reference lists them (`LaneProperty.laneToKey`): one for a
/// key lane, two for a turntable.
fn lane_keys(slot: LaneSlot) -> &'static [ScratchDir] {
    const KEY: [ScratchDir; 1] = [ScratchDir::Forward];
    const TURNTABLE: [ScratchDir; 2] = [ScratchDir::Forward, ScratchDir::Backward];
    if slot.scratch.is_some() { &TURNTABLE } else { &KEY }
}

/// Puts both sides of a pop'n character in neutral unless both already are
/// (`BMSPlayer.java:518-521`).
fn stand_in_neutral(timers: &mut TimerState, now_us: i64) {
    if timers.is_on(timer_id::PM_CHARA_1P_NEUTRAL) && timers.is_on(timer_id::PM_CHARA_2P_NEUTRAL) {
        return;
    }
    timers.set_on(timer_id::PM_CHARA_1P_NEUTRAL, now_us);
    timers.set_on(timer_id::PM_CHARA_2P_NEUTRAL, now_us);
}

#[cfg(test)]
mod tests;

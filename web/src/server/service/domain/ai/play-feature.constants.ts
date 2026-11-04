/**
 * Thresholds of the AI features that read play analyses (plan 2026-10-19-web-ai-features.md 5.1-5.4 and 5.8, W5 v1 draft). They pick
 * the deterministic numbers a digest carries, so changing one changes the digest and with it the cache key; bump
 * PLAY_FEATURE_ANALYSIS_VERSION when the meaning of a value changes. 2: digests read the weakness profile (2026-11-04).
 */
export const PLAY_FEATURE_ANALYSIS_VERSION = 2

/** Fewest analysed plays before the report or the gauge advice calls an AI: MIN_TAG_PLAYS, the fewest plays a tag claim stands on. */
export const PLAY_REPORT_MIN_PLAYS = 5
/** The report shows this many strengths and weaknesses, habits and preferred patterns. */
export const PLAY_REPORT_TOP_ITEMS = 5
/** A chart played this many times or more counts as one the player keeps choosing (plan 4.4 "선호 패턴" 1). */
export const PLAY_REPORT_REPEAT_PLAYS = 2
/** Tags a preference is shown for need at least this share difference over all played charts. */
export const PLAY_REPORT_PREFERENCE_MIN_DIFF = 0.01
/** Lanes shown with the largest bias first. */
export const PLAY_LANE_BIAS_TOP = 3

/** The coach names at most this many problem segments to the AI (plan 5.2). */
export const COACH_PROBLEM_SEGMENTS = 5
/** A problem segment names at most this many BP runs that start in it, the longest first. */
export const COACH_SEGMENT_RUNS = 2
/** A problem segment names at most this many gauge drops that end in it. */
export const COACH_SEGMENT_DROPS = 2
/** A problem segment names at most this many profile weaknesses its main tags share. */
export const COACH_SEGMENT_PROFILE_KEYS = 3

/** The comparison names at most this many segments with the largest EX difference (plan 5.3). */
export const COMPARE_GAP_SEGMENTS = 5

/** The gauge advice reads this many recent analysed plays. */
export const GAUGE_ADVICE_RECENT_PLAYS = 100
/** A lane option is described with its tag residuals from this many plays of it. */
export const GAUGE_ADVICE_OPTION_MIN_PLAYS = 3
/** Tags shown per lane option, the largest residual first. */
export const GAUGE_ADVICE_OPTION_TAGS = 3
/** Gauge-drop context tags of the weakness profile the advice names, the strongest evidence and lift first. */
export const GAUGE_ADVICE_DROP_TAGS = 3

/** The judge habit names this many tags whose strong timing strays furthest from the player's own (plan 2026-11-04 2.6 feature 4). */
export const JUDGE_HABIT_TAG_TIMING_TOP = 5

/**
 * The longest digest, in characters of its JSON, a feature hands an AI (plan 2026-10-19 7 "최소 토큰" `DIGEST_MAX_CHARS`): the report
 * carries the most, every other feature reading play analyses half as much. Digests stay under it by their item limits.
 */
export const PLAY_DIGEST_MAX_CHARS = { REPORT: 12_000, OTHER: 6_000 } as const

/**
 * The weakness profile (plan 2026-11-04 2.5). PROFILE_VERSION goes up whenever what it computes changes, so cached profiles of a
 * lower version are computed again, and the version is part of the input fingerprint. 2: the EX and BP limits are read off the fitted
 * curve between the middles of the two bins around the crossing, not at the lower edge of the bin after it (2026-11-04 training
 * difficulty plan M8). 3: the comfortable density is cut to the lower of the two limits, so it is never above a limit.
 */
export const PROFILE_VERSION = 3
/** The only window profiles are cached for: the last PROFILE_WINDOW_DAYS days. */
export const PROFILE_WINDOW_KEY = 'recent90'
export const PROFILE_WINDOW_DAYS = 90
/** The newest plays of the window a profile reads at most. */
export const PROFILE_MAX_PLAYS = 300
/** The trend compares the plays of the last PROFILE_RECENT_DAYS days with the older plays of the window. */
export const PROFILE_RECENT_DAYS = 30
/** Fewest analysed 7-key plays in the window before a profile is more than a notice (the need `traced_plays`). */
export const PROFILE_MIN_PLAYS = 10

/** One chart's plays weigh in a cell's estimate as at most this many notes, so replaying one chart cannot carry a cell. */
export const CHART_WEIGHT_CAP_NOTES = 500
/** A cell is tested only with this many notes, charts and plays behind it (`insufficient` below). */
export const MIN_CELL_NOTES = 300
export const MIN_CELL_CHARTS = 5
export const MIN_CELL_PLAYS = 5
/** The sample levels above `low`: `high` needs both of the first pair, `medium` both of the second. */
export const SAMPLE_LEVEL_EDGES = { HIGH_CHARTS: 20, HIGH_NOTES: 3000, MEDIUM_CHARTS: 10, MEDIUM_NOTES: 1000 } as const
/** The false discovery rate Benjamini–Hochberg holds within each test family. */
export const FDR_Q = 0.1

/** Which test family a cell kind's p-value is corrected in (plan 2.5.3); `density:limit` is a curve and takes no test. */
export const PROFILE_TEST_FAMILY = {
    tag: 'accuracy',
    solo: 'accuracy',
    combo: 'accuracy',
    lane: 'accuracy',
    hand: 'accuracy',
    bpm: 'accuracy',
    soflan: 'accuracy',
    fatigue: 'accuracy',
    density: 'accuracy',
    bp_context: 'accuracy',
    gauge_context: 'accuracy',
    timing: 'timing',
} as const

/** The density limit curve: local NPS bins this wide, each used with this many notes and charts. */
export const PROFILE_NPS_BIN_WIDTH = 2
export const MIN_BIN_NOTES = 150
export const MIN_BIN_CHARTS = 3
/** A density curve with fewer used bins than this is `insufficient`. */
export const PROFILE_DENSITY_MIN_BINS = 3
/** The comfortable density keeps the fitted EX ratio within this of the baseline; the limit is where it falls this far below it. */
export const COMFORT_EX_DROP = 0.03
export const LIMIT_EX_DROP = 0.1
/** The BP limit is where the fitted BP rate reaches this. */
export const LIMIT_BP_RATE = 0.08
/** Sustained against burst density compares a bin only when both contexts hold this many notes there. */
export const MIN_BIN_CONTEXT_NOTES = 50

/** A failure or gauge-drop context tag is a candidate with this many context notes and a lift this large. */
export const MIN_CONTEXT_NOTES = 30
export const CONTEXT_LIFT_MIN = 1.5
/** Added to each count of a chart's lift so a chart where the tag never came before a failure still has a finite log lift. */
export const CONTEXT_LIFT_PSEUDO_COUNT = 0.5

/** A tag pair is a weakness of its own only when it loses at least this much more than the worse of its two tags. */
export const COMBO_INTERACTION_MIN = 0.02

/** The severity of a weakness counts notes up to this many. */
export const SEVERITY_NOTES_CAP = 3000
/**
 * What one unit of each cell unit is worth in severity, an EX ratio being 1: a timing deviation of 1 ms counts as 1% EX, and a
 * context's natural log lift a tenth of itself, so cells of different units sort in one list.
 */
export const SEVERITY_UNIT_SCALE = { ex_residual: 1, ratio: 1, ms: 0.01, lift: 0.1, nps: 1 } as const

export const PROFILE_WEAKNESS_TOP = 8
export const PROFILE_STRENGTH_TOP = 5
/** The profile digest names this many strong timing tags and this many failure or gauge-drop context tags. */
export const PROFILE_DIGEST_TIMING_TOP = 3
export const PROFILE_DIGEST_CONTEXT_TOP = 3

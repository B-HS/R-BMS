import { CHART_PATTERN_MAX_TIME_US } from '@server/dto/chart-pattern'

/**
 * Every threshold of the deterministic analysis (plan 2026-10-19-web-ai-features.md 4.2-4.6, v1 draft). Changing any value
 * changes stored results, so ANALYSIS_VERSION goes up with it and stored features of a lower version are computed again.
 */
export const ANALYSIS_VERSION = 1

/**
 * The generation of one play's stored analysis (`play_analysis.summary`). It goes up with ANALYSIS_VERSION and also when the summary
 * of a play changes while the pattern classification does not, so chart features are not computed again for nothing. 2: segment EX,
 * FAST/SLOW and tags, timing by lane, gauges read only up to an empty gauge's failure (2026-10-20).
 */
export const PLAY_ANALYSIS_VERSION = 2

/** A local sixteenth note lasts `S16_US_PER_BPM / bpm` microseconds (`measure_us(bpm) / 16`, rbms-model). */
export const S16_US_PER_BPM = 15_000_000

/** The tempo a gap is measured in when the chart's own tempo is zero or negative there. */
export const FALLBACK_BPM = 120

/** A gap counts as within N sixteenths up to this share past it, so microsecond rounding of note times does not split a run. */
export const GAP_TOLERANCE_RATIO = 0.01

export const CHORD_MIN_KEYS = 2
/** Chords of this many keys or more share one tag, `chord_4plus`. */
export const CHORD_TOP_KEYS = 4

export const JACK_MAX_GAP_S16 = 2
export const JACK_MIN_RUN = 3

export const TRILL_MAX_GAP_S16 = 1
export const TRILL_MIN_RUN = 4

export const STAIR_MAX_GAP_S16 = 1
export const STAIR_MIN_RUN = 4

export const STREAM_MAX_GAP_S16 = 1
export const STREAM_MIN_RUN = 8

export const SCRATCH_LINK_S16 = 1
export const SCRATCH_RUN_GAP_S16 = 2
export const SCRATCH_RUN_MIN = 3

export const LN_FOLLOW_S16 = 1

export const SOFLAN_BPM_RATIO = 1.25
export const SOFLAN_STOP_MIN_MS = 100
export const SOFLAN_TAIL_MS = 2000
/** The scroll speed of a timeline that never changed it. Any other speed counts as soflan. */
export const NEUTRAL_SCROLL = 1

export const DENSE_PERCENTILE = 90
/** A dense window also holds more notes than this percentile of the windows (the median), so even density is not dense. */
export const DENSE_BASELINE_PERCENTILE = 50
/** The window notes are counted in for density: whole seconds of chart time, the bins of `note_density` (rbms-chart). */
export const DENSITY_WINDOW_US = 1_000_000
/** The most density windows a chart has: those of its longest allowed time (`CHART_PATTERN_MAX_TIME_US`). A later note counts in the last one. */
export const MAX_DENSITY_WINDOWS = Math.floor(CHART_PATTERN_MAX_TIME_US / DENSITY_WINDOW_US) + 1

export const MINE_NEAR_S16 = 1

/** BEAT_7K lanes: 0 to 6 are the keys 1 to 7, 7 is the scratch. */
export const SP7_SCRATCH_LANE = 7
export const SP7_LANE_NAMES = ['1', '2', '3', '4', '5', '6', '7', 'S'] as const

/** The default hand of each lane (plan 4.4): the scratch and keys 1-3 left, key 4 shared, keys 5-7 right. The real fingering is unknown. */
export const HAND = { LEFT: 'left', RIGHT: 'right', SHARED: 'shared' } as const
export type Hand = (typeof HAND)[keyof typeof HAND]
export const SP7_LANE_HANDS: readonly Hand[] = [HAND.LEFT, HAND.LEFT, HAND.LEFT, HAND.SHARED, HAND.RIGHT, HAND.RIGHT, HAND.RIGHT, HAND.LEFT]

export const SEGMENT_COUNT = 32
/** A segment names this many of its most frequent tags. */
export const SEGMENT_TOP_TAGS = 3

/** Seconds of chart time are grouped by how many judged notes they hold, this many notes per group, for accuracy against density. */
export const DENSITY_BUCKET_WIDTH = 5
/** Seconds holding at least this many notes share the top density group. */
export const DENSITY_BUCKET_TOP = 25

/** A timing histogram reaches this far either side of the note, one bucket per whole millisecond (`TIMING_DISTRIBUTION_RANGE_MS`, rbms-play). */
export const TIMING_HISTOGRAM_RANGE_MS = 150

export const GAUGE_DROP_WINDOW_MS = 4000
/** A drop counts when the gauge falls this share of its range or more inside one window. */
export const GAUGE_DROP_MIN_RATIO = 0.15

/** The share of the lane travel time that a skin labels GREEN (`IntegerPropertyFactory.java:555-556`, `scroll.rs` in rbms-chart). */
export const GREEN_DISPLAY_FACTOR = 0.6

export const REPORT_RECENT_PLAYS = 100
export const MIN_TAG_NOTES = 300
export const MIN_TAG_PLAYS = 5
export const MIN_TIMING_PLAYS = 10
export const MIN_TIMING_NOTES = 2000
/** The z value of a two-sided 95% Wilson score interval. */
export const WILSON_Z_95 = 1.959964

/** Ratios stored in results keep this many decimal places. */
export const RATIO_DECIMALS = 4

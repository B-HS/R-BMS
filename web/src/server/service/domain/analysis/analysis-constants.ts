import { CHART_PATTERN_MAX_TIME_US } from '@server/dto/chart-pattern'

/**
 * Every threshold of the deterministic analysis (plan 2026-10-19-web-ai-features.md 4.2-4.6, v1 draft). Changing any value
 * changes stored results, so ANALYSIS_VERSION goes up with it and stored features of a lower version are computed again.
 * 2: chart features add sliding-window NPS, tag pairs and the main BPM band (2026-11-04).
 */
export const ANALYSIS_VERSION = 2

/**
 * The generation of one play's stored analysis (`play_analysis.summary`). It goes up with ANALYSIS_VERSION and also when the summary
 * of a play changes while the pattern classification does not, so chart features are not computed again for nothing. 2: segment EX,
 * FAST/SLOW and tags, timing by lane, gauges read only up to an empty gauge's failure (2026-10-20). 3: sliding-window NPS cells,
 * timing moments by tag/lane/hand, BP runs, gauge-drop contexts, tag pairs, BPM bands, soflan phases, training sections (2026-11-04).
 */
export const PLAY_ANALYSIS_VERSION = 3

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

/** The centred window of a note's local NPS (plan 2026-11-04 2.2.1), the one its NPS bin is read from. */
export const LOCAL_NPS_WINDOW_US = 2_000_000
/** The short window that tells a burst: notes far denser over it than over SUSTAIN_WINDOW_US. */
export const BURST_WINDOW_US = 1_000_000
/** The long window that tells sustained density. */
export const SUSTAIN_WINDOW_US = 8_000_000
/** A note is in a burst when its BURST_WINDOW_US NPS is at least this many times its SUSTAIN_WINDOW_US NPS. */
export const BURST_RATIO = 1.5
/** Sustained density needs at least this local NPS. */
export const SUSTAIN_MIN_NPS = 6
/** Sustained density holds when the SUSTAIN_WINDOW_US NPS is at least this share of the local NPS. */
export const SUSTAIN_RATIO = 0.85

/** The density context of a note: a short burst, sustained density, or neither. */
export const DENSITY_CONTEXT = { BURST: 'burst', SUSTAINED: 'sustained', MIXED: 'mixed' } as const
export type DensityContext = (typeof DENSITY_CONTEXT)[keyof typeof DENSITY_CONTEXT]

/** Local NPS bins are this many notes per second wide. */
export const NPS_BIN_WIDTH = 1
/** Notes of this local NPS or more share the top bin. */
export const NPS_BIN_TOP = 40

/** The windows, in milliseconds, of a play's peak NPS: the most judged notes any window starting at a note holds. */
export const PEAK_NPS_WINDOWS_MS = [500, 1000, 2000, 4000, 8000] as const
/** The window of the peak rows per second, rows being distinct note times. */
export const PEAK_RPS_WINDOW_MS = 2000
/** Peak NPS values keep this many decimal places. */
export const PEAK_NPS_DECIMALS = 2
/** NPS values reported with a context (a run's start, a gauge drop's mean) keep this many decimal places. */
export const NPS_SUMMARY_DECIMALS = 1

/** The hand of each BEAT_7K lane for hand figures: keys 1-3 left, key 4 shared, keys 5-7 right, the scratch its own. */
export const HAND_KEYS = ['left', 'shared', 'right', 'scratch'] as const
export type HandKey = (typeof HAND_KEYS)[number]
export const SP7_LANE_HAND_KEYS: readonly HandKey[] = ['left', 'left', 'left', 'shared', 'right', 'right', 'right', 'scratch']

/** The lower edges of the BP run length buckets: 1, 2, 3-4, 5-9 and 10 or more. */
export const BP_RUN_BUCKETS = [1, 2, 3, 5, 10] as const
/** A BP run is listed when it is at least this many notes long. */
export const BP_RUN_MIN_REPORT = 3
/** The most BP runs listed, longest first. */
export const BP_RUN_TOP = 8
/** A run's notes before it count towards the failure context only when the run is at least this many notes long. */
export const BP_RUN_CONTEXT_MIN = 2
/** Play has recovered after a run once this many PGREAT or GREAT come in a row. */
export const RECOVERY_STREAK = 10
/** The judged notes before a run's first note that tell what led into it. */
export const PRECEDING_NOTES = 8
/** A run, a gauge drop or a section names this many of its most frequent tags. */
export const CONTEXT_TOP_TAGS = 3

/** A gauge drop's notes are read from this long before the drop starts. */
export const GAUGE_DROP_LOOKBACK_MS = 1000

/** The lower edges of the BPM bands notes are grouped by. */
export const BPM_BAND_EDGES = [120, 150, 180, 210] as const
/** The BPM band names, one more than BPM_BAND_EDGES: below the first edge, between each pair, and from the last edge on. */
export const BPM_BANDS = ['<120', '120-149', '150-179', '180-209', '210+'] as const
export type BpmBand = (typeof BPM_BANDS)[number]

/** The phases around a soflan event: the notes this long before it, and after it (past its stop). */
export const SOFLAN_PHASE_MS = 2000
/** The settled phase after a soflan event ends this long after the event's stop. */
export const SOFLAN_SETTLED_MS = 6000
/** A speed setting change this soon after a soflan event counts as adjusting to it. */
export const SOFLAN_ADJUST_WINDOW_MS = 3000

/** Sustained notes of a chart follow one another when they are at most this far apart. */
export const SUSTAIN_GAP_MAX_US = 500_000
/** The NPS percentiles a chart's features keep. */
export const CHART_NPS_PERCENTILES = { MEDIAN: 50, HIGH: 90 } as const
/** A chart's features keep this many of its most frequent tag pairs. */
export const CHART_COMBO_TOP = 10

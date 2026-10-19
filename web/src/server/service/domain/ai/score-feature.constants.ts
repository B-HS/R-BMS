/**
 * Thresholds and weights of the score-row AI features (plan 2026-10-19-web-ai-features.md 5.4-5.7 and 5.10, v1 draft). They pick
 * the deterministic numbers a digest carries, so changing one changes the digest and with it the cache key; bump the feature's
 * analysis version when the meaning of a value changes.
 */
export const SCORE_FEATURE_ANALYSIS_VERSION = 1

/** A level counts as cleared when at least this share of its charts carry a clear lamp (plan 5.5 `FRONTIER_CLEAR_RATE`). */
export const FRONTIER_CLEAR_RATE = 0.5
/** Candidates come from this many levels either side of the frontier level. */
export const FRONTIER_WINDOW_LEVELS = 1
/** The digest shows the level summary this many levels either side of the frontier. */
export const FRONTIER_DIGEST_LEVELS = 2

export const RECOMMEND_CANDIDATES = 40
export const RECOMMEND_PICKS = 10
export const RECOMMEND_SHOWN_WITHOUT_AI = 10

/** Base score of a candidate by how far its lamp is from a clear: a failed chart was close, an unplayed one is open, a clear wants a better lamp. */
export const CANDIDATE_KIND_WEIGHT = { failed: 3, unplayed: 2, lamp_up: 1 } as const
/** Score taken off per level between a candidate and the frontier. */
export const LEVEL_DISTANCE_WEIGHT = 1.5
/** Score added for a failed chart's EX rate (0 to 1), so a failed chart played closer to a clear ranks first. */
export const FAILED_EX_WEIGHT = 1
export const CANDIDATE_SCORE_DECIMALS = 3

/** A chart's tags shown with it: the top few by share of its judged notes, from a share of this or more. */
export const CHART_TOP_TAGS = 3
export const CHART_TAG_MIN_RATIO = 0.05
/** Tags that say little about a chart on their own and are left out of its top tags. */
export const CHART_TAGS_NOT_SHOWN = ['ln_end', 'with_scratch'] as const

export const FOLDER_CANDIDATES = 60
/** A focus-tag practice folder looks this many levels below the frontier and FRONTIER_WINDOW_LEVELS above it. */
export const FOCUS_TAG_LEVELS_BELOW = 2
export const FOCUS_TAG_MIN_RATIO = 0.1

export const GOAL_REMAINING_FOR_AI = 20
export const GOAL_PACE_DAYS = 28
export const GOAL_PACE_WEEKS = 4

export const WEEKLY_LIST_MAX = 10

export const JUDGE_HABIT_RECENT_PLAYS = 100
/** FAST and SLOW judgments (PGREAT not counted) needed before the FAST share is graded. */
export const JUDGE_HABIT_MIN_FAST_SLOW = 500
/** The plays with an average judge distance in each half before its change is graded. */
export const AVGJUDGE_MIN_HALF_PLAYS = 5
/** The client rule generation from which `avgjudge` and `extra.mean_offset_us` mean what they do now (ADR 2026-10-15 avgjudge). */
export const REFERENCE_RULE_VERSION = 4
/** An offset change smaller than this many milliseconds is not suggested. */
export const OFFSET_SUGGEST_MIN_MS = 1
/** Plays are grouped by their GREEN number in buckets this wide. */
export const GREEN_BUCKET_WIDTH = 50

export const PERCENT_DECIMALS = 2
export const MILLIS_DECIMALS = 1

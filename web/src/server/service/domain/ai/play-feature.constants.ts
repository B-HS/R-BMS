/**
 * Thresholds of the AI features that read play analyses (plan 2026-10-19-web-ai-features.md 5.1-5.4 and 5.8, W5 v1 draft). They pick
 * the deterministic numbers a digest carries, so changing one changes the digest and with it the cache key; bump
 * PLAY_FEATURE_ANALYSIS_VERSION when the meaning of a value changes.
 */
export const PLAY_FEATURE_ANALYSIS_VERSION = 1

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

/** The comparison names at most this many segments with the largest EX difference (plan 5.3). */
export const COMPARE_GAP_SEGMENTS = 5

/** The gauge advice reads this many recent analysed plays. */
export const GAUGE_ADVICE_RECENT_PLAYS = 100
/** A lane option is described with its tag residuals from this many plays of it. */
export const GAUGE_ADVICE_OPTION_MIN_PLAYS = 3
/** Tags shown per lane option, the largest residual first. */
export const GAUGE_ADVICE_OPTION_TAGS = 3

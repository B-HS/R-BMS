import { CHART_PATTERN_SCRATCH_LANE } from '@server/dto/chart-pattern'
import type { TrainingIntensity, TrainingLaneHint, TrainingPatternKind } from '@server/dto/training'
import type { PatternTag } from '@server/service/domain/analysis/pattern-tags'

/**
 * Every fixed value of the training chart generator (plan 2026-11-04 3.5-3.10). The same spec, seed and GENERATOR_VERSION give the
 * same bytes, so a change to any value that moves a note or a byte raises GENERATOR_VERSION.
 */
export const GENERATOR_VERSION = 1

/** The client parser generation the pattern documents of training charts are written for (`PARSER_VERSION`, rbms-library songdb). */
export const TRAINING_PARSER_VERSION = 5

/** A training chart id is this prefix and 32 random hex characters (plan 4.1). */
export const TRAINING_ID_PREFIX = 'tc_'
/** The hex characters of the id the chart's subtitle and file names carry. */
export const TRAINING_SHORT_ID_LENGTH = 8

/** A training chart lasts one minute: a count-in measure, then the notes, then a quiet tail. */
export const TRAINING_LENGTH_US = 60_000_000
export const TRAINING_TAIL_US = 1_000_000
/** Measure 0 is the count-in; notes start at measure 1. */
export const TRAINING_FIRST_NOTE_MEASURE = 1
export const TRAINING_COUNT_IN_CLICKS = 4

/** A measure of length 1.0 lasts `MICROS_PER_MEASURE_AT_ONE_BPM / bpm` microseconds (`US_PER_MEASURE_NUM`, rbms-model). */
export const MICROS_PER_MEASURE_AT_ONE_BPM = 240_000_000
export const MICROS_PER_SECOND = 1_000_000
export const BEATS_PER_MEASURE = 4
export const SECONDS_PER_MINUTE = 60

export const TRAINING_BPM_MIN = 100
export const TRAINING_BPM_MAX = 220
/** The base tempo is a multiple of this, so one minute at a rate of 1 is a whole number of measures (`bpm / 4`). */
export const TRAINING_BPM_STEP = 4
/** The tempo rates a section may take against the base tempo; every one of them keeps a multiple of 4 a whole number. */
export const TRAINING_BPM_RATIOS = [0.5, 0.75, 1, 1.25, 1.5, 2] as const
export const TRAINING_NEUTRAL_BPM_RATIO = 1
export const TRAINING_EFFECTIVE_BPM_MIN = 60
export const TRAINING_EFFECTIVE_BPM_MAX = 300

export const TRAINING_WEIGHT_MIN = 1
export const TRAINING_WEIGHT_MAX = 10
export const TRAINING_SECTION_MIN_MEASURES = 1

export const TRAINING_DEFAULT_LIMIT_NPS = 12
export const TRAINING_COMFORT_FALLBACK_RATIO = 0.7
export const TRAINING_OVER_RATIO = 1.15
/** The NPS range of each intensity, as shares of the player's comfort and limit density. */
export const TRAINING_INTENSITY_RANGE = {
    build: { min: { of: 'comfort', ratio: 0.6 }, max: { of: 'comfort', ratio: 1 } },
    push: { min: { of: 'comfort', ratio: 0.8 }, max: { of: 'limit', ratio: 1 } },
    over: { min: { of: 'comfort', ratio: 1 }, max: { of: 'limit', ratio: TRAINING_OVER_RATIO } },
} as const satisfies Record<TrainingIntensity, Record<'min' | 'max', { of: 'comfort' | 'limit'; ratio: number }>>
export const TRAINING_NPS_FLOOR = 2
export const TRAINING_NPS_CEIL = 40
export const TRAINING_REST_NPS_MIN = 1
export const TRAINING_REST_NPS_MAX = 3
/** NPS values a spec stores keep this many decimal places. */
export const TRAINING_NPS_DECIMALS = 2

export const TRAINING_CHORD_SIZE_MIN = 1
export const TRAINING_CHORD_SIZE_MAX = 4
/** A chord kind lays out at least this many keys a row. */
export const TRAINING_CHORD_KIND_MIN_SIZE = 2
export const TRAINING_CHORD_KINDS = ['chord_stream', 'chord_jack'] as const satisfies readonly TrainingPatternKind[]
export const TRAINING_SCRATCH_KINDS = ['scratch_solo', 'scratch_mixed', 'scratch_run'] as const satisfies readonly TrainingPatternKind[]
export const TRAINING_LN_KINDS = ['ln_hold', 'ln_release'] as const satisfies readonly TrainingPatternKind[]
export const TRAINING_MIX_RATIO_MAX = 0.5

/** Each section draws from its own generator, seeded by the chart seed and its index times this odd constant (the golden ratio). */
export const TRAINING_SECTION_SEED_STRIDE = 0x9e3779b1

/** Cells per measure a section may lay notes on. */
export const TRAINING_GRIDS = [4, 8, 12, 16, 24, 32] as const
/** Positions are counted in ticks of a measure: the least common multiple of TRAINING_GRIDS, so every cell is a whole tick. */
export const TRAINING_MEASURE_TICKS = 96
export const TRAINING_SIXTEENTH_TICKS = TRAINING_MEASURE_TICKS / 16
export const TRAINING_BEAT_TICKS = TRAINING_MEASURE_TICKS / BEATS_PER_MEASURE
/** The grids a run of a continuous kind is laid on, finest last: consecutive cells of a run are a sixteenth apart or closer. */
export const TRAINING_RUN_GRIDS = [16, 24, 32] as const
/** A rest is laid on quarter notes. */
export const TRAINING_REST_GRID = 4
/** Rows of one run of each continuous kind. */
export const TRAINING_RUN_ROWS = { stream: 16, trill: 8, stair: 7, jack: 4, chord_jack: 3, scratch_run: 4 } as const
export type TrainingContinuousKind = keyof typeof TRAINING_RUN_ROWS

export const TRAINING_SCRATCH_SOLO_KEY_SHARE = 0.25
/** A row of `scratch_mixed` is a key and a scratch. */
export const TRAINING_SCRATCH_MIXED_SHARE = 1
/** A row of `ln_release` is a long note's head and the hit after its end. */
export const TRAINING_LN_RELEASE_ROW_NOTES = 2
export const TRAINING_LN_HOLD_BEATS = 2
export const TRAINING_LN_RELEASE_BEATS = 1
/** A scratch after its key row, and a release hit after its long note's end, follow by this many sixteenths. */
export const TRAINING_FOLLOW_SIXTEENTHS = 1
/** A long note an `ln_ratio` turns a hit into lasts at least this many cells and at most a beat. */
export const TRAINING_LN_MIN_CELLS = 2
export const TRAINING_BURST_HIGH = 1.6
export const TRAINING_BURST_LOW = 0.4

/** The lanes a lane hint lays keys on (the scratch is laid only by the kinds and `scratch_ratio`). */
export const TRAINING_LANE_CANDIDATES = {
    all: [0, 1, 2, 3, 4, 5, 6],
    left: [0, 1, 2, 3],
    right: [3, 4, 5, 6],
    center: [1, 2, 3, 4, 5],
    wide: [0, 1, 2, 3, 4, 5, 6],
    adjacent: [0, 1, 2, 3, 4, 5, 6],
} as const satisfies Record<TrainingLaneHint, readonly number[]>
/** `wide` prefers a key at least this far from the one before, `adjacent` one exactly this far. */
export const TRAINING_WIDE_MIN_GAP = 3
export const TRAINING_ADJACENT_GAP = 1
/** The keys of each hand a long-note kind holds and hits with: key 4 is left out, as either hand may take it. */
export const TRAINING_HAND_LANES = { left: [0, 1, 2], right: [4, 5, 6] } as const
export const TRAINING_SCRATCH_LANE = CHART_PATTERN_SCRATCH_LANE
/** Keys 6 and 7 (channels 18 and 19): a chart with a note on one of them reads as 7 keys. */
export const TRAINING_SEVEN_KEY_LANES = [5, 6] as const
/** The key the 7-key check adds when a chart has neither. */
export const TRAINING_SEVEN_KEY_FALLBACK_LANE = 6

export const TRAINING_PLAY_LEVEL_MIN = 1
export const TRAINING_PLAY_LEVEL_MAX = 25

export const TRAINING_TITLE_PREFIX = 'R-BMS Training · '
export const TRAINING_TITLE_SEPARATOR = ' · '
export const TRAINING_GENRE = 'R-BMS TRAINING'
export const TRAINING_ARTIST = 'R-BMS'
export const TRAINING_PLAYER = 1
export const TRAINING_RANK = 2
/** `#LNMODE 1`: the chart's long notes are LN, whatever the player's LN MODE setting. */
export const TRAINING_LN_MODE = 1
export const TRAINING_TOTAL_MIN = 160
/** `#TOTAL = max(TRAINING_TOTAL_MIN, round(SCALE × N / (PER_NOTE × N + OFFSET)))` for N playable notes. */
export const TRAINING_TOTAL_COEFFICIENTS = { SCALE: 7.605, PER_NOTE: 0.01, OFFSET: 6.5 } as const
export const TRAINING_MAX_LINE_RESOLUTION = 192
/** Measure numbers are three decimal digits. */
export const TRAINING_MAX_MEASURE = 999
export const TRAINING_MEASURE_DIGITS = 3

export const TRAINING_HIT_WAV_NAME = 'hit.wav'
export const TRAINING_COUNT_WAV_NAME = 'count.wav'
export const TRAINING_HIT_TOKEN = '01'
export const TRAINING_COUNT_TOKEN = '02'
export const TRAINING_EMPTY_TOKEN = '00'
export const TRAINING_BGM_CHANNEL = '01'
export const TRAINING_BPM_CHANNEL = '08'
/** Lanes 0-6 (keys 1-7) and 7 (the scratch) to their 1P channels, and the same for long notes (02 §4.2). */
export const TRAINING_LANE_CHANNELS = ['11', '12', '13', '14', '15', '18', '19', '16'] as const
export const TRAINING_LN_CHANNELS = ['51', '52', '53', '54', '55', '58', '59', '56'] as const
/** `#BPMxx` and channel 08 tokens are numbered in base 36, two characters. */
export const TRAINING_TOKEN_RADIX = 36
export const TRAINING_TOKEN_LENGTH = 2

export const TRAINING_WAV_SAMPLE_RATE = 44_100
export const TRAINING_WAV_AMPLITUDE = 0.6
export const TRAINING_WAV_FADE_SAMPLES = 32
export const TRAINING_HIT_WAV = { frequencyHz: 2000, durationMs: 30, decayMs: 8 } as const
export const TRAINING_COUNT_WAV = { frequencyHz: 1000, durationMs: 60, decayMs: 20 } as const

export const TRAINING_WARMUP_KEY = 'warmup'
export const TRAINING_COOLDOWN_KEY = 'cooldown'
export const TRAINING_EDGE_KEYS = [TRAINING_WARMUP_KEY, TRAINING_COOLDOWN_KEY] as const

/** The kinds that train each pattern tag (plan 3.3 `TRAINING_KINDS_OF_KEY`). Mines are never laid, so `mine_near` trains on plain streams. */
export const TRAINING_KINDS_OF_TAG = {
    chord_2: ['chord_stream', 'chord_jack'],
    chord_3: ['chord_stream', 'chord_jack'],
    chord_4plus: ['chord_stream', 'chord_jack'],
    jack: ['jack', 'chord_jack'],
    trill: ['trill'],
    stair: ['stair'],
    stream: ['stream'],
    scratch_solo: ['scratch_solo'],
    scratch_chord: ['scratch_mixed'],
    scratch_mixed: ['scratch_mixed'],
    with_scratch: ['scratch_mixed'],
    scratch_run: ['scratch_run'],
    ln_head: ['ln_hold'],
    ln_end: ['ln_hold'],
    ln_hold_hit: ['ln_hold'],
    ln_release_then_hit: ['ln_release'],
    soflan: ['stream', 'chord_stream'],
    dense: ['burst', 'chord_stream'],
    mine_near: ['stream'],
} as const satisfies Record<PatternTag, readonly TrainingPatternKind[]>

/** Key prefixes whose argument is a pattern tag and that train on that tag's kinds. */
const TAG_KEY_PREFIXES = ['tag', 'solo', 'timing', 'bp_context', 'gauge_context'] as const

/** Key prefixes whose argument does not change the kinds that train them. */
const TRAINING_KINDS_OF_PREFIX = {
    lane: ['stream', 'trill', 'chord_stream'],
    hand: ['stream', 'trill', 'chord_stream'],
    bpm: ['stream', 'chord_stream'],
    soflan: ['stream', 'chord_stream'],
} as const satisfies Record<string, readonly TrainingPatternKind[]>

/** Keys that are a whole name of their own. */
const TRAINING_KINDS_OF_FIXED_KEY = {
    fatigue: ['chord_stream', 'burst'],
    'density:limit': ['stream', 'chord_stream'],
    'density:sustained': ['chord_stream', 'stream'],
    [TRAINING_WARMUP_KEY]: ['rest', 'stream'],
    [TRAINING_COOLDOWN_KEY]: ['rest', 'stream'],
} as const satisfies Record<string, readonly TrainingPatternKind[]>

const KEY_SEPARATOR = ':'
const COMBO_SEPARATOR = '+'
const COMBO_KEY_PREFIX = 'combo'
const COMBO_TAG_COUNT = 2

const toKindMap = (table: Record<string, readonly TrainingPatternKind[]>) => new Map(Object.entries(table))
const KINDS_BY_TAG = toKindMap(TRAINING_KINDS_OF_TAG)
const KINDS_BY_PREFIX = toKindMap(TRAINING_KINDS_OF_PREFIX)
const KINDS_BY_FIXED_KEY = toKindMap(TRAINING_KINDS_OF_FIXED_KEY)
const NO_KINDS: readonly TrainingPatternKind[] = []

const kindsOfTag = (tag: string) => KINDS_BY_TAG.get(tag) ?? NO_KINDS

/**
 * The kinds that may train a weakness key (plan 2.5.5 grammar), the first being the one a section of another kind is turned into.
 * A `combo:<a>+<b>` key takes the kinds of both tags, `a`'s first. A key the generator has no kind for gives none.
 */
export const trainingKindsOfKey = (key: string) => {
    const fixed = KINDS_BY_FIXED_KEY.get(key)
    if (fixed) return fixed
    const separator = key.indexOf(KEY_SEPARATOR)
    if (separator < 0) return NO_KINDS
    const prefix = key.slice(0, separator)
    const argument = key.slice(separator + 1)
    if ((TAG_KEY_PREFIXES as readonly string[]).includes(prefix)) return kindsOfTag(argument)
    if (prefix === COMBO_KEY_PREFIX) {
        const kinds = argument.split(COMBO_SEPARATOR).map(kindsOfTag)
        if (kinds.length !== COMBO_TAG_COUNT || kinds.some((list) => list.length === 0)) return NO_KINDS
        return [...new Set(kinds.flat())]
    }
    return KINDS_BY_PREFIX.get(prefix) ?? NO_KINDS
}

import { z } from 'zod'

export const CLEAR_LAMPS = [
    'NoPlay',
    'Failed',
    'AssistEasy',
    'LightAssistEasy',
    'Easy',
    'Normal',
    'Hard',
    'ExHard',
    'FullCombo',
    'Perfect',
    'Max',
] as const

export const GAUGE_TYPES = ['AssistEasy', 'Easy', 'Normal', 'Hard', 'ExHard', 'Hazard', 'Class', 'ExClass', 'ExHardClass'] as const

export const RANDOM_OPTIONS = ['Off', 'Mirror', 'Random', 'RRandom', 'SRandom', 'Spiral', 'HRandom', 'AllScratch', 'Converge'] as const

export type ClearLamp = (typeof CLEAR_LAMPS)[number]
export type GaugeType = (typeof GAUGE_TYPES)[number]
export type RandomOption = (typeof RANDOM_OPTIONS)[number]

export const clearLampSchema = z.enum(CLEAR_LAMPS)
export const gaugeTypeSchema = z.enum(GAUGE_TYPES)
export const randomOptionSchema = z.enum(RANDOM_OPTIONS)

export const clearLampToId = (lamp: ClearLamp) => CLEAR_LAMPS.indexOf(lamp)

export const clearLampFromId = (id: number) => CLEAR_LAMPS[id] ?? CLEAR_LAMPS[0]

/** The lowest lamp id that counts as a clear. Every lamp but NO PLAY and FAILED does, so AssistEasy and LightAssistEasy are clears. */
export const MIN_CLEAR_LAMP_ID = clearLampToId('AssistEasy')

export const gaugeTypeToId = (gauge: GaugeType) => GAUGE_TYPES.indexOf(gauge)

export const gaugeTypeFromId = (id: number) => GAUGE_TYPES[id] ?? GAUGE_TYPES[0]

export const MD5_LENGTH = 32
export const SHA256_LENGTH = 64

const HEX_PATTERN = /^[0-9a-f]*$/

export const md5FieldSchema = z.string().max(MD5_LENGTH).regex(HEX_PATTERN).default('')
export const sha256FieldSchema = z.string().max(SHA256_LENGTH).regex(HEX_PATTERN).default('')

/** The range the database's signed 32-bit INT columns hold. */
export const INT_COLUMN_MAX = 2_147_483_647
export const INT_COLUMN_MIN = -2_147_483_648

/** The largest magnitude the database's single-precision FLOAT columns hold. */
export const FLOAT_COLUMN_MAX = 3.402823466e38

export const isIntColumnValue = (value: number) => value >= INT_COLUMN_MIN && value <= INT_COLUMN_MAX

export const intColumnSchema = z.number().int().min(INT_COLUMN_MIN).max(INT_COLUMN_MAX)

export const countColumnSchema = z.number().int().min(0).max(INT_COLUMN_MAX)

export const floatColumnSchema = z.number().min(-FLOAT_COLUMN_MAX).max(FLOAT_COLUMN_MAX)

/**
 * Text kept to the characters a VARCHAR column of `length` holds. The value is informational, so a longer one is cut
 * instead of rejecting the play it came with.
 */
export const columnTextSchema = (length: number) => z.string().transform((value) => [...value].slice(0, length).join(''))

/** A client build's SHA-256 as lowercase hex. A longer or non-hex value can not be a build the server knows. */
export const buildHashSchema = z.string().max(SHA256_LENGTH).regex(HEX_PATTERN).nullish().default(null)

export const CLIENT_PLATFORM_LENGTH = 32
const INPUT_DEVICE_LENGTH = 32

export const isMd5Hash = (value: string) => value.length === MD5_LENGTH && HEX_PATTERN.test(value)

export const isSha256Hash = (value: string) => value.length === SHA256_LENGTH && HEX_PATTERN.test(value)

export const chartIdSchema = z.object({
    md5: md5FieldSchema,
    sha256: sha256FieldSchema,
})

export const playerIdSchema = z.object({
    id: z.string(),
})

/** The deepest JSON document a database JSON column takes, counting a scalar or an empty array or object as one level. */
export const JSON_COLUMN_DEPTH_MAX = 100

/** The longest object key a database JSON column takes, in UTF-8 bytes (ER_JSON_KEY_TOO_BIG past it). */
export const JSON_KEY_MAX_BYTES = 65_535

const utf8 = new TextEncoder()

const fitsJsonKey = (key: string) => key.isWellFormed() && utf8.encode(key).length <= JSON_KEY_MAX_BYTES

/**
 * Whether `value` is a document a database JSON column takes: it nests no deeper than `levels`, by the database's count,
 * every key and string in it is well-formed UTF-16, and no key is longer than JSON_KEY_MAX_BYTES in UTF-8. JSON text can
 * spell an unpaired surrogate (`"\ud800"`) and JavaScript keeps it, but the database refuses the document. It stops
 * descending once the depth is spent.
 */
export const fitsJsonColumn = (value: unknown, levels: number): boolean => {
    if (levels < 1) return false
    if (typeof value === 'string') return value.isWellFormed()
    if (typeof value !== 'object' || value === null) return true
    return Object.entries(value).every(([key, child]) => fitsJsonKey(key) && fitsJsonColumn(child, levels - 1))
}

/** A string that is stored inside a database JSON column, which refuses one with an unpaired UTF-16 surrogate. */
export const jsonTextSchema = z.string().refine((value) => value.isWellFormed())

export const extraSchema = z
    .record(z.string(), z.unknown())
    .refine((value) => fitsJsonColumn(value, JSON_COLUMN_DEPTH_MAX))
    .default({})

/**
 * A chart layout seed as a client sends it. The seed is informational, so a value outside the range
 * JavaScript and the database's signed BIGINT hold exactly (an older client sent the clock's
 * nanoseconds, above 2^53) is stored as 0 instead of rejecting the whole submission.
 */
export const seedSchema = z.number().transform((value) => (Number.isSafeInteger(value) ? value : 0))

export const includeScratchAutoSchema = z.stringbool({ truthy: ['true'], falsy: ['false'] }).default(false)

export const judgeBreakdownSchema = z.object({
    pgreat: countColumnSchema,
    great: countColumnSchema,
    good: countColumnSchema,
    bad: countColumnSchema,
    poor: countColumnSchema,
    miss: countColumnSchema,
    fast: countColumnSchema,
    slow: countColumnSchema,
    combobreak: countColumnSchema,
    epg: countColumnSchema.default(0),
    lpg: countColumnSchema.default(0),
    egr: countColumnSchema.default(0),
    lgr: countColumnSchema.default(0),
    egd: countColumnSchema.default(0),
    lgd: countColumnSchema.default(0),
    ebd: countColumnSchema.default(0),
    lbd: countColumnSchema.default(0),
    epr: countColumnSchema.default(0),
    lpr: countColumnSchema.default(0),
    ems: countColumnSchema.default(0),
    lms: countColumnSchema.default(0),
    avgjudge: z.number().int().default(0),
    empty_poor: countColumnSchema.default(0),
})

export const playOptionsSchema = z.object({
    gauge: gaugeTypeSchema,
    random: randomOptionSchema,
    random_p2: randomOptionSchema.nullish().default(null),
    scratch_auto: z.boolean(),
    lntype: intColumnSchema,
    input_device: columnTextSchema(INPUT_DEVICE_LENGTH),
    assist: z.array(z.string()),
    option: z.number().int().default(0),
    judge_rate: intColumnSchema.default(0),
    offset_ms: intColumnSchema.default(0),
    constant: z.boolean().default(false),
    hispeed: z.number().default(0),
    lift: floatColumnSchema.default(0),
    lane_cover: floatColumnSchema.default(0),
    total_override: z.number().default(0),
    autoplay: z.boolean().default(false),
    auto_offset: z.boolean().default(false),
    scratch_left: z.boolean().default(false),
    green_number: z.number().default(0),
})

export type ChartIdInput = z.infer<typeof chartIdSchema>
export type JudgeBreakdownInput = z.infer<typeof judgeBreakdownSchema>
export type PlayOptionsInput = z.infer<typeof playOptionsSchema>

export const settingsBlobSchema = z.object({
    name: z.string().min(1),
    content: z.string(),
    updated_at: z.number().int().default(0),
})

export type SettingsBlobInput = z.infer<typeof settingsBlobSchema>

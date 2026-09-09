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

export const gaugeTypeToId = (gauge: GaugeType) => GAUGE_TYPES.indexOf(gauge)

export const gaugeTypeFromId = (id: number) => GAUGE_TYPES[id] ?? GAUGE_TYPES[0]

export const MD5_LENGTH = 32
export const SHA256_LENGTH = 64

const HEX_PATTERN = /^[0-9a-f]*$/

export const md5FieldSchema = z.string().max(MD5_LENGTH).regex(HEX_PATTERN).default('')
export const sha256FieldSchema = z.string().max(SHA256_LENGTH).regex(HEX_PATTERN).default('')

export const isMd5Hash = (value: string) => value.length === MD5_LENGTH && HEX_PATTERN.test(value)

export const isSha256Hash = (value: string) => value.length === SHA256_LENGTH && HEX_PATTERN.test(value)

export const chartIdSchema = z.object({
    md5: md5FieldSchema,
    sha256: sha256FieldSchema,
})

export const playerIdSchema = z.object({
    id: z.string(),
})

export const extraSchema = z.record(z.string(), z.unknown()).default({})

export const judgeBreakdownSchema = z.object({
    pgreat: z.number().int().min(0),
    great: z.number().int().min(0),
    good: z.number().int().min(0),
    bad: z.number().int().min(0),
    poor: z.number().int().min(0),
    miss: z.number().int().min(0),
    fast: z.number().int().min(0),
    slow: z.number().int().min(0),
    combobreak: z.number().int().min(0),
    epg: z.number().int().min(0).default(0),
    lpg: z.number().int().min(0).default(0),
    egr: z.number().int().min(0).default(0),
    lgr: z.number().int().min(0).default(0),
    egd: z.number().int().min(0).default(0),
    lgd: z.number().int().min(0).default(0),
    ebd: z.number().int().min(0).default(0),
    lbd: z.number().int().min(0).default(0),
    epr: z.number().int().min(0).default(0),
    lpr: z.number().int().min(0).default(0),
    ems: z.number().int().min(0).default(0),
    lms: z.number().int().min(0).default(0),
    avgjudge: z.number().int().default(0),
    empty_poor: z.number().int().min(0).default(0),
})

export const playOptionsSchema = z.object({
    gauge: gaugeTypeSchema,
    random: randomOptionSchema,
    random_p2: randomOptionSchema.nullish().default(null),
    scratch_auto: z.boolean(),
    lntype: z.number().int(),
    input_device: z.string(),
    assist: z.array(z.string()),
    option: z.number().int().default(0),
    judge_rate: z.number().int().default(0),
    offset_ms: z.number().int().default(0),
    constant: z.boolean().default(false),
    hispeed: z.number().default(0),
    lift: z.number().default(0),
    lane_cover: z.number().default(0),
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

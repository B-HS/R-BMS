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

export type ClearLamp = (typeof CLEAR_LAMPS)[number]

/** The lowest lamp that counts as a clear. Mirrors the server's MIN_CLEAR_LAMP_ID: every lamp but NO PLAY and FAILED is a clear. */
export const MIN_CLEAR_LAMP = 'AssistEasy' satisfies ClearLamp

export const CLEAR_LAMP_LABEL: Record<ClearLamp, string> = {
    NoPlay: 'NO PLAY',
    Failed: 'FAILED',
    AssistEasy: 'ASSIST',
    LightAssistEasy: 'LIGHT ASSIST',
    Easy: 'EASY',
    Normal: 'CLEAR',
    Hard: 'HARD',
    ExHard: 'EX HARD',
    FullCombo: 'FULL COMBO',
    Perfect: 'PERFECT',
    Max: 'MAX',
}

export const LAMP_BACKGROUND: Record<ClearLamp, string> = {
    NoPlay: 'var(--rbms-lamp-noplay)',
    Failed: 'var(--rbms-lamp-failed)',
    AssistEasy: 'var(--rbms-lamp-assist)',
    LightAssistEasy: 'var(--rbms-lamp-light-assist)',
    Easy: 'var(--rbms-lamp-easy)',
    Normal: 'var(--rbms-lamp-normal)',
    Hard: 'var(--rbms-lamp-hard)',
    ExHard: 'var(--rbms-lamp-exhard)',
    FullCombo: 'var(--rbms-lamp-fullcombo)',
    Perfect: 'var(--rbms-lamp-perfect)',
    Max: 'var(--rbms-lamp-max)',
}

/**
 * The grade of a SCRATCH AUTO clear, which only the lamp LightAssistEasy has: 0 is a plain LIGHT ASSIST, 1 to 4 are the grade of the
 * gauge it cleared on. Mirrors the server's ASSIST_GRADES.
 */
export const ASSIST_GRADES = [0, 1, 2, 3, 4] as const

export type AssistGrade = (typeof ASSIST_GRADES)[number]

export const NO_ASSIST_GRADE = ASSIST_GRADES[0]

export const GRADED_LAMP = 'LightAssistEasy' satisfies ClearLamp

export const ASSIST_GRADE_LABEL: Record<AssistGrade, string> = {
    0: 'LIGHT ASSIST',
    1: 'ASSIST EASY',
    2: 'ASSIST CLEAR',
    3: 'ASSIST HARD',
    4: 'ASSIST EXHARD',
}

export const ASSIST_GRADE_BACKGROUND: Record<AssistGrade, string> = {
    0: 'var(--rbms-lamp-light-assist)',
    1: 'var(--rbms-lamp-assist-easy)',
    2: 'var(--rbms-lamp-assist-clear)',
    3: 'var(--rbms-lamp-assist-hard)',
    4: 'var(--rbms-lamp-assist-exhard)',
}

export type LampWithGrade = { lamp: ClearLamp; assistGrade: AssistGrade }

/** A lamp as it is shown: the assist grade names one of five LightAssistEasy lamps and means nothing to any other lamp. */
export const lampLabel = (lamp: ClearLamp, assistGrade: AssistGrade = NO_ASSIST_GRADE) =>
    lamp === GRADED_LAMP ? ASSIST_GRADE_LABEL[assistGrade] : CLEAR_LAMP_LABEL[lamp]

export const lampBackground = (lamp: ClearLamp, assistGrade: AssistGrade = NO_ASSIST_GRADE) =>
    lamp === GRADED_LAMP ? ASSIST_GRADE_BACKGROUND[assistGrade] : LAMP_BACKGROUND[lamp]

/** A key for a displayed lamp in a list: one per lamp, and one per assist grade of LightAssistEasy. */
export const lampKey = (lamp: ClearLamp, assistGrade: AssistGrade = NO_ASSIST_GRADE) => `${lamp}:${assistGrade}`

/** Every lamp as it is shown, lowest first: LightAssistEasy once per assist grade. */
export const DISPLAY_LAMPS = CLEAR_LAMPS.flatMap((lamp): LampWithGrade[] =>
    lamp === GRADED_LAMP ? ASSIST_GRADES.map((assistGrade) => ({ lamp, assistGrade })) : [{ lamp, assistGrade: NO_ASSIST_GRADE }],
)

export const isSameDisplayLamp = (left: LampWithGrade, right: LampWithGrade) =>
    left.lamp === right.lamp && (left.lamp !== GRADED_LAMP || left.assistGrade === right.assistGrade)

/** The place of a displayed lamp in DISPLAY_LAMPS, so a higher assist grade ranks above a lower one of the same lamp. */
export const displayLampRank = (lamp: ClearLamp, assistGrade: AssistGrade = NO_ASSIST_GRADE) =>
    DISPLAY_LAMPS.findIndex((candidate) => isSameDisplayLamp(candidate, { lamp, assistGrade }))

export const SUBMISSION_FLAGS = [
    'AUTOPLAY',
    'SCRATCH_AUTO',
    'ASSIST',
    'JUDGE_WIDTH',
    'TOTAL_OVERRIDE',
    'UNKNOWN_BUILD',
    'GUEST',
    'REPLAY_MISMATCH',
] as const

export type SubmissionFlag = (typeof SUBMISSION_FLAGS)[number]

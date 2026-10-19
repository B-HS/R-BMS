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
    LightAssistEasy: 'L-ASSIST',
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

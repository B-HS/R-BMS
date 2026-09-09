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

export const CLEAR_LAMP_LABEL: Record<ClearLamp, string> = {
    NoPlay: 'NO PLAY',
    Failed: 'FAILED',
    AssistEasy: 'ASSIST EASY',
    LightAssistEasy: 'L-ASSIST EASY',
    Easy: 'EASY',
    Normal: 'NORMAL',
    Hard: 'HARD',
    ExHard: 'EX HARD',
    FullCombo: 'FULL COMBO',
    Perfect: 'PERFECT',
    Max: 'MAX',
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

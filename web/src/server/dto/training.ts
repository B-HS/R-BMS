import { z } from 'zod'
import type { Locale } from '@shared/i18n/locale'

/** The note patterns the training chart synthesiser can lay out (plan 2026-11-04 3.3, 3.6). */
export const TRAINING_PATTERN_KINDS = [
    'stream',
    'chord_stream',
    'trill',
    'stair',
    'jack',
    'chord_jack',
    'scratch_solo',
    'scratch_mixed',
    'scratch_run',
    'ln_hold',
    'ln_release',
    'burst',
    'rest',
] as const
export type TrainingPatternKind = (typeof TRAINING_PATTERN_KINDS)[number]

/** Which keys a section leans on (plan 3.6 lane candidates). */
export const TRAINING_LANE_HINTS = ['all', 'left', 'right', 'center', 'wide', 'adjacent'] as const
export type TrainingLaneHint = (typeof TRAINING_LANE_HINTS)[number]

/** How hard a training chart is against the player's density profile (plan 3.5 NPS range). */
export const TRAINING_INTENSITIES = ['build', 'push', 'over'] as const
export type TrainingIntensity = (typeof TRAINING_INTENSITIES)[number]

export const TRAINING_LIMIT = {
    CHARTS_PER_USER: 20,
    DESCRIPTION_MAX: 255,
    INTENT_MAX: 100,
    ADVICE_MAX: 300,
    SECTIONS_MIN: 3,
    SECTIONS_MAX: 12,
    NOTE_MAX: 100,
    WRITE_MAX_BYTES: 1024,
} as const

/** The intensity as the public `#TITLE` spells it. The title names no weakness, since plays of a training chart are public (plan 3.7). */
export const TRAINING_INTENSITY_TITLE = { build: 'BUILD', push: 'PUSH', over: 'OVER' } as const satisfies Record<TrainingIntensity, string>

/** The name the user's training folder is made with, in the language of the screen it is made from, and never renamed after (plan 5.3). */
export const TRAINING_FOLDER_NAME = { ko: '훈련곡', en: 'Training', ja: 'トレーニング' } as const satisfies Record<Locale, string>

export const TRAINING_FOLDER_DESCRIPTION = {
    ko: 'AI 분석으로 만든 훈련곡입니다. R-BMS 선곡 화면의 MY FOLDERS 에서 바로 받아 플레이할 수 있습니다.',
    en: 'Training charts made from your AI analysis. Download and play them from MY FOLDERS on the R-BMS song select.',
    ja: 'AI分析から作ったトレーニング譜面です。R-BMSの選曲画面のMY FOLDERSからそのままダウンロードしてプレイできます。',
} as const satisfies Record<Locale, string>

/**
 * The plan a provider writes for a training chart (plan 3.3). Every property is required and none is nullable, so the provider's
 * structured output accepts it; ranges are not part of the provider schema and are enforced by `correctTrainingPlan` afterwards.
 */
export const aiTrainingPlanOutputSchema = z.object({
    description: z.string().trim().max(TRAINING_LIMIT.DESCRIPTION_MAX),
    base_bpm: z.number(),
    sections: z
        .array(
            z.object({
                focus_key: z.string(),
                kind: z.enum(TRAINING_PATTERN_KINDS),
                weight: z.number(),
                target_nps: z.number(),
                lane_hint: z.enum(TRAINING_LANE_HINTS),
                chord_size: z.number(),
                scratch_ratio: z.number(),
                ln_ratio: z.number(),
                bpm_ratio: z.number(),
                intent: z.string().trim().max(TRAINING_LIMIT.INTENT_MAX),
            }),
        )
        .min(1)
        .max(TRAINING_LIMIT.SECTIONS_MAX),
    advice: z.string().trim().max(TRAINING_LIMIT.ADVICE_MAX),
})

export type AiTrainingPlanOutput = z.infer<typeof aiTrainingPlanOutputSchema>
export type AiTrainingPlanSection = AiTrainingPlanOutput['sections'][number]

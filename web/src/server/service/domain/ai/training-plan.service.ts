import type { AiEffort } from '@server/dto/ai'
import type { WeaknessItem, WeaknessProfileView } from '@server/dto/ai-play'
import {
    TRAINING_AUTO_FOCUS,
    TRAINING_LANE_HINTS,
    TRAINING_LIMIT,
    aiTrainingPlanOutputSchema,
    type AiTrainingPlanOutput,
    type TrainingChartCreateInput,
} from '@server/dto/training'
import {
    AI_OUTPUT_LANGUAGE_NAME,
    DIGEST_IS_DATA_RULE,
    DIGEST_KEY_RULE,
    cleanLine,
    runAiFeature,
    type AiFeatureSpec,
} from '@server/service/domain/ai/ai-digest'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import { PLAY_FEATURE_ANALYSIS_VERSION } from '@server/service/domain/ai/play-feature.constants'
import { toProfileDigest } from '@server/service/domain/ai/weakness-profile.service'
import { parseWeaknessKey } from '@server/service/domain/analysis/weakness-key'
import { trainingBoundsOf, type TrainingAllowed } from '@server/service/domain/training/spec'
import { TRAINING_EDGE_KEYS, trainingKindsOfKey } from '@server/service/domain/training/training-constants'
import type { Locale } from '@shared/i18n/locale'

export const AI_TRAINING_PLAN_FEATURE = 'training_plan'
export const TRAINING_PLAN_PROMPT_VERSION = 1
export const TRAINING_PLAN_MAX_OUTPUT_TOKENS = 8000
export const TRAINING_PLAN_EFFORT = 'low' satisfies AiEffort

/** How many weaknesses, strengths and earlier charts a plan digest carries (plan 2026-11-04 3.2). */
export const TRAINING_DIGEST_LIMIT = { WEAKNESSES: 8, STRENGTHS: 3, PREVIOUS: 3 } as const

const SUBJECT_PREFIX = 'train'

/** One earlier training chart as the digest shows it: when, what it trained and how the plays on it moved. */
export type TrainingHistoryItem = { created_at: number; focus_keys: string[]; kinds: string[]; result_delta: number | null }

/** Whether the generator can train a weakness and its sample lets the AI point at it. */
const isTrainable = (item: WeaknessItem) => item.training_kinds.length > 0 && item.sample_level !== 'insufficient'

/** The profile's weaknesses a training chart can be planned for with `auto` (plan 3.1), strongest first. */
export const trainableWeaknessesOf = (profile: WeaknessProfileView) => profile.weaknesses.filter(isTrainable)

/** The weakness a request named, when the profile holds it and the generator has a kind for it (plan 3.1). */
export const focusWeaknessOf = (profile: WeaknessProfileView, key: string) =>
    profile.weaknesses.find((item) => item.key === key && item.training_kinds.length > 0) ?? null

/** The weaknesses a plan for one focus key is given: the key's own item, then the trainable pairs that share one of its tags. */
const focusedWeaknessesOf = (profile: WeaknessProfileView, focus: WeaknessItem) => {
    const tags = new Set(parseWeaknessKey(focus.key)?.tags ?? [])
    const related = trainableWeaknessesOf(profile).filter(
        (item) => item.key !== focus.key && item.kind === 'combo' && (parseWeaknessKey(item.key)?.tags ?? []).some((tag) => tags.has(tag)),
    )
    return [focus, ...related]
}

/** The cache subject of a plan: the request's focus and intensity (at most 76 characters, plan 3.2). */
export const trainingPlanSubjectKey = (request: TrainingChartCreateInput) => [SUBJECT_PREFIX, request.focus_key, request.intensity].join(':')

/**
 * The input a provider plans a training chart from (plan 3.2): the profile's window, density, weaknesses, strengths, timing,
 * contexts and hands, the frontier of the default table, the newest earlier charts, the ranges the plan is held to and what it may
 * name. With a focus key the weaknesses are cut to that key and its pairs and the allowed keys to it, the warm-up and the cool-down.
 */
export const trainingPlanDigestOf = (
    profile: WeaknessProfileView,
    history: readonly TrainingHistoryItem[],
    request: TrainingChartCreateInput,
    context: { frontierLevel: string | null },
) => {
    const digest = toProfileDigest(profile)
    const focus = request.focus_key === TRAINING_AUTO_FOCUS ? null : focusWeaknessOf(profile, request.focus_key)
    const weaknesses = (focus ? focusedWeaknessesOf(profile, focus) : trainableWeaknessesOf(profile)).slice(0, TRAINING_DIGEST_LIMIT.WEAKNESSES)
    const keys = [...(focus ? [focus.key] : weaknesses.map((item) => item.key)), ...TRAINING_EDGE_KEYS]
    const allowed = {
        kinds: [...new Set(keys.flatMap((key) => trainingKindsOfKey(key)))],
        lane_hints: TRAINING_LANE_HINTS,
        keys,
    } satisfies TrainingAllowed
    return {
        request: { focus_key: request.focus_key, intensity: request.intensity },
        window: { plays: digest.window.plays, charts: digest.window.charts, days: digest.window.days },
        density: {
            comfort_nps: digest.density.comfort_nps,
            limit_nps_ex: digest.density.limit_nps_ex,
            limit_nps_bp: digest.density.limit_nps_bp,
            max_seen_nps: digest.density.max_seen_nps,
            peak_seen_2s: digest.density.peak_seen_2s,
            sustained_minus_burst: digest.sustained_minus_burst.value,
        },
        weaknesses: weaknesses.map((item) => ({
            key: item.key,
            kind: item.kind,
            value: item.value,
            evidence: item.evidence,
            sample_level: item.sample_level,
            trend_delta: item.trend_delta,
            training_kinds: item.training_kinds,
        })),
        strengths: digest.strengths.slice(0, TRAINING_DIGEST_LIMIT.STRENGTHS).map((item) => ({ key: item.key, value: item.value })),
        timing: digest.timing.map((item) => ({ key: item.key, mean_ms: item.mean_ms, fast_share: item.fast_share })),
        context: digest.context.map((item) => ({ key: item.key, lift: item.lift })),
        hands: digest.hands.map((item) => ({ key: item.key, value: item.ex_residual })),
        frontier_level: context.frontierLevel,
        previous: history.slice(0, TRAINING_DIGEST_LIMIT.PREVIOUS),
        bounds: trainingBoundsOf(profile.density, request.intensity),
        allowed,
    }
}

export type TrainingPlanDigest = ReturnType<typeof trainingPlanDigestOf>

export const buildTrainingPlanSystemPrompt = (language: Locale) =>
    [
        'You plan one training chart of about one minute for a rhythm game (BMS, 7 keys and a scratch) player from their weakness profile.',
        DIGEST_IS_DATA_RULE,
        DIGEST_KEY_RULE,
        'Every section `focus_key` must be one of `allowed.keys`.',
        "A section's `kind` must be one of the `training_kinds` of the weakness its `focus_key` names; a `warmup` or `cooldown` section is `rest` or `stream`.",
        'Give each section a `weight`: only the ratio of the weights matters, and the server fits the sections to about 60 seconds.',
        'Keep `target_nps` within `bounds.nps_min` and `bounds.nps_max`, and `base_bpm` within `bounds.bpm_min` and `bounds.bpm_max`.',
        'Start with a warm-up section, put sections of the focus weaknesses in the middle at least twice, and end with a cool-down section.',
        `\`intent\` is one ${AI_OUTPUT_LANGUAGE_NAME[language]} line the player reads for the section; \`description\` and \`advice\` are short ${AI_OUTPUT_LANGUAGE_NAME[language]} sentences on the chart and on how to play it.`,
    ].join('\n')

export const aiTrainingPlanSpecOf = (language: Locale) =>
    ({
        feature: AI_TRAINING_PLAN_FEATURE,
        analysisVersion: PLAY_FEATURE_ANALYSIS_VERSION,
        promptVersion: TRAINING_PLAN_PROMPT_VERSION,
        system: buildTrainingPlanSystemPrompt(language),
        schemaName: 'training_plan',
        schema: aiTrainingPlanOutputSchema,
        maxOutputTokens: TRAINING_PLAN_MAX_OUTPUT_TOKENS,
        effort: TRAINING_PLAN_EFFORT,
    }) satisfies AiFeatureSpec<typeof aiTrainingPlanOutputSchema>

/**
 * The checks of a provider's plan that are stored with it (plan 3.4): fewer than SECTIONS_MIN sections on an allowed key rejects it,
 * and the free sentences are put on one line within their lengths. Sections are never dropped or changed here: that is
 * `correctTrainingPlan`'s, which runs again on every request so a cached plan gives the same corrections.
 */
export const refineTrainingPlanOutput = (output: AiTrainingPlanOutput, digest: Pick<TrainingPlanDigest, 'allowed'>) => {
    const allowedKeys = new Set(digest.allowed.keys)
    if (output.sections.filter((section) => allowedKeys.has(section.focus_key)).length < TRAINING_LIMIT.SECTIONS_MIN) return null
    return {
        ...output,
        description: cleanLine(output.description, TRAINING_LIMIT.DESCRIPTION_MAX),
        advice: cleanLine(output.advice, TRAINING_LIMIT.ADVICE_MAX),
        sections: output.sections.map((section) => ({ ...section, intent: cleanLine(section.intent, TRAINING_LIMIT.INTENT_MAX) })),
    }
}

/** The training plan feature: one provider call per request through the shared runner (keys, limits, cache and usage apply there). */
export const createAiTrainingPlanService = (deps: { runner: Pick<AiFeatureRunner, 'run'> }) => ({
    plan: (params: { userId: string; digest: TrainingPlanDigest; language: Locale }) =>
        runAiFeature(deps.runner, {
            ...aiTrainingPlanSpecOf(params.language),
            userId: params.userId,
            subjectKey: trainingPlanSubjectKey(params.digest.request),
            digest: params.digest,
            refine: (output) => refineTrainingPlanOutput(output, params.digest),
        }),
})

export type AiTrainingPlanService = ReturnType<typeof createAiTrainingPlanService>

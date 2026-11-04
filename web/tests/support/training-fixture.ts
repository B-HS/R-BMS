import { z } from 'zod'
import { weaknessProfileViewSchema, type WeaknessItem, type WeaknessProfileView } from '@server/dto/ai-play'
import { TRAINING_LIMIT, type AiTrainingPlanOutput } from '@server/dto/training'
import type { ChartPatternDocument } from '@server/dto/chart-pattern'
import { createAiTrainingPlanService, type TrainingPlanDigest } from '@server/service/domain/ai/training-plan.service'
import type { WeaknessProfileService } from '@server/service/domain/ai/weakness-profile.service'
import { createFolderService } from '@server/service/domain/folder/folder.service'
import { TRAINING_COOLDOWN_KEY, TRAINING_WARMUP_KEY, trainingKindsOfKey } from '@server/service/domain/training/training-constants'
import {
    createTrainingChartService,
    type TrainingChartRow,
    type TrainingChartServiceDb,
    type TrainingChartServiceDeps,
    type TrainingPlayRow,
} from '@server/service/domain/training/training.service'
import { FAKE_USAGE, createFakeAdapter, createMemoryAiServices, testKekEnv } from './ai-memory-db'
import { createMemoryFolderDb } from './folder-memory-db'

export const TRAINING_USER = 'user-alice'
export const TRAINING_OTHER_USER = 'user-bob'
export const TRAINING_API_KEY = 'sk-ant-api03-training-secret-WXYZ'
export const TRAINING_NOW = Date.UTC(2026, 10, 4, 12)
export const TRAINING_ORIGIN = 'https://example.test'

const planDigestKeysSchema = z.object({ allowed: z.object({ keys: z.array(z.string()) }) })

/** The part of a plan digest a fake provider reads: the keys a section may name. */
export type PlanDigestKeys = Pick<TrainingPlanDigest, 'allowed'> | z.infer<typeof planDigestKeysSchema>

type StoredChart = TrainingChartRow & { userId: string; bms: Uint8Array }

/** A stored score of a training chart, as the memory database keeps it. */
export type MemoryTrainingScore = TrainingPlayRow & { userId: string; sha256: string }

/**
 * An in-memory `training_chart`, `training_folder` and the score, pattern, trace and replay counts a removal reads. A folder link
 * whose folder was deleted reads as absent, as the foreign key's cascade leaves it. `insertResult` forces the outcome of the next
 * save (`limit` stands for a save that lost the count to another one).
 */
export const createMemoryTrainingDb = (aliveFolder: (folderId: string) => boolean) => {
    const charts = new Map<string, StoredChart>()
    const rows = new Map<string, TrainingChartRow>()
    const links = new Map<string, string>()
    const scores: MemoryTrainingScore[] = []
    const kept = { patterns: new Set<string>(), traces: new Map<string, number>(), replays: new Map<string, number>() }
    const state: { insertResult: 'limit' | null } = { insertResult: null }
    const owned = (userId: string) =>
        [...charts.values()].filter((chart) => chart.userId === userId).toSorted((a, b) => b.createdAt - a.createdAt || b.id.localeCompare(a.id))
    const strip = (chart: StoredChart) => rows.get(chart.id) ?? chart
    const linkOf = (userId: string) => {
        const folderId = links.get(userId)
        return folderId && aliveFolder(folderId) ? folderId : null
    }
    const scoresOf = (userId: string, sha256: string) =>
        scores.filter((play) => play.userId === userId && play.sha256 === sha256).toSorted((a, b) => b.playedAt - a.playedAt)
    const keyOf = (userId: string, sha256: string) => `${userId}|${sha256}`

    const db: TrainingChartServiceDb = {
        countCharts: async (userId) => owned(userId).length,
        listCharts: async (userId) => owned(userId).map(strip),
        findChart: async ({ id, userId }) => {
            const chart = charts.get(id)
            return chart && chart.userId === userId ? strip(chart) : null
        },
        insertChart: async (row) => {
            if (state.insertResult === 'limit' || owned(row.userId).length >= TRAINING_LIMIT.CHARTS_PER_USER) {
                state.insertResult = null
                return 'limit'
            }
            const { userId, bms, ...stored } = row
            charts.set(row.id, { ...stored, userId, bms })
            rows.set(row.id, stored)
            return 'inserted'
        },
        deleteChart: async ({ id, userId }) => {
            const chart = charts.get(id)
            if (!chart || chart.userId !== userId) return false
            charts.delete(id)
            rows.delete(id)
            return true
        },
        findDownload: async (id) => {
            const chart = charts.get(id)
            return chart ? { id: chart.id, bms: chart.bms } : null
        },
        findFolderLink: async (userId) => linkOf(userId),
        claimFolderLink: async ({ userId, folderId }) => {
            const existing = linkOf(userId)
            if (existing) return existing
            links.set(userId, folderId)
            return folderId
        },
        listPlayStats: async ({ userId, sha256s }) =>
            sha256s.flatMap((sha256) => {
                const plays = scoresOf(userId, sha256)
                if (plays.length === 0) return []
                return [
                    {
                        sha256,
                        plays: plays.length,
                        bestExRatio: Math.max(...plays.map((play) => play.exScore / play.maxExScore)),
                        lastPlayedAt: plays[0]?.playedAt ?? null,
                    },
                ]
            }),
        listPlays: async ({ userId, sha256, limit }) =>
            scoresOf(userId, sha256)
                .slice(0, limit)
                .map((play) => ({
                    scoreId: play.scoreId,
                    playedAt: play.playedAt,
                    clear: play.clear,
                    exScore: play.exScore,
                    maxExScore: play.maxExScore,
                    minbp: play.minbp,
                    random: play.random,
                })),
        countKept: async ({ userId, sha256 }) => ({
            scores: scoresOf(userId, sha256).length,
            traces: kept.traces.get(keyOf(userId, sha256)) ?? 0,
            replays: kept.replays.get(keyOf(userId, sha256)) ?? 0,
        }),
    }
    return { db, charts, links, scores, kept, state, keyOf }
}

export const trainingWeaknessOf = (key: string, kind: string, patch: Partial<WeaknessItem> = {}): WeaknessItem => ({
    key,
    kind,
    label_tags: [],
    value: -0.05,
    unit: 'ex_residual',
    evidence: 'strong',
    sample_level: 'medium',
    notes: 900,
    charts: 12,
    plays: 30,
    severity: 1,
    trend_delta: null,
    selectable_for_folder: true,
    training_kinds: [...trainingKindsOfKey(key)],
    ...patch,
})

/** A weakness profile of `plays` plays with the given weaknesses (trill and jack by default) and a density limit of 12 NPS. */
export const trainingProfileOf = (options: { plays?: number; weaknesses?: WeaknessItem[] } = {}): WeaknessProfileView =>
    weaknessProfileViewSchema.parse({
        profile_version: 1,
        window: {
            from: TRAINING_NOW - 90 * 86_400_000,
            to: TRAINING_NOW,
            days: 90,
            plays: options.plays ?? 40,
            charts: 20,
            notes: 30_000,
            stale: 0,
            auto_plays: 0,
            training_plays: 0,
        },
        density: {
            curve: [],
            baseline_ex: 0.8,
            comfort_nps: 8,
            limit_nps_ex: 12,
            limit_nps_bp: 13,
            max_seen_nps: 16,
            peak_seen_2s: 18,
            ceiling_reached_ex: false,
            evidence: 'strong',
            sustained_minus_burst: { value: null, evidence: 'insufficient', sample_level: 'insufficient' },
        },
        cells: [],
        weaknesses: options.weaknesses ?? [trainingWeaknessOf('tag:trill', 'tag'), trainingWeaknessOf('tag:jack', 'tag', { value: -0.03 })],
        strengths: [],
        bp_runs: { longest_mean: null, histogram: {}, recovery_median: null },
        gauge: { drops_per_play: null, drop_mean_nps: null, overall_mean_nps: null },
        soflan: { events: 0, adjusted_share: null },
        notice: null,
    })

/** A plan that keeps to the digest: a warm-up, each allowed focus key twice with its first kind, then a cool-down. */
export const planFromDigest = (digest: PlanDigestKeys): AiTrainingPlanOutput => {
    const focus = digest.allowed.keys.filter((key) => key !== TRAINING_WARMUP_KEY && key !== TRAINING_COOLDOWN_KEY)
    const section = (focusKey: string, kind: AiTrainingPlanOutput['sections'][number]['kind'], weight: number) => ({
        focus_key: focusKey,
        kind,
        weight,
        target_nps: 9,
        lane_hint: 'all' as const,
        chord_size: 2,
        scratch_ratio: 0,
        ln_ratio: 0,
        bpm_ratio: 1,
        intent: `${focusKey} 연습`,
    })
    const middle = [...focus, ...focus].map((key) => section(key, trainingKindsOfKey(key)[0] ?? 'stream', 3))
    return {
        description: '트릴과 잭을 번갈아 다지는 1분 곡',
        base_bpm: 150,
        sections: [section(TRAINING_WARMUP_KEY, 'stream', 1), ...middle, section(TRAINING_COOLDOWN_KEY, 'rest', 1)],
        advice: '처음 두 구간은 힘을 빼고 치세요.',
    }
}

type HarnessOptions = {
    profile?: WeaknessProfileView
    plan?: (digest: PlanDigestKeys) => unknown
    measured?: Record<
        string,
        { value: number; unit: 'ex_residual' | 'ms' | 'ratio' | 'nps'; notes: number; charts: number; plays: number; evidence: string } | null
    >
    analysed?: TrainingChartServiceDeps['traces']['analyzedPlaysOfScores']
}

/**
 * The training chart service on memory databases, the real AI runner and a fake Anthropic adapter (`anthropic.generateCalls` counts
 * provider calls). `registerKey` registers the user's key; ids count up from `tc_` + 32 zeros.
 */
export const createTrainingHarness = (options: HarnessOptions = {}) => {
    const state = { profile: options.profile ?? trainingProfileOf(), now: TRAINING_NOW }
    const now = () => state.now
    const anthropic = createFakeAdapter('anthropic', {
        validKeys: new Set([TRAINING_API_KEY]),
        generate: (input) => {
            const digest = planDigestKeysSchema.parse(JSON.parse(input.input))
            return { data: options.plan ? options.plan(digest) : planFromDigest(digest), usage: FAKE_USAGE }
        },
    })
    const ai = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => testKekEnv('v1'), now })
    let folderCounter = 0
    let chartCounter = 0
    const trainingCharts = () => [...training.charts.values()].map((chart) => ({ userId: chart.userId, md5: chart.md5, id: chart.id }))
    const folderMemory = createMemoryFolderDb([], trainingCharts)
    const folderService = createFolderService({
        db: folderMemory.db,
        newId: (prefix) => `${prefix}${(++folderCounter).toString(16).padStart(30, '0')}`,
        now,
        loadTableEntries: async () => null,
        lookupTableText: () => null,
    })
    const training = createMemoryTrainingDb((folderId) => folderMemory.folders.has(folderId))
    const uploads: ChartPatternDocument[] = []
    const failures: string[] = []
    const profile: Pick<WeaknessProfileService, 'current' | 'measureFocus'> = {
        current: async () => ({ profile: state.profile, fingerprint: 'f'.repeat(64), computedAt: state.now, cached: true }),
        measureFocus: async ({ keys }) => Object.fromEntries(keys.map((key) => [key, options.measured?.[key] ?? null])),
    }
    const service = createTrainingChartService({
        db: training.db,
        plans: createAiTrainingPlanService({ runner: ai.aiFeatureRunner }),
        profile,
        credentials: ai.aiCredentialService,
        folders: folderService,
        patterns: {
            readFeaturesMany: async (md5s) => new Map(md5s.filter((md5) => training.kept.patterns.has(md5)).map((md5) => [md5, {}])),
            upload: async ({ document }) => {
                uploads.push(document)
                return {
                    md5: document.md5,
                    outcome: 'created' as const,
                    parser_version: document.parser_version,
                    features_version: 2,
                    features_recomputed: true,
                    note_count: 0,
                }
            },
        },
        traces: { analyzedPlaysOfScores: options.analysed ?? (async () => new Map()) },
        frontierLevelOf: async () => '★12',
        newId: () => `tc_${(++chartCounter).toString(16).padStart(32, '0')}`,
        now,
        reportFailure: (step) => {
            failures.push(step)
        },
    })
    const registerKey = async (userId = TRAINING_USER) => {
        await ai.aiCredentialService.register({ userId, provider: 'anthropic', apiKey: TRAINING_API_KEY })
        await ai.aiCredentialService.setModel({ userId, provider: 'anthropic', model: 'claude-test' })
    }
    const create = (
        input: { focus_key: string; intensity: 'build' | 'push' | 'over' },
        userId = TRAINING_USER,
        language: 'ko' | 'en' | 'ja' = 'ko',
    ) => service.create({ userId, input, language, origin: TRAINING_ORIGIN })
    return { service, state, anthropic, ai, folderService, folderMemory, training, uploads, failures, registerKey, create }
}

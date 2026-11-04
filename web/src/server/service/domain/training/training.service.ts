import type { WeaknessProfileView } from '@server/dto/ai-play'
import { FOLDER_LIMIT, type FolderDetailOutput, type FolderItemInput } from '@server/dto/folder'
import {
    TRAINING_AUTO_FOCUS,
    TRAINING_FOLDER_DESCRIPTION,
    TRAINING_FOLDER_NAME,
    TRAINING_LIMIT,
    trainingDownloadPathOf,
    type TrainingBeforeAfter,
    type TrainingChartCreateInput,
    type TrainingChartDetail,
    type TrainingChartList,
    type TrainingChartRequest,
    type TrainingChartSummary,
    type TrainingPlay,
    type TrainingProfileSnapshot,
    type TrainingRemoval,
} from '@server/dto/training'
import type { AiServiceFailure } from '@server/service/domain/ai/ai.type'
import { cleanLine, notEnoughData } from '@server/service/domain/ai/ai-digest'
import type { AiCredentialService } from '@server/service/domain/ai/credential.service'
import { PROFILE_MIN_PLAYS } from '@server/service/domain/ai/play-feature.constants'
import {
    TRAINING_DIGEST_LIMIT,
    focusWeaknessOf,
    trainableWeaknessesOf,
    trainingPlanDigestOf,
    type AiTrainingPlanService,
    type TrainingHistoryItem,
} from '@server/service/domain/ai/training-plan.service'
import type { WeaknessProfileService } from '@server/service/domain/ai/weakness-profile.service'
import { RATIO_DECIMALS } from '@server/service/domain/analysis/analysis-constants'
import { roundToDecimals } from '@server/service/domain/analysis/play-cells'
import { WEAKNESS_KEY } from '@server/service/domain/analysis/weakness-key'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import type { FolderService } from '@server/service/domain/folder/folder.service'
import { writeTrainingBms } from '@server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '@server/service/domain/training/pattern-document'
import { correctTrainingPlan, trainingShortIdOf, trainingSpecInputOf, type TrainingSpec } from '@server/service/domain/training/spec'
import { synthesizeTraining } from '@server/service/domain/training/synthesize'
import { TRAINING_EDGE_KEYS, TRAINING_ARTIST } from '@server/service/domain/training/training-constants'
import { createTrainingZip } from '@server/service/domain/training/zip'
import type { Locale } from '@shared/i18n/locale'

/** The newest plays of a chart its detail lists (plan 2026-11-04 7.2 `TrainingPlay`). */
export const TRAINING_PLAYS_SHOWN = 50

const MICROS_PER_MILLI = 1000
const EX_PER_NOTE = 2
const TIMING_MS_DECIMALS = 1
const ISO_DATE_LENGTH = 10
const NOTE_SEPARATOR = ' · '
const KEY_LIST_SEPARATOR = ', '
const ZIP_FILE_PREFIX = 'rbms-training-'
const ZIP_FILE_EXTENSION = '.zip'

/** A stored training chart without its bytes. */
export type TrainingChartRow = {
    id: string
    md5: string
    sha256: string
    title: string
    playLevel: number
    bpm: number
    noteCount: number
    lengthMs: number
    generatorVersion: number
    spec: TrainingSpec
    request: TrainingChartRequest
    profileSnapshot: TrainingProfileSnapshot
    provider: string
    model: string
    createdAt: number
}

/** One score of the owner on a chart, with its EX denominator (`max_ex_score`, or twice the notes when it is 0). */
export type TrainingPlayRow = {
    scoreId: string
    playedAt: number
    clear: number
    exScore: number
    maxExScore: number
    minbp: number
    random: string
}

export type TrainingPlayStats = { sha256: string; plays: number; bestExRatio: number | null; lastPlayedAt: number | null }

/** A section's result in a play's stored analysis (plan 2.2.8): what the detail reads of `summary.metrics.training_sections`. */
export type TrainingSectionResult = {
    index: number
    chart_notes: number
    ex_score: number
    bp: number
    fast: number
    slow: number
    moments: { n: number; sum_us: number }
}

type AnalysedSections = { metrics: { training_sections: readonly TrainingSectionResult[] | null } | null }

export type TrainingKeptCounts = { scores: number; traces: number; replays: number }

export type TrainingChartServiceDb = {
    countCharts: (userId: string) => Promise<number>
    /** The account's training charts, newest first. */
    listCharts: (userId: string) => Promise<TrainingChartRow[]>
    /** The chart when `userId` made it; another account's chart is as absent as a missing one (plan 7.2). */
    findChart: (params: { id: string; userId: string }) => Promise<TrainingChartRow | null>
    /**
     * Stores a new chart unless the account already keeps TRAINING_LIMIT.CHARTS_PER_USER: the account's user row is locked first,
     * so two saves of one account never both pass the count (plan 4.4).
     */
    insertChart: (row: TrainingChartRow & { userId: string; bms: Uint8Array }) => Promise<'inserted' | 'limit'>
    deleteChart: (params: { id: string; userId: string }) => Promise<boolean>
    /** The bytes of a chart by its id alone, for the public download (plan 5.1). */
    findDownload: (id: string) => Promise<{ id: string; bms: Uint8Array } | null>
    findFolderLink: (userId: string) => Promise<string | null>
    /** Links the account's training folder unless one is linked, and answers the folder linked afterwards. */
    claimFolderLink: (params: { userId: string; folderId: string; createdAt: number }) => Promise<string>
    /** How the account's scores on each chart stand: their count, the best EX ratio and the newest play. */
    listPlayStats: (params: { userId: string; sha256s: readonly string[] }) => Promise<TrainingPlayStats[]>
    /** The account's scores on a chart, newest first, at most `limit`. */
    listPlays: (params: { userId: string; sha256: string; limit: number }) => Promise<TrainingPlayRow[]>
    /** What deleting a chart leaves of the account's: its scores, traces and replays on it. */
    countKept: (params: { userId: string; sha256: string }) => Promise<TrainingKeptCounts>
}

export type TrainingChartServiceDeps = {
    db: TrainingChartServiceDb
    plans: Pick<AiTrainingPlanService, 'plan'>
    profile: Pick<WeaknessProfileService, 'current' | 'measureFocus'>
    credentials: Pick<AiCredentialService, 'list'>
    folders: Pick<FolderService, 'list' | 'get' | 'create' | 'remove' | 'replaceItems' | 'removeItems' | 'listContaining'>
    /** Stores the new chart's pattern, and tells whether one is stored (only `chart-pattern` reads its table): what a delete leaves. */
    patterns: Pick<ChartPatternService, 'upload'> & { readFeaturesMany: (md5s: readonly string[]) => Promise<ReadonlyMap<string, unknown>> }
    /** The analysed plays of scores, by score id (`TraceService.analyzedPlaysOfScores`); only the section results are read. */
    traces: {
        analyzedPlaysOfScores: (params: {
            userId: string
            scoreIds: readonly string[]
        }) => Promise<ReadonlyMap<string, { summary: AnalysedSections }>>
    }
    /** The level the player is working on in the default difficulty table, for the plan digest. */
    frontierLevelOf: (userId: string) => Promise<string | null>
    newId: () => string
    now: () => number
    /** A step after the chart was saved failed (folder or pattern); the chart is still made. Codes only, never the chart. */
    reportFailure: (step: 'folder' | 'pattern', error: unknown) => void
}

export type TrainingChartFailure =
    { reason: 'chart_limit'; used: number; limit: number } | { reason: 'folder_unavailable'; limit: number } | { reason: 'not_found' }

export type TrainingChartResult<T> = { ok: true; value: T } | ({ ok: false } & (AiServiceFailure | TrainingChartFailure))

const fail = <T>(failure: AiServiceFailure | TrainingChartFailure): TrainingChartResult<T> => ({ ok: false, ...failure })

const isEdgeKey = (key: string) => (TRAINING_EDGE_KEYS as readonly string[]).includes(key)

/** The weaknesses a chart trains: its sections' focus keys but the warm-up and cool-down, in section order. */
export const trainingFocusKeysOf = (spec: TrainingSpec) => [
    ...new Set(spec.sections.map((section) => section.focus_key).filter((key) => !isEdgeKey(key))),
]

const kindsOf = (spec: TrainingSpec) => [...new Set(spec.sections.map((section) => section.kind))]

/** The private folder note of a chart (plan 5.3): what it trains and the first focus section's intent, at most NOTE_MAX. */
export const trainingFolderNoteOf = (row: Pick<TrainingChartRow, 'spec'>) => {
    const intent = row.spec.sections.find((section) => !isEdgeKey(section.focus_key))?.intent ?? ''
    const label = trainingFocusKeysOf(row.spec).join(KEY_LIST_SEPARATOR)
    return cleanLine([label, intent].filter((part) => part !== '').join(NOTE_SEPARATOR), FOLDER_LIMIT.NOTE_MAX)
}

const toFolderItem = (row: TrainingChartRow): FolderItemInput => ({
    md5: row.md5,
    sha256: row.sha256,
    title: row.title,
    artist: TRAINING_ARTIST,
    note: trainingFolderNoteOf(row),
})

/** The value of a focus key in the profile a chart is planned from: a weakness or strength, a cell, or the density limit. */
const targetOf = (profile: WeaknessProfileView, key: string) => {
    const item = [...profile.weaknesses, ...profile.strengths].find((candidate) => candidate.key === key)
    if (item && item.value !== null) return { value: item.value, unit: item.unit, notes: item.notes, charts: item.charts, evidence: item.evidence }
    const cell = profile.cells.find((candidate) => candidate.key === key)
    if (cell && cell.value !== null) return { value: cell.value, unit: cell.unit, notes: cell.notes, charts: cell.charts, evidence: cell.evidence }
    if (key !== WEAKNESS_KEY.density('limit') || profile.density.limit_nps_ex === null) return null
    return {
        value: profile.density.limit_nps_ex,
        unit: 'nps',
        notes: profile.window.notes,
        charts: profile.window.charts,
        evidence: profile.density.evidence,
    }
}

const exRatioOf = (exScore: number, maxExScore: number) => (maxExScore > 0 ? roundToDecimals(exScore / maxExScore, RATIO_DECIMALS) : null)

/**
 * The training charts (plan 2026-11-04 3, 5, 7): a chart is planned by the user's AI from the weakness profile, corrected, laid
 * out and written by the server, kept with its bytes, put in the user's "training" folder and its pattern stored, and read back
 * only by its owner, apart from the public zip of an id. Every check that can refuse a request runs before the provider is called.
 */
export const createTrainingChartService = (deps: TrainingChartServiceDeps) => {
    const downloadUrlOf = (origin: string, id: string) => `${origin}${trainingDownloadPathOf(id)}`

    const statsOf = async (userId: string, rows: readonly TrainingChartRow[]) =>
        new Map((await deps.db.listPlayStats({ userId, sha256s: rows.map((row) => row.sha256) })).map((stats) => [stats.sha256, stats]))

    const summaryOf = (row: TrainingChartRow, stats: TrainingPlayStats | undefined, origin: string) =>
        ({
            id: row.id,
            sha256: row.sha256,
            title: row.title,
            created_at: row.createdAt,
            focus_keys: trainingFocusKeysOf(row.spec),
            kinds: kindsOf(row.spec),
            note_count: row.noteCount,
            bpm: row.bpm,
            play_level: row.playLevel,
            length_ms: row.lengthMs,
            download_url: downloadUrlOf(origin, row.id),
            plays: stats?.plays ?? 0,
            best_ex_ratio: stats?.bestExRatio ?? null,
            last_played_at: stats?.lastPlayedAt ?? null,
        }) satisfies TrainingChartSummary

    const playsOf = async (userId: string, row: TrainingChartRow) => {
        const plays = await deps.db.listPlays({ userId, sha256: row.sha256, limit: TRAINING_PLAYS_SHOWN })
        const analysed = await deps.traces.analyzedPlaysOfScores({ userId, scoreIds: plays.map((play) => play.scoreId) })
        return plays.map((play): TrainingPlay => {
            const sections = analysed.get(play.scoreId)?.summary.metrics?.training_sections ?? null
            return {
                score_id: play.scoreId,
                played_at: play.playedAt,
                clear: play.clear,
                ex_score: play.exScore,
                ex_ratio: exRatioOf(play.exScore, play.maxExScore),
                bp: play.minbp,
                random: play.random,
                sections:
                    sections?.map((section) => ({
                        index: section.index,
                        ex_ratio: exRatioOf(section.ex_score, section.chart_notes * EX_PER_NOTE),
                        bp: section.bp,
                        mean_err_ms:
                            section.moments.n > 0
                                ? roundToDecimals(section.moments.sum_us / section.moments.n / MICROS_PER_MILLI, TIMING_MS_DECIMALS)
                                : null,
                        fast: section.fast,
                        slow: section.slow,
                    })) ?? null,
            }
        })
    }

    const beforeAfterOf = async (userId: string, row: TrainingChartRow) => {
        const targets = Object.entries(row.profileSnapshot.targets)
        if (targets.length === 0) return []
        const measured = await deps.profile.measureFocus({ userId, keys: targets.map(([key]) => key), sinceMs: row.createdAt })
        return targets.map(([key, before]): TrainingBeforeAfter => {
            const after = measured[key] ?? null
            return {
                focus_key: key,
                unit: before.unit,
                before,
                after: after
                    ? { value: after.value, unit: after.unit, notes: after.notes, charts: after.charts, plays: after.plays, evidence: after.evidence }
                    : null,
                delta: after ? roundToDecimals(after.value - before.value, RATIO_DECIMALS) : null,
            }
        })
    }

    const detailOf = async (userId: string, row: TrainingChartRow, origin: string) => {
        const [stats, plays, beforeAfter] = await Promise.all([statsOf(userId, [row]), playsOf(userId, row), beforeAfterOf(userId, row)])
        const { plays: playCount, ...summary } = summaryOf(row, stats.get(row.sha256), origin)
        return {
            ...summary,
            play_count: playCount,
            description: row.request.description,
            advice: row.request.advice,
            sections: row.spec.sections.map((section) => ({
                index: section.index,
                focus_key: section.focus_key,
                kind: section.kind,
                from_ms: Math.floor(section.from_us / MICROS_PER_MILLI),
                to_ms: Math.floor(section.to_us / MICROS_PER_MILLI),
                target_nps: section.target_nps,
                actual_nps: section.actual_nps,
                lane_hint: section.lane_hint,
                intent: section.intent,
            })),
            corrections: row.spec.corrections,
            plays,
            before_after: beforeAfter,
        } satisfies TrainingChartDetail
    }

    const linkedFolder = async (userId: string) => {
        const folderId = await deps.db.findFolderLink(userId)
        return folderId ? deps.folders.get({ userId, folderId }) : null
    }

    /** Whether the training folder is there or can be made (plan 5.3), checked before the provider is called. */
    const hasFolderRoom = async (userId: string) =>
        (await linkedFolder(userId)) !== null || (await deps.folders.list(userId)).length < FOLDER_LIMIT.FOLDERS_PER_USER

    const createFolder = async (userId: string, locale: Locale) => {
        const input = { name: TRAINING_FOLDER_NAME[locale], description: TRAINING_FOLDER_DESCRIPTION[locale] }
        const created = await deps.folders.create({ userId, input })
        if (created.ok || created.reason !== 'name_taken') return created
        const dated = `${input.name} ${new Date(deps.now()).toISOString().slice(0, ISO_DATE_LENGTH)}`
        return deps.folders.create({ userId, input: { ...input, name: dated } })
    }

    const makeLinkedFolder = async (userId: string, locale: Locale) => {
        const created = await createFolder(userId, locale)
        if (!created.ok) return null
        const claimedId = await deps.db.claimFolderLink({ userId, folderId: created.value.id, createdAt: deps.now() })
        if (claimedId !== created.value.id) await deps.folders.remove({ userId, folderId: created.value.id })
        return deps.folders.get({ userId, folderId: claimedId })
    }

    /**
     * Makes sure the account's training folder holds every one of its training charts (plan 5.3): the linked folder, or a new one
     * named in the screen's language, gets the charts it does not hold at its top, newest first. A folder edited meanwhile is left
     * as it is; the next chart fills it.
     */
    const ensureTrainingFolder = async (userId: string, locale: Locale) => {
        const folder: FolderDetailOutput | null = (await linkedFolder(userId)) ?? (await makeLinkedFolder(userId, locale))
        if (!folder) return
        const held = new Set(folder.items.map((item) => item.md5))
        const missing = (await deps.db.listCharts(userId)).filter((row) => !held.has(row.md5))
        if (missing.length === 0) return
        const kept = folder.items.map(({ md5, sha256, title, artist, note }) => ({ md5, sha256, title, artist, note }))
        await deps.folders.replaceItems({
            userId,
            folderId: folder.id,
            input: { items: [...missing.map(toFolderItem), ...kept], base_updated_at: folder.updated_at },
        })
    }

    const historyOf = async (userId: string): Promise<TrainingHistoryItem[]> => {
        const rows = (await deps.db.listCharts(userId)).slice(0, TRAINING_DIGEST_LIMIT.PREVIOUS)
        return Promise.all(
            rows.map(async (row) => {
                const plays = await deps.db.listPlays({ userId, sha256: row.sha256, limit: TRAINING_PLAYS_SHOWN })
                const ratios = plays.map((play) => exRatioOf(play.exScore, play.maxExScore)).filter((ratio) => ratio !== null)
                const newest = ratios[0]
                const oldest = ratios[ratios.length - 1]
                return {
                    created_at: row.createdAt,
                    focus_keys: trainingFocusKeysOf(row.spec),
                    kinds: kindsOf(row.spec),
                    result_delta:
                        ratios.length >= 2 && newest !== undefined && oldest !== undefined ? roundToDecimals(newest - oldest, RATIO_DECIMALS) : null,
                }
            }),
        )
    }

    const snapshotOf = (current: Awaited<ReturnType<WeaknessProfileService['current']>>, spec: TrainingSpec) =>
        ({
            profile_version: current.profile.profile_version,
            fingerprint: current.fingerprint,
            computed_at: current.computedAt,
            targets: Object.fromEntries(
                trainingFocusKeysOf(spec).flatMap((key) => {
                    const target = targetOf(current.profile, key)
                    return target ? [[key, target] as const] : []
                }),
            ),
        }) satisfies TrainingProfileSnapshot

    const ownedChart = async (params: { userId: string; id: string }) => deps.db.findChart(params)

    const removalOf = async (userId: string, row: TrainingChartRow) => {
        const [folderIds, kept, patterns] = await Promise.all([
            deps.folders.listContaining({ userId, md5: row.md5 }),
            deps.db.countKept({ userId, sha256: row.sha256 }),
            deps.patterns.readFeaturesMany([row.md5]),
        ])
        return {
            removes: { training_chart: 1, folder_items: folderIds.length },
            keeps: { chart_pattern: patterns.has(row.md5) ? 1 : 0, scores: kept.scores, traces: kept.traces, replays: kept.replays },
        } satisfies TrainingRemoval
    }

    return {
        /** The owner's charts, newest first, with the room left and whether an AI key is registered. No provider is called. */
        list: async (params: { userId: string; origin: string }) => {
            const [rows, credentials] = await Promise.all([deps.db.listCharts(params.userId), deps.credentials.list(params.userId)])
            const stats = await statsOf(params.userId, rows)
            return {
                charts: rows.map((row) => summaryOf(row, stats.get(row.sha256), params.origin)),
                used: rows.length,
                limit: TRAINING_LIMIT.CHARTS_PER_USER,
                can_create: rows.length < TRAINING_LIMIT.CHARTS_PER_USER,
                needs_key: credentials.length === 0,
            } satisfies TrainingChartList
        },

        /**
         * Makes one training chart (plan 3.1). Refused before any provider call when the account keeps the most charts, has no AI
         * key, has no room for the training folder, has too few traced plays, or names a weakness its profile cannot train. Then
         * the AI plans it once (no retry), the server corrects, lays out and writes it, and stores it if the account still has room;
         * the folder and the pattern are best effort after that.
         */
        create: async (params: {
            userId: string
            input: TrainingChartCreateInput
            language: Locale
            origin: string
        }): Promise<TrainingChartResult<TrainingChartDetail>> => {
            const { userId, input } = params
            const used = await deps.db.countCharts(userId)
            if (used >= TRAINING_LIMIT.CHARTS_PER_USER) return fail({ reason: 'chart_limit', used, limit: TRAINING_LIMIT.CHARTS_PER_USER })
            if ((await deps.credentials.list(userId)).length === 0) return fail({ reason: 'credential_not_found' })
            if (!(await hasFolderRoom(userId))) return fail({ reason: 'folder_unavailable', limit: FOLDER_LIMIT.FOLDERS_PER_USER })

            const current = await deps.profile.current({ userId })
            if (current.profile.window.plays < PROFILE_MIN_PLAYS) return notEnoughData('traced_plays')
            const hasFocus =
                input.focus_key === TRAINING_AUTO_FOCUS
                    ? trainableWeaknessesOf(current.profile).length > 0
                    : focusWeaknessOf(current.profile, input.focus_key) !== null
            if (!hasFocus) return notEnoughData('weakness_profile')

            const [history, frontierLevel] = await Promise.all([historyOf(userId), deps.frontierLevelOf(userId)])
            const digest = trainingPlanDigestOf(current.profile, history, input, { frontierLevel })
            const planned = await deps.plans.plan({ userId, digest, language: params.language })
            if (!planned.ok) return planned

            const plan = planned.value.output
            const corrected = correctTrainingPlan(plan, { allowed: digest.allowed, bounds: digest.bounds })
            const id = deps.newId()
            const createdAt = deps.now()
            const { spec, notes } = synthesizeTraining(
                trainingSpecInputOf({ id, userId, createdAt, intensity: input.intensity, plan: corrected.spec, corrections: corrected.corrections }),
            )
            const written = writeTrainingBms(spec, notes)
            const document = buildTrainingPatternDocument({ spec, lines: written.lines, md5: written.md5, sha256: written.sha256 })
            const row = {
                id,
                md5: written.md5,
                sha256: written.sha256,
                title: spec.title,
                playLevel: spec.play_level,
                bpm: spec.base_bpm,
                noteCount: document.playable_notes,
                lengthMs: Math.floor((document.notes.time_us.at(-1) ?? 0) / MICROS_PER_MILLI),
                generatorVersion: spec.generator_version,
                spec,
                request: { focus_key: input.focus_key, intensity: input.intensity, plan, advice: plan.advice, description: plan.description },
                profileSnapshot: snapshotOf(current, spec),
                provider: planned.value.provider,
                model: planned.value.model,
                createdAt,
            } satisfies TrainingChartRow
            if ((await deps.db.insertChart({ ...row, userId, bms: written.bytes })) === 'limit') {
                return fail({ reason: 'chart_limit', used: TRAINING_LIMIT.CHARTS_PER_USER, limit: TRAINING_LIMIT.CHARTS_PER_USER })
            }

            try {
                await ensureTrainingFolder(userId, params.language)
            } catch (error) {
                deps.reportFailure('folder', error)
            }
            try {
                await deps.patterns.upload({ document, uploadedBy: userId })
            } catch (error) {
                deps.reportFailure('pattern', error)
            }
            return { ok: true, value: await detailOf(userId, row, params.origin) }
        },

        /** One chart of the owner with its plays, their section results and the before/after of its focus keys; null otherwise. */
        detail: async (params: { userId: string; id: string; origin: string }) => {
            const row = await ownedChart(params)
            return row ? detailOf(params.userId, row, params.origin) : null
        },

        /** What deleting one chart of the owner would take away and leave (plan 7.2); null for any other id. */
        removal: async (params: { userId: string; id: string }) => {
            const row = await ownedChart(params)
            return row ? removalOf(params.userId, row) : null
        },

        /**
         * Deletes one chart of the owner and its items in every folder of the account; its pattern, chart row, scores, traces and
         * replays stay (plan 4.4). Null for any other id. The counts are those `removal` gives, taken before the delete.
         */
        remove: async (params: { userId: string; id: string }) => {
            const row = await ownedChart(params)
            if (!row) return null
            const removal = await removalOf(params.userId, row)
            if (!(await deps.db.deleteChart(params))) return null
            const folderIds = await deps.folders.listContaining({ userId: params.userId, md5: row.md5 })
            await Promise.all(folderIds.map((folderId) => deps.folders.removeItems({ userId: params.userId, folderId, md5s: [row.md5] })))
            return { ...removal, removed: true as const }
        },

        /** The zip of a chart by its id alone (plan 5.1), with the file name the client saves it as; null when there is none. */
        download: async (id: string) => {
            const found = await deps.db.findDownload(id)
            if (!found) return null
            return {
                fileName: `${ZIP_FILE_PREFIX}${trainingShortIdOf(found.id)}${ZIP_FILE_EXTENSION}`,
                bytes: createTrainingZip({ id: found.id, bms: found.bms }),
            }
        },
    }
}

export type TrainingChartService = ReturnType<typeof createTrainingChartService>

import type { ChartPatternDocument } from '@server/dto/chart-pattern'
import { TRACE_QUOTA_LEVEL, TRACE_USER_QUOTA, traceUploadSchema, type TraceUploadInput } from '@server/dto/trace'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import {
    CURRENT_ANALYSIS_VERSIONS,
    analysisFailureSummary,
    analyzeTrace,
    type AnalysisVersions,
    type PatternJoin,
    type TraceAnalysisFailure,
    type TraceAnalysisSummary,
} from '@server/service/domain/trace/trace-analysis'
import { findTraceMismatch, type TraceScoreRow } from '@server/service/domain/trace/trace-check'
import { gunzipJson, gzipJsonText } from '@server/service/shared/storage/gzip-json'

export type TraceInsertRow = {
    id: string
    scoreId: string
    userId: string
    chartSha256: string
    chartMd5: string | null
    playedAt: number
    formatVersion: number
    noteCount: number
    byteSize: number
    storedSize: number
    data: Buffer
    createdAt: number
}

export type TraceAnalysisRow = {
    traceId: string
    analysisVersion: number
    summary: TraceAnalysisSummary
    patternJoin: PatternJoin
    patternMd5: string | null
    computedAt: number
}

export type TraceAnalysisFailureRow = {
    traceId: string
    analysisVersion: number
    summary: TraceAnalysisFailure
    computedAt: number
}

export type TraceUsage = { count: number; bytes: number }

export type TraceUserUsageRow = { loginId: string; traces: number; bytes: number; storedBytes: number }

/** The score columns a play's analysis is read with: its result and the run conditions the analysis groups plays by. */
export type TracedPlayScore = {
    clear: number
    exScore: number
    maxExScore: number
    minbp: number
    gauge: number
    random: string
    scratchAuto: boolean
    judgeRate: number
    assist: number
    autoplay: boolean
    autoOffset: boolean
    constant: boolean
    greenNumber: number | null
}

/** One traced play of an account as stored: its trace's row, its score's columns and its stored analysis, if any. */
export type TracedPlayRow = {
    traceId: string
    scoreId: string
    chartSha256: string
    chartMd5: string | null
    playedAt: number
    score: TracedPlayScore
    analysis: { analysisVersion: number; summary: unknown } | null
}

export type PlayShareSettingRow = { userId: string; rivalSegments: boolean; updatedAt: number }

export type TraceServiceDb = {
    /** The score `scoreId` names when `userId` owns it; another account's score is as absent as a missing one. */
    findOwnedScore: (params: { scoreId: string; userId: string }) => Promise<TraceScoreRow | null>
    findTraceIdByScoreId: (params: { scoreId: string; userId: string }) => Promise<string | null>
    /** Inserts the trace unless one for its score is there; false when it was. */
    insertTraceIfAbsent: (row: TraceInsertRow) => Promise<boolean>
    /** How many traces the account keeps and their JSON bytes. */
    sumUserTraces: (userId: string) => Promise<TraceUsage>
    saveAnalysis: (row: TraceAnalysisRow) => Promise<void>
    /** Marks a trace whose analysis failed at these versions, so it is not computed again until one of them goes up. */
    saveAnalysisFailure: (row: TraceAnalysisFailureRow) => Promise<void>
    findStoredTrace: (params: {
        traceId: string
        userId: string
    }) => Promise<{ data: Buffer; chartSha256: string; analysis: { analysisVersion: number; summary: unknown } | null } | null>
    /** Every account that keeps traces, the most JSON bytes first, at most `limit`, and the totals over all accounts. */
    summarizeUsage: (
        limit: number,
    ) => Promise<{ top: TraceUserUsageRow[]; totals: { users: number; traces: number; bytes: number; storedBytes: number } }>
    /** How many accounts are at or past `near` and `over` of either quota limit. */
    countAccountsPast: (limits: { near: TraceUsage; over: TraceUsage }) => Promise<{ near: number; over: number }>
    /** Traces with no analysis, and with one whose play version or pattern classification version is lower than `versions`. */
    countAnalysesBehind: (versions: AnalysisVersions) => Promise<{ missing: number; stale: number }>
    /** Traces with no analysis or one behind `versions` the same way, in id order after `after`, at most `limit`. */
    listTracesBehind: (params: { versions: AnalysisVersions; after: string | null; limit: number }) => Promise<{ id: string; userId: string }[]>
    /** The account's traced plays, newest first, at most `limit`; autoplay is left out. */
    listRecentPlays: (params: { userId: string; limit: number }) => Promise<TracedPlayRow[]>
    /** The traced plays of these scores of the account. */
    listPlaysOfScores: (params: { userId: string; scoreIds: readonly string[] }) => Promise<TracedPlayRow[]>
    /** The account's traced play of one score. */
    findPlayOfScore: (params: { userId: string; scoreId: string }) => Promise<TracedPlayRow | null>
    /** The account's traced play with the highest EX score on a chart with that SCRATCH AUTO, the newest on a tie, leaving one score out. */
    findBestPlayOnChart: (params: {
        userId: string
        chartSha256: string
        scratchAuto: boolean
        excludeScoreId: string | null
    }) => Promise<TracedPlayRow | null>
    /** Whether `userId` keeps `rivalId` as a rival (`rival` table). */
    isRivalOf: (params: { userId: string; rivalId: string }) => Promise<boolean>
    findShareSetting: (userId: string) => Promise<PlayShareSettingRow | null>
    saveShareSetting: (row: PlayShareSettingRow) => Promise<void>
}

export type TraceServiceDeps = {
    db: TraceServiceDb
    newId: (prefix: string) => string
    now: () => number
    /** The chart's pattern by md5, for the pattern join only; it never leaves the server. */
    loadPattern: (md5: string) => Promise<ChartPatternDocument | null>
    /** Called when a stored trace could not be analysed; the trace stays stored and, once marked failed, is analysed again only after a version goes up. */
    reportAnalysisFailure: (error: unknown) => void
}

export const TRACE_ID_PREFIX = 'tr_'

/** Accounts the admin usage lists by name, the most bytes first. */
export const TRACE_USAGE_TOP_USERS = 20

export const TRACE_UPLOAD_OUTCOME = { CREATED: 'created', EXISTING: 'existing', SCORE_NOT_FOUND: 'score_not_found', MISMATCH: 'mismatch' } as const

export const TRACE_ANALYSIS_STATE = { STORED: 'stored', PENDING: 'pending' } as const

const NEAR_LIMITS = {
    count: Math.ceil(TRACE_USER_QUOTA.COUNT_MAX * TRACE_USER_QUOTA.NEAR_RATIO),
    bytes: Math.ceil(TRACE_USER_QUOTA.TOTAL_BYTES_MAX * TRACE_USER_QUOTA.NEAR_RATIO),
}

/** Where an account stands against the trace quota: `over` past either limit, `near` from NEAR_RATIO of either, `ok` otherwise. */
export const traceQuotaLevelOf = (usage: TraceUsage) => {
    if (usage.count > TRACE_USER_QUOTA.COUNT_MAX || usage.bytes > TRACE_USER_QUOTA.TOTAL_BYTES_MAX) return TRACE_QUOTA_LEVEL.OVER
    if (usage.count >= NEAR_LIMITS.count || usage.bytes >= NEAR_LIMITS.bytes) return TRACE_QUOTA_LEVEL.NEAR
    return TRACE_QUOTA_LEVEL.OK
}

/** The quota view an account is shown: what it keeps, the limits, and its level. A trace past the limits is still stored. */
export const toTraceStorage = (usage: TraceUsage) => ({
    traces: usage.count,
    bytes: usage.bytes,
    limit_traces: TRACE_USER_QUOTA.COUNT_MAX,
    limit_bytes: TRACE_USER_QUOTA.TOTAL_BYTES_MAX,
    level: traceQuotaLevelOf(usage),
})

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null

/** Stored analyses one read computes again at most; the rest are left out of that read and counted, and the admin recompute catches up. */
export const PLAY_ANALYSIS_READ_RECOMPUTE_MAX = 10

/** A stored summary of both current versions (play summary and pattern classification), which this build wrote and so reads as it is. */
const isCurrentSummary = (value: unknown): value is TraceAnalysisSummary =>
    isRecord(value) &&
    value.failed !== true &&
    value.analysis_version === CURRENT_ANALYSIS_VERSIONS.play &&
    value.pattern_analysis_version === CURRENT_ANALYSIS_VERSIONS.pattern

/** The marker of an analysis that failed at both current versions. */
const isCurrentFailure = (value: unknown): value is TraceAnalysisFailure =>
    isRecord(value) &&
    value.failed === true &&
    value.analysis_version === CURRENT_ANALYSIS_VERSIONS.play &&
    value.pattern_analysis_version === CURRENT_ANALYSIS_VERSIONS.pattern

/**
 * Keeps each run's trace (`play_trace`, gzip-compressed, never sent back out) and its analysis (`play_analysis`). An upload names
 * a score its account owns and is taken only when its judgements add up to that score's exactly; one score keeps one trace.
 */
export const createTraceService = (deps: TraceServiceDeps) => {
    const analyze = async (trace: TraceUploadInput, row: { traceId: string; chartSha256: string }) => {
        const pattern = trace.chart.md5 ? await deps.loadPattern(trace.chart.md5) : null
        const { summary, patternJoin, patternMd5 } = analyzeTrace({ trace, chartSha256: row.chartSha256, pattern })
        await deps.db.saveAnalysis({
            traceId: row.traceId,
            analysisVersion: CURRENT_ANALYSIS_VERSIONS.play,
            summary,
            patternJoin,
            patternMd5,
            computedAt: deps.now(),
        })
        return summary
    }

    const analyzeOrReport = async (trace: TraceUploadInput, row: { traceId: string; chartSha256: string }) => {
        try {
            await analyze(trace, row)
            return TRACE_ANALYSIS_STATE.STORED
        } catch (error) {
            deps.reportAnalysisFailure(error)
            return TRACE_ANALYSIS_STATE.PENDING
        }
    }

    const storedSummaryOf = (row: Pick<TracedPlayRow, 'analysis'>) => {
        const summary = row.analysis?.analysisVersion === CURRENT_ANALYSIS_VERSIONS.play ? row.analysis.summary : null
        return isCurrentSummary(summary) ? summary : null
    }

    const isFailedAtCurrentVersions = (row: Pick<TracedPlayRow, 'analysis'>) =>
        row.analysis?.analysisVersion === CURRENT_ANALYSIS_VERSIONS.play && isCurrentFailure(row.analysis.summary)

    const markFailed = async (traceId: string) => {
        try {
            await deps.db.saveAnalysisFailure({
                traceId,
                analysisVersion: CURRENT_ANALYSIS_VERSIONS.play,
                summary: analysisFailureSummary(),
                computedAt: deps.now(),
            })
        } catch (error) {
            deps.reportAnalysisFailure(error)
        }
    }

    /**
     * Computes a stored trace's analysis again and stores it. It never throws: a trace that does not read or fails to analyse is
     * reported and marked failed (`markFailed`) and answers null, so one broken trace cannot fail a read of the others.
     */
    const recomputeStored = async (params: { traceId: string; userId: string }) => {
        try {
            const stored = await deps.db.findStoredTrace(params)
            if (!stored) return null
            const parsed = traceUploadSchema.safeParse(await gunzipJson(stored.data))
            if (!parsed.success) {
                deps.reportAnalysisFailure(parsed.error)
                await markFailed(params.traceId)
                return null
            }
            return await analyze(parsed.data, { traceId: params.traceId, chartSha256: stored.chartSha256 })
        } catch (error) {
            deps.reportAnalysisFailure(error)
            await markFailed(params.traceId)
            return null
        }
    }

    /**
     * The plays with their current analyses. A play whose stored analysis is missing or of an older version is computed again, at most
     * PLAY_ANALYSIS_READ_RECOMPUTE_MAX of them in the given order; the others are left out and counted in `stale`. A play that cannot
     * be analysed, or was marked failed at the current versions, is left out and counted in `failed`; the rest are still answered.
     */
    const withCurrentAnalyses = async (userId: string, rows: readonly TracedPlayRow[]) => {
        const plays: (TracedPlayRow & { summary: TraceAnalysisSummary })[] = []
        let recomputed = 0
        let stale = 0
        let failed = 0
        for (const row of rows) {
            const stored = storedSummaryOf(row)
            if (stored) {
                plays.push({ ...row, summary: stored })
                continue
            }
            if (isFailedAtCurrentVersions(row)) {
                failed += 1
                continue
            }
            if (recomputed >= PLAY_ANALYSIS_READ_RECOMPUTE_MAX) {
                stale += 1
                continue
            }
            recomputed += 1
            const summary = await recomputeStored({ traceId: row.traceId, userId })
            if (summary) plays.push({ ...row, summary })
            else failed += 1
        }
        return { plays, stale, failed }
    }

    const existing = async (id: string, userId: string) => ({
        outcome: TRACE_UPLOAD_OUTCOME.EXISTING,
        id,
        storage: toTraceStorage(await deps.db.sumUserTraces(userId)),
    })

    return {
        /**
         * Stores the trace of a score the uploading account owns. A score that already has a trace answers that trace's id
         * (`existing`) whatever the body says. Otherwise the trace must agree with its score (`findTraceMismatch`) and is stored
         * and analysed (`created`); a failed analysis leaves it stored and `pending`. The quota never refuses a trace.
         */
        upload: async (params: { input: TraceUploadInput; pathHash: string; userId: string }) => {
            const { input, userId } = params
            const score = await deps.db.findOwnedScore({ scoreId: input.score_id, userId })
            if (!score) return { outcome: TRACE_UPLOAD_OUTCOME.SCORE_NOT_FOUND }
            const storedId = await deps.db.findTraceIdByScoreId({ scoreId: score.id, userId })
            if (storedId) return existing(storedId, userId)
            const mismatch = findTraceMismatch({ trace: input, pathHash: params.pathHash, score })
            if (mismatch) return { outcome: TRACE_UPLOAD_OUTCOME.MISMATCH, ...mismatch }

            const text = JSON.stringify(input)
            const id = deps.newId(TRACE_ID_PREFIX)
            const data = await gzipJsonText(text)
            const row = {
                id,
                scoreId: score.id,
                userId,
                chartSha256: score.chartSha256,
                chartMd5: score.chartMd5 ?? (input.chart.md5 || null),
                playedAt: score.playedAt,
                formatVersion: input.version,
                noteCount: input.notes.time_us.length,
                byteSize: Buffer.byteLength(text, 'utf8'),
                storedSize: data.length,
                data,
                createdAt: deps.now(),
            } satisfies TraceInsertRow
            if (!(await deps.db.insertTraceIfAbsent(row))) {
                const racedId = await deps.db.findTraceIdByScoreId({ scoreId: score.id, userId })
                if (racedId) return existing(racedId, userId)
                throw createAppError(ERROR_CODE.INTERNAL_ERROR, { reason: 'trace_insert_lost_to_a_missing_trace' })
            }
            const analysis = await analyzeOrReport(input, { traceId: id, chartSha256: score.chartSha256 })
            return { outcome: TRACE_UPLOAD_OUTCOME.CREATED, id, analysis, storage: toTraceStorage(await deps.db.sumUserTraces(userId)) }
        },

        /** The account's trace quota view, for the screen's warning. */
        usageOf: async (userId: string) => toTraceStorage(await deps.db.sumUserTraces(userId)),

        /** Counts for the admin: totals, the accounts nearest the quota by bytes, how many are near or over, and analyses behind. */
        adminUsage: async () => {
            const [{ top, totals }, past, analyses] = await Promise.all([
                deps.db.summarizeUsage(TRACE_USAGE_TOP_USERS),
                deps.db.countAccountsPast({
                    near: { count: NEAR_LIMITS.count, bytes: NEAR_LIMITS.bytes },
                    over: { count: TRACE_USER_QUOTA.COUNT_MAX, bytes: TRACE_USER_QUOTA.TOTAL_BYTES_MAX },
                }),
                deps.db.countAnalysesBehind(CURRENT_ANALYSIS_VERSIONS),
            ])
            return {
                limits: { traces: TRACE_USER_QUOTA.COUNT_MAX, bytes: TRACE_USER_QUOTA.TOTAL_BYTES_MAX, near_ratio: TRACE_USER_QUOTA.NEAR_RATIO },
                totals: { users: totals.users, traces: totals.traces, bytes: totals.bytes, stored_bytes: totals.storedBytes },
                accounts_near: past.near,
                accounts_over: past.over,
                top: top.map((row) => ({
                    login_id: row.loginId,
                    traces: row.traces,
                    bytes: row.bytes,
                    stored_bytes: row.storedBytes,
                    level: traceQuotaLevelOf({ count: row.traces, bytes: row.bytes }),
                })),
                analysis: {
                    analysis_version: CURRENT_ANALYSIS_VERSIONS.play,
                    pattern_analysis_version: CURRENT_ANALYSIS_VERSIONS.pattern,
                    missing: analyses.missing,
                    stale: analyses.stale,
                },
            }
        },

        /**
         * One run's analysis for its own account, computed again from the stored trace when it is missing or of an older version
         * (play summary or pattern classification). Null when the account keeps no such trace or the stored trace does not read as one or
         * could not be analysed.
         */
        readAnalysis: async (params: { traceId: string; userId: string }) => {
            const stored = await deps.db.findStoredTrace(params)
            if (!stored) return null
            const current = storedSummaryOf({ analysis: stored.analysis })
            if (current) return current
            if (isFailedAtCurrentVersions({ analysis: stored.analysis })) return null
            return recomputeStored(params)
        },

        /** The account's newest analysed plays (autoplay left out), at most `limit`, and how many were left out for an old analysis (`stale`) or one that failed (`failed`). */
        recentAnalyzedPlays: async (params: { userId: string; limit: number }) =>
            withCurrentAnalyses(params.userId, await deps.db.listRecentPlays(params)),

        /** The analysed plays of these scores of the account, by score id. A score with no trace, or no readable one, is absent. */
        analyzedPlaysOfScores: async (params: { userId: string; scoreIds: readonly string[] }) => {
            if (params.scoreIds.length === 0) return new Map<string, TracedPlayRow & { summary: TraceAnalysisSummary }>()
            const { plays } = await withCurrentAnalyses(params.userId, await deps.db.listPlaysOfScores(params))
            return new Map(plays.map((play) => [play.scoreId, play]))
        },

        /** The account's analysed play of one score, or null when it has no trace (or none that reads). */
        analyzedPlayOfScore: async (params: { userId: string; scoreId: string }) => {
            const row = await deps.db.findPlayOfScore(params)
            if (!row) return null
            const { plays } = await withCurrentAnalyses(params.userId, [row])
            return plays[0] ?? null
        },

        /** The account's best analysed play on a chart (highest EX, then newest) with that SCRATCH AUTO, other than `excludeScoreId`. */
        bestAnalyzedPlayOnChart: async (params: { userId: string; chartSha256: string; scratchAuto: boolean; excludeScoreId: string | null }) => {
            const row = await deps.db.findBestPlayOnChart(params)
            if (!row) return null
            const { plays } = await withCurrentAnalyses(params.userId, [row])
            return plays[0] ?? null
        },

        /**
         * A rival's segment totals on a chart, for a comparison only (ADR Q4): `allowed` is false unless the two keep each other as
         * rivals (a rival can be added without the other's consent, so one side's list is not enough) and the rival turned on
         * `rival_segments`. Only per-segment counts of the rival's best traced play there leave this service; its notes, lanes, timing
         * and tags do not.
         */
        rivalSegmentsOnChart: async (params: { requesterId: string; rivalId: string; chartSha256: string; scratchAuto: boolean }) => {
            const [requesterKeepsRival, rivalKeepsRequester, setting] = await Promise.all([
                deps.db.isRivalOf({ userId: params.requesterId, rivalId: params.rivalId }),
                deps.db.isRivalOf({ userId: params.rivalId, rivalId: params.requesterId }),
                deps.db.findShareSetting(params.rivalId),
            ])
            if (!requesterKeepsRival || !rivalKeepsRequester || setting?.rivalSegments !== true) return { allowed: false } as const
            const row = await deps.db.findBestPlayOnChart({
                userId: params.rivalId,
                chartSha256: params.chartSha256,
                scratchAuto: params.scratchAuto,
                excludeScoreId: null,
            })
            const { plays } = row ? await withCurrentAnalyses(params.rivalId, [row]) : { plays: [] }
            const play = plays[0]
            if (!play?.summary.metrics) return { allowed: true, play: null } as const
            return {
                allowed: true,
                play: {
                    scoreId: play.scoreId,
                    playedAt: play.playedAt,
                    exScore: play.score.exScore,
                    clear: play.score.clear,
                    random: play.summary.random,
                    scratchAuto: play.summary.scratch_auto,
                    lntype: play.summary.lntype,
                    runEnd: play.summary.run_end,
                    segments: play.summary.metrics.segments.map(({ id, chart_notes, notes, ex_score, bp }) => ({
                        id,
                        chart_notes,
                        notes,
                        ex_score,
                        bp,
                    })),
                },
            } as const
        },

        /** What the account shares of its analyses: off until it turns it on. */
        shareSettingOf: async (userId: string) => {
            const row = await deps.db.findShareSetting(userId)
            return { rival_segments: row?.rivalSegments ?? false, updated_at: row?.updatedAt ?? null }
        },

        putShareSetting: async (params: { userId: string; rivalSegments: boolean }) => {
            const updatedAt = deps.now()
            await deps.db.saveShareSetting({ userId: params.userId, rivalSegments: params.rivalSegments, updatedAt })
            return { rival_segments: params.rivalSegments, updated_at: updatedAt }
        },

        /**
         * Computes again, in id order after `after`, at most `limit` analyses that are missing or behind a version (play summary or
         * pattern classification). A trace that does not read or fails to analyse is listed in `skipped`, reported
         * (`reportAnalysisFailure`) and marked failed, so it is not looked at again until a version goes up; `next_after` is the last id
         * looked at (null when none was), so a caller passing it back goes on without repeating a trace.
         */
        recomputeStale: async (params: { limit: number; after: string | null }) => {
            const rows = await deps.db.listTracesBehind({ versions: CURRENT_ANALYSIS_VERSIONS, after: params.after, limit: params.limit })
            const skipped: string[] = []
            for (const row of rows) {
                if (!(await recomputeStored({ traceId: row.id, userId: row.userId }))) skipped.push(row.id)
            }
            const behind = await deps.db.countAnalysesBehind(CURRENT_ANALYSIS_VERSIONS)
            return {
                analysis_version: CURRENT_ANALYSIS_VERSIONS.play,
                pattern_analysis_version: CURRENT_ANALYSIS_VERSIONS.pattern,
                examined: rows.length,
                recomputed: rows.length - skipped.length,
                skipped,
                next_after: rows.at(-1)?.id ?? null,
                remaining: behind.missing + behind.stale,
            }
        },
    }
}

export type TraceService = ReturnType<typeof createTraceService>
export type AnalyzedPlay = NonNullable<Awaited<ReturnType<TraceService['analyzedPlayOfScore']>>>
export type RivalSegmentPlay = NonNullable<Extract<Awaited<ReturnType<TraceService['rivalSegmentsOnChart']>>, { allowed: true }>['play']>

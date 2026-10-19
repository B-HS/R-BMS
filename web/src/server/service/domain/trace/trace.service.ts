import type { ChartPatternDocument } from '@server/dto/chart-pattern'
import { TRACE_QUOTA_LEVEL, TRACE_USER_QUOTA, traceUploadSchema, type TraceUploadInput } from '@server/dto/trace'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { analyzeTrace, type PatternJoin, type TraceAnalysisSummary } from '@server/service/domain/trace/trace-analysis'
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

export type TraceUsage = { count: number; bytes: number }

export type TraceUserUsageRow = { loginId: string; traces: number; bytes: number; storedBytes: number }

export type TraceServiceDb = {
    /** The score `scoreId` names when `userId` owns it; another account's score is as absent as a missing one. */
    findOwnedScore: (params: { scoreId: string; userId: string }) => Promise<TraceScoreRow | null>
    findTraceIdByScoreId: (params: { scoreId: string; userId: string }) => Promise<string | null>
    /** Inserts the trace unless one for its score is there; false when it was. */
    insertTraceIfAbsent: (row: TraceInsertRow) => Promise<boolean>
    /** How many traces the account keeps and their JSON bytes. */
    sumUserTraces: (userId: string) => Promise<TraceUsage>
    saveAnalysis: (row: TraceAnalysisRow) => Promise<void>
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
    /** Traces with no analysis and with one older than `version`. */
    countAnalysesBehind: (version: number) => Promise<{ missing: number; stale: number }>
}

export type TraceServiceDeps = {
    db: TraceServiceDb
    newId: (prefix: string) => string
    now: () => number
    /** The chart's pattern by md5, for the pattern join only; it never leaves the server. */
    loadPattern: (md5: string) => Promise<ChartPatternDocument | null>
    /** Called when a stored trace could not be analysed; the trace stays stored and is analysed when it is next read. */
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

/** A stored summary of the current analysis version, which this build wrote and so reads as it is. */
const isCurrentSummary = (value: unknown): value is TraceAnalysisSummary => isRecord(value) && value.analysis_version === ANALYSIS_VERSION

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
            analysisVersion: ANALYSIS_VERSION,
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
                deps.db.countAnalysesBehind(ANALYSIS_VERSION),
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
                analysis: { analysis_version: ANALYSIS_VERSION, missing: analyses.missing, stale: analyses.stale },
            }
        },

        /**
         * One run's analysis for its own account, computed again from the stored trace when it is missing or of an older analysis
         * version. Null when the account keeps no such trace or the stored trace no longer reads as one.
         */
        readAnalysis: async (params: { traceId: string; userId: string }) => {
            const stored = await deps.db.findStoredTrace(params)
            if (!stored) return null
            const summary = stored.analysis?.analysisVersion === ANALYSIS_VERSION ? stored.analysis.summary : null
            if (isCurrentSummary(summary)) return summary
            const parsed = traceUploadSchema.safeParse(await gunzipJson(stored.data))
            if (!parsed.success) return null
            return analyze(parsed.data, { traceId: params.traceId, chartSha256: stored.chartSha256 })
        },
    }
}

export type TraceService = ReturnType<typeof createTraceService>

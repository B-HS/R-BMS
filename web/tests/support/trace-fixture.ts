import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { traceUploadSchema, type TraceUploadInput } from '@server/dto/trace'
import type { TraceScoreRow } from '@server/service/domain/trace/trace-check'
import {
    createTraceService,
    type TraceAnalysisRow,
    type TraceInsertRow,
    type TraceServiceDb,
    type TraceServiceDeps,
} from '@server/service/domain/trace/trace.service'

export const CONTRACT_DIR = join(import.meta.dirname, '..', '..', '..', 'contract', 'client-requests')

export const readContractTrace = (file: string) => traceUploadSchema.parse(JSON.parse(readFileSync(join(CONTRACT_DIR, file), 'utf8')))

export const TRACE_USER_ID = 'usr_trace'
export const OTHER_USER_ID = 'usr_other'

/** The score row a trace of `trace` is checked against: the counts the trace adds up to, as a client's score of the run says. */
export const scoreRowFor = (trace: TraceUploadInput, overrides: Partial<TraceScoreRow> = {}) =>
    ({
        id: trace.score_id,
        userId: TRACE_USER_ID,
        chartSha256: trace.chart.sha256,
        chartMd5: trace.chart.md5 || null,
        mode: trace.mode,
        playedAt: 1_791_970_074_354,
        pgreat: 0,
        great: 0,
        good: 0,
        bad: 0,
        poor: 0,
        epg: 0,
        lpg: 0,
        egr: 0,
        lgr: 0,
        egd: 0,
        lgd: 0,
        ebd: 0,
        lbd: 0,
        emptyPoor: 0,
        ...overrides,
    }) satisfies TraceScoreRow

/** An in-memory `play_trace` and `play_analysis` that behave like the compose implementation, the unique score id included. */
export const createMemoryTraceDb = (scores: readonly TraceScoreRow[], loginIds: Record<string, string> = {}) => {
    const traces = new Map<string, TraceInsertRow>()
    const analyses = new Map<string, TraceAnalysisRow>()
    const hooks: { beforeInsert: ((row: TraceInsertRow) => void) | null } = { beforeInsert: null }
    const usageOf = (userId: string) => {
        const owned = [...traces.values()].filter((row) => row.userId === userId)
        return { count: owned.length, bytes: owned.reduce((sum, row) => sum + row.byteSize, 0) }
    }
    const db: TraceServiceDb = {
        findOwnedScore: async ({ scoreId, userId }) => scores.find((row) => row.id === scoreId && row.userId === userId) ?? null,
        findTraceIdByScoreId: async ({ scoreId, userId }) =>
            [...traces.values()].find((row) => row.scoreId === scoreId && row.userId === userId)?.id ?? null,
        insertTraceIfAbsent: async (row) => {
            hooks.beforeInsert?.(row)
            if ([...traces.values()].some((stored) => stored.scoreId === row.scoreId)) return false
            traces.set(row.id, row)
            return true
        },
        sumUserTraces: async (userId) => usageOf(userId),
        saveAnalysis: async (row) => {
            analyses.set(row.traceId, row)
        },
        findStoredTrace: async ({ traceId, userId }) => {
            const row = traces.get(traceId)
            if (!row || row.userId !== userId) return null
            const analysis = analyses.get(traceId)
            return {
                data: row.data,
                chartSha256: row.chartSha256,
                analysis: analysis ? { analysisVersion: analysis.analysisVersion, summary: analysis.summary } : null,
            }
        },
        summarizeUsage: async (limit) => {
            const users = [...new Set([...traces.values()].map((row) => row.userId))]
            const rows = users.map((userId) => {
                const owned = [...traces.values()].filter((row) => row.userId === userId)
                return {
                    loginId: loginIds[userId] ?? userId,
                    traces: owned.length,
                    bytes: owned.reduce((sum, row) => sum + row.byteSize, 0),
                    storedBytes: owned.reduce((sum, row) => sum + row.storedSize, 0),
                }
            })
            const all = [...traces.values()]
            return {
                top: rows.toSorted((a, b) => b.bytes - a.bytes).slice(0, limit),
                totals: {
                    users: users.length,
                    traces: all.length,
                    bytes: all.reduce((sum, row) => sum + row.byteSize, 0),
                    storedBytes: all.reduce((sum, row) => sum + row.storedSize, 0),
                },
            }
        },
        countAccountsPast: async ({ near, over }) => {
            const users = [...new Set([...traces.values()].map((row) => row.userId))].map(usageOf)
            return {
                near: users.filter((usage) => usage.count >= near.count || usage.bytes >= near.bytes).length,
                over: users.filter((usage) => usage.count > over.count || usage.bytes > over.bytes).length,
            }
        },
        countAnalysesBehind: async (version) => {
            const all = [...traces.keys()]
            return {
                missing: all.filter((id) => !analyses.has(id)).length,
                stale: all.filter((id) => (analyses.get(id)?.analysisVersion ?? version) < version).length,
            }
        },
    }
    return { db, traces, analyses, hooks }
}

export const createMemoryTraceService = (memory: ReturnType<typeof createMemoryTraceDb>, overrides: Partial<Omit<TraceServiceDeps, 'db'>> = {}) => {
    const failures: unknown[] = []
    let nextId = 0
    const service = createTraceService({
        db: memory.db,
        newId: (prefix) => {
            nextId += 1
            return `${prefix}${nextId}`
        },
        now: () => 1_791_970_080_000,
        loadPattern: async () => null,
        reportAnalysisFailure: (error) => {
            failures.push(error)
        },
        ...overrides,
    })
    return { service, failures }
}

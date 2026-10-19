import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { traceUploadSchema, type TraceUploadInput } from '@server/dto/trace'
import type { TraceScoreRow } from '@server/service/domain/trace/trace-check'
import {
    createTraceService,
    type PlayShareSettingRow,
    type TraceAnalysisRow,
    type TracedPlayRow,
    type TracedPlayScore,
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
        lntype: trace.lntype,
        random: trace.random,
        scratchAuto: trace.auto_lanes.some(Boolean),
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

/** The score columns of a traced play when a test does not say: a cleared OFF run with no SCRATCH AUTO. */
export const DEFAULT_PLAY_SCORE: TracedPlayScore = {
    clear: 4,
    exScore: 0,
    maxExScore: 0,
    minbp: 0,
    gauge: 2,
    random: 'Off',
    scratchAuto: false,
    judgeRate: 100,
    assist: 0,
    autoplay: false,
    autoOffset: false,
    constant: false,
    greenNumber: null,
}

const patternVersionOf = (summary: Record<string, unknown>) =>
    typeof summary.pattern_analysis_version === 'number' ? summary.pattern_analysis_version : 0

const isBehindVersions = (analysis: StoredAnalysisRow, versions: { play: number; pattern: number }) =>
    analysis.analysisVersion < versions.play || patternVersionOf(analysis.summary) < versions.pattern

/** A `play_analysis` row as it is stored: a summary, or the marker of an analysis that failed at this version. */
export type StoredAnalysisRow = Omit<TraceAnalysisRow, 'summary' | 'patternJoin'> & { summary: Record<string, unknown>; patternJoin: string }

/**
 * An in-memory `play_trace`, `play_analysis`, `rival` and `play_share_setting` that behave like the compose implementation, the unique
 * score id included. `playScores` gives a traced play's score columns by score id.
 */
export const createMemoryTraceDb = (scores: readonly TraceScoreRow[], loginIds: Record<string, string> = {}) => {
    const traces = new Map<string, TraceInsertRow>()
    const analyses = new Map<string, StoredAnalysisRow>()
    const playScores = new Map<string, TracedPlayScore>()
    const rivals = new Set<string>()
    const shares = new Map<string, PlayShareSettingRow>()
    const playOf = (row: TraceInsertRow): TracedPlayRow => {
        const analysis = analyses.get(row.id)
        return {
            traceId: row.id,
            scoreId: row.scoreId,
            chartSha256: row.chartSha256,
            chartMd5: row.chartMd5,
            playedAt: row.playedAt,
            score: playScores.get(row.scoreId) ?? DEFAULT_PLAY_SCORE,
            analysis: analysis ? { analysisVersion: analysis.analysisVersion, summary: analysis.summary } : null,
        }
    }
    const playsOf = (userId: string) => [...traces.values()].filter((row) => row.userId === userId).map(playOf)
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
        saveAnalysisFailure: async (row) => {
            analyses.set(row.traceId, { ...row, patternJoin: 'failed', patternMd5: null })
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
        countAnalysesBehind: async (versions) => {
            const all = [...traces.keys()]
            return {
                missing: all.filter((id) => !analyses.has(id)).length,
                stale: all.filter((id) => {
                    const analysis = analyses.get(id)
                    return analysis !== undefined && isBehindVersions(analysis, versions)
                }).length,
            }
        },
        listTracesBehind: async ({ versions, after, limit }) =>
            [...traces.values()]
                .filter((row) => {
                    const analysis = analyses.get(row.id)
                    return (analysis === undefined || isBehindVersions(analysis, versions)) && (after === null || row.id > after)
                })
                .toSorted((a, b) => (a.id < b.id ? -1 : 1))
                .slice(0, limit)
                .map((row) => ({ id: row.id, userId: row.userId })),
        listRecentPlays: async ({ userId, limit }) =>
            playsOf(userId)
                .filter((play) => !play.score.autoplay)
                .toSorted((a, b) => b.playedAt - a.playedAt || (a.traceId < b.traceId ? 1 : -1))
                .slice(0, limit),
        listPlaysOfScores: async ({ userId, scoreIds }) => playsOf(userId).filter((play) => scoreIds.includes(play.scoreId)),
        findPlayOfScore: async ({ userId, scoreId }) => playsOf(userId).find((play) => play.scoreId === scoreId) ?? null,
        findBestPlayOnChart: async ({ userId, chartSha256, scratchAuto, excludeScoreId }) =>
            playsOf(userId)
                .filter(
                    (play) =>
                        play.chartSha256 === chartSha256 &&
                        play.score.scratchAuto === scratchAuto &&
                        !play.score.autoplay &&
                        play.scoreId !== excludeScoreId,
                )
                .toSorted((a, b) => b.score.exScore - a.score.exScore || b.playedAt - a.playedAt)[0] ?? null,
        isRivalOf: async ({ userId, rivalId }) => rivals.has(`${userId}>${rivalId}`),
        findShareSetting: async (userId) => shares.get(userId) ?? null,
        saveShareSetting: async (row) => {
            shares.set(row.userId, row)
        },
    }
    return { db, traces, analyses, hooks, playScores, rivals, shares }
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

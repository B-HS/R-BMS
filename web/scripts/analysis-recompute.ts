import { composeChartPattern } from '../src/server/compose/chart-pattern'
import { composeTrace } from '../src/server/compose/trace'
import type { ChartPatternService } from '../src/server/service/domain/chart-pattern/chart-pattern.service'
import type { TraceService } from '../src/server/service/domain/trace/trace.service'
import { parseRecomputeCommand, RECOMPUTE_COMMAND, type RecomputeCommand } from './dan-recompute'
import { createScriptDb, DATABASE_NOT_SET_MESSAGE } from './lib/script-db'
import { EXIT_CODE } from './seed-courses'

/** How many charts or play analyses one `recomputeStale` call computes again. A batch of this size has to finish in the two minutes the time limit leaves of the workflow's job limit. */
export const ANALYSIS_RECOMPUTE_BATCH = 100

/** No batch starts after this much time, so the script ends before the job limit of `.github/workflows/web-db.yml` (`timeout-minutes: 10`) and a run goes on where the last one stopped. */
export const ANALYSIS_RECOMPUTE_MAX_MS = 8 * 60 * 1000

export const USAGE = 'usage: bun scripts/analysis-recompute.ts <count|apply>'

type RecomputeBatch = Pick<Awaited<ReturnType<TraceService['recomputeStale']>>, 'examined' | 'recomputed' | 'skipped' | 'next_after' | 'remaining'>

/**
 * Computes again the chart features and the play analyses that are behind the current analysis version. `count` only counts them;
 * `apply` computes the chart features first, then the play analyses, in batches of `ANALYSIS_RECOMPUTE_BATCH` until a service has none
 * left. No batch starts once `ANALYSIS_RECOMPUTE_MAX_MS` has passed since the start: the script prints where it stopped and ends
 * with the success code, and a later `apply` goes on because what was computed is no longer behind. Only numbers are printed: the
 * Actions log is public, so neither the `login_id` that `adminUsage` carries nor the ids of the skipped items reach the log.
 */
export const recomputeAnalyses = async (params: {
    command: RecomputeCommand
    patterns: Pick<ChartPatternService, 'coverage' | 'recomputeStale'>
    traces: Pick<TraceService, 'adminUsage' | 'recomputeStale'>
    now: () => number
    log: (line: string) => void
}) => {
    const { command, patterns, traces, now, log } = params
    const startedAt = now()
    const [coverage, usage] = await Promise.all([patterns.coverage(), traces.adminUsage()])
    if (command === RECOMPUTE_COMMAND.COUNT) {
        log(`plan patternsStale ${coverage.stale_features}, analysesMissing ${usage.analysis.missing}, analysesStale ${usage.analysis.stale}`)
        return EXIT_CODE.OK
    }
    const progress = {
        patterns: { recomputed: 0, remaining: coverage.stale_features },
        analyses: { recomputed: 0, remaining: usage.analysis.missing + usage.analysis.stale },
        skipped: 0,
    }
    const walk = async (
        kind: 'patterns' | 'analyses',
        recomputeStale: (input: { limit: number; after: string | null }) => Promise<RecomputeBatch>,
    ) => {
        let after: string | null = null
        while (true) {
            const elapsedMs = now() - startedAt
            if (elapsedMs >= ANALYSIS_RECOMPUTE_MAX_MS) return elapsedMs
            const batch = await recomputeStale({ limit: ANALYSIS_RECOMPUTE_BATCH, after })
            progress[kind].recomputed += batch.recomputed
            progress[kind].remaining = batch.remaining
            progress.skipped += batch.skipped.length
            if (batch.examined === 0 || batch.next_after === null) return null
            after = batch.next_after
        }
    }
    const stoppedAfterMs = (await walk('patterns', patterns.recomputeStale)) ?? (await walk('analyses', traces.recomputeStale))
    const remaining = progress.patterns.remaining + progress.analyses.remaining
    log(
        `applied patterns ${progress.patterns.recomputed}, analyses ${progress.analyses.recomputed}, skipped ${progress.skipped}, remaining ${remaining}`,
    )
    if (stoppedAfterMs !== null) log(`stopped elapsedMs ${stoppedAfterMs}, remaining ${remaining}`)
    return EXIT_CODE.OK
}

/** What is printed for a failure: only the kind and code of the error, since a database error can name the server it could not reach. */
const describeFailure = (error: unknown) => {
    const code = typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : 'UNKNOWN'
    return `${error instanceof Error ? error.name : typeof error} ${code}`
}

const main = async () => {
    const command = parseRecomputeCommand(process.argv.slice(2))
    if (!command) {
        console.error(USAGE)
        return EXIT_CODE.USAGE
    }
    const scriptDb = createScriptDb()
    if (!scriptDb) {
        console.error(DATABASE_NOT_SET_MESSAGE)
        return EXIT_CODE.FAILED
    }
    const { db, close } = scriptDb
    try {
        const { chartPatternService } = composeChartPattern(db, { listCoverageTables: () => [] })
        const { traceService } = composeTrace(db, { chartPatternService })
        return await recomputeAnalyses({ command, patterns: chartPatternService, traces: traceService, now: Date.now, log: console.log })
    } finally {
        await close()
    }
}

if (import.meta.main) {
    main()
        .then((code) => {
            process.exitCode = code
        })
        .catch((error: unknown) => {
            console.error(`failed: ${describeFailure(error)}`)
            process.exitCode = EXIT_CODE.FAILED
        })
}

import { and, count, inArray, notInArray, or, type SQL } from 'drizzle-orm'
import type { MySqlTable } from 'drizzle-orm/mysql-core'
import {
    chart,
    chartBest,
    chartPattern,
    courseChart,
    playAnalysis,
    playTrace,
    replay,
    score,
    submissionAudit,
    tableChart,
    userFolderItem,
} from '../src/server/db/schema'
import { composeChartBlocklist } from '../src/server/compose/chart-blocklist'
import type { ChartBlocklistService } from '../src/server/service/domain/chart-blocklist/chart-blocklist.service'
import { createScriptDb, DATABASE_NOT_SET_MESSAGE } from './lib/script-db'

export const PURGE_COMMAND = { COUNT: 'count', DELETE: 'delete' } as const

export type PurgeCommand = (typeof PURGE_COMMAND)[keyof typeof PURGE_COMMAND]

export const USAGE = 'usage: bun scripts/chart-blocklist-purge.ts <count|delete>'

const EXIT_CODE = { OK: 0, FAILED: 1, USAGE: 2 } as const

export const parsePurgeCommand = (args: readonly string[]) => {
    const [command, ...rest] = args
    return rest.length === 0 ? Object.values(PURGE_COMMAND).find((candidate) => candidate === command) : undefined
}

type ScriptDb = NonNullable<ReturnType<typeof createScriptDb>>['db']
type Executor = ScriptDb | Parameters<Parameters<ScriptDb['transaction']>[0]>[0]

type PurgeTarget = { target: string; table: MySqlTable; where: SQL | undefined; isKept?: boolean }

/** The rows of the blocked charts, in the order they are deleted (dependents first). A chart a course names keeps its `chart` row. */
const purgeTargets = async (executor: Executor, hashes: { sha256s: string[]; md5s: string[] }) => {
    const { sha256s, md5s } = hashes
    const scoreIds = (
        await executor
            .select({ id: score.id })
            .from(score)
            .where(or(inArray(score.chartSha256, sha256s), inArray(score.chartMd5, md5s)))
    ).map((row) => row.id)
    const traceIds = (
        await executor
            .select({ id: playTrace.id })
            .from(playTrace)
            .where(or(inArray(playTrace.chartSha256, sha256s), inArray(playTrace.chartMd5, md5s), inArray(playTrace.scoreId, scoreIds)))
    ).map((row) => row.id)
    const courseChartSha256s = executor.select({ sha256: courseChart.chartSha256 }).from(courseChart)
    const targets: PurgeTarget[] = [
        { target: 'play_analysis', table: playAnalysis, where: or(inArray(playAnalysis.traceId, traceIds), inArray(playAnalysis.patternMd5, md5s)) },
        { target: 'play_trace', table: playTrace, where: inArray(playTrace.id, traceIds) },
        { target: 'replay', table: replay, where: or(inArray(replay.chartSha256, sha256s), inArray(replay.scoreId, scoreIds)) },
        { target: 'chart_best', table: chartBest, where: inArray(chartBest.chartSha256, sha256s) },
        { target: 'submission_audit', table: submissionAudit, where: inArray(submissionAudit.scoreId, scoreIds) },
        { target: 'score', table: score, where: inArray(score.id, scoreIds) },
        { target: 'chart_pattern', table: chartPattern, where: or(inArray(chartPattern.md5, md5s), inArray(chartPattern.sha256, sha256s)) },
        { target: 'table_chart', table: tableChart, where: inArray(tableChart.chartSha256, sha256s) },
        { target: 'user_folder_item', table: userFolderItem, where: or(inArray(userFolderItem.md5, md5s), inArray(userFolderItem.sha256, sha256s)) },
        { target: 'chart', table: chart, where: or(inArray(chart.sha256, sha256s), inArray(chart.md5, md5s)) },
        {
            target: 'course_chart_kept',
            table: courseChart,
            where: or(inArray(courseChart.chartSha256, sha256s), inArray(courseChart.md5, md5s)),
            isKept: true,
        },
    ]
    return { targets, courseChartSha256s }
}

const countTargets = async (executor: Executor, targets: PurgeTarget[]) =>
    Promise.all(
        targets.map(async ({ target, table, where }) => {
            const [row] = await executor.select({ value: count() }).from(table).where(where)
            return { target, rows: row?.value ?? 0 }
        }),
    )

/**
 * Counts (`count`) or deletes in one transaction (`delete`) every stored row of the charts of `chart_blocklist`, the way
 * `chart-purge.delete.sql` does for the charts removed at the owner's request. Answers the counts before and, for `delete`, after.
 */
export const purgeBlockedCharts = async (params: {
    command: PurgeCommand
    db: ScriptDb
    hashes: Awaited<ReturnType<ChartBlocklistService['listHashes']>>
}) => {
    const { command, db, hashes } = params
    if (command === PURGE_COMMAND.COUNT) {
        const { targets } = await purgeTargets(db, hashes)
        return { before: await countTargets(db, targets), after: null }
    }
    const before = await db.transaction(async (tx) => {
        const { targets, courseChartSha256s } = await purgeTargets(tx, hashes)
        const counted = await countTargets(tx, targets)
        for (const { table, where, target, isKept } of targets) {
            if (isKept) continue
            await tx.delete(table).where(target === 'chart' ? and(where, notInArray(chart.sha256, courseChartSha256s)) : where)
        }
        return counted
    })
    const { targets } = await purgeTargets(db, hashes)
    return { before, after: await countTargets(db, targets) }
}

const describeFailure = (error: unknown) => {
    const code = typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : 'UNKNOWN'
    return `${error instanceof Error ? error.name : typeof error} ${code}`
}

const main = async () => {
    const command = parsePurgeCommand(process.argv.slice(2))
    if (!command) {
        console.error(USAGE)
        return EXIT_CODE.USAGE
    }
    const scriptDb = createScriptDb()
    if (!scriptDb) {
        console.error(DATABASE_NOT_SET_MESSAGE)
        return EXIT_CODE.FAILED
    }
    try {
        const hashes = await composeChartBlocklist(scriptDb.db).chartBlocklistService.listHashes()
        console.log(`blocklist: ${hashes.sha256s.length} charts`)
        const { before, after } = await purgeBlockedCharts({ command, db: scriptDb.db, hashes })
        console.table(before)
        if (after) {
            console.log('after:')
            console.table(after)
        }
        return EXIT_CODE.OK
    } finally {
        await scriptDb.close()
    }
}

if (import.meta.main) {
    try {
        process.exitCode = await main()
    } catch (error: unknown) {
        console.error(`failed: ${describeFailure(error)}`)
        process.exitCode = EXIT_CODE.FAILED
    }
}

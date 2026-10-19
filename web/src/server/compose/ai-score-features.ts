import { and, asc, desc, eq, gte, inArray, isNull, lt, max, sql } from 'drizzle-orm'
import type { Database } from '@server/db'
import { aiGoal, aiPracticeFolder, chart, score } from '@server/db/schema'
import { aiGoalParamsSchema, aiScoreTimingExtraSchema } from '@server/dto/ai'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import { newId } from '@server/lib/token'
import { withLockRetry } from '@server/lib/with-lock-retry'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import { createAiGoalService, type AiGoalRow, type AiGoalServiceDb } from '@server/service/domain/ai/goal-plan.service'
import { createAiJudgeHabitService, type AiJudgeHabitServiceDb } from '@server/service/domain/ai/judge-habit.service'
import { createAiPracticeFolderService, type AiPracticeFolderServiceDb } from '@server/service/domain/ai/practice-folder.service'
import { createAiRecommendService, type TableFeatureDeps } from '@server/service/domain/ai/recommend.service'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import type { ChartTagLoad, ProgressTable } from '@server/service/domain/ai/table-progress'
import { createAiWeeklySummaryService, type AiWeeklySummaryServiceDb } from '@server/service/domain/ai/weekly-summary.service'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import type { FolderService } from '@server/service/domain/folder/folder.service'
import type { SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { orderedLevels, type TableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { exScoreDenominator } from '@server/compose/ai-record-query'

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)

const toProgressTable = (table: SnapshotTable) =>
    ({
        id: table.id,
        name: table.header.name,
        symbol: table.header.symbol,
        levels: orderedLevels(table),
        entries: table.entries,
    }) satisfies ProgressTable

const timingExtraOf = (extra: unknown) => {
    const parsed = aiScoreTimingExtraSchema.safeParse(extra)
    return {
        meanOffsetUs: parsed.success ? (parsed.data.mean_offset_us ?? null) : null,
        ruleVersion: parsed.success ? (parsed.data.rule_version ?? null) : null,
    }
}

const toGoalRow = (row: { id: string; kind: string; params: unknown; createdAt: number }) => {
    const parsed = aiGoalParamsSchema.safeParse({ ...(isRecord(row.params) ? row.params : {}), kind: row.kind })
    return parsed.success ? ({ id: row.id, params: parsed.data, createdAt: row.createdAt } satisfies AiGoalRow) : null
}

const goalSelection = { id: aiGoal.id, kind: aiGoal.kind, params: aiGoal.params, createdAt: aiGoal.createdAt }

export const createAiGoalDb = (db: Database) =>
    ({
        listActiveGoals: async (userId) => {
            const rows = await db
                .select(goalSelection)
                .from(aiGoal)
                .where(and(eq(aiGoal.userId, userId), isNull(aiGoal.archivedAt)))
                .orderBy(asc(aiGoal.createdAt), asc(aiGoal.id))
            return rows.flatMap((row) => toGoalRow(row) ?? [])
        },

        findActiveGoal: async ({ userId, goalId }) => {
            const [row] = await db
                .select(goalSelection)
                .from(aiGoal)
                .where(and(eq(aiGoal.id, goalId), eq(aiGoal.userId, userId), isNull(aiGoal.archivedAt)))
                .limit(1)
            return row ? toGoalRow(row) : null
        },

        insertGoal: async ({ id, userId, params, createdAt }) => {
            const { kind, ...rest } = params
            await db.insert(aiGoal).values({ id, userId, kind, params: rest, createdAt })
        },

        archiveGoal: async ({ userId, goalId, archivedAt }) => {
            const [result] = await db
                .update(aiGoal)
                .set({ archivedAt })
                .where(and(eq(aiGoal.id, goalId), eq(aiGoal.userId, userId), isNull(aiGoal.archivedAt)))
            return result.affectedRows > 0
        },
    }) satisfies AiGoalServiceDb

export const createAiPracticeFolderDb = (db: Database) =>
    ({
        findLinkedFolderId: async ({ userId, previewHash }) => {
            const [row] = await db
                .select({ folderId: aiPracticeFolder.folderId })
                .from(aiPracticeFolder)
                .where(and(eq(aiPracticeFolder.userId, userId), eq(aiPracticeFolder.previewHash, previewHash)))
                .limit(1)
            return row?.folderId ?? null
        },

        claimLink: withLockRetry(async ({ userId, previewHash, folderId, createdAt }) => {
            try {
                await db.insert(aiPracticeFolder).values({ userId, previewHash, folderId, createdAt })
                return folderId
            } catch (error) {
                if (!hasMysqlErrorCode(error, [MYSQL_ERROR_CODE.DUPLICATE_ENTRY])) throw error
            }
            const [existing] = await db
                .select({ folderId: aiPracticeFolder.folderId })
                .from(aiPracticeFolder)
                .where(and(eq(aiPracticeFolder.userId, userId), eq(aiPracticeFolder.previewHash, previewHash)))
                .limit(1)
            return existing?.folderId ?? folderId
        }),
    }) satisfies AiPracticeFolderServiceDb

export const createAiWeeklySummaryDb = (db: Database) =>
    ({
        listScoresBetween: async ({ userId, from, to }) => {
            const rows = await db
                .select({
                    chartSha256: score.chartSha256,
                    chartMd5: sql<string | null>`coalesce(${score.chartMd5}, ${chart.md5})`,
                    title: chart.title,
                    clear: score.clear,
                    exScore: score.exScore,
                    maxExScore: exScoreDenominator().mapWith(Number),
                    playedAt: score.playedAt,
                    fast: score.fast,
                    slow: score.slow,
                    pgreat: score.pgreat,
                    great: score.great,
                    good: score.good,
                    bad: score.bad,
                    scratchAuto: score.scratchAuto,
                    autoOffset: score.autoOffset,
                    extra: score.extra,
                })
                .from(score)
                .leftJoin(chart, eq(chart.sha256, score.chartSha256))
                .where(and(eq(score.userId, userId), gte(score.playedAt, from), lt(score.playedAt, to), eq(score.autoplay, false)))
                .orderBy(asc(score.playedAt), asc(score.id))
            return rows.map(({ pgreat, great, good, bad, extra, title, ...row }) => ({
                ...row,
                title: title ?? '',
                judgedNotes: pgreat + great + good + bad,
                ...timingExtraOf(extra),
            }))
        },

        listPriorBests: async ({ userId, chartSha256s, before }) => {
            if (chartSha256s.length === 0) return []
            const rows = await db
                .select({ chartSha256: score.chartSha256, scratchAuto: score.scratchAuto, clear: max(score.clear), exScore: max(score.exScore) })
                .from(score)
                .where(
                    and(
                        eq(score.userId, userId),
                        inArray(score.chartSha256, [...chartSha256s]),
                        lt(score.playedAt, before),
                        eq(score.autoplay, false),
                    ),
                )
                .groupBy(score.chartSha256, score.scratchAuto)
            return rows.map((row) => ({ ...row, clear: row.clear ?? 0, exScore: row.exScore ?? 0 }))
        },
    }) satisfies AiWeeklySummaryServiceDb

export const createAiJudgeHabitDb = (db: Database) =>
    ({
        listRecentTimingRows: async ({ userId, limit }) => {
            const rows = await db
                .select({
                    playedAt: score.playedAt,
                    pgreat: score.pgreat,
                    great: score.great,
                    good: score.good,
                    bad: score.bad,
                    fast: score.fast,
                    slow: score.slow,
                    egr: score.egr,
                    lgr: score.lgr,
                    egd: score.egd,
                    lgd: score.lgd,
                    ebd: score.ebd,
                    lbd: score.lbd,
                    avgjudge: score.avgjudge,
                    autoOffset: score.autoOffset,
                    greenNumber: score.greenNumber,
                    constant: score.constant,
                    exScore: score.exScore,
                    maxExScore: exScoreDenominator().mapWith(Number),
                    extra: score.extra,
                })
                .from(score)
                .where(and(eq(score.userId, userId), eq(score.autoplay, false)))
                .orderBy(desc(score.playedAt), desc(score.id))
                .limit(limit)
            return rows.map(({ extra, ...row }) => ({ ...row, ...timingExtraOf(extra) }))
        },
    }) satisfies AiJudgeHabitServiceDb

/** Each chart's tag shares from its pattern features (W3); a chart with no uploaded pattern is absent, and one read from older features is marked stale. */
export const createChartTagLoader = (chartPatternService: Pick<ChartPatternService, 'readFeaturesMany'>) => async (md5s: readonly string[]) => {
    const features = await chartPatternService.readFeaturesMany(md5s)
    return {
        shares: new Map(
            [...features].map(([md5, read]) => [
                md5,
                Object.fromEntries(Object.entries(read.features.tags).map(([tag, share]) => [tag, share.ratio])),
            ]),
        ),
        staleMd5s: new Set([...features].flatMap(([md5, read]) => (read.isStale ? [md5] : []))),
    } satisfies ChartTagLoad
}

type ComposeAiScoreFeaturesDeps = {
    aiFeatureRunner: Pick<AiFeatureRunner, 'run'>
    aiResultService: Pick<AiResultService, 'findLatest'>
    tableSnapshotService: Pick<TableSnapshotService, 'getRecords'>
    chartPatternService: Pick<ChartPatternService, 'readFeaturesMany'>
    folderService: Pick<FolderService, 'list' | 'get' | 'create' | 'replaceItems' | 'remove'>
    loadTables: () => Map<string, SnapshotTable>
}

export const composeAiScoreFeatures = (db: Database, deps: ComposeAiScoreFeaturesDeps) => {
    const now = () => Date.now()
    const tableDeps: TableFeatureDeps = {
        loadTable: (tableId) => {
            const tables = deps.loadTables()
            const table = tableId === undefined ? tables.values().next().value : tables.get(tableId)
            return table ? toProgressTable(table) : null
        },
        loadRecords: async ({ tableId, userId }) => (await deps.tableSnapshotService.getRecords({ tableId, userId }))?.records ?? null,
        loadChartTags: createChartTagLoader(deps.chartPatternService),
    }
    const levelLabelOf = (md5: string) => {
        for (const table of deps.loadTables().values()) {
            const entry = table.entries.find((candidate) => candidate.md5 === md5)
            if (entry) return `${table.header.symbol}${entry.level}`
        }
        return null
    }
    const shared = { runner: deps.aiFeatureRunner, results: deps.aiResultService }

    const aiRecommendService = createAiRecommendService({ ...tableDeps, ...shared })
    const aiGoalService = createAiGoalService({ ...tableDeps, ...shared, db: createAiGoalDb(db), newId, now })
    const aiPracticeFolderService = createAiPracticeFolderService({
        ...tableDeps,
        ...shared,
        db: createAiPracticeFolderDb(db),
        recommend: aiRecommendService,
        goals: aiGoalService,
        folders: deps.folderService,
        now,
    })
    const aiWeeklySummaryService = createAiWeeklySummaryService({ ...shared, db: createAiWeeklySummaryDb(db), levelLabelOf, now })
    const aiJudgeHabitService = createAiJudgeHabitService({ ...shared, db: createAiJudgeHabitDb(db) })

    return { aiRecommendService, aiGoalService, aiPracticeFolderService, aiWeeklySummaryService, aiJudgeHabitService }
}

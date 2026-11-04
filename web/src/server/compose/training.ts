import { randomBytes } from 'node:crypto'
import { and, count, desc, eq, inArray, max, sql } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { replay, score, trainingChart, trainingFolder } from '@server/db/schema'
import { TRAINING_LIMIT, trainingChartRequestSchema, trainingProfileSnapshotSchema } from '@server/dto/training'
import { summarizeError } from '@server/lib/error-summary'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import { withLockRetry } from '@server/lib/with-lock-retry'
import type { AiCredentialService } from '@server/service/domain/ai/credential.service'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import { readTableProgress } from '@server/service/domain/ai/table-progress'
import { RATIO_DECIMALS } from '@server/service/domain/analysis/analysis-constants'
import { roundToDecimals } from '@server/service/domain/analysis/play-cells'
import { createAiTrainingPlanService } from '@server/service/domain/ai/training-plan.service'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import type { FolderService } from '@server/service/domain/folder/folder.service'
import type { SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'
import type { TableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import type { TraceService } from '@server/service/domain/trace/trace.service'
import { trainingSpecSchema } from '@server/service/domain/training/spec'
import { TRAINING_ID_PREFIX } from '@server/service/domain/training/training-constants'
import { createTrainingChartService, type TrainingChartRow, type TrainingChartServiceDb } from '@server/service/domain/training/training.service'
import { exScoreDenominator } from '@server/compose/ai-record-query'
import { composeWeaknessProfile, createTableLoaders } from '@server/compose/ai-score-features'
import { createChartTraceCounter } from '@server/compose/trace'

/** Random bytes of a training chart id: 128 bits, so the public download path cannot be guessed (plan 2026-11-04 4.1). */
const TRAINING_ID_BYTES = 16

const chartSelection = {
    id: trainingChart.id,
    md5: trainingChart.md5,
    sha256: trainingChart.sha256,
    title: trainingChart.title,
    playLevel: trainingChart.playLevel,
    bpm: trainingChart.bpm,
    noteCount: trainingChart.noteCount,
    lengthMs: trainingChart.lengthMs,
    generatorVersion: trainingChart.generatorVersion,
    spec: trainingChart.spec,
    request: trainingChart.request,
    profileSnapshot: trainingChart.profileSnapshot,
    provider: trainingChart.provider,
    model: trainingChart.model,
    createdAt: trainingChart.createdAt,
}

/** A stored row with its three JSON columns read through their schemas: the server wrote them, so a row that does not read is an error. */
const toChartRow = (
    row: Omit<TrainingChartRow, 'spec' | 'request' | 'profileSnapshot'> & { spec: unknown; request: unknown; profileSnapshot: unknown },
) =>
    ({
        ...row,
        spec: trainingSpecSchema.parse(row.spec),
        request: trainingChartRequestSchema.parse(row.request),
        profileSnapshot: trainingProfileSnapshotSchema.parse(row.profileSnapshot),
    }) satisfies TrainingChartRow

const numberOrNull = (value: number | string | null | undefined) => (value === null || value === undefined ? null : Number(value))

export const createTrainingChartDb = (db: Database) => {
    const countTraces = createChartTraceCounter(db)
    const countOf = async (tx: Pick<Database, 'select'>, userId: string) => {
        const [row] = await tx.select({ value: count() }).from(trainingChart).where(eq(trainingChart.userId, userId))
        return row?.value ?? 0
    }

    return {
        countCharts: async (userId) => countOf(db, userId),

        listCharts: async (userId) => {
            const rows = await db
                .select(chartSelection)
                .from(trainingChart)
                .where(eq(trainingChart.userId, userId))
                .orderBy(desc(trainingChart.createdAt), desc(trainingChart.id))
            return rows.map(toChartRow)
        },

        findChart: async ({ id, userId }) => {
            const [row] = await db
                .select(chartSelection)
                .from(trainingChart)
                .where(and(eq(trainingChart.id, id), eq(trainingChart.userId, userId)))
                .limit(1)
            return row ? toChartRow(row) : null
        },

        insertChart: withLockRetry(async ({ userId, bms, ...row }) =>
            db.transaction(async (tx) => {
                await tx.select({ id: user.id }).from(user).where(eq(user.id, userId)).for('update')
                if ((await countOf(tx, userId)) >= TRAINING_LIMIT.CHARTS_PER_USER) return 'limit' as const
                await tx.insert(trainingChart).values({ ...row, userId, bms: Buffer.from(bms) })
                return 'inserted' as const
            }),
        ),

        deleteChart: withLockRetry(async ({ id, userId }) => {
            const [result] = await db.delete(trainingChart).where(and(eq(trainingChart.id, id), eq(trainingChart.userId, userId)))
            return result.affectedRows > 0
        }),

        findDownload: async (id) => {
            const [row] = await db
                .select({ id: trainingChart.id, bms: trainingChart.bms })
                .from(trainingChart)
                .where(eq(trainingChart.id, id))
                .limit(1)
            return row ? { id: row.id, bms: new Uint8Array(row.bms) } : null
        },

        findFolderLink: async (userId) => {
            const [row] = await db
                .select({ folderId: trainingFolder.folderId })
                .from(trainingFolder)
                .where(eq(trainingFolder.userId, userId))
                .limit(1)
            return row?.folderId ?? null
        },

        claimFolderLink: withLockRetry(async ({ userId, folderId, createdAt }) => {
            try {
                await db.insert(trainingFolder).values({ userId, folderId, createdAt })
                return folderId
            } catch (error) {
                if (!hasMysqlErrorCode(error, [MYSQL_ERROR_CODE.DUPLICATE_ENTRY])) throw error
            }
            const [existing] = await db
                .select({ folderId: trainingFolder.folderId })
                .from(trainingFolder)
                .where(eq(trainingFolder.userId, userId))
                .limit(1)
            return existing?.folderId ?? folderId
        }),

        listPlayStats: async ({ userId, sha256s }) => {
            if (sha256s.length === 0) return []
            const rows = await db
                .select({
                    sha256: score.chartSha256,
                    plays: count(),
                    bestExRatio: max(sql<number>`${score.exScore} / nullif(${exScoreDenominator()}, 0)`),
                    lastPlayedAt: max(score.playedAt),
                })
                .from(score)
                .where(and(eq(score.userId, userId), inArray(score.chartSha256, [...sha256s])))
                .groupBy(score.chartSha256)
            return rows.map((row) => ({
                sha256: row.sha256,
                plays: row.plays,
                bestExRatio: row.bestExRatio === null ? null : roundToDecimals(Number(row.bestExRatio), RATIO_DECIMALS),
                lastPlayedAt: numberOrNull(row.lastPlayedAt),
            }))
        },

        listPlays: async ({ userId, sha256, limit }) =>
            db
                .select({
                    scoreId: score.id,
                    playedAt: score.playedAt,
                    clear: score.clear,
                    exScore: score.exScore,
                    maxExScore: exScoreDenominator().mapWith(Number),
                    minbp: score.minbp,
                    random: score.random,
                })
                .from(score)
                .where(and(eq(score.userId, userId), eq(score.chartSha256, sha256)))
                .orderBy(desc(score.playedAt), desc(score.id))
                .limit(limit),

        countKept: async ({ userId, sha256 }) => {
            const [[scores], [replays], traces] = await Promise.all([
                db
                    .select({ value: count() })
                    .from(score)
                    .where(and(eq(score.userId, userId), eq(score.chartSha256, sha256))),
                db
                    .select({ value: count() })
                    .from(replay)
                    .where(and(eq(replay.userId, userId), eq(replay.chartSha256, sha256))),
                countTraces({ userId, chartSha256: sha256 }),
            ])
            return { scores: scores?.value ?? 0, traces, replays: replays?.value ?? 0 }
        },
    } satisfies TrainingChartServiceDb
}

type ComposeTrainingDeps = {
    aiFeatureRunner: Pick<AiFeatureRunner, 'run'>
    aiCredentialService: Pick<AiCredentialService, 'list'>
    folderService: FolderService
    chartPatternService: Pick<ChartPatternService, 'upload' | 'readFeaturesMany'>
    traceService: Pick<TraceService, 'analyzedPlaysOfScores' | 'analyzedPlaysInWindow'>
    tableSnapshotService: TableSnapshotService
    loadTables: () => Map<string, SnapshotTable>
}

export const composeTraining = (db: Database, deps: ComposeTrainingDeps) => {
    const tables = createTableLoaders(deps)
    const trainingChartService = createTrainingChartService({
        db: createTrainingChartDb(db),
        plans: createAiTrainingPlanService({ runner: deps.aiFeatureRunner }),
        profile: composeWeaknessProfile(db, deps.traceService),
        credentials: deps.aiCredentialService,
        folders: deps.folderService,
        patterns: deps.chartPatternService,
        traces: deps.traceService,
        frontierLevelOf: async (userId) => {
            const table = tables.loadTable(undefined)
            if (!table) return null
            const { frontier } = readTableProgress(table, (await tables.loadRecords({ tableId: table.id, userId })) ?? {})
            return frontier ? `${table.symbol}${frontier.level}` : null
        },
        newId: () => `${TRAINING_ID_PREFIX}${randomBytes(TRAINING_ID_BYTES).toString('hex')}`,
        now: () => Date.now(),
        reportFailure: (step, error) => console.error('training chart step failed', { step, ...summarizeError(error) }),
    })
    return { trainingChartService }
}

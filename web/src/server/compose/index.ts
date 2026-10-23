import 'server-only'
import { and, asc, between, count, eq, inArray, sql } from 'drizzle-orm'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { apiToken, chart, chartBest, clientBuild, rival, score, submissionAudit } from '@server/db/schema'
import { getAuth } from '@server/lib/auth'
import { generateApiToken, hashApiToken, newId } from '@server/lib/token'
import { withLockRetry } from '@server/lib/with-lock-retry'
import { createAuthService } from '@server/service/domain/auth/auth.service'
import { createChartService } from '@server/service/domain/chart/chart.service'
import { createPlayerService } from '@server/service/domain/player/player.service'
import { deriveAssistGrade } from '@server/service/domain/score/assist-grade'
import { createScoreService, type ScoreRankedRow, type ScoreRankingRow } from '@server/service/domain/score/score.service'
import { findSnapshotTextByMd5, getSnapshotTables } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { searchMd5sInTables } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { resolveBuildTrust } from '@server/service/shared/integrity/build-allowlist'
import { toChartRow } from '@server/compose/chart-row'
import { composeAdmin } from '@server/compose/admin'
import { composeAi } from '@server/compose/ai'
import { composeAiRecordQuery } from '@server/compose/ai-record-query'
import { composeAiPlayFeatures } from '@server/compose/ai-play-features'
import { composeAiScoreFeatures } from '@server/compose/ai-score-features'
import { composeChartPattern } from '@server/compose/chart-pattern'
import { composeCourse } from '@server/compose/course'
import { composeCourseCatalog } from '@server/compose/course-catalog'
import { composeDan } from '@server/compose/dan'
import { composeFe } from '@server/compose/fe'
import { composeReplay } from '@server/compose/replay'
import { composeRival } from '@server/compose/rival'
import { composeSetting } from '@server/compose/setting'
import { composeFolder } from '@server/compose/folder'
import { composeTable } from '@server/compose/table'
import { composeTableRefresh, tableCourseSyncOf } from '@server/compose/table-refresh'
import { composeTableSnapshot } from '@server/compose/table-snapshot'
import { composeTrace } from '@server/compose/trace'
import { chartBestScope, isBeatenBy, isRankedAheadOf, listOrder, rankInList } from '@server/compose/best-scope'
import { toJudgeBreakdown, type JudgeColumns } from '@server/compose/judge-row'

const judgeSelection = {
    pgreat: score.pgreat,
    great: score.great,
    good: score.good,
    bad: score.bad,
    poor: score.poor,
    miss: score.miss,
    fast: score.fast,
    slow: score.slow,
    combobreak: score.combobreak,
    epg: score.epg,
    lpg: score.lpg,
    egr: score.egr,
    lgr: score.lgr,
    egd: score.egd,
    lgd: score.lgd,
    ebd: score.ebd,
    lbd: score.lbd,
    epr: score.epr,
    lpr: score.lpr,
    ems: score.ems,
    lms: score.lms,
    avgjudge: score.avgjudge,
    emptyPoor: score.emptyPoor,
}

const rankingSelection = {
    scoreId: score.id,
    loginId: user.loginId,
    playerName: user.name,
    clear: score.clear,
    gauge: score.gauge,
    exScore: score.exScore,
    maxCombo: score.maxCombo,
    minbp: score.minbp,
    playedAt: score.playedAt,
    lntype: score.lntype,
    option: score.option,
    totalNotes: score.notes,
    replayId: score.replayId,
    scratchAuto: score.scratchAuto,
    ...judgeSelection,
}

type RankingSelectionRow = JudgeColumns & {
    scoreId: string
    loginId: string
    playerName: string
    clear: number
    gauge: number
    exScore: number
    maxCombo: number
    minbp: number
    playedAt: number
    lntype: number
    option: number
    totalNotes: number
    replayId: string | null
    scratchAuto: boolean
}

const toRankingRow = (row: RankingSelectionRow): ScoreRankingRow => ({
    scoreId: row.scoreId,
    loginId: row.loginId,
    playerName: row.playerName,
    clear: row.clear,
    assistGrade: deriveAssistGrade(row),
    exScore: row.exScore,
    maxCombo: row.maxCombo,
    minbp: row.minbp,
    playedAt: row.playedAt,
    lntype: row.lntype,
    option: row.option,
    totalNotes: row.totalNotes,
    replayId: row.replayId,
    scratchAuto: row.scratchAuto,
    judge: toJudgeBreakdown(row),
})

const composeAll = () => {
    const db = getDb()

    const chartService = createChartService({
        lookupTableText: findSnapshotTextByMd5,
        db: {
            findBySha256: async (sha256) => {
                const [row] = await db.select().from(chart).where(eq(chart.sha256, sha256)).limit(1)
                return row ? toChartRow(row) : null
            },
            findByMd5: async (md5) => {
                const [row] = await db.select().from(chart).where(eq(chart.md5, md5)).limit(1)
                return row ? toChartRow(row) : null
            },
            upsert: withLockRetry(async (row) => {
                const values = { ...row, extra: row.extra }
                await db
                    .insert(chart)
                    .values(values)
                    .onDuplicateKeyUpdate({ set: { ...values, sha256: sql`${chart.sha256}` } })
            }),
        },
    })

    const { danService } = composeDan(db)
    const { courseCatalogService } = composeCourseCatalog(db)

    const playerService = createPlayerService({
        listDisplayedDans: async (userId) => (await danService.listDisplayed([userId])).get(userId) ?? [],
        db: {
            findByLoginId: async (loginId) => {
                const [row] = await db.select().from(user).where(eq(user.loginId, loginId)).limit(1)
                return row
                    ? {
                          id: row.id,
                          loginId: row.loginId,
                          name: row.name,
                          rank: row.rank,
                          rankPoints: row.rankPoints,
                          totalPlays: row.totalPlays,
                          role: row.role,
                      }
                    : null
            },
            findById: async (id) => {
                const [row] = await db.select().from(user).where(eq(user.id, id)).limit(1)
                return row
                    ? {
                          id: row.id,
                          loginId: row.loginId,
                          name: row.name,
                          rank: row.rank,
                          rankPoints: row.rankPoints,
                          totalPlays: row.totalPlays,
                          role: row.role,
                      }
                    : null
            },
            countScores: async (userId) => {
                const [row] = await db.select({ value: count() }).from(score).where(eq(score.userId, userId))
                return row?.value ?? 0
            },
        },
    })

    const scoreService = createScoreService({
        newId,
        now: () => Date.now(),
        db: {
            findScoreByIdempotency: async ({ userId, chartSha256, playedAtFrom, playedAtTo }) => {
                const [row] = await db
                    .select({
                        id: score.id,
                        clear: score.clear,
                        gauge: score.gauge,
                        exScore: score.exScore,
                        minbp: score.minbp,
                        maxCombo: score.maxCombo,
                        ranked: score.ranked,
                        scratchAuto: score.scratchAuto,
                    })
                    .from(score)
                    .where(and(eq(score.userId, userId), eq(score.chartSha256, chartSha256), between(score.playedAt, playedAtFrom, playedAtTo)))
                    .orderBy(asc(score.playedAt))
                    .limit(1)
                return row ? { ...row, assistGrade: deriveAssistGrade(row) } : null
            },
            insertScore: withLockRetry(async (row) => {
                await db.insert(score).values(row)
            }),
            getChartBest: async ({ chartSha256, userId, scratchAuto }) => {
                const [row] = await db
                    .select({ clear: chartBest.clear, assistGrade: chartBest.assistGrade, exScore: chartBest.exScore, minbp: chartBest.minbp })
                    .from(chartBest)
                    .where(and(eq(chartBest.chartSha256, chartSha256), eq(chartBest.userId, userId), eq(chartBest.scratchAuto, scratchAuto)))
                    .limit(1)
                return row ?? null
            },
            upsertChartBestIfBetter: withLockRetry(async (row) => {
                const [inserted] = await db.insert(chartBest).ignore().values(row)
                if (inserted.affectedRows > 0) return true
                const [replaced] = await db
                    .update(chartBest)
                    .set({
                        scoreId: row.scoreId,
                        clear: row.clear,
                        assistGrade: row.assistGrade,
                        exScore: row.exScore,
                        minbp: row.minbp,
                        maxCombo: row.maxCombo,
                    })
                    .where(
                        and(
                            eq(chartBest.chartSha256, row.chartSha256),
                            eq(chartBest.userId, row.userId),
                            eq(chartBest.scratchAuto, row.scratchAuto),
                            isBeatenBy(chartBest, row),
                        ),
                    )
                return replaced.affectedRows > 0
            }),
            countBetterBests: async ({ chartSha256, clear, assistGrade, exScore, minbp, lnmode, includeScratchAuto }) => {
                const base = db.select({ value: count() }).from(chartBest).$dynamic()
                const [row] = await (lnmode === undefined ? base : base.innerJoin(score, eq(score.id, chartBest.scoreId))).where(
                    and(
                        eq(chartBest.chartSha256, chartSha256),
                        chartBestScope(db, includeScratchAuto),
                        lnmode === undefined ? undefined : eq(score.lntype, lnmode),
                        isRankedAheadOf(chartBest, { clear, assistGrade, exScore, minbp }),
                    ),
                )
                return row?.value ?? 0
            },
            getRanking: async ({ chartSha256, limit, offset, lnmode, userIds, includeScratchAuto }): Promise<ScoreRankedRow[]> => {
                const rows = await db
                    .select({ ...rankingSelection, rank: rankInList(chartBest) })
                    .from(chartBest)
                    .innerJoin(score, eq(score.id, chartBest.scoreId))
                    .innerJoin(user, eq(user.id, chartBest.userId))
                    .where(
                        and(
                            eq(chartBest.chartSha256, chartSha256),
                            chartBestScope(db, includeScratchAuto),
                            lnmode === undefined ? undefined : eq(score.lntype, lnmode),
                            userIds ? inArray(chartBest.userId, userIds) : undefined,
                        ),
                    )
                    .orderBy(...listOrder(chartBest))
                    .limit(limit)
                    .offset(offset)
                return rows.map((row) => ({ ...toRankingRow(row), rank: row.rank }))
            },
            getBestByLoginId: async ({ chartSha256, loginId, lnmode, includeScratchAuto }) => {
                const [row] = await db
                    .select(rankingSelection)
                    .from(chartBest)
                    .innerJoin(score, eq(score.id, chartBest.scoreId))
                    .innerJoin(user, eq(user.id, chartBest.userId))
                    .where(
                        and(
                            eq(chartBest.chartSha256, chartSha256),
                            chartBestScope(db, includeScratchAuto),
                            eq(user.loginId, loginId),
                            lnmode === undefined ? undefined : eq(score.lntype, lnmode),
                        ),
                    )
                    .limit(1)
                return row ? toRankingRow(row) : null
            },
            getScoreById: async (scoreId) => {
                const [row] = await db
                    .select({ ...rankingSelection, totalNotes: score.notes })
                    .from(score)
                    .innerJoin(user, eq(user.id, score.userId))
                    .where(eq(score.id, scoreId))
                    .limit(1)
                return row ? toRankingRow(row) : null
            },
            getRivalUserIds: async (loginId) => {
                const [owner] = await db.select({ id: user.id }).from(user).where(eq(user.loginId, loginId)).limit(1)
                if (!owner) return []
                const rows = await db.select({ rivalId: rival.rivalId }).from(rival).where(eq(rival.userId, owner.id))
                return [owner.id, ...rows.map((row) => row.rivalId)]
            },
            insertAudit: async (row) => {
                await db.insert(submissionAudit).values(row)
            },
        },
    })

    const authService = createAuthService({
        generateToken: generateApiToken,
        hashToken: hashApiToken,
        newId,
        now: () => Date.now(),
        provider: {
            signUpEmail: async (body) => {
                const result = await getAuth().api.signUpEmail({ body, returnHeaders: true })
                return { headers: result.headers, user: result.response?.user }
            },
            signInEmail: async (body) => {
                const result = await getAuth().api.signInEmail({ body, returnHeaders: true })
                return { headers: result.headers, user: result.response?.user }
            },
            getSessionUser: async (headers) => {
                const session = await getAuth().api.getSession({ headers })
                return session?.user
            },
        },
        db: {
            findUserByLoginId: async (loginId) => {
                const [row] = await db.select().from(user).where(eq(user.loginId, loginId)).limit(1)
                return row ? { id: row.id, email: row.email, name: row.name, loginId: row.loginId, role: row.role } : null
            },
            findUserById: async (id) => {
                const [row] = await db.select().from(user).where(eq(user.id, id)).limit(1)
                return row ? { id: row.id, name: row.name, loginId: row.loginId, role: row.role } : null
            },
            insertApiToken: async (row) => {
                await db.insert(apiToken).values(row)
            },
            findApiToken: async (tokenHash) => {
                const [row] = await db
                    .select({ id: apiToken.id, userId: apiToken.userId, lastUsedAt: apiToken.lastUsedAt })
                    .from(apiToken)
                    .where(and(eq(apiToken.tokenHash, tokenHash), eq(apiToken.revoked, false)))
                    .limit(1)
                return row ?? null
            },
            touchApiToken: async (id, at) => {
                await db.update(apiToken).set({ lastUsedAt: at }).where(eq(apiToken.id, id))
            },
            revokeTokensForUser: async (userId) => {
                await db.update(apiToken).set({ revoked: true }).where(eq(apiToken.userId, userId))
            },
            revokeTokenByHash: async (tokenHash) => {
                const [result] = await db
                    .update(apiToken)
                    .set({ revoked: true })
                    .where(and(eq(apiToken.tokenHash, tokenHash), eq(apiToken.revoked, false)))
                return result.affectedRows > 0
            },
        },
    })

    const buildAllowlist = {
        db: {
            findBuild: async (sha256: string) => {
                const [row] = await db.select({ trusted: clientBuild.trusted }).from(clientBuild).where(eq(clientBuild.sha256, sha256)).limit(1)
                return row ?? null
            },
        },
    }

    const { courseService } = composeCourse(db)
    const tableSnapshot = composeTableSnapshot(db)
    const ai = composeAi(db)
    const folder = composeFolder(db, { tableSnapshotService: tableSnapshot.tableSnapshotService, lookupTableText: findSnapshotTextByMd5 })
    const patterns = composeChartPattern(db, {
        listCoverageTables: () =>
            [...getSnapshotTables().values()].map((table) => ({
                id: table.id,
                name: table.header.name,
                md5s: [...new Set(table.entries.map((entry) => entry.md5))],
            })),
    })

    const trace = composeTrace(db, { chartPatternService: patterns.chartPatternService })

    return {
        chartService,
        playerService,
        scoreService,
        authService,
        courseService,
        danService,
        courseCatalogService,
        ...composeTableRefresh(db, { courses: tableCourseSyncOf(courseCatalogService), dan: danService }),
        ...composeReplay(db),
        ...composeSetting(db),
        ...composeRival(db),
        ...composeAdmin(db),
        ...composeFe(db, {
            lookupTableText: findSnapshotTextByMd5,
            searchTableMd5s: (query) => searchMd5sInTables(getSnapshotTables(), query),
            listDisplayedDans: danService.listDisplayed,
        }),
        ...composeTable(db, courseService),
        ...tableSnapshot,
        ...folder,
        ...ai,
        ...composeAiRecordQuery(db, { aiFeatureRunner: ai.aiFeatureRunner, loadTables: getSnapshotTables }),
        ...patterns,
        ...trace,
        ...composeAiScoreFeatures(db, {
            aiFeatureRunner: ai.aiFeatureRunner,
            aiResultService: ai.aiResultService,
            tableSnapshotService: tableSnapshot.tableSnapshotService,
            chartPatternService: patterns.chartPatternService,
            folderService: folder.folderService,
            traceService: trace.traceService,
            loadTables: getSnapshotTables,
        }),
        ...composeAiPlayFeatures(db, {
            aiFeatureRunner: ai.aiFeatureRunner,
            aiResultService: ai.aiResultService,
            traceService: trace.traceService,
            tableSnapshotService: tableSnapshot.tableSnapshotService,
            chartPatternService: patterns.chartPatternService,
            loadTables: getSnapshotTables,
        }),
        resolveBuildTrust: (sha256: string | null) => resolveBuildTrust(buildAllowlist, sha256),
    }
}

let composed: ReturnType<typeof composeAll> | null = null

export const compose = () => {
    if (composed) return composed
    composed = composeAll()
    return composed
}

export type Composed = ReturnType<typeof compose>

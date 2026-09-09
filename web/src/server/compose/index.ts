import 'server-only'
import { and, count, desc, eq, inArray, or, sql } from 'drizzle-orm'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { apiToken, chart, chartBest, clientBuild, rival, score, submissionAudit } from '@server/db/schema'
import { getAuth } from '@server/lib/auth'
import { generateApiToken, hashApiToken, newId } from '@server/lib/token'
import { createAuthService } from '@server/service/domain/auth/auth.service'
import { createChartService } from '@server/service/domain/chart/chart.service'
import { createPlayerService } from '@server/service/domain/player/player.service'
import { createScoreService, type ScoreRankingRow } from '@server/service/domain/score/score.service'
import { resolveBuildTrust } from '@server/service/shared/integrity/build-allowlist'
import { toChartRow } from '@server/compose/chart-row'
import { composeAdmin } from '@server/compose/admin'
import { composeCourse } from '@server/compose/course'
import { composeFe } from '@server/compose/fe'
import { composeReplay } from '@server/compose/replay'
import { composeRival } from '@server/compose/rival'
import { composeSetting } from '@server/compose/setting'
import { composeTable } from '@server/compose/table'
import type { JudgeBreakdownInput } from '@server/dto/common'

type JudgeColumns = {
    pgreat: number
    great: number
    good: number
    bad: number
    poor: number
    miss: number
    fast: number
    slow: number
    combobreak: number
    epg: number
    lpg: number
    egr: number
    lgr: number
    egd: number
    lgd: number
    ebd: number
    lbd: number
    epr: number
    lpr: number
    ems: number
    lms: number
    avgjudge: number
    emptyPoor: number
}

const toJudgeBreakdown = (row: JudgeColumns): JudgeBreakdownInput => ({
    pgreat: row.pgreat,
    great: row.great,
    good: row.good,
    bad: row.bad,
    poor: row.poor,
    miss: row.miss,
    fast: row.fast,
    slow: row.slow,
    combobreak: row.combobreak,
    epg: row.epg,
    lpg: row.lpg,
    egr: row.egr,
    lgr: row.lgr,
    egd: row.egd,
    lgd: row.lgd,
    ebd: row.ebd,
    lbd: row.lbd,
    epr: row.epr,
    lpr: row.lpr,
    ems: row.ems,
    lms: row.lms,
    avgjudge: row.avgjudge,
    empty_poor: row.emptyPoor,
})

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
    exScore: score.exScore,
    maxCombo: score.maxCombo,
    minbp: score.minbp,
    playedAt: score.playedAt,
    lntype: score.lntype,
    option: score.option,
    totalNotes: score.notes,
    ...judgeSelection,
}

type RankingSelectionRow = JudgeColumns & {
    scoreId: string
    loginId: string
    playerName: string
    clear: number
    exScore: number
    maxCombo: number
    minbp: number
    playedAt: number
    lntype: number
    option: number
    totalNotes: number
}

const toRankingRow = (row: RankingSelectionRow): ScoreRankingRow => ({
    scoreId: row.scoreId,
    loginId: row.loginId,
    playerName: row.playerName,
    clear: row.clear,
    exScore: row.exScore,
    maxCombo: row.maxCombo,
    minbp: row.minbp,
    playedAt: row.playedAt,
    lntype: row.lntype,
    option: row.option,
    totalNotes: row.totalNotes,
    judge: toJudgeBreakdown(row),
})

const composeAll = () => {
    const db = getDb()

    const chartService = createChartService({
        db: {
            findBySha256: async (sha256) => {
                const [row] = await db.select().from(chart).where(eq(chart.sha256, sha256)).limit(1)
                return row ? toChartRow(row) : null
            },
            findByMd5: async (md5) => {
                const [row] = await db.select().from(chart).where(eq(chart.md5, md5)).limit(1)
                return row ? toChartRow(row) : null
            },
            upsert: async (row) => {
                const values = { ...row, extra: row.extra }
                await db
                    .insert(chart)
                    .values(values)
                    .onDuplicateKeyUpdate({ set: { ...values, sha256: sql`${chart.sha256}` } })
            },
        },
    })

    const playerService = createPlayerService({
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
            findScoreIdByIdempotency: async ({ userId, chartSha256, playedAt }) => {
                const [row] = await db
                    .select({ id: score.id })
                    .from(score)
                    .where(and(eq(score.userId, userId), eq(score.chartSha256, chartSha256), eq(score.playedAt, playedAt)))
                    .limit(1)
                return row?.id ?? null
            },
            insertScore: async (row) => {
                await db.insert(score).values(row)
            },
            getChartBest: async ({ chartSha256, userId }) => {
                const [row] = await db
                    .select({ clear: chartBest.clear, exScore: chartBest.exScore, minbp: chartBest.minbp })
                    .from(chartBest)
                    .where(and(eq(chartBest.chartSha256, chartSha256), eq(chartBest.userId, userId)))
                    .limit(1)
                return row ?? null
            },
            upsertChartBest: async (row) => {
                await db
                    .insert(chartBest)
                    .values(row)
                    .onDuplicateKeyUpdate({
                        set: { scoreId: row.scoreId, clear: row.clear, exScore: row.exScore, minbp: row.minbp, maxCombo: row.maxCombo },
                    })
            },
            countBetterBests: async ({ chartSha256, clear, exScore }) => {
                const [row] = await db
                    .select({ value: count() })
                    .from(chartBest)
                    .where(
                        and(
                            eq(chartBest.chartSha256, chartSha256),
                            or(sql`${chartBest.clear} > ${clear}`, and(eq(chartBest.clear, clear), sql`${chartBest.exScore} > ${exScore}`)),
                        ),
                    )
                return row?.value ?? 0
            },
            getRanking: async ({ chartSha256, limit, offset, lnmode, userIds }) => {
                const rows = await db
                    .select(rankingSelection)
                    .from(chartBest)
                    .innerJoin(score, eq(score.id, chartBest.scoreId))
                    .innerJoin(user, eq(user.id, chartBest.userId))
                    .where(
                        and(
                            eq(chartBest.chartSha256, chartSha256),
                            lnmode === undefined ? undefined : eq(score.lntype, lnmode),
                            userIds ? inArray(chartBest.userId, userIds) : undefined,
                        ),
                    )
                    .orderBy(desc(chartBest.clear), desc(chartBest.exScore), chartBest.minbp)
                    .limit(limit)
                    .offset(offset)
                return rows.map(toRankingRow)
            },
            getBestByLoginId: async ({ chartSha256, loginId, lnmode }) => {
                const [row] = await db
                    .select(rankingSelection)
                    .from(chartBest)
                    .innerJoin(score, eq(score.id, chartBest.scoreId))
                    .innerJoin(user, eq(user.id, chartBest.userId))
                    .where(
                        and(
                            eq(chartBest.chartSha256, chartSha256),
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

    return {
        chartService,
        playerService,
        scoreService,
        authService,
        courseService,
        ...composeReplay(db),
        ...composeSetting(db),
        ...composeRival(db),
        ...composeAdmin(db),
        ...composeFe(db),
        ...composeTable(db, courseService),
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

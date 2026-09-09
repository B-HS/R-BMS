import { clearLampFromId, clearLampToId, gaugeTypeToId, type JudgeBreakdownInput } from '@server/dto/common'
import type { ScoreRecordOutput, ScoreSubmissionInput, SubmitResponseOutput } from '@server/dto/score'
import { deriveExScore, deriveMinbp, isBetterBest } from '@server/service/domain/score/best-policy'
import { BUILD_TRUST, rankedPolicy, type BuildTrust, type ScoreFlag } from '@server/service/domain/score/ranked-policy'

export const GUEST_PLAYER_ID = 'guest'

const PLAYED_AT_FUTURE_TOLERANCE_MS = 5 * 60 * 1000
const USER_AGENT_MAX_LENGTH = 255

export type ScoreRankingRow = {
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
    judge: JudgeBreakdownInput | null
}

export type ScoreInsertRow = ReturnType<typeof buildScoreRow>

export type ScoreServiceDb = {
    findScoreIdByIdempotency: (params: { userId: string; chartSha256: string; playedAt: number }) => Promise<string | null>
    insertScore: (row: ScoreInsertRow) => Promise<void>
    getChartBest: (params: { chartSha256: string; userId: string }) => Promise<{ clear: number; exScore: number; minbp: number } | null>
    upsertChartBest: (row: {
        chartSha256: string
        userId: string
        scoreId: string
        clear: number
        exScore: number
        minbp: number
        maxCombo: number
    }) => Promise<void>
    countBetterBests: (params: { chartSha256: string; clear: number; exScore: number }) => Promise<number>
    getRanking: (params: { chartSha256: string; limit: number; offset: number; lnmode?: number; userIds?: string[] }) => Promise<ScoreRankingRow[]>
    getBestByLoginId: (params: { chartSha256: string; loginId: string; lnmode?: number }) => Promise<ScoreRankingRow | null>
    getScoreById: (scoreId: string) => Promise<ScoreRankingRow | null>
    getRivalUserIds: (loginId: string) => Promise<string[]>
    insertAudit: (row: {
        id: string
        scoreId: string | null
        userId: string | null
        clientBuildSha256: string | null
        clientPlatform: string | null
        ip: string | null
        userAgent: string | null
        accepted: boolean
        ranked: boolean
        flags: ScoreFlag[]
    }) => Promise<void>
}

export type ScoreServiceDeps = {
    db: ScoreServiceDb
    newId: (prefix: string) => string
    now: () => number
}

export const buildScoreRow = (params: {
    id: string
    userId: string | null
    guestName: string | null
    chartSha256: string
    chartMd5: string | null
    input: ScoreSubmissionInput
    exScore: number
    minbp: number
    ranked: boolean
    flags: ScoreFlag[]
}) => {
    const { input } = params
    return {
        id: params.id,
        userId: params.userId,
        guestName: params.guestName,
        chartSha256: params.chartSha256,
        chartMd5: params.chartMd5,
        mode: input.mode,
        lntype: input.options.lntype,
        clear: clearLampToId(input.clear),
        epg: input.judge.epg,
        lpg: input.judge.lpg,
        egr: input.judge.egr,
        lgr: input.judge.lgr,
        egd: input.judge.egd,
        lgd: input.judge.lgd,
        ebd: input.judge.ebd,
        lbd: input.judge.lbd,
        epr: input.judge.epr,
        lpr: input.judge.lpr,
        ems: input.judge.ems,
        lms: input.judge.lms,
        pgreat: input.judge.pgreat,
        great: input.judge.great,
        good: input.judge.good,
        bad: input.judge.bad,
        poor: input.judge.poor,
        miss: input.judge.miss,
        fast: input.judge.fast,
        slow: input.judge.slow,
        combobreak: input.judge.combobreak,
        emptyPoor: input.judge.empty_poor,
        avgjudge: input.judge.avgjudge,
        exScore: params.exScore,
        maxExScore: input.max_ex_score,
        maxCombo: input.max_combo,
        notes: input.total_notes,
        passnotes: input.passnotes,
        minbp: params.minbp,
        gaugeValue: input.gauge_value,
        gauge: gaugeTypeToId(input.options.gauge),
        option: input.options.option,
        random: input.options.random,
        randomP2: input.options.random_p2 ?? null,
        scratchLeft: input.options.scratch_left,
        scratchAuto: input.options.scratch_auto,
        seed: input.seed,
        hispeed: input.options.hispeed,
        constant: input.options.constant,
        greenNumber: Math.round(input.options.green_number),
        lift: input.options.lift,
        laneCover: input.options.lane_cover,
        assist: input.options.assist.length,
        judgeRate: input.options.judge_rate,
        offsetMs: input.options.offset_ms,
        autoOffset: input.options.auto_offset,
        totalOverride: input.options.total_override,
        autoplay: input.options.autoplay,
        inputDevice: input.options.input_device,
        judgeAlgorithm: input.judge_algorithm,
        rule: input.rule,
        skin: input.skin,
        client: input.client,
        clientBuildSha256: input.client_build_sha256 ?? null,
        clientPlatform: input.client_platform ?? null,
        ranked: params.ranked,
        flags: params.flags,
        verified: false,
        replayId: input.replay_id ?? null,
        playedAt: input.played_at,
        extra: input.extra,
    }
}

export const toScoreRecord = (row: ScoreRankingRow, rank: number | null): ScoreRecordOutput => ({
    player: { id: row.loginId },
    player_name: row.playerName,
    clear: clearLampFromId(row.clear),
    ex_score: row.exScore,
    max_combo: row.maxCombo,
    minbp: row.minbp,
    rank,
    played_at: row.playedAt,
    lntype: row.lntype,
    option: row.option,
    total_notes: row.totalNotes,
    judge: row.judge,
    extra: {},
})

export const isPlayedAtPlausible = (playedAt: number, now: number) => playedAt - now <= PLAYED_AT_FUTURE_TOLERANCE_MS

export const judgedNoteCount = (judge: JudgeBreakdownInput) =>
    judge.pgreat + judge.great + judge.good + judge.bad + Math.max(judge.poor - judge.empty_poor, 0) + judge.miss

export const isJudgeCountConsistent = (judge: JudgeBreakdownInput, totalNotes: number) => totalNotes === 0 || judgedNoteCount(judge) <= totalNotes

const truncateUserAgent = (userAgent: string | null) => (userAgent === null ? null : userAgent.slice(0, USER_AGENT_MAX_LENGTH))

export const createScoreService = (deps: ScoreServiceDeps) => ({
    submit: async (params: {
        input: ScoreSubmissionInput
        chartSha256: string
        chartMd5: string | null
        user: { id: string; loginId: string; name: string } | null
        buildTrust: BuildTrust
        requireTrustedBuild: boolean
        ip: string | null
        userAgent: string | null
    }) => {
        const { input, user } = params
        const isGuest = !user
        const { ranked, flags } = rankedPolicy({
            options: input.options,
            buildTrust: params.buildTrust,
            isGuest,
            requireTrustedBuild: params.requireTrustedBuild,
        })
        const exScore = input.ex_score > 0 ? input.ex_score : deriveExScore(input.judge)
        const minbp = input.minbp > 0 ? input.minbp : deriveMinbp(input.judge)
        const clear = clearLampToId(input.clear)

        const existingId = user
            ? await deps.db.findScoreIdByIdempotency({ userId: user.id, chartSha256: params.chartSha256, playedAt: input.played_at })
            : null
        if (existingId) {
            const rank = user && ranked ? (await deps.db.countBetterBests({ chartSha256: params.chartSha256, clear, exScore })) + 1 : null
            const response: SubmitResponseOutput = {
                accepted: true,
                rank,
                previous_best: null,
                message: 'duplicate submission ignored',
                ranked,
                flags,
                is_new_best: false,
                score_id: existingId,
            }
            return { scoreId: existingId, duplicated: true, ranked, flags, response }
        }

        const scoreId = deps.newId('sc_')
        const previous = user ? await deps.db.getChartBest({ chartSha256: params.chartSha256, userId: user.id }) : null
        await deps.db.insertScore(
            buildScoreRow({
                id: scoreId,
                userId: user?.id ?? null,
                guestName: user ? null : GUEST_PLAYER_ID,
                chartSha256: params.chartSha256,
                chartMd5: params.chartMd5,
                input,
                exScore,
                minbp,
                ranked,
                flags,
            }),
        )

        const isNewBest = Boolean(user) && ranked && isBetterBest({ clear, exScore, minbp }, previous)
        if (user && isNewBest) {
            await deps.db.upsertChartBest({
                chartSha256: params.chartSha256,
                userId: user.id,
                scoreId,
                clear,
                exScore,
                minbp,
                maxCombo: input.max_combo,
            })
        }

        const rank = ranked && user ? (await deps.db.countBetterBests({ chartSha256: params.chartSha256, clear, exScore })) + 1 : null

        await deps.db
            .insertAudit({
                id: deps.newId('au_'),
                scoreId,
                userId: user?.id ?? null,
                clientBuildSha256: input.client_build_sha256 ?? null,
                clientPlatform: input.client_platform ?? null,
                ip: params.ip,
                userAgent: truncateUserAgent(params.userAgent),
                accepted: true,
                ranked,
                flags,
            })
            .catch(() => undefined)

        const response: SubmitResponseOutput = {
            accepted: true,
            rank,
            previous_best: previous?.exScore ?? null,
            message: ranked ? 'saved' : `recorded (unranked: ${flags.join(',').toLowerCase()})`,
            ranked,
            flags,
            is_new_best: isNewBest,
            score_id: scoreId,
        }

        return { scoreId, duplicated: false, ranked, flags, response }
    },

    recordRejection: async (params: {
        userId: string | null
        clientBuildSha256: string | null
        clientPlatform: string | null
        ip: string | null
        userAgent: string | null
        flags: ScoreFlag[]
    }) => {
        await deps.db.insertAudit({
            id: deps.newId('au_'),
            scoreId: null,
            userId: params.userId,
            clientBuildSha256: params.clientBuildSha256,
            clientPlatform: params.clientPlatform,
            ip: params.ip,
            userAgent: truncateUserAgent(params.userAgent),
            accepted: false,
            ranked: false,
            flags: params.flags,
        })
    },

    getRanking: async (params: { chartSha256: string; limit: number; page: number; lnmode?: number; rivalOf?: string }) => {
        const userIds = params.rivalOf ? await deps.db.getRivalUserIds(params.rivalOf) : undefined
        if (userIds && userIds.length === 0) return []
        const rows = await deps.db.getRanking({
            chartSha256: params.chartSha256,
            limit: params.limit,
            offset: (params.page - 1) * params.limit,
            lnmode: params.lnmode,
            userIds,
        })
        const base = (params.page - 1) * params.limit
        return rows.map((row, index) => toScoreRecord(row, base + index + 1))
    },

    getBest: async (params: { chartSha256: string; loginId: string; lnmode?: number }) => {
        const row = await deps.db.getBestByLoginId(params)
        if (!row) return null
        const rank = (await deps.db.countBetterBests({ chartSha256: params.chartSha256, clear: row.clear, exScore: row.exScore })) + 1
        return toScoreRecord(row, rank)
    },

    getById: async (scoreId: string) => {
        const row = await deps.db.getScoreById(scoreId)
        if (!row) return null
        return toScoreRecord(row, null)
    },
})

export const BUILD_TRUST_LEVELS = BUILD_TRUST

export type ScoreService = ReturnType<typeof createScoreService>

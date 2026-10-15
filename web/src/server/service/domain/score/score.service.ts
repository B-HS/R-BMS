import { clearLampFromId, clearLampToId, gaugeTypeToId, type ClearLamp, type JudgeBreakdownInput } from '@server/dto/common'
import type { ScoreRecordOutput, ScoreSubmissionInput, SubmitResponseOutput } from '@server/dto/score'
import {
    deriveExScore,
    deriveMinbp,
    effectiveJudgeCounts,
    EX_PER_PGREAT,
    isBetterBest,
    type EffectiveJudgeCounts,
} from '@server/service/domain/score/best-policy'
import { BUILD_TRUST, rankedPolicy, submissionMessage, type BuildTrust, type ScoreFlag } from '@server/service/domain/score/ranked-policy'

export const GUEST_PLAYER_ID = 'guest'

const PLAYED_AT_FUTURE_TOLERANCE_MS = 5 * 60 * 1000
const USER_AGENT_MAX_LENGTH = 255
const IDEMPOTENCY_WINDOW_MS = 1000
const JUDGE_BAD_KEEPS_NOTE_MODES: ReadonlySet<string> = new Set(['POPN_9K'])

export const DUPLICATE_SUBMISSION_MESSAGE = 'duplicate submission ignored'

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
    replayId?: string | null
    scratchAuto: boolean
    judge: JudgeBreakdownInput | null
}

export type ScoreRankedRow = ScoreRankingRow & { rank: number }

export type BestScope = { includeScratchAuto: boolean }

export type ScoreInsertRow = ReturnType<typeof buildScoreRow>

export type ScoreServiceDb = {
    findScoreIdByIdempotency: (params: { userId: string; chartSha256: string; playedAtFrom: number; playedAtTo: number }) => Promise<string | null>
    insertScore: (row: ScoreInsertRow) => Promise<void>
    getChartBest: (params: {
        chartSha256: string
        userId: string
        scratchAuto: boolean
    }) => Promise<{ clear: number; exScore: number; minbp: number } | null>
    /**
     * Makes this run the player's best only when it beats the stored one, as one atomic statement pair, and says whether it did.
     * Two runs racing for the same row can not leave the weaker one behind.
     */
    upsertChartBestIfBetter: (row: {
        chartSha256: string
        userId: string
        scratchAuto: boolean
        scoreId: string
        clear: number
        exScore: number
        minbp: number
        maxCombo: number
    }) => Promise<boolean>
    countBetterBests: (params: BestScope & { chartSha256: string; clear: number; exScore: number; minbp: number; lnmode?: number }) => Promise<number>
    getRanking: (
        params: BestScope & { chartSha256: string; limit: number; offset: number; lnmode?: number; userIds?: string[] },
    ) => Promise<ScoreRankedRow[]>
    getBestByLoginId: (params: BestScope & { chartSha256: string; loginId: string; lnmode?: number }) => Promise<ScoreRankingRow | null>
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
    scratchAuto: boolean
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
        scratchAuto: params.scratchAuto,
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
    extra: { ...(row.replayId ? { replay_id: row.replayId } : {}), ...(row.scratchAuto ? { scratch_auto: true } : {}) },
})

export const isPlayedAtPlausible = (playedAt: number, now: number) => playedAt - now <= PLAYED_AT_FUTURE_TOLERANCE_MS

const normalizeMode = (mode: string) => mode.trim().toUpperCase().replaceAll('-', '_')

/**
 * Whether a BAD takes its note in `mode`. In POPN_9K it leaves the note to be judged again (`JudgeProperty.PMS`
 * `judgeVanish`); every other mode consumes it.
 */
export const badTakesNote = (mode: string) => !JUDGE_BAD_KEEPS_NOTE_MODES.has(normalizeMode(mode))

/**
 * Counts only the judgements that consume a note: PGREAT, GREAT, GOOD, BAD when `badTakesItsNote`, and POOR (a missed note).
 * An empty POOR (`miss` on the wire) consumes nothing.
 */
export const countNotesTaken = (counts: EffectiveJudgeCounts, badTakesItsNote: boolean) =>
    counts.pgreat + counts.great + counts.good + (badTakesItsNote ? counts.bad : 0) + counts.poor

export const judgedNoteCount = (judge: JudgeBreakdownInput, mode: string) => countNotesTaken(effectiveJudgeCounts(judge), badTakesNote(mode))

/**
 * The judgements that took a note are no more than the notes the body says the chart has. A body that says 0 notes took
 * none: every client that sends judge counts sends the chart's note count with them.
 */
export const isJudgeCountConsistent = (judge: JudgeBreakdownInput, totalNotes: number, mode: string) => judgedNoteCount(judge, mode) <= totalNotes

/**
 * A claimed EX score has to be the one the judge counts add up to (two per PGREAT, one per GREAT). Zero means the client
 * sent none, and the server derives it from the counts.
 */
export const isExScoreConsistent = (judge: JudgeBreakdownInput, exScore: number) => exScore === 0 || exScore === deriveExScore(judge)

/** A best EX the body claims is no more than two per note it says the chart has. Zero means the client sent none. */
export const isMaxExScoreConsistent = (maxExScore: number, totalNotes: number) => maxExScore === 0 || maxExScore <= totalNotes * EX_PER_PGREAT

/**
 * A BP the body claims counts at least every BAD, POOR and empty POOR it judged; a run the gauge ended adds the notes it
 * never reached on top. Zero means the client sent none, and the server derives it from the counts.
 */
export const isMinbpConsistent = (judge: JudgeBreakdownInput, minbp: number) => minbp === 0 || minbp >= deriveMinbp(judge)

const FULL_COMBO_LAMPS: ReadonlySet<ClearLamp> = new Set(['FullCombo', 'Perfect', 'Max'])

/**
 * Whether the lamp can stand on these counts by the reference rule (`BMSPlayer.java:868`). A full combo has every note it
 * took inside one streak, so it has no POOR and, where a BAD takes its note, no BAD. PERFECT has no GOOD, and MAX no GREAT
 * either. An empty POOR (`miss`) takes no note and can come before the first note, so it never rules a lamp out, and nor
 * does a pop'n BAD for the same reason. Lamps below FULL COMBO are the gauge's and are not checked.
 */
export const isClearLampConsistent = (clear: ClearLamp, judge: JudgeBreakdownInput, mode: string) => {
    if (!FULL_COMBO_LAMPS.has(clear)) return true
    const counts = effectiveJudgeCounts(judge)
    if (counts.poor > 0 || (badTakesNote(mode) && counts.bad > 0)) return false
    if (clear === 'FullCombo') return true
    if (counts.good > 0) return false
    return clear === 'Perfect' || counts.great === 0
}

export type SubmissionIntegrityFailure = { field: string; reason: string }

/**
 * The first reason a score body can not be what a real play sends, or null. Only the body is read: the chart row the
 * server keeps was written by whoever submitted the chart first, so its note count bounds nothing.
 */
export const findSubmissionIntegrityFailure = (
    input: Pick<ScoreSubmissionInput, 'judge' | 'total_notes' | 'ex_score' | 'max_ex_score' | 'minbp' | 'clear' | 'mode'>,
): SubmissionIntegrityFailure | null => {
    if (!isExScoreConsistent(input.judge, input.ex_score)) return { field: 'ex_score', reason: 'ex_score does not match the judge counts' }
    if (!isJudgeCountConsistent(input.judge, input.total_notes, input.mode)) return { field: 'judge', reason: 'judge count exceeds total_notes' }
    if (!isMaxExScoreConsistent(input.max_ex_score, input.total_notes)) return { field: 'max_ex_score', reason: 'max_ex_score exceeds two per note' }
    if (!isMinbpConsistent(input.judge, input.minbp)) return { field: 'minbp', reason: 'minbp is below the BAD, POOR and empty POOR count' }
    if (!isClearLampConsistent(input.clear, input.judge, input.mode))
        return { field: 'clear', reason: 'clear lamp is not reachable with these judge counts' }
    return null
}

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
        const { ranked, flags, scratchAuto } = rankedPolicy({
            options: input.options,
            buildTrust: params.buildTrust,
            isGuest,
            requireTrustedBuild: params.requireTrustedBuild,
        })
        const exScore = input.ex_score > 0 ? input.ex_score : deriveExScore(input.judge)
        const minbp = input.minbp > 0 ? input.minbp : deriveMinbp(input.judge)
        const clear = clearLampToId(input.clear)
        const countRank = async () =>
            (await deps.db.countBetterBests({ chartSha256: params.chartSha256, clear, exScore, minbp, includeScratchAuto: scratchAuto })) + 1

        const playedAtFrom = Math.floor(input.played_at / IDEMPOTENCY_WINDOW_MS) * IDEMPOTENCY_WINDOW_MS
        const existingId = user
            ? await deps.db.findScoreIdByIdempotency({
                  userId: user.id,
                  chartSha256: params.chartSha256,
                  playedAtFrom,
                  playedAtTo: playedAtFrom + IDEMPOTENCY_WINDOW_MS - 1,
              })
            : null
        if (existingId) {
            const rank = user && ranked ? await countRank() : null
            const response: SubmitResponseOutput = {
                accepted: true,
                rank,
                previous_best: null,
                message: DUPLICATE_SUBMISSION_MESSAGE,
                ranked,
                flags,
                is_new_best: false,
                score_id: existingId,
            }
            return { scoreId: existingId, duplicated: true, ranked, flags, response }
        }

        const scoreId = deps.newId('sc_')
        const previous = user ? await deps.db.getChartBest({ chartSha256: params.chartSha256, userId: user.id, scratchAuto }) : null
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
                scratchAuto,
                flags,
            }),
        )

        const isNewBest =
            user !== null &&
            ranked &&
            isBetterBest({ clear, exScore, minbp }, previous) &&
            (await deps.db.upsertChartBestIfBetter({
                chartSha256: params.chartSha256,
                userId: user.id,
                scratchAuto,
                scoreId,
                clear,
                exScore,
                minbp,
                maxCombo: input.max_combo,
            }))

        const rank = ranked && user ? await countRank() : null

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
            message: submissionMessage(ranked, flags),
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

    recordAcceptance: async (params: {
        scoreId: string
        userId: string
        clientBuildSha256: string | null
        clientPlatform: string | null
        ip: string | null
        userAgent: string | null
        ranked: boolean
        flags: ScoreFlag[]
    }) => {
        await deps.db.insertAudit({
            id: deps.newId('au_'),
            scoreId: params.scoreId,
            userId: params.userId,
            clientBuildSha256: params.clientBuildSha256,
            clientPlatform: params.clientPlatform,
            ip: params.ip,
            userAgent: truncateUserAgent(params.userAgent),
            accepted: true,
            ranked: params.ranked,
            flags: params.flags,
        })
    },

    getRanking: async (params: BestScope & { chartSha256: string; limit: number; page: number; lnmode?: number; rivalOf?: string }) => {
        const userIds = params.rivalOf ? await deps.db.getRivalUserIds(params.rivalOf) : undefined
        if (userIds && userIds.length === 0) return []
        const rows = await deps.db.getRanking({
            chartSha256: params.chartSha256,
            limit: params.limit,
            offset: (params.page - 1) * params.limit,
            lnmode: params.lnmode,
            userIds,
            includeScratchAuto: params.includeScratchAuto,
        })
        return rows.map((row) => toScoreRecord(row, row.rank))
    },

    getBest: async (params: BestScope & { chartSha256: string; loginId: string; lnmode?: number }) => {
        const row = await deps.db.getBestByLoginId(params)
        if (!row) return null
        const rank =
            (await deps.db.countBetterBests({
                chartSha256: params.chartSha256,
                clear: row.clear,
                exScore: row.exScore,
                minbp: row.minbp,
                lnmode: params.lnmode,
                includeScratchAuto: params.includeScratchAuto,
            })) + 1
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

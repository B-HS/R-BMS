import { clearLampToId } from '@server/dto/common'
import type { CourseMetaInput, CourseSubmissionInput } from '@server/dto/course'
import type { SubmitResponseOutput } from '@server/dto/score'
import { deriveExScore, deriveMinbp, isBetterBest } from '@server/service/domain/score/best-policy'
import {
    assistFlags,
    buildTrustFlags,
    isBuildBlockingRank,
    isScratchAutoPlay,
    rankBlockingFlags,
    submissionMessage,
    type BuildTrust,
    type ScoreFlag,
} from '@server/service/domain/score/ranked-policy'
import {
    isExScoreConsistent,
    isJudgeCountConsistent,
    LONG_NOTE_COUNT_MAX_FACTOR,
    toScoreRecord,
    type BestScope,
    type ScoreRankedRow,
    type ScoreRankingRow,
    type SubmissionIntegrityFailure,
} from '@server/service/domain/score/score.service'

export type CourseChartRow = {
    position: number
    chartSha256: string
    md5: string | null
}

export type CourseRow = {
    courseHash: string
    name: string
    lntype: number
    constraint: unknown
    trophy: unknown
    extra: unknown
}

export type CourseScoreInsertRow = {
    id: string
    userId: string | null
    courseHash: string
    clear: number
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
    exScore: number
    maxCombo: number
    gaugeValue: number
    minbp: number
    trophy: string | null
    scratchAuto: boolean
    ranked: boolean
    playedAt: number
    extra: Record<string, unknown>
}

export type CourseServiceDb = {
    findCourse: (courseHash: string) => Promise<CourseRow | null>
    findCourseCharts: (courseHash: string) => Promise<CourseChartRow[]>
    upsertCourse: (row: {
        courseHash: string
        name: string
        lntype: number
        constraint: string[]
        trophy: unknown[]
        extra: Record<string, unknown>
    }) => Promise<void>
    replaceCourseCharts: (courseHash: string, charts: { position: number; chartSha256: string }[]) => Promise<void>
    findCourseScoreIdByIdempotency: (params: { userId: string; courseHash: string; playedAt: number }) => Promise<string | null>
    insertCourseScore: (row: CourseScoreInsertRow) => Promise<void>
    getCourseBest: (params: {
        courseHash: string
        userId: string
        scratchAuto: boolean
    }) => Promise<{ clear: number; exScore: number; minbp: number } | null>
    upsertCourseBestIfBetter: (row: {
        courseHash: string
        userId: string
        scratchAuto: boolean
        courseScoreId: string
        clear: number
        exScore: number
        minbp: number
        maxCombo: number
    }) => Promise<boolean>
    countBetterCourseBests: (params: BestScope & { courseHash: string; clear: number; exScore: number; minbp: number }) => Promise<number>
    getCourseRanking: (params: BestScope & { courseHash: string; limit: number; offset: number; userIds?: string[] }) => Promise<ScoreRankedRow[]>
    getCourseBestByLoginId: (params: BestScope & { courseHash: string; loginId: string }) => Promise<ScoreRankingRow | null>
    getRivalUserIds: (loginId: string) => Promise<string[]>
}

export type CourseServiceDeps = {
    db: CourseServiceDb
    newId: (prefix: string) => string
}

export const toCourseMeta = (row: CourseRow, charts: CourseChartRow[]): CourseMetaInput => ({
    course_hash: row.courseHash,
    name: row.name,
    lntype: row.lntype,
    charts: charts.map((chart) => ({ md5: chart.md5 ?? '', sha256: chart.chartSha256 })),
    constraint: (row.constraint as string[]) ?? [],
    trophy: (row.trophy as CourseMetaInput['trophy']) ?? [],
    extra: (row.extra as Record<string, unknown>) ?? {},
})

const COURSE_JUDGE_MODE = ''

/**
 * The first reason a course body can not be what a real run sends, or null. `knownCourseNotes` is the sum of the notes the
 * server holds for every chart of the course, 0 when any of them is unknown, which leaves the judge counts without a bound.
 */
export const findCourseIntegrityFailure = (
    input: Pick<CourseSubmissionInput, 'judge' | 'ex_score'>,
    knownCourseNotes: number,
): SubmissionIntegrityFailure | null => {
    if (!isExScoreConsistent(input.judge, input.ex_score)) return { field: 'ex_score', reason: 'ex_score does not match the judge counts' }
    if (!isJudgeCountConsistent(input.judge, knownCourseNotes * LONG_NOTE_COUNT_MAX_FACTOR, COURSE_JUDGE_MODE))
        return { field: 'judge', reason: 'judge count exceeds the notes of the course' }
    return null
}

export const buildCourseScoreRow = (params: {
    id: string
    userId: string | null
    input: CourseSubmissionInput
    exScore: number
    minbp: number
    ranked: boolean
    scratchAuto: boolean
    flags: ScoreFlag[]
}): CourseScoreInsertRow => {
    const { input } = params
    return {
        id: params.id,
        userId: params.userId,
        courseHash: input.course_hash,
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
        exScore: params.exScore,
        maxCombo: input.max_combo,
        gaugeValue: input.gauge_value,
        minbp: params.minbp,
        trophy: input.trophy ?? null,
        scratchAuto: params.scratchAuto,
        ranked: params.ranked,
        playedAt: input.played_at,
        extra:
            params.flags.length > 0 ? { ...input.extra, flags: params.flags, scratch_auto: input.scratch_auto, assist: input.assist } : input.extra,
    }
}

export const createCourseService = (deps: CourseServiceDeps) => ({
    exists: async (courseHash: string) => (await deps.db.findCourse(courseHash)) !== null,

    getMeta: async (courseHash: string) => {
        const row = await deps.db.findCourse(courseHash)
        if (!row) return null
        const charts = await deps.db.findCourseCharts(courseHash)
        return toCourseMeta(row, charts)
    },

    upsertMeta: async (meta: CourseMetaInput) => {
        await deps.db.upsertCourse({
            courseHash: meta.course_hash,
            name: meta.name,
            lntype: meta.lntype,
            constraint: meta.constraint,
            trophy: meta.trophy,
            extra: meta.extra,
        })
        const charts = meta.charts
            .map((chart, index) => ({ position: index, chartSha256: chart.sha256 }))
            .filter((chart) => chart.chartSha256.length > 0)
        await deps.db.replaceCourseCharts(meta.course_hash, charts)
        const row = await deps.db.findCourse(meta.course_hash)
        return row ? toCourseMeta(row, await deps.db.findCourseCharts(meta.course_hash)) : null
    },

    submit: async (params: {
        input: CourseSubmissionInput
        user: { id: string; loginId: string }
        buildTrust: BuildTrust
        requireTrustedBuild: boolean
    }) => {
        const { input, user } = params
        const exScore = input.ex_score > 0 ? input.ex_score : deriveExScore(input.judge)
        const minbp = input.minbp > 0 ? input.minbp : deriveMinbp(input.judge)
        const clear = clearLampToId(input.clear)
        const flags = [...assistFlags(input), ...buildTrustFlags(params.buildTrust)]
        const ranked = rankBlockingFlags(flags).length === 0 && !isBuildBlockingRank(params.buildTrust, params.requireTrustedBuild)
        const scratchAuto = isScratchAutoPlay(input)

        const existingId = await deps.db.findCourseScoreIdByIdempotency({
            userId: user.id,
            courseHash: input.course_hash,
            playedAt: input.played_at,
        })
        if (existingId) {
            const response: SubmitResponseOutput = {
                accepted: true,
                rank: null,
                previous_best: null,
                message: 'duplicate submission ignored',
                ranked,
                flags,
                is_new_best: false,
                score_id: existingId,
            }
            return { courseScoreId: existingId, duplicated: true, ranked, flags, response }
        }

        const courseScoreId = deps.newId('cs_')
        const previous = await deps.db.getCourseBest({ courseHash: input.course_hash, userId: user.id, scratchAuto })
        await deps.db.insertCourseScore(
            buildCourseScoreRow({ id: courseScoreId, userId: user.id, input, exScore, minbp, ranked, scratchAuto, flags }),
        )

        const isNewBest =
            ranked &&
            isBetterBest({ clear, exScore, minbp }, previous) &&
            (await deps.db.upsertCourseBestIfBetter({
                courseHash: input.course_hash,
                userId: user.id,
                scratchAuto,
                courseScoreId,
                clear,
                exScore,
                minbp,
                maxCombo: input.max_combo,
            }))

        const rank = ranked
            ? (await deps.db.countBetterCourseBests({ courseHash: input.course_hash, clear, exScore, minbp, includeScratchAuto: scratchAuto })) + 1
            : null

        const response: SubmitResponseOutput = {
            accepted: true,
            rank,
            previous_best: previous?.exScore ?? null,
            message: submissionMessage(ranked, flags),
            ranked,
            flags,
            is_new_best: isNewBest,
            score_id: courseScoreId,
        }

        return { courseScoreId, duplicated: false, ranked, flags, response }
    },

    getRanking: async (params: BestScope & { courseHash: string; limit: number; page: number; rivalOf?: string }) => {
        const userIds = params.rivalOf ? await deps.db.getRivalUserIds(params.rivalOf) : undefined
        if (userIds && userIds.length === 0) return []
        const rows = await deps.db.getCourseRanking({
            courseHash: params.courseHash,
            limit: params.limit,
            offset: (params.page - 1) * params.limit,
            userIds,
            includeScratchAuto: params.includeScratchAuto,
        })
        return rows.map((row) => toScoreRecord(row, row.rank))
    },

    getBest: async (params: BestScope & { courseHash: string; loginId: string }) => {
        const row = await deps.db.getCourseBestByLoginId(params)
        if (!row) return null
        const rank =
            (await deps.db.countBetterCourseBests({
                courseHash: params.courseHash,
                clear: row.clear,
                exScore: row.exScore,
                minbp: row.minbp,
                includeScratchAuto: params.includeScratchAuto,
            })) + 1
        return toScoreRecord(row, rank)
    },
})

export type CourseService = ReturnType<typeof createCourseService>

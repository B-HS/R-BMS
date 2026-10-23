import { clearLampToId, gaugeTypeToId, NO_ASSIST_GRADE, type AssistGrade } from '@server/dto/common'
import type { CourseGaugeRule, CourseMetaInput, CourseSubmissionInput, StoredCourseStage } from '@server/dto/course'
import type { SubmitResponseOutput } from '@server/dto/score'
import { deriveAssistGrade } from '@server/service/domain/score/assist-grade'
import { deriveExScore, deriveMinbp, effectiveJudgeCounts, EX_PER_PGREAT, isBetterBest } from '@server/service/domain/score/best-policy'
import {
    assistFlags,
    buildTrustFlags,
    isBuildBlockingRank,
    isScratchAutoPlay,
    rankBlockingFlags,
    SCORE_FLAG,
    submissionMessage,
    type BuildTrust,
    type ScoreFlag,
} from '@server/service/domain/score/ranked-policy'
import {
    areDerivedCountsStorable,
    countNotesTaken,
    DERIVED_COUNTS_FAILURE,
    DUPLICATE_SUBMISSION_MESSAGE,
    isExScoreConsistent,
    isMinbpConsistent,
    toScoreRecord,
    type BestScope,
    type ScoreRankedRow,
    type ScoreRankingRow,
    type StoredRunForBest,
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
    pgreat: number
    great: number
    good: number
    bad: number
    poor: number
    miss: number
    fast: number
    slow: number
    combobreak: number
    emptyPoor: number
    exScore: number
    maxExScore: number
    maxCombo: number
    gaugeValue: number
    minbp: number
    trophy: string | null
    scratchAuto: boolean
    ranked: boolean
    playedAt: number
    gauge: number | null
    courseGauge: CourseGaugeRule | null
    stages: StoredCourseStage[] | null
    extra: Record<string, unknown>
}

export type StoredCourseRun = StoredRunForBest &
    Pick<CourseScoreInsertRow, 'gauge' | 'courseGauge' | 'gaugeValue' | 'stages' | 'playedAt'> & { extra: unknown }

export type SubmittedCourseRun = Pick<
    CourseScoreInsertRow,
    'id' | 'userId' | 'courseHash' | 'clear' | 'ranked' | 'scratchAuto' | 'gauge' | 'courseGauge' | 'gaugeValue' | 'stages' | 'playedAt'
> & { flags: ScoreFlag[] }

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
    replaceCourseCharts: (courseHash: string, charts: { position: number; chartSha256: string; md5: string }[]) => Promise<void>
    findCourseScoreByIdempotency: (params: { userId: string; courseHash: string; playedAt: number }) => Promise<StoredCourseRun | null>
    insertCourseScore: (row: CourseScoreInsertRow) => Promise<void>
    getCourseBest: (params: {
        courseHash: string
        userId: string
        scratchAuto: boolean
    }) => Promise<{ clear: number; assistGrade: AssistGrade; exScore: number; minbp: number } | null>
    upsertCourseBestIfBetter: (row: {
        courseHash: string
        userId: string
        scratchAuto: boolean
        courseScoreId: string
        clear: number
        assistGrade: AssistGrade
        exScore: number
        minbp: number
        maxCombo: number
    }) => Promise<boolean>
    countBetterCourseBests: (
        params: BestScope & { courseHash: string; clear: number; assistGrade: AssistGrade; exScore: number; minbp: number },
    ) => Promise<number>
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

/**
 * A course runs charts of any mode and its body names none, so a BAD is never counted as taking a note: a pop'n BAD
 * leaves its note to be judged again.
 */
const COURSE_BAD_TAKES_NOTE = false

/** The assist grade of a course run: none when the body names no gauge (a client from before 2026-10-23). */
export const courseAssistGrade = (run: { clear: number; gauge: number | null }): AssistGrade =>
    run.gauge === null ? NO_ASSIST_GRADE : deriveAssistGrade({ clear: run.clear, gauge: run.gauge })

/** The stages a body played, each with the chart the course lists at that place. */
export const toStoredStages = (input: Pick<CourseSubmissionInput, 'stages' | 'charts'>): StoredCourseStage[] | null =>
    input.stages.length === 0
        ? null
        : input.stages.map((stage, index) => ({ ...stage, md5: input.charts[index]?.md5 ?? '', sha256: input.charts[index]?.sha256 ?? '' }))

const SCORE_FLAGS: readonly unknown[] = Object.values(SCORE_FLAG)

const isScoreFlag = (value: unknown): value is ScoreFlag => SCORE_FLAGS.includes(value)

/** The flags a run was stored with: `extra.flags` of its row, without any value that is not a flag. */
export const storedFlagsOf = (extra: unknown): ScoreFlag[] =>
    typeof extra === 'object' && extra !== null && 'flags' in extra && Array.isArray(extra.flags) ? extra.flags.filter(isScoreFlag) : []

/**
 * The first reason a course body can not be what a real run sends, or null. Only the body is read: the judgements that
 * took a note are held to the best EX the body claims (two per note), because the chart rows the server keeps were
 * written by whoever submitted those charts first and bound nothing. The lamp is not checked: a course of 0.1.0 sends
 * its last stage's own lamp, which says nothing of the course's counts.
 */
export const findCourseIntegrityFailure = (
    input: Pick<CourseSubmissionInput, 'judge' | 'ex_score' | 'max_ex_score' | 'minbp'>,
): SubmissionIntegrityFailure | null => {
    if (!isExScoreConsistent(input.judge, input.ex_score)) return { field: 'ex_score', reason: 'ex_score does not match the judge counts' }
    if (countNotesTaken(effectiveJudgeCounts(input.judge), COURSE_BAD_TAKES_NOTE) * EX_PER_PGREAT > input.max_ex_score)
        return { field: 'judge', reason: 'judge count exceeds the notes of the course' }
    if (!isMinbpConsistent(input.judge, input.minbp)) return { field: 'minbp', reason: 'minbp is below the BAD, POOR and empty POOR count' }
    if (!areDerivedCountsStorable(input.judge)) return DERIVED_COUNTS_FAILURE
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
        exScore: params.exScore,
        maxExScore: input.max_ex_score,
        maxCombo: input.max_combo,
        gaugeValue: input.gauge_value,
        minbp: params.minbp,
        trophy: input.trophy ?? null,
        scratchAuto: params.scratchAuto,
        ranked: params.ranked,
        playedAt: input.played_at,
        gauge: input.gauge === null ? null : gaugeTypeToId(input.gauge),
        courseGauge: input.course_gauge,
        stages: toStoredStages(input),
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
            .map((chart, index) => ({ position: index, chartSha256: chart.sha256, md5: chart.md5 }))
            .filter((chart) => chart.chartSha256.length > 0 || chart.md5.length > 0)
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

        const countRank = async (run: { clear: number; assistGrade: AssistGrade; exScore: number; minbp: number; scratchAuto: boolean }) =>
            (await deps.db.countBetterCourseBests({
                courseHash: input.course_hash,
                clear: run.clear,
                assistGrade: run.assistGrade,
                exScore: run.exScore,
                minbp: run.minbp,
                includeScratchAuto: run.scratchAuto,
            })) + 1

        const stored = await deps.db.findCourseScoreByIdempotency({
            userId: user.id,
            courseHash: input.course_hash,
            playedAt: input.played_at,
        })
        if (stored) {
            if (stored.ranked) {
                await deps.db.upsertCourseBestIfBetter({
                    courseHash: input.course_hash,
                    userId: user.id,
                    scratchAuto: stored.scratchAuto,
                    courseScoreId: stored.id,
                    clear: stored.clear,
                    assistGrade: stored.assistGrade,
                    exScore: stored.exScore,
                    minbp: stored.minbp,
                    maxCombo: stored.maxCombo,
                })
            }
            const response: SubmitResponseOutput = {
                accepted: true,
                rank: stored.ranked ? await countRank(stored) : null,
                previous_best: null,
                message: DUPLICATE_SUBMISSION_MESSAGE,
                ranked,
                flags,
                is_new_best: false,
                score_id: stored.id,
            }
            const scoreRow: SubmittedCourseRun = {
                id: stored.id,
                userId: user.id,
                courseHash: input.course_hash,
                clear: stored.clear,
                ranked: stored.ranked,
                scratchAuto: stored.scratchAuto,
                flags: storedFlagsOf(stored.extra),
                gauge: stored.gauge,
                courseGauge: stored.courseGauge,
                gaugeValue: stored.gaugeValue,
                stages: stored.stages,
                playedAt: stored.playedAt,
            }
            return { courseScoreId: stored.id, duplicated: true, ranked, flags, response, scoreRow }
        }

        const courseScoreId = deps.newId('cs_')
        const row = buildCourseScoreRow({ id: courseScoreId, userId: user.id, input, exScore, minbp, ranked, scratchAuto, flags })
        const assistGrade = courseAssistGrade({ clear, gauge: row.gauge })
        const previous = await deps.db.getCourseBest({ courseHash: input.course_hash, userId: user.id, scratchAuto })
        await deps.db.insertCourseScore(row)

        const isNewBest =
            ranked &&
            isBetterBest({ clear, assistGrade, exScore, minbp }, previous) &&
            (await deps.db.upsertCourseBestIfBetter({
                courseHash: input.course_hash,
                userId: user.id,
                scratchAuto,
                courseScoreId,
                clear,
                assistGrade,
                exScore,
                minbp,
                maxCombo: input.max_combo,
            }))

        const rank = ranked ? await countRank({ clear, assistGrade, exScore, minbp, scratchAuto }) : null

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

        const scoreRow: SubmittedCourseRun = {
            id: row.id,
            userId: row.userId,
            courseHash: row.courseHash,
            clear: row.clear,
            ranked: row.ranked,
            scratchAuto: row.scratchAuto,
            flags,
            gauge: row.gauge,
            courseGauge: row.courseGauge,
            gaugeValue: row.gaugeValue,
            stages: row.stages,
            playedAt: row.playedAt,
        }

        return { courseScoreId, duplicated: false, ranked, flags, response, scoreRow }
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
                assistGrade: row.assistGrade,
                exScore: row.exScore,
                minbp: row.minbp,
                includeScratchAuto: params.includeScratchAuto,
            })) + 1
        return toScoreRecord(row, rank)
    },
})

export type CourseService = ReturnType<typeof createCourseService>

import { clearLampToId } from '@server/dto/common'
import type { CourseMetaInput, CourseSubmissionInput } from '@server/dto/course'
import { deriveExScore, deriveMinbp, isBetterBest } from '@server/service/domain/score/best-policy'
import { toScoreRecord, type ScoreRankingRow } from '@server/service/domain/score/score.service'

const RANKED_COURSE_SUBMISSION = true

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
    getCourseBest: (params: { courseHash: string; userId: string }) => Promise<{ clear: number; exScore: number; minbp: number } | null>
    upsertCourseBest: (row: {
        courseHash: string
        userId: string
        courseScoreId: string
        clear: number
        exScore: number
        minbp: number
        maxCombo: number
    }) => Promise<void>
    countBetterCourseBests: (params: { courseHash: string; clear: number; exScore: number }) => Promise<number>
    getCourseRanking: (params: { courseHash: string; limit: number; offset: number; userIds?: string[] }) => Promise<ScoreRankingRow[]>
    getCourseBestByLoginId: (params: { courseHash: string; loginId: string }) => Promise<ScoreRankingRow | null>
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

export const buildCourseScoreRow = (params: {
    id: string
    userId: string | null
    input: CourseSubmissionInput
    exScore: number
    minbp: number
    ranked: boolean
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
        ranked: params.ranked,
        playedAt: input.played_at,
        extra: input.extra,
    }
}

export const createCourseService = (deps: CourseServiceDeps) => ({
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

    submit: async (params: { input: CourseSubmissionInput; user: { id: string; loginId: string } }) => {
        const { input, user } = params
        const exScore = input.ex_score > 0 ? input.ex_score : deriveExScore(input.judge)
        const minbp = input.minbp > 0 ? input.minbp : deriveMinbp(input.judge)
        const clear = clearLampToId(input.clear)

        const existingId = await deps.db.findCourseScoreIdByIdempotency({
            userId: user.id,
            courseHash: input.course_hash,
            playedAt: input.played_at,
        })
        if (existingId) {
            return {
                courseScoreId: existingId,
                duplicated: true,
                response: { accepted: true, rank: null, previous_best: null, message: 'duplicate submission ignored' },
            }
        }

        const courseScoreId = deps.newId('cs_')
        const previous = await deps.db.getCourseBest({ courseHash: input.course_hash, userId: user.id })
        await deps.db.insertCourseScore(
            buildCourseScoreRow({ id: courseScoreId, userId: user.id, input, exScore, minbp, ranked: RANKED_COURSE_SUBMISSION }),
        )

        if (isBetterBest({ clear, exScore, minbp }, previous)) {
            await deps.db.upsertCourseBest({
                courseHash: input.course_hash,
                userId: user.id,
                courseScoreId,
                clear,
                exScore,
                minbp,
                maxCombo: input.max_combo,
            })
        }

        const rank = (await deps.db.countBetterCourseBests({ courseHash: input.course_hash, clear, exScore })) + 1

        return {
            courseScoreId,
            duplicated: false,
            response: { accepted: true, rank, previous_best: previous?.exScore ?? null, message: 'saved' },
        }
    },

    getRanking: async (params: { courseHash: string; limit: number; page: number; rivalOf?: string }) => {
        const userIds = params.rivalOf ? await deps.db.getRivalUserIds(params.rivalOf) : undefined
        if (userIds && userIds.length === 0) return []
        const rows = await deps.db.getCourseRanking({
            courseHash: params.courseHash,
            limit: params.limit,
            offset: (params.page - 1) * params.limit,
            userIds,
        })
        const base = (params.page - 1) * params.limit
        return rows.map((row, index) => toScoreRecord(row, base + index + 1))
    },

    getBest: async (params: { courseHash: string; loginId: string }) => {
        const row = await deps.db.getCourseBestByLoginId(params)
        if (!row) return null
        const rank = (await deps.db.countBetterCourseBests({ courseHash: params.courseHash, clear: row.clear, exScore: row.exScore })) + 1
        return toScoreRecord(row, rank)
    },
})

export type CourseService = ReturnType<typeof createCourseService>

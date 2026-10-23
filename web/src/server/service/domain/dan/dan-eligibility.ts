import { clearLampToId, gaugeTypeFromId, type GaugeType, type RandomOption } from '@server/dto/common'
import type { SubmittedCourseRun } from '@server/service/domain/course/course.service'
import type { CatalogDan } from '@server/service/domain/course-catalog/course-catalog.type'
import { SCORE_FLAG } from '@server/service/domain/score/ranked-policy'

export const DAN_INELIGIBLE_REASON = {
    NOT_A_DAN_COURSE: 'not_a_dan_course',
    GUEST: 'guest',
    UNRANKED: 'unranked',
    NOT_CLEARED: 'not_cleared',
    SCRATCH_AUTO: 'scratch_auto',
    ASSISTED: 'assisted',
    NO_STAGE_FACTS: 'no_stage_facts',
    NOT_A_COURSE_GAUGE: 'not_a_course_gauge',
    STAGE_COUNT: 'stage_count',
    CHART_MISMATCH: 'chart_mismatch',
    RANDOM_NOT_ALLOWED: 'random_not_allowed',
    LN_MODE_MISMATCH: 'ln_mode_mismatch',
} as const

export type DanIneligibleReason = (typeof DAN_INELIGIBLE_REASON)[keyof typeof DAN_INELIGIBLE_REASON]

/** What a stored course run says, without the columns that only name the row. */
export type DanRunFacts = Omit<SubmittedCourseRun, 'id' | 'courseHash' | 'playedAt'>

/** What a dan course asks of a run: the dan it gives, its constraint tokens (as `canonicalConstraints` writes them) and its charts in stage order. */
export type DanCourseFacts = { dan: CatalogDan | null; constraint: string[]; charts: { md5: string; sha256: string }[] }

/** The lowest lamp a cleared course can end on. NO PLAY, FAILED, ASSIST and LIGHT ASSIST are below it. */
const LOWEST_DAN_CLEAR_LAMP_ID = clearLampToId('Easy')

const COURSE_GAUGE_TYPES: readonly GaugeType[] = ['Class', 'ExClass', 'ExHardClass']

/** The RANDOM options each grade constraint lets a stage be played with. A course with another grade constraint (or none) allows any. */
const RANDOMS_ALLOWED_BY_GRADE_CONSTRAINT: Readonly<Record<string, readonly RandomOption[]>> = {
    grade: ['Off'],
    grade_mirror: ['Off', 'Mirror'],
}

/** The long note constraints, whose place in the list is the LN type the client numbers them with: 0 LN, 1 CN, 2 HCN. */
const LONG_NOTE_CONSTRAINTS = ['ln', 'cn', 'hcn']

const refuse = (reason: DanIneligibleReason) => ({ eligible: false as const, reason })

const isSameChart = (stage: { md5: string; sha256: string }, chart: { md5: string; sha256: string }) =>
    chart.md5 === ''
        ? chart.sha256 !== '' && stage.sha256.toLowerCase() === chart.sha256.toLowerCase()
        : stage.md5.toLowerCase() === chart.md5.toLowerCase()

/**
 * Whether a stored course run earns the dan of its course, and when it does not, the first reason in this order. Nothing here is
 * stored; the reason is for tests and logs.
 *
 * A dan needs a ranked run of a signed-in player that ended with gauge left on a course gauge, without SCRATCH AUTO or any assist,
 * and (the run states how every stage was played) the stages must be the course's charts in order, with the RANDOM option and the
 * long note type its constraints allow. Which gauge rule (LR2, BEATORAJA, IIDX) the run used does not matter. The constraints on the
 * speed, the GOOD and GREAT judgements and the gauge table are not checked: the body holds no fact about the first two, and the gauge
 * rule is the player's setting.
 */
export const judgeDanEligibility = (run: DanRunFacts, course: DanCourseFacts) => {
    const { dan } = course
    const { userId, stages, courseGauge, gauge } = run
    if (dan === null) return refuse(DAN_INELIGIBLE_REASON.NOT_A_DAN_COURSE)
    if (userId === null) return refuse(DAN_INELIGIBLE_REASON.GUEST)
    if (!run.ranked) return refuse(DAN_INELIGIBLE_REASON.UNRANKED)
    if (run.clear < LOWEST_DAN_CLEAR_LAMP_ID || run.gaugeValue <= 0) return refuse(DAN_INELIGIBLE_REASON.NOT_CLEARED)
    if (run.scratchAuto || run.flags.includes(SCORE_FLAG.SCRATCH_AUTO)) return refuse(DAN_INELIGIBLE_REASON.SCRATCH_AUTO)
    if (run.flags.includes(SCORE_FLAG.ASSIST)) return refuse(DAN_INELIGIBLE_REASON.ASSISTED)
    if (stages === null || courseGauge === null || gauge === null) return refuse(DAN_INELIGIBLE_REASON.NO_STAGE_FACTS)
    if (![gaugeTypeFromId(gauge), ...stages.map((stage) => stage.gauge)].every((type) => COURSE_GAUGE_TYPES.includes(type)))
        return refuse(DAN_INELIGIBLE_REASON.NOT_A_COURSE_GAUGE)
    if (stages.length !== course.charts.length) return refuse(DAN_INELIGIBLE_REASON.STAGE_COUNT)
    if (!stages.every((stage, index) => isSameChart(stage, course.charts[index]))) return refuse(DAN_INELIGIBLE_REASON.CHART_MISMATCH)
    const allowedRandoms = Object.entries(RANDOMS_ALLOWED_BY_GRADE_CONSTRAINT).find(([token]) => course.constraint.includes(token))?.[1]
    if (allowedRandoms && !stages.every((stage) => allowedRandoms.includes(stage.random))) return refuse(DAN_INELIGIBLE_REASON.RANDOM_NOT_ALLOWED)
    const requiredLongNoteType = LONG_NOTE_CONSTRAINTS.findIndex((token) => course.constraint.includes(token))
    if (requiredLongNoteType >= 0 && !stages.every((stage) => stage.lntype === requiredLongNoteType))
        return refuse(DAN_INELIGIBLE_REASON.LN_MODE_MISMATCH)
    return { eligible: true as const, grant: { userId, dan, courseGauge, gauge } }
}

export type DanEligibility = ReturnType<typeof judgeDanEligibility>

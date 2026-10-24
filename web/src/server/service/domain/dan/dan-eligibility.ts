import { clearLampToId, gaugeTypeFromId, type GaugeType, type RandomOption } from '@server/dto/common'
import type { SubmittedCourseRun } from '@server/service/domain/course/course.service'
import type { CatalogDan } from '@server/service/domain/course-catalog/course-catalog.type'
import { rankBlockingFlags, SCORE_FLAG } from '@server/service/domain/score/ranked-policy'

export const DAN_INELIGIBLE_REASON = {
    NOT_A_DAN_COURSE: 'not_a_dan_course',
    GUEST: 'guest',
    UNRANKED: 'unranked',
    NOT_CLEARED: 'not_cleared',
    NO_STAGE_FACTS: 'no_stage_facts',
    NOT_A_COURSE_GAUGE: 'not_a_course_gauge',
    STAGE_COUNT: 'stage_count',
    CHART_MISMATCH: 'chart_mismatch',
    RANDOM_NOT_ALLOWED: 'random_not_allowed',
    LN_MODE_MISMATCH: 'ln_mode_mismatch',
} as const

export type DanIneligibleReason = (typeof DAN_INELIGIBLE_REASON)[keyof typeof DAN_INELIGIBLE_REASON]

/** The two kinds of dan a run can earn: a regular one, or an A-dan for a run that met every rule but the one against assist. */
export const DAN_AWARD_KIND = { FORMAL: 'formal', ASSISTED: 'assisted' } as const

export type DanAwardKind = (typeof DAN_AWARD_KIND)[keyof typeof DAN_AWARD_KIND]

/** Why a run that earns a dan earns the A-dan: SCRATCH AUTO, an ASSIST or LIGHT ASSIST lamp, or an assist flag (a widened judge or own TOTAL). */
export const DAN_ASSIST_SOURCE = { SCRATCH_AUTO: 'scratch_auto', ASSIST_LAMP: 'assist_lamp', ASSIST_FLAG: 'assist_flag' } as const

export type DanAssistSource = (typeof DAN_ASSIST_SOURCE)[keyof typeof DAN_ASSIST_SOURCE]

/** What a stored course run says, without the columns that only name the row. */
export type DanRunFacts = Omit<SubmittedCourseRun, 'id' | 'courseHash' | 'playedAt'>

/** What a dan course asks of a run: the dan it gives, its constraint tokens (as `canonicalConstraints` writes them) and its charts in stage order. */
export type DanCourseFacts = { dan: CatalogDan | null; constraint: string[]; charts: { md5: string; sha256: string }[] }

/** The lowest lamp a course that earns any dan can end on. NO PLAY and FAILED are below it. */
const LOWEST_DAN_CLEAR_LAMP_ID = clearLampToId('AssistEasy')

/** The lowest lamp a run without assist ends on. ASSIST and LIGHT ASSIST are below it. */
const LOWEST_UNASSISTED_CLEAR_LAMP_ID = clearLampToId('Easy')

const COURSE_GAUGE_TYPES: readonly GaugeType[] = ['Class', 'ExClass', 'ExHardClass']

/** The RANDOM options each grade constraint lets a stage be played with. A course with another grade constraint (or none) allows any. */
const RANDOMS_ALLOWED_BY_GRADE_CONSTRAINT: Readonly<Record<string, readonly RandomOption[]>> = {
    grade: ['Off'],
    grade_mirror: ['Off', 'Mirror'],
}

/** The long note constraints, whose place in the list is the LN type the client numbers them with: 0 LN, 1 CN, 2 HCN. */
const LONG_NOTE_CONSTRAINTS = ['ln', 'cn', 'hcn']

const refuse = (reason: DanIneligibleReason) => ({ eligible: false as const, reason })

/**
 * Whether a run would have been ranked had it not used assist. A run stored before the server wrote this fact is taken to have
 * been, unless a flag other than assist blocks the ranking: a course run carries no such flag, so this assumes the build did not
 * block it.
 */
const isRankedWithoutAssist = (run: Pick<DanRunFacts, 'rankedWithoutAssist' | 'flags'>) =>
    run.rankedWithoutAssist ?? rankBlockingFlags(run.flags.filter((flag) => flag !== SCORE_FLAG.ASSIST)).length === 0

const isSameChart = (stage: { md5: string; sha256: string }, chart: { md5: string; sha256: string }) =>
    chart.md5 === ''
        ? chart.sha256 !== '' && stage.sha256.toLowerCase() === chart.sha256.toLowerCase()
        : stage.md5.toLowerCase() === chart.md5.toLowerCase()

/**
 * Whether a stored course run earns the dan of its course, which kind, and when it earns none, the first reason in this order.
 * Nothing here is stored; the reason is for tests and logs.
 *
 * A dan needs a run of a signed-in player that was ranked, or would have been but for assist, that ended with gauge left on a
 * course gauge, and (the run states how every stage was played) the stages must be the course's charts in order, with the RANDOM
 * option and the long note type its constraints allow. Such a run earns the regular dan, or the A-dan when it used SCRATCH AUTO,
 * ended on an ASSIST or LIGHT ASSIST lamp or carries the assist flag; it never earns both. Which gauge rule (LR2, BEATORAJA, IIDX)
 * the run used does not matter. The constraints on the speed, the GOOD and GREAT judgements and the gauge table are not checked:
 * the body holds no fact about the first two, and the gauge rule is the player's setting.
 */
export const judgeDanEligibility = (run: DanRunFacts, course: DanCourseFacts) => {
    const { dan } = course
    const { userId, stages, courseGauge, gauge } = run
    const hasAssistFlag = run.flags.includes(SCORE_FLAG.ASSIST)
    if (dan === null) return refuse(DAN_INELIGIBLE_REASON.NOT_A_DAN_COURSE)
    if (userId === null) return refuse(DAN_INELIGIBLE_REASON.GUEST)
    if (!run.ranked && !(hasAssistFlag && isRankedWithoutAssist(run))) return refuse(DAN_INELIGIBLE_REASON.UNRANKED)
    if (run.clear < LOWEST_DAN_CLEAR_LAMP_ID || run.gaugeValue <= 0) return refuse(DAN_INELIGIBLE_REASON.NOT_CLEARED)
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
    const assistedBy = [
        ...(run.scratchAuto || run.flags.includes(SCORE_FLAG.SCRATCH_AUTO) ? [DAN_ASSIST_SOURCE.SCRATCH_AUTO] : []),
        ...(run.clear < LOWEST_UNASSISTED_CLEAR_LAMP_ID ? [DAN_ASSIST_SOURCE.ASSIST_LAMP] : []),
        ...(hasAssistFlag ? [DAN_ASSIST_SOURCE.ASSIST_FLAG] : []),
    ]
    return {
        eligible: true as const,
        kind: assistedBy.length === 0 ? DAN_AWARD_KIND.FORMAL : DAN_AWARD_KIND.ASSISTED,
        assistedBy,
        grant: { userId, dan, courseGauge, gauge },
    }
}

export type DanEligibility = ReturnType<typeof judgeDanEligibility>

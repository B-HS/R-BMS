import { describe, expect, test } from 'bun:test'
import { clearLampToId, gaugeTypeToId } from '@server/dto/common'
import type { StoredCourseStage } from '@server/dto/course'
import {
    DAN_ASSIST_SOURCE,
    DAN_AWARD_KIND,
    DAN_INELIGIBLE_REASON,
    judgeDanEligibility,
    type DanCourseFacts,
    type DanRunFacts,
} from '@server/service/domain/dan/dan-eligibility'
import { SCORE_FLAG } from '@server/service/domain/score/ranked-policy'

const USER_ID = 'user-1'
const GAUGE_LEFT = 12.5
const CHARTS = ['a', 'b', 'c', 'd'].map((seed) => ({ md5: seed.repeat(32), sha256: seed.repeat(64) }))
const DAN = { system: 'genoside2018_sp', label: '初段', rank: 1 } as const
const CLASS_STAGE = { gauge: 'Class', random: 'Off', lntype: 0 } as const

const stagesOf = (override: Partial<StoredCourseStage> = {}): StoredCourseStage[] =>
    CHARTS.map((chart) => ({ ...CLASS_STAGE, ...chart, ...override }))

const baseRun = (overrides: Partial<DanRunFacts> = {}): DanRunFacts => ({
    userId: USER_ID,
    clear: clearLampToId('Normal'),
    ranked: true,
    scratchAuto: false,
    flags: [],
    rankedWithoutAssist: null,
    gauge: gaugeTypeToId('Class'),
    courseGauge: 'LR2',
    gaugeValue: GAUGE_LEFT,
    stages: stagesOf(),
    ...overrides,
})

const baseCourse = (overrides: Partial<DanCourseFacts> = {}): DanCourseFacts => ({
    dan: DAN,
    constraint: ['grade_mirror', 'gauge_lr2', 'ln'],
    charts: CHARTS,
    ...overrides,
})

const reasonOf = (run: DanRunFacts, course: DanCourseFacts) => {
    const verdict = judgeDanEligibility(run, course)
    return verdict.eligible ? null : verdict.reason
}

const kindOf = (run: DanRunFacts, course: DanCourseFacts = baseCourse()) => {
    const verdict = judgeDanEligibility(run, course)
    return verdict.eligible ? { kind: verdict.kind, assistedBy: verdict.assistedBy } : verdict.reason
}

const ASSISTED = DAN_AWARD_KIND.ASSISTED

const CUSTOM_JUDGE_RUN = { clear: clearLampToId('AssistEasy'), ranked: false, flags: [SCORE_FLAG.ASSIST, SCORE_FLAG.UNKNOWN_BUILD] }

describe('judgeDanEligibility 인정', () => {
    test('조건을 모두 지킨 LR2 판은 인정된다', () => {
        expect(judgeDanEligibility(baseRun(), baseCourse())).toEqual({
            eligible: true,
            kind: DAN_AWARD_KIND.FORMAL,
            assistedBy: [],
            grant: { userId: USER_ID, dan: DAN, courseGauge: 'LR2', gauge: gaugeTypeToId('Class') },
        })
    })

    test('BEATORAJA·IIDX 모드 판도 인정되고 모드가 기록된다', () => {
        for (const courseGauge of ['BEATORAJA', 'IIDX'] as const) {
            const verdict = judgeDanEligibility(baseRun({ courseGauge }), baseCourse())
            expect(verdict.eligible && verdict.grant.courseGauge).toBe(courseGauge)
        }
    })

    test('EX 와 EX-HARD 코스 게이지로 끝낸 판도 인정된다', () => {
        const run = baseRun({
            gauge: gaugeTypeToId('ExHardClass'),
            clear: clearLampToId('ExHard'),
            stages: [
                { ...stagesOf()[0], gauge: 'ExClass' },
                ...stagesOf()
                    .slice(1)
                    .map((stage) => ({ ...stage, gauge: 'ExHardClass' as const })),
            ],
        })
        expect(reasonOf(run, baseCourse())).toBeNull()
    })

    test('대소문자만 다른 md5 는 같은 차트다', () => {
        const run = baseRun({ stages: stagesOf().map((stage) => ({ ...stage, md5: stage.md5.toUpperCase() })) })
        expect(reasonOf(run, baseCourse())).toBeNull()
    })

    test('코스 차트에 md5 가 없으면 sha256 으로 견준다', () => {
        const course = baseCourse({ charts: CHARTS.map((chart) => ({ md5: '', sha256: chart.sha256 })) })
        expect(reasonOf(baseRun({ stages: stagesOf().map((stage) => ({ ...stage, md5: '' })) }), course)).toBeNull()
        expect(reasonOf(baseRun({ stages: stagesOf().map((stage) => ({ ...stage, md5: '', sha256: '' })) }), course)).toBe(
            DAN_INELIGIBLE_REASON.CHART_MISMATCH,
        )
    })

    test('제약이 없는 코스는 RANDOM·CN 으로 쳐도 인정된다', () => {
        const run = baseRun({ stages: stagesOf({ random: 'Random', lntype: 1 }) })
        expect(reasonOf(run, baseCourse({ constraint: [] }))).toBeNull()
    })
})

describe('judgeDanEligibility A-단위', () => {
    test('SCRATCH AUTO 로 LIGHT ASSIST 램프가 된 순위 판은 A-단위다', () => {
        const run = baseRun({ scratchAuto: true, flags: [SCORE_FLAG.SCRATCH_AUTO], clear: clearLampToId('LightAssistEasy') })
        expect(kindOf(run)).toEqual({ kind: ASSISTED, assistedBy: [DAN_ASSIST_SOURCE.SCRATCH_AUTO, DAN_ASSIST_SOURCE.ASSIST_LAMP] })
    })

    test('EASY 이상 램프라도 SCRATCH AUTO 판은 A-단위다', () => {
        expect(kindOf(baseRun({ scratchAuto: true }))).toEqual({ kind: ASSISTED, assistedBy: [DAN_ASSIST_SOURCE.SCRATCH_AUTO] })
        expect(kindOf(baseRun({ flags: [SCORE_FLAG.SCRATCH_AUTO] }))).toEqual({ kind: ASSISTED, assistedBy: [DAN_ASSIST_SOURCE.SCRATCH_AUTO] })
    })

    test('ASSIST·LIGHT ASSIST 램프만 있는 순위 판은 A-단위다', () => {
        for (const lamp of ['AssistEasy', 'LightAssistEasy'] as const)
            expect(kindOf(baseRun({ clear: clearLampToId(lamp) }))).toEqual({ kind: ASSISTED, assistedBy: [DAN_ASSIST_SOURCE.ASSIST_LAMP] })
    })

    test('넓힌 판정 판은 어시스트만 빼면 순위였다는 사실이 참이면 A-단위다', () => {
        expect(kindOf(baseRun({ ...CUSTOM_JUDGE_RUN, rankedWithoutAssist: true }))).toEqual({
            kind: ASSISTED,
            assistedBy: [DAN_ASSIST_SOURCE.ASSIST_LAMP, DAN_ASSIST_SOURCE.ASSIST_FLAG],
        })
    })

    test('그 사실이 저장되지 않은 넓힌 판정 판은 순위였다고 보고 A-단위다', () => {
        expect(kindOf(baseRun({ ...CUSTOM_JUDGE_RUN, rankedWithoutAssist: null }))).toEqual({
            kind: ASSISTED,
            assistedBy: [DAN_ASSIST_SOURCE.ASSIST_LAMP, DAN_ASSIST_SOURCE.ASSIST_FLAG],
        })
    })

    test('어시스트 판도 스테이지 제약을 지켜야 한다', () => {
        const run = baseRun({ ...CUSTOM_JUDGE_RUN, rankedWithoutAssist: true, stages: stagesOf({ random: 'Random' }) })
        expect(reasonOf(run, baseCourse())).toBe(DAN_INELIGIBLE_REASON.RANDOM_NOT_ALLOWED)
        expect(reasonOf(baseRun({ scratchAuto: true, stages: null }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NO_STAGE_FACTS)
        expect(reasonOf(baseRun({ scratchAuto: true, stages: stagesOf({ lntype: 1 }) }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.LN_MODE_MISMATCH)
    })

    test('UNKNOWN_BUILD 만 있는 순위 판은 정식 단위다', () => {
        expect(kindOf(baseRun({ flags: [SCORE_FLAG.UNKNOWN_BUILD] }))).toEqual({ kind: DAN_AWARD_KIND.FORMAL, assistedBy: [] })
    })
})

describe('judgeDanEligibility 거절 이유', () => {
    test('단위 코스가 아니면 not_a_dan_course', () => {
        expect(reasonOf(baseRun(), baseCourse({ dan: null }))).toBe(DAN_INELIGIBLE_REASON.NOT_A_DAN_COURSE)
    })

    test('게스트는 guest', () => {
        expect(reasonOf(baseRun({ userId: null }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.GUEST)
    })

    test('순위에 오르지 않는 판은 unranked', () => {
        expect(reasonOf(baseRun({ ranked: false }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.UNRANKED)
    })

    test('NO PLAY·FAILED 램프는 not_cleared', () => {
        for (const lamp of ['NoPlay', 'Failed'] as const)
            expect(reasonOf(baseRun({ clear: clearLampToId(lamp) }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NOT_CLEARED)
    })

    test('게이지가 0 이하이면 not_cleared', () => {
        expect(reasonOf(baseRun({ gaugeValue: 0 }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NOT_CLEARED)
        expect(reasonOf(baseRun({ gaugeValue: -1 }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NOT_CLEARED)
    })

    test('어시스트 판도 FAILED 나 게이지 0 이면 not_cleared', () => {
        expect(reasonOf(baseRun({ ...CUSTOM_JUDGE_RUN, rankedWithoutAssist: true, clear: clearLampToId('Failed') }), baseCourse())).toBe(
            DAN_INELIGIBLE_REASON.NOT_CLEARED,
        )
        expect(reasonOf(baseRun({ scratchAuto: true, clear: clearLampToId('LightAssistEasy'), gaugeValue: 0 }), baseCourse())).toBe(
            DAN_INELIGIBLE_REASON.NOT_CLEARED,
        )
    })

    test('빌드가 막아 순위에 오르지 않은 어시스트 없는 판은 unranked', () => {
        expect(reasonOf(baseRun({ ranked: false, flags: [SCORE_FLAG.UNKNOWN_BUILD] }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.UNRANKED)
        expect(reasonOf(baseRun({ ranked: false, flags: [SCORE_FLAG.UNKNOWN_BUILD], rankedWithoutAssist: true }), baseCourse())).toBe(
            DAN_INELIGIBLE_REASON.UNRANKED,
        )
    })

    test('어시스트만 빼도 순위에 오르지 않았을 판은 unranked', () => {
        expect(reasonOf(baseRun({ ...CUSTOM_JUDGE_RUN, rankedWithoutAssist: false }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.UNRANKED)
    })

    test('저장된 사실이 없을 때 어시스트 말고 순위를 막는 플래그가 있으면 unranked', () => {
        expect(reasonOf(baseRun({ ...CUSTOM_JUDGE_RUN, flags: [SCORE_FLAG.ASSIST, SCORE_FLAG.JUDGE_WIDTH] }), baseCourse())).toBe(
            DAN_INELIGIBLE_REASON.UNRANKED,
        )
    })

    test('스테이지·코스 게이지·최종 게이지 중 하나라도 없으면 no_stage_facts', () => {
        expect(reasonOf(baseRun({ stages: null }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NO_STAGE_FACTS)
        expect(reasonOf(baseRun({ courseGauge: null }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NO_STAGE_FACTS)
        expect(reasonOf(baseRun({ gauge: null }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NO_STAGE_FACTS)
    })

    test('코스 게이지가 아닌 스테이지나 최종 게이지는 not_a_course_gauge', () => {
        expect(reasonOf(baseRun({ stages: [{ ...stagesOf()[0], gauge: 'Hard' }, ...stagesOf().slice(1)] }), baseCourse())).toBe(
            DAN_INELIGIBLE_REASON.NOT_A_COURSE_GAUGE,
        )
        expect(reasonOf(baseRun({ gauge: gaugeTypeToId('Normal') }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NOT_A_COURSE_GAUGE)
    })

    test('스테이지 수가 코스 차트 수와 다르면 stage_count', () => {
        expect(reasonOf(baseRun({ stages: stagesOf().slice(1) }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.STAGE_COUNT)
        expect(reasonOf(baseRun({ stages: [...stagesOf(), ...stagesOf()] }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.STAGE_COUNT)
    })

    test('다른 곡이거나 순서가 다르면 chart_mismatch', () => {
        expect(reasonOf(baseRun({ stages: stagesOf().map((stage) => ({ ...stage, md5: '1'.repeat(32) })) }), baseCourse())).toBe(
            DAN_INELIGIBLE_REASON.CHART_MISMATCH,
        )
        expect(reasonOf(baseRun({ stages: stagesOf().toReversed() }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.CHART_MISMATCH)
    })

    test('여러 이유가 겹치면 먼저 걸리는 이유를 돌려준다', () => {
        expect(reasonOf(baseRun({ ranked: false, clear: clearLampToId('Failed'), stages: null }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.UNRANKED)
        expect(reasonOf(baseRun({ clear: clearLampToId('Failed'), scratchAuto: true }), baseCourse())).toBe(DAN_INELIGIBLE_REASON.NOT_CLEARED)
    })
})

describe('judgeDanEligibility 제약', () => {
    test('grade_mirror 코스의 RANDOM 은 random_not_allowed', () => {
        const run = baseRun({ stages: [{ ...stagesOf()[0], random: 'Random' }, ...stagesOf().slice(1)] })
        expect(reasonOf(run, baseCourse({ constraint: ['grade_mirror'] }))).toBe(DAN_INELIGIBLE_REASON.RANDOM_NOT_ALLOWED)
    })

    test('grade_mirror 코스의 MIRROR 는 인정된다', () => {
        expect(reasonOf(baseRun({ stages: stagesOf({ random: 'Mirror' }) }), baseCourse({ constraint: ['grade_mirror'] }))).toBeNull()
    })

    test('grade 코스의 MIRROR 는 random_not_allowed', () => {
        expect(reasonOf(baseRun({ stages: stagesOf({ random: 'Mirror' }) }), baseCourse({ constraint: ['grade'] }))).toBe(
            DAN_INELIGIBLE_REASON.RANDOM_NOT_ALLOWED,
        )
    })

    test('grade_random 코스는 RANDOM 도 인정된다', () => {
        expect(reasonOf(baseRun({ stages: stagesOf({ random: 'Random' }) }), baseCourse({ constraint: ['grade_random'] }))).toBeNull()
    })

    test('ln 코스의 CN 스테이지는 ln_mode_mismatch', () => {
        const run = baseRun({ stages: [...stagesOf().slice(0, -1), { ...stagesOf()[3], lntype: 1 }] })
        expect(reasonOf(run, baseCourse({ constraint: ['ln'] }))).toBe(DAN_INELIGIBLE_REASON.LN_MODE_MISMATCH)
    })

    test('cn 과 hcn 코스는 그 종류로 친 판만 인정된다', () => {
        expect(reasonOf(baseRun({ stages: stagesOf({ lntype: 1 }) }), baseCourse({ constraint: ['cn'] }))).toBeNull()
        expect(reasonOf(baseRun({ stages: stagesOf({ lntype: 2 }) }), baseCourse({ constraint: ['hcn'] }))).toBeNull()
        expect(reasonOf(baseRun({ stages: stagesOf({ lntype: 0 }) }), baseCourse({ constraint: ['hcn'] }))).toBe(
            DAN_INELIGIBLE_REASON.LN_MODE_MISMATCH,
        )
    })

    test('속도·GOOD·GREAT·게이지 표 제약은 서버가 확인하지 않는다', () => {
        const course = baseCourse({ constraint: ['no_speed', 'no_good', 'gauge_5k'] })
        expect(reasonOf(baseRun(), course)).toBeNull()
    })
})

import { createHash } from 'node:crypto'
import { clearLampToId, gaugeTypeToId } from '@server/dto/common'
import type { SubmittedCourseRun } from '@server/service/domain/course/course.service'
import type { DanCourseRecord } from '@server/service/domain/dan/dan.service'

export const DAN_PLAYED_AT = 1_797_000_000_000
export const DAN_STAGE_COUNT = 4
export const DAN_GAUGE_LEFT = 12.5

const hashOf = (algorithm: 'md5' | 'sha256', seed: string) => createHash(algorithm).update(seed).digest('hex')

const chartsOf = (courseKey: string) =>
    Array.from({ length: DAN_STAGE_COUNT }, (_, index) => ({
        md5: hashOf('md5', `${courseKey}-${index}`),
        sha256: hashOf('sha256', `${courseKey}-${index}`),
    }))

export const FIRST_DAN_COURSE: DanCourseRecord = {
    courseHash: 'dan-fixture-first',
    dan: { system: 'genoside2018_sp', label: '初段', rank: 1 },
    constraint: ['grade_mirror', 'gauge_lr2', 'ln'],
    charts: chartsOf('first'),
}

export const SECOND_DAN_COURSE: DanCourseRecord = {
    courseHash: 'dan-fixture-second',
    dan: { system: 'genoside2018_sp', label: '二段', rank: 2 },
    constraint: ['grade_mirror', 'gauge_lr2', 'ln'],
    charts: chartsOf('second'),
}

export const STELLA_DAN_COURSE: DanCourseRecord = {
    courseHash: 'dan-fixture-stella',
    dan: { system: 'stella', label: 'st0', rank: 0 },
    constraint: ['grade_mirror', 'gauge_lr2', 'ln'],
    charts: chartsOf('stella'),
}

/** A stored run that earns the dan of `course`: ranked, cleared on CLASS with gauge left, every stage played on the course's chart in order. */
export const danRun = (course: DanCourseRecord, overrides: Partial<SubmittedCourseRun> = {}): SubmittedCourseRun => ({
    id: 'cs_fixture_1',
    userId: 'user-1',
    courseHash: course.courseHash,
    clear: clearLampToId('Normal'),
    ranked: true,
    scratchAuto: false,
    flags: [],
    rankedWithoutAssist: null,
    gauge: gaugeTypeToId('Class'),
    courseGauge: 'LR2',
    gaugeValue: DAN_GAUGE_LEFT,
    stages: course.charts.map((chart) => ({ gauge: 'Class', random: 'Off', lntype: 0, ...chart })),
    playedAt: DAN_PLAYED_AT,
    ...overrides,
})

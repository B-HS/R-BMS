import { describe, expect, test } from 'bun:test'
import {
    CHART_PATTERN_LANE_COUNT,
    CHART_PATTERN_MAX_NOTES,
    CHART_PATTERN_MAX_TIME_US,
    chartPatternDocumentSchema,
    chartPatternMd5ParamSchema,
} from '@server/dto/chart-pattern'
import { makePatternDocument, type FixtureNote } from '../support/chart-pattern-fixture'

const NOTES: FixtureNote[] = [
    [0, 0, 0],
    [0, 7, 0],
    [100_000, 1, 1],
    [400_000, 1, 2],
    [500_000, 3, 3],
]

const issuesOf = (value: unknown) => {
    const parsed = chartPatternDocumentSchema.safeParse(value)
    return parsed.success ? [] : parsed.error.issues
}

describe('chartPatternDocumentSchema (rbms-cli chart-pattern 출력 형식)', () => {
    test('CLI 가 쓰는 모양을 그대로 받고 play_level null 과 total 0 도 받는다', () => {
        const document = makePatternDocument(NOTES, {
            play_level: null,
            total: 0,
            declared_ln: 'cn',
            tempo: [{ t_us: 0, bpm: 150, stop_us: 0, scroll: 1 }],
        })
        const parsed = chartPatternDocumentSchema.parse(document)
        expect(parsed).toEqual(document)
        expect(JSON.stringify(parsed)).toBe(JSON.stringify(document))
        const reordered = Object.fromEntries(Object.entries(document).toReversed())
        expect(JSON.stringify(chartPatternDocumentSchema.parse(reordered))).toBe(JSON.stringify(document))
    })

    test('모르는 필드는 버리고 저장하지 않는다', () => {
        expect(chartPatternDocumentSchema.parse({ ...makePatternDocument(NOTES), extra_audio: 'x' })).not.toHaveProperty('extra_audio')
    })

    test('열 길이가 다르면 한 개의 오류로 거절한다', () => {
        const document = makePatternDocument(NOTES)
        const issues = issuesOf({ ...document, notes: { ...document.notes, section: document.notes.section.slice(1) } })
        expect(issues).toHaveLength(1)
        expect(issues[0].path).toEqual(['notes', 'section'])
    })

    test('시각·레인 순서가 어긋나면 그 위치를 가리켜 거절하고, 같은 시각·같은 레인이 이어지는 것은 받는다', () => {
        const document = makePatternDocument(NOTES)
        const swapped = { ...document, notes: { ...document.notes, lane: [7, 0, 1, 1, 3] } }
        expect(issuesOf(swapped)[0]?.path).toEqual(['notes', 'time_us', 1])
        const repeated = { ...document, notes: { ...document.notes, time_us: [0, 0, 0, 0, 0], lane: [0, 0, 1, 1, 3] } }
        expect(issuesOf(repeated)).toEqual([])
    })

    test('레인 범위·kind 0~3·음수 시각을 벗어나면 많은 노트가 틀려도 첫 위치 하나만 보고한다', () => {
        const document = makePatternDocument(NOTES)
        const wrongLanes = issuesOf({ ...document, notes: { ...document.notes, lane: NOTES.map(() => CHART_PATTERN_LANE_COUNT) } })
        expect(wrongLanes).toHaveLength(1)
        expect(wrongLanes[0].path).toEqual(['notes', 'lane', 0])
        expect(issuesOf({ ...document, notes: { ...document.notes, kind: [0, 0, 1, 2, 4] } })[0]?.path).toEqual(['notes', 'kind', 4])
        expect(issuesOf({ ...document, notes: { ...document.notes, time_us: [-1, 0, 100_000, 400_000, 500_000] } })[0]?.path).toEqual([
            'notes',
            'time_us',
            0,
        ])
        expect(issuesOf({ ...document, notes: { ...document.notes, lane: [0, 7, 1.5, 1, 3] } })[0]?.path).toEqual(['notes', 'lane', 2])
    })

    test('형식 태그·버전·모드·해시 형식이 다르면 거절한다', () => {
        const document = makePatternDocument(NOTES)
        for (const wrong of [
            { format: 'rbms-chart' },
            { version: 2 },
            { mode: 'BEAT_14K' },
            { md5: document.md5.toUpperCase() },
            { sha256: 'abc' },
            { parser_version: 0 },
            { total: -1 },
            { declared_ln: 'mgq' },
        ]) {
            expect(issuesOf({ ...document, ...wrong }).length).toBeGreaterThan(0)
        }
    })

    test('tempo 는 t_us 순서여야 한다', () => {
        const tempo = [
            { t_us: 200, bpm: 150, stop_us: 0, scroll: 1 },
            { t_us: 100, bpm: 180, stop_us: 0, scroll: 1 },
        ]
        expect(issuesOf(makePatternDocument(NOTES, { tempo })).length).toBe(1)
    })

    test('시각 상한은 24시간이고 그 값까지는 받으며 넘으면 첫 위치 하나만 가리켜 거절한다', () => {
        const hours24 = 24 * 60 * 60 * 1_000_000
        expect(CHART_PATTERN_MAX_TIME_US).toBe(hours24)
        const document = makePatternDocument(NOTES)
        expect(issuesOf({ ...document, notes: { ...document.notes, time_us: [0, 0, 100_000, 400_000, hours24] } })).toEqual([])
        const late = issuesOf({ ...document, notes: { ...document.notes, time_us: [0, 0, 100_000, 400_000, hours24 + 1] } })
        expect(late).toHaveLength(1)
        expect(late[0].path).toEqual(['notes', 'time_us', 4])
        const hugeAndSafe = issuesOf({ ...document, notes: { ...document.notes, time_us: [0, 0, 100_000, 400_000, Number.MAX_SAFE_INTEGER] } })
        expect(hugeAndSafe[0]?.path).toEqual(['notes', 'time_us', 4])
    })

    test('tempo 의 t_us 도 같은 시각 상한을 넘으면 거절한다', () => {
        const hours24 = CHART_PATTERN_MAX_TIME_US
        expect(issuesOf(makePatternDocument(NOTES, { tempo: [{ t_us: hours24, bpm: 150, stop_us: 0, scroll: 1 }] }))).toEqual([])
        expect(issuesOf(makePatternDocument(NOTES, { tempo: [{ t_us: hours24 + 1, bpm: 150, stop_us: 0, scroll: 1 }] })).length).toBeGreaterThan(0)
    })

    test('노트 수는 상한까지 받고 넘으면 열 하나의 한 개 오류로 거절한다', () => {
        const columns = (length: number) => ({
            time_us: Array.from({ length }, () => 0),
            lane: Array.from({ length }, () => 0),
            kind: Array.from({ length }, () => 0),
            section: Array.from({ length }, () => 0),
        })
        const document = makePatternDocument(NOTES)
        expect(issuesOf({ ...document, notes: columns(CHART_PATTERN_MAX_NOTES) })).toEqual([])
        const tooMany = issuesOf({ ...document, notes: columns(CHART_PATTERN_MAX_NOTES + 1) })
        expect(tooMany.length).toBeGreaterThan(0)
        expect(tooMany.every((issue) => issue.path[0] === 'notes')).toBe(true)
    })

    test('경로의 md5 는 소문자 32자 16진만 받는다', () => {
        expect(chartPatternMd5ParamSchema.safeParse({ md5: 'a'.repeat(32) }).success).toBe(true)
        for (const md5 of ['A'.repeat(32), 'a'.repeat(31), '../etc/passwd']) expect(chartPatternMd5ParamSchema.safeParse({ md5 }).success).toBe(false)
    })
})

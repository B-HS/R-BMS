import { describe, expect, test } from 'bun:test'
import { BP_RUN_BUCKET_KEYS, computeBpRuns, type RunNote } from '@server/service/domain/analysis/bp-runs'
import type { Grade } from '@server/service/domain/analysis/play-cells'
import type { PatternTag } from '@server/service/domain/analysis/pattern-tags'

const STEP_US = 100_000

const notesOf = (grades: readonly Grade[], tagsAt: (index: number) => PatternTag[] = () => [], npsAt: (index: number) => number = () => 8) =>
    grades.map((grade, index): RunNote => ({ timeUs: (index + 1) * STEP_US, grade, tags: tagsAt(index), nps2: npsAt(index) }))

const repeat = (grade: Grade, count: number) => Array.from({ length: count }, () => grade)

describe('연속 BAD/POOR 구간(2.2.4)', () => {
    const grades: Grade[] = [
        ...repeat('pgreat', 8),
        'bad',
        'poor',
        'bad',
        ...repeat('great', 10),
        'bad',
        'good',
        'poor',
        'poor',
        'pgreat',
        'pgreat',
        'bad',
        'bad',
        'bad',
        ...repeat('pgreat', 5),
    ]
    const tagsAt = (index: number): PatternTag[] => {
        if (index < 5) return ['trill']
        if (index < 8) return ['chord_2']
        return ['stream']
    }
    const runs = computeBpRuns(notesOf(grades, tagsAt, (index) => (index === 8 ? 12.34 : 8)))

    test('길이 칸은 1·2·3-4·5-9·10+ 이고 가장 긴 구간의 노트 수를 남긴다', () => {
        expect(BP_RUN_BUCKET_KEYS).toEqual(['1', '2', '3-4', '5-9', '10+'])
        expect(runs.histogram).toEqual({ '1': 1, '2': 1, '3-4': 2, '5-9': 0, '10+': 0 })
        expect(runs.longest).toBe(3)
        const long = computeBpRuns(notesOf([...repeat('poor', 5), 'great', ...repeat('bad', 12)]))
        expect(long.histogram).toEqual({ '1': 0, '2': 0, '3-4': 0, '5-9': 1, '10+': 1 })
        expect(long.longest).toBe(12)
    })

    test('3노트 이상 구간을 길이 큰 순·같으면 앞선 순으로 남기고, 시작 NPS·회복 노트 수·직전 8노트 태그를 붙인다', () => {
        expect(runs.runs).toEqual([
            {
                from_ms: 900,
                to_ms: 1100,
                length: 3,
                start_nps: 12.3,
                recovery_notes: 10,
                preceding_tags: [
                    { tag: 'trill', notes: 5 },
                    { tag: 'chord_2', notes: 3 },
                ],
            },
            { from_ms: 2800, to_ms: 3000, length: 3, start_nps: 8, recovery_notes: null, preceding_tags: [{ tag: 'stream', notes: 8 }] },
        ])
    })

    test('2노트 이상 구간의 직전 창을 합친 태그 빈도와 노트 수는 겹치는 노트를 한 번만 센다', () => {
        expect(runs.context_notes).toBe(20)
        expect(runs.context_tags).toEqual({ chord_2: 3, trill: 5, stream: 12 })
    })

    test('BP 가 없으면 빈 구간과 0 이다', () => {
        expect(computeBpRuns(notesOf(repeat('great', 4)))).toEqual({
            histogram: { '1': 0, '2': 0, '3-4': 0, '5-9': 0, '10+': 0 },
            longest: 0,
            runs: [],
            context_tags: {},
            context_notes: 0,
        })
    })

    test('목록은 BP_RUN_TOP 개까지다', () => {
        const many = Array.from({ length: 10 }, (_, index): Grade[] => [...repeat('poor', 3 + index), 'pgreat']).flat()
        const listed = computeBpRuns(notesOf(many)).runs
        expect(listed).toHaveLength(8)
        expect(listed.map((run) => run.length)).toEqual([12, 11, 10, 9, 8, 7, 6, 5])
    })
})

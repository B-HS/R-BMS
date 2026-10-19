import { describe, expect, test } from 'bun:test'
import { MAX_DENSITY_WINDOWS } from '@server/service/domain/analysis/analysis-constants'
import {
    classifyPattern,
    densityWindowCounts,
    type PatternNote,
    type PatternNoteKind,
    type PatternTag,
    type TempoPoint,
} from '@server/service/domain/analysis/pattern-tags'

const BPM = 150
const S16 = 100_000
const SECOND = 1_000_000
const SCRATCH = 7

const note = (timeUs: number, lane: number, kind: PatternNoteKind = 'normal'): PatternNote => ({ timeUs, lane, kind })

const ordered = (notes: PatternNote[]) => notes.toSorted((a, b) => a.timeUs - b.timeUs || a.lane - b.lane)

const classify = (notes: PatternNote[], tempo: TempoPoint[] = []) => {
    const input = ordered(notes)
    return { notes: input, tags: classifyPattern({ notes: input, tempo, initBpm: BPM }) }
}

const WINDOW_TAGS: PatternTag[] = ['dense', 'soflan', 'mine_near']

const shapeTags = (notes: PatternNote[], tempo: TempoPoint[] = []) =>
    classify(notes, tempo).tags.map((tags) => tags.filter((tag) => !WINDOW_TAGS.includes(tag)))

const lanesAt = (lanes: number[], gapUs = S16, startUs = 0) => lanes.map((lane, index) => note(startUs + index * gapUs, lane))

describe('패턴 분류 v1 골든 (BPM 150, 16분 = 100ms)', () => {
    test('두 레인을 16분 간격으로 번갈아 4개 이상 치면 트릴이고 계단이나 연타가 아니다', () => {
        expect(shapeTags(lanesAt([0, 1, 0, 1, 0]))).toEqual([['trill'], ['trill'], ['trill'], ['trill'], ['trill']])
    })

    test('트릴은 세 번째 레인이 끼면 끊기고, 3개뿐이거나 간격이 16분보다 길면 트릴이 아니다', () => {
        expect(shapeTags(lanesAt([0, 1, 0, 1, 4]))).toEqual([['trill'], ['trill'], ['trill'], ['trill'], []])
        expect(shapeTags(lanesAt([0, 1, 0]))).toEqual([[], [], []])
        expect(shapeTags(lanesAt([0, 1, 0, 1], 2 * S16))).toEqual([[], [], [], []])
    })

    test('이웃 레인으로 한 방향 4개 이상이면 계단이고, 방향이 바뀌면 새 계단이 그 앞 노트부터 시작한다', () => {
        expect(shapeTags(lanesAt([0, 1, 2, 3, 4]))).toEqual([['stair'], ['stair'], ['stair'], ['stair'], ['stair']])
        expect(shapeTags(lanesAt([6, 5, 4, 3]))).toEqual([['stair'], ['stair'], ['stair'], ['stair']])
        expect(shapeTags(lanesAt([0, 1, 2, 3, 2, 1, 0]))).toEqual([['stair'], ['stair'], ['stair'], ['stair'], ['stair'], ['stair'], ['stair']])
        expect(shapeTags(lanesAt([0, 1, 2, 1, 0]))).toEqual([[], [], [], [], []])
    })

    test('같은 레인 간격이 16분 둘 이하로 3개 이상이면 연타이고 간격이 셋이면 끊긴다', () => {
        const notes = [note(0, 2), note(2 * S16, 2), note(4 * S16, 2), note(7 * S16, 2)]
        expect(shapeTags(notes)).toEqual([['jack'], ['jack'], ['jack'], []])
    })

    test('동시치기 행이 같은 레인을 이어 받으면 그 레인의 노트는 연타이기도 하다', () => {
        const rows = [0, 1, 2].flatMap((index) => [note(index * S16, 0), note(index * S16, 2)])
        expect(shapeTags(rows)).toEqual(Array.from({ length: 6 }, () => ['chord_2', 'jack']))
    })

    test('한 행의 건반 수로 동시치기 2·3·4개 이상을 나누고, 같은 행 스크래치는 동시치기가 아니라 scratch_chord 다', () => {
        const notes = [
            note(0, 0),
            note(0, 2),
            note(SECOND, 1),
            note(SECOND, 3),
            note(SECOND, 5),
            ...[0, 1, 2, 3, 4].map((lane) => note(2 * SECOND, lane)),
            note(2 * SECOND, SCRATCH),
        ]
        expect(shapeTags(notes)).toEqual([
            ['chord_2'],
            ['chord_2'],
            ['chord_3'],
            ['chord_3'],
            ['chord_3'],
            ['chord_4plus'],
            ['chord_4plus'],
            ['chord_4plus'],
            ['chord_4plus'],
            ['chord_4plus'],
            ['scratch_chord'],
        ])
    })

    test('스크래치는 16분 안에 건반이 없으면 단독, 있으면 섞임(그 건반은 with_scratch), 16분 둘 간격 3개 이상이면 연속이다', () => {
        const notes = [
            note(0, SCRATCH),
            note(3 * S16, 0),
            note(SECOND, SCRATCH),
            note(SECOND + S16 / 2, 4),
            note(2 * SECOND, SCRATCH),
            note(2 * SECOND + 2 * S16, SCRATCH),
            note(2 * SECOND + 4 * S16, SCRATCH),
        ]
        expect(shapeTags(notes)).toEqual([
            ['scratch_solo'],
            [],
            ['scratch_mixed'],
            ['with_scratch'],
            ['scratch_solo', 'scratch_run'],
            ['scratch_solo', 'scratch_run'],
            ['scratch_solo', 'scratch_run'],
        ])
    })

    test('LN 을 누른 동안 다른 레인에서 친 노트, LN 을 뗀 뒤 16분 안에 같은 손으로 친 노트를 가린다', () => {
        const notes = [
            note(0, 0, 'ln_head'),
            note(4 * S16, 3),
            note(8 * S16, 0, 'ln_end'),
            note(8 * S16 + S16 / 2, 1),
            note(8 * S16 + S16 / 2, 5),
            note(12 * S16, 6),
        ]
        expect(shapeTags(notes)).toEqual([['ln_head'], ['ln_hold_hit'], [], ['chord_2', 'ln_release_then_hit'], ['chord_2'], []])
    })

    test('LN 끝은 판정되지 않아 태그가 없고, CN·HCN 끝(charge_end)은 ln_end 이다', () => {
        const ln = classify([note(0, 0, 'ln_head'), note(SECOND, 0, 'ln_end')]).tags
        const cn = classify([note(0, 0, 'ln_head'), note(SECOND, 0, 'charge_end')]).tags
        expect(ln[1]).toEqual([])
        expect(cn[1]).toContain('ln_end')
    })

    test('BPM 이 1.25배 이상 바뀐 지점과 100ms 이상 STOP 뒤 2초, SCROLL 이 1 이 아닌 구간의 노트가 소프란이다', () => {
        const tempo: TempoPoint[] = [
            { tUs: 2 * SECOND, bpm: 300, stopUs: 0, scroll: 1 },
            { tUs: 6 * SECOND, bpm: 300, stopUs: 200_000, scroll: 1 },
            { tUs: 10 * SECOND, bpm: 300, stopUs: 0, scroll: 0.5 },
        ]
        const times = [1.5, 2.5, 4.5, 8, 8.5, 11].map((seconds) => seconds * SECOND)
        const { tags } = classify(
            times.map((timeUs) => note(timeUs, 0)),
            tempo,
        )
        expect(tags.map((list) => list.includes('soflan'))).toEqual([false, true, false, true, false, true])
    })

    test('1.2배 BPM 변화는 소프란이 아니고, BPM 이 바뀐 뒤의 간격은 그 BPM 의 16분으로 잰다', () => {
        const slight = classify([note(SECOND, 0), note(3 * SECOND, 0)], [{ tUs: 2 * SECOND, bpm: 180, stopUs: 0, scroll: 1 }]).tags
        expect(slight.map((list) => list.includes('soflan'))).toEqual([false, false])
        const doubled: TempoPoint[] = [{ tUs: 0, bpm: 300, stopUs: 0, scroll: 1 }]
        const trillAt300 = classifyPattern({ notes: lanesAt([0, 1, 0, 1], S16 / 2), tempo: doubled, initBpm: BPM })
        expect(trillAt300.map((list) => list.includes('trill'))).toEqual([true, true, true, true])
        const trillAt150 = classifyPattern({ notes: lanesAt([0, 1, 0, 1], S16 / 2 + S16 / 4), tempo: doubled, initBpm: BPM })
        expect(trillAt150.map((list) => list.includes('trill'))).toEqual([false, false, false, false])
    })

    test('트릴·계단·연타가 아닌 16분 이하 간격의 8행 이상이 연속 난타이고 7행이면 아니다', () => {
        const eight = shapeTags(lanesAt([0, 3, 1, 5, 2, 6, 0, 4]))
        expect(eight.every((tags) => tags.includes('stream'))).toBe(true)
        const seven = shapeTags(lanesAt([0, 3, 1, 5, 2, 6, 0]))
        expect(seven.some((tags) => tags.includes('stream'))).toBe(false)
        const trillInStream = shapeTags(lanesAt([0, 1, 0, 1, 5, 2, 6, 3]))
        expect(trillInStream.map((tags) => tags.includes('stream'))).toEqual([false, false, false, false, true, true, true, true])
    })

    test('1초 창의 판정 노트 수가 차트의 90 분위 이상인 창의 노트가 고밀도다', () => {
        const sparse = Array.from({ length: 9 }, (_, second) => note(second * SECOND, second % 7))
        const burst = Array.from({ length: 6 }, (_, index) => note(9 * SECOND + index * S16, index))
        const { notes, tags } = classify([...sparse, ...burst])
        expect(notes.filter((_, index) => tags[index].includes('dense')).map((entry) => entry.timeUs)).toEqual(burst.map((entry) => entry.timeUs))
    })

    test('지뢰 앞뒤 16분 안의 판정 노트가 mine_near 이고 지뢰 자신은 태그가 없다', () => {
        const { tags } = classify([note(SECOND, 4, 'mine'), note(SECOND + S16 / 2, 0), note(SECOND + 3 * S16, 0)])
        expect(tags[0]).toEqual([])
        expect(tags[1]).toContain('mine_near')
        expect(tags[2]).not.toContain('mine_near')
    })

    test('같은 입력은 항상 같은 결과다', () => {
        const notes = lanesAt([0, 1, 2, 3, 2, 1, 0, 7, 0, 1, 0, 1])
        expect(classify(notes).tags).toEqual(classify(notes).tags)
    })
})

describe('밀도 창 수 (densityWindowCounts)', () => {
    test('창 하나는 1초이고 마지막 노트까지의 창 수만큼만 만든다', () => {
        expect(densityWindowCounts([note(0, 0), note(500_000, 1), note(2 * SECOND + 1, 2)])).toEqual([2, 0, 1])
        expect(densityWindowCounts([])).toEqual([])
    })

    test('시각이 한도를 크게 넘는 노트가 들어와도 큰 배열을 만들지 않고 마지막 창에 센다', () => {
        const counts = densityWindowCounts([note(0, 0), note(Number.MAX_SAFE_INTEGER, 1)])
        expect(counts).toHaveLength(MAX_DENSITY_WINDOWS)
        expect(counts[0]).toBe(1)
        expect(counts.at(-1)).toBe(1)
    })
})

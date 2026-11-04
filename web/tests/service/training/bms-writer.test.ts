import { describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { countPlayableNotes, trainingTotalOf, writeTrainingBms, type TrainingNote } from '@server/service/domain/training/bms-writer'
import type { TrainingSection, TrainingSpec } from '@server/service/domain/training/spec'

const sectionOf = (partial: Partial<TrainingSection>): TrainingSection => ({
    index: 0,
    focus_key: 'tag:stream',
    kind: 'stream',
    lane_hint: 'all',
    chord_size: 1,
    scratch_ratio: 0,
    ln_ratio: 0,
    bpm: 160,
    target_nps: 8,
    measure_from: 1,
    measure_count: 2,
    from_us: 0,
    to_us: 0,
    grid: 16,
    actual_nps: 0,
    notes: 0,
    intent: '',
    ...partial,
})

const SPEC: TrainingSpec = {
    generator_version: 1,
    seed: 1,
    base_bpm: 160,
    title: 'R-BMS Training · 2026-11-04 · PUSH',
    subtitle: '[00000000]',
    play_level: 9,
    sections: [
        sectionOf({ index: 0, measure_from: 1, measure_count: 1 }),
        sectionOf({ index: 1, measure_from: 2, measure_count: 1, bpm: 240 }),
        sectionOf({ index: 2, measure_from: 3, measure_count: 1, bpm: 80 }),
        sectionOf({ index: 3, measure_from: 4, measure_count: 1, bpm: 160 }),
    ],
    corrections: [],
}

const note = (measure: number, num: number, den: number, lane: number, kind: TrainingNote['kind'] = 'normal'): TrainingNote => ({
    measure,
    num,
    den,
    lane,
    kind,
})

const NOTES = [
    note(1, 0, 1, 0),
    note(1, 1, 4, 5, 'ln_head'),
    note(1, 1, 3, 3),
    note(1, 1, 2, 7),
    note(2, 1, 6, 0),
    note(2, 0, 1, 5, 'ln_end'),
    note(4, 3, 4, 6),
]

describe('BMS 작성기 (writeTrainingBms)', () => {
    test('헤더는 정해진 순서이고 BPM 변화는 값 오름차순 번호, 줄은 마디·채널 순이다', () => {
        const { text } = writeTrainingBms(SPEC, NOTES)
        expect(text.split('\n')).toEqual([
            '#PLAYER 1',
            '#GENRE R-BMS TRAINING',
            '#TITLE R-BMS Training · 2026-11-04 · PUSH',
            '#SUBTITLE [00000000]',
            '#ARTIST R-BMS',
            '#BPM 160',
            '#PLAYLEVEL 9',
            '#RANK 2',
            '#TOTAL 160',
            '#LNMODE 1',
            '#WAV01 hit.wav',
            '#WAV02 count.wav',
            '#BPM01 80',
            '#BPM02 160',
            '#BPM03 240',
            '#00001:02020202',
            '#00111:01',
            '#00114:000100',
            '#00116:0001',
            '#00158:00010000',
            '#00208:03',
            '#00211:000100000000',
            '#00258:01',
            '#00308:01',
            '#00408:02',
            '#00419:00000001',
            '',
        ])
    })

    test('UTF-8 이고 BOM 이 없으며 해시는 쓴 바이트 그대로의 md5·sha256 이다', () => {
        const written = writeTrainingBms(SPEC, NOTES)
        expect(Array.from(written.bytes.slice(0, 3))).toEqual(Array.from(new TextEncoder().encode('#PL')))
        expect(written.md5).toBe(createHash('md5').update(written.bytes).digest('hex'))
        expect(written.sha256).toBe(createHash('sha256').update(written.bytes).digest('hex'))
        expect(new TextDecoder().decode(written.bytes)).toBe(written.text)
    })

    test('줄 해상도는 그 줄 분모의 최소공배수이고 쓴 그대로 lines 로 돌려준다', () => {
        const { lines } = writeTrainingBms(SPEC, [note(1, 1, 4, 0), note(1, 1, 6, 0), note(1, 5, 8, 0)])
        expect(lines.find((line) => line.channel === '11')).toEqual({
            measure: 1,
            channel: '11',
            resolution: 24,
            tokens: Array.from({ length: 24 }, (_, index) => ([4, 6, 15].includes(index) ? '01' : '00')),
        })
    })

    test('TOTAL 은 노트 수로 늘고 160 아래로 내려가지 않으며 LN 끝은 세지 않는다', () => {
        expect(trainingTotalOf(0)).toBe(160)
        expect(trainingTotalOf(300)).toBe(240)
        expect(trainingTotalOf(1000)).toBe(461)
        expect(countPlayableNotes(NOTES)).toBe(6)
    })
})

import { describe, expect, test } from 'bun:test'
import { inflateRawSync } from 'node:zlib'
import { crc32 } from '@server/service/domain/training/crc32'
import { TRAINING_COUNT_WAV, TRAINING_HIT_WAV } from '@server/service/domain/training/training-constants'
import { createClickWav } from '@server/service/domain/training/wav'
import { createTrainingZip, trainingChartFileNameOf } from '@server/service/domain/training/zip'

const ID = `tc_${'0123456789abcdef'.repeat(2)}`
const BMS = new TextEncoder().encode('#TITLE R-BMS Training · 2026-11-04 · PUSH\n#BPM 160\n#00111:01010101\n'.repeat(20))

type ReadEntry = { name: string; method: number; flags: number; crc: number; size: number; data: Uint8Array; offset: number }

/** Reads an archive the way a plain zip reader does: the end record, then the central directory, then each local header. */
const readZip = (bytes: Uint8Array) => {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    const end = bytes.length - 22
    expect(view.getUint32(end, true)).toBe(0x06054b50)
    const count = view.getUint16(end + 10, true)
    const centralSize = view.getUint32(end + 12, true)
    const centralOffset = view.getUint32(end + 16, true)
    expect(centralOffset + centralSize).toBe(end)
    const entries: ReadEntry[] = []
    let cursor = centralOffset
    for (let index = 0; index < count; index += 1) {
        expect(view.getUint32(cursor, true)).toBe(0x02014b50)
        const nameLength = view.getUint16(cursor + 28, true)
        expect([view.getUint16(cursor + 30, true), view.getUint16(cursor + 32, true)]).toEqual([0, 0])
        const offset = view.getUint32(cursor + 42, true)
        const name = new TextDecoder().decode(bytes.slice(cursor + 46, cursor + 46 + nameLength))
        expect(view.getUint32(offset, true)).toBe(0x04034b50)
        const flags = view.getUint16(offset + 6, true)
        const method = view.getUint16(offset + 8, true)
        expect([view.getUint16(offset + 10, true), view.getUint16(offset + 12, true)]).toEqual([0, 0x0021])
        const crc = view.getUint32(offset + 14, true)
        const storedSize = view.getUint32(offset + 18, true)
        const size = view.getUint32(offset + 22, true)
        const localNameLength = view.getUint16(offset + 26, true)
        expect(view.getUint16(offset + 28, true)).toBe(0)
        expect(new TextDecoder().decode(bytes.slice(offset + 30, offset + 30 + localNameLength))).toBe(name)
        const dataFrom = offset + 30 + localNameLength
        expect(view.getUint32(cursor + 16, true)).toBe(crc)
        entries.push({ name, method, flags, crc, size, data: bytes.slice(dataFrom, dataFrom + storedSize), offset })
        cursor += 46 + nameLength
    }
    return entries
}

const contentOf = (entry: ReadEntry) => (entry.method === 8 ? new Uint8Array(inflateRawSync(entry.data)) : entry.data)

describe('CRC-32', () => {
    test('표준 검사값 "123456789" 는 0xCBF43926', () => {
        expect(crc32(new TextEncoder().encode('123456789'))).toBe(0xcbf43926)
        expect(crc32(new Uint8Array())).toBe(0)
    })
})

describe('훈련곡 zip (createTrainingZip)', () => {
    test('뿌리에 차트(deflate)와 클릭 두 개(저장)가 있고 deflate 를 되풀면 내용과 CRC 가 원본과 같다', () => {
        const entries = readZip(createTrainingZip({ id: ID, bms: BMS }))
        expect(entries.map(({ name, method, flags }) => ({ name, method, flags }))).toEqual([
            { name: 'rbms-training-01234567.bms', method: 8, flags: 0x0800 },
            { name: 'hit.wav', method: 0, flags: 0x0800 },
            { name: 'count.wav', method: 0, flags: 0x0800 },
        ])
        const expected = [BMS, createClickWav(TRAINING_HIT_WAV), createClickWav(TRAINING_COUNT_WAV)]
        entries.forEach((entry, index) => {
            const content = contentOf(entry)
            expect(Buffer.from(content).equals(Buffer.from(expected[index]))).toBe(true)
            expect(entry.size).toBe(content.length)
            expect(entry.crc).toBe(crc32(content))
        })
        expect(entries[0].data.length).toBeLessThan(BMS.length)
    })

    test('저장 방식은 차트 바이트를 그대로 담고, 같은 입력은 같은 zip 이다', () => {
        const stored = createTrainingZip({ id: ID, bms: BMS, bmsMethod: 'store' })
        const [chart] = readZip(stored)
        expect(chart.method).toBe(0)
        expect(Buffer.from(chart.data).equals(Buffer.from(BMS))).toBe(true)
        expect(Buffer.from(createTrainingZip({ id: ID, bms: BMS, bmsMethod: 'store' })).equals(Buffer.from(stored))).toBe(true)
        expect(Array.from(stored.slice(0, 4))).toEqual([0x50, 0x4b, 0x03, 0x04])
    })

    test('차트 이름은 id 의 16진 앞 8자다', () => {
        expect(trainingChartFileNameOf(`tc_${'f'.repeat(32)}`)).toBe('rbms-training-ffffffff.bms')
    })
})

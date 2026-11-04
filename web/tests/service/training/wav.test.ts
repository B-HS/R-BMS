import { describe, expect, test } from 'bun:test'
import { TRAINING_COUNT_WAV, TRAINING_HIT_WAV } from '@server/service/domain/training/training-constants'
import { CLICK_WAV_HEADER_BYTES, createClickWav } from '@server/service/domain/training/wav'

const ascii = (bytes: Uint8Array, from: number, length: number) => String.fromCharCode(...bytes.slice(from, from + length))

const samplesOf = (bytes: Uint8Array) => {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    return Array.from({ length: (bytes.length - CLICK_WAV_HEADER_BYTES) / 2 }, (_, index) => view.getInt16(CLICK_WAV_HEADER_BYTES + index * 2, true))
}

describe('클릭 wav (createClickWav)', () => {
    test('RIFF/WAVE PCM 16비트 모노 44.1kHz 헤더와 길이', () => {
        const hit = createClickWav(TRAINING_HIT_WAV)
        const view = new DataView(hit.buffer)
        expect([ascii(hit, 0, 4), ascii(hit, 8, 4), ascii(hit, 12, 4), ascii(hit, 36, 4)]).toEqual(['RIFF', 'WAVE', 'fmt ', 'data'])
        expect(view.getUint32(4, true)).toBe(hit.length - 8)
        expect([view.getUint32(16, true), view.getUint16(20, true), view.getUint16(22, true)]).toEqual([16, 1, 1])
        expect([view.getUint32(24, true), view.getUint32(28, true), view.getUint16(32, true), view.getUint16(34, true)]).toEqual([
            44_100, 88_200, 2, 16,
        ])
        expect(view.getUint32(40, true)).toBe(1323 * 2)
        expect(hit.length).toBe(2690)
        expect(createClickWav(TRAINING_COUNT_WAV).length).toBe(5336)
    })

    test('샘플은 감쇠하는 사인이고 마지막 32 샘플이 0 까지 줄어든다', () => {
        const samples = samplesOf(createClickWav(TRAINING_HIT_WAV))
        expect(samples[0]).toBe(0)
        expect(samples[1]).toBe(Math.round(0.6 * 32767 * Math.sin((2 * Math.PI * 2000) / 44_100) * Math.exp(-1 / ((44_100 * 8) / 1000))))
        expect(samples.at(-1)).toBe(0)
        expect(Math.max(...samples.slice(0, 50).map(Math.abs))).toBeGreaterThan(Math.max(...samples.slice(-200).map(Math.abs)) * 10)
    })

    test('같은 설정이면 같은 바이트다', () => {
        expect(Buffer.from(createClickWav(TRAINING_COUNT_WAV)).equals(Buffer.from(createClickWav(TRAINING_COUNT_WAV)))).toBe(true)
    })
})

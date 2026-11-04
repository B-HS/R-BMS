import { TRAINING_WAV_AMPLITUDE, TRAINING_WAV_FADE_SAMPLES, TRAINING_WAV_SAMPLE_RATE } from '@server/service/domain/training/training-constants'

const RIFF_HEADER_BYTES = 44
const RIFF_SIZE_EXCLUDED_BYTES = 8
const FORMAT_CHUNK_BYTES = 16
const PCM_FORMAT = 1
const CHANNELS = 1
const BITS_PER_SAMPLE = 16
const BITS_PER_BYTE = 8
const BYTES_PER_SAMPLE = BITS_PER_SAMPLE / BITS_PER_BYTE
const INT16_MAX = 32767
const MILLIS_PER_SECOND = 1000

const OFFSET = {
    RIFF: 0,
    RIFF_SIZE: 4,
    WAVE: 8,
    FORMAT: 12,
    FORMAT_SIZE: 16,
    AUDIO_FORMAT: 20,
    CHANNELS: 22,
    SAMPLE_RATE: 24,
    BYTE_RATE: 28,
    BLOCK_ALIGN: 32,
    BITS: 34,
    DATA: 36,
    DATA_SIZE: 40,
} as const

const writeAscii = (view: DataView, offset: number, text: string) => {
    for (let index = 0; index < text.length; index += 1) view.setUint8(offset + index, text.charCodeAt(index))
}

/** The header size of the WAVE files `createClickWav` writes: the samples start here. */
export const CLICK_WAV_HEADER_BYTES = RIFF_HEADER_BYTES

/**
 * A click as a WAVE file (plan 3.8): 16-bit mono PCM at TRAINING_WAV_SAMPLE_RATE, a sine of `frequencyHz` that decays with time
 * constant `decayMs`, its last TRAINING_WAV_FADE_SAMPLES samples faded linearly to zero so it ends without a pop.
 */
export const createClickWav = (click: { frequencyHz: number; durationMs: number; decayMs: number }) => {
    const sampleCount = Math.round((TRAINING_WAV_SAMPLE_RATE * click.durationMs) / MILLIS_PER_SECOND)
    const dataBytes = sampleCount * BYTES_PER_SAMPLE
    const bytes = new Uint8Array(RIFF_HEADER_BYTES + dataBytes)
    const view = new DataView(bytes.buffer)
    writeAscii(view, OFFSET.RIFF, 'RIFF')
    view.setUint32(OFFSET.RIFF_SIZE, bytes.length - RIFF_SIZE_EXCLUDED_BYTES, true)
    writeAscii(view, OFFSET.WAVE, 'WAVE')
    writeAscii(view, OFFSET.FORMAT, 'fmt ')
    view.setUint32(OFFSET.FORMAT_SIZE, FORMAT_CHUNK_BYTES, true)
    view.setUint16(OFFSET.AUDIO_FORMAT, PCM_FORMAT, true)
    view.setUint16(OFFSET.CHANNELS, CHANNELS, true)
    view.setUint32(OFFSET.SAMPLE_RATE, TRAINING_WAV_SAMPLE_RATE, true)
    view.setUint32(OFFSET.BYTE_RATE, TRAINING_WAV_SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE, true)
    view.setUint16(OFFSET.BLOCK_ALIGN, CHANNELS * BYTES_PER_SAMPLE, true)
    view.setUint16(OFFSET.BITS, BITS_PER_SAMPLE, true)
    writeAscii(view, OFFSET.DATA, 'data')
    view.setUint32(OFFSET.DATA_SIZE, dataBytes, true)
    const decaySamples = (TRAINING_WAV_SAMPLE_RATE * click.decayMs) / MILLIS_PER_SECOND
    const fadeFrom = sampleCount - TRAINING_WAV_FADE_SAMPLES
    for (let index = 0; index < sampleCount; index += 1) {
        const fade = index < fadeFrom ? 1 : (sampleCount - 1 - index) / (TRAINING_WAV_FADE_SAMPLES - 1)
        const wave = Math.sin((2 * Math.PI * click.frequencyHz * index) / TRAINING_WAV_SAMPLE_RATE) * Math.exp(-index / decaySamples)
        view.setInt16(RIFF_HEADER_BYTES + index * BYTES_PER_SAMPLE, Math.round(TRAINING_WAV_AMPLITUDE * INT16_MAX * wave * fade), true)
    }
    return bytes
}

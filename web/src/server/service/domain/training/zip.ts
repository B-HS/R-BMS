import { deflateRawSync } from 'node:zlib'
import { crc32 } from '@server/service/domain/training/crc32'
import { trainingShortIdOf } from '@server/service/domain/training/spec'
import {
    TRAINING_COUNT_WAV,
    TRAINING_COUNT_WAV_NAME,
    TRAINING_HIT_WAV,
    TRAINING_HIT_WAV_NAME,
} from '@server/service/domain/training/training-constants'
import { createClickWav } from '@server/service/domain/training/wav'

export const TRAINING_ZIP_METHODS = ['deflate', 'store'] as const
export type TrainingZipMethod = (typeof TRAINING_ZIP_METHODS)[number]

const SIGNATURE = { LOCAL: 0x04034b50, CENTRAL: 0x02014b50, END: 0x06054b50 } as const
const COMPRESSION = { store: 0, deflate: 8 } as const satisfies Record<TrainingZipMethod, number>
/** Version 2.0 is what deflate needs; the upper byte 0 says the attributes are MS-DOS ones. */
const ZIP_VERSION = 20
/** General purpose bit 11: the names are UTF-8. */
const UTF8_NAME_FLAG = 0x0800
/** The fixed MS-DOS stamp of every entry, 1980-01-01 00:00, so the same files make the same archive. */
const DOS_DATE = 0x0021
const DOS_TIME = 0
const LOCAL_HEADER_BYTES = 30
const CENTRAL_HEADER_BYTES = 46
const END_RECORD_BYTES = 22

const TRAINING_CHART_FILE_PREFIX = 'rbms-training-'
const TRAINING_CHART_FILE_EXTENSION = '.bms'

/** The chart's name in the archive: `rbms-training-<short id>.bms`. */
export const trainingChartFileNameOf = (id: string) => `${TRAINING_CHART_FILE_PREFIX}${trainingShortIdOf(id)}${TRAINING_CHART_FILE_EXTENSION}`

const CLICK_FILES = [
    { name: TRAINING_HIT_WAV_NAME, data: createClickWav(TRAINING_HIT_WAV) },
    { name: TRAINING_COUNT_WAV_NAME, data: createClickWav(TRAINING_COUNT_WAV) },
] as const

type Entry = { name: Uint8Array; method: TrainingZipMethod; crc: number; size: number; stored: Uint8Array; offset: number }

const UINT16_BYTES = 2
const UINT32_BYTES = 4
const NONE = 0

/** Writes little-endian fields one after another from `at`. */
const createFieldWriter = (bytes: Uint8Array, at: number) => {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    let cursor = at
    return {
        u16: (value: number) => {
            view.setUint16(cursor, value, true)
            cursor += UINT16_BYTES
        },
        u32: (value: number) => {
            view.setUint32(cursor, value, true)
            cursor += UINT32_BYTES
        },
        raw: (data: Uint8Array) => {
            bytes.set(data, cursor)
            cursor += data.length
        },
        cursor: () => cursor,
    }
}

/** A local header (with the entry's data after it) or a central directory record; returns where the next record starts. */
const writeEntry = (bytes: Uint8Array, at: number, entry: Entry, isCentral: boolean) => {
    const field = createFieldWriter(bytes, at)
    field.u32(isCentral ? SIGNATURE.CENTRAL : SIGNATURE.LOCAL)
    if (isCentral) field.u16(ZIP_VERSION)
    field.u16(ZIP_VERSION)
    field.u16(UTF8_NAME_FLAG)
    field.u16(COMPRESSION[entry.method])
    field.u16(DOS_TIME)
    field.u16(DOS_DATE)
    field.u32(entry.crc)
    field.u32(entry.stored.length)
    field.u32(entry.size)
    field.u16(entry.name.length)
    field.u16(NONE)
    if (isCentral) {
        field.u16(NONE)
        field.u16(NONE)
        field.u16(NONE)
        field.u32(NONE)
        field.u32(entry.offset)
    }
    field.raw(entry.name)
    if (!isCentral) field.raw(entry.stored)
    return field.cursor()
}

/**
 * The zip a client downloads for a training chart (plan 3.9): the chart and the two click sounds at the archive's root, a local
 * header and data per file, then the central directory and the end record, with no data descriptor, extra field or ZIP64. The chart
 * is deflated (`deflate`) or stored as it is (`store`, whose bytes do not depend on the zlib build, for the golden archive); the
 * sounds are stored.
 */
export const createTrainingZip = (params: { id: string; bms: Uint8Array; bmsMethod?: TrainingZipMethod }) => {
    const bmsMethod = params.bmsMethod ?? 'deflate'
    const files = [
        { name: trainingChartFileNameOf(params.id), data: params.bms, method: bmsMethod },
        ...CLICK_FILES.map((file) => ({ ...file, method: 'store' as const })),
    ]
    const encoder = new TextEncoder()
    let offset = 0
    const entries = files.map((file): Entry => {
        const name = encoder.encode(file.name)
        const stored = file.method === 'deflate' ? new Uint8Array(deflateRawSync(file.data)) : file.data
        const entry = { name, method: file.method, crc: crc32(file.data), size: file.data.length, stored, offset }
        offset += LOCAL_HEADER_BYTES + name.length + stored.length
        return entry
    })
    const centralOffset = offset
    const centralBytes = entries.reduce((sum, entry) => sum + CENTRAL_HEADER_BYTES + entry.name.length, 0)
    const bytes = new Uint8Array(centralOffset + centralBytes + END_RECORD_BYTES)
    for (const entry of entries) writeEntry(bytes, entry.offset, entry, false)
    const end = createFieldWriter(
        bytes,
        entries.reduce((cursor, entry) => writeEntry(bytes, cursor, entry, true), centralOffset),
    )
    end.u32(SIGNATURE.END)
    end.u16(NONE)
    end.u16(NONE)
    end.u16(entries.length)
    end.u16(entries.length)
    end.u32(centralBytes)
    end.u32(centralOffset)
    end.u16(NONE)
    return bytes
}

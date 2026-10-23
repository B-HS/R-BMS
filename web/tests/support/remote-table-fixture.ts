import { createHash } from 'node:crypto'
import type {
    RemoteSnapshotRow,
    RemoteSnapshotVersion,
    TableCoursePlan,
    TableSnapshotStoreDb,
} from '@server/service/domain/table-refresh/table-refresh.service'
import { REMOTE_TABLES } from '@server/service/domain/table-refresh/table-sources'
import type { FetchBytes, FetchedBytes } from '@server/service/shared/http/fetch-limited'

export const STELLA = REMOTE_TABLES[0]
export const SATELLITE = REMOTE_TABLES[1]
export const STELLA_DATA_URL = 'https://stellabms.xyz/st/score.json'
export const SATELLITE_DATA_URL = 'https://stellabms.xyz/sl/score.json'
export const FIRST_NOW = 1_760_000_000_000
export const DAY_MS = 86_400_000

const HEX_RADIX = 16
const MD5_LENGTH = 32
const LEVEL_SPAN = 13

export const md5Of = (n: number) => n.toString(HEX_RADIX).padStart(MD5_LENGTH, '0')

export const encodeJson = (value: unknown) => new TextEncoder().encode(JSON.stringify(value))

export const sha256Of = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')

export const stellaHeader = (overrides: Record<string, unknown> = {}) => ({
    name: 'Stella',
    symbol: 'st',
    data_url: 'score.json',
    course: [
        [{ name: 'Stella Skill Simulator 4th st0', constraint: ['grade_mirror', 'gauge_lr2', 'ln'], md5: [md5Of(1), md5Of(2), md5Of(3), md5Of(4)] }],
    ],
    ...overrides,
})

export const satelliteHeader = () => ({ name: 'Satellite', symbol: 'sl', data_url: 'score.json' })

export const remoteEntries = (count: number, options: { offset?: number; levelOf?: (n: number) => string } = {}) =>
    Array.from({ length: count }, (_, index) => {
        const n = (options.offset ?? 0) + index + 1
        return {
            id: n,
            md5: md5Of(n),
            sha256: n.toString(HEX_RADIX).padStart(64, 'a'),
            title: `title-${n}`,
            artist: `artist-${n}`,
            url: `https://example.test/song/${n}`,
            url_diff: '',
            level: options.levelOf ? options.levelOf(n) : String(n % LEVEL_SPAN),
        }
    })

export const ok = (value: unknown): FetchedBytes => ({ ok: true, status: 200, bytes: encodeJson(value), contentType: 'application/json' })

export const okBytes = (bytes: Uint8Array, status = 200): FetchedBytes => ({ ok: true, status, bytes, contentType: 'application/json' })

export const failure = (reason: Extract<FetchedBytes, { ok: false }>['reason']): FetchedBytes => ({ ok: false, reason })

/** A fetch that answers from a table of URL to answer and records the URLs it was asked for, in order. */
export const createStubFetch = (answers: Record<string, FetchedBytes | (() => FetchedBytes)>) => {
    const asked: string[] = []
    const fetchBytes: FetchBytes = async (url) => {
        asked.push(url)
        const answer = answers[url]
        if (!answer) return failure('network')
        return typeof answer === 'function' ? answer() : answer
    }
    return { fetchBytes, asked }
}

export const toVersion = (row: RemoteSnapshotRow): RemoteSnapshotVersion => ({
    tableId: row.tableId,
    headerSha256: row.headerSha256,
    dataSha256: row.dataSha256,
    fetchedAt: row.fetchedAt,
    checkedAt: row.checkedAt,
    lastOutcome: row.lastOutcome,
})

export const rowOf = (
    definition: typeof STELLA | typeof SATELLITE,
    header: unknown,
    entries: unknown,
    stamps: { fetchedAt?: number; checkedAt?: number; lastOutcome?: string } = {},
): RemoteSnapshotRow => {
    const headerBytes = encodeJson(header)
    const dataBytes = encodeJson(entries)
    return {
        tableId: definition.id,
        headerUrl: definition.headerUrl,
        dataUrl: definition.id === STELLA.id ? STELLA_DATA_URL : SATELLITE_DATA_URL,
        headerSha256: sha256Of(headerBytes),
        dataSha256: sha256Of(dataBytes),
        headerBytes,
        dataBytes,
        entryCount: Array.isArray(entries) ? entries.length : 0,
        levelCount: 0,
        fetchedAt: stamps.fetchedAt ?? FIRST_NOW,
        checkedAt: stamps.checkedAt ?? FIRST_NOW,
        lastOutcome: stamps.lastOutcome ?? 'saved',
    }
}

/** A store in memory with the same compare-and-set rule as the database one, and counters for what the cache read. */
export const createMemoryStore = (initial: RemoteSnapshotRow[] = []) => {
    const rows = new Map(initial.map((row) => [row.tableId, row]))
    const calls = { listVersions: 0, loadSnapshot: 0, saveSnapshot: 0, markChecked: 0 }
    const store: TableSnapshotStoreDb = {
        listVersions: async () => {
            calls.listVersions += 1
            return [...rows.values()].map(toVersion)
        },
        loadSnapshot: async (tableId) => {
            calls.loadSnapshot += 1
            return rows.get(tableId) ?? null
        },
        saveSnapshot: async (row, previousDataSha256) => {
            calls.saveSnapshot += 1
            const existing = rows.get(row.tableId)
            if (previousDataSha256 === null ? existing : existing?.dataSha256 !== previousDataSha256) return false
            rows.set(row.tableId, row)
            return true
        },
        markChecked: async ({ tableId, checkedAt, outcome }) => {
            calls.markChecked += 1
            const existing = rows.get(tableId)
            if (existing) rows.set(tableId, { ...existing, checkedAt, lastOutcome: outcome })
        },
    }
    return { store, rows, calls }
}

export const planOf = (id: string, counts: Partial<Omit<TableCoursePlan, 'id'>> = {}): TableCoursePlan => ({
    id,
    inserted: 0,
    changed: 0,
    unchanged: 0,
    deactivated: 0,
    danCourses: 0,
    newDanHashes: [],
    ...counts,
})

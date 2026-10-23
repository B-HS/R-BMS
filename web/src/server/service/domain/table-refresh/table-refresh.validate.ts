import { remoteEntriesSchema, remoteHeaderSchema, type RemoteEntry, type RemoteHeader } from '@server/dto/table-refresh'
import type { TableSourceDefinition } from '@server/service/domain/table-refresh/table-sources'

export const TABLE_REFRESH_OUTCOME = { SAVED: 'saved', UNCHANGED: 'unchanged', REJECTED: 'rejected', FAILED: 'failed' } as const
export type TableRefreshOutcome = (typeof TABLE_REFRESH_OUTCOME)[keyof typeof TABLE_REFRESH_OUTCOME]

export const TABLE_REFRESH_REASON = {
    FETCH_FAILED: 'fetch_failed',
    HTTP_STATUS: 'http_status',
    TOO_LARGE: 'too_large',
    NOT_JSON: 'not_json',
    HEADER_INVALID: 'header_invalid',
    NAME_MISMATCH: 'name_mismatch',
    SYMBOL_MISMATCH: 'symbol_mismatch',
    DATA_URL_OFF_SITE: 'data_url_off_site',
    ENTRIES_INVALID: 'entries_invalid',
    DUPLICATE_MD5: 'duplicate_md5',
    TOO_FEW_ENTRIES: 'too_few_entries',
    SHRANK: 'shrank',
    CHANGED_TOO_MUCH: 'changed_too_much',
    COURSE_SYNC_FAILED: 'course_sync_failed',
    UNEXPECTED_ERROR: 'unexpected_error',
} as const
export type TableRefreshReason = (typeof TABLE_REFRESH_REASON)[keyof typeof TABLE_REFRESH_REASON]

export const TABLE_FIRST_SNAPSHOT_MIN_ENTRIES = 500
export const TABLE_MIN_KEPT_PERCENT = 90
export const TABLE_MAX_CHANGE_PERCENT = 25

const PERCENT = 100
const INTEGER_LEVEL_PATTERN = /^\d+$/
const HTTPS_PROTOCOL = 'https:'

type Failure = { ok: false; reason: TableRefreshReason }

export type EntryDiff = { added: number; removed: number; releveled: number }

const failure = (reason: TableRefreshReason): Failure => ({ ok: false, reason })

/** Parses downloaded bytes as JSON text. A leading byte order mark is dropped and bytes that are not UTF-8 are refused. */
export const parseJsonBytes = (bytes: Uint8Array) => {
    try {
        const value: unknown = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes))
        return { ok: true as const, value }
    } catch {
        return failure(TABLE_REFRESH_REASON.NOT_JSON)
    }
}

const resolveDataUrl = (dataUrl: string, headerUrl: string) => (URL.canParse(dataUrl, headerUrl) ? new URL(dataUrl, headerUrl) : null)

export const validateRemoteHeader = (definition: TableSourceDefinition, raw: unknown) => {
    const parsed = remoteHeaderSchema.safeParse(raw)
    if (!parsed.success) return failure(TABLE_REFRESH_REASON.HEADER_INVALID)
    if (parsed.data.name !== definition.expectedName) return failure(TABLE_REFRESH_REASON.NAME_MISMATCH)
    if (parsed.data.symbol !== definition.symbol) return failure(TABLE_REFRESH_REASON.SYMBOL_MISMATCH)
    const dataUrl = resolveDataUrl(parsed.data.data_url, definition.headerUrl)
    if (!dataUrl || dataUrl.protocol !== HTTPS_PROTOCOL || dataUrl.hostname !== definition.allowedHost) {
        return failure(TABLE_REFRESH_REASON.DATA_URL_OFF_SITE)
    }
    return { ok: true as const, header: parsed.data, dataUrl: dataUrl.href }
}

export const validateRemoteEntries = (raw: unknown) => {
    const parsed = remoteEntriesSchema.safeParse(raw)
    if (!parsed.success) return failure(TABLE_REFRESH_REASON.ENTRIES_INVALID)
    if (new Set(parsed.data.map((entry) => entry.md5)).size !== parsed.data.length) return failure(TABLE_REFRESH_REASON.DUPLICATE_MD5)
    return { ok: true as const, entries: parsed.data }
}

/** How a new set of charts differs from the one stored: charts that appear, charts that vanish, and charts that moved to another level. */
export const diffEntries = (
    previous: readonly Pick<RemoteEntry, 'md5' | 'level'>[],
    next: readonly Pick<RemoteEntry, 'md5' | 'level'>[],
): EntryDiff => {
    const previousLevels = new Map(previous.map((entry) => [entry.md5, entry.level]))
    const nextMd5s = new Set(next.map((entry) => entry.md5))
    return {
        added: next.filter((entry) => !previousLevels.has(entry.md5)).length,
        removed: previous.filter((entry) => !nextMd5s.has(entry.md5)).length,
        releveled: next.filter((entry) => previousLevels.has(entry.md5) && previousLevels.get(entry.md5) !== entry.level).length,
    }
}

/**
 * Whether a downloaded table may replace the stored one. An empty table never does, a first copy must be large enough to be the real
 * table, and a table that lost more than a tenth of its charts or changed more than a quarter of them is held back for a person to
 * look at. `allowLargeChange` lifts only the last two checks.
 */
export const judgeChange = (params: { previousCount: number | null; nextCount: number; diff: EntryDiff | null; allowLargeChange: boolean }) => {
    const { previousCount, nextCount, diff, allowLargeChange } = params
    if (nextCount === 0 || (previousCount === null && nextCount < TABLE_FIRST_SNAPSHOT_MIN_ENTRIES))
        return failure(TABLE_REFRESH_REASON.TOO_FEW_ENTRIES)
    if (previousCount === null || allowLargeChange) return { ok: true as const }
    if (nextCount * PERCENT < previousCount * TABLE_MIN_KEPT_PERCENT) return failure(TABLE_REFRESH_REASON.SHRANK)
    const changed = diff ? diff.added + diff.removed + diff.releveled : 0
    if (changed * PERCENT > previousCount * TABLE_MAX_CHANGE_PERCENT) return failure(TABLE_REFRESH_REASON.CHANGED_TOO_MUCH)
    return { ok: true as const }
}

/**
 * The level order of a table: the header's own list when it has one, otherwise the levels the charts use, in number order when every
 * level is a whole number and in the order they first appear when not.
 */
export const deriveLevelOrder = (header: Pick<RemoteHeader, 'level_order'>, entries: readonly Pick<RemoteEntry, 'level'>[]) => {
    if (header.level_order && header.level_order.length > 0) return header.level_order
    const levels = [...new Set(entries.map((entry) => entry.level))]
    return levels.every((level) => INTEGER_LEVEL_PATTERN.test(level)) ? levels.toSorted((a, b) => Number(a) - Number(b)) : levels
}

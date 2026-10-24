import type { LampCount } from '@features/lamp/lamp-stack-bar'
import type { TableSnapshotEntry, TableSnapshotLevel, TableSnapshotRecord } from '@entities/table-snapshot.type'
import {
    CLEAR_LAMPS,
    DISPLAY_LAMPS,
    MIN_CLEAR_LAMP,
    displayLampRank,
    isSameDisplayLamp,
    type ClearLamp,
    type LampWithGrade,
} from '@shared/constants/clear-lamp'
import { getMaxExScore } from '@shared/lib/format'
import { serializeSearchParams } from '@shared/lib/search-params'

const TABLE_FILTER_PARAM = { QUERY: 'q', FROM: 'from', TO: 'to', LAMP: 'lamp', SORT: 'sort' } as const

type LampFilterDefinition = { value: string; min?: ClearLamp; max?: ClearLamp }

export const TABLE_LAMP_FILTERS = [
    { value: 'all' },
    { value: 'noplay', max: 'NoPlay' },
    { value: 'failed', min: 'Failed', max: 'Failed' },
    { value: 'unclear', max: 'Failed' },
    { value: 'cleared', min: MIN_CLEAR_LAMP },
    { value: 'easy', min: 'Easy' },
    { value: 'clear', min: 'Normal' },
    { value: 'hard', min: 'Hard' },
    { value: 'exhard', min: 'ExHard' },
    { value: 'fullcombo', min: 'FullCombo' },
] as const satisfies readonly LampFilterDefinition[]

type TableLampFilter = (typeof TABLE_LAMP_FILTERS)[number]['value']

export const TABLE_SORTS = [
    { value: 'table', needsRecords: false },
    { value: 'title', needsRecords: false },
    { value: 'lamp', needsRecords: true },
    { value: 'rate', needsRecords: true },
] as const

type TableSort = (typeof TABLE_SORTS)[number]['value']

export type TableFilterState = {
    query: string
    from: string | null
    to: string | null
    lamp: TableLampFilter
    sort: TableSort
}

type TableRecordMap = Record<string, TableSnapshotRecord>

export const DEFAULT_TABLE_FILTER: TableFilterState = { query: '', from: null, to: null, lamp: 'all', sort: 'table' }

const TITLE_COLLATOR = new Intl.Collator('ja')
const MD5_PREFIX_PATTERN = /^[0-9a-f]{6,32}$/
const SAFE_ANCHOR_PATTERN = /^[0-9A-Za-z-]+$/
const UNPLAYED_RATE = -1

const isLampFilter = (value: string | null): value is TableLampFilter => TABLE_LAMP_FILTERS.some((filter) => filter.value === value)
const isTableSort = (value: string | null): value is TableSort => TABLE_SORTS.some((sort) => sort.value === value)

export const parseTableLampFilter = (value: string | null) => (isLampFilter(value) ? value : DEFAULT_TABLE_FILTER.lamp)
export const parseTableSort = (value: string | null) => (isTableSort(value) ? value : DEFAULT_TABLE_FILTER.sort)
const lampRank = (lamp: ClearLamp) => CLEAR_LAMPS.indexOf(lamp)

/** Folds width and case so full-width and half-width spellings of a title match, the same rule the server search uses. */
const normaliseTableSearch = (value: string) => value.normalize('NFKC').toLowerCase().trim()

/** Gives every level a fragment id that is safe in a URL; levels that are not plain words fall back to their position. */
export const levelAnchorId = (level: string, index: number) => (SAFE_ANCHOR_PATTERN.test(level) ? `lv-${level}` : `lv-other-${index}`)

/** Reads the shareable filter from the address, dropping values that do not name a level, lamp filter or sort of this table. */
export const readTableFilter = (params: Pick<URLSearchParams, 'get'>, levels: readonly string[]): TableFilterState => {
    const from = params.get(TABLE_FILTER_PARAM.FROM)
    const to = params.get(TABLE_FILTER_PARAM.TO)
    const lamp = params.get(TABLE_FILTER_PARAM.LAMP)
    const sort = params.get(TABLE_FILTER_PARAM.SORT)
    const validFrom = from !== null && levels.includes(from) ? from : null
    const validTo = to !== null && levels.includes(to) ? to : null
    const isReversed = validFrom !== null && validTo !== null && levels.indexOf(validFrom) > levels.indexOf(validTo)
    return {
        query: params.get(TABLE_FILTER_PARAM.QUERY) ?? '',
        from: isReversed ? validTo : validFrom,
        to: isReversed ? validFrom : validTo,
        lamp: parseTableLampFilter(lamp),
        sort: parseTableSort(sort),
    }
}

/** Writes only the values that differ from the defaults, so the plain table keeps a clean address. */
export const writeTableFilter = (filter: TableFilterState) =>
    serializeSearchParams({
        [TABLE_FILTER_PARAM.QUERY]: filter.query.trim() === '' ? undefined : filter.query,
        [TABLE_FILTER_PARAM.FROM]: filter.from ?? undefined,
        [TABLE_FILTER_PARAM.TO]: filter.to ?? undefined,
        [TABLE_FILTER_PARAM.LAMP]: filter.lamp === DEFAULT_TABLE_FILTER.lamp ? undefined : filter.lamp,
        [TABLE_FILTER_PARAM.SORT]: filter.sort === DEFAULT_TABLE_FILTER.sort ? undefined : filter.sort,
    })

/** Counts the filters a reader has narrowed, for the badge on the mobile filter button. Search is shown on its own. */
export const countActiveTableFilters = (filter: TableFilterState) =>
    [filter.from !== null || filter.to !== null, filter.lamp !== DEFAULT_TABLE_FILTER.lamp, filter.sort !== DEFAULT_TABLE_FILTER.sort].filter(Boolean)
        .length

const entryLamp = (entry: TableSnapshotEntry, records: TableRecordMap | null): ClearLamp => records?.[entry.md5]?.clear ?? 'NoPlay'

const recordLamp = (record: TableSnapshotRecord | undefined): LampWithGrade =>
    record ? { lamp: record.clear, assistGrade: record.assist_grade } : { lamp: 'NoPlay', assistGrade: 0 }

const entryDisplayRank = (entry: TableSnapshotEntry, records: TableRecordMap) => {
    const { lamp, assistGrade } = recordLamp(records[entry.md5])
    return displayLampRank(lamp, assistGrade)
}

const entryRate = (entry: TableSnapshotEntry, records: TableRecordMap | null) => {
    const record = records?.[entry.md5]
    if (!record) return UNPLAYED_RATE
    const maxExScore = getMaxExScore(record.total_notes)
    return maxExScore === 0 ? 0 : record.ex_score / maxExScore
}

const matchesQuery = (entry: TableSnapshotEntry, needle: string) =>
    needle === '' ||
    (MD5_PREFIX_PATTERN.test(needle) && entry.md5.startsWith(needle)) ||
    normaliseTableSearch(entry.title).includes(needle) ||
    normaliseTableSearch(entry.artist).includes(needle)

const matchesLamp = (lamp: ClearLamp, filter: TableLampFilter) => {
    const definition: LampFilterDefinition = TABLE_LAMP_FILTERS.find((candidate) => candidate.value === filter) ?? TABLE_LAMP_FILTERS[0]
    const rank = lampRank(lamp)
    if (definition.min && rank < lampRank(definition.min)) return false
    if (definition.max && rank > lampRank(definition.max)) return false
    return true
}

const sortEntries = (entries: TableSnapshotEntry[], sort: TableSort, records: TableRecordMap | null) => {
    if (sort === 'title') return entries.toSorted((left, right) => TITLE_COLLATOR.compare(left.title, right.title))
    if (sort === 'lamp' && records) return entries.toSorted((left, right) => entryDisplayRank(left, records) - entryDisplayRank(right, records))
    if (sort === 'rate' && records) return entries.toSorted((left, right) => entryRate(left, records) - entryRate(right, records))
    return entries
}

type FilteredTableLevel = {
    level: string
    anchorId: string
    levelEntries: TableSnapshotEntry[]
    entries: TableSnapshotEntry[]
}

/**
 * Applies search, level range, lamp filter and sort to the grouped table.
 * Lamp filters and record sorts are ignored while `records` is null (signed out or still loading).
 */
export const filterTableLevels = (levels: readonly TableSnapshotLevel[], filter: TableFilterState, records: TableRecordMap | null) => {
    const order = levels.map((group) => group.level)
    const fromIndex = filter.from === null ? 0 : order.indexOf(filter.from)
    const toIndex = filter.to === null ? order.length - 1 : order.indexOf(filter.to)
    const needle = normaliseTableSearch(filter.query)
    const isLampActive = records !== null && filter.lamp !== DEFAULT_TABLE_FILTER.lamp
    const isNarrowed = needle !== '' || isLampActive

    return levels.flatMap((group, index): FilteredTableLevel[] => {
        if (index < fromIndex || index > toIndex) return []
        const matched = group.entries.filter(
            (entry) => matchesQuery(entry, needle) && (!isLampActive || matchesLamp(entryLamp(entry, records), filter.lamp)),
        )
        if (isNarrowed && matched.length === 0) return []
        return [
            {
                level: group.level,
                anchorId: levelAnchorId(group.level, index),
                levelEntries: group.entries,
                entries: sortEntries(matched, filter.sort, records),
            },
        ]
    })
}

/** Counts lamps from the best lamp down to NO PLAY, so a stacked bar fills from the left like progress. */
export const countTableLamps = (entries: readonly TableSnapshotEntry[], records: TableRecordMap): LampCount[] => {
    const lamps = entries.map((entry) => recordLamp(records[entry.md5]))
    return DISPLAY_LAMPS.toReversed().map((displayed) => ({
        ...displayed,
        count: lamps.filter((candidate) => isSameDisplayLamp(candidate, displayed)).length,
    }))
}

export const countPlayedEntries = (entries: readonly TableSnapshotEntry[], records: TableRecordMap) =>
    entries.filter((entry) => records[entry.md5] !== undefined).length

/** Summarises a whole table from its record map alone; the server returns records only for this table's md5s. */
export const summariseTableRecords = (records: TableRecordMap, total: number) => {
    const lamps = Object.values(records).map(recordLamp)
    const unplayed = Math.max(total - lamps.length, 0)
    return {
        played: lamps.length,
        lamps: DISPLAY_LAMPS.toReversed().map((displayed) => ({
            ...displayed,
            count: lamps.filter((candidate) => isSameDisplayLamp(candidate, displayed)).length + (displayed.lamp === 'NoPlay' ? unplayed : 0),
        })),
    }
}

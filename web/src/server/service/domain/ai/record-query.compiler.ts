import { z } from 'zod'
import {
    AI_RECORD_QUERY_ALLOWED_OPS,
    AI_RECORD_QUERY_LIMIT,
    type AiRecordQueryField,
    type AiRecordQueryFilter,
    type AiRecordQueryIssue,
    type AiRecordQueryRunInput,
} from '@server/dto/ai'
import { CLEAR_LAMPS, GAUGE_TYPES, INT_COLUMN_MAX, RANDOM_OPTIONS, clearLampToId, gaugeTypeToId } from '@server/dto/common'

type AllowedOp<TField extends AiRecordQueryField> = (typeof AI_RECORD_QUERY_ALLOWED_OPS)[TField][number]

export type RecordQueryTableEntry = { md5: string; level: string; title: string; artist: string }

export type RecordQueryTable = { id: string; symbol: string; levels: readonly string[]; entries: readonly RecordQueryTableEntry[] }

/** One filter after parsing, in the form the database layer turns into bound conditions. Level, table and (for tables) title are not here: they choose the charts. */
export type RecordCondition =
    | { field: 'lamp'; op: AllowedOp<'lamp'>; values: number[] }
    | { field: 'gauge'; op: AllowedOp<'gauge'>; values: number[] }
    | { field: 'random'; op: AllowedOp<'random'>; values: string[] }
    | { field: 'ex_rate'; op: AllowedOp<'ex_rate'>; basisPoints: number }
    | { field: 'bp'; op: AllowedOp<'bp'>; value: number }
    | { field: 'played_at'; op: AllowedOp<'played_at'>; at: number }
    | { field: 'scratch_auto'; op: AllowedOp<'scratch_auto'>; value: boolean }
    | { field: 'title'; op: AllowedOp<'title'>; text: string }

export type RecordQueryScopeEntry = RecordQueryTableEntry & { tableIndex: number; levelIndex: number; levelLabel: string }

export type RecordQuerySort = Pick<AiRecordQueryRunInput, 'sort_field' | 'sort_dir'>

export type CompiledRecordQuery =
    | { source: 'scores'; conditions: RecordCondition[]; sort: RecordQuerySort; limit: number }
    | { source: 'table'; conditions: RecordCondition[]; entries: RecordQueryScopeEntry[]; sort: RecordQuerySort; limit: number }

export type RecordQueryCompileResult = { ok: true; query: CompiledRecordQuery } | { ok: false; issues: AiRecordQueryIssue[] }

/** One EX rate percent in basis points: rates are compared as integers so 99.99 % never rounds into 100 %. */
export const EX_RATE_BASIS_POINTS_PER_PERCENT = 100

const IN_SEPARATOR = ','
const DAY_MS = 86_400_000
const EX_RATE_PATTERN = new RegExp(`^\\d{1,3}(\\.\\d{1,${AI_RECORD_QUERY_LIMIT.EX_RATE_DECIMALS}})?$`)
const PERCENT_SUFFIX = /%$/
const DIGITS_PATTERN = /^\d+$/

/** The lowest lamp id, NO PLAY: the lamp of a chart in a table that has no record yet. */
export const NO_PLAY_LAMP_ID = clearLampToId('NoPlay')

const normaliseText = (value: string) => value.normalize('NFKC').toLowerCase()

const findName = <TName extends string>(names: readonly TName[], value: string) => names.find((name) => name.toLowerCase() === value.toLowerCase())

const splitValues = (op: string, value: string) => (op === 'in' ? value.split(IN_SEPARATOR) : [value]).map((part) => part.trim())

const parseNames = <TName extends string>(names: readonly TName[], op: string, value: string) => {
    const parts = splitValues(op, value)
    if (parts.length > AI_RECORD_QUERY_LIMIT.MAX_IN_VALUES) return null
    const found = parts.map((part) => findName(names, part))
    return found.every((name): name is TName => name !== undefined) ? found : null
}

const exRateSchema = z
    .string()
    .trim()
    .transform((value) => value.replace(PERCENT_SUFFIX, ''))
    .pipe(z.string().regex(EX_RATE_PATTERN))
    .transform(Number)
    .pipe(z.number().min(0).max(AI_RECORD_QUERY_LIMIT.EX_RATE_MAX_PERCENT))
    .transform((percent) => Math.round(percent * EX_RATE_BASIS_POINTS_PER_PERCENT))

const bpSchema = z.string().trim().regex(DIGITS_PATTERN).transform(Number).pipe(z.number().int().max(INT_COLUMN_MAX))

const scratchAutoSchema = z.stringbool({ truthy: ['true'], falsy: ['false'] })

const titleSchema = z.string().trim().min(1).max(AI_RECORD_QUERY_LIMIT.TITLE_MAX_CHARS)

const playedDateSchema = z.iso.date()
const playedDateTimeSchema = z.iso.datetime({ offset: true })

const parsePlayedAt = (op: AllowedOp<'played_at'>, value: string) => {
    const text = value.trim()
    if (playedDateSchema.safeParse(text).success) {
        const startOfDay = Date.parse(`${text}T00:00:00Z`)
        return op === 'gte' ? startOfDay : startOfDay + DAY_MS - 1
    }
    if (!playedDateTimeSchema.safeParse(text).success) return null
    const at = Date.parse(text)
    return Number.isNaN(at) ? null : at
}

const isAllowed = <TField extends AiRecordQueryField>(field: TField, op: string): op is AllowedOp<TField> =>
    AI_RECORD_QUERY_ALLOWED_OPS[field].some((allowed: string) => allowed === op)

const parseCondition = (filter: AiRecordQueryFilter): RecordCondition | null => {
    const { op, value } = filter
    if (filter.field === 'lamp' && isAllowed('lamp', op)) {
        const names = parseNames(CLEAR_LAMPS, op, value)
        return names && { field: 'lamp', op, values: names.map(clearLampToId) }
    }
    if (filter.field === 'gauge' && isAllowed('gauge', op)) {
        const names = parseNames(GAUGE_TYPES, op, value)
        return names && { field: 'gauge', op, values: names.map(gaugeTypeToId) }
    }
    if (filter.field === 'random' && isAllowed('random', op)) {
        const names = parseNames(RANDOM_OPTIONS, op, value)
        return names && { field: 'random', op, values: names }
    }
    if (filter.field === 'ex_rate' && isAllowed('ex_rate', op)) {
        const parsed = exRateSchema.safeParse(value)
        return parsed.success ? { field: 'ex_rate', op, basisPoints: parsed.data } : null
    }
    if (filter.field === 'bp' && isAllowed('bp', op)) {
        const parsed = bpSchema.safeParse(value)
        return parsed.success ? { field: 'bp', op, value: parsed.data } : null
    }
    if (filter.field === 'played_at' && isAllowed('played_at', op)) {
        const at = parsePlayedAt(op, value)
        return at === null ? null : { field: 'played_at', op, at }
    }
    if (filter.field === 'scratch_auto' && isAllowed('scratch_auto', op)) {
        const parsed = scratchAutoSchema.safeParse(value.trim())
        return parsed.success ? { field: 'scratch_auto', op, value: parsed.data } : null
    }
    if (filter.field === 'title' && isAllowed('title', op)) {
        const parsed = titleSchema.safeParse(value)
        return parsed.success ? { field: 'title', op, text: parsed.data } : null
    }
    return null
}

/** Whether a condition on an ordered value (a lamp id) lets `actual` through. */
export const acceptsOrdered = (condition: { op: string; values: readonly number[] }, actual: number) => {
    const [first = Number.NaN] = condition.values
    if (condition.op === 'eq') return actual === first
    if (condition.op === 'ne') return actual !== first
    if (condition.op === 'gte') return actual >= first
    if (condition.op === 'lte') return actual <= first
    return condition.values.includes(actual)
}

/** Whether a chart with no record (NO PLAY, no EX, BP, date or options) passes every condition, so a table query lists it too. */
export const acceptsUnplayed = (conditions: readonly RecordCondition[]) =>
    conditions.every((condition) => condition.field === 'lamp' && acceptsOrdered(condition, NO_PLAY_LAMP_ID))

export const clampRecordQueryLimit = (limit: number) => Math.min(AI_RECORD_QUERY_LIMIT.ROWS_MAX, Math.max(AI_RECORD_QUERY_LIMIT.ROWS_MIN, limit))

type LevelRule = { op: AllowedOp<'level'>; values: string[] }

const stripSymbol = (symbols: readonly string[], value: string) => {
    const symbol = symbols.find((candidate) => candidate !== '' && value.startsWith(candidate))
    return symbol ? value.slice(symbol.length) : value
}

const parseLevelRule = (filter: AiRecordQueryFilter, tables: readonly RecordQueryTable[]): LevelRule | null => {
    const { op } = filter
    if (!isAllowed('level', op)) return null
    const known = new Set(tables.flatMap((table) => table.levels))
    const symbols = tables.map((table) => table.symbol)
    const values = splitValues(op, filter.value).map((part) => stripSymbol(symbols, part))
    if (values.length > AI_RECORD_QUERY_LIMIT.MAX_IN_VALUES || !values.every((value) => known.has(value))) return null
    return { op, values }
}

const levelAccepts = (rule: LevelRule, table: RecordQueryTable, level: string) => {
    const [first = ''] = rule.values
    if (rule.op === 'eq') return level === first
    if (rule.op === 'ne') return level !== first
    if (rule.op === 'in') return rule.values.includes(level)
    const bound = table.levels.indexOf(first)
    const actual = table.levels.indexOf(level)
    if (bound < 0 || actual < 0) return false
    return rule.op === 'gte' ? actual >= bound : actual <= bound
}

/**
 * Checks every filter against the allowed field and operator table and parses its value with the field's own schema. It reports
 * each filter that fails and then runs nothing; otherwise it returns the conditions and, when a table or level filter is present,
 * the table charts the query looks at. The result never names a user: the caller adds the session user's id.
 */
export const compileRecordQuery = (plan: AiRecordQueryRunInput, tables: readonly RecordQueryTable[]): RecordQueryCompileResult => {
    const issues: AiRecordQueryIssue[] = []
    const conditions: RecordCondition[] = []
    const tableIds: string[] = []
    const levelFilters: { index: number; filter: AiRecordQueryFilter }[] = []
    const titleTexts: string[] = []

    for (const [index, filter] of plan.filters.entries()) {
        if (index >= AI_RECORD_QUERY_LIMIT.MAX_FILTERS) {
            issues.push({ index, reason: 'too_many_filters' })
            continue
        }
        if (!isAllowed(filter.field, filter.op)) {
            issues.push({ index, reason: 'op_not_allowed' })
            continue
        }
        if (filter.value.length > AI_RECORD_QUERY_LIMIT.VALUE_MAX_CHARS) {
            issues.push({ index, reason: 'value_invalid' })
            continue
        }
        const table = filter.field === 'table' ? tables.find((candidate) => candidate.id === filter.value.trim().toLowerCase()) : undefined
        if (table) tableIds.push(table.id)
        if (filter.field === 'level') levelFilters.push({ index, filter })
        const condition = parseCondition(filter)
        if (condition) conditions.push(condition)
        if (condition?.field === 'title') titleTexts.push(condition.text)
        const isParsed = filter.field === 'level' || table !== undefined || condition !== null
        if (!isParsed) issues.push({ index, reason: 'value_invalid' })
    }

    const targetTables = tables.filter((table) => tableIds.every((id) => id === table.id))
    const levelRules = levelFilters.flatMap(({ index, filter }) => {
        const rule = parseLevelRule(filter, targetTables)
        if (rule) return [rule]
        issues.push({ index, reason: 'value_invalid' })
        return []
    })

    if (issues.length > 0) return { ok: false, issues: issues.toSorted((left, right) => left.index - right.index) }

    const sort = { sort_field: plan.sort_field, sort_dir: plan.sort_dir }
    const limit = clampRecordQueryLimit(plan.limit)
    if (tableIds.length === 0 && levelFilters.length === 0) return { ok: true, query: { source: 'scores', conditions, sort, limit } }

    const needles = titleTexts.map(normaliseText)
    const entries = tables.flatMap((table, tableIndex) =>
        targetTables.includes(table)
            ? table.entries
                  .filter((entry) => levelRules.every((rule) => levelAccepts(rule, table, entry.level)))
                  .filter((entry) => needles.every((needle) => normaliseText(entry.title).includes(needle)))
                  .map((entry) => ({
                      ...entry,
                      tableIndex,
                      levelIndex: table.levels.indexOf(entry.level),
                      levelLabel: `${table.symbol}${entry.level}`,
                  }))
            : [],
    )
    return {
        ok: true,
        query: { source: 'table', conditions: conditions.filter((condition) => condition.field !== 'title'), entries, sort, limit },
    }
}

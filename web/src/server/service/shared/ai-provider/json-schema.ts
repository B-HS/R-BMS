import { z } from 'zod'

export type JsonSchemaValue = string | number | boolean | null | JsonSchemaValue[] | { [key: string]: JsonSchemaValue }

export type ProviderJsonSchema = { [key: string]: JsonSchemaValue }

export const AI_SCHEMA_LIMITS = {
    MAX_DEPTH: 10,
    MAX_UNION_COUNT: 16,
    MAX_PROPERTY_COUNT: 5000,
    MAX_ENUM_VALUE_COUNT: 1000,
} as const

const ROOT_PATH = '$'
const ROOT_DEPTH = 1
const MAX_MIN_ITEMS = 1
const MIN_UNION_BRANCHES = 2
const JSON_SCHEMA_DRAFT = 'draft-2020-12'

const ALLOWED_TYPES = new Set(['object', 'array', 'string', 'number', 'integer', 'boolean'])
const ALLOWED_FORMATS = new Set(['date-time', 'time', 'date', 'duration', 'email', 'hostname', 'ipv4', 'ipv6', 'uuid'])
const ALLOWED_KEYWORDS = new Set([
    'type',
    'properties',
    'required',
    'additionalProperties',
    'items',
    'anyOf',
    'enum',
    'description',
    'format',
    'minItems',
])
const STRIPPED_KEYWORDS = new Set([
    '$schema',
    'title',
    'default',
    'examples',
    'minimum',
    'maximum',
    'exclusiveMinimum',
    'exclusiveMaximum',
    'multipleOf',
    'minLength',
    'maxLength',
    'maxItems',
    'uniqueItems',
    'pattern',
])
const SCHEMA_LIST_KEYWORDS = new Set(['anyOf', 'allOf', 'prefixItems'])
const SCHEMA_KEYWORDS = new Set(['items', 'additionalProperties', 'not', 'propertyNames', 'contains', 'if', 'then', 'else'])

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)

const isProviderJsonSchema = (value: JsonSchemaValue): value is ProviderJsonSchema =>
    typeof value === 'object' && value !== null && !Array.isArray(value)

const toJsonValue = (value: unknown): JsonSchemaValue => {
    if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return value
    if (Array.isArray(value)) return value.map(toJsonValue)
    if (isRecord(value)) return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, toJsonValue(child)]))
    return null
}

const normalizeTypeArray = (node: ProviderJsonSchema): ProviderJsonSchema => {
    const { type, ...rest } = node
    if (!Array.isArray(type) || type.includes('null')) return node
    if (type.length === 1) return { ...rest, type: type[0] }
    return { ...rest, anyOf: type.map((member) => ({ type: member })) }
}

const sanitizeSchemaNode = (node: unknown): JsonSchemaValue => {
    if (!isRecord(node)) return toJsonValue(node)

    const sanitized = Object.fromEntries(
        Object.entries(node).flatMap(([key, value]): [string, JsonSchemaValue][] => {
            if (STRIPPED_KEYWORDS.has(key)) return []
            if (key === 'format' && !ALLOWED_FORMATS.has(String(value))) return []
            if (key === 'minItems' && typeof value === 'number' && value > MAX_MIN_ITEMS) return []
            if (key === 'const') return [['enum', [toJsonValue(value)]]]
            if (key === 'oneOf' && Array.isArray(value)) return [['anyOf', value.map(sanitizeSchemaNode)]]
            if (key === 'properties' && isRecord(value)) {
                return [[key, Object.fromEntries(Object.entries(value).map(([name, child]) => [name, sanitizeSchemaNode(child)]))]]
            }
            if (SCHEMA_LIST_KEYWORDS.has(key) && Array.isArray(value)) return [[key, value.map(sanitizeSchemaNode)]]
            if (SCHEMA_KEYWORDS.has(key)) return [[key, sanitizeSchemaNode(value)]]
            return [[key, toJsonValue(value)]]
        }),
    )

    const normalized = normalizeTypeArray(sanitized)
    if (!isRecord(node.properties)) return normalized
    return {
        ...normalized,
        additionalProperties: 'additionalProperties' in sanitized ? sanitized.additionalProperties : false,
        required: Object.keys(node.properties),
    }
}

type SchemaNodeEntry = { path: string; node: unknown; depth: number }

const collectSchemaNodes = (node: unknown, path: string, depth: number): SchemaNodeEntry[] => {
    if (!isRecord(node)) return [{ path, node, depth }]
    const children = [
        ...(isRecord(node.properties) ? Object.entries(node.properties).map(([name, child]) => ({ path: `${path}.${name}`, child })) : []),
        ...(node.items === undefined ? [] : [{ path: `${path}[]`, child: node.items }]),
        ...(Array.isArray(node.anyOf) ? node.anyOf.map((child, index) => ({ path: `${path}|${index}`, child })) : []),
    ]
    return [{ path, node, depth }, ...children.flatMap((entry) => collectSchemaNodes(entry.child, entry.path, depth + 1))]
}

const hasSameKeys = (required: unknown, properties: Record<string, unknown>) => {
    if (!Array.isArray(required)) return false
    const propertyNames = Object.keys(properties)
    return required.length === propertyNames.length && propertyNames.every((name) => required.includes(name))
}

const listNodeViolations = ({ path, node, depth }: SchemaNodeEntry) => {
    if (!isRecord(node)) return [`${path}: schema must be an object`]

    const { type, anyOf, items, properties, required, additionalProperties, minItems, format } = node
    const isObjectType = type === 'object'
    const unionViolations = Array.isArray(anyOf)
        ? [
              ...(anyOf.length < MIN_UNION_BRANCHES ? [`${path}: anyOf needs at least ${MIN_UNION_BRANCHES} branches`] : []),
              ...(anyOf.some((branch) => isRecord(branch) && branch.type === 'null') ? [`${path}: nullable unions are not allowed`] : []),
          ]
        : []
    const typeViolations = (() => {
        if (anyOf !== undefined) return []
        if (Array.isArray(type)) return [`${path}: type arrays (nullable unions) are not allowed`]
        if (typeof type !== 'string') return [`${path}: schema needs a type or anyOf`]
        if (!ALLOWED_TYPES.has(type)) return [`${path}: type "${type}" is not in the provider common subset`]
        return []
    })()

    return [
        ...Object.keys(node)
            .filter((keyword) => !ALLOWED_KEYWORDS.has(keyword))
            .map((keyword) => `${path}: keyword "${keyword}" is not in the provider common subset`),
        ...unionViolations,
        ...typeViolations,
        ...(depth === ROOT_DEPTH && (!isObjectType || anyOf !== undefined) ? [`${path}: root must be an object without anyOf`] : []),
        ...(isObjectType && !isRecord(properties) ? [`${path}: object needs properties`] : []),
        ...(isObjectType && isRecord(properties) && !hasSameKeys(required, properties) ? [`${path}: every property must be required`] : []),
        ...(isObjectType && additionalProperties !== false ? [`${path}: object needs additionalProperties false`] : []),
        ...(!isObjectType && (properties !== undefined || required !== undefined || additionalProperties !== undefined)
            ? [`${path}: properties, required and additionalProperties belong to objects only`]
            : []),
        ...(type === 'array' && !isRecord(items) ? [`${path}: array needs a single items schema`] : []),
        ...(minItems !== undefined && minItems !== 0 && minItems !== MAX_MIN_ITEMS ? [`${path}: minItems must be 0 or ${MAX_MIN_ITEMS}`] : []),
        ...(format !== undefined && !ALLOWED_FORMATS.has(String(format))
            ? [`${path}: format "${String(format)}" is not in the provider common subset`]
            : []),
        ...(node.enum !== undefined &&
        !(
            Array.isArray(node.enum) &&
            node.enum.length > 0 &&
            node.enum.every((value) => typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean')
        )
            ? [`${path}: enum must be a non-empty list of strings, numbers or booleans`]
            : []),
    ]
}

/**
 * Counts what the strictest provider limits are measured on: nesting depth (root is 1),
 * `anyOf` nodes, object properties and enum values.
 */
export const measureJsonSchema = (jsonSchema: unknown) => {
    const entries = collectSchemaNodes(jsonSchema, ROOT_PATH, ROOT_DEPTH)
    return {
        depth: Math.max(...entries.map((entry) => entry.depth)),
        unionCount: entries.filter((entry) => isRecord(entry.node) && Array.isArray(entry.node.anyOf)).length,
        propertyCount: entries.reduce(
            (sum, entry) => sum + (isRecord(entry.node) && isRecord(entry.node.properties) ? Object.keys(entry.node.properties).length : 0),
            0,
        ),
        enumValueCount: entries.reduce(
            (sum, entry) => sum + (isRecord(entry.node) && Array.isArray(entry.node.enum) ? entry.node.enum.length : 0),
            0,
        ),
    }
}

/**
 * Lists every way a JSON Schema leaves the subset that Anthropic and OpenAI strict mode both accept.
 * An empty list means the schema can be sent to every provider.
 */
export const listJsonSchemaViolations = (jsonSchema: unknown) => {
    const measures = measureJsonSchema(jsonSchema)
    return [
        ...collectSchemaNodes(jsonSchema, ROOT_PATH, ROOT_DEPTH).flatMap(listNodeViolations),
        ...(measures.depth > AI_SCHEMA_LIMITS.MAX_DEPTH ? [`depth ${measures.depth} exceeds ${AI_SCHEMA_LIMITS.MAX_DEPTH}`] : []),
        ...(measures.unionCount > AI_SCHEMA_LIMITS.MAX_UNION_COUNT
            ? [`${measures.unionCount} unions exceed ${AI_SCHEMA_LIMITS.MAX_UNION_COUNT}`]
            : []),
        ...(measures.propertyCount > AI_SCHEMA_LIMITS.MAX_PROPERTY_COUNT
            ? [`${measures.propertyCount} properties exceed ${AI_SCHEMA_LIMITS.MAX_PROPERTY_COUNT}`]
            : []),
        ...(measures.enumValueCount > AI_SCHEMA_LIMITS.MAX_ENUM_VALUE_COUNT
            ? [`${measures.enumValueCount} enum values exceed ${AI_SCHEMA_LIMITS.MAX_ENUM_VALUE_COUNT}`]
            : []),
    ]
}

/**
 * Converts a Zod schema into the JSON Schema subset shared by every provider. Constraint keywords the
 * providers reject (lengths, ranges, patterns, maxItems) are dropped here and must be enforced by
 * re-validating the response with the original Zod schema.
 */
export const buildProviderJsonSchema = (schema: z.ZodType) => {
    try {
        const jsonSchema = sanitizeSchemaNode(
            z.toJSONSchema(schema, { io: 'input', target: JSON_SCHEMA_DRAFT, cycles: 'throw', reused: 'inline', unrepresentable: 'throw' }),
        )
        const violations = listJsonSchemaViolations(jsonSchema)
        if (violations.length > 0 || !isProviderJsonSchema(jsonSchema)) return { ok: false as const, violations }
        return { ok: true as const, jsonSchema }
    } catch (error) {
        return {
            ok: false as const,
            violations: [`schema cannot be represented as JSON Schema: ${error instanceof Error ? error.message : 'unknown error'}`],
        }
    }
}

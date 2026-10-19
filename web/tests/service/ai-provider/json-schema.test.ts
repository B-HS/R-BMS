import { describe, expect, test } from 'bun:test'
import { z } from 'zod'
import {
    AI_SCHEMA_LIMITS,
    buildProviderJsonSchema,
    listJsonSchemaViolations,
    measureJsonSchema,
    type ProviderJsonSchema,
} from '@server/service/shared/ai-provider/json-schema'

const MD5_LENGTH = 32
const UNION_FIELD_COUNT_OVER_LIMIT = AI_SCHEMA_LIMITS.MAX_UNION_COUNT + 1

const buildOk = (schema: z.ZodType) => {
    const result = buildProviderJsonSchema(schema)
    if (!result.ok) throw new Error(`expected a convertible schema: ${result.violations.join('; ')}`)
    return result.jsonSchema
}

const buildViolations = (schema: z.ZodType) => {
    const result = buildProviderJsonSchema(schema)
    return result.ok ? [] : result.violations
}

const nestObjects = (levels: number): z.ZodType => (levels === 0 ? z.string() : z.object({ child: nestObjects(levels - 1) }))

const unionFields = (count: number) =>
    Object.fromEntries(Array.from({ length: count }, (_, index) => [`field${index}`, z.union([z.string(), z.number()])]))

const objectNode = (properties: Record<string, ProviderJsonSchema>): ProviderJsonSchema => ({
    type: 'object',
    properties,
    required: Object.keys(properties),
    additionalProperties: false,
})

const featureSchema = z.object({
    headline: z.string().min(1).max(80),
    findings: z
        .array(
            z.object({
                kind: z.enum(['timing', 'stamina', 'reading']),
                severity: z.number().int().min(1).max(5),
                evidence: z.array(z.string().max(60)).max(3),
                detail: z.discriminatedUnion('type', [
                    z.object({ type: z.literal('window'), fromMs: z.number(), toMs: z.number() }),
                    z.object({ type: z.literal('note'), text: z.string() }),
                ]),
            }),
        )
        .min(1)
        .max(5),
    next: z.object({ md5: z.string().length(MD5_LENGTH), reason: z.string().optional() }),
})

describe('buildProviderJsonSchema 변환', () => {
    test('제약 키워드를 지우고 모든 속성을 required 로, 객체마다 additionalProperties false 로 만든다', () => {
        const schema = z.object({
            name: z.string().min(1).max(10),
            count: z.number().int().min(0).max(100),
            kind: z.literal('a'),
            tags: z.array(z.string()).min(2).max(5),
            flag: z.boolean().default(true),
            note: z.string().optional(),
            email: z.email(),
            at: z.iso.datetime(),
            code: z.string().regex(/^[a-z]+$/),
            described: z.string().describe('what it is'),
        })

        expect(buildOk(schema)).toEqual({
            type: 'object',
            properties: {
                name: { type: 'string' },
                count: { type: 'integer' },
                kind: { type: 'string', enum: ['a'] },
                tags: { type: 'array', items: { type: 'string' } },
                flag: { type: 'boolean' },
                note: { type: 'string' },
                email: { type: 'string', format: 'email' },
                at: { type: 'string', format: 'date-time' },
                code: { type: 'string' },
                described: { type: 'string', description: 'what it is' },
            },
            required: ['name', 'count', 'kind', 'tags', 'flag', 'note', 'email', 'at', 'code', 'described'],
            additionalProperties: false,
        })
    })

    test('minItems 1 은 두고 2 이상은 지운다', () => {
        const jsonSchema = buildOk(z.object({ one: z.array(z.string()).min(1), two: z.array(z.string()).min(2) }))

        expect(jsonSchema.properties).toEqual({
            one: { type: 'array', items: { type: 'string' }, minItems: 1 },
            two: { type: 'array', items: { type: 'string' } },
        })
    })

    test('discriminatedUnion 의 oneOf 는 anyOf 로 바꾼다', () => {
        const jsonSchema = buildOk(
            z.object({ detail: z.discriminatedUnion('type', [z.object({ type: z.literal('a') }), z.object({ type: z.literal('b') })]) }),
        )

        expect(JSON.stringify(jsonSchema)).toContain('"anyOf"')
        expect(JSON.stringify(jsonSchema)).not.toContain('"oneOf"')
        expect(JSON.stringify(jsonSchema)).not.toContain('"const"')
    })

    test('원시 타입 union 의 type 배열은 anyOf 로 바꾼다', () => {
        const jsonSchema = buildOk(z.object({ value: z.union([z.string(), z.number()]) }))

        expect(jsonSchema.properties).toEqual({ value: { anyOf: [{ type: 'string' }, { type: 'number' }] } })
        expect(measureJsonSchema(jsonSchema).unionCount).toBe(1)
    })

    test('속성 이름이 키워드와 같아도 지우지 않는다', () => {
        const jsonSchema = buildOk(z.object({ default: z.string(), pattern: z.string(), minimum: z.number(), title: z.string() }))

        expect(Object.keys(jsonSchema.properties as object)).toEqual(['default', 'pattern', 'minimum', 'title'])
        expect(jsonSchema.required).toEqual(['default', 'pattern', 'minimum', 'title'])
    })

    test('transform 은 입력 쪽 모양으로 변환한다', () => {
        const jsonSchema = buildOk(z.object({ count: z.string().transform(Number) }))

        expect(jsonSchema.properties).toEqual({ count: { type: 'string' } })
    })

    test('기능 스키마 같은 중첩 구조는 공통 부분집합만 쓴다', () => {
        const jsonSchema = buildOk(featureSchema)

        expect(listJsonSchemaViolations(jsonSchema)).toEqual([])
        expect(JSON.stringify(jsonSchema)).not.toMatch(
            /"(minLength|maxLength|minimum|maximum|maxItems|pattern|const|oneOf|allOf|\$ref|\$defs|default|\$schema)"/,
        )
        expect(measureJsonSchema(jsonSchema)).toMatchObject({ unionCount: 1, enumValueCount: 5 })
    })

    test('변환 결과는 Zod 스키마와 같은 모양의 데이터를 받아들인다', () => {
        const valid = {
            headline: 'ok',
            findings: [{ kind: 'timing', severity: 3, evidence: ['a'], detail: { type: 'note', text: 'x' } }],
            next: { md5: 'a'.repeat(MD5_LENGTH), reason: 'because' },
        }

        expect(featureSchema.safeParse(valid).success).toBe(true)
    })
})

describe('buildProviderJsonSchema 거절', () => {
    test.each([
        ['nullable', z.object({ a: z.string().nullable() }), 'nullable'],
        ['null 허용 union', z.object({ a: z.union([z.string(), z.null()]) }), 'nullable'],
        ['record', z.object({ a: z.record(z.string(), z.number()) }), 'additionalProperties'],
        ['tuple', z.object({ a: z.tuple([z.string(), z.number()]) }), 'prefixItems'],
        ['any', z.object({ a: z.any() }), 'type'],
        ['루트가 object 가 아님', z.array(z.object({ a: z.string() })), 'root'],
        ['루트 union', z.discriminatedUnion('t', [z.object({ t: z.literal('a') }), z.object({ t: z.literal('b') })]), 'root'],
        ['루트 nullable', z.object({ a: z.string() }).nullable(), 'nullable'],
    ])('%s 는 거절한다', (_label, schema, expectedFragment) => {
        const violations = buildViolations(schema)

        expect(violations.length).toBeGreaterThan(0)
        expect(violations.join('\n')).toContain(expectedFragment)
    })

    test('재귀 스키마는 거절한다', () => {
        type Node = { value: string; next?: Node }
        const recursive: z.ZodType<Node> = z.object({ value: z.string(), next: z.lazy(() => recursive).optional() })

        expect(buildViolations(recursive).join('\n')).toContain('cannot be represented')
    })

    test('JSON Schema 로 나타낼 수 없는 타입은 거절한다', () => {
        expect(buildViolations(z.object({ at: z.date() })).join('\n')).toContain('cannot be represented')
    })

    test('중첩이 한도를 넘으면 거절하고 한도 안은 받는다', () => {
        const atLimit = AI_SCHEMA_LIMITS.MAX_DEPTH - 1

        expect(buildViolations(nestObjects(atLimit))).toEqual([])
        expect(buildViolations(nestObjects(atLimit + 1)).join('\n')).toContain('depth')
    })

    test('union 이 한도를 넘으면 거절하고 한도 안은 받는다', () => {
        expect(buildViolations(z.object(unionFields(AI_SCHEMA_LIMITS.MAX_UNION_COUNT)))).toEqual([])
        expect(buildViolations(z.object(unionFields(UNION_FIELD_COUNT_OVER_LIMIT))).join('\n')).toContain('unions')
    })
})

describe('listJsonSchemaViolations', () => {
    test('부분집합 안의 손으로 쓴 스키마는 위반이 없다', () => {
        const schema = objectNode({
            a: { type: 'string', enum: ['x', 'y'], description: 'a' },
            b: { type: 'array', items: { type: 'integer' }, minItems: 0 },
            c: { anyOf: [{ type: 'string' }, { type: 'number' }] },
            d: { type: 'string', format: 'uuid' },
        })

        expect(listJsonSchemaViolations(schema)).toEqual([])
    })

    test.each([
        ['allOf', objectNode({ a: { allOf: [{ type: 'string' }] } }), 'allOf'],
        ['$ref', objectNode({ a: { $ref: '#/$defs/x' } }), '$ref'],
        ['const', objectNode({ a: { type: 'string', const: 'x' } }), 'const'],
        ['maxLength', objectNode({ a: { type: 'string', maxLength: 3 } }), 'maxLength'],
        ['정규식', objectNode({ a: { type: 'string', pattern: '^a$' } }), 'pattern'],
        ['type 배열', objectNode({ a: { type: ['string', 'null'] } }), 'type arrays'],
        ['null 타입', objectNode({ a: { type: 'null' } }), 'null'],
        ['null 가지가 있는 anyOf', objectNode({ a: { anyOf: [{ type: 'string' }, { type: 'null' }] } }), 'nullable'],
        ['가지 하나짜리 anyOf', objectNode({ a: { anyOf: [{ type: 'string' }] } }), 'anyOf needs'],
        ['타입 없음', objectNode({ a: {} }), 'type or anyOf'],
        ['목록 형태 items', objectNode({ a: { type: 'array', items: [{ type: 'string' }] } }), 'items'],
        ['items 없는 배열', objectNode({ a: { type: 'array' } }), 'items'],
        ['minItems 2', objectNode({ a: { type: 'array', items: { type: 'string' }, minItems: 2 } }), 'minItems'],
        ['허용 목록 밖 format', objectNode({ a: { type: 'string', format: 'uri' } }), 'format'],
        ['빈 enum', objectNode({ a: { type: 'string', enum: [] } }), 'enum'],
        ['객체 enum', objectNode({ a: { type: 'string', enum: [{ x: 1 }] } }), 'enum'],
        ['additionalProperties true', { ...objectNode({ a: { type: 'string' } }), additionalProperties: true }, 'additionalProperties'],
        ['required 누락', { ...objectNode({ a: { type: 'string' }, b: { type: 'string' } }), required: ['a'] }, 'required'],
        ['문자열에 properties', objectNode({ a: { type: 'string', properties: {} } }), 'belong to objects'],
        ['true 스키마', objectNode({ a: true as unknown as ProviderJsonSchema }), 'schema must be an object'],
    ])('%s 는 위반이다', (_label, schema, expectedFragment) => {
        const violations = listJsonSchemaViolations(schema)

        expect(violations.length).toBeGreaterThan(0)
        expect(violations.join('\n')).toContain(expectedFragment)
    })

    test('속성 수와 enum 값 수 한도를 넘으면 위반이다', () => {
        const manyProperties = objectNode(
            Object.fromEntries(Array.from({ length: AI_SCHEMA_LIMITS.MAX_PROPERTY_COUNT + 1 }, (_, index) => [`p${index}`, { type: 'string' }])),
        )
        const manyEnumValues = objectNode({
            e: { type: 'integer', enum: Array.from({ length: AI_SCHEMA_LIMITS.MAX_ENUM_VALUE_COUNT + 1 }, (_, index) => index) },
        })

        expect(listJsonSchemaViolations(manyProperties).join('\n')).toContain('properties exceed')
        expect(listJsonSchemaViolations(manyEnumValues).join('\n')).toContain('enum values exceed')
    })
})

describe('measureJsonSchema', () => {
    test('깊이는 루트를 1 로 세고 union·속성·enum 값을 센다', () => {
        const schema = objectNode({
            list: { type: 'array', items: objectNode({ kind: { type: 'string', enum: ['a', 'b', 'c'] } }) },
            either: { anyOf: [{ type: 'string' }, { type: 'number' }] },
        })

        expect(measureJsonSchema(schema)).toEqual({ depth: 4, unionCount: 1, propertyCount: 3, enumValueCount: 3 })
    })
})

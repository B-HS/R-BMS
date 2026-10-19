import { describe, expect, test } from 'bun:test'
import {
    AI_RECORD_QUERY_ALLOWED_OPS,
    AI_RECORD_QUERY_FIELDS,
    AI_RECORD_QUERY_OPS,
    type AiRecordQueryFilter,
    type AiRecordQueryRunInput,
} from '@server/dto/ai'
import { clearLampToId, gaugeTypeToId } from '@server/dto/common'
import { acceptsUnplayed, compileRecordQuery, type RecordQueryTable } from '@server/service/domain/ai/record-query.compiler'

const TABLES: RecordQueryTable[] = [
    {
        id: 'insane1',
        symbol: '★',
        levels: ['1', '2', '12', '13', '???'],
        entries: [
            { md5: 'a'.repeat(32), level: '1', title: 'Alpha', artist: 'A' },
            { md5: 'b'.repeat(32), level: '12', title: 'Ｂｒａｖｏ Song', artist: 'B' },
            { md5: 'c'.repeat(32), level: '13', title: 'Charlie', artist: 'C' },
            { md5: 'd'.repeat(32), level: '???', title: 'Delta', artist: 'D' },
        ],
    },
]

const VALID_VALUE = {
    level: '12',
    lamp: 'Hard',
    ex_rate: '90',
    bp: '10',
    played_at: '2026-10-01',
    random: 'Random',
    gauge: 'Hard',
    scratch_auto: 'false',
    title: 'Alpha',
    table: 'insane1',
} satisfies Record<AiRecordQueryFilter['field'], string>

const plan = (filters: AiRecordQueryFilter[], overrides: Partial<AiRecordQueryRunInput> = {}): AiRecordQueryRunInput => ({
    filters,
    sort_field: 'played_at',
    sort_dir: 'desc',
    limit: 20,
    ...overrides,
})

const compileOne = (filter: AiRecordQueryFilter) => compileRecordQuery(plan([filter]), TABLES)

describe('기록 질문 필터 컴파일러의 허용 표', () => {
    test('항목마다 허용된 연산자만 통과하고 나머지는 op_not_allowed 로 실행하지 않는다', () => {
        for (const field of AI_RECORD_QUERY_FIELDS) {
            const allowed: readonly string[] = AI_RECORD_QUERY_ALLOWED_OPS[field]
            for (const op of AI_RECORD_QUERY_OPS) {
                const value = op === 'in' ? `${VALID_VALUE[field]},${VALID_VALUE[field]}` : VALID_VALUE[field]
                const result = compileOne({ field, op, value })
                if (allowed.includes(op)) expect({ field, op, ok: result.ok }).toEqual({ field, op, ok: true })
                else expect({ field, op, result }).toEqual({ field, op, result: { ok: false, issues: [{ index: 0, reason: 'op_not_allowed' }] } })
            }
        }
    })

    test('제목은 포함(contains)만, 친 날짜는 이상·이하만, SCRATCH AUTO 는 같음만 쓴다', () => {
        expect(AI_RECORD_QUERY_ALLOWED_OPS.title).toEqual(['contains'])
        expect(AI_RECORD_QUERY_ALLOWED_OPS.played_at).toEqual(['gte', 'lte'])
        expect(AI_RECORD_QUERY_ALLOWED_OPS.scratch_auto).toEqual(['eq'])
    })

    test('여러 필터 중 틀린 것만 순서대로 알려 주고 하나라도 틀리면 조건을 돌려주지 않는다', () => {
        const result = compileRecordQuery(
            plan([
                { field: 'lamp', op: 'gte', value: 'Hard' },
                { field: 'bp', op: 'in', value: '1,2' },
                { field: 'ex_rate', op: 'gte', value: '101' },
            ]),
            TABLES,
        )
        expect(result).toEqual({
            ok: false,
            issues: [
                { index: 1, reason: 'op_not_allowed' },
                { index: 2, reason: 'value_invalid' },
            ],
        })
    })

    test('필터가 여덟 개를 넘으면 넘친 것마다 too_many_filters 다', () => {
        const filters = Array.from({ length: 10 }, () => ({ field: 'bp' as const, op: 'gte' as const, value: '1' }))
        const result = compileRecordQuery(plan(filters), TABLES)
        expect(result).toEqual({
            ok: false,
            issues: [
                { index: 8, reason: 'too_many_filters' },
                { index: 9, reason: 'too_many_filters' },
            ],
        })
    })

    test('limit 은 거절하지 않고 1~100 으로 자른다', () => {
        for (const [limit, expected] of [
            [0, 1],
            [-5, 1],
            [50, 50],
            [500, 100],
        ]) {
            const result = compileRecordQuery(plan([], { limit }), TABLES)
            expect(result.ok && result.query.limit).toBe(expected)
        }
    })
})

describe('값 파싱', () => {
    const parsed = (filter: AiRecordQueryFilter) => {
        const result = compileOne(filter)
        return result.ok ? result.query.conditions : result.issues
    }

    test('램프·게이지·배치 옵션은 정해진 이름만 받고 대소문자는 가리지 않으며 in 은 쉼표로 나눈다', () => {
        expect(parsed({ field: 'lamp', op: 'gte', value: 'hard' })).toEqual([{ field: 'lamp', op: 'gte', values: [clearLampToId('Hard')] }])
        expect(parsed({ field: 'lamp', op: 'in', value: 'Easy, Normal' })).toEqual([
            { field: 'lamp', op: 'in', values: [clearLampToId('Easy'), clearLampToId('Normal')] },
        ])
        expect(parsed({ field: 'gauge', op: 'eq', value: 'EXHARD' })).toEqual([{ field: 'gauge', op: 'eq', values: [gaugeTypeToId('ExHard')] }])
        expect(parsed({ field: 'random', op: 'ne', value: 'off' })).toEqual([{ field: 'random', op: 'ne', values: ['Off'] }])
        for (const filter of [
            { field: 'lamp', op: 'eq', value: 'HARD CLEAR' },
            { field: 'lamp', op: 'in', value: 'Easy,' },
            { field: 'gauge', op: 'eq', value: '3' },
            { field: 'random', op: 'eq', value: 'R-RANDOM' },
            { field: 'lamp', op: 'in', value: Array.from({ length: 21 }, () => 'Easy').join(',') },
        ] satisfies AiRecordQueryFilter[]) {
            expect(parsed(filter)).toEqual([{ index: 0, reason: 'value_invalid' }])
        }
    })

    test('EX 비율은 0~100 의 소수 둘째 자리까지를 만분율 정수로 바꾼다', () => {
        expect(parsed({ field: 'ex_rate', op: 'gte', value: '85.5' })).toEqual([{ field: 'ex_rate', op: 'gte', basisPoints: 8550 }])
        expect(parsed({ field: 'ex_rate', op: 'lte', value: '99.99%' })).toEqual([{ field: 'ex_rate', op: 'lte', basisPoints: 9999 }])
        for (const value of ['100.01', '-1', '90.123', 'ninety', '', '1e2']) {
            expect(parsed({ field: 'ex_rate', op: 'gte', value })).toEqual([{ index: 0, reason: 'value_invalid' }])
        }
    })

    test('BP 는 0 이상의 정수만 받는다', () => {
        expect(parsed({ field: 'bp', op: 'lte', value: '0' })).toEqual([{ field: 'bp', op: 'lte', value: 0 }])
        for (const value of ['-1', '1.5', '99999999999', 'abc']) {
            expect(parsed({ field: 'bp', op: 'lte', value })).toEqual([{ index: 0, reason: 'value_invalid' }])
        }
    })

    test('친 날짜는 UTC 날짜면 이상은 그날 시작, 이하는 그날 끝이고 오프셋 있는 ISO 시각도 받는다', () => {
        const start = Date.UTC(2026, 9, 1)
        expect(parsed({ field: 'played_at', op: 'gte', value: '2026-10-01' })).toEqual([{ field: 'played_at', op: 'gte', at: start }])
        expect(parsed({ field: 'played_at', op: 'lte', value: '2026-10-01' })).toEqual([
            { field: 'played_at', op: 'lte', at: Date.UTC(2026, 9, 2) - 1 },
        ])
        expect(parsed({ field: 'played_at', op: 'gte', value: '2026-10-01T09:00:00+09:00' })).toEqual([{ field: 'played_at', op: 'gte', at: start }])
        for (const value of ['2026-13-01', '2026/10/01', '어제', '2026-10-01T09:00:00']) {
            expect(parsed({ field: 'played_at', op: 'gte', value })).toEqual([{ index: 0, reason: 'value_invalid' }])
        }
    })

    test('SCRATCH AUTO 는 true·false 만, 제목은 1~100자만 받는다', () => {
        expect(parsed({ field: 'scratch_auto', op: 'eq', value: 'true' })).toEqual([{ field: 'scratch_auto', op: 'eq', value: true }])
        expect(parsed({ field: 'scratch_auto', op: 'eq', value: 'yes' })).toEqual([{ index: 0, reason: 'value_invalid' }])
        expect(parsed({ field: 'title', op: 'contains', value: '  ' })).toEqual([{ index: 0, reason: 'value_invalid' }])
        expect(parsed({ field: 'title', op: 'contains', value: 'x'.repeat(101) })).toEqual([{ index: 0, reason: 'value_invalid' }])
        expect(parsed({ field: 'title', op: 'contains', value: '%_\\' })).toEqual([{ field: 'title', op: 'contains', text: '%_\\' }])
    })

    test('값이 200자를 넘으면 파싱하지 않고 value_invalid 다', () => {
        expect(parsed({ field: 'random', op: 'in', value: 'Off,'.repeat(60) })).toEqual([{ index: 0, reason: 'value_invalid' }])
    })
})

describe('난이도표 범위', () => {
    const tableQuery = (filters: AiRecordQueryFilter[]) => {
        const result = compileRecordQuery(plan(filters), TABLES)
        expect(result.ok && result.query.source).toBe('table')
        return result.ok && result.query.source === 'table' ? result.query : { entries: [], conditions: [] }
    }

    test('레벨·표 필터가 없으면 모든 판 쿼리이고 있으면 표 쿼리다', () => {
        const scores = compileRecordQuery(plan([{ field: 'lamp', op: 'gte', value: 'Hard' }]), TABLES)
        expect(scores.ok && scores.query.source).toBe('scores')
        expect(tableQuery([{ field: 'table', op: 'eq', value: 'insane1' }]).entries).toHaveLength(4)
    })

    test('레벨은 표의 레벨 이름만 받고 기호(★)는 떼며 이상·이하는 표의 레벨 순서를 따른다', () => {
        const md5sOf = (filters: AiRecordQueryFilter[]) => tableQuery(filters).entries.map((entry) => entry.md5[0])
        expect(md5sOf([{ field: 'level', op: 'eq', value: '★12' }])).toEqual(['b'])
        expect(md5sOf([{ field: 'level', op: 'gte', value: '12' }])).toEqual(['b', 'c', 'd'])
        expect(md5sOf([{ field: 'level', op: 'lte', value: '12' }])).toEqual(['a', 'b'])
        expect(md5sOf([{ field: 'level', op: 'in', value: '1, ★13' }])).toEqual(['a', 'c'])
        expect(md5sOf([{ field: 'level', op: 'ne', value: '???' }])).toEqual(['a', 'b', 'c'])
        expect(tableQuery([{ field: 'level', op: 'eq', value: '12' }]).entries[0]).toMatchObject({ levelLabel: '★12', tableIndex: 0, levelIndex: 2 })
        expect(compileOne({ field: 'level', op: 'eq', value: '99' })).toEqual({ ok: false, issues: [{ index: 0, reason: 'value_invalid' }] })
        expect(compileOne({ field: 'table', op: 'eq', value: 'satellite' })).toEqual({ ok: false, issues: [{ index: 0, reason: 'value_invalid' }] })
    })

    test('표 쿼리의 제목은 표 이름(NFKC·대소문자 무시)으로 곡을 고르고 DB 조건에서는 빠진다', () => {
        const query = tableQuery([
            { field: 'table', op: 'eq', value: 'insane1' },
            { field: 'title', op: 'contains', value: 'bravo' },
            { field: 'bp', op: 'lte', value: '5' },
        ])
        expect(query.entries.map((entry) => entry.title)).toEqual(['Ｂｒａｖｏ Song'])
        expect(query.conditions).toEqual([{ field: 'bp', op: 'lte', value: 5 }])
    })

    test('안 친 곡은 램프 조건만 있고 그 조건이 NO PLAY 를 통과할 때만 함께 보인다', () => {
        expect(acceptsUnplayed([])).toBe(true)
        expect(acceptsUnplayed([{ field: 'lamp', op: 'lte', values: [clearLampToId('Failed')] }])).toBe(true)
        expect(acceptsUnplayed([{ field: 'lamp', op: 'ne', values: [clearLampToId('NoPlay')] }])).toBe(false)
        expect(acceptsUnplayed([{ field: 'lamp', op: 'gte', values: [clearLampToId('Easy')] }])).toBe(false)
        expect(acceptsUnplayed([{ field: 'bp', op: 'lte', value: 5 }])).toBe(false)
    })

    test('결과에는 사용자 필드가 없고 사용자 항목은 스키마가 받지 않는다', () => {
        const result = compileRecordQuery(plan([{ field: 'bp', op: 'gte', value: '0' }]), TABLES)
        expect(JSON.stringify(result)).not.toContain('user')
        expect(AI_RECORD_QUERY_FIELDS.some((field: string) => field.includes('user'))).toBe(false)
    })
})

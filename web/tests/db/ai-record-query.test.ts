import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { inArray, like } from 'drizzle-orm'
import { createAiRecordQueryDb } from '@server/compose/ai-record-query'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, score } from '@server/db/schema'
import type { AiRecordQueryFilter, AiRecordQueryRunInput } from '@server/dto/ai'
import { clearLampToId, gaugeTypeToId } from '@server/dto/common'
import type { RecordQueryTable } from '@server/service/domain/ai/record-query.compiler'
import { createAiRecordQueryService } from '@server/service/domain/ai/record-query.service'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 기록 질문 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'w1rq'
const OWNER = `${PREFIX}_owner`
const OTHER = `${PREFIX}_other`
const DAY_MS = 86_400_000
const T0 = Date.UTC(2026, 8, 1, 12)
const NOTES = 1000
const MAX_EX = 2000

const CHARTS = {
    alpha: { sha256: '9a'.repeat(32), md5: '9a'.repeat(16), title: 'W9 Alpha 100%_x', level: 12 },
    bravo: { sha256: '9b'.repeat(32), md5: '9b'.repeat(16), title: 'W9 Bravo 100 ab', level: 13 },
    charlie: { sha256: '9c'.repeat(32), md5: '9c'.repeat(16), title: 'W9 Charlie', level: 13 },
}

const TABLES: RecordQueryTable[] = [
    {
        id: 'w9table',
        symbol: '★',
        levels: ['12', '13'],
        entries: [
            { md5: CHARTS.alpha.md5, level: '12', title: 'Alpha (table)', artist: 'A' },
            { md5: CHARTS.bravo.md5, level: '13', title: 'Bravo (table)', artist: 'B' },
            { md5: CHARTS.charlie.md5, level: '13', title: 'Charlie (table)', artist: 'C' },
        ],
    },
]

const SCORES = [
    {
        id: `${PREFIX}_s1`,
        userId: OWNER,
        chartSha256: CHARTS.alpha.sha256,
        clear: clearLampToId('Hard'),
        exScore: 1800,
        maxExScore: MAX_EX,
        minbp: 5,
        random: 'Random',
        gauge: gaugeTypeToId('Hard'),
        playedAt: T0,
        scratchAuto: false,
    },
    {
        id: `${PREFIX}_s2`,
        userId: OWNER,
        chartSha256: CHARTS.alpha.sha256,
        clear: clearLampToId('Easy'),
        exScore: 1500,
        maxExScore: MAX_EX,
        minbp: 20,
        random: 'Off',
        gauge: gaugeTypeToId('Easy'),
        playedAt: T0 + DAY_MS,
        scratchAuto: false,
    },
    {
        id: `${PREFIX}_s3`,
        userId: OWNER,
        chartSha256: CHARTS.bravo.sha256,
        clear: clearLampToId('Failed'),
        exScore: 1000,
        maxExScore: 0,
        minbp: 50,
        random: 'Mirror',
        gauge: gaugeTypeToId('Normal'),
        playedAt: T0 + 2 * DAY_MS,
        scratchAuto: true,
    },
    {
        id: `${PREFIX}_s4`,
        userId: OTHER,
        chartSha256: CHARTS.alpha.sha256,
        clear: clearLampToId('Max'),
        exScore: MAX_EX,
        maxExScore: MAX_EX,
        minbp: 0,
        random: 'Off',
        gauge: gaugeTypeToId('Hard'),
        playedAt: T0 + DAY_MS,
        scratchAuto: false,
    },
]

const BESTS = [
    { score: SCORES[0], scratchAuto: false },
    { score: SCORES[2], scratchAuto: true },
    { score: SCORES[3], scratchAuto: false },
].flatMap(({ score: row, scratchAuto }) =>
    row
        ? [
              {
                  chartSha256: row.chartSha256,
                  userId: row.userId,
                  scoreId: row.id,
                  clear: row.clear,
                  exScore: row.exScore,
                  minbp: row.minbp,
                  maxCombo: 0,
                  scratchAuto,
              },
          ]
        : [],
)

const service = () =>
    createAiRecordQueryService({
        runner: { run: async () => ({ ok: false, reason: 'credential_not_found' }) },
        db: createAiRecordQueryDb(getDb()),
        loadTables: () => TABLES,
        now: () => T0,
    })

const runAs = async (userId: string, filters: AiRecordQueryFilter[], overrides: Partial<AiRecordQueryRunInput> = {}) => {
    const answer = await service().run({ userId, plan: { filters, sort_field: 'played_at', sort_dir: 'desc', limit: 50, ...overrides } })
    expect(answer.issues).toEqual([])
    return answer.result
}

const scoreIds = async (filters: AiRecordQueryFilter[], overrides: Partial<AiRecordQueryRunInput> = {}) =>
    (await runAs(OWNER, filters, overrides))?.rows.map((row) => row.score_id)

describe.skipIf(!hasDatabase)('기록 질문 쿼리 (임시 MySQL)', () => {
    const cleanup = async () => {
        const db = getDb()
        await db.delete(chartBest).where(like(chartBest.userId, `${PREFIX}_%`))
        await db.delete(score).where(like(score.id, `${PREFIX}_%`))
        await db.delete(chart).where(
            inArray(
                chart.sha256,
                Object.values(CHARTS).map((row) => row.sha256),
            ),
        )
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    }

    beforeAll(async () => {
        await cleanup()
        const db = getDb()
        await db.insert(user).values([OWNER, OTHER].map((id) => ({ id, name: id, email: `${id}@w1rq.invalid`, loginId: id })))
        await db.insert(chart).values(Object.values(CHARTS).map((row) => ({ ...row, notes: NOTES, mode: 'BEAT_7K' })))
        await db.insert(score).values(SCORES.map((row) => ({ ...row, chartMd5: null, notes: NOTES, mode: 'BEAT_7K' })))
        await db.insert(chartBest).values(BESTS)
    })

    afterAll(cleanup)

    test('모든 판 쿼리는 세션 사용자의 판만 읽고 램프·정렬·limit 을 바인딩된 조건으로 적용한다', async () => {
        expect(await scoreIds([{ field: 'lamp', op: 'gte', value: 'Easy' }])).toEqual([`${PREFIX}_s2`, `${PREFIX}_s1`])
        expect(await scoreIds([{ field: 'lamp', op: 'gte', value: 'Easy' }], { limit: 1 })).toEqual([`${PREFIX}_s2`])
        expect(await scoreIds([{ field: 'lamp', op: 'eq', value: 'Max' }])).toEqual([])
        expect((await runAs(OTHER, [{ field: 'lamp', op: 'eq', value: 'Max' }]))?.rows.map((row) => row.score_id)).toEqual([`${PREFIX}_s4`])
    })

    test('EX 비율은 최대 EX 가 0 이면 노트 수 × 2 로 나누고 정수 비교로 경계를 지킨다', async () => {
        expect(await scoreIds([{ field: 'ex_rate', op: 'gte', value: '90' }])).toEqual([`${PREFIX}_s1`])
        expect(await scoreIds([{ field: 'ex_rate', op: 'lte', value: '50' }])).toEqual([`${PREFIX}_s3`])
        expect(await scoreIds([{ field: 'ex_rate', op: 'gte', value: '75' }], { sort_field: 'ex_rate', sort_dir: 'asc' })).toEqual([
            `${PREFIX}_s2`,
            `${PREFIX}_s1`,
        ])
        const rows = (await runAs(OWNER, [{ field: 'ex_rate', op: 'lte', value: '50' }]))?.rows
        expect(rows?.[0]).toMatchObject({ ex_rate: 50, lamp: 'Failed', gauge: 'Normal', random: 'Mirror', scratch_auto: true, bp: 50 })
    })

    test('BP·배치 옵션·게이지·날짜·SCRATCH AUTO 조건', async () => {
        expect(
            await scoreIds([
                { field: 'bp', op: 'lte', value: '20' },
                { field: 'random', op: 'in', value: 'Random,Off' },
            ]),
        ).toEqual([`${PREFIX}_s2`, `${PREFIX}_s1`])
        expect(await scoreIds([{ field: 'random', op: 'ne', value: 'Off' }])).toEqual([`${PREFIX}_s3`, `${PREFIX}_s1`])
        expect(await scoreIds([{ field: 'gauge', op: 'eq', value: 'Hard' }])).toEqual([`${PREFIX}_s1`])
        expect(await scoreIds([{ field: 'played_at', op: 'gte', value: '2026-09-02' }])).toEqual([`${PREFIX}_s3`, `${PREFIX}_s2`])
        expect(await scoreIds([{ field: 'played_at', op: 'lte', value: '2026-09-02' }])).toEqual([`${PREFIX}_s2`, `${PREFIX}_s1`])
        expect(await scoreIds([{ field: 'scratch_auto', op: 'eq', value: 'true' }])).toEqual([`${PREFIX}_s3`])
    })

    test('제목 검색은 % 와 _ 를 글자 그대로 찾는다', async () => {
        expect(await scoreIds([{ field: 'title', op: 'contains', value: '100%_' }])).toEqual([`${PREFIX}_s2`, `${PREFIX}_s1`])
        expect(await scoreIds([{ field: 'title', op: 'contains', value: 'W9 B' }])).toEqual([`${PREFIX}_s3`])
    })

    test('표 쿼리는 곡마다 본인 최고 기록(SCRATCH AUTO 만 있으면 그것)과 안 친 곡을 보이고 다른 사용자 기록은 섞지 않는다', async () => {
        const result = await runAs(OWNER, [{ field: 'level', op: 'gte', value: '12' }], { sort_field: 'level', sort_dir: 'asc' })
        expect(result?.source).toBe('table')
        expect(result?.rows.map((row) => [row.chart_md5, row.lamp, row.score_id, row.table_level])).toEqual([
            [CHARTS.alpha.md5, 'Hard', `${PREFIX}_s1`, '★12'],
            [CHARTS.bravo.md5, 'Failed', `${PREFIX}_s3`, '★13'],
            [CHARTS.charlie.md5, 'NoPlay', null, '★13'],
        ])
        const unclear = await runAs(OWNER, [
            { field: 'table', op: 'eq', value: 'w9table' },
            { field: 'lamp', op: 'lte', value: 'Failed' },
        ])
        expect(unclear?.rows.map((row) => row.chart_md5)).toEqual([CHARTS.bravo.md5, CHARTS.charlie.md5])
        const normalOnly = await runAs(OWNER, [
            { field: 'table', op: 'eq', value: 'w9table' },
            { field: 'scratch_auto', op: 'eq', value: 'false' },
        ])
        expect(normalOnly?.rows.map((row) => row.chart_md5)).toEqual([CHARTS.alpha.md5])
        const other = await runAs(OTHER, [{ field: 'table', op: 'eq', value: 'w9table' }])
        expect(other?.rows.map((row) => [row.chart_md5, row.lamp])).toEqual([
            [CHARTS.alpha.md5, 'Max'],
            [CHARTS.bravo.md5, 'NoPlay'],
            [CHARTS.charlie.md5, 'NoPlay'],
        ])
    })
})

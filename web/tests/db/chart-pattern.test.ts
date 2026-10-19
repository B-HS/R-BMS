import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { gunzipSync } from 'node:zlib'
import { and, eq, inArray, like, sql } from 'drizzle-orm'
import { createChartPatternDb } from '@server/compose/chart-pattern'
import { getDb } from '@server/db'
import { chartPattern } from '@server/db/schema'
import { CHART_PATTERN_READ_RECOMPUTE_MAX } from '@server/dto/chart-pattern'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { computeChartFeatures } from '@server/service/domain/analysis/chart-features'
import { createChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { makePatternDocument, type FixtureNote } from '../support/chart-pattern-fixture'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 곡별 패턴 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const MD5_PREFIX = 'feed'
const md5Of = (index: number) => `${MD5_PREFIX}${String(index).padStart(28, '0')}`
const ADMIN_ID = 'w3-admin'
const NOW = Date.UTC(2026, 9, 20, 12)
const GZIP_MAGIC = [0x1f, 0x8b]
const SMALL_NOTES: FixtureNote[] = Array.from({ length: 500 }, (_, index): FixtureNote => [index * 60_000, index % 8, index % 9 === 0 ? 3 : 0])
const LARGEST_MEASURED_BYTES = 1_600_000
const LARGE_NOTE_COUNT = 70_000
const PARALLEL_UPLOADS = 6

const createService = (md5s: string[] = []) =>
    createChartPatternService({
        db: createChartPatternDb(getDb()),
        now: () => NOW,
        listCoverageTables: () => [{ id: 'insane1', name: 'insane', md5s }],
    })

describe.skipIf(!hasDatabase)('곡별 패턴 표 (임시 MySQL)', () => {
    beforeAll(async () => {
        await getDb()
            .delete(chartPattern)
            .where(like(chartPattern.md5, `${MD5_PREFIX}%`))
    })

    afterAll(async () => {
        await getDb()
            .delete(chartPattern)
            .where(like(chartPattern.md5, `${MD5_PREFIX}%`))
    })

    test('마이그레이션이 chart_pattern 표, mediumblob data 열, sha256 색인을 만든다', async () => {
        const [columns] = await getDb().execute(
            sql`SELECT column_name AS name, data_type AS type FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'chart_pattern' AND column_name IN ('data', 'features')`,
        )
        expect(
            Array.isArray(columns) ? columns.map((row) => `${String(Object.values(row)[0])}:${String(Object.values(row)[1])}`).toSorted() : [],
        ).toEqual(['data:mediumblob', 'features:json'])
        const [indexes] = await getDb().execute(
            sql`SELECT column_name AS name FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'chart_pattern' AND index_name = 'chart_pattern_sha256_idx'`,
        )
        expect(Array.isArray(indexes) ? indexes.map((row) => String(Object.values(row)[0])) : []).toEqual(['sha256'])
    })

    test('created → unchanged → replaced 가 한 행에서 일어나고 data 는 gzip 바이트로 저장되어 그대로 풀린다', async () => {
        const md5 = md5Of(1)
        const service = createService()
        const document = makePatternDocument(SMALL_NOTES, { md5 })
        expect((await service.upload({ document, uploadedBy: ADMIN_ID })).outcome).toBe('created')
        expect((await service.upload({ document, uploadedBy: ADMIN_ID })).outcome).toBe('unchanged')
        const [row] = await getDb().select().from(chartPattern).where(eq(chartPattern.md5, md5))
        expect(Buffer.isBuffer(row.data)).toBe(true)
        expect([...row.data.subarray(0, 2)]).toEqual(GZIP_MAGIC)
        expect(gunzipSync(row.data).toString('utf8')).toBe(JSON.stringify(document))
        expect(row).toMatchObject({
            parserVersion: 5,
            featuresVersion: ANALYSIS_VERSION,
            uploadedBy: ADMIN_ID,
            updatedAt: NOW,
            noteCount: SMALL_NOTES.length,
        })
        expect(row.features).toEqual(computeChartFeatures(document))
        expect(await service.readPattern(md5)).toEqual(document)

        const newer = makePatternDocument(SMALL_NOTES.slice(10), { md5, parser_version: 6 })
        expect((await service.upload({ document: newer, uploadedBy: ADMIN_ID })).outcome).toBe('replaced')
        const rows = await getDb()
            .select({ parserVersion: chartPattern.parserVersion, noteCount: chartPattern.noteCount })
            .from(chartPattern)
            .where(eq(chartPattern.md5, md5))
        expect(rows).toEqual([{ parserVersion: 6, noteCount: SMALL_NOTES.length - 10 }])
        expect(await service.readPattern(md5)).toEqual(newer)
    })

    test('같은 패턴을 동시에 여러 번 처음 올려도 한 행이고 하나만 created 다', async () => {
        const md5 = md5Of(2)
        const service = createService()
        const outcomes = await Promise.all(
            Array.from({ length: PARALLEL_UPLOADS }, () =>
                service.upload({ document: makePatternDocument(SMALL_NOTES, { md5 }), uploadedBy: ADMIN_ID }),
            ),
        )
        expect(outcomes.filter((result) => result.outcome === 'created')).toHaveLength(1)
        expect(outcomes.filter((result) => result.outcome === 'replaced')).toHaveLength(0)
        const [counted] = await getDb()
            .select({ value: sql<number>`count(*)` })
            .from(chartPattern)
            .where(eq(chartPattern.md5, md5))
        expect(Number(counted.value)).toBe(1)
    })

    test(`가장 큰 측정 차트(약 1.6 MB JSON)보다 큰 ${LARGE_NOTE_COUNT}노트 패턴도 저장하고 읽는다`, async () => {
        const md5 = md5Of(3)
        const notes = Array.from({ length: LARGE_NOTE_COUNT }, (_, index): FixtureNote => [index * 4_321 + 123_456_789, index % 8, index % 3])
        const document = makePatternDocument(notes, { md5 })
        expect(Buffer.byteLength(JSON.stringify(document))).toBeGreaterThan(LARGEST_MEASURED_BYTES)
        const service = createService()
        expect((await service.upload({ document, uploadedBy: ADMIN_ID })).outcome).toBe('created')
        expect(await service.readPattern(md5)).toEqual(document)
    })

    test('읽을 때 낮은 버전의 특징은 다시 계산해 저장하고, coverage 는 표 md5 중 있는 수를 센다', async () => {
        const md5 = md5Of(1)
        await getDb().update(chartPattern).set({ featuresVersion: 0 }).where(eq(chartPattern.md5, md5))
        const service = createService([md5Of(1), md5Of(2), md5Of(9)])
        const coverageBefore = await service.coverage()
        expect(coverageBefore.tables[0]).toMatchObject({ table_md5s: 3, covered: 2, missing: [md5Of(9)] })
        expect(coverageBefore.stale_features).toBeGreaterThanOrEqual(1)
        expect(await service.readFeatures(md5)).toMatchObject({ analysis_version: ANALYSIS_VERSION, layout: 'original' })
        const [row] = await getDb().select({ featuresVersion: chartPattern.featuresVersion }).from(chartPattern).where(eq(chartPattern.md5, md5))
        expect(row.featuresVersion).toBe(ANALYSIS_VERSION)
    })

    test('낮은 버전 특징은 읽기 한 번에 상한까지만 다시 계산하고, 일괄 재계산이 md5 순서로 이어 받아 끝낸다', async () => {
        const service = createService()
        const total = CHART_PATTERN_READ_RECOMPUTE_MAX + 5
        const md5s = Array.from({ length: total }, (_, index) => md5Of(100 + index))
        for (const md5 of md5s) await service.upload({ document: makePatternDocument(SMALL_NOTES, { md5 }), uploadedBy: ADMIN_ID })
        await getDb().update(chartPattern).set({ featuresVersion: 0 }).where(inArray(chartPattern.md5, md5s))
        const countCurrent = async () => {
            const [counted] = await getDb()
                .select({ value: sql<number>`count(*)` })
                .from(chartPattern)
                .where(and(inArray(chartPattern.md5, md5s), eq(chartPattern.featuresVersion, ANALYSIS_VERSION)))
            return Number(counted.value)
        }

        const read = await service.readFeaturesMany(md5s)
        expect(read.size).toBe(total)
        expect([...read.values()].filter((entry) => entry.isStale)).toHaveLength(total - CHART_PATTERN_READ_RECOMPUTE_MAX)
        expect(await countCurrent()).toBe(CHART_PATTERN_READ_RECOMPUTE_MAX)

        let after: string | null = null
        let batches = 0
        do {
            const step = await service.recomputeStale({ limit: 4, after })
            expect(step.examined).toBeLessThanOrEqual(4)
            expect(step.skipped).toEqual([])
            after = step.next_after
            batches += 1
        } while (after !== null && batches < 50)
        expect(after).toBeNull()
        expect(await countCurrent()).toBe(total)
        expect((await service.coverage()).stale_features).toBe(0)
        expect([...(await service.readFeaturesMany(md5s)).values()].some((entry) => entry.isStale)).toBe(false)
    })
})

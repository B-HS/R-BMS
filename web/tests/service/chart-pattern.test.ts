import { describe, expect, test } from 'bun:test'
import { gunzipSync } from 'node:zlib'
import { CHART_PATTERN_READ_RECOMPUTE_MAX } from '@server/dto/chart-pattern'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { computeChartFeatures } from '@server/service/domain/analysis/chart-features'
import { createChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { gunzipJson, gzipJsonText } from '@server/service/shared/storage/gzip-json'
import { FIXTURE_MD5, createMemoryChartPatternDb, makePatternDocument, type FixtureNote } from '../support/chart-pattern-fixture'

const NOW = Date.UTC(2026, 9, 20, 12)
const ADMIN_ID = 'user-admin'
const GZIP_MAGIC = [0x1f, 0x8b]
const NOTES: FixtureNote[] = Array.from({ length: 400 }, (_, index): FixtureNote => [index * 50_000, index % 8, 0])
const OTHER_MD5 = 'a'.repeat(32)

const createService = (tables = [{ id: 'insane1', name: '発狂BMS', md5s: [FIXTURE_MD5, OTHER_MD5] }]) => {
    const memory = createMemoryChartPatternDb()
    const service = createChartPatternService({ db: memory.db, now: () => NOW, listCoverageTables: () => tables })
    return { memory, service }
}

describe('gzip JSON 저장 형식', () => {
    test('압축한 바이트를 풀면 원래 JSON 과 같다', async () => {
        const document = makePatternDocument(NOTES)
        const text = JSON.stringify(document)
        const bytes = await gzipJsonText(text)
        expect([...bytes.subarray(0, 2)]).toEqual(GZIP_MAGIC)
        expect(bytes.byteLength).toBeLessThan(Buffer.byteLength(text))
        expect(gunzipSync(bytes).toString('utf8')).toBe(text)
        expect(await gunzipJson(bytes)).toEqual(document)
    })
})

describe('곡별 패턴 업로드(멱등)', () => {
    test('처음은 created 로 gzip 압축해 특징과 함께 저장하고, 같은 것을 다시 올리면 아무것도 바꾸지 않는다', async () => {
        const { memory, service } = createService()
        const document = makePatternDocument(NOTES)
        expect(await service.upload({ document, uploadedBy: ADMIN_ID })).toEqual({
            md5: FIXTURE_MD5,
            outcome: 'created',
            parser_version: 5,
            features_version: ANALYSIS_VERSION,
            features_recomputed: true,
            note_count: NOTES.length,
        })
        const stored = memory.rows.get(FIXTURE_MD5)
        expect(stored).toMatchObject({
            parserVersion: 5,
            featuresVersion: ANALYSIS_VERSION,
            uploadedBy: ADMIN_ID,
            updatedAt: NOW,
            noteCount: NOTES.length,
        })
        expect([...(stored?.data.subarray(0, 2) ?? [])]).toEqual(GZIP_MAGIC)
        expect(stored?.byteSize).toBe(Buffer.byteLength(JSON.stringify(document)))
        expect(stored?.features).toEqual(computeChartFeatures(document))

        const again = await service.upload({ document: makePatternDocument(NOTES), uploadedBy: 'someone-else' })
        expect(again).toMatchObject({ outcome: 'unchanged', features_recomputed: false })
        expect(memory.calls).toEqual({ insert: 1, replace: 0, updateFeatures: 0 })
        expect(memory.rows.get(FIXTURE_MD5)?.uploadedBy).toBe(ADMIN_ID)
    })

    test('parser_version 이 다르면 덮어쓰고 특징을 다시 계산한다', async () => {
        const { memory, service } = createService()
        await service.upload({ document: makePatternDocument(NOTES), uploadedBy: ADMIN_ID })
        const newer = makePatternDocument(NOTES, { parser_version: 6 })
        expect(await service.upload({ document: newer, uploadedBy: ADMIN_ID })).toMatchObject({ outcome: 'replaced', parser_version: 6 })
        expect(memory.rows.get(FIXTURE_MD5)?.parserVersion).toBe(6)
        expect(await service.readPattern(FIXTURE_MD5)).toEqual(newer)
    })

    test('같은 parser_version 이라도 내용이 다르면 덮어쓴다', async () => {
        const { memory, service } = createService()
        await service.upload({ document: makePatternDocument(NOTES), uploadedBy: ADMIN_ID })
        const changed = makePatternDocument(NOTES.slice(1))
        expect((await service.upload({ document: changed, uploadedBy: ADMIN_ID })).outcome).toBe('replaced')
        expect(memory.calls.replace).toBe(1)
        expect(await service.readPattern(FIXTURE_MD5)).toEqual(changed)
    })

    test('내용이 같아도 특징 버전이 낮으면 특징만 다시 계산하고 unchanged 로 답한다', async () => {
        const { memory, service } = createService()
        await service.upload({ document: makePatternDocument(NOTES), uploadedBy: ADMIN_ID })
        const row = memory.rows.get(FIXTURE_MD5)
        if (row) memory.rows.set(FIXTURE_MD5, { ...row, featuresVersion: ANALYSIS_VERSION - 1 })
        expect(await service.upload({ document: makePatternDocument(NOTES), uploadedBy: ADMIN_ID })).toMatchObject({
            outcome: 'unchanged',
            features_recomputed: true,
        })
        expect(memory.calls).toEqual({ insert: 1, replace: 0, updateFeatures: 1 })
        expect(memory.rows.get(FIXTURE_MD5)?.featuresVersion).toBe(ANALYSIS_VERSION)
    })

    test('동시에 처음 올린 같은 패턴은 하나가 created, 다른 하나는 unchanged 다', async () => {
        const { memory, service } = createService()
        const document = makePatternDocument(NOTES)
        memory.hooks.beforeInsert = (row) => {
            memory.hooks.beforeInsert = null
            memory.rows.set(row.md5, row)
        }
        expect((await service.upload({ document, uploadedBy: ADMIN_ID })).outcome).toBe('unchanged')
        expect(memory.calls.replace).toBe(0)
    })

    test('읽을 때 특징 버전이 낮으면 저장된 배치로 다시 계산해 저장한다', async () => {
        const { memory, service } = createService()
        const document = makePatternDocument(NOTES)
        await service.upload({ document, uploadedBy: ADMIN_ID })
        expect(await service.readFeatures(FIXTURE_MD5)).toEqual(computeChartFeatures(document))
        expect(memory.calls.updateFeatures).toBe(0)
        const row = memory.rows.get(FIXTURE_MD5)
        if (row) memory.rows.set(FIXTURE_MD5, { ...row, features: computeChartFeatures(makePatternDocument([])), featuresVersion: 0 })
        expect(await service.readFeatures(FIXTURE_MD5)).toEqual(computeChartFeatures(document))
        expect(memory.calls.updateFeatures).toBe(1)
        expect(await service.readFeatures(OTHER_MD5)).toBeNull()
    })
})

const md5Of = (index: number) => index.toString(16).padStart(32, '0')
const OLD_VERSION = ANALYSIS_VERSION - 1
const OLD_TAG_RATIO = 0.123

const oldFeaturesOf = (document: ReturnType<typeof makePatternDocument>) => ({
    ...computeChartFeatures(document),
    analysis_version: OLD_VERSION,
    tags: { trill: { ratio: OLD_TAG_RATIO, notes: 1 } },
})

const seedStale = async (service: ReturnType<typeof createService>['service'], memory: ReturnType<typeof createService>['memory'], count: number) => {
    for (let index = 0; index < count; index += 1) {
        const document = makePatternDocument(NOTES, { md5: md5Of(index) })
        await service.upload({ document, uploadedBy: ADMIN_ID })
        const row = memory.rows.get(md5Of(index))
        if (row) memory.rows.set(md5Of(index), { ...row, features: oldFeaturesOf(document), featuresVersion: OLD_VERSION })
    }
    memory.calls.updateFeatures = 0
}

describe('여러 곡의 특징 읽기(readFeaturesMany)', () => {
    test('버전이 낮은 특징은 요청 한 번에 CHART_PATTERN_READ_RECOMPUTE_MAX 곡까지만 다시 계산하고 나머지는 옛 특징 그대로 stale 로 표시한다', async () => {
        const { memory, service } = createService()
        const total = CHART_PATTERN_READ_RECOMPUTE_MAX + 7
        await seedStale(service, memory, total)
        const md5s = Array.from({ length: total }, (_, index) => md5Of(index))
        const read = await service.readFeaturesMany(md5s)
        expect(memory.calls.updateFeatures).toBe(CHART_PATTERN_READ_RECOMPUTE_MAX)
        expect(read.size).toBe(total)
        const fresh = md5s.slice(0, CHART_PATTERN_READ_RECOMPUTE_MAX)
        const stale = md5s.slice(CHART_PATTERN_READ_RECOMPUTE_MAX)
        expect(fresh.every((md5) => read.get(md5)?.isStale === false && read.get(md5)?.features.analysis_version === ANALYSIS_VERSION)).toBe(true)
        expect(stale.every((md5) => read.get(md5)?.isStale === true && read.get(md5)?.features.tags.trill?.ratio === OLD_TAG_RATIO)).toBe(true)
        expect(stale.every((md5) => memory.rows.get(md5)?.featuresVersion === OLD_VERSION)).toBe(true)
    })

    test('다시 계산한 곡은 저장되어 다음 읽기에서는 계산하지 않고, 남은 곡이 이어서 계산된다', async () => {
        const { memory, service } = createService()
        const total = CHART_PATTERN_READ_RECOMPUTE_MAX * 2
        await seedStale(service, memory, total)
        const md5s = Array.from({ length: total }, (_, index) => md5Of(index))
        await service.readFeaturesMany(md5s)
        const second = await service.readFeaturesMany(md5s)
        expect(memory.calls.updateFeatures).toBe(total)
        expect([...second.values()].every((entry) => !entry.isStale)).toBe(true)
        const third = await service.readFeaturesMany(md5s)
        expect(memory.calls.updateFeatures).toBe(total)
        expect(third.size).toBe(total)
    })

    test('현재 버전 특징은 계산하지 않고, 읽을 수 없는 옛 특징은 계산 몫이 없으면 빠지며 패턴이 없는 곡도 빠진다', async () => {
        const { memory, service } = createService()
        await seedStale(service, memory, CHART_PATTERN_READ_RECOMPUTE_MAX + 1)
        await service.upload({ document: makePatternDocument(NOTES, { md5: 'f'.repeat(32) }), uploadedBy: ADMIN_ID })
        memory.calls.updateFeatures = 0
        const broken = md5Of(CHART_PATTERN_READ_RECOMPUTE_MAX)
        memory.rawFeatures.set(broken, { analysis_version: OLD_VERSION })
        const md5s = [...Array.from({ length: CHART_PATTERN_READ_RECOMPUTE_MAX + 1 }, (_, index) => md5Of(index)), 'f'.repeat(32), 'e'.repeat(32)]
        const read = await service.readFeaturesMany(md5s)
        expect(read.has(broken)).toBe(false)
        expect(read.has('e'.repeat(32))).toBe(false)
        expect(read.get('f'.repeat(32))?.isStale).toBe(false)
        expect(memory.calls.updateFeatures).toBe(CHART_PATTERN_READ_RECOMPUTE_MAX)
        expect((await service.readFeaturesMany([])).size).toBe(0)
    })
})

describe('관리자 일괄 재계산(recomputeStale)', () => {
    test('버전이 낮은 곡만 md5 순서로 배치 크기만큼 다시 계산하고, 다음 시작점을 돌려주며, 끝나면 시작점이 null 이다', async () => {
        const { memory, service } = createService()
        await seedStale(service, memory, 5)
        await service.upload({ document: makePatternDocument(NOTES, { md5: 'f'.repeat(32) }), uploadedBy: ADMIN_ID })
        memory.calls.updateFeatures = 0
        const first = await service.recomputeStale({ limit: 2, after: null })
        expect(first).toEqual({ analysis_version: ANALYSIS_VERSION, examined: 2, recomputed: 2, skipped: [], next_after: md5Of(1), remaining: 3 })
        const second = await service.recomputeStale({ limit: 2, after: first.next_after })
        expect(second).toMatchObject({ examined: 2, recomputed: 2, next_after: md5Of(3), remaining: 1 })
        const last = await service.recomputeStale({ limit: 2, after: second.next_after })
        expect(last).toMatchObject({ examined: 1, recomputed: 1, next_after: null, remaining: 0 })
        expect(memory.calls.updateFeatures).toBe(5)
        expect(memory.rows.get(md5Of(4))?.featuresVersion).toBe(ANALYSIS_VERSION)
        expect(memory.rows.get(md5Of(4))?.features).toEqual(computeChartFeatures(makePatternDocument(NOTES, { md5: md5Of(4) })))
    })

    test('읽을 수 없는 배치는 건너뛰고 skipped 에 적으며, 시작점이 그 곡을 지나 다음 배치로 넘어간다', async () => {
        const { memory, service } = createService()
        await seedStale(service, memory, 3)
        const row = memory.rows.get(md5Of(0))
        if (row) memory.rows.set(md5Of(0), { ...row, data: Buffer.from('not gzip') })
        const first = await service.recomputeStale({ limit: 1, after: null })
        expect(first).toMatchObject({ examined: 1, recomputed: 0, skipped: [md5Of(0)], next_after: md5Of(0), remaining: 3 })
        const second = await service.recomputeStale({ limit: 5, after: first.next_after })
        expect(second).toMatchObject({ examined: 2, recomputed: 2, skipped: [], next_after: null, remaining: 1 })
    })
})

describe('범위(coverage)', () => {
    test('표 md5 중 패턴이 있는 수와 없는 md5, 전체 패턴 수를 센다', async () => {
        const { service } = createService()
        await service.upload({ document: makePatternDocument(NOTES), uploadedBy: ADMIN_ID })
        await service.upload({ document: makePatternDocument(NOTES, { md5: 'b'.repeat(32) }), uploadedBy: ADMIN_ID })
        expect(await service.coverage()).toEqual({
            tables: [{ table_id: 'insane1', name: '発狂BMS', table_md5s: 2, covered: 1, missing: [OTHER_MD5] }],
            patterns_total: 2,
            stale_features: 0,
            analysis_version: ANALYSIS_VERSION,
        })
    })
})

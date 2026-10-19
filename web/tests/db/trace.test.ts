import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { gunzipSync } from 'node:zlib'
import { eq, inArray, like, sql } from 'drizzle-orm'
import { createChartPatternDb } from '@server/compose/chart-pattern'
import { createTraceDb } from '@server/compose/trace'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chartPattern, playAnalysis, playTrace, score } from '@server/db/schema'
import { TRACE_MAX_BYTES, type TraceUploadInput } from '@server/dto/trace'
import { PLAY_ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { createChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { PATTERN_JOIN } from '@server/service/domain/trace/trace-analysis'
import { countTraceJudgements } from '@server/service/domain/trace/trace-check'
import { TRACE_UPLOAD_OUTCOME, createTraceService } from '@server/service/domain/trace/trace.service'
import { makePatternDocument, type FixtureNote } from '../support/chart-pattern-fixture'
import { readContractTrace } from '../support/trace-fixture'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 판별 trace DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'w4tr'
const OWNER = `${PREFIX}_owner`
const OTHER = `${PREFIX}_other`
const NOW = Date.UTC(2026, 9, 20, 12)
const NORMAL = readContractTrace('trace-normal.json')
const PARALLEL_UPLOADS = 4
const LARGE_NOTES = 60_000
const LARGE_NOTE_GAP_US = 3_000
const LANES = 8
const LATE_ERR_US = -12_345

const scoreIdOf = (name: string) => `${PREFIX}_${name}`

const scoreRowOf = (trace: TraceUploadInput, userId: string, playedAt: number) => ({
    ...countTraceJudgements(trace),
    id: trace.score_id,
    userId,
    chartSha256: trace.chart.sha256,
    chartMd5: trace.chart.md5,
    mode: trace.mode,
    notes: trace.notes.time_us.length,
    playedAt,
})

const largeTrace = () => {
    const indexes = Array.from({ length: LARGE_NOTES }, (_, index) => index)
    return {
        ...NORMAL,
        score_id: scoreIdOf('large'),
        notes: {
            time_us: indexes.map((index) => 1_000_000_000 + index * LARGE_NOTE_GAP_US),
            lane: indexes.map((index) => index % LANES),
            kind: indexes.map(() => 0),
            state: indexes.map((index) => 1 + (index % 5)),
            err_us: indexes.map((index) => (index % 2 === 0 ? index : LATE_ERR_US)),
        },
    } satisfies TraceUploadInput
}

const TRACES = {
    normal: { ...NORMAL, score_id: scoreIdOf('normal') },
    race: { ...NORMAL, score_id: scoreIdOf('race') },
    other: { ...NORMAL, score_id: scoreIdOf('other') },
    large: largeTrace(),
}

const issued = { ids: 0 }

const createService = () => {
    const patterns = createChartPatternService({ db: createChartPatternDb(getDb()), now: () => NOW, listCoverageTables: () => [] })
    const failures: unknown[] = []
    const service = createTraceService({
        db: createTraceDb(getDb()),
        newId: (prefix) => {
            issued.ids += 1
            return `${prefix}${PREFIX}_${issued.ids}`
        },
        now: () => NOW,
        loadPattern: patterns.readPattern,
        reportAnalysisFailure: (error) => {
            failures.push(error)
        },
    })
    return { service, failures, patterns }
}

describe.skipIf(!hasDatabase)('판별 trace 표 (임시 MySQL)', () => {
    const cleanup = async () => {
        const db = getDb()
        await db.delete(playTrace).where(like(playTrace.userId, `${PREFIX}_%`))
        await db.delete(score).where(like(score.id, `${PREFIX}_%`))
        await db.delete(chartPattern).where(eq(chartPattern.md5, NORMAL.chart.md5))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    }

    beforeAll(async () => {
        await cleanup()
        const db = getDb()
        await db.insert(user).values([OWNER, OTHER].map((id) => ({ id, name: id, email: `${id}@w4tr.invalid`, loginId: id })))
        await db
            .insert(score)
            .values([
                scoreRowOf(TRACES.normal, OWNER, NOW),
                scoreRowOf(TRACES.race, OWNER, NOW + 1),
                scoreRowOf(TRACES.other, OTHER, NOW),
                scoreRowOf(TRACES.large, OWNER, NOW + 2),
            ])
    })

    afterAll(cleanup)

    test('마이그레이션 0009 가 play_trace(mediumblob data, score_id 고유)와 play_analysis(json summary)를 만든다', async () => {
        const [columns] = await getDb().execute(
            sql`SELECT concat(table_name, '.', column_name, ':', data_type) AS name FROM information_schema.columns WHERE table_schema = DATABASE() AND ((table_name = 'play_trace' AND column_name = 'data') OR (table_name = 'play_analysis' AND column_name = 'summary'))`,
        )
        expect(Array.isArray(columns) ? columns.map((row) => String(Object.values(row)[0])).toSorted() : []).toEqual([
            'play_analysis.summary:json',
            'play_trace.data:mediumblob',
        ])
        const [unique] = await getDb().execute(
            sql`SELECT column_name AS name FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'play_trace' AND index_name = 'play_trace_score_uniq' AND non_unique = 0`,
        )
        expect(Array.isArray(unique) ? unique.map((row) => String(Object.values(row)[0])) : []).toEqual(['score_id'])
    })

    test('점수 행과 맞는 trace 를 gzip 으로 저장하고, 같은 파일의 곡별 패턴과 맞춰 분석한다', async () => {
        const { service, failures, patterns } = createService()
        const notes = NORMAL.notes.time_us.map((timeUs, index): FixtureNote => [timeUs, NORMAL.notes.lane[index], NORMAL.notes.kind[index]])
        await patterns.upload({ document: makePatternDocument(notes, { md5: NORMAL.chart.md5, sha256: NORMAL.chart.sha256 }), uploadedBy: OWNER })
        const result = await service.upload({ input: TRACES.normal, pathHash: NORMAL.chart.md5, userId: OWNER })
        expect(result).toMatchObject({ outcome: TRACE_UPLOAD_OUTCOME.CREATED, analysis: 'stored', storage: { traces: 1, level: 'ok' } })
        expect(failures).toEqual([])
        const [stored] = await getDb().select().from(playTrace).where(eq(playTrace.scoreId, TRACES.normal.score_id))
        expect(JSON.parse(gunzipSync(stored.data).toString('utf8'))).toEqual(TRACES.normal)
        expect(stored).toMatchObject({
            userId: OWNER,
            chartSha256: NORMAL.chart.sha256,
            chartMd5: NORMAL.chart.md5,
            playedAt: NOW,
            storedSize: stored.data.length,
        })
        const [analysis] = await getDb().select().from(playAnalysis).where(eq(playAnalysis.traceId, stored.id))
        expect(analysis).toMatchObject({ analysisVersion: PLAY_ANALYSIS_VERSION, patternJoin: PATTERN_JOIN.MATCHED, patternMd5: NORMAL.chart.md5 })
    })

    test(`같은 점수의 trace 를 ${PARALLEL_UPLOADS}개 동시에 올려도 행은 하나이고 모두 같은 id 를 받는다`, async () => {
        const { service } = createService()
        const results = await Promise.all(
            Array.from({ length: PARALLEL_UPLOADS }, () => service.upload({ input: TRACES.race, pathHash: NORMAL.chart.md5, userId: OWNER })),
        )
        const rows = await getDb().select({ id: playTrace.id }).from(playTrace).where(eq(playTrace.scoreId, TRACES.race.score_id))
        expect(rows).toHaveLength(1)
        expect(results.map((result) => ('id' in result ? result.id : null))).toEqual(Array.from({ length: PARALLEL_UPLOADS }, () => rows[0].id))
        expect(results.filter((result) => result.outcome === TRACE_UPLOAD_OUTCOME.CREATED)).toHaveLength(1)
    })

    test('다른 사용자의 점수에는 붙지 않고, 본인 trace 의 분석은 본인만 읽는다', async () => {
        const { service } = createService()
        expect(await service.upload({ input: TRACES.other, pathHash: NORMAL.chart.md5, userId: OWNER })).toEqual({
            outcome: TRACE_UPLOAD_OUTCOME.SCORE_NOT_FOUND,
        })
        const [stored] = await getDb().select({ id: playTrace.id }).from(playTrace).where(eq(playTrace.scoreId, TRACES.normal.score_id))
        expect(await service.readAnalysis({ traceId: stored.id, userId: OTHER })).toBeNull()
        expect(await service.readAnalysis({ traceId: stored.id, userId: OWNER })).toMatchObject({ pattern: { join: PATTERN_JOIN.MATCHED } })
    })

    test(`${LARGE_NOTES.toLocaleString('en-US')}노트 trace 가 한도 안에서 mediumblob 에 들어가고, 지운 분석은 읽을 때 다시 계산한다`, async () => {
        const { service } = createService()
        const bytes = new TextEncoder().encode(JSON.stringify(TRACES.large)).length
        expect(bytes).toBeLessThanOrEqual(TRACE_MAX_BYTES)
        const result = await service.upload({ input: TRACES.large, pathHash: NORMAL.chart.md5, userId: OWNER })
        expect(result).toMatchObject({ outcome: TRACE_UPLOAD_OUTCOME.CREATED, analysis: 'stored' })
        const [stored] = await getDb().select().from(playTrace).where(eq(playTrace.scoreId, TRACES.large.score_id))
        expect([stored.byteSize, stored.noteCount]).toEqual([bytes, LARGE_NOTES])
        await getDb().delete(playAnalysis).where(eq(playAnalysis.traceId, stored.id))
        const summary = await service.readAnalysis({ traceId: stored.id, userId: OWNER })
        expect(summary?.metrics?.overall.notes).toBe(LARGE_NOTES)
        expect(summary?.pattern.join).toBe(PATTERN_JOIN.MISMATCH)
    })

    test('보관량과 관리자 수치를 SQL 로 센다', async () => {
        const { service } = createService()
        const traceDb = createTraceDb(getDb())
        const rows = await getDb().select({ byteSize: playTrace.byteSize }).from(playTrace).where(eq(playTrace.userId, OWNER))
        const bytes = rows.reduce((sum, row) => sum + row.byteSize, 0)
        expect(await traceDb.sumUserTraces(OWNER)).toEqual({ count: 3, bytes })
        expect(await service.usageOf(OWNER)).toMatchObject({ traces: 3, bytes, level: 'ok' })
        expect(
            await traceDb.countAccountsPast({
                near: { count: 3, bytes: Number.MAX_SAFE_INTEGER },
                over: { count: 3, bytes: Number.MAX_SAFE_INTEGER },
            }),
        ).toMatchObject({
            near: 1,
            over: 0,
        })
        const usage = await service.adminUsage()
        expect(usage.top.find((row) => row.login_id === OWNER)).toMatchObject({ traces: 3, bytes })
        expect(usage.analysis.analysis_version).toBe(PLAY_ANALYSIS_VERSION)
    })

    test('사용자를 지우면 그 trace 와 분석도 지워진다', async () => {
        const ids = (await getDb().select({ id: playTrace.id }).from(playTrace).where(eq(playTrace.userId, OWNER))).map((row) => row.id)
        expect(ids.length).toBeGreaterThan(0)
        await getDb().delete(score).where(eq(score.userId, OWNER))
        await getDb().delete(user).where(eq(user.id, OWNER))
        expect(await getDb().select({ id: playTrace.id }).from(playTrace).where(inArray(playTrace.id, ids))).toEqual([])
        expect(await getDb().select({ id: playAnalysis.traceId }).from(playAnalysis).where(inArray(playAnalysis.traceId, ids))).toEqual([])
    })
})

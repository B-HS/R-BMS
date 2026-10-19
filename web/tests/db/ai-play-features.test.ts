import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { eq, like, sql } from 'drizzle-orm'
import { createAiPlayCompareDb, createPlayScoreDb } from '@server/compose/ai-play-features'
import { createAiJudgeHabitDb } from '@server/compose/ai-score-features'
import { createTraceDb } from '@server/compose/trace'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, playAnalysis, playShareSetting, playTrace, rival, score } from '@server/db/schema'
import type { TraceUploadInput } from '@server/dto/trace'
import { ANALYSIS_VERSION, PLAY_ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { countTraceJudgements } from '@server/service/domain/trace/trace-check'
import { TRACE_UPLOAD_OUTCOME, createTraceService } from '@server/service/domain/trace/trace.service'
import { readContractTrace } from '../support/trace-fixture'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 판별 기반 AI 기능 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'w5pf'
const OWNER = `${PREFIX}_owner`
const RIVAL = `${PREFIX}_rival`
const NOW = Date.UTC(2026, 9, 20, 12)
const NORMAL = readContractTrace('trace-normal.json')
const EX = { first: 30, best: 40, autoplay: 64, rival: 35 } as const

const scoreIdOf = (name: string) => `${PREFIX}_${name}`

const TRACES = {
    first: { ...NORMAL, score_id: scoreIdOf('first') },
    best: { ...NORMAL, score_id: scoreIdOf('best') },
    autoplay: { ...NORMAL, score_id: scoreIdOf('auto') },
    rival: { ...NORMAL, score_id: scoreIdOf('rival') },
}

const scoreRowOf = (trace: TraceUploadInput, userId: string, playedAt: number, overrides: { exScore: number; autoplay?: boolean }) => ({
    ...countTraceJudgements(trace),
    id: trace.score_id,
    userId,
    chartSha256: trace.chart.sha256,
    chartMd5: trace.chart.md5,
    mode: trace.mode,
    notes: trace.notes.time_us.length,
    maxExScore: trace.notes.time_us.length * 2,
    greenNumber: 500,
    playedAt,
    ...overrides,
})

const issued = { ids: 0 }

const CURRENT_VERSIONS = { play: PLAY_ANALYSIS_VERSION, pattern: ANALYSIS_VERSION }

const failures: unknown[] = []

const createService = () =>
    createTraceService({
        db: createTraceDb(getDb()),
        newId: (prefix) => {
            issued.ids += 1
            return `${prefix}${PREFIX}_${String(issued.ids).padStart(4, '0')}`
        },
        now: () => NOW,
        loadPattern: async () => null,
        reportAnalysisFailure: (error) => {
            failures.push(error)
        },
    })

describe.skipIf(!hasDatabase)('판별 기반 AI 기능 (임시 MySQL)', () => {
    const cleanup = async () => {
        const db = getDb()
        await db.delete(playTrace).where(like(playTrace.userId, `${PREFIX}_%`))
        await db.delete(chartBest).where(like(chartBest.userId, `${PREFIX}_%`))
        await db.delete(rival).where(like(rival.userId, `${PREFIX}_%`))
        await db.delete(score).where(like(score.id, `${PREFIX}_%`))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await db.delete(chart).where(eq(chart.sha256, NORMAL.chart.sha256))
    }

    beforeAll(async () => {
        await cleanup()
        const db = getDb()
        await db.insert(user).values([OWNER, RIVAL].map((id) => ({ id, name: `${id} name`, email: `${id}@w5pf.invalid`, loginId: id })))
        await db.insert(chart).values({ sha256: NORMAL.chart.sha256, md5: NORMAL.chart.md5, title: 'W5 Chart', artist: 'W5 Artist' })
        await db
            .insert(score)
            .values([
                scoreRowOf(TRACES.first, OWNER, NOW, { exScore: EX.first }),
                scoreRowOf(TRACES.best, OWNER, NOW + 1, { exScore: EX.best }),
                scoreRowOf(TRACES.autoplay, OWNER, NOW + 2, { exScore: EX.autoplay, autoplay: true }),
                scoreRowOf(TRACES.rival, RIVAL, NOW, { exScore: EX.rival }),
            ])
        await db.insert(chartBest).values({
            chartSha256: NORMAL.chart.sha256,
            userId: RIVAL,
            scoreId: TRACES.rival.score_id,
            clear: 5,
            exScore: EX.rival,
            minbp: 2,
            scratchAuto: false,
        })
        const service = createService()
        for (const [name, trace] of Object.entries(TRACES)) {
            const userId = name === 'rival' ? RIVAL : OWNER
            expect((await service.upload({ input: trace, pathHash: NORMAL.chart.md5, userId })).outcome).toBe(TRACE_UPLOAD_OUTCOME.CREATED)
        }
    })

    afterAll(cleanup)

    test('마이그레이션 0010 이 play_share_setting(rival_segments 기본 false)을 만들고 사용자를 지우면 같이 지워진다', async () => {
        const [columns] = await getDb().execute(
            sql`SELECT concat(column_name, ':', column_default) AS name FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = 'play_share_setting' AND column_name = 'rival_segments'`,
        )
        expect(Array.isArray(columns) ? columns.map((row) => String(Object.values(row)[0])) : []).toEqual(['rival_segments:0'])
        const db = getDb()
        const ghost = `${PREFIX}_ghost`
        await db.insert(user).values({ id: ghost, name: ghost, email: `${ghost}@w5pf.invalid`, loginId: ghost })
        await createTraceDb(db).saveShareSetting({ userId: ghost, rivalSegments: true, updatedAt: NOW })
        await db.delete(user).where(eq(user.id, ghost))
        expect(await db.select().from(playShareSetting).where(eq(playShareSetting.userId, ghost))).toEqual([])
    })

    test('최근 판은 최신순이고 오토플레이를 빼며, 곡 최고 판은 한 점수를 빼고 EX 가 가장 높은 판이다', async () => {
        const traceDb = createTraceDb(getDb())
        const recent = await traceDb.listRecentPlays({ userId: OWNER, limit: 10 })
        expect(recent.map((row) => row.scoreId)).toEqual([TRACES.best.score_id, TRACES.first.score_id])
        expect(recent[0]).toMatchObject({ score: { exScore: EX.best, maxExScore: NORMAL.notes.time_us.length * 2, greenNumber: 500, random: 'Off' } })
        expect(recent[0].analysis?.analysisVersion).toBe(PLAY_ANALYSIS_VERSION)
        const best = await traceDb.findBestPlayOnChart({
            userId: OWNER,
            chartSha256: NORMAL.chart.sha256,
            scratchAuto: false,
            excludeScoreId: TRACES.best.score_id,
        })
        expect(best?.scoreId).toBe(TRACES.first.score_id)
        expect(
            await traceDb.findBestPlayOnChart({ userId: OWNER, chartSha256: NORMAL.chart.sha256, scratchAuto: true, excludeScoreId: null }),
        ).toBeNull()
        expect(
            (await traceDb.listPlaysOfScores({ userId: RIVAL, scoreIds: [TRACES.first.score_id, TRACES.rival.score_id] })).map((row) => row.scoreId),
        ).toEqual([TRACES.rival.score_id])
    })

    test('재계산 대상은 분석이 없거나 판 분석 버전이 낮은 trace 를 id 순서로 이어서 고른다', async () => {
        const traceDb = createTraceDb(getDb())
        expect(await traceDb.listTracesBehind({ versions: CURRENT_VERSIONS, after: null, limit: 10 })).toEqual([])
        const next = { ...CURRENT_VERSIONS, play: PLAY_ANALYSIS_VERSION + 1 }
        const behind = await traceDb.listTracesBehind({ versions: next, after: null, limit: 2 })
        expect(behind).toHaveLength(2)
        const rest = await traceDb.listTracesBehind({ versions: next, after: behind[1].id, limit: 10 })
        expect(rest.every((row) => row.id > behind[1].id)).toBe(true)
    })

    test('요약의 곡별 분류 버전이 낮거나 없는 분석도 뒤처진 것으로 SQL 이 고르고 센다', async () => {
        const db = getDb()
        const traceDb = createTraceDb(db)
        const [target] = await db.select({ id: playTrace.id }).from(playTrace).where(eq(playTrace.scoreId, TRACES.first.score_id))
        const [stored] = await db.select().from(playAnalysis).where(eq(playAnalysis.traceId, target.id))
        const summary = stored.summary as Record<string, unknown>
        const before = await traceDb.countAnalysesBehind(CURRENT_VERSIONS)
        const mine = (rows: { id: string }[]) => rows.filter((row) => row.id === target.id)

        await db
            .update(playAnalysis)
            .set({ summary: { ...summary, pattern_analysis_version: ANALYSIS_VERSION - 1 } })
            .where(eq(playAnalysis.traceId, target.id))
        expect(mine(await traceDb.listTracesBehind({ versions: CURRENT_VERSIONS, after: null, limit: 1000 }))).toHaveLength(1)
        expect((await traceDb.countAnalysesBehind(CURRENT_VERSIONS)).stale).toBe(before.stale + 1)

        const withoutPatternVersion = Object.fromEntries(Object.entries(summary).filter(([key]) => key !== 'pattern_analysis_version'))
        await db.update(playAnalysis).set({ summary: withoutPatternVersion }).where(eq(playAnalysis.traceId, target.id))
        expect(mine(await traceDb.listTracesBehind({ versions: CURRENT_VERSIONS, after: null, limit: 1000 }))).toHaveLength(1)
        expect((await traceDb.countAnalysesBehind(CURRENT_VERSIONS)).stale).toBe(before.stale + 1)

        const service = createService()
        const read = await service.analyzedPlayOfScore({ userId: OWNER, scoreId: TRACES.first.score_id })
        expect(read?.summary.pattern_analysis_version).toBe(ANALYSIS_VERSION)
        expect(mine(await traceDb.listTracesBehind({ versions: CURRENT_VERSIONS, after: null, limit: 1000 }))).toEqual([])
        expect(await traceDb.countAnalysesBehind(CURRENT_VERSIONS)).toEqual(before)
    })

    test('읽을 수 없는 trace 는 던지지 않고 실패로 표시해 나머지로 답하며, 표시는 같은 버전에서 다시 계산하지 않는다', async () => {
        const db = getDb()
        const service = createService()
        const brokenScore = scoreIdOf('broken')
        const brokenTrace = { ...NORMAL, score_id: brokenScore }
        await db.insert(score).values(scoreRowOf(brokenTrace, OWNER, NOW + 3, { exScore: EX.first }))
        expect((await service.upload({ input: brokenTrace, pathHash: NORMAL.chart.md5, userId: OWNER })).outcome).toBe(TRACE_UPLOAD_OUTCOME.CREATED)
        const [row] = await db.select({ id: playTrace.id }).from(playTrace).where(eq(playTrace.scoreId, brokenScore))
        await db
            .update(playTrace)
            .set({ data: Buffer.from('not gzip') })
            .where(eq(playTrace.id, row.id))
        await db.delete(playAnalysis).where(eq(playAnalysis.traceId, row.id))

        const reported = failures.length
        const first = await service.recentAnalyzedPlays({ userId: OWNER, limit: 100 })
        expect(first.plays.map((play) => play.scoreId).toSorted()).toEqual([TRACES.best.score_id, TRACES.first.score_id].toSorted())
        expect([first.stale, first.failed]).toEqual([0, 1])
        expect(failures).toHaveLength(reported + 1)
        const [marker] = await db.select().from(playAnalysis).where(eq(playAnalysis.traceId, row.id))
        expect(marker).toMatchObject({ analysisVersion: PLAY_ANALYSIS_VERSION, patternJoin: 'failed', patternMd5: null })
        expect(marker.summary).toMatchObject({ failed: true, analysis_version: PLAY_ANALYSIS_VERSION, pattern_analysis_version: ANALYSIS_VERSION })

        const second = await service.recentAnalyzedPlays({ userId: OWNER, limit: 100 })
        expect([second.plays.length, second.failed]).toEqual([2, 1])
        expect(failures).toHaveLength(reported + 1)
        expect(await createTraceDb(db).listTracesBehind({ versions: CURRENT_VERSIONS, after: null, limit: 1000 })).toEqual([])
        await db.delete(playTrace).where(eq(playTrace.id, row.id))
        await db.delete(score).where(eq(score.id, brokenScore))
    })

    test('라이벌의 구간 요약은 서로 라이벌이고 허용했을 때만 나오고, 목록·공개 최고 기록은 compose 가 읽는다', async () => {
        const db = getDb()
        const service = createService()
        const compareDb = createAiPlayCompareDb(db)
        const params = { requesterId: OWNER, rivalId: RIVAL, chartSha256: NORMAL.chart.sha256, scratchAuto: false }
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
        await db.insert(rival).values({ userId: OWNER, rivalId: RIVAL })
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
        await service.putShareSetting({ userId: RIVAL, rivalSegments: true })
        await service.putShareSetting({ userId: RIVAL, rivalSegments: true })
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
        expect(await compareDb.listRivals(OWNER)).toEqual([{ userId: RIVAL, loginId: RIVAL, name: `${RIVAL} name`, allowsSegments: false }])

        await db.insert(rival).values({ userId: RIVAL, rivalId: OWNER })
        const shared = await service.rivalSegmentsOnChart(params)
        expect(shared.allowed && shared.play?.scoreId).toBe(TRACES.rival.score_id)
        expect(await compareDb.listRivals(OWNER)).toEqual([{ userId: RIVAL, loginId: RIVAL, name: `${RIVAL} name`, allowsSegments: true }])

        await service.putShareSetting({ userId: RIVAL, rivalSegments: false })
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
        expect(await compareDb.listRivals(OWNER)).toEqual([{ userId: RIVAL, loginId: RIVAL, name: `${RIVAL} name`, allowsSegments: false }])
        await service.putShareSetting({ userId: RIVAL, rivalSegments: true })

        expect(await compareDb.findPublicBest({ userId: RIVAL, chartSha256: NORMAL.chart.sha256, scratchAuto: false })).toMatchObject({
            scoreId: TRACES.rival.score_id,
            clear: 5,
            exScore: EX.rival,
            pgreat: countTraceJudgements(NORMAL).pgreat,
        })
        expect(await compareDb.findLoginId(OWNER)).toBe(OWNER)
        expect(await createPlayScoreDb(db).findOwnedPlayScore({ userId: OWNER, scoreId: TRACES.best.score_id })).toMatchObject({
            title: 'W5 Chart',
            artist: 'W5 Artist',
            exScore: EX.best,
            chartMd5: NORMAL.chart.md5,
        })
        expect(await createPlayScoreDb(db).findOwnedPlayScore({ userId: OWNER, scoreId: TRACES.rival.score_id })).toBeNull()
    })

    test('판정 습관 행은 trace 가 있는 판에 그 판의 GREEN 과 노트 단위 오차를 붙인다', async () => {
        const rows = await createAiJudgeHabitDb(getDb(), createService()).listRecentTimingRows({ userId: OWNER, limit: 10 })
        expect(rows).toHaveLength(2)
        expect(rows[0].traceGreen).toEqual({ green: NORMAL.speed.end.green, greenMs: NORMAL.speed.end.green_ms })
        expect(rows[0].traceTiming?.notes).toBeGreaterThan(0)
        expect(rows[0]).toMatchObject({ greenNumber: 500 })
    })
})

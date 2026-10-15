import { afterAll, beforeAll, describe, expect, mock, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { inArray, like } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { apiToken, chart, chartBest, courseBest, courseScore, score } from '@server/db/schema'
import { hashApiToken } from '@server/lib/token'
import type { createChartUpsertRoute } from '@server/route/chart'
import type { createCourseSubmitRoute } from '@server/route/course'
import type { createSubmitScoreRoute } from '@server/route/score'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 제출 검증 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 's19i'
const PLAYERS = ['first', 'second'] as const
const BASE_PLAYED_AT = 1_791_000_000_000
const PLAY_SPACING_MS = 1_000
const CREATED = 201
const BAD_REQUEST = 400
const POISONED_SHA = '1'.repeat(64)
const PLANTED_SHA = '2'.repeat(64)
const FORGED_SHA = '3'.repeat(64)
const CHART_SHAS = [POISONED_SHA, PLANTED_SHA, FORGED_SHA]
const COURSE_HASH = '4'.repeat(64)
const REAL_NOTES = 1500
const REAL_PGREAT = 1400
const CONTRACT_COURSE = join(import.meta.dirname, '..', '..', '..', 'contract', 'client-requests', 'course-normal.json')

type Player = (typeof PLAYERS)[number]

const loginOf = (player: Player) => `${PREFIX}_${player}`
const tokenOf = (player: Player) => `rbms_${PREFIX}_${player}_token`
const json = (player: Player) => ({ 'content-type': 'application/json', authorization: `Bearer ${tokenOf(player)}` })
const emptyContext = { params: Promise.resolve({}) }

const zeroJudge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

const scoreBody = (player: Player, play: { sha256: string; totalNotes: number; judge: Record<string, number>; offset: number }) => ({
    api_version: 1,
    chart: { md5: '', sha256: play.sha256 },
    player: { id: loginOf(player) },
    mode: 'BEAT_7K',
    clear: 'Normal',
    ex_score: 0,
    max_ex_score: play.totalNotes * 2,
    judge: { ...zeroJudge, ...play.judge },
    max_combo: 1,
    total_notes: play.totalNotes,
    minbp: 0,
    gauge_value: 80,
    options: { gauge: 'Normal', random: 'Off', scratch_auto: false, lntype: 0, input_device: 'keyboard', assist: [], judge_rate: 100 },
    played_at: BASE_PLAYED_AT + play.offset * PLAY_SPACING_MS,
})

const realPlay = (player: Player, sha256: string, offset: number) =>
    scoreBody(player, { sha256, totalNotes: REAL_NOTES, judge: { pgreat: REAL_PGREAT, great: REAL_NOTES - REAL_PGREAT }, offset })

describe.skipIf(!hasDatabase)('제출 검증과 차트 노트 수 (임시 MySQL)', () => {
    let submitScoreRoute: ReturnType<typeof createSubmitScoreRoute>
    let submitCourseRoute: ReturnType<typeof createCourseSubmitRoute>
    let upsertChartRoute: ReturnType<typeof createChartUpsertRoute>

    const post = (path: string, player: Player, body: unknown) =>
        new Request(`http://localhost:3000${path}`, { method: 'POST', body: JSON.stringify(body), headers: json(player) })

    const submitScore = (player: Player, body: unknown) => submitScoreRoute(post('/api/scores', player, body), emptyContext)

    const cleanUp = async () => {
        const db = getDb()
        const ids = PLAYERS.map(loginOf)
        await db.delete(score).where(inArray(score.userId, ids))
        await db.delete(chartBest).where(inArray(chartBest.userId, ids))
        await db.delete(courseScore).where(inArray(courseScore.userId, ids))
        await db.delete(courseBest).where(inArray(courseBest.userId, ids))
        await db.delete(chart).where(inArray(chart.sha256, CHART_SHAS))
        await db.delete(apiToken).where(inArray(apiToken.userId, ids))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    }

    beforeAll(async () => {
        mock.module('next/cache', () => ({ cacheLife: () => undefined, cacheTag: () => undefined, revalidateTag: () => undefined }))
        submitScoreRoute = (await import('@server/route/score')).createSubmitScoreRoute()
        submitCourseRoute = (await import('@server/route/course')).createCourseSubmitRoute()
        upsertChartRoute = (await import('@server/route/chart')).createChartUpsertRoute()
        await cleanUp()
        const db = getDb()
        await db
            .insert(user)
            .values(
                PLAYERS.map((player) => ({ id: loginOf(player), name: player, email: `${loginOf(player)}@s19.invalid`, loginId: loginOf(player) })),
            )
        await db
            .insert(apiToken)
            .values(PLAYERS.map((player) => ({ id: `tk_${loginOf(player)}`, userId: loginOf(player), tokenHash: hashApiToken(tokenOf(player)) })))
    })

    afterAll(cleanUp)

    test('차트를 처음 만든 제출의 total_notes 가 다른 사용자의 정상 판을 막지 않는다', async () => {
        const first = await submitScore('first', scoreBody('first', { sha256: POISONED_SHA, totalNotes: 1, judge: { pgreat: 1 }, offset: 0 }))
        expect(first.status).toBe(CREATED)
        const real = await submitScore('second', realPlay('second', POISONED_SHA, 1))
        expect(real.status).toBe(CREATED)
    })

    test('일반 사용자 토큰으로 POST /api/charts 에 미리 심은 notes 도 정상 판을 막지 않는다', async () => {
        const planted = await upsertChartRoute(post('/api/charts', 'first', { chart: { sha256: PLANTED_SHA, notes: 1 } }), emptyContext)
        expect(planted.status).toBe(200)
        const real = await submitScore('second', realPlay('second', PLANTED_SHA, 2))
        expect(real.status).toBe(CREATED)
    })

    test('그런 차트가 든 코스도 막지 않는다', async () => {
        const course = {
            course_hash: COURSE_HASH,
            player: { id: loginOf('second') },
            clear: 'Normal',
            ex_score: REAL_PGREAT * 2 * 2,
            max_ex_score: REAL_NOTES * 2 * 2,
            judge: { ...zeroJudge, pgreat: REAL_PGREAT * 2 },
            minbp: 0,
            charts: [
                { md5: '', sha256: POISONED_SHA },
                { md5: '', sha256: PLANTED_SHA },
            ],
            played_at: BASE_PLAYED_AT + 3 * PLAY_SPACING_MS,
        }
        const response = await submitCourseRoute(post('/api/courses', 'second', course), emptyContext)
        expect(response.status).toBe(CREATED)
    })

    test('분리 판정(epg)을 키워 노트 수를 넘긴 위조는 라우트에서 판정 필드로 거절한다', async () => {
        const forged = { ...scoreBody('first', { sha256: FORGED_SHA, totalNotes: 500, judge: { epg: 100_000 }, offset: 4 }), ex_score: 200_000 }
        const response = await submitScore('first', forged)
        expect(response.status).toBe(BAD_REQUEST)
        expect(await response.json()).toMatchObject({ error: { code: 'VALIDATION_ERROR', details: { field: 'judge' } } })
    })

    test('코스 본문의 판정 합계를 저장하고 순위가 그대로 돌려준다 (계약 본문 course-normal.json)', async () => {
        const contract = JSON.parse(readFileSync(CONTRACT_COURSE, 'utf8'))
        const body = { ...contract, player: { id: loginOf('first') } }
        const response = await submitCourseRoute(post('/api/courses', 'first', body), emptyContext)
        expect(response.status).toBe(CREATED)
        const [row] = await compose().courseService.getRanking({ courseHash: contract.course_hash, limit: 10, page: 1, includeScratchAuto: false })
        expect(row.judge).toMatchObject({ pgreat: 56, great: 0, poor: 0, fast: 56 })
    })
})

import { afterAll, beforeAll, describe, expect, mock, test } from 'bun:test'
import { and, count, eq, inArray, like } from 'drizzle-orm'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { composeChartBlocklist } from '@server/compose/chart-blocklist'
import { apiToken, chart, chartBest, chartBlocklist, course, courseChart, score, submissionAudit } from '@server/db/schema'
import { hashApiToken } from '@server/lib/token'
import type { createChartUpsertRoute } from '@server/route/chart'
import type { createSubmitScoreRoute } from '@server/route/score'
import { purgeBlockedCharts } from '../../scripts/chart-blocklist-purge'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 차트 차단 목록 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'cbl'
const LOGIN = `${PREFIX}_player`
const TOKEN = `rbms_${PREFIX}_player_token`
const BASE_PLAYED_AT = 1_793_000_000_000
const PLAY_SPACING_MS = 1_000
const CREATED = 201
const BLOCKED = 422
const NOTES = 1000
const PGREAT = 900
const PURGED_SHA = 'e1'.repeat(32)
const COURSE_SHA = 'e2'.repeat(32)
const UNLISTED_SHA = 'e3'.repeat(32)
const FAKE_SHAS = [PURGED_SHA, COURSE_SHA, UNLISTED_SHA]
const COURSE_HASH = `${PREFIX}_course`
const BLOCKLIST_SOURCE = { id: `${PREFIX}-source`, label: 'blocklist test' }
const blocked = { sha256: 'e4'.repeat(32), md5: 'e5'.repeat(16), path: 'blocked.bme' }
const emptyContext = { params: Promise.resolve({}) }
const headers = { 'content-type': 'application/json', authorization: `Bearer ${TOKEN}` }

const zeroJudge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

const scoreBody = (chartHashes: { md5: string; sha256: string }, offset: number) => ({
    api_version: 1,
    chart: chartHashes,
    player: { id: LOGIN },
    mode: 'BEAT_7K',
    clear: 'Normal',
    ex_score: PGREAT * 2 + (NOTES - PGREAT),
    max_ex_score: NOTES * 2,
    judge: { ...zeroJudge, pgreat: PGREAT, great: NOTES - PGREAT },
    max_combo: NOTES,
    total_notes: NOTES,
    minbp: 0,
    gauge_value: 80,
    options: { gauge: 'Normal', random: 'Off', scratch_auto: false, lntype: 0, input_device: 'keyboard', assist: [], judge_rate: 100 },
    played_at: BASE_PLAYED_AT + offset * PLAY_SPACING_MS,
})

const countByTarget = (rows: { target: string; rows: number }[]) => Object.fromEntries(rows.map((row) => [row.target, row.rows]))

describe.skipIf(!hasDatabase)('차트 차단 목록 (임시 MySQL)', () => {
    let submitScoreRoute: ReturnType<typeof createSubmitScoreRoute>
    let upsertChartRoute: ReturnType<typeof createChartUpsertRoute>
    const blockedShas = [blocked.sha256]
    const blocklistService = () => composeChartBlocklist(getDb()).chartBlocklistService

    const post = (path: string, body: unknown) => new Request(`http://localhost:3000${path}`, { method: 'POST', body: JSON.stringify(body), headers })

    const chartRows = async (sha256s: string[]) =>
        (await getDb().select({ value: count() }).from(chart).where(inArray(chart.sha256, sha256s)))[0].value

    const cleanUp = async () => {
        const db = getDb()
        await db.delete(score).where(eq(score.userId, LOGIN))
        await db.delete(chartBest).where(eq(chartBest.userId, LOGIN))
        await db.delete(submissionAudit).where(eq(submissionAudit.userId, LOGIN))
        await db.delete(courseChart).where(eq(courseChart.courseHash, COURSE_HASH))
        await db.delete(course).where(eq(course.courseHash, COURSE_HASH))
        await db.delete(chart).where(inArray(chart.sha256, [...FAKE_SHAS, ...blockedShas]))
        await db.delete(chartBlocklist).where(eq(chartBlocklist.sourceId, BLOCKLIST_SOURCE.id))
        await db.delete(apiToken).where(eq(apiToken.userId, LOGIN))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    }

    beforeAll(async () => {
        mock.module('next/cache', () => ({ cacheLife: () => undefined, cacheTag: () => undefined, revalidateTag: () => undefined }))
        submitScoreRoute = (await import('@server/route/score')).createSubmitScoreRoute()
        upsertChartRoute = (await import('@server/route/chart')).createChartUpsertRoute()
        await cleanUp()
        const db = getDb()
        await db.insert(user).values({ id: LOGIN, name: 'player', email: `${LOGIN}@cbl.invalid`, loginId: LOGIN })
        await db.insert(apiToken).values({ id: `tk_${LOGIN}`, userId: LOGIN, tokenHash: hashApiToken(TOKEN) })
        await blocklistService().register({ source: BLOCKLIST_SOURCE, charts: [blocked], addedBy: LOGIN })
    })

    afterAll(cleanUp)

    test('chart_blocklist 에 같은 차트를 다시 넣거나 md5 만 같은 차트를 넣으면 세기만 하고 넣지 않으며, sha256·md5 로 찾는다', async () => {
        const again = await blocklistService().register({
            source: BLOCKLIST_SOURCE,
            charts: [blocked, { sha256: 'e6'.repeat(32), md5: blocked.md5, path: 'same-md5.bme' }],
            addedBy: LOGIN,
        })
        expect(again).toEqual({ received: 2, added: 0, alreadyListed: 2 })
        expect(await blocklistService().isBlocked({ sha256: blocked.sha256 })).toBe(true)
        expect(await blocklistService().isBlocked({ md5: blocked.md5, sha256: UNLISTED_SHA })).toBe(true)
        expect(await blocklistService().isBlocked({ md5: '', sha256: UNLISTED_SHA })).toBe(false)
        expect(await blocklistService().listSources()).toContainEqual({
            sourceId: BLOCKLIST_SOURCE.id,
            sourceLabel: BLOCKLIST_SOURCE.label,
            charts: 1,
        })
        const { sha256s } = await blocklistService().listHashes()
        expect(sha256s).toContain(blocked.sha256)
    })

    test('목록의 차트로 낸 점수는 422 IR_CHART_BLOCKED 로 거절하고 감사에 남기며 차트 행을 만들지 않는다', async () => {
        const response = await submitScoreRoute(post('/api/scores', scoreBody({ md5: blocked.md5, sha256: blocked.sha256 }, 0)), emptyContext)
        expect(response.status).toBe(BLOCKED)
        expect(await response.json()).toMatchObject({ error: { code: 'IR_CHART_BLOCKED' } })
        expect(await chartRows(blockedShas)).toBe(0)
        const [rejected] = await getDb()
            .select({ value: count() })
            .from(submissionAudit)
            .where(and(eq(submissionAudit.userId, LOGIN), eq(submissionAudit.accepted, false)))
        expect(rejected.value).toBe(1)
    })

    test('md5 만 맞아도 거절한다', async () => {
        const response = await submitScoreRoute(post('/api/scores', scoreBody({ md5: blocked.md5, sha256: UNLISTED_SHA }, 1)), emptyContext)
        expect(response.status).toBe(BLOCKED)
        expect(await chartRows([UNLISTED_SHA])).toBe(0)
    })

    test('POST /api/charts 로도 목록의 차트를 만들지 못한다', async () => {
        const response = await upsertChartRoute(post('/api/charts', { chart: { sha256: blocked.sha256, notes: NOTES } }), emptyContext)
        expect(response.status).toBe(BLOCKED)
        expect(await chartRows(blockedShas)).toBe(0)
    })

    test('정리 스크립트는 count 로 세기만 하고 delete 로 지우며, 코스가 쓰는 차트 행과 목록 밖의 기록은 남긴다', async () => {
        const submitted = await Promise.all(
            FAKE_SHAS.map((sha256, index) => submitScoreRoute(post('/api/scores', scoreBody({ md5: '', sha256 }, 10 + index)), emptyContext)),
        )
        expect(submitted.map((response) => response.status)).toEqual([CREATED, CREATED, CREATED])
        await getDb().insert(course).values({ courseHash: COURSE_HASH, name: 'blocklist course' })
        await getDb().insert(courseChart).values({ courseHash: COURSE_HASH, position: 0, chartSha256: COURSE_SHA })
        const hashes = { sha256s: [PURGED_SHA, COURSE_SHA], md5s: [] }

        const counted = await purgeBlockedCharts({ command: 'count', db: getDb(), hashes })
        expect(counted.after).toBeNull()
        expect(countByTarget(counted.before)).toMatchObject({ score: 2, chart_best: 2, chart: 2, course_chart_kept: 1 })
        expect(await chartRows([PURGED_SHA, COURSE_SHA])).toBe(2)

        const purged = await purgeBlockedCharts({ command: 'delete', db: getDb(), hashes })
        expect(countByTarget(purged.before)).toMatchObject({ score: 2, chart: 2 })
        expect(countByTarget(purged.after ?? [])).toMatchObject({ score: 0, chart_best: 0, submission_audit: 0, chart: 1, course_chart_kept: 1 })
        expect(await chartRows([COURSE_SHA])).toBe(1)
        const [kept] = await getDb().select({ value: count() }).from(score).where(eq(score.chartSha256, UNLISTED_SHA))
        expect(kept.value).toBe(1)
    })
})

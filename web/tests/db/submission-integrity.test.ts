import { afterAll, beforeAll, describe, expect, mock, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { and, count, eq, inArray, like } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { apiToken, chart, chartBest, courseBest, courseScore, score, submissionAudit } from '@server/db/schema'
import { FLOAT_COLUMN_MAX, INT_COLUMN_MAX, INT_COLUMN_MIN, JSON_COLUMN_DEPTH_MAX, JSON_KEY_MAX_BYTES } from '@server/dto/common'
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
const EDGE_SHA = '7'.repeat(64)
const SPLIT_SHA = '8'.repeat(64)
const OUT_OF_RANGE_SHAS = ['a', 'b', 'c', 'd', 'e', 'f'].map((digit) => digit.repeat(64))
const LONE_SURROGATE_SHAS = ['5', '6'].map((digit) => digit.repeat(64))
const KEY_TOO_LONG_SHAS = ['0b', '0d'].map((pair) => pair.repeat(32))
const KEY_EDGE_SHA = '0c'.repeat(32)
const CHART_SHAS = [
    POISONED_SHA,
    PLANTED_SHA,
    FORGED_SHA,
    EDGE_SHA,
    SPLIT_SHA,
    KEY_EDGE_SHA,
    ...OUT_OF_RANGE_SHAS,
    ...LONE_SURROGATE_SHAS,
    ...KEY_TOO_LONG_SHAS,
]
const THREE_BYTE_CHARACTER = '가'
const THREE_BYTE_KEY_MAX = JSON_KEY_MAX_BYTES / 3
const LONE_HIGH_SURROGATE = String.fromCharCode(0xd800)
const LONE_LOW_SURROGATE = String.fromCharCode(0xdc00)
const OUT_OF_RANGE_COURSE_HASH = '9'.repeat(64)
const PAST_INT = INT_COLUMN_MAX + 1
const HALF_PAST_INT = 1_500_000_000
const PAST_FLOAT = 1e300
const GREEN_NUMBER_PAST_INT = 1e12
const TEXT_PAST_COLUMN = 'X'.repeat(70)
const COURSE_HASH = '4'.repeat(64)
const REAL_NOTES = 1500
const REAL_PGREAT = 1400
const CONTRACT_COURSE = join(import.meta.dirname, '..', '..', '..', 'contract', 'client-requests', 'course-normal.json')

type Player = (typeof PLAYERS)[number]

const loginOf = (player: Player) => `${PREFIX}_${player}`
const tokenOf = (player: Player) => `rbms_${PREFIX}_${player}_token`
const json = (player: Player) => ({ 'content-type': 'application/json', authorization: `Bearer ${tokenOf(player)}` })
const emptyContext = { params: Promise.resolve({}) }

const nestedArray = (depth: number): unknown => (depth <= 1 ? [] : [nestedArray(depth - 1)])

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
        await db.delete(submissionAudit).where(inArray(submissionAudit.userId, ids))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    }

    const chartRowCount = async (sha256s: string[]) =>
        (await getDb().select({ value: count() }).from(chart).where(inArray(chart.sha256, sha256s)))[0].value

    const rejectedAuditCount = async (player: Player) =>
        (
            await getDb()
                .select({ value: count() })
                .from(submissionAudit)
                .where(and(eq(submissionAudit.userId, loginOf(player)), eq(submissionAudit.accepted, false)))
        )[0].value

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

    test('DB 열을 넘는 수를 가진 점수 본문은 400 으로 거절하고 거절을 감사에 남기며 차트 행을 쓰지 않는다', async () => {
        const [totalPastInt, exPastInt, gaugePastFloat, popnBpPastInt, offsetPastInt, extraTooDeep] = OUT_OF_RANGE_SHAS
        const bodies = [
            scoreBody('first', { sha256: totalPastInt, totalNotes: PAST_INT, judge: { pgreat: PAST_INT }, offset: 10 }),
            {
                ...scoreBody('first', { sha256: exPastInt, totalNotes: HALF_PAST_INT, judge: { pgreat: HALF_PAST_INT }, offset: 11 }),
                max_ex_score: 0,
            },
            { ...scoreBody('first', { sha256: gaugePastFloat, totalNotes: 10, judge: { pgreat: 10 }, offset: 12 }), gauge_value: PAST_FLOAT },
            {
                ...scoreBody('first', { sha256: popnBpPastInt, totalNotes: 10, judge: { bad: INT_COLUMN_MAX, miss: INT_COLUMN_MAX }, offset: 13 }),
                mode: 'POPN_9K',
                max_ex_score: 0,
            },
            (() => {
                const body = scoreBody('first', { sha256: offsetPastInt, totalNotes: 10, judge: { pgreat: 10 }, offset: 14 })
                return { ...body, options: { ...body.options, offset_ms: PAST_INT } }
            })(),
            {
                ...scoreBody('first', { sha256: extraTooDeep, totalNotes: 10, judge: { pgreat: 10 }, offset: 15 }),
                extra: { deep: nestedArray(JSON_COLUMN_DEPTH_MAX) },
            },
        ]
        const auditsBefore = await rejectedAuditCount('first')
        for (const body of bodies) {
            const response = await submitScore('first', body)
            expect(response.status).toBe(BAD_REQUEST)
            expect(await response.json()).toMatchObject({ error: { code: 'VALIDATION_ERROR' } })
        }
        expect(await rejectedAuditCount('first')).toBe(auditsBefore + bodies.length)
        expect(await chartRowCount(OUT_OF_RANGE_SHAS)).toBe(0)
    })

    test('extra 의 값이나 키에 짝 없는 서로게이트가 있으면 DB 의 JSON 이 받지 못하므로 400 으로 거절하고 차트 행을 쓰지 않는다', async () => {
        const [inValue, inKey] = LONE_SURROGATE_SHAS
        const bodies = [
            {
                ...scoreBody('first', { sha256: inValue, totalNotes: 10, judge: { pgreat: 10 }, offset: 16 }),
                extra: { note: [`a${LONE_HIGH_SURROGATE}`] },
            },
            { ...scoreBody('first', { sha256: inKey, totalNotes: 10, judge: { pgreat: 10 }, offset: 17 }), extra: { [LONE_LOW_SURROGATE]: 1 } },
        ]
        for (const body of bodies) {
            const response = await submitScore('first', body)
            expect(response.status).toBe(BAD_REQUEST)
            expect(await response.json()).toMatchObject({ error: { code: 'VALIDATION_ERROR' } })
        }
        expect(await chartRowCount(LONE_SURROGATE_SHAS)).toBe(0)
    })

    test('extra 의 키가 DB 의 JSON 키 길이(UTF-8 바이트)를 넘으면 400 으로 거절하고 거절을 감사에 남기며 차트 행을 쓰지 않는다', async () => {
        const [asciiPast, multiBytePast] = KEY_TOO_LONG_SHAS
        const bodies = [
            {
                ...scoreBody('first', { sha256: asciiPast, totalNotes: 10, judge: { pgreat: 10 }, offset: 18 }),
                extra: { ['k'.repeat(JSON_KEY_MAX_BYTES + 1)]: 1 },
            },
            {
                ...scoreBody('first', { sha256: multiBytePast, totalNotes: 10, judge: { pgreat: 10 }, offset: 19 }),
                extra: { [THREE_BYTE_CHARACTER.repeat(THREE_BYTE_KEY_MAX + 1)]: 1 },
            },
        ]
        const auditsBefore = await rejectedAuditCount('first')
        for (const body of bodies) {
            const response = await submitScore('first', body)
            expect(response.status).toBe(BAD_REQUEST)
            expect(await response.json()).toMatchObject({ error: { code: 'VALIDATION_ERROR' } })
        }
        expect(await rejectedAuditCount('first')).toBe(auditsBefore + bodies.length)
        expect(await chartRowCount(KEY_TOO_LONG_SHAS)).toBe(0)
    })

    test('extra 의 키가 DB 의 JSON 키 길이에 닿으면 받아 그대로 저장한다', async () => {
        const asciiKey = 'k'.repeat(JSON_KEY_MAX_BYTES)
        const multiByteKey = THREE_BYTE_CHARACTER.repeat(THREE_BYTE_KEY_MAX)
        const body = {
            ...scoreBody('second', { sha256: KEY_EDGE_SHA, totalNotes: 10, judge: { pgreat: 10 }, offset: 31 }),
            extra: { [asciiKey]: 1, [multiByteKey]: 2 },
        }
        const response = await submitScore('second', body)
        expect(response.status).toBe(CREATED)
        const [stored] = await getDb().select({ extra: score.extra }).from(score).where(eq(score.chartSha256, KEY_EDGE_SHA))
        expect(stored.extra).toEqual({ [asciiKey]: 1, [multiByteKey]: 2 })
    })

    test('DB 열을 넘는 코스 본문도 400 으로 거절하고 행을 쓰지 않는다', async () => {
        const course = (judge: Record<string, number>, offset: number) => ({
            course_hash: OUT_OF_RANGE_COURSE_HASH,
            player: { id: loginOf('first') },
            clear: 'Normal',
            ex_score: 0,
            max_ex_score: INT_COLUMN_MAX,
            judge: { ...zeroJudge, ...judge },
            minbp: 0,
            played_at: BASE_PLAYED_AT + offset * PLAY_SPACING_MS,
        })
        for (const body of [course({ pgreat: PAST_INT }, 20), course({ bad: INT_COLUMN_MAX, miss: INT_COLUMN_MAX }, 21)]) {
            const response = await submitCourseRoute(post('/api/courses', 'first', body), emptyContext)
            expect(response.status).toBe(BAD_REQUEST)
        }
        const [{ value }] = await getDb().select({ value: count() }).from(courseScore).where(eq(courseScore.courseHash, OUT_OF_RANGE_COURSE_HASH))
        expect(value).toBe(0)
    })

    test('열의 끝에 닿는 본문은 받아 그대로 저장하고, 긴 글자는 열 길이로 잘라 저장한다', async () => {
        const halfMax = Math.floor(INT_COLUMN_MAX / 2)
        const base = scoreBody('second', { sha256: EDGE_SHA, totalNotes: INT_COLUMN_MAX, judge: { pgreat: halfMax, great: 1 }, offset: 30 })
        const body = {
            ...base,
            mode: `BEAT_7K${TEXT_PAST_COLUMN}`,
            max_ex_score: INT_COLUMN_MAX,
            max_combo: INT_COLUMN_MAX,
            passnotes: INT_COLUMN_MAX,
            gauge_value: FLOAT_COLUMN_MAX,
            judge: { ...base.judge, fast: INT_COLUMN_MAX, slow: INT_COLUMN_MAX, combobreak: INT_COLUMN_MAX, empty_poor: INT_COLUMN_MAX },
            judge_algorithm: TEXT_PAST_COLUMN,
            rule: TEXT_PAST_COLUMN,
            skin: TEXT_PAST_COLUMN,
            client: TEXT_PAST_COLUMN,
            client_platform: TEXT_PAST_COLUMN,
            extra: { deep: nestedArray(JSON_COLUMN_DEPTH_MAX - 1) },
            options: {
                ...base.options,
                lntype: INT_COLUMN_MAX,
                judge_rate: INT_COLUMN_MAX,
                offset_ms: INT_COLUMN_MIN,
                lift: FLOAT_COLUMN_MAX,
                lane_cover: -FLOAT_COLUMN_MAX,
                green_number: GREEN_NUMBER_PAST_INT,
                input_device: TEXT_PAST_COLUMN,
            },
        }
        const response = await submitScore('second', body)
        expect(response.status).toBe(CREATED)
        const [stored] = await getDb().select().from(score).where(eq(score.chartSha256, EDGE_SHA))
        expect(stored).toMatchObject({
            exScore: INT_COLUMN_MAX,
            maxCombo: INT_COLUMN_MAX,
            notes: INT_COLUMN_MAX,
            offsetMs: INT_COLUMN_MIN,
            greenNumber: null,
            mode: `BEAT_7K${TEXT_PAST_COLUMN}`.slice(0, 16),
            skin: TEXT_PAST_COLUMN.slice(0, 64),
            inputDevice: TEXT_PAST_COLUMN.slice(0, 32),
            clientPlatform: TEXT_PAST_COLUMN.slice(0, 32),
        })
        const [chartRow] = await getDb().select({ mode: chart.mode, notes: chart.notes }).from(chart).where(eq(chart.sha256, EDGE_SHA))
        expect(chartRow).toEqual({ mode: `BEAT_7K${TEXT_PAST_COLUMN}`.slice(0, 16), notes: INT_COLUMN_MAX })
    })

    test('점수 순위, best, 상세, 플레이어 기록은 검사가 읽은 분리 합을 판정으로 돌려준다', async () => {
        const forged = scoreBody('second', { sha256: SPLIT_SHA, totalNotes: 10, judge: { epg: 8, egr: 1, epr: 1, pgreat: 10 }, offset: 40 })
        const response = await submitScore('second', forged)
        expect(response.status).toBe(CREATED)
        const { score_id: scoreId } = await response.json()
        const split = { pgreat: 8, great: 1, poor: 1, epg: 8, egr: 1, epr: 1 }
        const services = compose()
        const [ranked] = await services.scoreService.getRanking({ chartSha256: SPLIT_SHA, limit: 10, page: 1, includeScratchAuto: false })
        expect(ranked).toMatchObject({ ex_score: 17, minbp: 1, judge: split })
        const best = await services.scoreService.getBest({ chartSha256: SPLIT_SHA, loginId: loginOf('second'), includeScratchAuto: false })
        expect(best?.judge).toMatchObject(split)
        const detail = await services.scoreService.getById(scoreId)
        expect(detail?.judge).toMatchObject(split)
        const history = await services.listPlayerScoreRows({ userId: loginOf('second'), limit: 50, page: 1 })
        expect(history.find((row) => row.scoreId === scoreId)?.judge).toMatchObject(split)
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

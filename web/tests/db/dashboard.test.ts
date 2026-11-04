import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { eq, inArray, like } from 'drizzle-orm'
import { composeDashboard } from '@server/compose/dashboard'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, course, courseBest, courseScore, replay, rival, score } from '@server/db/schema'
import { clearLampToId, type ClearLamp } from '@server/dto/common'
import { dashboardCalendarOutputSchema, dashboardOverviewOutputSchema } from '@server/dto/dashboard'
import { toDashboardRank } from '@server/service/domain/dashboard/dashboard.service'
import { MINUTE_MS, TODAY_START, noonOf } from '../service/dashboard/dashboard-fixture'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 대시보드 집계 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'u5d'
const NOTES = 1000
const CHART_SEEDS = { a: 7101, b: 7102, c: 7103, d: 7104, w: 7105, x: 7106, y: 7107, z: 7108, v: 7109 } as const

type ChartKey = keyof typeof CHART_SEEDS
type Owner = 'me' | 'r1' | 'r2' | 'o1' | 'none'

const OWNERS: Owner[] = ['me', 'r1', 'r2', 'o1', 'none']
const userId = (owner: Owner) => `${PREFIX}_${owner}`
const hexOf = (seed: number, length: number) => seed.toString(16).padStart(length, '0')
const shaOf = (key: ChartKey) => hexOf(CHART_SEEDS[key], 64)
const md5Of = (key: ChartKey) => hexOf(CHART_SEEDS[key], 32)

type Play = {
    id: string
    owner: Owner
    chart: ChartKey
    clear: ClearLamp
    exScore: number
    playedAt: number
    scratchAuto?: boolean
    ranked?: boolean
    notes?: number
}

const PLAYS: Play[] = [
    { id: 'me1', owner: 'me', chart: 'a', clear: 'Hard', exScore: 1800, playedAt: noonOf(3) },
    { id: 'me7', owner: 'me', chart: 'w', clear: 'Easy', exScore: 1000, playedAt: noonOf(3, 4) },
    { id: 'me2', owner: 'me', chart: 'b', clear: 'Failed', exScore: 600, playedAt: noonOf(2) },
    { id: 'me3', owner: 'me', chart: 'c', clear: 'Easy', exScore: 1000, playedAt: noonOf(2, 5) },
    { id: 'me4', owner: 'me', chart: 'c', clear: 'FullCombo', exScore: 1500, playedAt: noonOf(1), scratchAuto: true },
    { id: 'me5', owner: 'me', chart: 'd', clear: 'Normal', exScore: 1400, playedAt: noonOf(1, 3), notes: 0 },
    { id: 'me6', owner: 'me', chart: 'a', clear: 'Normal', exScore: 900, playedAt: noonOf(0) },
    { id: 'r1a', owner: 'r1', chart: 'a', clear: 'Hard', exScore: 1700, playedAt: noonOf(5) },
    { id: 'r1b', owner: 'r1', chart: 'b', clear: 'Failed', exScore: 600, playedAt: noonOf(4) },
    { id: 'r1c', owner: 'r1', chart: 'c', clear: 'Hard', exScore: 1200, playedAt: noonOf(3) },
    { id: 'r1x', owner: 'r1', chart: 'x', clear: 'Hard', exScore: 1000, playedAt: noonOf(2) },
    { id: 'r1u', owner: 'r1', chart: 'x', clear: 'Easy', exScore: 100, playedAt: noonOf(0, 9), ranked: false },
    { id: 'r2d', owner: 'r2', chart: 'd', clear: 'Normal', exScore: 1000, playedAt: noonOf(1) },
    { id: 'r2y', owner: 'r2', chart: 'y', clear: 'Hard', exScore: 1000, playedAt: noonOf(0, 1) },
    { id: 'r2z', owner: 'r2', chart: 'z', clear: 'Hard', exScore: 1000, playedAt: noonOf(0, 2) },
    { id: 'r2v', owner: 'r2', chart: 'v', clear: 'Failed', exScore: 100, playedAt: noonOf(0, 3) },
    { id: 'o1a', owner: 'o1', chart: 'a', clear: 'Failed', exScore: 100, playedAt: noonOf(0, 8) },
]

const BEST_PLAY_IDS = ['me1', 'me7', 'me2', 'me3', 'me4', 'me5', 'r1a', 'r1b', 'r1c', 'r1x', 'r2d', 'r2y', 'r2z', 'r2v', 'o1a']

const COURSE_HASHES = { first: `${PREFIX}-course-first`, second: `${PREFIX}-course-second` }

const scoreRowOf = (play: Play) => ({
    id: `${PREFIX}_${play.id}`,
    userId: userId(play.owner),
    chartSha256: shaOf(play.chart),
    chartMd5: md5Of(play.chart),
    mode: 'BEAT_7K',
    clear: clearLampToId(play.clear),
    exScore: play.exScore,
    notes: play.notes ?? NOTES,
    scratchAuto: play.scratchAuto ?? false,
    ranked: play.ranked ?? true,
    playedAt: play.playedAt,
})

const bestRowOf = (play: Play) => ({
    chartSha256: shaOf(play.chart),
    userId: userId(play.owner),
    scoreId: `${PREFIX}_${play.id}`,
    clear: clearLampToId(play.clear),
    exScore: play.exScore,
    scratchAuto: play.scratchAuto ?? false,
})

const unusedTables = { getDetail: async () => null, getRecords: async () => null }

describe.skipIf(!hasDatabase)('대시보드 집계 SQL (임시 MySQL)', () => {
    const db = getDb()
    const { dashboardService } = composeDashboard(db, {
        tableSnapshotService: unusedTables,
        listCatalogGroups: async () => ({ groups: [] }),
        lookupTableText: () => null,
    })
    const me = userId('me')

    const cleanup = async () => {
        const ids = OWNERS.map(userId)
        await db.delete(replay).where(inArray(replay.userId, ids))
        await db.delete(rival).where(inArray(rival.userId, ids))
        await db.delete(courseBest).where(inArray(courseBest.userId, ids))
        await db.delete(courseScore).where(inArray(courseScore.userId, ids))
        await db.delete(course).where(like(course.courseHash, `${PREFIX}-%`))
        await db.delete(chartBest).where(inArray(chartBest.userId, ids))
        await db.delete(score).where(inArray(score.userId, ids))
        await db.delete(chart).where(like(chart.title, `${PREFIX}-%`))
        await db.delete(user).where(inArray(user.id, ids))
    }

    beforeAll(async () => {
        await cleanup()
        await db.insert(user).values(
            OWNERS.map((owner) => ({
                id: userId(owner),
                name: `Name ${owner}`,
                email: `${PREFIX}_${owner}@u5d.invalid`,
                loginId: userId(owner),
            })),
        )
        await db.insert(chart).values(
            (Object.keys(CHART_SEEDS) as ChartKey[]).map((key) => ({
                sha256: shaOf(key),
                md5: md5Of(key),
                title: `${PREFIX}-${key}`,
                mode: 'BEAT_7K',
                notes: NOTES,
                level: key === 'a' ? 12 : null,
            })),
        )
        await db.insert(score).values(PLAYS.map(scoreRowOf))
        await db.insert(chartBest).values(PLAYS.filter((play) => BEST_PLAY_IDS.includes(play.id)).map(bestRowOf))
        await db.insert(rival).values([
            { userId: me, rivalId: userId('r1') },
            { userId: me, rivalId: userId('r2') },
        ])
        await db.insert(course).values([
            { courseHash: COURSE_HASHES.first, name: `${PREFIX}-first` },
            { courseHash: COURSE_HASHES.second, name: `${PREFIX}-second` },
        ])
        const courseRun = (id: string, owner: Owner, courseHash: string, clear: ClearLamp, exScore: number, playedAt: number) => ({
            id: `${PREFIX}_${id}`,
            userId: userId(owner),
            courseHash,
            clear: clearLampToId(clear),
            exScore,
            playedAt,
        })
        await db
            .insert(courseScore)
            .values([
                courseRun('cs1', 'me', COURSE_HASHES.first, 'Hard', 3000, noonOf(3)),
                courseRun('cs2', 'me', COURSE_HASHES.second, 'Failed', 1000, noonOf(2)),
                courseRun('cs3', 'me', COURSE_HASHES.first, 'Normal', 2500, noonOf(1)),
                courseRun('cs4', 'o1', COURSE_HASHES.first, 'Max', 9000, noonOf(0)),
            ])
        await db.insert(courseBest).values([
            { courseHash: COURSE_HASHES.first, userId: me, courseScoreId: `${PREFIX}_cs1`, clear: clearLampToId('Hard'), exScore: 3000 },
            { courseHash: COURSE_HASHES.second, userId: me, courseScoreId: `${PREFIX}_cs2`, clear: clearLampToId('Failed'), exScore: 1000 },
        ])
        await db.insert(replay).values([
            { id: `${PREFIX}_rp1`, userId: me, chartSha256: shaOf('a'), scoreId: `${PREFIX}_me1`, createdAt: new Date(noonOf(2)) },
            { id: `${PREFIX}_rp2`, userId: me, chartSha256: shaOf('b'), scoreId: null, createdAt: new Date(noonOf(1)) },
            { id: `${PREFIX}_rp3`, userId: userId('o1'), chartSha256: shaOf('a'), createdAt: new Date(noonOf(0)) },
        ])
    })

    afterAll(async () => {
        await cleanup()
    })

    test('overview: 최고 기록 타일은 일반·SCRATCH AUTO 중 더 좋은 쪽 한 줄씩만 센다', async () => {
        const { tiles } = await dashboardService.overview({ userId: me, todayStart: TODAY_START })
        expect(tiles).toMatchObject({ played_charts: 5, cleared_charts: 4, hard_or_better: 2, full_combo_or_better: 1 })
    })

    test('overview: 판 수·마지막 시각·플레이 시간·연속·이번 주는 score 표의 판 시각에서 나온다', async () => {
        const { tiles } = await dashboardService.overview({ userId: me, todayStart: TODAY_START })
        expect(tiles).toMatchObject({
            total_plays: 7,
            last_played_at: noonOf(0),
            play_ms_30d: (2 + 4 + 2 + 5 + 2 + 3 + 2) * MINUTE_MS,
            active_days_30d: 4,
            streak_days: 4,
            best_streak_days: 4,
            this_week_plays: 7,
        })
    })

    test('overview: 등급 막대는 노트 수가 있는 최고 기록만 9묶음에 센다', async () => {
        const { grades } = await dashboardService.overview({ userId: me, todayStart: TODAY_START })
        expect(Object.fromEntries(grades.map(({ grade, count }) => [grade, count]))).toEqual({
            F: 0,
            E: 1,
            D: 0,
            C: 1,
            B: 0,
            A: 1,
            AA: 0,
            AAA: 1,
            MAX: 0,
        })
    })

    test('overview: 판이 없는 사용자는 모두 0 이고 날짜 기준이 오늘보다 앞이면 연속이 끊긴다', async () => {
        const empty = await dashboardService.overview({ userId: userId('none'), todayStart: TODAY_START })
        expect(empty.tiles).toMatchObject({ total_plays: 0, played_charts: 0, last_played_at: null, streak_days: 0, play_ms_30d: 0 })
        expect(empty.grades.every(({ count }) => count === 0)).toBe(true)
        const later = await dashboardService.overview({ userId: me, todayStart: TODAY_START + 10 * 86_400_000 })
        expect(later.tiles).toMatchObject({ streak_days: 0, this_week_plays: 0, total_plays: 7 })
    })

    test('overview: 응답이 DTO 를 통과한다', async () => {
        const overview = await dashboardService.overview({ userId: me, todayStart: TODAY_START })
        const rank = toDashboardRank(await dashboardService.leaderboardStandings(), me)
        expect(dashboardOverviewOutputSchema.safeParse({ ...overview, rank }).success).toBe(true)
    })

    test('종합 순위: 많이 클리어한 사람이 앞서고 동률은 공동 순위, 다음 순위는 건너뛰며 최고 기록이 없으면 순위가 없다', async () => {
        const standings = await dashboardService.leaderboardStandings()
        const positionOf = (owner: Owner) => toDashboardRank(standings, userId(owner)).position
        const [mine, first, second, last] = (['me', 'r1', 'r2', 'o1'] as const).map(positionOf)
        expect(mine).not.toBeNull()
        expect(first).toBe(second)
        expect(mine as number).toBeLessThan(first as number)
        expect(last as number).toBeGreaterThanOrEqual((first as number) + 2)
        expect(positionOf('none')).toBeNull()
        expect(standings.total_players).toBeGreaterThanOrEqual(4)
        expect(Object.keys(standings.positions)).toHaveLength(standings.total_players)
    })

    test('calendar: 요청한 창 길이만큼 날을 돌려주고 DTO 를 통과한다', async () => {
        const calendar = await dashboardService.calendar({ userId: me, todayStart: TODAY_START, days: 182 })
        expect(calendar.days).toHaveLength(182)
        expect(calendar).toMatchObject({ total_plays: 7, active_days: 4, streak_days: 4, best_streak_days: 4 })
        expect(calendar.days.at(-1)).toMatchObject({ date: '2026-11-05', plays: 1 })
        expect(dashboardCalendarOutputSchema.safeParse(calendar).success).toBe(true)
    })

    test('courses: 최고 기록과 최근 기록은 본인 것만, 최신순이며 코스 이름이 붙는다', async () => {
        const courses = await dashboardService.courses({ userId: me })
        expect(courses.recent).toEqual([
            { course_hash: COURSE_HASHES.first, name: `${PREFIX}-first`, clear: 'Normal', ex_score: 2500, played_at: noonOf(1) },
            { course_hash: COURSE_HASHES.second, name: `${PREFIX}-second`, clear: 'Failed', ex_score: 1000, played_at: noonOf(2) },
            { course_hash: COURSE_HASHES.first, name: `${PREFIX}-first`, clear: 'Hard', ex_score: 3000, played_at: noonOf(3) },
        ])
    })

    test('rivals: 라이벌 목록은 로그인 id 순이고 최근 판은 라이벌의 공개(ranked) 판만 최신순이다', async () => {
        const rivals = await dashboardService.rivals({ userId: me, limit: 3 })
        expect(rivals.rivals).toEqual([
            { login_id: userId('r1'), name: 'Name r1' },
            { login_id: userId('r2'), name: 'Name r2' },
        ])
        expect(rivals.recent.map((row) => row.score_id)).toEqual([`${PREFIX}_r2v`, `${PREFIX}_r2z`, `${PREFIX}_r2y`])
        expect(rivals.recent[0]).toMatchObject({ player: { id: userId('r2') }, clear: 'Failed', chart: { title: `${PREFIX}-v`, md5: md5Of('v') } })
        const all = await dashboardService.rivals({ userId: me, limit: 20 })
        expect(all.recent.map((row) => row.score_id)).not.toContain(`${PREFIX}_r1u`)
        expect(all.recent.map((row) => row.score_id)).not.toContain(`${PREFIX}_o1a`)
        expect(all.recent).toHaveLength(8)
    })

    test('rivals: 승패는 양쪽 모두 더 좋은 쪽 최고 기록의 EX 로 견주고 겹치지 않는 곡은 세지 않는다', async () => {
        const { versus } = await dashboardService.rivals({ userId: me, limit: 3 })
        expect(versus).toEqual([
            { login_id: userId('r1'), common: 3, wins: 2, losses: 0, draws: 1 },
            { login_id: userId('r2'), common: 1, wins: 1, losses: 0, draws: 0 },
        ])
    })

    test('rivals: 라이벌이 없는 사용자는 빈 응답이다', async () => {
        expect(await dashboardService.rivals({ userId: userId('o1'), limit: 10 })).toEqual({ rivals: [], recent: [], versus: [] })
    })

    test('replays: 본인 리플레이만 최신순이고 점수가 이어지지 않은 것은 램프와 EX 가 null 이다', async () => {
        const { replays } = await dashboardService.replays({ userId: me, limit: 10 })
        expect(replays).toEqual([
            {
                id: `${PREFIX}_rp2`,
                created_at: noonOf(1),
                chart: { md5: md5Of('b'), sha256: shaOf('b'), title: `${PREFIX}-b`, level: null },
                clear: null,
                ex_score: null,
            },
            {
                id: `${PREFIX}_rp1`,
                created_at: noonOf(2),
                chart: { md5: md5Of('a'), sha256: shaOf('a'), title: `${PREFIX}-a`, level: 12 },
                clear: 'Hard',
                ex_score: 1800,
            },
        ])
        expect((await dashboardService.replays({ userId: me, limit: 1 })).replays).toHaveLength(1)
    })

    test('replays: 리플레이 본문(data 열)은 읽지 않는다', async () => {
        await db
            .update(replay)
            .set({ data: 'x'.repeat(2048) })
            .where(eq(replay.id, `${PREFIX}_rp1`))
        const { replays } = await dashboardService.replays({ userId: me, limit: 10 })
        expect(JSON.stringify(replays)).not.toContain('xxxx')
    })
})

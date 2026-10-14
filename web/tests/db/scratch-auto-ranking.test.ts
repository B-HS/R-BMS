import { beforeAll, describe, expect, test } from 'bun:test'
import { inArray } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, courseBest, courseScore, score } from '@server/db/schema'
import { courseSubmissionSchema } from '@server/dto/course'
import { scoreSubmissionSchema } from '@server/dto/score'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] SCRATCH AUTO 순위 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const CHART_SHA = 'e'.repeat(64)
const CHART_MD5 = 'd'.repeat(32)
const COURSE_HASH = 'wave20-course'
const BASE_PLAYED_AT = 1_780_000_000_000
const PLAYERS = ['alice', 'bob', 'carol', 'dave'] as const

type PlayerLogin = (typeof PLAYERS)[number]

const userId = (login: PlayerLogin) => `u_${login}`

const judge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

const chartPlay = (login: PlayerLogin, play: { clear: string; exScore: number; minbp: number; scratchAuto: boolean; offset: number }) =>
    scoreSubmissionSchema.parse({
        chart: { md5: CHART_MD5, sha256: CHART_SHA },
        player: { id: login },
        mode: 'BEAT_7K',
        clear: play.clear,
        ex_score: play.exScore,
        minbp: play.minbp,
        judge,
        total_notes: 1000,
        options: {
            gauge: 'Normal',
            random: 'Off',
            scratch_auto: play.scratchAuto,
            lntype: 0,
            input_device: 'keyboard',
            assist: play.scratchAuto ? ['AUTO_SCRATCH'] : [],
            judge_rate: 100,
        },
        played_at: BASE_PLAYED_AT + play.offset,
    })

const coursePlay = (login: PlayerLogin, play: { clear: string; exScore: number; scratchAuto?: boolean; offset: number }) =>
    courseSubmissionSchema.parse({
        course_hash: COURSE_HASH,
        player: { id: login },
        clear: play.clear,
        ex_score: play.exScore,
        minbp: 5,
        judge,
        played_at: BASE_PLAYED_AT + play.offset,
        ...(play.scratchAuto === undefined ? {} : { scratch_auto: play.scratchAuto, assist: play.scratchAuto ? ['AUTO_SCRATCH'] : [] }),
    })

describe.skipIf(!hasDatabase)('SCRATCH AUTO 순위 (임시 MySQL)', () => {
    const submitChart = (login: PlayerLogin, play: Parameters<typeof chartPlay>[1]) =>
        compose().scoreService.submit({
            input: chartPlay(login, play),
            chartSha256: CHART_SHA,
            chartMd5: CHART_MD5,
            user: { id: userId(login), loginId: login, name: login },
            buildTrust: 'trusted',
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })

    const submitCourse = (login: PlayerLogin, play: Parameters<typeof coursePlay>[1]) =>
        compose().courseService.submit({ input: coursePlay(login, play), user: { id: userId(login), loginId: login } })

    const chartRanking = async (includeScratchAuto: boolean) =>
        (await compose().scoreService.getRanking({ chartSha256: CHART_SHA, limit: 50, page: 1, includeScratchAuto })).map((row) => ({
            player: row.player.id,
            ex: row.ex_score,
            rank: row.rank,
            scratchAuto: row.extra.scratch_auto === true,
        }))

    const courseRanking = async (includeScratchAuto: boolean) =>
        (await compose().courseService.getRanking({ courseHash: COURSE_HASH, limit: 50, page: 1, includeScratchAuto })).map((row) => ({
            player: row.player.id,
            ex: row.ex_score,
            scratchAuto: row.extra.scratch_auto === true,
        }))

    const submitted: Record<string, Awaited<ReturnType<typeof submitChart>>> = {}

    beforeAll(async () => {
        const db = getDb()
        const ids = PLAYERS.map(userId)
        await db.delete(score).where(inArray(score.userId, ids))
        await db.delete(chartBest).where(inArray(chartBest.userId, ids))
        await db.delete(courseScore).where(inArray(courseScore.userId, ids))
        await db.delete(courseBest).where(inArray(courseBest.userId, ids))
        await db.delete(user).where(inArray(user.id, ids))
        await db.insert(user).values(PLAYERS.map((login) => ({ id: userId(login), name: login, email: `${login}@wave20.invalid`, loginId: login })))
        await db
            .insert(chart)
            .values({ sha256: CHART_SHA, md5: CHART_MD5, title: 'Wave 20', mode: 'BEAT_7K', notes: 1000 })
            .onDuplicateKeyUpdate({ set: { title: 'Wave 20' } })

        submitted.aliceNormal = await submitChart('alice', { clear: 'Hard', exScore: 1000, minbp: 20, scratchAuto: false, offset: 1 })
        submitted.aliceScratch = await submitChart('alice', { clear: 'ExHard', exScore: 1500, minbp: 5, scratchAuto: true, offset: 2 })
        submitted.bob = await submitChart('bob', { clear: 'Hard', exScore: 1200, minbp: 10, scratchAuto: false, offset: 3 })
        submitted.carolScratch = await submitChart('carol', { clear: 'Normal', exScore: 900, minbp: 30, scratchAuto: true, offset: 4 })
        submitted.daveNormal = await submitChart('dave', { clear: 'Easy', exScore: 800, minbp: 40, scratchAuto: false, offset: 5 })
        submitted.daveScratch = await submitChart('dave', { clear: 'Easy', exScore: 800, minbp: 40, scratchAuto: true, offset: 6 })
        submitted.aliceScratchWorse = await submitChart('alice', { clear: 'Hard', exScore: 1100, minbp: 9, scratchAuto: true, offset: 7 })
    })

    test('SCRATCH AUTO 판은 ranked 로 저장되고 SCRATCH AUTO 베스트에만 들어간다', async () => {
        expect(submitted.aliceScratch.ranked).toBe(true)
        expect(submitted.aliceScratch.response.flags).toEqual(['SCRATCH_AUTO'])
        const bests = await getDb()
            .select()
            .from(chartBest)
            .where(inArray(chartBest.userId, [userId('alice')]))
        expect(bests.map((row) => ({ scratchAuto: row.scratchAuto, ex: row.exScore })).toSorted((a, b) => a.ex - b.ex)).toEqual([
            { scratchAuto: false, ex: 1000 },
            { scratchAuto: true, ex: 1500 },
        ])
    })

    test('기본 목록은 일반 판 최고만, 포함 목록은 플레이어마다 더 높은 쪽을 보여 준다', async () => {
        expect(await chartRanking(false)).toEqual([
            { player: 'bob', ex: 1200, rank: 1, scratchAuto: false },
            { player: 'alice', ex: 1000, rank: 2, scratchAuto: false },
            { player: 'dave', ex: 800, rank: 3, scratchAuto: false },
        ])
        expect(await chartRanking(true)).toEqual([
            { player: 'alice', ex: 1500, rank: 1, scratchAuto: true },
            { player: 'bob', ex: 1200, rank: 2, scratchAuto: false },
            { player: 'carol', ex: 900, rank: 3, scratchAuto: true },
            { player: 'dave', ex: 800, rank: 4, scratchAuto: false },
        ])
    })

    test('제출 응답의 rank 는 그 판이 속한 목록의 순위다', () => {
        expect(submitted.aliceNormal.response.rank).toBe(1)
        expect(submitted.aliceScratch.response.rank).toBe(1)
        expect(submitted.bob.response.rank).toBe(1)
        expect(submitted.carolScratch.response.rank).toBe(3)
        expect(submitted.daveScratch.response.rank).toBe(4)
        expect(submitted.aliceScratchWorse.response.rank).toBe(3)
        expect(submitted.aliceScratchWorse.response.is_new_best).toBe(false)
        expect(submitted.aliceScratchWorse.response.previous_best).toBe(1500)
    })

    test('내 최고 기록 조회도 목록과 같은 범위를 따른다', async () => {
        const normal = await compose().scoreService.getBest({ chartSha256: CHART_SHA, loginId: 'alice', includeScratchAuto: false })
        const included = await compose().scoreService.getBest({ chartSha256: CHART_SHA, loginId: 'alice', includeScratchAuto: true })
        expect([normal?.ex_score, normal?.rank, normal?.extra.scratch_auto]).toEqual([1000, 2, undefined])
        expect([included?.ex_score, included?.rank, included?.extra.scratch_auto]).toEqual([1500, 1, true])
        expect(await compose().scoreService.getBest({ chartSha256: CHART_SHA, loginId: 'carol', includeScratchAuto: false })).toBeNull()
    })

    test('리더보드의 클리어·플레이 곡·제출 수는 기본으로 SCRATCH AUTO 를 빼고 토글로 넣는다', async () => {
        const counts = async (includeScratchAuto: boolean) => {
            const { items } = await compose().feService.playerLeaderboard({
                sort: 'cleared',
                page: 1,
                limit: 100,
                include_scratch_auto: includeScratchAuto,
            })
            return Object.fromEntries(
                items
                    .filter((item) => (PLAYERS as readonly string[]).includes(item.id))
                    .map((item) => [item.id, [item.cleared_charts, item.played_charts, item.total_plays]]),
            )
        }
        expect(await counts(false)).toEqual({ alice: [1, 1, 1], bob: [1, 1, 1], carol: [0, 0, 0], dave: [1, 1, 1] })
        expect(await counts(true)).toEqual({ alice: [1, 1, 3], bob: [1, 1, 1], carol: [1, 1, 1], dave: [1, 1, 2] })
    })

    test('플레이어 통계와 표의 내 기록은 더 높은 쪽을 SCRATCH AUTO 표시와 함께 쓴다', async () => {
        const stats = await compose().feService.playerStats({ userId: userId('alice'), totalPlays: 3 })
        expect(stats.lamps).toEqual([{ clear: 'ExHard', count: 1 }])
        expect(stats.scratch_auto_charts).toBe(1)
        const daveStats = await compose().feService.playerStats({ userId: userId('dave'), totalPlays: 2 })
        expect([daveStats.lamps, daveStats.scratch_auto_charts]).toEqual([[{ clear: 'Easy', count: 1 }], 0])
    })

    test('홈 최근 활동은 SCRATCH AUTO 판을 빼고 플레이어 최근 기록은 표시와 함께 넣는다', async () => {
        const recent = await compose().feService.recentActivity(100)
        expect(recent.filter((row) => row.chart.sha256 === CHART_SHA).map((row) => [row.player.id, row.scratch_auto])).toEqual([
            ['dave', false],
            ['bob', false],
            ['alice', false],
        ])
        const aliceRecent = await compose().feService.playerRecent({ userId: userId('alice'), limit: 10 })
        expect(aliceRecent.map((row) => row.scratch_auto)).toEqual([true, true, false])
    })

    test('코스도 같은 규칙이고 scratch_auto 가 없는 옛 클라이언트 코스는 일반 판이다', async () => {
        const legacy = await submitCourse('bob', { clear: 'Hard', exScore: 4000, offset: 11 })
        const normal = await submitCourse('alice', { clear: 'Normal', exScore: 3000, scratchAuto: false, offset: 12 })
        const scratch = await submitCourse('alice', { clear: 'Hard', exScore: 4500, scratchAuto: true, offset: 13 })
        const judgeWidened = await compose().courseService.submit({
            input: courseSubmissionSchema.parse({
                ...coursePlay('carol', { clear: 'Max', exScore: 9999, offset: 14 }),
                scratch_auto: true,
                assist: ['AUTO_SCRATCH', 'CUSTOM_JUDGE'],
            }),
            user: { id: userId('carol'), loginId: 'carol' },
        })
        expect([legacy.response.ranked, legacy.response.flags, legacy.response.rank]).toEqual([true, [], 1])
        expect([normal.response.rank, scratch.response.rank, scratch.response.flags]).toEqual([2, 1, ['SCRATCH_AUTO']])
        expect([judgeWidened.response.ranked, judgeWidened.response.rank]).toEqual([false, null])
        expect(await courseRanking(false)).toEqual([
            { player: 'bob', ex: 4000, scratchAuto: false },
            { player: 'alice', ex: 3000, scratchAuto: false },
        ])
        expect(await courseRanking(true)).toEqual([
            { player: 'alice', ex: 4500, scratchAuto: true },
            { player: 'bob', ex: 4000, scratchAuto: false },
        ])
    })
})

import { beforeAll, describe, expect, test } from 'bun:test'
import { and, eq, inArray, like } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, courseBest, courseScore, score } from '@server/db/schema'
import { courseSubmissionSchema } from '@server/dto/course'
import { scoreSubmissionSchema } from '@server/dto/score'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 최고 기록 경합·순위 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 's19'
const BASE_PLAYED_AT = 1_790_000_000_000
const PLAY_SPACING_MS = 1_000
const RACE_ROUNDS = 20
const NOTES = 1000
const PLAYERS = ['p1', 'p2', 'p3', 'p4', 'p5', 'racer'] as const

type Player = (typeof PLAYERS)[number]

const userId = (player: Player) => `${PREFIX}_${player}`
const hexOf = (seed: number, length: number) => seed.toString(16).padStart(length, '0')
const chartSha = (seed: number) => hexOf(seed, 64)
const chartMd5 = (seed: number) => hexOf(seed, 32)

const judge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

const chartPlay = (
    player: Player,
    play: { chartSeed: number; clear: string; exScore: number; minbp: number; offset: number; lntype?: number; scratchAuto?: boolean },
) =>
    scoreSubmissionSchema.parse({
        chart: { md5: chartMd5(play.chartSeed), sha256: chartSha(play.chartSeed) },
        player: { id: `${PREFIX}_${player}` },
        mode: 'BEAT_7K',
        clear: play.clear,
        ex_score: play.exScore,
        minbp: play.minbp,
        judge,
        total_notes: NOTES,
        options: {
            gauge: 'Normal',
            random: 'Off',
            scratch_auto: play.scratchAuto ?? false,
            lntype: play.lntype ?? 0,
            input_device: 'keyboard',
            assist: play.scratchAuto ? ['AUTO_SCRATCH'] : [],
            judge_rate: 100,
        },
        played_at: BASE_PLAYED_AT + play.offset * PLAY_SPACING_MS,
    })

const coursePlay = (player: Player, play: { courseHash: string; clear: string; exScore: number; minbp: number; offset: number }) =>
    courseSubmissionSchema.parse({
        course_hash: play.courseHash,
        player: { id: `${PREFIX}_${player}` },
        clear: play.clear,
        ex_score: play.exScore,
        minbp: play.minbp,
        judge,
        played_at: BASE_PLAYED_AT + play.offset * PLAY_SPACING_MS,
    })

const submitChart = (player: Player, play: Parameters<typeof chartPlay>[1]) =>
    compose().scoreService.submit({
        input: chartPlay(player, play),
        chartSha256: chartSha(play.chartSeed),
        chartMd5: chartMd5(play.chartSeed),
        user: { id: userId(player), loginId: `${PREFIX}_${player}`, name: player },
        buildTrust: 'trusted',
        requireTrustedBuild: false,
        ip: null,
        userAgent: null,
    })

const submitCourse = (player: Player, play: Parameters<typeof coursePlay>[1]) =>
    compose().courseService.submit({
        input: coursePlay(player, play),
        user: { id: userId(player), loginId: `${PREFIX}_${player}` },
        buildTrust: 'trusted',
        requireTrustedBuild: false,
    })

describe.skipIf(!hasDatabase)('최고 기록 경합과 순위 번호 (임시 MySQL)', () => {
    beforeAll(async () => {
        const db = getDb()
        const ids = PLAYERS.map(userId)
        await db.delete(score).where(inArray(score.userId, ids))
        await db.delete(chartBest).where(inArray(chartBest.userId, ids))
        await db.delete(courseScore).where(inArray(courseScore.userId, ids))
        await db.delete(courseBest).where(inArray(courseBest.userId, ids))
        await db.delete(user).where(inArray(user.id, ids))
        await db.delete(chart).where(like(chart.title, `${PREFIX}-%`))
        await db.insert(user).values(
            PLAYERS.map((player) => ({
                id: userId(player),
                name: player,
                email: `${PREFIX}_${player}@s19.invalid`,
                loginId: `${PREFIX}_${player}`,
            })),
        )
        const chartSeeds = [...Array.from({ length: RACE_ROUNDS }, (_, index) => 1000 + index), 1, 2, 3]
        await db.insert(chart).values(
            chartSeeds.map((seed) => ({
                sha256: chartSha(seed),
                md5: chartMd5(seed),
                title: `${PREFIX}-${seed}`,
                mode: 'BEAT_7K',
                notes: NOTES,
            })),
        )
    })

    const storedBest = async (player: Player, seed: number) => {
        const [row] = await getDb()
            .select()
            .from(chartBest)
            .where(and(eq(chartBest.chartSha256, chartSha(seed)), eq(chartBest.userId, userId(player)), eq(chartBest.scratchAuto, false)))
        return row
    }

    test('이미 있는 더 좋은 최고 기록은 나중에 도착한 낮은 판이 덮지 못한다', async () => {
        const { scoreService } = compose()
        const seed = 500
        await getDb()
            .insert(chart)
            .values({ sha256: chartSha(seed), md5: chartMd5(seed), title: `${PREFIX}-${seed}`, mode: 'BEAT_7K', notes: NOTES })
        const high = await submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1400, minbp: 5, offset: 1 })
        const low = await submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 5, offset: 2 })
        expect(high.response.is_new_best).toBe(true)
        expect(low.response.is_new_best).toBe(false)
        const row = await storedBest('racer', seed)
        expect(row.exScore).toBe(1400)
        expect(row.scoreId).toBe(high.scoreId)
        expect(await scoreService.getBest({ chartSha256: chartSha(seed), loginId: `${PREFIX}_racer`, includeScratchAuto: false })).toMatchObject({
            ex_score: 1400,
        })
    })

    test('DB 문장 하나가 조건부 갱신이라 램프, EX, BP 순서로 더 좋을 때만 바뀐다', async () => {
        const { scoreService } = compose()
        const seed = 501
        await getDb()
            .insert(chart)
            .values({ sha256: chartSha(seed), md5: chartMd5(seed), title: `${PREFIX}-${seed}`, mode: 'BEAT_7K', notes: NOTES })
        const first = await submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 10, offset: 1 })
        expect(first.response.is_new_best).toBe(true)
        const worseBp = await submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 11, offset: 2 })
        const sameScore = await submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 10, offset: 3 })
        const lowerLamp = await submitChart('racer', { chartSeed: seed, clear: 'Normal', exScore: 1800, minbp: 0, offset: 4 })
        expect([worseBp, sameScore, lowerLamp].map((result) => result.response.is_new_best)).toEqual([false, false, false])
        expect((await storedBest('racer', seed)).scoreId).toBe(first.scoreId)
        const betterBp = await submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 9, offset: 5 })
        expect(betterBp.response.is_new_best).toBe(true)
        const higherLamp = await submitChart('racer', { chartSeed: seed, clear: 'ExHard', exScore: 500, minbp: 99, offset: 6 })
        expect(higherLamp.response.is_new_best).toBe(true)
        expect(await storedBest('racer', seed)).toMatchObject({ scoreId: higherLamp.scoreId, clear: 7, exScore: 500, minbp: 99 })
        expect(await scoreService.getById(higherLamp.scoreId)).toMatchObject({ ex_score: 500 })
    })

    test('같은 차트에 높은 판과 낮은 판이 동시에 들어와도 매번 높은 판이 남는다', async () => {
        const outcomes = await Promise.all(
            Array.from({ length: RACE_ROUNDS }, async (_, index) => {
                const seed = 1000 + index
                const [highFirst, lowFirst] = index % 2 === 0 ? [true, false] : [false, true]
                const high = () => submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1400, minbp: 5, offset: 10 + index })
                const low = () => submitChart('racer', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 5, offset: 100 + index })
                const [first, second] = await Promise.all(highFirst && !lowFirst ? [high(), low()] : [low(), high()])
                const row = await storedBest('racer', seed)
                return { exScore: row.exScore, newBests: [first, second].filter((result) => result.response.is_new_best).length }
            }),
        )
        expect(outcomes.map((outcome) => outcome.exScore)).toEqual(Array.from({ length: RACE_ROUNDS }, () => 1400))
        expect(outcomes.every((outcome) => outcome.newBests >= 1)).toBe(true)
    })

    test('코스도 낮은 판이 나중에 도착해도 최고 기록을 덮지 못한다', async () => {
        const courseHash = `${PREFIX}-course-race`
        const high = await submitCourse('racer', { courseHash, clear: 'Hard', exScore: 4000, minbp: 5, offset: 1 })
        const low = await submitCourse('racer', { courseHash, clear: 'Hard', exScore: 3000, minbp: 5, offset: 2 })
        expect([high.response.is_new_best, low.response.is_new_best]).toEqual([true, false])
        const [row] = await getDb()
            .select()
            .from(courseBest)
            .where(and(eq(courseBest.courseHash, courseHash), eq(courseBest.userId, userId('racer'))))
        expect(row).toMatchObject({ exScore: 4000, courseScoreId: high.courseScoreId })
    })

    test('코스의 높은 판과 낮은 판이 동시에 들어와도 높은 판이 남는다', async () => {
        const results = await Promise.all(
            Array.from({ length: RACE_ROUNDS }, async (_, index) => {
                const courseHash = `${PREFIX}-course-concurrent-${index}`
                await Promise.all([
                    submitCourse('racer', { courseHash, clear: 'Hard', exScore: 4000, minbp: 5, offset: 10 + index }),
                    submitCourse('racer', { courseHash, clear: 'Hard', exScore: 3000, minbp: 5, offset: 200 + index }),
                ])
                const [row] = await getDb()
                    .select()
                    .from(courseBest)
                    .where(and(eq(courseBest.courseHash, courseHash), eq(courseBest.userId, userId('racer'))))
                return row.exScore
            }),
        )
        expect(results).toEqual(Array.from({ length: RACE_ROUNDS }, () => 4000))
    })

    describe('순위 번호', () => {
        const seed = 1
        const rankingOf = async (page = 1, limit = 50) =>
            (await compose().scoreService.getRanking({ chartSha256: chartSha(seed), limit, page, includeScratchAuto: false })).map((row) => ({
                player: row.player.id.replace(`${PREFIX}_`, ''),
                rank: row.rank,
            }))
        const bestRankOf = async (player: Player) =>
            (await compose().scoreService.getBest({ chartSha256: chartSha(seed), loginId: `${PREFIX}_${player}`, includeScratchAuto: false }))?.rank

        beforeAll(async () => {
            await submitChart('p1', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 5, offset: 1 })
            await submitChart('p2', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 9, offset: 2 })
            await submitChart('p3', { chartSeed: seed, clear: 'Hard', exScore: 1000, minbp: 9, offset: 3 })
            await submitChart('p4', { chartSeed: seed, clear: 'Easy', exScore: 2000, minbp: 1, offset: 4 })
            await submitChart('p5', { chartSeed: seed, clear: 'Hard', exScore: 900, minbp: 0, offset: 5 })
        })

        test('같은 램프와 EX 에서 BP 만 다른 두 사람은 목록과 제출 응답이 같은 순위를 말한다', async () => {
            const rows = await rankingOf()
            expect(rows.find((row) => row.player === 'p1')?.rank).toBe(1)
            expect(rows.find((row) => row.player === 'p2')?.rank).toBe(2)
            expect(await bestRankOf('p1')).toBe(1)
            expect(await bestRankOf('p2')).toBe(2)
        })

        test('램프, EX, BP 가 모두 같은 사람들은 공동 순위이고 다음 순위는 건너뛴다', async () => {
            const rows = await rankingOf()
            expect(rows.map((row) => row.rank)).toEqual([1, 2, 2, 4, 5])
            expect(rows.map((row) => row.player).filter((player) => player === 'p2' || player === 'p3')).toHaveLength(2)
            expect([await bestRankOf('p2'), await bestRankOf('p3'), await bestRankOf('p5'), await bestRankOf('p4')]).toEqual([2, 2, 4, 5])
        })

        test('쪽을 나눠도 동점자가 겹치거나 빠지지 않고 순위는 전체 기준이다', async () => {
            const pages = await Promise.all([1, 2, 3].map((page) => rankingOf(page, 2)))
            const players = pages.flat().map((row) => row.player)
            expect(players).toHaveLength(5)
            expect(new Set(players).size).toBe(5)
            expect(pages.flat().map((row) => row.rank)).toEqual([1, 2, 2, 4, 5])
            expect(pages.map((page) => page.length)).toEqual([2, 2, 1])
        })

        test('같은 목록을 여러 번 읽어도 동점자의 순서가 바뀌지 않는다', async () => {
            const reads = await Promise.all(Array.from({ length: 10 }, () => rankingOf()))
            expect(new Set(reads.map((read) => read.map((row) => row.player).join(','))).size).toBe(1)
        })

        test('제출 응답의 순위도 같은 정의다', async () => {
            const second = await submitChart('racer', { chartSeed: 2, clear: 'Hard', exScore: 1000, minbp: 9, offset: 1 })
            const third = await submitChart('p1', { chartSeed: 2, clear: 'Hard', exScore: 1000, minbp: 9, offset: 2 })
            const fourth = await submitChart('p2', { chartSeed: 2, clear: 'Hard', exScore: 1000, minbp: 5, offset: 3 })
            expect([second.response.rank, third.response.rank, fourth.response.rank]).toEqual([1, 1, 1])
            const fifth = await submitChart('p3', { chartSeed: 2, clear: 'Hard', exScore: 1000, minbp: 10, offset: 4 })
            expect(fifth.response.rank).toBe(4)
        })

        test('LN 방식 필터는 최고 기록이 그 방식인 사람만 남기고 그 안에서 순위를 센다', async () => {
            const lnSeed = 3
            await submitChart('p1', { chartSeed: lnSeed, clear: 'Hard', exScore: 1000, minbp: 5, offset: 1, lntype: 1 })
            await submitChart('p2', { chartSeed: lnSeed, clear: 'Hard', exScore: 900, minbp: 5, offset: 2, lntype: 0 })
            const ranking = async (lnmode: number) =>
                (
                    await compose().scoreService.getRanking({ chartSha256: chartSha(lnSeed), limit: 50, page: 1, lnmode, includeScratchAuto: false })
                ).map((row) => [row.player.id.replace(`${PREFIX}_`, ''), row.rank])
            expect(await ranking(1)).toEqual([['p1', 1]])
            expect(await ranking(0)).toEqual([['p2', 1]])
            const best = (lnmode?: number) =>
                compose().scoreService.getBest({ chartSha256: chartSha(lnSeed), loginId: `${PREFIX}_p2`, lnmode, includeScratchAuto: false })
            expect((await best(0))?.rank).toBe(1)
            expect((await best())?.rank).toBe(2)
        })

        test('코스 목록도 같은 순위 정의와 쪽 안정성을 쓴다', async () => {
            const courseHash = `${PREFIX}-course-ranks`
            await submitCourse('p1', { courseHash, clear: 'Hard', exScore: 4000, minbp: 5, offset: 1 })
            await submitCourse('p2', { courseHash, clear: 'Hard', exScore: 4000, minbp: 9, offset: 2 })
            await submitCourse('p3', { courseHash, clear: 'Hard', exScore: 4000, minbp: 9, offset: 3 })
            await submitCourse('p4', { courseHash, clear: 'Normal', exScore: 5000, minbp: 0, offset: 4 })
            const pages = await Promise.all(
                [1, 2].map((page) => compose().courseService.getRanking({ courseHash, limit: 2, page, includeScratchAuto: false })),
            )
            expect(pages.flat().map((row) => row.rank)).toEqual([1, 2, 2, 4])
            expect(new Set(pages.flat().map((row) => row.player.id)).size).toBe(4)
            const best = await compose().courseService.getBest({ courseHash, loginId: `${PREFIX}_p3`, includeScratchAuto: false })
            expect(best?.rank).toBe(2)
        })
    })
})

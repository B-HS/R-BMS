import { beforeAll, describe, expect, test } from 'bun:test'
import { inArray } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, score } from '@server/db/schema'
import { scoreSubmissionSchema } from '@server/dto/score'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 클리어 기준·플레이 기록 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'clr'
const BASE_PLAYED_AT = 1_795_000_000_000
const PLAY_SPACING_MS = 1_000
const NOTES = 1000
const TIED_OFFSET = 21
const PLAYERS = ['lamps', 'history', 'ties', 'empty'] as const

type Player = (typeof PLAYERS)[number]

const userId = (player: Player) => `${PREFIX}_${player}`
const hexOf = (seed: number, length: number) => seed.toString(16).padStart(length, '0')
const chartSha = (seed: number) => hexOf(seed, 64)
const chartMd5 = (seed: number) => hexOf(seed, 32)
const CHART_SEEDS = [1, 2, 3, 4, 5].map((index) => 0xc1ea0000 + index)

const judge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

type Play = { chartSeed: number; clear: string; offset: number; scratchAuto?: boolean }

const chartPlay = (player: Player, play: Play) =>
    scoreSubmissionSchema.parse({
        chart: { md5: chartMd5(play.chartSeed), sha256: chartSha(play.chartSeed) },
        player: { id: userId(player) },
        mode: 'BEAT_7K',
        clear: play.clear,
        ex_score: 100 + play.offset,
        minbp: 10,
        judge,
        total_notes: NOTES,
        options: {
            gauge: 'Normal',
            random: 'Off',
            scratch_auto: play.scratchAuto ?? false,
            lntype: 0,
            input_device: 'keyboard',
            assist: play.scratchAuto ? ['AUTO_SCRATCH'] : [],
            judge_rate: 100,
        },
        played_at: BASE_PLAYED_AT + play.offset * PLAY_SPACING_MS,
    })

describe.skipIf(!hasDatabase)('클리어 기준과 플레이 기록 (임시 MySQL)', () => {
    const submit = (player: Player, play: Play) =>
        compose().scoreService.submit({
            input: chartPlay(player, play),
            chartSha256: chartSha(play.chartSeed),
            chartMd5: chartMd5(play.chartSeed),
            user: { id: userId(player), loginId: userId(player), name: player },
            buildTrust: 'trusted',
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })

    const leaderboardCounts = async (includeScratchAuto: boolean) => {
        const { items } = await compose().feService.playerLeaderboard({
            sort: 'cleared',
            page: 1,
            limit: 100,
            include_scratch_auto: includeScratchAuto,
        })
        const row = items.find((item) => item.id === userId('lamps'))
        return [row?.cleared_charts, row?.played_charts]
    }

    beforeAll(async () => {
        const db = getDb()
        const ids = PLAYERS.map(userId)
        await db.delete(score).where(inArray(score.userId, ids))
        await db.delete(chartBest).where(inArray(chartBest.userId, ids))
        await db.delete(user).where(inArray(user.id, ids))
        await db
            .insert(user)
            .values(PLAYERS.map((player) => ({ id: userId(player), name: player, email: `${player}@clear.invalid`, loginId: userId(player) })))
        await db
            .insert(chart)
            .values(
                CHART_SEEDS.map((seed) => ({ sha256: chartSha(seed), md5: chartMd5(seed), title: `Clear ${seed}`, mode: 'BEAT_7K', notes: NOTES })),
            )
            .onDuplicateKeyUpdate({ set: { mode: 'BEAT_7K' } })

        await submit('lamps', { chartSeed: CHART_SEEDS[0], clear: 'AssistEasy', offset: 1 })
        await submit('lamps', { chartSeed: CHART_SEEDS[1], clear: 'LightAssistEasy', offset: 2, scratchAuto: true })
        await submit('lamps', { chartSeed: CHART_SEEDS[2], clear: 'Failed', offset: 3 })
        await submit('lamps', { chartSeed: CHART_SEEDS[3], clear: 'Easy', offset: 4 })

        for (const offset of [11, 12, 13, 14, 15]) {
            await submit('history', { chartSeed: CHART_SEEDS[offset % CHART_SEEDS.length], clear: 'Normal', offset })
        }
    })

    test('리더보드의 클리어는 ASSIST 와 L-ASSIST 를 세고 FAILED 만 뺀다', async () => {
        expect(await leaderboardCounts(true)).toEqual([3, 4])
        expect(await leaderboardCounts(false)).toEqual([2, 3])
    })

    test('플레이어 통계의 클리어도 같은 기준이다', async () => {
        const stats = await compose().feService.playerStats({ userId: userId('lamps'), totalPlays: 4 })
        expect(stats.cleared).toBe(3)
        expect(stats.lamps.map((entry) => entry.clear).toSorted()).toEqual(['AssistEasy', 'Easy', 'Failed', 'LightAssistEasy'])
        expect(stats.scratch_auto_charts).toBe(1)
    })

    test('플레이 기록은 최신순으로 페이지를 나눠 모든 판을 준다', async () => {
        const read = (page: number, limit: number) => compose().feService.playerRecent({ userId: userId('history'), page, limit })
        const first = await read(1, 2)
        const second = await read(2, 2)
        const third = await read(3, 2)
        const beyond = await read(4, 2)
        expect([first.total, second.total, third.total, beyond.total]).toEqual([5, 5, 5, 5])
        const pages = [first, second, third, beyond].map((page) => page.items.map((item) => item.played_at - BASE_PLAYED_AT))
        expect(pages).toEqual([[15_000, 14_000], [13_000, 12_000], [11_000], []])
        const everything = await read(1, 100)
        expect(everything.items.map((item) => item.score_id)).toEqual([...first.items, ...second.items, ...third.items].map((item) => item.score_id))
    })

    test('판이 없는 플레이어는 빈 목록과 0판이다', async () => {
        expect(await compose().feService.playerRecent({ userId: userId('empty'), page: 1, limit: 20 })).toEqual({ items: [], total: 0 })
    })

    test('같은 시각의 판은 점수 id 내림차순으로 페이지 경계를 넘어도 한 번씩만 나온다', async () => {
        for (const chartSeed of CHART_SEEDS.slice(0, 4)) await submit('ties', { chartSeed, clear: 'Normal', offset: TIED_OFFSET })
        const read = (page: number, limit: number) => compose().feService.playerRecent({ userId: userId('ties'), page, limit })
        const pages = [await read(1, 3), await read(2, 3)]
        const paged = pages.flatMap((page) => page.items.map((item) => item.score_id))
        expect(pages.map((page) => page.total)).toEqual([4, 4])
        expect(new Set(paged).size).toBe(4)
        expect(paged).toEqual(paged.toSorted().toReversed())
    })
})

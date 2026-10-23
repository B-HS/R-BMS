import { beforeAll, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { and, eq, inArray, sql } from 'drizzle-orm'
import { compose } from '@server/compose'
import { composeTableSnapshot } from '@server/compose/table-snapshot'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, courseBest, courseScore, score } from '@server/db/schema'
import { courseSubmissionSchema } from '@server/dto/course'
import { scoreSubmissionSchema } from '@server/dto/score'
import { getSnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 어시스트 등급 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'agr'
const BASE_PLAYED_AT = 1_796_000_000_000
const PLAY_SPACING_MS = 1_000
const NOTES = 1000
const COURSE_HASH = 'assist-grade-course'
const GAUGE_COURSE_HASH = 'assist-grade-course-gauge'
const TABLE_ID = 'insane1'
const PLAYERS = ['alice', 'bob', 'carol', 'dave', 'erin', 'frank'] as const
const BACKFILL_SEED = 0xa5510000
const STALE_ASSIST_GRADE = 3

type Player = (typeof PLAYERS)[number]

const userId = (player: Player) => `${PREFIX}_${player}`
const hexOf = (seed: number, length: number) => seed.toString(16).padStart(length, '0')
const CHART_SEED = 0xa5500001
const chartSha = (seed: number) => hexOf(seed, 64)
const chartMd5 = (seed: number) => hexOf(seed, 32)

const judge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

type Play = { clear: string; gauge: string; exScore: number; offset: number; scratchAuto: boolean; chartSeed?: number; md5?: string }

const chartPlay = (player: Player, play: Play) =>
    scoreSubmissionSchema.parse({
        chart: { md5: play.md5 ?? chartMd5(play.chartSeed ?? CHART_SEED), sha256: chartSha(play.chartSeed ?? CHART_SEED) },
        player: { id: userId(player) },
        mode: 'BEAT_7K',
        clear: play.clear,
        ex_score: play.exScore,
        minbp: 10,
        judge,
        total_notes: NOTES,
        options: {
            gauge: play.gauge,
            random: 'Off',
            scratch_auto: play.scratchAuto,
            lntype: 0,
            input_device: 'keyboard',
            assist: play.scratchAuto ? ['AUTO_SCRATCH'] : [],
            judge_rate: 100,
        },
        played_at: BASE_PLAYED_AT + play.offset * PLAY_SPACING_MS,
    })

const scratchAutoPlay = (gauge: string, exScore: number, offset: number): Play => ({
    clear: 'LightAssistEasy',
    gauge,
    exScore,
    offset,
    scratchAuto: true,
})

describe.skipIf(!hasDatabase)('어시스트 등급 (임시 MySQL)', () => {
    const submit = (player: Player, play: Play) =>
        compose().scoreService.submit({
            input: chartPlay(player, play),
            chartSha256: chartSha(play.chartSeed ?? CHART_SEED),
            chartMd5: play.md5 ?? chartMd5(play.chartSeed ?? CHART_SEED),
            user: { id: userId(player), loginId: userId(player), name: player },
            buildTrust: 'trusted',
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })

    const ranking = async (includeScratchAuto: boolean) =>
        (await compose().scoreService.getRanking({ chartSha256: chartSha(CHART_SEED), limit: 50, page: 1, includeScratchAuto })).map((row) => ({
            player: row.player.id.slice(PREFIX.length + 1),
            clear: row.clear,
            assistGrade: row.assist_grade,
            ex: row.ex_score,
            rank: row.rank,
        }))

    const storedBest = async (player: Player, chartSeed = CHART_SEED, isScratchAuto = true) => {
        const [row] = await getDb()
            .select()
            .from(chartBest)
            .where(
                and(eq(chartBest.userId, userId(player)), eq(chartBest.chartSha256, chartSha(chartSeed)), eq(chartBest.scratchAuto, isScratchAuto)),
            )
        return row
    }

    beforeAll(async () => {
        const db = getDb()
        const ids = PLAYERS.map(userId)
        await db.delete(score).where(inArray(score.userId, ids))
        await db.delete(chartBest).where(inArray(chartBest.userId, ids))
        await db.delete(courseScore).where(inArray(courseScore.userId, ids))
        await db.delete(courseBest).where(inArray(courseBest.userId, ids))
        await db.delete(user).where(inArray(user.id, ids))
        await db
            .insert(user)
            .values(PLAYERS.map((player) => ({ id: userId(player), name: player, email: `${player}@grade.invalid`, loginId: userId(player) })))
        await db
            .insert(chart)
            .values({ sha256: chartSha(CHART_SEED), md5: chartMd5(CHART_SEED), title: 'Assist Grade', mode: 'BEAT_7K', notes: NOTES })
            .onDuplicateKeyUpdate({ set: { title: 'Assist Grade' } })

        await submit('alice', scratchAutoPlay('Hard', 1000, 1))
        await submit('bob', scratchAutoPlay('Normal', 1500, 2))
        await submit('carol', scratchAutoPlay('AssistEasy', 2000, 3))
        await submit('dave', { clear: 'Easy', gauge: 'Easy', exScore: 100, offset: 4, scratchAuto: false })
        await submit('erin', scratchAutoPlay('ExHard', 500, 5))
    })

    test('chart_best 에 끝난 게이지로 파생한 어시스트 등급이 저장된다', async () => {
        expect(
            await Promise.all(
                (['alice', 'bob', 'carol', 'erin'] as const).map(async (player) => [player, (await storedBest(player))?.assistGrade] as const),
            ),
        ).toEqual([
            ['alice', 3],
            ['bob', 2],
            ['carol', 0],
            ['erin', 4],
        ])
        expect((await storedBest('dave', CHART_SEED, false))?.assistGrade).toBe(0)
    })

    test('순위는 램프, 어시스트 등급, EX 순이고 EX 가 가장 높아도 등급이 낮으면 아래다', async () => {
        expect(await ranking(true)).toEqual([
            { player: 'dave', clear: 'Easy', assistGrade: 0, ex: 100, rank: 1 },
            { player: 'erin', clear: 'LightAssistEasy', assistGrade: 4, ex: 500, rank: 2 },
            { player: 'alice', clear: 'LightAssistEasy', assistGrade: 3, ex: 1000, rank: 3 },
            { player: 'bob', clear: 'LightAssistEasy', assistGrade: 2, ex: 1500, rank: 4 },
            { player: 'carol', clear: 'LightAssistEasy', assistGrade: 0, ex: 2000, rank: 5 },
        ])
        expect(await ranking(false)).toEqual([{ player: 'dave', clear: 'Easy', assistGrade: 0, ex: 100, rank: 1 }])
    })

    test('내 순위 조회와 제출 응답의 순위가 같은 키로 센다', async () => {
        const best = await compose().scoreService.getBest({ chartSha256: chartSha(CHART_SEED), loginId: userId('bob'), includeScratchAuto: true })
        expect([best?.rank, best?.assist_grade, best?.clear]).toEqual([4, 2, 'LightAssistEasy'])
    })

    test('등급이 낮은 판은 EX 가 훨씬 높아도 최고 기록을 바꾸지 못하고 높은 판은 EX 가 낮아도 바꾼다', async () => {
        const lowerGrade = await submit('alice', scratchAutoPlay('Normal', 3000, 11))
        expect([lowerGrade.response.is_new_best, lowerGrade.response.rank]).toEqual([false, 4])
        expect(await storedBest('alice')).toMatchObject({ assistGrade: 3, exScore: 1000 })

        const higherGrade = await submit('alice', scratchAutoPlay('ExHard', 800, 12))
        expect([higherGrade.response.is_new_best, higherGrade.response.rank]).toEqual([true, 2])
        expect(await storedBest('alice')).toMatchObject({ assistGrade: 4, exScore: 800 })
        expect(await ranking(true)).toMatchObject([
            { player: 'dave' },
            { player: 'alice', assistGrade: 4, ex: 800 },
            { player: 'erin' },
            { player: 'bob' },
            { player: 'carol' },
        ])
    })

    test('같은 등급이면 EX 가 높은 판이 이긴다', async () => {
        const higherEx = await submit('alice', scratchAutoPlay('Hazard', 900, 13))
        expect(higherEx.response.is_new_best).toBe(true)
        expect(await storedBest('alice')).toMatchObject({ assistGrade: 4, exScore: 900 })
        const lowerEx = await submit('alice', scratchAutoPlay('ExHard', 850, 14))
        expect(lowerEx.response.is_new_best).toBe(false)
    })

    test('같은 판을 다시 보내도 최고 기록과 순위가 변하지 않는다', async () => {
        const first = await submit('bob', scratchAutoPlay('Hard', 700, 21))
        expect([first.response.is_new_best, first.duplicated]).toEqual([true, false])
        const again = await submit('bob', scratchAutoPlay('Hard', 700, 21))
        expect([again.duplicated, again.response.is_new_best, again.response.rank]).toEqual([true, false, first.response.rank])
        expect(await storedBest('bob')).toMatchObject({ assistGrade: 3, exScore: 700 })
    })

    test('SCRATCH AUTO 칸과 일반 칸을 합치면 더 높은 쪽을 보여 주되 같은 키로 고른다', async () => {
        await submit('frank', { clear: 'Hard', gauge: 'Hard', exScore: 100, offset: 31, scratchAuto: false })
        await submit('frank', scratchAutoPlay('ExHard', 3000, 32))
        const frank = (await ranking(true)).find((row) => row.player === 'frank')
        expect([frank?.clear, frank?.assistGrade, frank?.ex]).toEqual(['Hard', 0, 100])
        await submit('erin', { clear: 'LightAssistEasy', gauge: 'Normal', exScore: 9000, offset: 33, scratchAuto: false })
        const erin = (await ranking(true)).find((row) => row.player === 'erin')
        expect([erin?.assistGrade, erin?.ex]).toEqual([4, 500])
    })

    test('플레이어 통계는 등급마다 램프를 따로 센다', async () => {
        const stats = await compose().feService.playerStats({ userId: userId('alice'), totalPlays: 4 })
        expect(stats.lamps).toEqual([{ clear: 'LightAssistEasy', assist_grade: 4, count: 1 }])
        expect(stats.cleared).toBe(1)
    })

    test('플레이 기록 목록은 판마다 그 판의 게이지로 등급을 붙인다', async () => {
        const { items } = await compose().feService.playerRecent({ userId: userId('alice'), page: 1, limit: 10 })
        expect(items.map((item) => [item.clear, item.assist_grade])).toEqual([
            ['LightAssistEasy', 4],
            ['LightAssistEasy', 4],
            ['LightAssistEasy', 4],
            ['LightAssistEasy', 2],
            ['LightAssistEasy', 3],
        ])
    })

    test('발광 표의 내 기록에 등급이 실린다', async () => {
        const table = getSnapshotTable(TABLE_ID)
        const entry = table?.entries[0]
        if (!entry) throw new Error('snapshot table missing')
        const tableChartSeed = 0xa5500002
        await getDb()
            .insert(chart)
            .values({ sha256: chartSha(tableChartSeed), md5: entry.md5, title: 'Table Grade', mode: 'BEAT_7K', notes: NOTES })
            .onDuplicateKeyUpdate({ set: { title: 'Table Grade' } })
        await submit('carol', { ...scratchAutoPlay('Hard', 600, 41), chartSeed: tableChartSeed, md5: entry.md5 })
        const records = await composeTableSnapshot(getDb()).tableSnapshotService.getRecords({ tableId: TABLE_ID, userId: userId('carol') })
        expect(records?.records[entry.md5]).toMatchObject({ clear: 'LightAssistEasy', assist_grade: 3, ex_score: 600, scratch_auto: true })
    })

    test('게이지가 없는 옛 코스 본문은 코스 최고 기록의 등급이 0 이다', async () => {
        const run = await compose().courseService.submit({
            input: courseSubmissionSchema.parse({
                course_hash: COURSE_HASH,
                player: { id: userId('alice') },
                clear: 'LightAssistEasy',
                ex_score: 5000,
                minbp: 5,
                judge,
                played_at: BASE_PLAYED_AT + 51 * PLAY_SPACING_MS,
                scratch_auto: true,
                assist: ['AUTO_SCRATCH'],
            }),
            user: { id: userId('alice'), loginId: userId('alice') },
            buildTrust: 'trusted',
            requireTrustedBuild: false,
        })
        expect(run.response.is_new_best).toBe(true)
        const [stored] = await getDb()
            .select()
            .from(courseBest)
            .where(eq(courseBest.userId, userId('alice')))
        expect(stored).toMatchObject({ clear: 3, assistGrade: 0, exScore: 5000 })
        const [first] = await compose().courseService.getRanking({ courseHash: COURSE_HASH, limit: 10, page: 1, includeScratchAuto: true })
        expect([first?.clear, first?.assist_grade]).toEqual(['LightAssistEasy', 0])
    })

    test('게이지를 실은 코스 본문은 그 게이지 등급으로 순위에 오른다', async () => {
        const run = await compose().courseService.submit({
            input: courseSubmissionSchema.parse({
                course_hash: GAUGE_COURSE_HASH,
                player: { id: userId('bob') },
                clear: 'LightAssistEasy',
                ex_score: 5000,
                minbp: 5,
                judge,
                played_at: BASE_PLAYED_AT + 52 * PLAY_SPACING_MS,
                scratch_auto: true,
                assist: ['AUTO_SCRATCH'],
                gauge: 'Class',
            }),
            user: { id: userId('bob'), loginId: userId('bob') },
            buildTrust: 'trusted',
            requireTrustedBuild: false,
        })
        expect(run.response.is_new_best).toBe(true)
        const [stored] = await getDb()
            .select()
            .from(courseBest)
            .where(and(eq(courseBest.userId, userId('bob')), eq(courseBest.courseHash, GAUGE_COURSE_HASH)))
        expect(stored).toMatchObject({ clear: 3, assistGrade: 2, exScore: 5000 })
        const [first] = await compose().courseService.getRanking({ courseHash: GAUGE_COURSE_HASH, limit: 10, page: 1, includeScratchAuto: true })
        expect([first?.clear, first?.assist_grade]).toEqual(['LightAssistEasy', 2])
    })

    test('0011 의 백필은 chart_best 행을 그 행이 가리키는 score 의 게이지 등급으로 채우고 옛 코드가 남긴 낡은 등급은 0 으로 되돌린다', async () => {
        const db = getDb()
        const backfill =
            readFileSync(new URL('../../drizzle/0011_assist_grade.sql', import.meta.url), 'utf8')
                .split('--> statement-breakpoint')
                .at(-1) ?? ''
        const cases = [
            { gauge: 0, clear: 3, expected: 0 },
            { gauge: 1, clear: 3, expected: 1 },
            { gauge: 2, clear: 3, expected: 2 },
            { gauge: 3, clear: 3, expected: 3 },
            { gauge: 4, clear: 3, expected: 4 },
            { gauge: 5, clear: 3, expected: 4 },
            { gauge: 6, clear: 3, expected: 2 },
            { gauge: 7, clear: 3, expected: 3 },
            { gauge: 8, clear: 3, expected: 4 },
            { gauge: 99, clear: 3, expected: 0 },
            { gauge: 3, clear: 6, expected: 0 },
            { gauge: 3, clear: 2, expected: 0 },
        ].map((entry, index) => ({
            ...entry,
            id: `${PREFIX}_bf_${index}`,
            chartSha256: chartSha(BACKFILL_SEED + index),
            updatedAt: new Date(Date.UTC(2026, 0, 1 + index)),
        }))
        await db.delete(chartBest).where(
            inArray(
                chartBest.chartSha256,
                cases.map((entry) => entry.chartSha256),
            ),
        )
        await db.delete(score).where(
            inArray(
                score.id,
                cases.map((entry) => entry.id),
            ),
        )
        await db
            .insert(score)
            .values(cases.map((entry) => ({ id: entry.id, chartSha256: entry.chartSha256, clear: entry.clear, gauge: entry.gauge, playedAt: 1 })))
        await db.insert(chartBest).values(
            cases.map((entry): typeof chartBest.$inferInsert => ({
                chartSha256: entry.chartSha256,
                userId: userId('frank'),
                scoreId: entry.id,
                clear: entry.clear,
                assistGrade: entry.clear === 3 ? 0 : STALE_ASSIST_GRADE,
                exScore: 7,
                minbp: 3,
                maxCombo: 5,
                scratchAuto: true,
                updatedAt: entry.updatedAt,
            })),
        )
        await db.execute(sql.raw(backfill))
        const rows = await db
            .select()
            .from(chartBest)
            .where(
                inArray(
                    chartBest.chartSha256,
                    cases.map((entry) => entry.chartSha256),
                ),
            )
        expect(rows.map((row): [string, number] => [row.scoreId, row.assistGrade]).toSorted()).toEqual(
            cases.map((entry): [string, number] => [entry.id, entry.expected]).toSorted(),
        )
        expect(rows.every((row) => row.exScore === 7 && row.minbp === 3 && row.maxCombo === 5 && row.scratchAuto)).toBe(true)
        expect(rows.map((row) => row.updatedAt.getTime()).toSorted()).toEqual(cases.map((entry) => entry.updatedAt.getTime()).toSorted())
        await db.delete(chartBest).where(
            inArray(
                chartBest.chartSha256,
                cases.map((entry) => entry.chartSha256),
            ),
        )
        await db.delete(score).where(
            inArray(
                score.id,
                cases.map((entry) => entry.id),
            ),
        )
    })
})

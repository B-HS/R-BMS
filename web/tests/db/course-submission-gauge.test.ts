import { beforeAll, describe, expect, test } from 'bun:test'
import { and, eq, inArray, isNull } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { courseBest, courseScore } from '@server/db/schema'
import { gaugeTypeToId } from '@server/dto/common'
import { courseSubmissionSchema } from '@server/dto/course'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 코스 제출 게이지 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'csg'
const COURSE_HASH = 'course-submission-gauge'
const UPGRADED_COURSE_HASH = 'course-submission-gauge-upgraded'
const BASE_PLAYED_AT = 1_797_000_000_000
const PLAY_SPACING_MS = 1_000
const PLAYERS = ['alice', 'bob', 'carol'] as const
const LOW_EX_SCORE = 100
const HIGH_EX_SCORE = 5000
const EX_HARD_CLASS_ASSIST_GRADE = 4
const CHARTS = [
    { md5: 'a'.repeat(32), sha256: 'b'.repeat(64) },
    { md5: 'c'.repeat(32), sha256: 'd'.repeat(64) },
]
const EX_HARD_CLASS_STAGE = { gauge: 'ExHardClass', random: 'Mirror', lntype: 1 } as const

type Player = (typeof PLAYERS)[number]

const userId = (player: Player) => `${PREFIX}_${player}`

const judge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

const coursePlay = (player: Player, play: { exScore: number; offset: number; courseHash?: string } & Record<string, unknown>) => {
    const { exScore, offset, courseHash, ...gaugeFields } = play
    return courseSubmissionSchema.parse({
        course_hash: courseHash ?? COURSE_HASH,
        player: { id: userId(player) },
        clear: 'LightAssistEasy',
        ex_score: exScore,
        minbp: 5,
        judge,
        charts: CHARTS,
        played_at: BASE_PLAYED_AT + offset * PLAY_SPACING_MS,
        scratch_auto: true,
        assist: ['AUTO_SCRATCH'],
        ...gaugeFields,
    })
}

describe.skipIf(!hasDatabase)('코스 제출의 게이지·스테이지·등급 (임시 MySQL)', () => {
    const submit = (player: Player, play: Parameters<typeof coursePlay>[1]) =>
        compose().courseService.submit({
            input: coursePlay(player, play),
            user: { id: userId(player), loginId: userId(player) },
            buildTrust: 'trusted',
            requireTrustedBuild: false,
        })

    const storedScore = async (player: Player) => {
        const [row] = await getDb()
            .select()
            .from(courseScore)
            .where(eq(courseScore.userId, userId(player)))
        return row
    }

    const storedBest = async (player: Player, courseHash = COURSE_HASH) => {
        const [row] = await getDb()
            .select()
            .from(courseBest)
            .where(and(eq(courseBest.userId, userId(player)), eq(courseBest.courseHash, courseHash)))
        return row
    }

    const gaugeFields = { gauge: 'ExHardClass', course_gauge: 'BEATORAJA', stages: [EX_HARD_CLASS_STAGE, EX_HARD_CLASS_STAGE] }

    beforeAll(async () => {
        const db = getDb()
        const ids = PLAYERS.map(userId)
        await db.delete(courseScore).where(inArray(courseScore.userId, ids))
        await db.delete(courseBest).where(inArray(courseBest.userId, ids))
        await db.delete(user).where(inArray(user.id, ids))
        await db
            .insert(user)
            .values(PLAYERS.map((player) => ({ id: userId(player), name: player, email: `${player}@course-gauge.invalid`, loginId: userId(player) })))
        await submit('alice', { exScore: LOW_EX_SCORE, offset: 1, ...gaugeFields })
        await submit('bob', { exScore: HIGH_EX_SCORE, offset: 2 })
    })

    test('코스 제출이 gauge·course_gauge·stages 를 저장하고 course_best.assist_grade 를 채운다', async () => {
        expect(await storedBest('alice')).toMatchObject({ clear: 3, assistGrade: EX_HARD_CLASS_ASSIST_GRADE, exScore: LOW_EX_SCORE })
        expect(await storedScore('alice')).toMatchObject({
            gauge: gaugeTypeToId('ExHardClass'),
            courseGauge: 'BEATORAJA',
            stages: CHARTS.map((chart) => ({ ...EX_HARD_CLASS_STAGE, md5: chart.md5, sha256: chart.sha256 })),
        })
    })

    test('게이지가 없는 옛 클라이언트의 본문은 게이지·코스 게이지·스테이지가 NULL 이고 등급은 0 이다', async () => {
        expect(await storedBest('bob')).toMatchObject({ clear: 3, assistGrade: 0, exScore: HIGH_EX_SCORE })
        const nulls = await getDb()
            .select({ id: courseScore.id })
            .from(courseScore)
            .where(and(eq(courseScore.userId, userId('bob')), isNull(courseScore.gauge), isNull(courseScore.courseGauge), isNull(courseScore.stages)))
        expect(nulls).toHaveLength(1)
    })

    test('등급이 있는 판은 EX 가 낮아도 등급이 없는 판 위에 순위가 매겨진다', async () => {
        const ranking = await compose().courseService.getRanking({ courseHash: COURSE_HASH, limit: 10, page: 1, includeScratchAuto: true })
        expect(ranking.map((row) => [row.player.id, row.assist_grade, row.rank])).toEqual([
            [userId('alice'), EX_HARD_CLASS_ASSIST_GRADE, 1],
            [userId('bob'), 0, 2],
        ])
    })

    test('같은 판을 다시 보내면 처음 판에 저장된 게이지로 등급을 다시 만들고 판을 늘리지 않는다', async () => {
        const again = await submit('alice', { exScore: LOW_EX_SCORE, offset: 1, gauge: 'Class', course_gauge: 'LR2', stages: [] })
        expect(again.duplicated).toBe(true)
        expect(again.scoreRow.gauge).toBe(gaugeTypeToId('ExHardClass'))
        expect(again.scoreRow.courseGauge).toBe('BEATORAJA')
        expect(again.scoreRow.stages).toHaveLength(CHARTS.length)
        expect(await storedBest('alice')).toMatchObject({ assistGrade: EX_HARD_CLASS_ASSIST_GRADE })
        expect(
            await getDb()
                .select({ id: courseScore.id })
                .from(courseScore)
                .where(eq(courseScore.userId, userId('alice'))),
        ).toHaveLength(1)
    })

    test('0012 이전 모양의 판(게이지 NULL)을 게이지를 실어 다시 보내도 등급은 0 이다', async () => {
        const again = await submit('bob', { exScore: HIGH_EX_SCORE, offset: 2, ...gaugeFields })
        expect(again.duplicated).toBe(true)
        expect(again.scoreRow).toMatchObject({ gauge: null, courseGauge: null, stages: null })
        expect(await storedBest('bob')).toMatchObject({ assistGrade: 0 })
    })

    test('등급 없이 저장된 최고 기록은 EX 가 낮아도 더 높은 등급의 판이 오면 바뀐다', async () => {
        await submit('carol', { exScore: HIGH_EX_SCORE, offset: 3, courseHash: UPGRADED_COURSE_HASH })
        expect(await storedBest('carol', UPGRADED_COURSE_HASH)).toMatchObject({ assistGrade: 0, exScore: HIGH_EX_SCORE })
        const better = await submit('carol', { exScore: LOW_EX_SCORE, offset: 4, courseHash: UPGRADED_COURSE_HASH, ...gaugeFields })
        expect(better.response.is_new_best).toBe(true)
        expect(await storedBest('carol', UPGRADED_COURSE_HASH)).toMatchObject({ assistGrade: EX_HARD_CLASS_ASSIST_GRADE, exScore: LOW_EX_SCORE })
    })
})

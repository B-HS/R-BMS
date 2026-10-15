import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { and, eq, inArray } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, courseBest, courseScore, score } from '@server/db/schema'
import { courseSubmissionSchema } from '@server/dto/course'
import { scoreSubmissionSchema } from '@server/dto/score'
import { DUPLICATE_SUBMISSION_MESSAGE } from '@server/service/domain/score/score.service'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 최고 기록 복구 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 's19b'
const PLAYER = `${PREFIX}_player`
const CHART_SHA = '0e'.repeat(32)
const COURSE_HASH = `${PREFIX}-course`
const PLAYED_AT = 1_792_000_000_000
const NOTES = 1000
const EX_SCORE = 1500
const MINBP = 7

const judge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }
const playerOf = { id: PLAYER, loginId: PLAYER, name: PLAYER }

const submitChart = () =>
    compose().scoreService.submit({
        input: scoreSubmissionSchema.parse({
            chart: { md5: '', sha256: CHART_SHA },
            player: { id: PLAYER },
            mode: 'BEAT_7K',
            clear: 'Hard',
            ex_score: EX_SCORE,
            minbp: MINBP,
            judge,
            total_notes: NOTES,
            options: { gauge: 'Hard', random: 'Off', scratch_auto: false, lntype: 0, input_device: 'keyboard', assist: [], judge_rate: 100 },
            played_at: PLAYED_AT,
        }),
        chartSha256: CHART_SHA,
        chartMd5: null,
        user: playerOf,
        buildTrust: 'trusted',
        requireTrustedBuild: false,
        ip: null,
        userAgent: null,
    })

const submitCourse = () =>
    compose().courseService.submit({
        input: courseSubmissionSchema.parse({
            course_hash: COURSE_HASH,
            player: { id: PLAYER },
            clear: 'Normal',
            ex_score: EX_SCORE,
            minbp: MINBP,
            judge,
            played_at: PLAYED_AT,
        }),
        user: playerOf,
        buildTrust: 'trusted',
        requireTrustedBuild: false,
    })

const cleanUp = async () => {
    const db = getDb()
    await db.delete(score).where(eq(score.userId, PLAYER))
    await db.delete(chartBest).where(eq(chartBest.userId, PLAYER))
    await db.delete(courseScore).where(eq(courseScore.userId, PLAYER))
    await db.delete(courseBest).where(eq(courseBest.userId, PLAYER))
    await db.delete(chart).where(inArray(chart.sha256, [CHART_SHA]))
    await db.delete(user).where(eq(user.id, PLAYER))
}

describe.skipIf(!hasDatabase)('점수는 남고 최고 기록 갱신이 빠진 판의 재전송 (임시 MySQL)', () => {
    beforeAll(async () => {
        await cleanUp()
        const db = getDb()
        await db.insert(user).values({ id: PLAYER, name: PLAYER, email: `${PLAYER}@s19.invalid`, loginId: PLAYER })
        await db.insert(chart).values({ sha256: CHART_SHA, title: PREFIX, mode: 'BEAT_7K', notes: NOTES })
    })

    afterAll(cleanUp)

    test('차트: 최고 기록 행이 없는 저장된 판을 다시 보내면 중복 응답과 함께 그 판이 최고 기록이 되고 순위도 그 뒤에 센다', async () => {
        const first = await submitChart()
        await getDb()
            .delete(chartBest)
            .where(and(eq(chartBest.userId, PLAYER), eq(chartBest.chartSha256, CHART_SHA)))

        const resent = await submitChart()
        expect(resent.duplicated).toBe(true)
        expect(resent.response).toMatchObject({ message: DUPLICATE_SUBMISSION_MESSAGE, score_id: first.scoreId, rank: 1, is_new_best: false })
        const [best] = await getDb().select().from(chartBest).where(eq(chartBest.userId, PLAYER))
        expect(best).toMatchObject({ scoreId: first.scoreId, exScore: EX_SCORE, minbp: MINBP, scratchAuto: false })

        const again = await submitChart()
        expect(again.duplicated).toBe(true)
        expect(await getDb().select().from(chartBest).where(eq(chartBest.userId, PLAYER))).toEqual([best])
    })

    test('코스: 최고 기록 행이 없는 저장된 판을 다시 보내면 그 판이 코스 최고 기록이 된다', async () => {
        const first = await submitCourse()
        await getDb().delete(courseBest).where(eq(courseBest.userId, PLAYER))

        const resent = await submitCourse()
        expect(resent.duplicated).toBe(true)
        expect(resent.response).toMatchObject({ score_id: first.courseScoreId, rank: 1, is_new_best: false })
        const [best] = await getDb().select().from(courseBest).where(eq(courseBest.userId, PLAYER))
        expect(best).toMatchObject({ courseScoreId: first.courseScoreId, exScore: EX_SCORE, minbp: MINBP })
    })
})

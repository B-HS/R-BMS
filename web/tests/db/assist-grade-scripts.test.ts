import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { inArray, like } from 'drizzle-orm'
import mysql from 'mysql2/promise'
import { getDb } from '@server/db'
import { chartBest, score } from '@server/db/schema'
import { ASSIST_GRADES, GAUGE_TYPES, clearLampToId, gaugeTypeToId, type AssistGrade } from '@server/dto/common'
import { getRequiredDatabaseUrl } from '@server/lib/env'
import { deriveAssistGrade } from '@server/service/domain/score/assist-grade'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log(
        '[skip] 어시스트 등급 보정 운영 SQL 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행',
    )
}

const SCRIPTS_DIR = join(import.meta.dirname, '..', '..', 'scripts')
const PREFIX = 'agbf'
const USER_ID = `${PREFIX}_user`
const CHART_SEED = 0xa5b00000
const SHA256_LENGTH = 64
const UNKNOWN_GAUGE_ID = 99
const LIGHT_ASSIST = clearLampToId('LightAssistEasy')
const NORMAL_CLEAR = clearLampToId('Normal')
const STALE_LABEL = 'chart_best rows whose assist_grade differs from the grade of the score they point to (assist-grade.backfill.sql would change)'
const GRADED_LABEL = 'chart_best rows with an assist_grade above 0'
const ORPHAN_LABEL = 'chart_best rows whose score row is gone (the backfill can not reach them)'
const KEPT = { exScore: 7, minbp: 3, maxCombo: 5 }

const withConnection = async <T>(run: (connection: mysql.Connection) => Promise<T>) => {
    const connection = await mysql.createConnection({ uri: getRequiredDatabaseUrl(), multipleStatements: true })
    try {
        return await run(connection)
    } finally {
        await connection.end()
    }
}

const runScript = async (name: string) => {
    const sql = await readFile(join(SCRIPTS_DIR, name), 'utf8')
    const [results] = await withConnection((connection) => connection.query(sql))
    return results
}

const affectedRowsOf = (result: unknown) => (typeof result === 'object' && result !== null && 'affectedRows' in result ? result.affectedRows : null)

const countOf = (result: unknown, label: string) =>
    (Array.isArray(result) ? result : [])
        .map((row) =>
            typeof row === 'object' && row !== null && 'target' in row && 'row_count' in row && row.target === label ? row.row_count : null,
        )
        .find((count) => count !== null)

const countsNow = async () => {
    const result = await runScript('assist-grade.count.sql')
    return { stale: countOf(result, STALE_LABEL), graded: countOf(result, GRADED_LABEL), orphan: countOf(result, ORPHAN_LABEL) }
}

type Fixture = { clear: number; gauge: number; stored: AssistGrade; expected: AssistGrade }

const gaugeFixtures = GAUGE_TYPES.map((gauge, index): Fixture => {
    const gaugeId = gaugeTypeToId(gauge)
    return {
        clear: LIGHT_ASSIST,
        gauge: gaugeId,
        stored: ASSIST_GRADES[(index + 1) % ASSIST_GRADES.length],
        expected: deriveAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeId }),
    }
})

const FIXTURES: Fixture[] = [
    ...gaugeFixtures,
    { clear: LIGHT_ASSIST, gauge: UNKNOWN_GAUGE_ID, stored: 4, expected: 0 },
    { clear: NORMAL_CLEAR, gauge: gaugeTypeToId('Hard'), stored: 4, expected: 0 },
    { clear: NORMAL_CLEAR, gauge: gaugeTypeToId('Hard'), stored: 0, expected: 0 },
    { clear: LIGHT_ASSIST, gauge: gaugeTypeToId('ExHard'), stored: 4, expected: 4 },
]

const ids = FIXTURES.map((_, index) => ({
    scoreId: `${PREFIX}_score_${index}`,
    chartSha256: (CHART_SEED + index).toString(16).padStart(SHA256_LENGTH, '0'),
    updatedAt: new Date(Date.UTC(2026, 0, 1 + index)),
}))
const ORPHAN_SHA256 = (CHART_SEED + FIXTURES.length).toString(16).padStart(SHA256_LENGTH, '0')
const STALE_FIXTURE_COUNT = FIXTURES.filter((fixture) => fixture.stored !== fixture.expected).length
const GRADED_FIXTURE_COUNT = FIXTURES.filter((fixture) => fixture.stored > 0).length
const ORPHAN_STORED_GRADE = 3

describe.skipIf(!hasDatabase)('어시스트 등급 보정 운영 SQL (임시 MySQL)', () => {
    const cleanUp = async () => {
        const db = getDb()
        await db.delete(chartBest).where(like(chartBest.userId, `${PREFIX}_%`))
        await db.delete(score).where(like(score.id, `${PREFIX}_%`))
    }

    const storedRows = () =>
        getDb()
            .select()
            .from(chartBest)
            .where(inArray(chartBest.chartSha256, [...ids.map((entry) => entry.chartSha256), ORPHAN_SHA256]))

    beforeAll(cleanUp)

    afterAll(cleanUp)

    test('낡은 행만 세고 보정은 그 행의 등급만 점수의 게이지로 되돌리며 다시 돌려도 아무 것도 바꾸지 않는다', async () => {
        const before = await countsNow()
        const db = getDb()
        await db.insert(score).values(
            FIXTURES.map((fixture, index) => ({
                id: ids[index].scoreId,
                chartSha256: ids[index].chartSha256,
                clear: fixture.clear,
                gauge: fixture.gauge,
                playedAt: 1,
            })),
        )
        await db.insert(chartBest).values([
            ...FIXTURES.map((fixture, index): typeof chartBest.$inferInsert => ({
                chartSha256: ids[index].chartSha256,
                userId: USER_ID,
                scoreId: ids[index].scoreId,
                clear: fixture.clear,
                assistGrade: fixture.stored,
                ...KEPT,
                scratchAuto: fixture.clear === LIGHT_ASSIST,
                updatedAt: ids[index].updatedAt,
            })),
            {
                chartSha256: ORPHAN_SHA256,
                userId: USER_ID,
                scoreId: `${PREFIX}_gone`,
                clear: LIGHT_ASSIST,
                assistGrade: ORPHAN_STORED_GRADE,
                ...KEPT,
                scratchAuto: true,
            },
        ])

        const counted = await countsNow()
        expect(counted.stale).toBe((before.stale ?? 0) + STALE_FIXTURE_COUNT)
        expect(counted.graded).toBe((before.graded ?? 0) + GRADED_FIXTURE_COUNT + 1)
        expect(counted.orphan).toBe((before.orphan ?? 0) + 1)

        const changed = affectedRowsOf(await runScript('assist-grade.backfill.sql'))
        expect(changed).toBe(counted.stale)

        const rows = await storedRows()
        expect(rows.map((row): [string, number] => [row.scoreId, row.assistGrade]).toSorted()).toEqual(
            [
                ...FIXTURES.map((fixture, index): [string, number] => [ids[index].scoreId, fixture.expected]),
                [`${PREFIX}_gone`, ORPHAN_STORED_GRADE] as [string, number],
            ].toSorted(),
        )
        expect(rows.every((row) => row.exScore === KEPT.exScore && row.minbp === KEPT.minbp && row.maxCombo === KEPT.maxCombo)).toBe(true)
        expect(
            rows
                .filter((row) => row.scoreId !== `${PREFIX}_gone`)
                .map((row) => row.updatedAt.getTime())
                .toSorted(),
        ).toEqual(ids.map((entry) => entry.updatedAt.getTime()).toSorted())

        const after = await countsNow()
        expect(after.stale).toBe(0)
        expect(after.orphan).toBe(counted.orphan)
        expect(affectedRowsOf(await runScript('assist-grade.backfill.sql'))).toBe(0)
    })
})

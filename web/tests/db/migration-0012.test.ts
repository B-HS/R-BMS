import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { eq, like, sql } from 'drizzle-orm'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { courseChart, courseScore, userDan, userDanDisplay } from '@server/db/schema'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 0012 마이그레이션 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const DRIZZLE_DIR = join(import.meta.dirname, '..', '..', 'drizzle')
const MIGRATION_TAG = '0012_course_dan_tables'
const PREVIOUS_MIGRATION_TAG = '0011_assist_grade'
const STATEMENT_BREAKPOINT = '--> statement-breakpoint'
const CREATED_TABLE = /^CREATE TABLE `([^`]+)`/
const ADDED_COLUMN = /^ALTER TABLE `([^`]+)` ADD `([^`]+)` /
const ADDED_CONSTRAINT = /^ALTER TABLE `([^`]+)` ADD CONSTRAINT `[^`]+` FOREIGN KEY/
const CREATED_INDEX = /^CREATE INDEX `[^`]+` ON `([^`]+)`/
const NOT_NULL = 'NOT NULL'
const DEFAULT_CLAUSE = 'DEFAULT'

const NEW_TABLES = ['course_group', 'course_group_member', 'course_source', 'dan_course', 'table_snapshot', 'user_dan', 'user_dan_display']
const NEW_COLUMNS_OF_EXISTING_TABLES = [
    'course_chart.md5',
    'course_chart.title',
    'course_score.course_gauge',
    'course_score.gauge',
    'course_score.stages',
]
const EXISTING_TABLES_WITH_NEW_COLUMNS = ['course_chart', 'course_score']

const PREFIX = 'm12'
const OWNER = `${PREFIX}_owner`
const COURSE_HASH = `${PREFIX}_course`
const SCORE_ID = `${PREFIX}_score`
const MD5_LENGTH = 32
const SHA256_LENGTH = 64
const MD5_ONLY_CHART = 'a'.repeat(MD5_LENGTH)
const SHA256_CHART = 'b'.repeat(SHA256_LENGTH)
const PLAYED_AT = Date.UTC(2026, 9, 23, 12)
const FIRST_POSITION = 0
const SECOND_POSITION = 1
const DAN_RANK = 1
const GAUGE_ID = 8

type JournalEntry = { idx: number; when: number; tag: string }

const readJournalEntries = () => {
    const journal: { entries: JournalEntry[] } = JSON.parse(readFileSync(join(DRIZZLE_DIR, 'meta', '_journal.json'), 'utf8'))
    return journal.entries
}

const readStatements = () =>
    readFileSync(join(DRIZZLE_DIR, `${MIGRATION_TAG}.sql`), 'utf8')
        .split(STATEMENT_BREAKPOINT)
        .map((statement) => statement.trim())
        .filter((statement) => statement.length > 0)

const namesOf = (rows: unknown, key: string) => (Array.isArray(rows) ? rows.map((row) => String(row[key])) : [])

describe('0012 마이그레이션 파일', () => {
    test('0012 는 기존 표의 열 타입을 바꾸지 않고 열과 표를 더하기만 한다', () => {
        const statements = readStatements()
        const createdTables = statements.flatMap((statement) => CREATED_TABLE.exec(statement)?.[1] ?? [])
        const addedColumns = statements.flatMap((statement) => {
            const match = ADDED_COLUMN.exec(statement)
            return match ? [{ table: match[1], column: match[2], statement }] : []
        })
        const constrainedTables = statements.flatMap((statement) => ADDED_CONSTRAINT.exec(statement)?.[1] ?? [])
        const indexedTables = statements.flatMap((statement) => CREATED_INDEX.exec(statement)?.[1] ?? [])

        expect(createdTables.toSorted()).toEqual(NEW_TABLES)
        expect(addedColumns.map((added) => `${added.table}.${added.column}`).toSorted()).toEqual(NEW_COLUMNS_OF_EXISTING_TABLES)
        expect(statements).toHaveLength(createdTables.length + addedColumns.length + constrainedTables.length + indexedTables.length)
        for (const added of addedColumns) {
            expect(EXISTING_TABLES_WITH_NEW_COLUMNS).toContain(added.table)
            expect(!added.statement.includes(NOT_NULL) || added.statement.includes(DEFAULT_CLAUSE)).toBe(true)
        }
        for (const table of [...constrainedTables, ...indexedTables]) expect(createdTables).toContain(table)
    })

    test('0012 의 저널 시각은 0011 보다 커서 drizzle 이 건너뛰지 않는다', () => {
        const entries = readJournalEntries()
        const previous = entries.find((entry) => entry.tag === PREVIOUS_MIGRATION_TAG)
        const current = entries.find((entry) => entry.tag === MIGRATION_TAG)
        expect(previous).toBeDefined()
        expect(current?.idx).toBe((previous?.idx ?? Number.NaN) + 1)
        expect(current?.when).toBeGreaterThan(previous?.when ?? Number.POSITIVE_INFINITY)
    })
})

describe.skipIf(!hasDatabase)('0012 마이그레이션 (임시 MySQL)', () => {
    beforeAll(async () => {
        const db = getDb()
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await db.delete(courseChart).where(like(courseChart.courseHash, `${PREFIX}_%`))
        await db.delete(courseScore).where(like(courseScore.id, `${PREFIX}_%`))
        await db.insert(user).values({ id: OWNER, name: OWNER, email: `${OWNER}@m12.invalid`, loginId: OWNER })
    })

    afterAll(async () => {
        const db = getDb()
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await db.delete(courseChart).where(like(courseChart.courseHash, `${PREFIX}_%`))
        await db.delete(courseScore).where(like(courseScore.id, `${PREFIX}_%`))
    })

    test('마이그레이션이 새 표 일곱 개와 기존 표의 새 열 다섯 개를 만든다', async () => {
        const [tables] = await getDb().execute(
            sql`SELECT table_name AS name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name IN (${sql.join(
                NEW_TABLES.map((name) => sql`${name}`),
                sql`, `,
            )})`,
        )
        expect(namesOf(tables, 'name').toSorted()).toEqual(NEW_TABLES)
        const [columns] = await getDb().execute(
            sql`SELECT CONCAT(table_name, '.', column_name) AS name FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name IN (${sql.join(
                EXISTING_TABLES_WITH_NEW_COLUMNS.map((name) => sql`${name}`),
                sql`, `,
            )}) AND column_name IN ('md5', 'title', 'gauge', 'course_gauge', 'stages')`,
        )
        expect(namesOf(columns, 'name').toSorted()).toEqual(NEW_COLUMNS_OF_EXISTING_TABLES)
    })

    test('0012 뒤 course_chart 에 sha256 이 빈 md5 차트를 넣고 읽을 수 있다', async () => {
        const db = getDb()
        await db.insert(courseChart).values([
            { courseHash: COURSE_HASH, position: FIRST_POSITION, chartSha256: '', md5: MD5_ONLY_CHART, title: 'md5 만 있는 곡' },
            { courseHash: COURSE_HASH, position: SECOND_POSITION, chartSha256: SHA256_CHART },
        ])
        const rows = await db.select().from(courseChart).where(eq(courseChart.courseHash, COURSE_HASH)).orderBy(courseChart.position)
        expect(rows).toEqual([
            { courseHash: COURSE_HASH, position: FIRST_POSITION, chartSha256: '', md5: MD5_ONLY_CHART, title: 'md5 만 있는 곡' },
            { courseHash: COURSE_HASH, position: SECOND_POSITION, chartSha256: SHA256_CHART, md5: '', title: '' },
        ])
    })

    test('course_score 의 새 열은 비어 있어도 되어 0012 이전 모양의 판을 그대로 넣을 수 있다', async () => {
        const db = getDb()
        await db.insert(courseScore).values({ id: SCORE_ID, courseHash: COURSE_HASH, playedAt: PLAYED_AT })
        const [row] = await db.select().from(courseScore).where(eq(courseScore.id, SCORE_ID))
        expect(row).toMatchObject({ gauge: null, courseGauge: null, stages: null })
    })

    test('user_dan 과 user_dan_display 는 사용자를 지우면 같이 지워진다', async () => {
        const db = getDb()
        await db.insert(userDan).values({
            userId: OWNER,
            system: 'genoside-sp',
            label: 'first',
            rank: DAN_RANK,
            courseHash: COURSE_HASH,
            courseScoreId: SCORE_ID,
            courseGauge: 'LR2',
            gauge: GAUGE_ID,
            acquiredAt: PLAYED_AT,
        })
        await db.insert(userDanDisplay).values({ userId: OWNER, system: 'genoside-sp', label: null })
        expect(await db.select().from(userDan).where(eq(userDan.userId, OWNER))).toHaveLength(1)
        expect(await db.select().from(userDanDisplay).where(eq(userDanDisplay.userId, OWNER))).toHaveLength(1)

        await db.delete(user).where(eq(user.id, OWNER))

        expect(await db.select().from(userDan).where(eq(userDan.userId, OWNER))).toEqual([])
        expect(await db.select().from(userDanDisplay).where(eq(userDanDisplay.userId, OWNER))).toEqual([])
    })
})

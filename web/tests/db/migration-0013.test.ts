import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { eq, like, sql } from 'drizzle-orm'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { userDan, userDanDisplay } from '@server/db/schema'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 0013 마이그레이션 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const DRIZZLE_DIR = join(import.meta.dirname, '..', '..', 'drizzle')
const MIGRATION_TAG = '0013_assisted_dan'
const PREVIOUS_MIGRATION_TAG = '0012_course_dan_tables'
const STATEMENT_BREAKPOINT = '--> statement-breakpoint'
const EXPECTED_STATEMENTS = [
    'ALTER TABLE `user_dan` ADD `assisted` boolean DEFAULT false NOT NULL;',
    'ALTER TABLE `user_dan` DROP PRIMARY KEY, ADD PRIMARY KEY(`user_id`,`system`,`label`,`assisted`);',
    'ALTER TABLE `user_dan_display` ADD `assisted` boolean DEFAULT false NOT NULL;',
]
const USER_DAN_PRIMARY_KEY = ['user_id', 'system', 'label', 'assisted']
const USER_DAN_DISPLAY_PRIMARY_KEY = ['user_id', 'system']

const PREFIX = 'm13'
const OWNER = `${PREFIX}_owner`
const SYSTEM = 'genoside2018_sp'
const LABEL = '十段'
const DAN_RANK = 10
const GAUGE_ID = 6
const ACQUIRED_AT = Date.UTC(2026, 9, 24, 12)

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

const primaryKeyOf = async (table: string) => {
    const [rows] = await getDb().execute(
        sql`SELECT column_name AS name FROM information_schema.key_column_usage WHERE table_schema = DATABASE() AND table_name = ${table} AND constraint_name = 'PRIMARY' ORDER BY ordinal_position`,
    )
    return namesOf(rows, 'name')
}

const danRow = (overrides: Partial<typeof userDan.$inferInsert> = {}): typeof userDan.$inferInsert => ({
    userId: OWNER,
    system: SYSTEM,
    label: LABEL,
    rank: DAN_RANK,
    courseHash: `${PREFIX}_course`,
    courseScoreId: `${PREFIX}_score`,
    courseGauge: 'LR2',
    gauge: GAUGE_ID,
    acquiredAt: ACQUIRED_AT,
    ...overrides,
})

describe('0013 마이그레이션 파일', () => {
    test('0013 은 두 열을 기본값 false 로 더하고 user_dan 의 기본 키를 한 문장으로 바꾼다', () => {
        expect(readStatements()).toEqual(EXPECTED_STATEMENTS)
    })

    test('0013 의 저널 시각은 0012 보다 커서 drizzle 이 건너뛰지 않는다', () => {
        const entries = readJournalEntries()
        const previous = entries.find((entry) => entry.tag === PREVIOUS_MIGRATION_TAG)
        const current = entries.find((entry) => entry.tag === MIGRATION_TAG)
        expect(previous).toBeDefined()
        expect(current?.idx).toBe((previous?.idx ?? Number.NaN) + 1)
        expect(current?.when).toBeGreaterThan(previous?.when ?? Number.POSITIVE_INFINITY)
    })
})

describe.skipIf(!hasDatabase)('0013 마이그레이션 (임시 MySQL)', () => {
    beforeAll(async () => {
        const db = getDb()
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await db.insert(user).values({ id: OWNER, name: OWNER, email: `${OWNER}@m13.invalid`, loginId: OWNER })
    })

    afterAll(async () => {
        await getDb()
            .delete(user)
            .where(like(user.id, `${PREFIX}_%`))
    })

    test('user_dan 의 기본 키는 assisted 를 마지막 열로 넣고 user_dan_display 의 기본 키는 그대로다', async () => {
        expect(await primaryKeyOf('user_dan')).toEqual(USER_DAN_PRIMARY_KEY)
        expect(await primaryKeyOf('user_dan_display')).toEqual(USER_DAN_DISPLAY_PRIMARY_KEY)
    })

    test('assisted 를 보내지 않는 옛 삽입은 정식 단위가 된다', async () => {
        const db = getDb()
        await db.insert(userDan).values(danRow())
        await db.insert(userDanDisplay).values({ userId: OWNER, system: SYSTEM, label: LABEL })
        expect((await db.select().from(userDan).where(eq(userDan.userId, OWNER))).map((row) => row.assisted)).toEqual([false])
        expect((await db.select().from(userDanDisplay).where(eq(userDanDisplay.userId, OWNER))).map((row) => row.assisted)).toEqual([false])
    })

    test('같은 label 의 정식 행과 A-단위 행은 함께 있고 같은 종류의 두 번째 행은 무시된다', async () => {
        const db = getDb()
        const [assisted] = await db
            .insert(userDan)
            .ignore()
            .values(danRow({ assisted: true, courseScoreId: `${PREFIX}_assisted` }))
        expect(assisted.affectedRows).toBe(1)
        const [again] = await db
            .insert(userDan)
            .ignore()
            .values(danRow({ assisted: true, courseScoreId: `${PREFIX}_later` }))
        expect(again.affectedRows).toBe(0)
        const rows = await db.select().from(userDan).where(eq(userDan.userId, OWNER)).orderBy(userDan.assisted)
        expect(rows.map((row) => [row.label, row.assisted, row.courseScoreId])).toEqual([
            [LABEL, false, `${PREFIX}_score`],
            [LABEL, true, `${PREFIX}_assisted`],
        ])
    })
})

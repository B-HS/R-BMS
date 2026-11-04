import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { eq, like, sql } from 'drizzle-orm'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { playProfile, trainingChart, trainingFolder, userFolder } from '@server/db/schema'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 0014 마이그레이션 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const DRIZZLE_DIR = join(import.meta.dirname, '..', '..', 'drizzle')
const MIGRATION_TAG = '0014_training_chart'
const PREVIOUS_MIGRATION_TAG = '0013_assisted_dan'
const STATEMENT_BREAKPOINT = '--> statement-breakpoint'

const createTable = (name: string, lines: string[]) => `CREATE TABLE \`${name}\` (\n\t${lines.join(',\n\t')}\n);`

const EXPECTED_STATEMENTS = [
    createTable('play_profile', [
        '`user_id` varchar(36) NOT NULL',
        '`window_key` varchar(16) NOT NULL',
        '`profile_version` int NOT NULL',
        '`fingerprint` char(64) NOT NULL',
        '`data` json NOT NULL',
        '`computed_at` bigint NOT NULL',
        'CONSTRAINT `play_profile_user_id_window_key_pk` PRIMARY KEY(`user_id`,`window_key`)',
    ]),
    createTable('training_chart', [
        '`id` varchar(36) NOT NULL',
        '`user_id` varchar(36) NOT NULL',
        '`md5` char(32) NOT NULL',
        '`sha256` char(64) NOT NULL',
        '`title` varchar(200) NOT NULL',
        '`play_level` int NOT NULL',
        '`bpm` int NOT NULL',
        '`note_count` int NOT NULL',
        '`length_ms` int NOT NULL',
        '`generator_version` int NOT NULL',
        '`spec` json NOT NULL',
        '`request` json NOT NULL',
        '`profile_snapshot` json NOT NULL',
        '`bms` mediumblob NOT NULL',
        '`provider` varchar(16) NOT NULL',
        '`model` varchar(128) NOT NULL',
        '`created_at` bigint NOT NULL',
        'CONSTRAINT `training_chart_id` PRIMARY KEY(`id`)',
        'CONSTRAINT `training_chart_md5_uniq` UNIQUE(`md5`)',
    ]),
    createTable('training_folder', [
        '`user_id` varchar(36) NOT NULL',
        '`folder_id` varchar(36) NOT NULL',
        '`created_at` bigint NOT NULL',
        'CONSTRAINT `training_folder_user_id` PRIMARY KEY(`user_id`)',
    ]),
    'ALTER TABLE `play_profile` ADD CONSTRAINT `play_profile_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;',
    'ALTER TABLE `training_chart` ADD CONSTRAINT `training_chart_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;',
    'ALTER TABLE `training_folder` ADD CONSTRAINT `training_folder_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE cascade ON UPDATE no action;',
    'ALTER TABLE `training_folder` ADD CONSTRAINT `training_folder_folder_id_user_folder_id_fk` FOREIGN KEY (`folder_id`) REFERENCES `user_folder`(`id`) ON DELETE cascade ON UPDATE no action;',
    'CREATE INDEX `training_chart_sha256_idx` ON `training_chart` (`sha256`);',
    'CREATE INDEX `training_chart_user_created_idx` ON `training_chart` (`user_id`,`created_at`);',
]

const EXPECTED_COLUMNS = {
    training_chart: [
        'id varchar(36) NO',
        'user_id varchar(36) NO',
        'md5 char(32) NO',
        'sha256 char(64) NO',
        'title varchar(200) NO',
        'play_level int NO',
        'bpm int NO',
        'note_count int NO',
        'length_ms int NO',
        'generator_version int NO',
        'spec json NO',
        'request json NO',
        'profile_snapshot json NO',
        'bms mediumblob NO',
        'provider varchar(16) NO',
        'model varchar(128) NO',
        'created_at bigint NO',
    ],
    training_folder: ['user_id varchar(36) NO', 'folder_id varchar(36) NO', 'created_at bigint NO'],
    play_profile: [
        'user_id varchar(36) NO',
        'window_key varchar(16) NO',
        'profile_version int NO',
        'fingerprint char(64) NO',
        'data json NO',
        'computed_at bigint NO',
    ],
}

const EXPECTED_INDEXES = {
    training_chart: [
        'PRIMARY unique id',
        'training_chart_md5_uniq unique md5',
        'training_chart_sha256_idx index sha256',
        'training_chart_user_created_idx index user_id,created_at',
    ],
    training_folder: ['PRIMARY unique user_id', 'training_folder_folder_id_user_folder_id_fk index folder_id'],
    play_profile: ['PRIMARY unique user_id,window_key'],
}

const EXPECTED_FOREIGN_KEYS = {
    training_chart: ['training_chart_user_id_user_id_fk user_id -> user.id CASCADE'],
    training_folder: [
        'training_folder_folder_id_user_folder_id_fk folder_id -> user_folder.id CASCADE',
        'training_folder_user_id_user_id_fk user_id -> user.id CASCADE',
    ],
    play_profile: ['play_profile_user_id_user_id_fk user_id -> user.id CASCADE'],
}

const PREFIX = 'm14'
const OWNER = `${PREFIX}_owner`
const OTHER = `${PREFIX}_other`
const FOLDER_ID = `${PREFIX}_folder`
const CHART_ID = `tc_${'0'.repeat(32)}`
const OTHER_CHART_ID = `tc_${'1'.repeat(32)}`
const MD5 = 'a'.repeat(32)
const OTHER_MD5 = 'b'.repeat(32)
const SHA256 = 'c'.repeat(64)
const FINGERPRINT = 'd'.repeat(64)
const WINDOW_KEY = 'recent90'
const OTHER_WINDOW_KEY = 'recent30'
const CREATED_AT = Date.UTC(2026, 10, 4, 12)
const PLAY_LEVEL = 5
const BPM = 160
const NOTE_COUNT = 120
const LENGTH_MS = 60_000
const GONE_USER = `${PREFIX}_gone`
const GONE_FOLDER_ID = `${PREFIX}_gone_folder`
const GONE_CHART_ID = `tc_${'2'.repeat(32)}`
const GONE_MD5 = 'e'.repeat(32)
const SECOND_FOLDER_ID = `${PREFIX}_second_folder`
const BMS_BYTES = Buffer.from([0x00, 0x7f, 0x80, 0xff, 0x0a, 0x23])
const SNAPSHOT = { profile_version: 1, fingerprint: FINGERPRINT, computed_at: CREATED_AT, targets: {} }

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

const rowsOf = (result: unknown) => (Array.isArray(result) && Array.isArray(result[0]) ? result[0] : [])

const columnsOf = async (table: string) =>
    rowsOf(
        await getDb().execute(
            sql`SELECT column_name AS name, column_type AS type, is_nullable AS nullable FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = ${table} ORDER BY ordinal_position`,
        ),
    ).map((row) => `${row.name} ${row.type} ${row.nullable}`)

const indexesOf = async (table: string) =>
    rowsOf(
        await getDb().execute(
            sql`SELECT index_name AS name, non_unique AS nonUnique, GROUP_CONCAT(column_name ORDER BY seq_in_index) AS cols FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = ${table} GROUP BY index_name, non_unique ORDER BY index_name`,
        ),
    ).map((row) => `${row.name} ${Number(row.nonUnique) === 0 ? 'unique' : 'index'} ${row.cols}`)

const foreignKeysOf = async (table: string) =>
    rowsOf(
        await getDb().execute(
            sql`SELECT k.constraint_name AS name, k.column_name AS col, k.referenced_table_name AS refTable, k.referenced_column_name AS refCol, r.delete_rule AS deleteRule FROM information_schema.key_column_usage k JOIN information_schema.referential_constraints r ON r.constraint_schema = k.constraint_schema AND r.table_name = k.table_name AND r.constraint_name = k.constraint_name WHERE k.table_schema = DATABASE() AND k.table_name = ${table} AND k.referenced_table_name IS NOT NULL ORDER BY k.constraint_name`,
        ),
    ).map((row) => `${row.name} ${row.col} -> ${row.refTable}.${row.refCol} ${String(row.deleteRule).replace(' ', '_')}`)

const isDuplicateEntry = async (pending: Promise<unknown>) => {
    try {
        await pending
        return false
    } catch (error) {
        return hasMysqlErrorCode(error, [MYSQL_ERROR_CODE.DUPLICATE_ENTRY])
    }
}

const profileRow = (overrides: Partial<typeof playProfile.$inferInsert> = {}): typeof playProfile.$inferInsert => ({
    userId: OWNER,
    windowKey: WINDOW_KEY,
    profileVersion: 1,
    fingerprint: FINGERPRINT,
    data: { cells: [] },
    computedAt: CREATED_AT,
    ...overrides,
})

const chartRow = (overrides: Partial<typeof trainingChart.$inferInsert> = {}): typeof trainingChart.$inferInsert => ({
    id: CHART_ID,
    userId: OWNER,
    md5: MD5,
    sha256: SHA256,
    title: 'TRAINING',
    playLevel: PLAY_LEVEL,
    bpm: BPM,
    noteCount: NOTE_COUNT,
    lengthMs: LENGTH_MS,
    generatorVersion: 1,
    spec: { sections: [] },
    request: { focus_key: 'density', intensity: 'normal' },
    profileSnapshot: SNAPSHOT,
    bms: BMS_BYTES,
    provider: 'anthropic',
    model: 'test-model',
    createdAt: CREATED_AT,
    ...overrides,
})

describe('0014 마이그레이션 파일', () => {
    test('0014 는 세 표와 두 색인과 네 외래 키를 이 순서의 아홉 문장으로 만든다', () => {
        expect(readStatements()).toEqual(EXPECTED_STATEMENTS)
    })

    test('0014 는 기본 키를 바꾸는 문장을 담지 않아 0013 의 1553 실패 경로를 타지 않는다', () => {
        expect(readStatements().filter((statement) => /DROP PRIMARY KEY|DROP FOREIGN KEY|DROP INDEX/.test(statement))).toEqual([])
    })

    test('0014 의 저널 시각은 0013 보다 커서 drizzle 이 건너뛰지 않는다', () => {
        const entries = readJournalEntries()
        const previous = entries.find((entry) => entry.tag === PREVIOUS_MIGRATION_TAG)
        const current = entries.find((entry) => entry.tag === MIGRATION_TAG)
        expect(previous).toBeDefined()
        expect(current?.idx).toBe((previous?.idx ?? Number.NaN) + 1)
        expect(current?.when).toBeGreaterThan(previous?.when ?? Number.POSITIVE_INFINITY)
    })
})

describe.skipIf(!hasDatabase)('0014 마이그레이션 (임시 MySQL)', () => {
    beforeAll(async () => {
        const db = getDb()
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await db.insert(user).values([OWNER, OTHER].map((id) => ({ id, name: id, email: `${id}@m14.invalid`, loginId: id })))
    })

    beforeEach(async () => {
        const db = getDb()
        await db.delete(trainingChart).where(like(trainingChart.userId, `${PREFIX}_%`))
        await db.delete(trainingFolder).where(like(trainingFolder.userId, `${PREFIX}_%`))
        await db.delete(playProfile).where(like(playProfile.userId, `${PREFIX}_%`))
        await db.delete(userFolder).where(like(userFolder.userId, `${PREFIX}_%`))
        await db.delete(user).where(eq(user.id, GONE_USER))
    })

    afterAll(async () => {
        await getDb()
            .delete(user)
            .where(like(user.id, `${PREFIX}_%`))
    })

    test('세 표의 열은 4절의 이름·타입·널 허용 여부 그대로다', async () => {
        for (const [table, expected] of Object.entries(EXPECTED_COLUMNS)) expect(await columnsOf(table)).toEqual(expected)
    })

    test('세 표의 기본 키·고유 키·색인이 4절 그대로다', async () => {
        for (const [table, expected] of Object.entries(EXPECTED_INDEXES)) expect(await indexesOf(table)).toEqual(expected)
    })

    test('외래 키는 모두 삭제 시 CASCADE 이고 training_folder 는 user_folder 도 가리킨다', async () => {
        for (const [table, expected] of Object.entries(EXPECTED_FOREIGN_KEYS)) expect(await foreignKeysOf(table)).toEqual(expected)
    })

    test('user 를 지우면 세 표의 행이 함께 지워진다', async () => {
        const db = getDb()
        await db.insert(user).values({ id: GONE_USER, name: GONE_USER, email: `${GONE_USER}@m14.invalid`, loginId: GONE_USER })
        await db.insert(userFolder).values({ id: GONE_FOLDER_ID, userId: GONE_USER, name: 'training' })
        await db.insert(trainingChart).values(chartRow({ id: GONE_CHART_ID, userId: GONE_USER, md5: GONE_MD5 }))
        await db.insert(trainingFolder).values({ userId: GONE_USER, folderId: GONE_FOLDER_ID, createdAt: CREATED_AT })
        await db.insert(playProfile).values(profileRow({ userId: GONE_USER }))
        await db.delete(user).where(eq(user.id, GONE_USER))
        const remaining = await Promise.all([
            db.select().from(trainingChart).where(eq(trainingChart.userId, GONE_USER)),
            db.select().from(trainingFolder).where(eq(trainingFolder.userId, GONE_USER)),
            db.select().from(playProfile).where(eq(playProfile.userId, GONE_USER)),
            db.select().from(userFolder).where(eq(userFolder.userId, GONE_USER)),
        ])
        expect(remaining.map((rows) => rows.length)).toEqual([0, 0, 0, 0])
    })

    test('user_folder 를 지우면 training_folder 행만 지워지고 사용자와 훈련곡은 남는다', async () => {
        const db = getDb()
        await db.insert(userFolder).values({ id: FOLDER_ID, userId: OWNER, name: 'training' })
        await db.insert(trainingFolder).values({ userId: OWNER, folderId: FOLDER_ID, createdAt: CREATED_AT })
        await db.insert(trainingChart).values(chartRow())
        await db.delete(userFolder).where(eq(userFolder.id, FOLDER_ID))
        expect(await db.select().from(trainingFolder).where(eq(trainingFolder.userId, OWNER))).toEqual([])
        expect((await db.select().from(user).where(eq(user.id, OWNER))).length).toBe(1)
        expect((await db.select().from(trainingChart).where(eq(trainingChart.userId, OWNER))).map((row) => row.id)).toEqual([CHART_ID])
    })

    test('BMS 바이트와 JSON 열은 그대로 돌아온다', async () => {
        const db = getDb()
        await db.insert(trainingChart).values(chartRow())
        const [stored] = await db.select().from(trainingChart).where(eq(trainingChart.id, CHART_ID))
        expect(Buffer.isBuffer(stored?.bms)).toBe(true)
        expect(stored?.bms.equals(BMS_BYTES)).toBe(true)
        expect(stored?.spec).toEqual({ sections: [] })
        expect(stored?.profileSnapshot).toEqual(SNAPSHOT)
        expect(stored?.createdAt).toBe(CREATED_AT)
    })

    test('md5 는 사용자와 관계없이 하나뿐이고 sha256 은 겹칠 수 있다', async () => {
        const db = getDb()
        await db.insert(trainingChart).values(chartRow())
        expect(await isDuplicateEntry(db.insert(trainingChart).values(chartRow({ id: OTHER_CHART_ID, userId: OTHER })))).toBe(true)
        await db.insert(trainingChart).values(chartRow({ id: OTHER_CHART_ID, userId: OTHER, md5: OTHER_MD5 }))
        expect((await db.select().from(trainingChart).where(eq(trainingChart.sha256, SHA256))).length).toBe(2)
    })

    test('play_profile 은 (user_id, window_key) 하나에 한 행이고 창 키가 다르면 따로 둔다', async () => {
        const db = getDb()
        await db.insert(playProfile).values(profileRow())
        expect(await isDuplicateEntry(db.insert(playProfile).values(profileRow()))).toBe(true)
        await db.insert(playProfile).values(profileRow({ windowKey: OTHER_WINDOW_KEY }))
        expect((await db.select().from(playProfile).where(eq(playProfile.userId, OWNER))).map((stored) => stored.windowKey).toSorted()).toEqual([
            OTHER_WINDOW_KEY,
            WINDOW_KEY,
        ])
    })

    test('training_folder 는 사용자당 한 행이다', async () => {
        const db = getDb()
        await db.insert(userFolder).values([
            { id: FOLDER_ID, userId: OWNER, name: 'training' },
            { id: SECOND_FOLDER_ID, userId: OWNER, name: 'training two' },
        ])
        await db.insert(trainingFolder).values({ userId: OWNER, folderId: FOLDER_ID, createdAt: CREATED_AT })
        expect(await isDuplicateEntry(db.insert(trainingFolder).values({ userId: OWNER, folderId: SECOND_FOLDER_ID, createdAt: CREATED_AT }))).toBe(
            true,
        )
    })
})

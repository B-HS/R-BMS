import { afterAll, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { eq, sql } from 'drizzle-orm'
import { getDb } from '@server/db'
import { chartBlocklist } from '@server/db/schema'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 0015 마이그레이션 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const DRIZZLE_DIR = join(import.meta.dirname, '..', '..', 'drizzle')
const MIGRATION_TAG = '0015_chart_blocklist'
const PREVIOUS_MIGRATION_TAG = '0014_training_chart'
const STATEMENT_BREAKPOINT = '--> statement-breakpoint'
const SOURCE_ID = 'm15-source'

const EXPECTED_STATEMENTS = [
    [
        'CREATE TABLE `chart_blocklist` (',
        '\t`sha256` char(64) NOT NULL,',
        '\t`md5` char(32) NOT NULL,',
        '\t`source_id` varchar(64) NOT NULL,',
        '\t`source_label` varchar(200) NOT NULL,',
        '\t`path` varchar(512) NOT NULL,',
        '\t`added_by` varchar(36),',
        '\t`created_at` timestamp(3) NOT NULL DEFAULT (now()),',
        '\tCONSTRAINT `chart_blocklist_sha256` PRIMARY KEY(`sha256`),',
        '\tCONSTRAINT `chart_blocklist_md5_uniq` UNIQUE(`md5`)',
        ');',
    ].join('\n'),
    'CREATE INDEX `chart_blocklist_source_idx` ON `chart_blocklist` (`source_id`);',
]

const EXPECTED_INDEXES = ['PRIMARY unique sha256', 'chart_blocklist_md5_uniq unique md5', 'chart_blocklist_source_idx index source_id']

const readStatements = () =>
    readFileSync(join(DRIZZLE_DIR, `${MIGRATION_TAG}.sql`), 'utf8')
        .split(STATEMENT_BREAKPOINT)
        .map((statement) => statement.trim())
        .filter((statement) => statement.length > 0)

const rowsOf = (result: unknown) => (Array.isArray(result) && Array.isArray(result[0]) ? result[0] : [])

const isDuplicateEntry = async (pending: Promise<unknown>) => {
    try {
        await pending
        return false
    } catch (error) {
        return hasMysqlErrorCode(error, [MYSQL_ERROR_CODE.DUPLICATE_ENTRY])
    }
}

const row = (seed: string, overrides: Partial<typeof chartBlocklist.$inferInsert> = {}) => ({
    sha256: seed.repeat(64),
    md5: seed.repeat(32),
    sourceId: SOURCE_ID,
    sourceLabel: 'Migration 0015',
    path: `${seed}.bms`,
    ...overrides,
})

describe('0015 마이그레이션 파일', () => {
    test('저널의 마지막 두 항목이 0014, 0015 이다', () => {
        const journal: { entries: { tag: string }[] } = JSON.parse(readFileSync(join(DRIZZLE_DIR, 'meta', '_journal.json'), 'utf8'))
        expect(journal.entries.slice(-2).map((entry) => entry.tag)).toEqual([PREVIOUS_MIGRATION_TAG, MIGRATION_TAG])
    })

    test('chart_blocklist 표 하나와 출처 색인만 만든다', () => {
        expect(readStatements()).toEqual(EXPECTED_STATEMENTS)
    })
})

describe.skipIf(!hasDatabase)('0015 마이그레이션 (임시 MySQL)', () => {
    afterAll(async () => {
        await getDb().delete(chartBlocklist).where(eq(chartBlocklist.sourceId, SOURCE_ID))
    })

    test('적용된 표의 색인은 sha256 기본 키, md5 유일, 출처 색인이다', async () => {
        const indexes = rowsOf(
            await getDb().execute(
                sql`SELECT index_name AS name, non_unique AS nonUnique, GROUP_CONCAT(column_name ORDER BY seq_in_index) AS cols FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'chart_blocklist' GROUP BY index_name, non_unique ORDER BY index_name`,
            ),
        ).map((index) => `${index.name} ${Number(index.nonUnique) === 0 ? 'unique' : 'index'} ${index.cols}`)
        expect(indexes.toSorted()).toEqual(EXPECTED_INDEXES.toSorted())
    })

    test('같은 md5 를 다른 sha256 으로 다시 넣지 못한다', async () => {
        await getDb().insert(chartBlocklist).values(row('f'))
        expect(
            await isDuplicateEntry(
                getDb()
                    .insert(chartBlocklist)
                    .values(row('f', { sha256: 'e'.repeat(64) })),
            ),
        ).toBe(true)
    })
})

import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import mysql, { type RowDataPacket } from 'mysql2/promise'
import { z } from 'zod'
import { parseEnv, resolveDatabaseUrl } from '../src/server/lib/env'

const MIGRATIONS_DIR = join(import.meta.dirname, '..', 'drizzle')
const JOURNAL_PATH = join(MIGRATIONS_DIR, 'meta', '_journal.json')
const MIGRATIONS_TABLE = '__drizzle_migrations'
const HASH_PREVIEW_LENGTH = 8
const CONNECT_TIMEOUT_MS = 10_000
const BASELINE_FLAG = '--baseline-sql'
const CREATE_TABLE_PATTERN = /CREATE TABLE `([^`]+)`/g
const EXIT_CODE = { OK: 0, ACTION_NEEDED: 1, FAILED: 2 } as const

const journalSchema = z.object({
    entries: z.array(z.object({ idx: z.number().int(), when: z.number().int(), tag: z.string().min(1) })),
})

type Migration = { tag: string; when: number; hash: string; tables: string[] }

type AppliedRow = { hash: string; createdAt: number }

const preview = (hash: string) => hash.slice(0, HASH_PREVIEW_LENGTH)

const iso = (millis: number) => new Date(millis).toISOString()

const loadMigrations = async (): Promise<Migration[]> => {
    const journal = journalSchema.parse(JSON.parse(await readFile(JOURNAL_PATH, 'utf8')))
    return Promise.all(
        journal.entries.map(async (entry) => {
            const sql = await readFile(join(MIGRATIONS_DIR, `${entry.tag}.sql`), 'utf8')
            return {
                tag: entry.tag,
                when: entry.when,
                hash: createHash('sha256').update(sql).digest('hex'),
                tables: [...sql.matchAll(CREATE_TABLE_PATTERN)].map((match) => match[1]),
            }
        }),
    )
}

const printBaselineSql = (migrations: Migration[]) => {
    const [baseline] = migrations
    console.log(`-- Adopt "${baseline.tag}" for a database whose tables already exist (built with drizzle-kit push).`)
    console.log('-- Run it once, only when the status above says "tables exist, no migration table".')
    console.log(`CREATE TABLE IF NOT EXISTS \`${MIGRATIONS_TABLE}\` (id serial primary key, hash text not null, created_at bigint);`)
    console.log(`INSERT INTO \`${MIGRATIONS_TABLE}\` (\`hash\`, \`created_at\`) VALUES ('${baseline.hash}', ${baseline.when});`)
}

const run = async () => {
    const migrations = await loadMigrations()
    if (process.argv.includes(BASELINE_FLAG)) {
        printBaselineSql(migrations)
        return EXIT_CODE.OK
    }

    console.log('Migration files (drizzle/meta/_journal.json)')
    for (const migration of migrations) {
        console.log(`  ${migration.tag}  hash=${preview(migration.hash)}  when=${iso(migration.when)}  creates ${migration.tables.length} tables`)
    }

    const url = resolveDatabaseUrl(parseEnv(process.env))
    if (!url) {
        console.error('Set DATABASE_URL (or DB_HOST, DB_USER, DB_NAME) in the environment. Nothing was read.')
        return EXIT_CODE.FAILED
    }

    const connection = await mysql.createConnection({ uri: url, connectTimeout: CONNECT_TIMEOUT_MS })
    try {
        const [tableRows] = await connection.query<RowDataPacket[]>(
            'SELECT table_name AS name FROM information_schema.tables WHERE table_schema = DATABASE()',
        )
        const existing = new Set(tableRows.map((row) => String(row.name)))
        const hasMigrationTable = existing.has(MIGRATIONS_TABLE)

        const applied: AppliedRow[] = []
        if (hasMigrationTable) {
            const [rows] = await connection.query<RowDataPacket[]>(
                `SELECT hash, created_at AS createdAt FROM \`${MIGRATIONS_TABLE}\` ORDER BY created_at`,
            )
            applied.push(...rows.map((row) => ({ hash: String(row.hash), createdAt: Number(row.createdAt) })))
        }

        console.log(`\n${MIGRATIONS_TABLE}: ${hasMigrationTable ? `${applied.length} rows` : 'missing'}`)
        for (const row of applied) console.log(`  hash=${preview(row.hash)}  created_at=${row.createdAt} (${iso(row.createdAt)})`)

        const lastCreatedAt = applied.reduce((latest, row) => Math.max(latest, row.createdAt), Number.NEGATIVE_INFINITY)
        console.log('\nPer migration file (the migrator runs a file only when its "when" is newer than the latest row)')
        let pending = 0
        for (const migration of migrations) {
            const recorded = applied.some((row) => row.hash === migration.hash)
            const wouldRun = applied.length === 0 || lastCreatedAt < migration.when
            const present = migration.tables.filter((table) => existing.has(table))
            if (wouldRun) pending += 1
            console.log(
                `  ${migration.tag}  recorded=${recorded ? 'yes' : 'no'}  would_run=${wouldRun ? 'yes' : 'no'}  tables=${present.length}/${migration.tables.length}`,
            )
            const absent = migration.tables.filter((table) => !existing.has(table))
            if (absent.length > 0) console.log(`    missing tables: ${absent.join(', ')}`)
        }

        const baselineTables = migrations[0]?.tables ?? []
        const baselineComplete = baselineTables.every((table) => existing.has(table))
        console.log('')
        if (!hasMigrationTable && baselineComplete) {
            console.log(
                `Result: tables exist but ${MIGRATIONS_TABLE} is missing (built with push). Do not run db:migrate yet; run "bun scripts/db-migration-status.ts ${BASELINE_FLAG}", apply that SQL once, then run db:migrate.`,
            )
            return EXIT_CODE.ACTION_NEEDED
        }
        if (!hasMigrationTable && existing.size === 0) {
            console.log('Result: empty database. Run db:migrate to apply every file.')
            return EXIT_CODE.ACTION_NEEDED
        }
        if (!hasMigrationTable) {
            console.log(`Result: ${MIGRATIONS_TABLE} is missing and the baseline tables are incomplete. Stop and inspect the schema by hand.`)
            return EXIT_CODE.ACTION_NEEDED
        }
        if (pending > 0) {
            console.log(`Result: ${pending} migration file(s) pending. Run db:migrate before deploying the new web build.`)
            return EXIT_CODE.ACTION_NEEDED
        }
        console.log('Result: up to date.')
        return EXIT_CODE.OK
    } finally {
        await connection.end()
    }
}

run()
    .then((code) => {
        process.exitCode = code
    })
    .catch((error: unknown) => {
        const code = typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : 'UNKNOWN'
        console.error(`Failed: ${code}`)
        process.exitCode = EXIT_CODE.FAILED
    })

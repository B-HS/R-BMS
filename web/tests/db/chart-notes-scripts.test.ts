import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { eq, inArray, like } from 'drizzle-orm'
import mysql, { type RowDataPacket } from 'mysql2/promise'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, score } from '@server/db/schema'
import { getRequiredDatabaseUrl } from '@server/lib/env'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 차트 노트 수 운영 SQL 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const SCRIPTS_DIR = join(import.meta.dirname, '..', '..', 'scripts')
const BACKUP_TABLE = 'chart_notes_backup'
const PREFIX = 'cns'
const PLAYERS = ['first', 'second', 'third'] as const
const BASE_PLAYED_AT = 1_792_000_000_000
const POISONED_SHA = '5'.repeat(64)
const SPLIT_SHA = '6'.repeat(64)
const SCRATCH_SHA = '7'.repeat(64)
const CHART_SHAS = [POISONED_SHA, SPLIT_SHA, SCRATCH_SHA]
const POISONED_NOTES = 1
const REAL_NOTES = 1000
const SPLIT_NOTES = 300
const OUTVOTED_NOTES = 301
const SCRATCH_CHART_NOTES = 1500
const SCRATCH_AUTO_NOTES = 1400
const SCRATCH_PLAYED_AT_OFFSET = 100
const DIFFER_LABEL = 'chart rows whose notes differ from an accepted LN-mode total_notes (SCRATCH AUTO plays left out)'
const WOULD_CHANGE_LABEL = 'chart rows chart-notes.repair.sql would change (to the LN-mode total_notes the most users sent without SCRATCH AUTO)'
const BACKUP_LABEL = 'chart_notes_backup tables (1 = chart-notes.repair.sql has run, and it will not run again until that table is dropped)'

type Player = (typeof PLAYERS)[number]

const loginOf = (player: Player) => `${PREFIX}_${player}`

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

const rowsOf = (result: unknown): unknown[] => (Array.isArray(result) ? result : [])

const affectedRowsOf = (result: unknown) => (typeof result === 'object' && result !== null && 'affectedRows' in result ? result.affectedRows : null)

const countOf = (result: unknown, label: string) =>
    rowsOf(result)
        .map((row) =>
            typeof row === 'object' && row !== null && 'target' in row && 'row_count' in row && row.target === label ? row.row_count : null,
        )
        .find((count) => count !== null)

const dropBackup = () => withConnection((connection) => connection.query(`DROP TABLE IF EXISTS \`${BACKUP_TABLE}\``))

const notesOf = async (sha256: string) => {
    const [row] = await getDb().select({ notes: chart.notes }).from(chart).where(eq(chart.sha256, sha256))
    return row?.notes
}

describe.skipIf(!hasDatabase)('차트 노트 수 운영 SQL (임시 MySQL)', () => {
    const cleanUp = async () => {
        const db = getDb()
        await db.delete(score).where(inArray(score.chartSha256, CHART_SHAS))
        await db.delete(chart).where(inArray(chart.sha256, CHART_SHAS))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await dropBackup()
    }

    beforeAll(async () => {
        await cleanUp()
        const db = getDb()
        await db
            .insert(user)
            .values(
                PLAYERS.map((player) => ({ id: loginOf(player), name: player, email: `${loginOf(player)}@cns.invalid`, loginId: loginOf(player) })),
            )
        await db.insert(chart).values([
            { sha256: POISONED_SHA, notes: POISONED_NOTES },
            { sha256: SPLIT_SHA, notes: SPLIT_NOTES },
        ])
        const plays: { player: Player; sha256: string; notes: number }[] = [
            { player: 'first', sha256: POISONED_SHA, notes: REAL_NOTES },
            { player: 'second', sha256: POISONED_SHA, notes: REAL_NOTES },
            { player: 'third', sha256: POISONED_SHA, notes: POISONED_NOTES },
            { player: 'first', sha256: SPLIT_SHA, notes: SPLIT_NOTES },
            { player: 'second', sha256: SPLIT_SHA, notes: SPLIT_NOTES },
            { player: 'third', sha256: SPLIT_SHA, notes: OUTVOTED_NOTES },
        ]
        await db.insert(score).values(
            plays.map((play, index) => ({
                id: `${PREFIX}-${index}`,
                userId: loginOf(play.player),
                chartSha256: play.sha256,
                notes: play.notes,
                playedAt: BASE_PLAYED_AT + index,
            })),
        )
    })

    afterAll(cleanUp)

    test('SCRATCH AUTO 판의 줄어든 total_notes 는 다르다고도, 복구가 바꿀 행으로도 세지 않는다', async () => {
        const before = await runScript('chart-notes.count.sql')
        const db = getDb()
        await db.insert(chart).values({ sha256: SCRATCH_SHA, notes: SCRATCH_CHART_NOTES })
        const plays: { player: Player; notes: number; scratchAuto: boolean }[] = [
            { player: 'first', notes: SCRATCH_CHART_NOTES, scratchAuto: false },
            { player: 'second', notes: SCRATCH_AUTO_NOTES, scratchAuto: true },
            { player: 'third', notes: SCRATCH_AUTO_NOTES, scratchAuto: true },
        ]
        await db.insert(score).values(
            plays.map((play, index) => ({
                id: `${PREFIX}-scratch-${index}`,
                userId: loginOf(play.player),
                chartSha256: SCRATCH_SHA,
                notes: play.notes,
                scratchAuto: play.scratchAuto,
                playedAt: BASE_PLAYED_AT + SCRATCH_PLAYED_AT_OFFSET + index,
            })),
        )
        const after = await runScript('chart-notes.count.sql')
        expect(countOf(after, DIFFER_LABEL)).toBe(countOf(before, DIFFER_LABEL))
        expect(countOf(after, WOULD_CHANGE_LABEL)).toBe(countOf(before, WOULD_CHANGE_LABEL))
    })

    test('세기는 복구가 바꿀 행 수를 따로 보여 주고, 복구는 바꾼 행을 먼저 백업한다', async () => {
        const wouldChange = countOf(await runScript('chart-notes.count.sql'), WOULD_CHANGE_LABEL)
        expect(typeof wouldChange).toBe('number')

        const repaired = await runScript('chart-notes.repair.sql')
        const [, backedUp, updated] = rowsOf(repaired).map(affectedRowsOf)
        expect(backedUp).toBe(wouldChange)
        expect(updated).toBe(wouldChange)
        expect(await notesOf(POISONED_SHA)).toBe(REAL_NOTES)
        expect(await notesOf(SPLIT_SHA)).toBe(SPLIT_NOTES)
        expect(await notesOf(SCRATCH_SHA)).toBe(SCRATCH_CHART_NOTES)

        const [backup] = await withConnection((connection) =>
            connection.query<RowDataPacket[]>(`SELECT \`old_notes\`, \`new_notes\` FROM \`${BACKUP_TABLE}\` WHERE \`sha256\` = ?`, [POISONED_SHA]),
        )
        expect(backup.map((row) => [row.old_notes, row.new_notes])).toEqual([[POISONED_NOTES, REAL_NOTES]])
    })

    test('두 번째 복구는 백업을 덮어쓰지 않고 멈추며, 되돌리기는 백업의 옛 값으로 돌린다', async () => {
        await expect(runScript('chart-notes.repair.sql')).rejects.toThrow()
        await runScript('chart-notes.restore.sql')
        expect(await notesOf(POISONED_SHA)).toBe(POISONED_NOTES)
        expect(await notesOf(SPLIT_SHA)).toBe(SPLIT_NOTES)
    })

    test('백업을 지우면 세기가 0 으로 보이고 복구를 다시 돌릴 수 있다', async () => {
        await runScript('chart-notes.drop-backup.sql')
        expect(countOf(await runScript('chart-notes.count.sql'), BACKUP_LABEL)).toBe(0)
        await runScript('chart-notes.repair.sql')
        expect(await notesOf(POISONED_SHA)).toBe(REAL_NOTES)
        expect(await notesOf(SCRATCH_SHA)).toBe(SCRATCH_CHART_NOTES)
    })
})

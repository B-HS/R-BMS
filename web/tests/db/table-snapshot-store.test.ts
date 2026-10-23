import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { eq, like } from 'drizzle-orm'
import { createTableSnapshotStore } from '@server/compose/table-refresh'
import { getDb } from '@server/db'
import { tableSnapshot } from '@server/db/schema'
import { DAY_MS, FIRST_NOW, STELLA, remoteEntries, rowOf, stellaHeader } from '../support/remote-table-fixture'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 표 스냅샷 저장소 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'tss'
const BIG_TABLE_ENTRIES = 5000
const ONE_MIB = 1024 * 1024
const idOf = (name: string) => `${PREFIX}_${name}`

const rowFor = (name: string, entries: unknown, stamps?: Parameters<typeof rowOf>[3]) => ({
    ...rowOf(STELLA, stellaHeader(), entries, stamps),
    tableId: idOf(name),
})

describe.skipIf(!hasDatabase)('표 스냅샷 저장소 (임시 MySQL)', () => {
    const store = createTableSnapshotStore(getDb())

    const clean = async () => {
        await getDb()
            .delete(tableSnapshot)
            .where(like(tableSnapshot.tableId, `${PREFIX}_%`))
    }

    beforeAll(clean)
    afterAll(clean)

    test('약 1 MB data 를 압축해 저장하고 같은 바이트로 읽는다', async () => {
        const big = rowFor('big', remoteEntries(BIG_TABLE_ENTRIES))
        expect(big.dataBytes.byteLength).toBeGreaterThan(ONE_MIB * 0.9)
        expect(await store.saveSnapshot(big, null)).toBe(true)

        const loaded = await store.loadSnapshot(big.tableId)
        expect(loaded).not.toBeNull()
        expect(Buffer.from(loaded?.dataBytes ?? []).equals(Buffer.from(big.dataBytes))).toBe(true)
        expect(Buffer.from(loaded?.headerBytes ?? []).equals(Buffer.from(big.headerBytes))).toBe(true)
        expect(loaded).toMatchObject({
            tableId: big.tableId,
            headerUrl: big.headerUrl,
            dataUrl: big.dataUrl,
            headerSha256: big.headerSha256,
            dataSha256: big.dataSha256,
            fetchedAt: FIRST_NOW,
            checkedAt: FIRST_NOW,
            lastOutcome: 'saved',
        })

        const [raw] = await getDb().select({ dataGz: tableSnapshot.dataGz }).from(tableSnapshot).where(eq(tableSnapshot.tableId, big.tableId))
        expect(raw.dataGz.byteLength).toBeLessThan(big.dataBytes.byteLength)
    })

    test('첫 저장이 겹치면 두 번째는 덮지 않고 false 를 돌려준다', async () => {
        const first = rowFor('race', remoteEntries(3))
        const second = rowFor('race', remoteEntries(4))
        expect(await store.saveSnapshot(first, null)).toBe(true)
        expect(await store.saveSnapshot(second, null)).toBe(false)
        expect((await store.loadSnapshot(first.tableId))?.dataSha256).toBe(first.dataSha256)
    })

    test('previous 가 다르면 update 영향이 0 이라 false 이고 내용은 그대로이다', async () => {
        const current = rowFor('cas', remoteEntries(3))
        const next = rowFor('cas', remoteEntries(5), { fetchedAt: FIRST_NOW + DAY_MS })
        await store.saveSnapshot(current, null)

        expect(await store.saveSnapshot(next, next.dataSha256)).toBe(false)
        expect((await store.loadSnapshot(current.tableId))?.dataSha256).toBe(current.dataSha256)

        expect(await store.saveSnapshot(next, current.dataSha256)).toBe(true)
        expect(await store.loadSnapshot(next.tableId)).toMatchObject({ dataSha256: next.dataSha256, fetchedAt: FIRST_NOW + DAY_MS })
    })

    test('markChecked 는 내용 열을 바꾸지 않고 확인 시각과 결과만 바꾼다', async () => {
        const stored = rowFor('mark', remoteEntries(3))
        await store.saveSnapshot(stored, null)

        await store.markChecked({ tableId: stored.tableId, checkedAt: FIRST_NOW + DAY_MS, outcome: 'rejected:shrank' })

        const loaded = await store.loadSnapshot(stored.tableId)
        expect(loaded).toMatchObject({ checkedAt: FIRST_NOW + DAY_MS, lastOutcome: 'rejected:shrank', fetchedAt: FIRST_NOW })
        expect(loaded?.dataSha256).toBe(stored.dataSha256)
        expect(Buffer.from(loaded?.dataBytes ?? []).equals(Buffer.from(stored.dataBytes))).toBe(true)
    })

    test('listVersions 는 내용 없이 해시와 시각만 돌려주고, 없는 표는 null 이다', async () => {
        const stored = rowFor('list', remoteEntries(2))
        await store.saveSnapshot(stored, null)
        const version = (await store.listVersions()).find((candidate) => candidate.tableId === stored.tableId)
        expect(version).toEqual({
            tableId: stored.tableId,
            headerSha256: stored.headerSha256,
            dataSha256: stored.dataSha256,
            fetchedAt: FIRST_NOW,
            checkedAt: FIRST_NOW,
            lastOutcome: 'saved',
        })
        expect(await store.loadSnapshot(idOf('missing'))).toBeNull()
    })
})

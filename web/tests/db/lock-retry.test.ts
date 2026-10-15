import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { eq, inArray, sql } from 'drizzle-orm'
import { z } from 'zod'
import { getDb, LOCK_WAIT_TIMEOUT_SECONDS } from '@server/db'
import { user } from '@server/db/auth-schema'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import { withLockRetry } from '@server/lib/with-lock-retry'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 교착 재시도 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 's19l'
const ROW_A = `${PREFIX}_a`
const ROW_B = `${PREFIX}_b`

const lockOneThenOther = (first: string, second: string, held: () => void, otherHeld: Promise<void>, attempts: string[]) => async () =>
    getDb().transaction(async (tx) => {
        attempts.push(first)
        await tx.select({ id: user.id }).from(user).where(eq(user.id, first)).for('update')
        held()
        await otherHeld
        await tx.select({ id: user.id }).from(user).where(eq(user.id, second)).for('update')
        await tx
            .update(user)
            .set({ name: `${first}-won` })
            .where(eq(user.id, first))
    })

const crossedPair = (wrap: (write: () => Promise<void>) => () => Promise<void>) => {
    const attempts: string[] = []
    const aHeld = Promise.withResolvers<void>()
    const bHeld = Promise.withResolvers<void>()
    const writeA = wrap(lockOneThenOther(ROW_A, ROW_B, aHeld.resolve, bHeld.promise, attempts))
    const writeB = wrap(lockOneThenOther(ROW_B, ROW_A, bHeld.resolve, aHeld.promise, attempts))
    return { attempts, settled: Promise.allSettled([writeA(), writeB()]) }
}

describe.skipIf(!hasDatabase)('교착 재시도 (임시 MySQL)', () => {
    beforeAll(async () => {
        const db = getDb()
        await db.delete(user).where(inArray(user.id, [ROW_A, ROW_B]))
        await db.insert(user).values([ROW_A, ROW_B].map((id) => ({ id, name: id, email: `${id}@s19.invalid`, loginId: id })))
    })

    afterAll(async () => {
        await getDb()
            .delete(user)
            .where(inArray(user.id, [ROW_A, ROW_B]))
    })

    test('서로 엇갈려 잠그는 두 트랜잭션은 그대로 두면 한쪽이 교착 오류로 진다', async () => {
        const { settled } = crossedPair((write) => write)
        const results = await settled
        const rejected = results.flatMap((result) => (result.status === 'rejected' ? [result.reason] : []))
        expect(rejected).toHaveLength(1)
        expect(hasMysqlErrorCode(rejected[0], [MYSQL_ERROR_CODE.LOCK_DEADLOCK])).toBe(true)
    })

    test('withLockRetry 로 감싸면 진 쪽이 처음부터 다시 해서 둘 다 끝난다', async () => {
        const { attempts, settled } = crossedPair(withLockRetry)
        const results = await settled
        expect(results.map((result) => result.status)).toEqual(['fulfilled', 'fulfilled'])
        expect(attempts).toHaveLength(3)
        const rows = await getDb()
            .select({ id: user.id, name: user.name })
            .from(user)
            .where(inArray(user.id, [ROW_A, ROW_B]))
        expect(rows.map((row) => row.name).toSorted()).toEqual([`${ROW_A}-won`, `${ROW_B}-won`])
    })

    test('풀의 연결은 잠금 대기 상한을 LOCK_WAIT_TIMEOUT_SECONDS 로 둔다 (문장 하나, 트랜잭션 모두)', async () => {
        const readTimeout = sql`SELECT @@SESSION.innodb_lock_wait_timeout AS seconds`
        const timeoutRowsSchema = z.array(z.object({ seconds: z.coerce.number() })).length(1)
        const [statementRows] = await getDb().execute(readTimeout)
        const [transactionRows] = await getDb().transaction(async (tx) => tx.execute(readTimeout))
        const seconds = [statementRows, transactionRows].map((rows) => timeoutRowsSchema.parse(rows)[0].seconds)
        expect(seconds).toEqual([LOCK_WAIT_TIMEOUT_SECONDS, LOCK_WAIT_TIMEOUT_SECONDS])
    })
})

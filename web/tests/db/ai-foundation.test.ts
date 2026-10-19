import { afterAll, beforeAll, describe, expect, spyOn, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { and, eq, inArray, like, sql } from 'drizzle-orm'
import { createAiDbs } from '@server/compose/ai'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { aiCredential, aiInflight, aiResult, aiSetting, aiUsage, aiUserSetting } from '@server/db/schema'
import { AI_RUN_HOLD_MS } from '@server/service/domain/ai/usage.service'
import { FAKE_USAGE, createAiServices, createFakeAdapter, testKekEnv } from '../support/ai-memory-db'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] AI 기반 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'w1ai'
const OWNER = `${PREFIX}_owner`
const OTHER = `${PREFIX}_other`
const USERS = [OWNER, OTHER]
const API_KEY = 'sk-ant-api03-db-secret-QRST'
const ADMIN_SEED_EMAIL = 'hs@gumyo.net'
const ADMIN_SEED_USER = `${PREFIX}_seed`
const ADMIN_SEED_SQL = readFileSync(join(import.meta.dirname, '..', '..', 'drizzle', '0006_admin_seed.sql'), 'utf8')
const ADMIN_SEED_COUNT_SQL = readFileSync(join(import.meta.dirname, '..', '..', 'scripts', 'admin-seed.count.sql'), 'utf8')
const KEY_HINT_COLUMN_LENGTH = 8
const PARALLEL_RESERVES = 8
const NOON = Date.UTC(2026, 9, 19, 12)
const NEW_TABLES = ['ai_credential', 'ai_inflight', 'ai_result', 'ai_setting', 'ai_usage', 'ai_user_setting']

const servicesFor = (env: () => Record<string, string>, now = () => NOON) => {
    const anthropic = createFakeAdapter('anthropic', { validKeys: new Set([API_KEY]) })
    return { anthropic, ...createAiServices(createAiDbs(getDb()), { adapters: { anthropic: anthropic.adapter }, env, now }) }
}

const catchError = async (pending: Promise<unknown>) => {
    try {
        await pending
        return null
    } catch (error) {
        return error
    }
}

const countRows = async () => {
    const [rows] = await getDb().execute(sql.raw(ADMIN_SEED_COUNT_SQL))
    return Object.fromEntries((Array.isArray(rows) ? rows : []).map((row) => [String(row.target), Number(row.row_count)]))
}

const affectedRows = (result: unknown) =>
    Array.isArray(result) && typeof result[0] === 'object' && result[0] !== null && 'affectedRows' in result[0] ? result[0].affectedRows : null

describe.skipIf(!hasDatabase)('AI 기반 표 (임시 MySQL)', () => {
    beforeAll(async () => {
        const db = getDb()
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await db.delete(aiSetting)
        await db.insert(user).values(USERS.map((id) => ({ id, name: id, email: `${id}@w1ai.invalid`, loginId: id })))
    })

    afterAll(async () => {
        const db = getDb()
        await db.delete(aiSetting)
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    })

    test('마이그레이션이 새 표 여섯 개와 chart_best 의 사용자 색인을 만든다', async () => {
        const [tables] = await getDb().execute(
            sql`SELECT table_name AS name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name LIKE 'ai\\_%'`,
        )
        expect(Array.isArray(tables) ? tables.map((row) => String(Object.values(row)[0])).toSorted() : []).toEqual(expect.arrayContaining(NEW_TABLES))
        const [indexes] = await getDb().execute(
            sql`SELECT column_name AS name FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'chart_best' AND index_name = 'chart_best_user_idx' ORDER BY seq_in_index`,
        )
        expect(Array.isArray(indexes) ? indexes.map((row) => String(Object.values(row)[0])) : []).toEqual(['user_id', 'scratch_auto'])
    })

    describe('키 저장', () => {
        test('등록하면 암호문만 저장되고 다시 등록하면 한 행이 교체되며 모델은 비워진다', async () => {
            const services = servicesFor(() => testKekEnv('v1'))
            expect((await services.aiCredentialService.register({ userId: OWNER, provider: 'anthropic', apiKey: API_KEY })).ok).toBe(true)
            expect(await services.aiCredentialService.setModel({ userId: OWNER, provider: 'anthropic', model: 'claude-test' })).toMatchObject({
                model: 'claude-test',
            })
            expect((await services.aiCredentialService.register({ userId: OWNER, provider: 'anthropic', apiKey: API_KEY })).ok).toBe(true)
            const rows = await getDb().select().from(aiCredential).where(eq(aiCredential.userId, OWNER))
            expect(rows).toHaveLength(1)
            expect(rows[0]).toMatchObject({ provider: 'anthropic', keyHint: 'QRST', model: null, kekVersion: 'v1', lastErrorKind: null })
            expect(JSON.stringify(rows)).not.toContain(API_KEY)
        })

        test('KEK 회전 뒤 처음 쓸 때 행의 kek_version 이 새 버전으로 바뀌고 계속 열린다', async () => {
            const rotated = servicesFor(() => testKekEnv('v2', ['v1', 'v2']))
            expect((await rotated.aiCredentialService.listModels({ userId: OWNER, provider: 'anthropic' })).ok).toBe(true)
            const [row] = await getDb().select().from(aiCredential).where(eq(aiCredential.userId, OWNER))
            expect(row.kekVersion).toBe('v2')
            const onlyNew = servicesFor(() => testKekEnv('v2'))
            expect((await onlyNew.aiCredentialService.revalidate({ userId: OWNER, provider: 'anthropic' })).ok).toBe(true)
            expect(onlyNew.anthropic.seenKeys).toEqual([API_KEY])
        })

        test('재포장은 읽었을 때의 감싼 DEK 가 그대로인 행만 바꾼다', async () => {
            const db = getDb()
            const [row] = await db.select().from(aiCredential).where(eq(aiCredential.userId, OWNER))
            const sealed = { ...row, wrappedDek: 'A'.repeat(row.wrappedDek.length), kekVersion: 'v9' }
            const credentialDb = createAiDbs(db).credentialDb
            await credentialDb.rewrapCredential({
                userId: OWNER,
                provider: 'anthropic',
                previous: { kekVersion: row.kekVersion, wrappedDek: `${row.wrappedDek.slice(0, -1)}x`, dekIv: row.dekIv },
                sealed,
            })
            const [unchanged] = await db.select().from(aiCredential).where(eq(aiCredential.userId, OWNER))
            expect([unchanged.kekVersion, unchanged.wrappedDek]).toEqual([row.kekVersion, row.wrappedDek])
        })

        test('키 쓰기의 DB 오류는 암호문·감싼 DEK 없는 오류로 바뀌고 로그에도 남지 않는다', async () => {
            const errors = spyOn(console, 'error').mockImplementation(() => undefined)
            try {
                const [row] = await getDb().select().from(aiCredential).where(eq(aiCredential.userId, OWNER))
                const write = createAiDbs(getDb()).credentialDb.upsertCredential({
                    userId: OWNER,
                    provider: 'anthropic',
                    sealed: {
                        ciphertext: row.ciphertext,
                        iv: row.iv,
                        authTag: row.authTag,
                        wrappedDek: row.wrappedDek,
                        dekIv: row.dekIv,
                        dekTag: row.dekTag,
                        kekVersion: row.kekVersion,
                    },
                    keyHint: 'k'.repeat(KEY_HINT_COLUMN_LENGTH + 1),
                    validatedAt: NOON,
                    errorKind: null,
                    updatedAt: NOON,
                })
                const caught = await catchError(write)
                expect(caught).toMatchObject({ code: 'INTERNAL_ERROR', details: { reason: 'ai_credential_write_failed' } })
                const surfaced = JSON.stringify([caught, errors.mock.calls])
                for (const secret of [row.ciphertext, row.wrappedDek, row.dekTag]) expect(surfaced).not.toContain(secret)
                expect(JSON.stringify(errors.mock.calls)).toContain('ER_DATA_TOO_LONG')
            } finally {
                errors.mockRestore()
            }
        })

        test('다른 사용자 행으로 옮긴 암호문은 열리지 않는다', async () => {
            const db = getDb()
            const [row] = await db.select().from(aiCredential).where(eq(aiCredential.userId, OWNER))
            await db.insert(aiCredential).values({ ...row, userId: OTHER })
            const services = servicesFor(() => testKekEnv('v2'))
            expect(await services.aiCredentialService.listModels({ userId: OTHER, provider: 'anthropic' })).toEqual({
                ok: false,
                reason: 'credential_undecryptable',
                cause: 'tampered',
            })
            expect(await services.aiCredentialService.remove({ userId: OTHER, provider: 'anthropic' })).toBe(true)
            expect(await services.aiCredentialService.remove({ userId: OTHER, provider: 'anthropic' })).toBe(false)
        })
    })

    describe('사용량과 동시 실행', () => {
        test('동시에 여러 번 잡아도 동시 실행 1건이면 정확히 하나만 잡힌다', async () => {
            const services = servicesFor(() => testKekEnv('v1'))
            const limits = { dailyCalls: 30, concurrentRuns: 1, monthlyTokenLimit: null }
            const results = await Promise.all(
                Array.from({ length: PARALLEL_RESERVES }, () => services.aiUsageService.reserve({ userId: OWNER, limits })),
            )
            expect(results.filter((result) => result.ok)).toHaveLength(1)
            expect(results.filter((result) => !result.ok && result.limit === 'concurrent_runs')).toHaveLength(PARALLEL_RESERVES - 1)
            for (const result of results) if (result.ok) await services.aiUsageService.release(result.hold)
            expect(await getDb().select().from(aiInflight).where(eq(aiInflight.userId, OWNER))).toHaveLength(0)
        })

        test('동시에 여러 번 잡아도 하루 호출 수는 한도를 넘지 않는다', async () => {
            const services = servicesFor(() => testKekEnv('v1'))
            await getDb().delete(aiUsage).where(eq(aiUsage.userId, OTHER))
            const limits = { dailyCalls: 3, concurrentRuns: PARALLEL_RESERVES, monthlyTokenLimit: null }
            const results = await Promise.all(
                Array.from({ length: PARALLEL_RESERVES }, () => services.aiUsageService.reserve({ userId: OTHER, limits })),
            )
            expect(results.filter((result) => result.ok)).toHaveLength(limits.dailyCalls)
            for (const result of results) if (result.ok) await services.aiUsageService.release(result.hold)
            const [row] = await getDb()
                .select()
                .from(aiUsage)
                .where(and(eq(aiUsage.userId, OTHER), eq(aiUsage.day, '2026-10-19')))
            expect(row.calls).toBe(limits.dailyCalls)
            expect(await getDb().select().from(aiInflight).where(eq(aiInflight.userId, OTHER))).toHaveLength(0)
        })

        test('토큰은 같은 날 행에 더해지고 월 합계로 읽힌다', async () => {
            const services = servicesFor(() => testKekEnv('v1'))
            const limits = { dailyCalls: 30, concurrentRuns: 1, monthlyTokenLimit: null }
            const reserved = await services.aiUsageService.reserve({ userId: OWNER, limits })
            expect(reserved.ok).toBe(true)
            if (!reserved.ok) return
            await services.aiUsageService.record({ hold: reserved.hold, usage: FAKE_USAGE })
            await services.aiUsageService.record({ hold: reserved.hold, usage: FAKE_USAGE })
            await services.aiUsageService.release(reserved.hold)
            const summary = await services.aiUsageService.summary({ userId: OWNER, limits })
            expect(summary.this_month).toMatchObject({ input_tokens: 240, output_tokens: 80, cached_input_tokens: 40 })
        })

        test('풀리지 않은 자리는 보유 시간이 지나면 다른 실행이 가져간다', async () => {
            const clock = { now: NOON }
            const services = servicesFor(
                () => testKekEnv('v1'),
                () => clock.now,
            )
            const limits = { dailyCalls: 30, concurrentRuns: 1, monthlyTokenLimit: null }
            const stale = await services.aiUsageService.reserve({ userId: OWNER, limits })
            expect(stale.ok).toBe(true)
            expect((await services.aiUsageService.reserve({ userId: OWNER, limits })).ok).toBe(false)
            clock.now += AI_RUN_HOLD_MS
            const fresh = await services.aiUsageService.reserve({ userId: OWNER, limits })
            expect(fresh.ok).toBe(true)
            if (stale.ok) await services.aiUsageService.release(stale.hold)
            expect(await getDb().select().from(aiInflight).where(eq(aiInflight.userId, OWNER))).toHaveLength(1)
            if (fresh.ok) await services.aiUsageService.release(fresh.hold)
        })
    })

    describe('결과 캐시와 설정', () => {
        test('같은 입력 해시는 한 행이고 가장 최근 결과를 찾을 수 있다', async () => {
            const services = servicesFor(() => testKekEnv('v1'))
            const base = {
                userId: OWNER,
                feature: 'query',
                subjectKey: 'q:1',
                analysisVersion: 1,
                promptVersion: 1,
                provider: 'anthropic' as const,
                model: 'm',
            }
            await services.aiResultService.save({ ...base, inputHash: 'a'.repeat(64), output: { n: 1 }, usage: FAKE_USAGE })
            await services.aiResultService.save({ ...base, inputHash: 'a'.repeat(64), output: { n: 2 }, usage: FAKE_USAGE })
            const rows = await getDb().select().from(aiResult).where(eq(aiResult.userId, OWNER))
            expect(rows).toHaveLength(1)
            expect(await services.aiResultService.findCached({ ...base, inputHash: 'a'.repeat(64) })).toMatchObject({
                output: { n: 2 },
                usage: FAKE_USAGE,
            })
            expect(await services.aiResultService.findCached({ ...base, inputHash: 'b'.repeat(64) })).toBeNull()
            expect(await services.aiResultService.findLatest(base)).toMatchObject({ inputHash: 'a'.repeat(64) })
        })

        test('운영자 한도와 사용자 월 상한을 저장하고 다시 읽는다', async () => {
            const services = servicesFor(() => testKekEnv('v1'))
            expect(await services.aiSettingsService.resolveLimits(OWNER)).toEqual({ dailyCalls: 30, concurrentRuns: 1, monthlyTokenLimit: null })
            await services.aiSettingsService.putOperatorLimits({
                userId: OWNER,
                input: { daily_calls: 40, concurrent_runs: 2, default_monthly_token_limit: 1000 },
            })
            await services.aiSettingsService.putOperatorLimits({
                userId: OWNER,
                input: { daily_calls: 45, concurrent_runs: 2, default_monthly_token_limit: 1000 },
            })
            await services.aiSettingsService.putUserSetting({ userId: OWNER, input: { monthly_token_limit: 700, default_provider: 'openai' } })
            expect(await getDb().select().from(aiSetting)).toHaveLength(1)
            expect(await services.aiSettingsService.resolveLimits(OWNER)).toEqual({ dailyCalls: 45, concurrentRuns: 2, monthlyTokenLimit: 700 })
            expect(await services.aiSettingsService.resolveLimits(OTHER)).toEqual({ dailyCalls: 45, concurrentRuns: 2, monthlyTokenLimit: 1000 })
            expect(await services.aiSettingsService.getUserSetting(OWNER)).toMatchObject({ default_provider: 'openai' })
        })
    })

    test('사용자를 지우면 그 사용자의 AI 행이 모두 함께 지워진다', async () => {
        const db = getDb()
        await db.delete(user).where(eq(user.id, OWNER))
        for (const table of [aiCredential, aiUsage, aiInflight, aiResult, aiUserSetting]) {
            expect(await db.select().from(table).where(eq(table.userId, OWNER))).toHaveLength(0)
        }
    })

    test('관리자 지정 마이그레이션은 그 이메일 한 행만 admin 으로 바꾸고 다시 돌려도 바꾸지 않는다', async () => {
        const db = getDb()
        const [existing] = await db.select({ id: user.id }).from(user).where(eq(user.email, ADMIN_SEED_EMAIL))
        expect(existing).toBeUndefined()
        const adminsBefore = (await countRows()).admins
        await db.insert(user).values({ id: ADMIN_SEED_USER, name: 'seed', email: ADMIN_SEED_EMAIL, loginId: ADMIN_SEED_USER })
        expect(await countRows()).toEqual({ user_with_email: 1, user_to_change: 1, admins: adminsBefore })
        expect(affectedRows(await db.execute(sql.raw(ADMIN_SEED_SQL)))).toBe(1)
        expect(affectedRows(await db.execute(sql.raw(ADMIN_SEED_SQL)))).toBe(0)
        expect(await countRows()).toEqual({ user_with_email: 1, user_to_change: 0, admins: adminsBefore + 1 })
        const roles = await db
            .select({ id: user.id, role: user.role })
            .from(user)
            .where(inArray(user.id, [ADMIN_SEED_USER, OTHER]))
        expect(roles.toSorted((left, right) => left.id.localeCompare(right.id))).toEqual([
            { id: OTHER, role: 'user' },
            { id: ADMIN_SEED_USER, role: 'admin' },
        ])
    })
})

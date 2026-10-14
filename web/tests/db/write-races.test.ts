import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { and, eq, inArray, like } from 'drizzle-orm'
import { compose } from '@server/compose'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { apiToken, replay, settingBlob, userFolder, userFolderItem } from '@server/db/schema'
import { FOLDER_LIMIT, folderCreateSchema, folderItemInputSchema, folderItemsReplaceSchema } from '@server/dto/folder'
import { replayUploadSchema } from '@server/dto/replay'
import { SETTING_NAMES_PER_USER_MAX, settingPutSchema } from '@server/dto/setting'
import { hashApiToken } from '@server/lib/token'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 쓰기 경합 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 's19w'
const OWNER = `${PREFIX}_owner`
const OTHER = `${PREFIX}_other`
const USERS = [OWNER, OTHER]
const CHART_SHA256 = 'c'.repeat(64)
const CONCURRENT_WRITES = 12
const REPLAY_EVENTS = [{ t_us: 1_000, lane: 0, press: true }]

const md5Of = (n: number) => n.toString(16).padStart(32, '0')
const itemOf = (n: number) => folderItemInputSchema.parse({ md5: md5Of(n), title: `title-${n}`, artist: 'artist' })

const createFolder = async (userId: string, name: string) => {
    const result = await compose().folderService.create({ userId, input: folderCreateSchema.parse({ name }) })
    if (!result.ok) throw new Error(`create failed: ${result.reason}`)
    return result.value
}

const storedItems = async (folderId: string) => getDb().select().from(userFolderItem).where(eq(userFolderItem.folderId, folderId))

describe.skipIf(!hasDatabase)('쓰기 경합 (임시 MySQL)', () => {
    beforeAll(async () => {
        const db = getDb()
        await db.delete(userFolder).where(inArray(userFolder.userId, USERS))
        await db.delete(settingBlob).where(inArray(settingBlob.userId, USERS))
        await db.delete(replay).where(inArray(replay.userId, USERS))
        await db.delete(apiToken).where(inArray(apiToken.userId, USERS))
        await db.delete(user).where(inArray(user.id, USERS))
        await db.insert(user).values(USERS.map((id) => ({ id, name: id, email: `${id}@s19.invalid`, loginId: id })))
    })

    afterAll(async () => {
        await getDb()
            .delete(user)
            .where(like(user.id, `${PREFIX}_%`))
    })

    describe('설정 낙관적 잠금', () => {
        const put = (name: string, content: string, base?: number) =>
            compose().settingService.put({
                userId: OWNER,
                name,
                input: settingPutSchema.parse({ content, ...(base === undefined ? {} : { base_updated_at: base }) }),
            })

        test('같은 base 로 동시에 올린 쓰기 중 정확히 하나만 저장되고 나머지는 충돌과 서버 최신본을 받는다', async () => {
            const first = await put('race-base', 'v1')
            if (first.status !== 'stored') throw new Error('first write must store')
            const results = await Promise.all(
                Array.from({ length: CONCURRENT_WRITES }, (_, index) => put('race-base', `from-${index}`, first.updated_at)),
            )
            const stored = results.filter((result) => result.status === 'stored')
            expect(stored).toHaveLength(1)
            expect(results.filter((result) => result.status === 'conflict')).toHaveLength(CONCURRENT_WRITES - 1)
            const [row] = await getDb()
                .select()
                .from(settingBlob)
                .where(and(eq(settingBlob.userId, OWNER), eq(settingBlob.name, 'race-base')))
            expect(stored[0]).toMatchObject({ updated_at: row.updatedAt })
            for (const result of results) {
                if (result.status === 'conflict') expect(result.server).toMatchObject({ content: row.content, updated_at: row.updatedAt })
            }
            expect(row.updatedAt).toBeGreaterThan(first.updated_at)
        })

        test('없던 이름을 base 0 으로 동시에 만들면 한쪽만 만들어진다', async () => {
            const results = await Promise.all([put('race-create', 'a', 0), put('race-create', 'b', 0), put('race-create', 'c', 0)])
            expect(results.filter((result) => result.status === 'stored')).toHaveLength(1)
            expect(results.filter((result) => result.status === 'conflict')).toHaveLength(2)
        })

        test('base 없는 쓰기는 마지막 쓰기가 이기고 같은 이름은 한 행이다', async () => {
            await put('last-wins', 'one')
            await put('last-wins', 'two')
            const rows = await getDb()
                .select()
                .from(settingBlob)
                .where(and(eq(settingBlob.userId, OWNER), eq(settingBlob.name, 'last-wins')))
            expect(rows).toHaveLength(1)
            expect(rows[0].content).toBe('two')
        })

        test('사용자별 이름 한도에 이르면 새 이름은 거절하고 있던 이름은 고친다', async () => {
            const names = await getDb().select({ name: settingBlob.name }).from(settingBlob).where(eq(settingBlob.userId, OTHER))
            expect(names).toHaveLength(0)
            for (let index = 0; index < SETTING_NAMES_PER_USER_MAX; index += 1) {
                const result = await compose().settingService.put({
                    userId: OTHER,
                    name: `n${index}`,
                    input: settingPutSchema.parse({ content: '()' }),
                })
                expect(result.status).toBe('stored')
            }
            const overflow = await compose().settingService.put({ userId: OTHER, name: 'overflow', input: settingPutSchema.parse({ content: '()' }) })
            expect(overflow).toEqual({ status: 'too_many_names', limit: SETTING_NAMES_PER_USER_MAX })
            const edited = await compose().settingService.put({ userId: OTHER, name: 'n0', input: settingPutSchema.parse({ content: '(1)' }) })
            expect(edited.status).toBe('stored')
        })
    })

    describe('폴더 동시 쓰기', () => {
        test('같은 곡을 동시에 여러 번 넣어도 오류 없이 한 번만 들어가고 개수가 맞다', async () => {
            const folder = await createFolder(OWNER, 'concurrent-same')
            const results = await Promise.all(
                Array.from({ length: CONCURRENT_WRITES }, () =>
                    compose().folderService.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] }),
                ),
            )
            expect(results.every((result) => result.ok)).toBe(true)
            const added = results.flatMap((result) => (result.ok ? [result.value.added] : []))
            expect(added.reduce((sum, count) => sum + count, 0)).toBe(1)
            expect(await storedItems(folder.id)).toHaveLength(1)
            const [row] = await getDb().select().from(userFolder).where(eq(userFolder.id, folder.id))
            expect(row.itemCount).toBe(1)
        })

        test('서로 다른 곡을 동시에 넣으면 모두 들어가고 position 이 겹치지 않는다', async () => {
            const folder = await createFolder(OWNER, 'concurrent-distinct')
            const results = await Promise.all(
                Array.from({ length: CONCURRENT_WRITES }, (_, index) =>
                    compose().folderService.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(100 + index)] }),
                ),
            )
            expect(results.every((result) => result.ok)).toBe(true)
            const rows = await storedItems(folder.id)
            expect(rows).toHaveLength(CONCURRENT_WRITES)
            expect(new Set(rows.map((row) => row.position)).size).toBe(CONCURRENT_WRITES)
            const stamps = results.flatMap((result) => (result.ok ? [result.value.updated_at] : []))
            expect(new Set(stamps).size).toBe(CONCURRENT_WRITES)
            const [row] = await getDb().select().from(userFolder).where(eq(userFolder.id, folder.id))
            expect(row.itemCount).toBe(CONCURRENT_WRITES)
            expect(row.updatedAt).toBe(Math.max(...stamps))
        })

        test('폴더 한도에 가까운 곳에 동시에 들어온 요청들이 합쳐서 한도를 넘지 못한다', async () => {
            const folder = await createFolder(OWNER, 'concurrent-limit')
            const fillTo = FOLDER_LIMIT.ITEMS_PER_FOLDER - 3
            for (let start = 0; start < fillTo; start += FOLDER_LIMIT.ITEMS_PER_REQUEST) {
                const items = Array.from({ length: Math.min(FOLDER_LIMIT.ITEMS_PER_REQUEST, fillTo - start) }, (_, index) =>
                    itemOf(10_000 + start + index),
                )
                const filled = await compose().folderService.addItems({ userId: OWNER, folderId: folder.id, items })
                expect(filled.ok).toBe(true)
            }
            const results = await Promise.all(
                Array.from({ length: 6 }, (_, index) =>
                    compose().folderService.addItems({
                        userId: OWNER,
                        folderId: folder.id,
                        items: [itemOf(900_000 + index * 2), itemOf(900_001 + index * 2)],
                    }),
                ),
            )
            const failed = results.filter((result) => !result.ok)
            expect(failed.length).toBeGreaterThan(0)
            expect(failed.every((result) => !result.ok && result.reason === 'limit_exceeded')).toBe(true)
            const [row] = await getDb().select().from(userFolder).where(eq(userFolder.id, folder.id))
            expect(row.itemCount).toBeLessThanOrEqual(FOLDER_LIMIT.ITEMS_PER_FOLDER)
            expect((await storedItems(folder.id)).length).toBe(row.itemCount)
        })
    })

    describe('폴더 변경끼리의 교착', () => {
        test('곡 넣기, 빼기, 순서 바꾸기, 메모가 한 폴더에 동시에 와도 모두 오류 없이 끝난다', async () => {
            const folder = await createFolder(OWNER, 'mixed-writes')
            const { folderService } = compose()
            await folderService.addItems({
                userId: OWNER,
                folderId: folder.id,
                items: Array.from({ length: 20 }, (_, index) => itemOf(2_000 + index)),
            })
            const settled = await Promise.all(
                Array.from({ length: CONCURRENT_WRITES }, (_, round) =>
                    Promise.all([
                        folderService.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(3_000 + round)] }),
                        folderService.removeItems({ userId: OWNER, folderId: folder.id, md5s: [md5Of(2_000 + round)] }),
                        folderService.updateItemNote({ userId: OWNER, folderId: folder.id, md5: md5Of(2_019 - round), note: `note-${round}` }),
                    ]),
                ),
            )
            expect(settled.flat().every((result) => result.ok)).toBe(true)
            const rows = await storedItems(folder.id)
            const [row] = await getDb().select().from(userFolder).where(eq(userFolder.id, folder.id))
            expect(row.itemCount).toBe(rows.length)
            expect(rows).toHaveLength(20)
        })
    })

    describe('폴더 한 요청 저장', () => {
        test('곡과 이름·설명이 한 번에 저장되고 응답의 updated_at 이 저장된 값이다', async () => {
            const folder = await createFolder(OWNER, 'one-request')
            const result = await compose().folderService.replaceItems({
                userId: OWNER,
                folderId: folder.id,
                input: folderItemsReplaceSchema.parse({
                    items: [itemOf(1), itemOf(2)],
                    name: 'one-request-renamed',
                    description: 'now with songs',
                    base_updated_at: folder.updated_at,
                }),
            })
            if (!result.ok) throw new Error(`replace failed: ${result.reason}`)
            const [row] = await getDb().select().from(userFolder).where(eq(userFolder.id, folder.id))
            expect(row).toMatchObject({
                name: 'one-request-renamed',
                description: 'now with songs',
                itemCount: 2,
                updatedAt: result.value.updated_at,
            })
        })

        test('이름이 이미 쓰이면 name_taken 이고 곡도 이름도 바뀌지 않는다', async () => {
            await createFolder(OWNER, 'taken-name')
            const folder = await createFolder(OWNER, 'keeps-songs')
            await compose().folderService.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1), itemOf(2), itemOf(3)] })
            const before = await storedItems(folder.id)
            const result = await compose().folderService.replaceItems({
                userId: OWNER,
                folderId: folder.id,
                input: folderItemsReplaceSchema.parse({ items: [itemOf(9)], name: 'taken-name' }),
            })
            expect(result).toEqual({ ok: false, reason: 'name_taken' })
            const after = await storedItems(folder.id)
            expect(after.map((row) => row.md5).toSorted()).toEqual(before.map((row) => row.md5).toSorted())
            const [row] = await getDb().select().from(userFolder).where(eq(userFolder.id, folder.id))
            expect(row).toMatchObject({ name: 'keeps-songs', itemCount: 3 })
        })

        test('같은 base 로 동시에 저장하면 한쪽만 저장되고 나머지는 conflict 이다', async () => {
            const folder = await createFolder(OWNER, 'base-race')
            const replace = (n: number) =>
                compose().folderService.replaceItems({
                    userId: OWNER,
                    folderId: folder.id,
                    input: folderItemsReplaceSchema.parse({ items: [itemOf(n)], base_updated_at: folder.updated_at }),
                })
            const results = await Promise.all(Array.from({ length: CONCURRENT_WRITES }, (_, index) => replace(500 + index)))
            expect(results.filter((result) => result.ok)).toHaveLength(1)
            expect(results.filter((result) => !result.ok && result.reason === 'conflict')).toHaveLength(CONCURRENT_WRITES - 1)
            expect(await storedItems(folder.id)).toHaveLength(1)
        })
    })

    describe('API 토큰 폐기', () => {
        test('로그인 토큰을 폐기하면 그 토큰만 못 쓰고 같은 사용자의 다른 토큰은 살아 있다', async () => {
            const { authService } = compose()
            const keep = await authService.issueToken(OWNER, 'keep')
            const drop = await authService.issueToken(OWNER, 'drop')
            expect(await authService.revokeApiToken(drop)).toBe(true)
            expect(await authService.resolveApiTokenUser(drop)).toBeNull()
            expect(await authService.resolveApiTokenUser(keep)).toMatchObject({ id: OWNER })
            expect(await authService.revokeApiToken(drop)).toBe(false)
            expect(await authService.revokeApiToken('rbms_never_issued')).toBe(false)
            const [row] = await getDb()
                .select()
                .from(apiToken)
                .where(eq(apiToken.tokenHash, hashApiToken(drop)))
            expect(row.revoked).toBe(true)
        })
    })

    describe('리플레이 저장량', () => {
        test('사용자별 개수와 바이트 합계를 센다', async () => {
            const { replayService } = compose()
            const upload = (events: typeof REPLAY_EVENTS) =>
                replayService.upload({ input: replayUploadSchema.parse({ format: 'f', events }), chartSha256: CHART_SHA256, userId: OTHER })
            const first = await upload(REPLAY_EVENTS)
            const second = await upload([...REPLAY_EVENTS, ...REPLAY_EVENTS])
            expect(first.created && second.created).toBe(true)
            const rows = await getDb().select({ size: replay.size }).from(replay).where(eq(replay.userId, OTHER))
            expect(rows).toHaveLength(2)
        })

        test('replayExists 는 저장된 리플레이만 true 다', async () => {
            const { replayService } = compose()
            const uploaded = await replayService.upload({
                input: replayUploadSchema.parse({ format: 'f', events: REPLAY_EVENTS }),
                chartSha256: CHART_SHA256,
                userId: OWNER,
            })
            if (!uploaded.created) throw new Error('upload must create')
            expect(await replayService.exists(uploaded.id)).toBe(true)
            expect(await replayService.exists('rp_not_stored')).toBe(false)
        })
    })
})

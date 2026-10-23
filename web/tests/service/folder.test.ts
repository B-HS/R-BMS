import { describe, expect, test } from 'bun:test'
import { FOLDER_LIMIT, folderCreateSchema, folderItemInputSchema, folderItemsReplaceSchema } from '@server/dto/folder'
import {
    createFolderService,
    isSameSet,
    nextStamp,
    planItemInsert,
    resolveItemText,
    sanitiseCopiedText,
    type TableImportEntry,
} from '@server/service/domain/folder/folder.service'
import { createMemoryFolderDb } from '../support/folder-memory-db'

const OWNER = 'user-owner'
const STRANGER = 'user-stranger'
const NOW = 1_760_000_000_000

const md5Of = (n: number) => n.toString(16).padStart(32, '0')

const itemOf = (n: number, extra: Partial<ReturnType<typeof folderItemInputSchema.parse>> = {}) =>
    folderItemInputSchema.parse({ md5: md5Of(n), title: `title-${n}`, artist: `artist-${n}`, ...extra })

const TABLE_ENTRIES: TableImportEntry[] = [
    { md5: md5Of(901), sha256: '', title: 'Table One', artist: 'Table Artist', level: '10' },
    { md5: md5Of(902), sha256: '', title: 'Table Two', artist: 'Table Artist', level: '10' },
    { md5: md5Of(903), sha256: '', title: 'Table Three', artist: 'Table Artist', level: '11' },
]

const createHarness = (options: { now?: () => number; charts?: Parameters<typeof createMemoryFolderDb>[0] } = {}) => {
    const memory = createMemoryFolderDb(options.charts)
    let counter = 0
    const service = createFolderService({
        db: memory.db,
        newId: (prefix) => `${prefix}${(++counter).toString(16).padStart(30, '0')}`,
        now: options.now ?? (() => NOW),
        loadTableEntries: async (tableId) => (tableId === 'insane1' ? TABLE_ENTRIES : null),
        lookupTableText: (md5) => (md5 === md5Of(777) ? { title: 'From Table', artist: 'Table Maker' } : null),
    })
    const createFolder = async (name = 'main', userId = OWNER) => {
        const result = await service.create({ userId, input: folderCreateSchema.parse({ name }) })
        if (!result.ok) throw new Error(`create failed: ${result.reason}`)
        return result.value
    }
    return { ...memory, service, createFolder }
}

describe('folderService.create', () => {
    test('끝에 붙고 position 이 0 부터 빈틈없이 매겨진다', async () => {
        const { createFolder } = createHarness()
        const first = await createFolder('a')
        const second = await createFolder('b')
        expect([first.position, second.position]).toEqual([0, 1])
        expect(first.is_public).toBe(false)
        expect(first.item_count).toBe(0)
    })

    test('같은 이름이면 name_taken 이다', async () => {
        const { service, createFolder } = createHarness()
        await createFolder('dup')
        const result = await service.create({ userId: OWNER, input: folderCreateSchema.parse({ name: 'dup' }) })
        expect(result).toEqual({ ok: false, reason: 'name_taken' })
    })

    test('다른 사용자는 같은 이름을 쓸 수 있다', async () => {
        const { createFolder } = createHarness()
        await createFolder('shared-name', OWNER)
        expect((await createFolder('shared-name', STRANGER)).name).toBe('shared-name')
    })

    test('폴더가 50개면 51번째는 limit_exceeded 이고 아무것도 만들지 않는다', async () => {
        const { service, createFolder } = createHarness()
        for (let n = 0; n < FOLDER_LIMIT.FOLDERS_PER_USER; n += 1) await createFolder(`folder-${n}`)
        const result = await service.create({ userId: OWNER, input: folderCreateSchema.parse({ name: 'one-too-many' }) })
        expect(result).toMatchObject({ ok: false, reason: 'limit_exceeded', details: { scope: 'folders', limit: 50, current: 50 } })
        expect(await service.list(OWNER)).toHaveLength(FOLDER_LIMIT.FOLDERS_PER_USER)
    })
})

describe('남의 폴더는 없는 폴더와 같다', () => {
    test('읽기, 고치기, 지우기, 곡 넣기, 순서, 가져오기가 전부 not_found 이다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder('mine')
        const target = { userId: STRANGER, folderId: folder.id }
        expect(await service.get(target)).toBeNull()
        expect(await service.update({ ...target, input: { name: 'stolen' } })).toEqual({ ok: false, reason: 'not_found' })
        expect(await service.remove(target)).toBe(false)
        expect(await service.addItems({ ...target, items: [itemOf(1)] })).toEqual({ ok: false, reason: 'not_found' })
        expect(await service.removeItems({ ...target, md5s: [md5Of(1)] })).toEqual({ ok: false, reason: 'not_found' })
        expect(await service.reorderItems({ ...target, md5s: [] })).toEqual({ ok: false, reason: 'not_found' })
        expect(await service.replaceItems({ ...target, input: { items: [] } })).toEqual({ ok: false, reason: 'not_found' })
        expect(await service.importFromTable({ ...target, tableId: 'insane1' })).toEqual({ ok: false, reason: 'not_found' })
        expect(await service.updateItemNote({ ...target, md5: md5Of(1), note: 'x' })).toEqual({ ok: false, reason: 'not_found' })
        expect(await service.list(OWNER)).toHaveLength(1)
    })

    test('존재하지 않는 폴더 id 도 not_found 이다', async () => {
        const { service } = createHarness()
        expect(await service.get({ userId: OWNER, folderId: 'uf_missing' })).toBeNull()
    })
})

describe('folderService.update, remove, reorderFolders', () => {
    test('이름 겹침과 존재하지 않음을 구분한다', async () => {
        const { service, createFolder } = createHarness()
        const a = await createFolder('a')
        await createFolder('b')
        expect(await service.update({ userId: OWNER, folderId: a.id, input: { name: 'b' } })).toEqual({ ok: false, reason: 'name_taken' })
        const renamed = await service.update({ userId: OWNER, folderId: a.id, input: { name: 'renamed', description: 'memo' } })
        expect(renamed).toMatchObject({ ok: true, value: { name: 'renamed', description: 'memo' } })
    })

    test('지우면 뒤 폴더의 position 이 당겨진다', async () => {
        const { service, createFolder } = createHarness()
        const a = await createFolder('a')
        await createFolder('b')
        await createFolder('c')
        expect(await service.remove({ userId: OWNER, folderId: a.id })).toBe(true)
        expect((await service.list(OWNER)).map((folder) => [folder.name, folder.position])).toEqual([
            ['b', 0],
            ['c', 1],
        ])
    })

    test('폴더 순서 바꾸기는 지금 집합과 같아야 한다', async () => {
        const { service, createFolder } = createHarness()
        const a = await createFolder('a')
        const b = await createFolder('b')
        expect(await service.reorderFolders({ userId: OWNER, folderIds: [a.id] })).toEqual({ ok: false, reason: 'set_mismatch' })
        expect(await service.reorderFolders({ userId: OWNER, folderIds: [a.id, a.id] })).toEqual({ ok: false, reason: 'set_mismatch' })
        const reordered = await service.reorderFolders({ userId: OWNER, folderIds: [b.id, a.id] })
        expect(reordered.ok && reordered.value.map((folder) => folder.name)).toEqual(['b', 'a'])
    })
})

describe('folderService.addItems', () => {
    test('요청 안의 중복과 이미 든 곡을 건너뛰고 끝에 붙인다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        const first = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1), itemOf(2), itemOf(1)] })
        expect(first).toMatchObject({ ok: true, value: { added: 2, skipped: 1, item_count: 2 } })
        const second = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(2), itemOf(3)] })
        expect(second).toMatchObject({ ok: true, value: { added: 1, skipped: 1, item_count: 3 } })
        const detail = await service.get({ userId: OWNER, folderId: folder.id })
        expect(detail?.items.map((item) => item.md5)).toEqual([md5Of(1), md5Of(2), md5Of(3)])
    })

    test('폴더당 2000곡을 넘으면 아무것도 넣지 않는다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        for (let start = 0; start < FOLDER_LIMIT.ITEMS_PER_FOLDER; start += FOLDER_LIMIT.ITEMS_PER_REQUEST) {
            const batch = Array.from({ length: FOLDER_LIMIT.ITEMS_PER_REQUEST }, (_, offset) => itemOf(start + offset + 1))
            expect((await service.addItems({ userId: OWNER, folderId: folder.id, items: batch })).ok).toBe(true)
        }
        const result = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(9000), itemOf(9001)] })
        expect(result).toMatchObject({
            ok: false,
            reason: 'limit_exceeded',
            details: { scope: 'items_per_folder', limit: 2000, current: 2000, requested: 2 },
        })
        expect((await service.get({ userId: OWNER, folderId: folder.id }))?.item_count).toBe(FOLDER_LIMIT.ITEMS_PER_FOLDER)
    })

    test('사용자 전체 곡이 20000 을 넘으면 거부한다', async () => {
        const { service, createFolder, folders } = createHarness()
        const folder = await createFolder()
        const other = await createFolder('other')
        const stored = folders.get(other.id)
        if (stored) folders.set(other.id, { ...stored, itemCount: FOLDER_LIMIT.ITEMS_PER_USER })
        const result = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] })
        expect(result).toMatchObject({ ok: false, reason: 'limit_exceeded', details: { scope: 'items_per_user', limit: 20000 } })
    })

    test('모든 곡이 이미 있으면 쓰지 않고 updated_at 도 그대로다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        const added = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] })
        const again = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] })
        expect(again).toMatchObject({ ok: true, value: { added: 0, skipped: 1 } })
        expect(again.ok && added.ok && again.value.updated_at).toBe(added.ok ? added.value.updated_at : -1)
    })

    test('제목과 아티스트는 차트 행, 표, 요청 순으로 채운다', async () => {
        const chartSha = 'c'.repeat(64)
        const { service, createFolder } = createHarness({
            charts: [
                { md5: md5Of(1), sha256: chartSha, title: 'From Chart', artist: '' },
                { md5: md5Of(2), sha256: 'd'.repeat(64), title: '', artist: '' },
            ],
        })
        const folder = await createFolder()
        await service.addItems({
            userId: OWNER,
            folderId: folder.id,
            items: [
                itemOf(1, { title: 'requested' }),
                itemOf(2, { title: '', artist: '' }),
                itemOf(777, { title: 'requested', artist: 'requested' }),
            ],
        })
        const detail = await service.get({ userId: OWNER, folderId: folder.id })
        expect(detail?.items[0]).toMatchObject({ title: 'From Chart', artist: 'artist-1', sha256: chartSha })
        expect(detail?.items[1]).toMatchObject({ title: '', artist: '', sha256: 'd'.repeat(64) })
        expect(detail?.items[2]).toMatchObject({ title: 'From Table', artist: 'Table Maker', sha256: '' })
    })

    test('어떤 바뀜이든 updated_at 이 오르고 같은 밀리초에도 앞서지 않는다', async () => {
        const { service, createFolder } = createHarness({ now: () => NOW })
        const folder = await createFolder()
        const first = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] })
        const second = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(2)] })
        const removed = await service.removeItems({ userId: OWNER, folderId: folder.id, md5s: [md5Of(1)] })
        const stamps = [first, second, removed].map((result) => (result.ok ? result.value.updated_at : -1))
        expect(stamps[0]).toBeGreaterThan(folder.updated_at)
        expect(stamps[1]).toBeGreaterThan(stamps[0])
        expect(stamps[2]).toBeGreaterThan(stamps[1])
    })
})

describe('folderService.removeItems, reorderItems, updateItemNote', () => {
    test('빼면 개수와 removed 가 맞고 없는 곡은 세지 않는다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1), itemOf(2), itemOf(3)] })
        const result = await service.removeItems({ userId: OWNER, folderId: folder.id, md5s: [md5Of(2), md5Of(2), md5Of(99)] })
        expect(result).toMatchObject({ ok: true, value: { removed: 1, item_count: 2 } })
    })

    test('곡 순서 바꾸기는 지금 집합과 같아야 하고 다르면 set_mismatch 이다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1), itemOf(2), itemOf(3)] })
        const base = { userId: OWNER, folderId: folder.id }
        expect(await service.reorderItems({ ...base, md5s: [md5Of(1), md5Of(2)] })).toEqual({ ok: false, reason: 'set_mismatch' })
        expect(await service.reorderItems({ ...base, md5s: [md5Of(1), md5Of(2), md5Of(99)] })).toEqual({ ok: false, reason: 'set_mismatch' })
        expect(await service.reorderItems({ ...base, md5s: [md5Of(1), md5Of(1), md5Of(2)] })).toEqual({ ok: false, reason: 'set_mismatch' })
        expect((await service.reorderItems({ ...base, md5s: [md5Of(3), md5Of(1), md5Of(2)] })).ok).toBe(true)
        expect((await service.get(base))?.items.map((item) => item.md5)).toEqual([md5Of(3), md5Of(1), md5Of(2)])
    })

    test('메모를 고치고 없는 곡이면 not_found 이다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] })
        const base = { userId: OWNER, folderId: folder.id }
        expect(await service.updateItemNote({ ...base, md5: md5Of(1), note: '오른손' })).toMatchObject({ ok: true, value: { note: '오른손' } })
        expect(await service.updateItemNote({ ...base, md5: md5Of(2), note: 'x' })).toEqual({ ok: false, reason: 'not_found' })
    })
})

describe('folderService.replaceItems', () => {
    test('요청 순서대로 통째로 바꾸고 added, removed, skipped 를 센다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1), itemOf(2), itemOf(3)] })
        const result = await service.replaceItems({
            userId: OWNER,
            folderId: folder.id,
            input: folderItemsReplaceSchema.parse({ items: [itemOf(3, { note: 'n3' }), itemOf(4), itemOf(1), itemOf(4)] }),
        })
        expect(result).toMatchObject({ ok: true, value: { added: 1, removed: 1, skipped: 1, item_count: 3 } })
        const detail = await service.get({ userId: OWNER, folderId: folder.id })
        expect(detail?.items.map((item) => [item.md5, item.note])).toEqual([
            [md5Of(3), 'n3'],
            [md5Of(4), ''],
            [md5Of(1), ''],
        ])
        expect(detail?.items[0]?.title).toBe('title-3')
    })

    test('base_updated_at 이 어긋나면 conflict 와 서버 값을 돌려주고 쓰지 않는다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        const added = await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] })
        const serverStamp = added.ok ? added.value.updated_at : -1
        const stale = await service.replaceItems({
            userId: OWNER,
            folderId: folder.id,
            input: folderItemsReplaceSchema.parse({ items: [], base_updated_at: serverStamp - 1 }),
        })
        expect(stale).toEqual({ ok: false, reason: 'conflict', details: { server_updated_at: serverStamp } })
        expect((await service.get({ userId: OWNER, folderId: folder.id }))?.item_count).toBe(1)
        const fresh = await service.replaceItems({
            userId: OWNER,
            folderId: folder.id,
            input: folderItemsReplaceSchema.parse({ items: [], base_updated_at: serverStamp }),
        })
        expect(fresh).toMatchObject({ ok: true, value: { removed: 1, item_count: 0 } })
    })
})

describe('folderService.replaceItems 의 이름·설명 한 요청 저장', () => {
    test('곡과 이름·설명이 같은 요청으로 바뀌고 응답의 updated_at 이 목록에도 그대로다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder('old name')
        const result = await service.replaceItems({
            userId: OWNER,
            folderId: folder.id,
            input: folderItemsReplaceSchema.parse({
                items: [itemOf(1)],
                name: 'new name',
                description: 'new description',
                base_updated_at: folder.updated_at,
            }),
        })
        expect(result).toMatchObject({ ok: true, value: { added: 1, item_count: 1 } })
        const stamp = result.ok ? result.value.updated_at : -1
        expect(await service.get({ userId: OWNER, folderId: folder.id })).toMatchObject({
            name: 'new name',
            description: 'new description',
            updated_at: stamp,
            items: [{ md5: md5Of(1) }],
        })
    })

    test('이름이 다른 폴더와 겹치면 name_taken 이고 곡도 바뀌지 않는다', async () => {
        const { service, createFolder } = createHarness()
        await createFolder('taken')
        const folder = await createFolder('mine')
        await service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1), itemOf(2)] })
        const before = await service.get({ userId: OWNER, folderId: folder.id })
        const result = await service.replaceItems({
            userId: OWNER,
            folderId: folder.id,
            input: folderItemsReplaceSchema.parse({ items: [itemOf(3)], name: 'Taken' }),
        })
        expect(result).toEqual({ ok: false, reason: 'name_taken' })
        expect(await service.get({ userId: OWNER, folderId: folder.id })).toEqual(before)
    })

    test('이름과 설명을 보내지 않으면 그대로 두고 곡만 바꾼다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder('keep me')
        await service.replaceItems({ userId: OWNER, folderId: folder.id, input: folderItemsReplaceSchema.parse({ items: [itemOf(1)] }) })
        expect(await service.get({ userId: OWNER, folderId: folder.id })).toMatchObject({ name: 'keep me', item_count: 1 })
    })
})

describe('folderService.addItems 동시 요청', () => {
    test('같은 곡을 동시에 넣으면 한쪽만 added 로 세고 다른 쪽은 skipped 이며 오류가 없다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        const add = () => service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(1)] })
        const results = await Promise.all([add(), add()])
        expect(results.every((result) => result.ok)).toBe(true)
        const counted = results.flatMap((result) => (result.ok ? [result.value] : []))
        expect(counted.map((value) => value.added).toSorted()).toEqual([0, 1])
        expect(counted.map((value) => value.skipped).toSorted()).toEqual([0, 1])
        expect((await service.get({ userId: OWNER, folderId: folder.id }))?.item_count).toBe(1)
    })

    test('동시에 들어온 두 요청이 합쳐서 폴더 한도를 넘으면 뒤에 처리된 쪽이 limit_exceeded 이다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        const block = (offset: number) => Array.from({ length: FOLDER_LIMIT.ITEMS_PER_REQUEST }, (_, index) => itemOf(offset + index))
        const fillTo = FOLDER_LIMIT.ITEMS_PER_FOLDER - FOLDER_LIMIT.ITEMS_PER_REQUEST
        for (let start = 0; start < fillTo; start += FOLDER_LIMIT.ITEMS_PER_REQUEST) {
            await service.addItems({ userId: OWNER, folderId: folder.id, items: block(start) })
        }
        const results = await Promise.all([
            service.addItems({ userId: OWNER, folderId: folder.id, items: [itemOf(900_000), itemOf(900_001)] }),
            service.addItems({ userId: OWNER, folderId: folder.id, items: block(800_000) }),
        ])
        const failed = results.filter((result) => !result.ok)
        expect(failed).toHaveLength(1)
        expect(failed[0]).toMatchObject({ reason: 'limit_exceeded', details: { scope: 'items_per_folder' } })
        expect((await service.get({ userId: OWNER, folderId: folder.id }))?.item_count).toBeLessThanOrEqual(FOLDER_LIMIT.ITEMS_PER_FOLDER)
    })
})

describe('folderService.importFromTable', () => {
    test('레벨을 고르면 그 레벨만, 안 고르면 전부 넣고 두 번째는 건너뛴다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        const base = { userId: OWNER, folderId: folder.id, tableId: 'insane1' }
        expect(await service.importFromTable({ ...base, levels: ['10'] })).toMatchObject({ ok: true, value: { added: 2, skipped: 0, item_count: 2 } })
        expect(await service.importFromTable(base)).toMatchObject({ ok: true, value: { added: 1, skipped: 2, item_count: 3 } })
        const detail = await service.get({ userId: OWNER, folderId: folder.id })
        expect(detail?.items[0]).toMatchObject({ title: 'Table One', artist: 'Table Artist', sha256: '' })
    })

    test('모르는 표는 table_not_found, 없는 레벨은 0곡이다', async () => {
        const { service, createFolder } = createHarness()
        const folder = await createFolder()
        expect(await service.importFromTable({ userId: OWNER, folderId: folder.id, tableId: 'nope' })).toEqual({
            ok: false,
            reason: 'table_not_found',
        })
        expect(await service.importFromTable({ userId: OWNER, folderId: folder.id, tableId: 'insane1', levels: ['99'] })).toMatchObject({
            ok: true,
            value: { added: 0 },
        })
    })
})

describe('folderService.listContaining', () => {
    test('내 폴더 중 그 곡이 든 폴더의 id 만 돌려준다', async () => {
        const { service, createFolder } = createHarness()
        const a = await createFolder('a')
        const b = await createFolder('b')
        const theirs = await createFolder('theirs', STRANGER)
        await service.addItems({ userId: OWNER, folderId: b.id, items: [itemOf(1)] })
        await service.addItems({ userId: STRANGER, folderId: theirs.id, items: [itemOf(1)] })
        expect(await service.listContaining({ userId: OWNER, md5: md5Of(1) })).toEqual([b.id])
        expect(await service.listContaining({ userId: OWNER, md5: md5Of(2) })).toEqual([])
        expect(a.id).not.toBe(b.id)
    })
})

describe('순수 함수', () => {
    test('isSameSet 은 순서와 무관하고 중복을 허용하지 않는다', () => {
        expect(isSameSet(['a', 'b'], ['b', 'a'])).toBe(true)
        expect(isSameSet(['a', 'b'], ['a', 'a'])).toBe(false)
        expect(isSameSet(['a'], ['a', 'b'])).toBe(false)
    })

    test('planItemInsert 는 한도를 폴더, 사용자 순으로 본다', () => {
        const incoming = [itemOf(1), itemOf(2)]
        expect(planItemInsert({ existingMd5s: [], incoming, folderCount: 1999, userCount: 0 }).exceeded?.scope).toBe('items_per_folder')
        expect(planItemInsert({ existingMd5s: [], incoming, folderCount: 0, userCount: 19999 }).exceeded?.scope).toBe('items_per_user')
        expect(planItemInsert({ existingMd5s: [md5Of(1)], incoming, folderCount: 1999, userCount: 0 }).exceeded).toBeNull()
    })

    test('sanitiseCopiedText 는 제어 문자를 지우고 200자(코드 포인트)로 자른다', () => {
        expect(sanitiseCopiedText('  a\u0000b\nc  ')).toBe('abc')
        expect(sanitiseCopiedText('가'.repeat(300))).toHaveLength(FOLDER_LIMIT.TEXT_COPY_MAX)
    })

    test('resolveItemText 는 빈 값을 건너뛰어 다음 출처를 쓴다', () => {
        const resolved = resolveItemText({
            input: { sha256: '', title: 'requested', artist: 'requested' },
            chart: { md5: md5Of(1), sha256: 'e'.repeat(64), title: '', artist: 'chart artist' },
            tableText: { title: 'table title', artist: 'table artist' },
        })
        expect(resolved).toEqual({ sha256: 'e'.repeat(64), title: 'table title', artist: 'chart artist' })
    })

    test('nextStamp 는 이전 값보다 항상 크다', () => {
        expect(nextStamp(100, 100)).toBe(101)
        expect(nextStamp(100, 50)).toBe(100)
    })
})

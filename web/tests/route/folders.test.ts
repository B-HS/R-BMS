import { beforeEach, describe, expect, test } from 'bun:test'
import { FOLDER_LIMIT } from '@server/dto/folder'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createFolderRoutes } from '@server/route/folder'
import { createFolderService } from '@server/service/domain/folder/folder.service'
import { createMemoryFolderDb } from '../support/folder-memory-db'

const ALICE: AuthUser = { id: 'user-alice', loginId: 'alice', name: 'Alice', role: 'user' }
const BOB: AuthUser = { id: 'user-bob', loginId: 'bob', name: 'Bob', role: 'user' }
const USERS = new Map([ALICE, BOB].map((user) => [user.loginId, user]))
const BEARER_TOKENS = new Map([
    ['token-alice', ALICE],
    ['token-bob', BOB],
])
const MD5 = 'a'.repeat(32)
const ORIGIN = 'http://localhost:3000'

const createRoutes = () => {
    let counter = 0
    const memory = createMemoryFolderDb()
    const folderService = createFolderService({
        db: memory.db,
        newId: (prefix) => `${prefix}${(++counter).toString(16).padStart(30, '0')}`,
        now: () => 1_760_000_000_000,
        loadTableEntries: async (tableId) => (tableId === 'insane1' ? [{ md5: MD5, sha256: '', title: 'T', artist: 'A', level: '1' }] : null),
        lookupTableText: () => null,
    })
    return createFolderRoutes(() => ({
        authService: {
            resolveSessionUser: async (request) => USERS.get(request.headers.get('x-session-user') ?? '') ?? null,
            resolveApiTokenUser: async (token) => BEARER_TOKENS.get(token) ?? null,
        },
        folderService,
    }))
}

const call = (method: string, path: string, init: { user?: AuthUser; bearer?: string; body?: unknown; headers?: Record<string, string> } = {}) =>
    new Request(`${ORIGIN}${path}`, {
        method,
        headers: {
            'content-type': 'application/json',
            ...(init.user ? { 'x-session-user': init.user.loginId } : {}),
            ...(init.bearer ? { authorization: `Bearer ${init.bearer}` } : {}),
            ...init.headers,
        },
        body: init.body === undefined ? undefined : JSON.stringify(init.body),
    })

const folderContext = (folderId: string) => ({ params: Promise.resolve({ folderId }) })

const createFolder = async (routes: ReturnType<typeof createRoutes>, user: AuthUser, name: string) => {
    const response = await routes.create(call('POST', '/api/fe/me/folders', { user, body: { name } }), undefined)
    return (await response.json()) as { success: boolean; data: { id: string; name: string; updated_at: number } }
}

beforeEach(() => resetRateLimits())

describe('편집 API 의 인증', () => {
    test('세션이 없으면 전부 401 UNAUTHORIZED 이다', async () => {
        const routes = createRoutes()
        const id = `uf_${'a'.repeat(30)}`
        const attempts = [
            routes.list(call('GET', '/api/fe/me/folders'), undefined),
            routes.create(call('POST', '/api/fe/me/folders', { body: { name: 'x' } }), undefined),
            routes.reorder(call('PUT', '/api/fe/me/folders/order', { body: { folder_ids: [] } }), undefined),
            routes.detail(call('GET', `/api/fe/me/folders/${id}`), folderContext(id)),
            routes.update(call('PATCH', `/api/fe/me/folders/${id}`, { body: { name: 'x' } }), folderContext(id)),
            routes.remove(call('DELETE', `/api/fe/me/folders/${id}`), folderContext(id)),
            routes.addItems(call('POST', `/api/fe/me/folders/${id}/items`, { body: { items: [{ md5: MD5 }] } }), folderContext(id)),
            routes.removeItems(call('POST', `/api/fe/me/folders/${id}/items/remove`, { body: { md5s: [MD5] } }), folderContext(id)),
            routes.reorderItems(call('PUT', `/api/fe/me/folders/${id}/items/order`, { body: { md5s: [] } }), folderContext(id)),
            routes.replaceItems(call('PUT', `/api/fe/me/folders/${id}/items`, { body: { items: [] } }), folderContext(id)),
            routes.importFromTable(call('POST', `/api/fe/me/folders/${id}/import`, { body: { table_id: 'insane1' } }), folderContext(id)),
            routes.containing(call('GET', `/api/fe/me/folders/containing/${MD5}`), { params: Promise.resolve({ md5: MD5 }) }),
        ]
        for (const response of await Promise.all(attempts)) {
            expect(response.status).toBe(401)
            expect((await response.json()).error.code).toBe('UNAUTHORIZED')
        }
    })

    test('API 토큰(Bearer)만으로는 편집할 수 없다', async () => {
        const routes = createRoutes()
        const response = await routes.create(call('POST', '/api/fe/me/folders', { bearer: 'token-alice', body: { name: 'x' } }), undefined)
        expect(response.status).toBe(401)
    })
})

describe('폴더 만들기, 고치기, 지우기', () => {
    test('만들면 201 과 봉투와 private 캐시 헤더를 돌려준다', async () => {
        const routes = createRoutes()
        const response = await routes.create(
            call('POST', '/api/fe/me/folders', { user: ALICE, body: { name: '연습 12단', description: '하루 한 판' } }),
            undefined,
        )
        expect(response.status).toBe(201)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        const body = await response.json()
        expect(body).toMatchObject({
            success: true,
            data: { name: '연습 12단', description: '하루 한 판', is_public: false, position: 0, item_count: 0 },
        })
        expect(body.data.id).toMatch(/^uf_[0-9a-f]{30}$/)
    })

    test('같은 이름은 409 IR_FOLDER_NAME_TAKEN 이다', async () => {
        const routes = createRoutes()
        await createFolder(routes, ALICE, 'dup')
        const response = await routes.create(call('POST', '/api/fe/me/folders', { user: ALICE, body: { name: 'dup' } }), undefined)
        expect(response.status).toBe(409)
        expect((await response.json()).error.code).toBe('IR_FOLDER_NAME_TAKEN')
    })

    test('51번째 폴더는 409 IR_FOLDER_LIMIT_EXCEEDED 이다', async () => {
        const routes = createRoutes()
        for (let n = 0; n < FOLDER_LIMIT.FOLDERS_PER_USER; n += 1) await createFolder(routes, ALICE, `f-${n}`)
        const response = await routes.create(call('POST', '/api/fe/me/folders', { user: ALICE, body: { name: 'over' } }), undefined)
        expect(response.status).toBe(409)
        expect((await response.json()).error.code).toBe('IR_FOLDER_LIMIT_EXCEEDED')
    })

    test('잘못된 입력은 400 VALIDATION_ERROR 이다', async () => {
        const routes = createRoutes()
        const empty = await routes.create(call('POST', '/api/fe/me/folders', { user: ALICE, body: { name: '' } }), undefined)
        expect(empty.status).toBe(400)
        expect((await empty.json()).error.code).toBe('VALIDATION_ERROR')
        const notJson = await routes.create(
            new Request(`${ORIGIN}/api/fe/me/folders`, { method: 'POST', headers: { 'x-session-user': 'alice' }, body: 'not json' }),
            undefined,
        )
        expect(notJson.status).toBe(400)
    })

    test('모양이 틀린 폴더 id 는 400 이다', async () => {
        const routes = createRoutes()
        const response = await routes.detail(call('GET', '/api/fe/me/folders/..%2Fx', { user: ALICE }), folderContext('../x'))
        expect(response.status).toBe(400)
    })

    test('본문이 512KB 를 넘으면 413 IR_PAYLOAD_TOO_LARGE 이다', async () => {
        const routes = createRoutes()
        const response = await routes.create(
            call('POST', '/api/fe/me/folders', {
                user: ALICE,
                body: { name: 'x' },
                headers: { 'content-length': String(FOLDER_LIMIT.WRITE_MAX_BYTES + 1) },
            }),
            undefined,
        )
        expect(response.status).toBe(413)
        expect((await response.json()).error.code).toBe('IR_PAYLOAD_TOO_LARGE')
    })

    test('고치고 지운다. 지운 뒤 읽으면 404 이고 지우기는 204 이다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'old')).data
        const patched = await routes.update(
            call('PATCH', `/api/fe/me/folders/${folder.id}`, { user: ALICE, body: { name: 'new' } }),
            folderContext(folder.id),
        )
        expect((await patched.json()).data.name).toBe('new')
        const removed = await routes.remove(call('DELETE', `/api/fe/me/folders/${folder.id}`, { user: ALICE }), folderContext(folder.id))
        expect(removed.status).toBe(204)
        const gone = await routes.detail(call('GET', `/api/fe/me/folders/${folder.id}`, { user: ALICE }), folderContext(folder.id))
        expect(gone.status).toBe(404)
        expect((await gone.json()).error.code).toBe('IR_FOLDER_NOT_FOUND')
    })
})

describe('남의 폴더', () => {
    test('읽기, 고치기, 지우기, 곡 넣기, 가져오기가 전부 404 IR_FOLDER_NOT_FOUND 이고 폴더는 그대로다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'alice-only')).data
        const context = folderContext(folder.id)
        const responses = await Promise.all([
            routes.detail(call('GET', `/api/fe/me/folders/${folder.id}`, { user: BOB }), context),
            routes.update(call('PATCH', `/api/fe/me/folders/${folder.id}`, { user: BOB, body: { name: 'mine now' } }), context),
            routes.remove(call('DELETE', `/api/fe/me/folders/${folder.id}`, { user: BOB }), context),
            routes.addItems(call('POST', `/api/fe/me/folders/${folder.id}/items`, { user: BOB, body: { items: [{ md5: MD5 }] } }), context),
            routes.replaceItems(call('PUT', `/api/fe/me/folders/${folder.id}/items`, { user: BOB, body: { items: [] } }), context),
            routes.importFromTable(call('POST', `/api/fe/me/folders/${folder.id}/import`, { user: BOB, body: { table_id: 'insane1' } }), context),
        ])
        for (const response of responses) {
            expect(response.status).toBe(404)
            expect((await response.json()).error.code).toBe('IR_FOLDER_NOT_FOUND')
        }
        const stillThere = await routes.detail(call('GET', `/api/fe/me/folders/${folder.id}`, { user: ALICE }), context)
        expect((await stillThere.json()).data).toMatchObject({ name: 'alice-only', item_count: 0 })
        const bobList = await routes.list(call('GET', '/api/fe/me/folders', { user: BOB }), undefined)
        expect((await bobList.json()).data).toEqual([])
    })
})

describe('곡 넣기, 순서, 통째로 바꾸기, 가져오기', () => {
    test('넣고 읽으면 순서와 개수가 맞고 containing 이 폴더 id 를 돌려준다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'main')).data
        const context = folderContext(folder.id)
        const added = await routes.addItems(
            call('POST', `/api/fe/me/folders/${folder.id}/items`, { user: ALICE, body: { items: [{ md5: MD5, title: 'Song', artist: 'Me' }] } }),
            context,
        )
        expect(added.status).toBe(200)
        expect((await added.json()).data).toMatchObject({ added: 1, removed: 0, skipped: 0, item_count: 1 })
        const containing = await routes.containing(call('GET', `/api/fe/me/folders/containing/${MD5}`, { user: ALICE }), {
            params: Promise.resolve({ md5: MD5 }),
        })
        expect((await containing.json()).data).toEqual([folder.id])
        const detail = await routes.detail(call('GET', `/api/fe/me/folders/${folder.id}`, { user: ALICE }), context)
        expect((await detail.json()).data.items).toEqual([{ md5: MD5, sha256: '', title: 'Song', artist: 'Me', note: '' }])
    })

    test('통째로 바꾸기의 base_updated_at 이 어긋나면 409 IR_FOLDER_CONFLICT 이다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'main')).data
        const response = await routes.replaceItems(
            call('PUT', `/api/fe/me/folders/${folder.id}/items`, { user: ALICE, body: { items: [], base_updated_at: folder.updated_at + 5 } }),
            folderContext(folder.id),
        )
        expect(response.status).toBe(409)
        expect((await response.json()).error.code).toBe('IR_FOLDER_CONFLICT')
    })

    test('집합이 다른 곡 순서는 400 이다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'main')).data
        const response = await routes.reorderItems(
            call('PUT', `/api/fe/me/folders/${folder.id}/items/order`, { user: ALICE, body: { md5s: [MD5] } }),
            folderContext(folder.id),
        )
        expect(response.status).toBe(400)
    })

    test('모르는 표 가져오기는 404 IR_TABLE_NOT_FOUND, 아는 표는 넣는다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'main')).data
        const unknown = await routes.importFromTable(
            call('POST', `/api/fe/me/folders/${folder.id}/import`, { user: ALICE, body: { table_id: 'nope' } }),
            folderContext(folder.id),
        )
        expect(unknown.status).toBe(404)
        expect((await unknown.json()).error.code).toBe('IR_TABLE_NOT_FOUND')
        const known = await routes.importFromTable(
            call('POST', `/api/fe/me/folders/${folder.id}/import`, { user: ALICE, body: { table_id: 'insane1' } }),
            folderContext(folder.id),
        )
        expect((await known.json()).data).toMatchObject({ added: 1 })
    })
})

describe('rate limit', () => {
    test('쓰기는 분당 60회를 넘으면 429 RATE_LIMITED 이다', async () => {
        const routes = createRoutes()
        let last = 0
        for (let n = 0; n <= 60; n += 1) {
            const response = await routes.create(call('POST', '/api/fe/me/folders', { user: ALICE, body: { name: '' } }), undefined)
            last = response.status
        }
        expect(last).toBe(429)
        const other = await routes.create(call('POST', '/api/fe/me/folders', { user: BOB, body: { name: '' } }), undefined)
        expect(other.status).toBe(400)
    })

    test('표에서 가져오기는 분당 10회를 넘으면 429 이다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'main')).data
        const statuses: number[] = []
        for (let n = 0; n < 11; n += 1) {
            const response = await routes.importFromTable(
                call('POST', `/api/fe/me/folders/${folder.id}/import`, { user: ALICE, body: { table_id: 'insane1' } }),
                folderContext(folder.id),
            )
            statuses.push(response.status)
        }
        expect(statuses.slice(0, 10).every((status) => status === 200)).toBe(true)
        expect(statuses[10]).toBe(429)
    })
})

describe('클라이언트가 읽는 API', () => {
    const playerContext = (playerId: string, folderId?: string) => ({ params: Promise.resolve({ playerId, folderId }) })

    test('인증이 없으면 401 이다', async () => {
        const routes = createRoutes()
        const response = await routes.readIndex(call('GET', '/api/players/alice/folders'), playerContext('alice'))
        expect(response.status).toBe(401)
    })

    test('다른 사람의 playerId 는 Bearer 든 세션이든 403 FORBIDDEN 이다', async () => {
        const routes = createRoutes()
        const byBearer = await routes.readIndex(call('GET', '/api/players/alice/folders', { bearer: 'token-bob' }), playerContext('alice'))
        expect(byBearer.status).toBe(403)
        expect((await byBearer.json()).error.code).toBe('FORBIDDEN')
        const bySession = await routes.readIndex(call('GET', '/api/players/alice/folders', { user: BOB }), playerContext('alice'))
        expect(bySession.status).toBe(403)
    })

    test('본인 토큰이면 봉투 없는 색인과 폴더를 돌려준다', async () => {
        const routes = createRoutes()
        const folder = (await createFolder(routes, ALICE, 'main')).data
        await routes.addItems(
            call('POST', `/api/fe/me/folders/${folder.id}/items`, { user: ALICE, body: { items: [{ md5: MD5, title: 'Song', artist: 'Me' }] } }),
            folderContext(folder.id),
        )
        const index = await routes.readIndex(call('GET', '/api/players/alice/folders', { bearer: 'token-alice' }), playerContext('alice'))
        expect(index.status).toBe(200)
        expect(index.headers.get('cache-control')).toBe('private, no-store')
        const indexBody = await index.json()
        expect(indexBody.success).toBeUndefined()
        expect(indexBody.folders).toHaveLength(1)
        expect(indexBody.folders[0]).toMatchObject({ id: folder.id, name: 'main', item_count: 1, is_public: false })

        const detail = await routes.readFolder(
            call('GET', `/api/players/alice/folders/${folder.id}`, { bearer: 'token-alice' }),
            playerContext('alice', folder.id),
        )
        const detailBody = await detail.json()
        expect(detailBody).toMatchObject({ id: folder.id, name: 'main', items: [{ md5: MD5, sha256: '', title: 'Song', artist: 'Me', note: '' }] })
    })

    test('없는 폴더는 404 IR_FOLDER_NOT_FOUND, 남의 폴더 id 도 404 이다', async () => {
        const routes = createRoutes()
        const bobFolder = (await createFolder(routes, BOB, 'bobs')).data
        const missing = await routes.readFolder(
            call('GET', `/api/players/alice/folders/uf_${'b'.repeat(30)}`, { bearer: 'token-alice' }),
            playerContext('alice', `uf_${'b'.repeat(30)}`),
        )
        expect(missing.status).toBe(404)
        const foreign = await routes.readFolder(
            call('GET', `/api/players/alice/folders/${bobFolder.id}`, { bearer: 'token-alice' }),
            playerContext('alice', bobFolder.id),
        )
        expect(foreign.status).toBe(404)
        expect((await foreign.json()).error.code).toBe('IR_FOLDER_NOT_FOUND')
    })
})

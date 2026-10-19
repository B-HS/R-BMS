import { beforeEach, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join, relative } from 'node:path'
import { TRACE_MAX_BYTES, TRACE_UPLOAD_RATE_LIMIT } from '@server/dto/trace'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createTraceRoutes } from '@server/route/trace'
import { countTraceJudgements } from '@server/service/domain/trace/trace-check'
import { OTHER_USER_ID, TRACE_USER_ID, createMemoryTraceDb, createMemoryTraceService, readContractTrace, scoreRowFor } from '../support/trace-fixture'

const PLAYER: AuthUser = { id: TRACE_USER_ID, loginId: 'tracer', name: 'Tracer', role: 'user' }
const OTHER: AuthUser = { id: OTHER_USER_ID, loginId: 'other', name: 'Other', role: 'user' }
const ADMIN: AuthUser = { id: 'usr_admin', loginId: 'admin', name: 'Admin', role: 'admin' }
const TOKENS = new Map([
    ['rbms_player', PLAYER],
    ['rbms_other', OTHER],
    ['rbms_admin', ADMIN],
])
const SESSIONS = new Map([['player-session', PLAYER]])
const ORIGIN = 'http://localhost:3000'
const WEB_ROOT = join(import.meta.dirname, '..', '..')
const NORMAL = readContractTrace('trace-normal.json')

const createRoutes = () => {
    const memory = createMemoryTraceDb([scoreRowFor(NORMAL, countTraceJudgements(NORMAL))], { [TRACE_USER_ID]: 'tracer' })
    const { service } = createMemoryTraceService(memory)
    const routes = createTraceRoutes(
        () => ({
            authService: {
                resolveApiTokenUser: async (plain) => TOKENS.get(plain) ?? null,
                resolveSessionUser: async (request) => SESSIONS.get(request.headers.get('cookie') ?? '') ?? null,
            },
            traceService: service,
        }),
        async () => undefined,
    )
    return { routes, memory }
}

const bearer = (token: string) => ({ authorization: `Bearer ${token}` })

const uploadRequest = (hash: string, body: unknown, headers: Record<string, string> = {}) =>
    new Request(`${ORIGIN}/api/charts/${hash}/traces`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', ...headers },
        body: typeof body === 'string' ? body : JSON.stringify(body),
    })

const hashContext = (hash: string) => ({ params: Promise.resolve({ hash }) })

const upload = (routes: ReturnType<typeof createRoutes>['routes'], body: unknown, headers: Record<string, string>, hash = NORMAL.chart.md5) =>
    routes.upload(uploadRequest(hash, body, headers), hashContext(hash))

beforeEach(() => resetRateLimits())

describe('POST /api/charts/[hash]/traces', () => {
    test('API 토큰이 없거나 모르는 토큰이면 401 이고 저장하지 않는다', async () => {
        const { routes, memory } = createRoutes()
        expect((await upload(routes, NORMAL, {})).status).toBe(401)
        expect((await upload(routes, NORMAL, { cookie: 'player-session' })).status).toBe(401)
        expect((await upload(routes, NORMAL, bearer('rbms_unknown'))).status).toBe(401)
        expect(memory.traces.size).toBe(0)
    })

    test('처음 올린 trace 는 201 과 id, 같은 점수의 재전송은 200 과 같은 id 다', async () => {
        const { routes, memory } = createRoutes()
        const first = await upload(routes, NORMAL, bearer('rbms_player'))
        expect(first.status).toBe(201)
        const created = await first.json()
        expect(created).toMatchObject({ id: 'tr_1', analysis: 'stored', storage: { traces: 1, level: 'ok' } })
        const again = await upload(routes, NORMAL, bearer('rbms_player'))
        expect(again.status).toBe(200)
        expect(await again.json()).toMatchObject({ id: 'tr_1' })
        expect(memory.traces.size).toBe(1)
    })

    test('경로의 해시는 sha256 이어도 되고, 해시 꼴이 아니면 404 다', async () => {
        const { routes } = createRoutes()
        expect((await upload(routes, NORMAL, bearer('rbms_player'), NORMAL.chart.sha256)).status).toBe(201)
        expect((await upload(routes, NORMAL, bearer('rbms_player'), 'not-a-hash')).status).toBe(404)
    })

    test('다른 사용자의 점수에 붙이면 존재를 드러내지 않는 404 다', async () => {
        const { routes, memory } = createRoutes()
        const response = await upload(routes, NORMAL, bearer('rbms_other'))
        expect(response.status).toBe(404)
        expect((await response.json()).error.code).toBe('IR_SCORE_NOT_FOUND')
        expect(memory.traces.size).toBe(0)
    })

    test('점수와 합계가 어긋나면 400 IR_TRACE_MISMATCH 와 어긋난 칸이다', async () => {
        const { routes } = createRoutes()
        const state = NORMAL.notes.state.map((value, index) => (index === 0 ? 1 : value))
        const response = await upload(routes, { ...NORMAL, notes: { ...NORMAL.notes, state } }, bearer('rbms_player'))
        expect(response.status).toBe(400)
        expect((await response.json()).error).toMatchObject({ code: 'IR_TRACE_MISMATCH', details: { field: 'judge.pgreat' } })
    })

    test('형식이 다른 본문은 400 VALIDATION_ERROR 다', async () => {
        const { routes } = createRoutes()
        const response = await upload(routes, { ...NORMAL, notes: { ...NORMAL.notes, lane: NORMAL.notes.lane.slice(1) } }, bearer('rbms_player'))
        expect(response.status).toBe(400)
        expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
    })

    test('클라이언트 한도(2 MiB)를 넘는 본문은 인증 전에 413 이다', async () => {
        const { routes } = createRoutes()
        const oversized = uploadRequest(NORMAL.chart.md5, NORMAL, { 'content-length': String(TRACE_MAX_BYTES + 1) })
        expect((await routes.upload(oversized, hashContext(NORMAL.chart.md5))).status).toBe(413)
        const streamed = `${JSON.stringify(NORMAL).slice(0, -1)},"pad":"${'x'.repeat(TRACE_MAX_BYTES)}"}`
        expect((await upload(routes, streamed, bearer('rbms_player'))).status).toBe(413)
    })

    test(`한 계정은 분당 ${TRACE_UPLOAD_RATE_LIMIT.limit}건까지 받고 그다음은 429 다`, async () => {
        const { routes } = createRoutes()
        const statuses = []
        for (let sent = 0; sent <= TRACE_UPLOAD_RATE_LIMIT.limit; sent += 1)
            statuses.push((await upload(routes, NORMAL, bearer('rbms_player'))).status)
        expect(statuses.filter((status) => status === 429)).toHaveLength(1)
        expect(statuses.at(-1)).toBe(429)
    })
})

describe('GET /api/fe/me/traces/usage 와 GET /api/admin/traces/usage', () => {
    test('본인 보관량은 세션으로만 읽고, 캐시하지 않는다', async () => {
        const { routes } = createRoutes()
        expect((await routes.mine(new Request(`${ORIGIN}/api/fe/me/traces/usage`, { headers: bearer('rbms_player') }), undefined)).status).toBe(401)
        const response = await routes.mine(new Request(`${ORIGIN}/api/fe/me/traces/usage`, { headers: { cookie: 'player-session' } }), undefined)
        expect(response.status).toBe(200)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        expect(await response.json()).toMatchObject({ success: true, data: { traces: 0, level: 'ok' } })
    })

    test('관리자 수치는 관리자 API 토큰만 받는다', async () => {
        const { routes } = createRoutes()
        await upload(routes, NORMAL, bearer('rbms_player'))
        const request = (headers: Record<string, string>) => new Request(`${ORIGIN}/api/admin/traces/usage`, { headers })
        expect((await routes.adminUsage(request({}), undefined)).status).toBe(401)
        expect((await routes.adminUsage(request(bearer('rbms_player')), undefined)).status).toBe(403)
        const response = await routes.adminUsage(request(bearer('rbms_admin')), undefined)
        expect(response.status).toBe(200)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        expect((await response.json()).data).toMatchObject({ totals: { traces: 1 }, top: [{ login_id: 'tracer', traces: 1 }] })
    })
})

describe('trace 읽기 공개 경로 없음', () => {
    const routeFiles = [...new Bun.Glob('src/app/**/route.ts').scanSync({ cwd: WEB_ROOT })].toSorted()
    const sourceFiles = [...new Bun.Glob('src/**/*.{ts,tsx}').scanSync({ cwd: WEB_ROOT })]
    const read = (path: string) => readFileSync(join(WEB_ROOT, path), 'utf8')

    test('trace 라우트는 업로드 POST 하나와 보관량 GET 둘뿐이다', async () => {
        const traceRoutes = routeFiles.filter((path) => read(path).includes('createTraceRoutes'))
        expect(traceRoutes).toEqual([
            'src/app/api/admin/traces/usage/route.ts',
            'src/app/api/charts/[hash]/traces/route.ts',
            'src/app/api/fe/me/traces/usage/route.ts',
        ])
        const exportsOf = async (path: string) => Object.keys(await import(join(WEB_ROOT, path))).toSorted()
        expect(await Promise.all(traceRoutes.map(exportsOf))).toEqual([['GET'], ['POST'], ['GET']])
    })

    test('play_trace·play_analysis 표는 compose 만 읽고, 저장된 trace 를 푸는 곳은 서비스뿐이다', () => {
        const tableUsers = sourceFiles.filter((path) => /\b(playTrace|playAnalysis)\b/.test(read(path)) && !path.endsWith('db/schema.ts'))
        expect(tableUsers.map((path) => relative('src', path))).toEqual(['server/compose/trace.ts'])
        const readers = sourceFiles.filter((path) => /\b(findStoredTrace|readAnalysis)\b/.test(read(path)))
        expect(readers.toSorted()).toEqual(['src/server/compose/trace.ts', 'src/server/service/domain/trace/trace.service.ts'])
        expect(read('src/server/route/trace.ts')).toContain("Pick<TraceService, 'upload' | 'usageOf' | 'adminUsage'>")
    })
})

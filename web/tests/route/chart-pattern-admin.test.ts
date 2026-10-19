import { beforeEach, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join, relative } from 'node:path'
import { CHART_PATTERN_MAX_BYTES, CHART_PATTERN_RECOMPUTE_BATCH } from '@server/dto/chart-pattern'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createAdminChartPatternRoutes } from '@server/route/admin'
import { createChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { FIXTURE_MD5, createMemoryChartPatternDb, makePatternDocument, type FixtureNote } from '../support/chart-pattern-fixture'

const ADMIN: AuthUser = { id: 'user-admin', loginId: 'admin', name: 'Admin', role: 'admin' }
const PLAYER: AuthUser = { id: 'user-player', loginId: 'player', name: 'Player', role: 'user' }
const TOKENS = new Map([
    ['rbms_admin_token', ADMIN],
    ['rbms_player_token', PLAYER],
])
const ORIGIN = 'http://localhost:3000'
const NOTES: FixtureNote[] = Array.from({ length: 64 }, (_, index): FixtureNote => [index * 100_000, index % 8, 0])
const WEB_ROOT = join(import.meta.dirname, '..', '..')

const createRoutes = () => {
    const memory = createMemoryChartPatternDb()
    const chartPatternService = createChartPatternService({
        db: memory.db,
        now: () => 0,
        listCoverageTables: () => [{ id: 'insane1', name: 'insane', md5s: [FIXTURE_MD5, 'c'.repeat(32)] }],
    })
    const routes = createAdminChartPatternRoutes(
        () => ({ authService: { resolveApiTokenUser: async (plain) => TOKENS.get(plain) ?? null }, chartPatternService }),
        async () => undefined,
    )
    return { routes, memory }
}

const putRequest = (md5: string, body: unknown, headers: Record<string, string> = {}) =>
    new Request(`${ORIGIN}/api/admin/chart-patterns/${md5}`, {
        method: 'PUT',
        headers: { 'content-type': 'application/json', ...headers },
        body: typeof body === 'string' ? body : JSON.stringify(body),
    })

const md5Context = (md5: string) => ({ params: Promise.resolve({ md5 }) })

const bearer = (token: string) => ({ authorization: `Bearer ${token}` })

beforeEach(() => resetRateLimits())

describe('관리자 Bearer(withAdminBearer)', () => {
    test('토큰이 없거나 세션 쿠키만 있거나 모르는 토큰이면 401, 관리자가 아닌 사용자의 토큰이면 403 이고 저장하지 않는다', async () => {
        const { routes, memory } = createRoutes()
        const document = makePatternDocument(NOTES)
        const cases: { headers: Record<string, string>; status: number }[] = [
            { headers: {}, status: 401 },
            { headers: { cookie: 'better-auth.session_token=admin-session' }, status: 401 },
            { headers: bearer('rbms_unknown'), status: 401 },
            { headers: bearer('rbms_player_token'), status: 403 },
        ]
        for (const { headers, status } of cases) {
            const response = await routes.put(putRequest(FIXTURE_MD5, document, headers), md5Context(FIXTURE_MD5))
            expect(response.status).toBe(status)
            const coverage = await routes.coverage(new Request(`${ORIGIN}/api/admin/chart-patterns/coverage`, { headers }), undefined)
            expect(coverage.status).toBe(status)
        }
        expect(memory.rows.size).toBe(0)
    })

    test('역할은 요청마다 토큰 사용자에서 읽는다: 관리자에서 내려가면 같은 토큰이 바로 403 이다', async () => {
        const { routes } = createRoutes()
        const document = makePatternDocument(NOTES)
        expect((await routes.put(putRequest(FIXTURE_MD5, document, bearer('rbms_admin_token')), md5Context(FIXTURE_MD5))).status).toBe(201)
        TOKENS.set('rbms_admin_token', { ...ADMIN, role: 'user' })
        try {
            expect((await routes.put(putRequest(FIXTURE_MD5, document, bearer('rbms_admin_token')), md5Context(FIXTURE_MD5))).status).toBe(403)
        } finally {
            TOKENS.set('rbms_admin_token', ADMIN)
        }
    })
})

describe('PUT /api/admin/chart-patterns/[md5]', () => {
    test('처음은 201 created, 같은 것을 다시 올리면 200 unchanged, parser_version 이 다르면 200 replaced 이고 응답에 배치가 없다', async () => {
        const { routes } = createRoutes()
        const document = makePatternDocument(NOTES)
        const created = await routes.put(putRequest(FIXTURE_MD5, document, bearer('rbms_admin_token')), md5Context(FIXTURE_MD5))
        expect(created.status).toBe(201)
        expect(created.headers.get('cache-control')).toBe('private, no-store')
        const createdText = await created.text()
        expect(JSON.parse(createdText).data).toMatchObject({ md5: FIXTURE_MD5, outcome: 'created', note_count: NOTES.length })
        expect(createdText).not.toContain('time_us')
        const unchanged = await routes.put(
            putRequest(FIXTURE_MD5, JSON.stringify(document, null, 2), bearer('rbms_admin_token')),
            md5Context(FIXTURE_MD5),
        )
        expect(unchanged.status).toBe(200)
        expect((await unchanged.json()).data.outcome).toBe('unchanged')
        const replaced = await routes.put(
            putRequest(FIXTURE_MD5, { ...document, parser_version: 6 }, bearer('rbms_admin_token')),
            md5Context(FIXTURE_MD5),
        )
        expect(replaced.status).toBe(200)
        expect((await replaced.json()).data.outcome).toBe('replaced')
    })

    test('발광 표 밖의 md5 도 받는다(ADR Q9 B)', async () => {
        const { routes, memory } = createRoutes()
        const outside = 'd'.repeat(32)
        const response = await routes.put(
            putRequest(outside, makePatternDocument(NOTES, { md5: outside }), bearer('rbms_admin_token')),
            md5Context(outside),
        )
        expect(response.status).toBe(201)
        expect(memory.rows.has(outside)).toBe(true)
    })

    test('형식이 틀린 본문·경로와 본문의 md5 가 다른 요청·잘못된 경로는 400 이고 저장하지 않는다', async () => {
        const { routes, memory } = createRoutes()
        const document = makePatternDocument(NOTES)
        const wrongLane = { ...document, notes: { ...document.notes, lane: document.notes.lane.map(() => 9) } }
        const responses = [
            await routes.put(putRequest(FIXTURE_MD5, wrongLane, bearer('rbms_admin_token')), md5Context(FIXTURE_MD5)),
            await routes.put(putRequest(FIXTURE_MD5, '{not json', bearer('rbms_admin_token')), md5Context(FIXTURE_MD5)),
            await routes.put(putRequest('e'.repeat(32), document, bearer('rbms_admin_token')), md5Context('e'.repeat(32))),
            await routes.put(putRequest('NOT-A-MD5', document, bearer('rbms_admin_token')), md5Context('NOT-A-MD5')),
        ]
        expect(responses.map((response) => response.status)).toEqual([400, 400, 400, 400])
        const bodies = await Promise.all(responses.map((response) => response.json()))
        expect(bodies.map((body) => body.error.code)).toEqual([
            'VALIDATION_ERROR',
            'VALIDATION_ERROR',
            'IR_CHART_PATTERN_INVALID',
            'VALIDATION_ERROR',
        ])
        expect(bodies[0].error.details.field).toBe('notes.lane.0')
        expect(bodies[2].error.details).toMatchObject({ field: 'md5', reason: 'md5_mismatch' })
        expect(bodies[3].error.details.field).toBe('md5')
        expect(memory.rows.size).toBe(0)
    })

    test('본문이 CHART_PATTERN_MAX_BYTES 를 넘으면 413 이다', async () => {
        const { routes } = createRoutes()
        const response = await routes.put(
            putRequest(FIXTURE_MD5, '{}', { ...bearer('rbms_admin_token'), 'content-length': String(CHART_PATTERN_MAX_BYTES + 1) }),
            md5Context(FIXTURE_MD5),
        )
        expect(response.status).toBe(413)
    })
})

describe('GET /api/admin/chart-patterns/coverage', () => {
    test('관리자 토큰으로 표 md5 수와 있는 수만 돌려준다', async () => {
        const { routes } = createRoutes()
        await routes.put(putRequest(FIXTURE_MD5, makePatternDocument(NOTES), bearer('rbms_admin_token')), md5Context(FIXTURE_MD5))
        const response = await routes.coverage(
            new Request(`${ORIGIN}/api/admin/chart-patterns/coverage`, { headers: bearer('rbms_admin_token') }),
            undefined,
        )
        expect(response.status).toBe(200)
        const text = await response.text()
        expect(JSON.parse(text).data).toMatchObject({
            tables: [{ table_id: 'insane1', table_md5s: 2, covered: 1, missing: ['c'.repeat(32)] }],
            patterns_total: 1,
        })
        expect(text).not.toContain('time_us')
    })
})

const recomputeRequest = (body: unknown, headers: Record<string, string> = {}) =>
    new Request(`${ORIGIN}/api/admin/chart-patterns/recompute`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', ...headers },
        body: typeof body === 'string' ? body : JSON.stringify(body),
    })

const md5Of = (index: number) => index.toString(16).padStart(32, '0')

describe('POST /api/admin/chart-patterns/recompute', () => {
    const seedStale = async (count: number) => {
        const { routes, memory } = createRoutes()
        for (let index = 0; index < count; index += 1) {
            await routes.put(
                putRequest(md5Of(index), makePatternDocument(NOTES, { md5: md5Of(index) }), bearer('rbms_admin_token')),
                md5Context(md5Of(index)),
            )
            const row = memory.rows.get(md5Of(index))
            if (row) memory.rows.set(md5Of(index), { ...row, featuresVersion: ANALYSIS_VERSION - 1 })
        }
        return { routes, memory }
    }

    test('토큰이 없거나 관리자가 아니면 401·403 이고 아무것도 다시 계산하지 않는다', async () => {
        const { routes, memory } = await seedStale(2)
        expect((await routes.recompute(recomputeRequest({}), undefined)).status).toBe(401)
        expect((await routes.recompute(recomputeRequest({}, bearer('rbms_player_token')), undefined)).status).toBe(403)
        expect([...memory.rows.values()].every((row) => row.featuresVersion === ANALYSIS_VERSION - 1)).toBe(true)
    })

    test('관리자 토큰이면 배치 크기만큼 다시 계산해 개수와 다음 시작점만 답하고, 기본 배치 크기가 있다', async () => {
        const { routes, memory } = await seedStale(3)
        const first = await routes.recompute(recomputeRequest({ limit: 2 }, bearer('rbms_admin_token')), undefined)
        expect(first.status).toBe(200)
        expect(first.headers.get('cache-control')).toBe('private, no-store')
        const text = await first.text()
        expect(JSON.parse(text).data).toEqual({
            analysis_version: ANALYSIS_VERSION,
            examined: 2,
            recomputed: 2,
            skipped: [],
            next_after: md5Of(1),
            remaining: 1,
        })
        expect(text).not.toContain('time_us')
        const rest = await routes.recompute(recomputeRequest({ after: md5Of(1) }, bearer('rbms_admin_token')), undefined)
        expect((await rest.json()).data).toMatchObject({ examined: 1, recomputed: 1, next_after: null, remaining: 0 })
        expect(CHART_PATTERN_RECOMPUTE_BATCH.DEFAULT).toBeGreaterThan(1)
        expect(memory.calls.updateFeatures).toBe(3)
    })

    test('배치 크기가 범위 밖이거나 시작점이 md5 가 아니면 400 이다', async () => {
        const { routes, memory } = await seedStale(1)
        const bad = [{ limit: 0 }, { limit: CHART_PATTERN_RECOMPUTE_BATCH.MAX + 1 }, { limit: 1.5 }, { after: 'NOT-A-MD5' }, '{not json']
        for (const body of bad) {
            const response = await routes.recompute(recomputeRequest(body, bearer('rbms_admin_token')), undefined)
            expect(response.status).toBe(400)
        }
        expect(memory.calls.updateFeatures).toBe(0)
    })
})

describe('패턴 읽기 공개 경로 없음', () => {
    const routeFiles = [...new Bun.Glob('src/app/**/route.ts').scanSync({ cwd: WEB_ROOT })].toSorted()
    const sourceFiles = [...new Bun.Glob('src/**/*.{ts,tsx}').scanSync({ cwd: WEB_ROOT })]
    const read = (path: string) => readFileSync(join(WEB_ROOT, path), 'utf8')

    test('패턴 라우트는 PUT 하나, coverage GET 하나, recompute POST 하나뿐이다', async () => {
        const patternRoutes = routeFiles.filter((path) => read(path).includes('createAdminChartPatternRoutes'))
        expect(patternRoutes).toEqual([
            'src/app/api/admin/chart-patterns/[md5]/route.ts',
            'src/app/api/admin/chart-patterns/coverage/route.ts',
            'src/app/api/admin/chart-patterns/recompute/route.ts',
        ])
        const exportsOf = async (path: string) => Object.keys(await import(join(WEB_ROOT, path))).toSorted()
        expect(await exportsOf(patternRoutes[0])).toEqual(['PUT'])
        expect(await exportsOf(patternRoutes[1])).toEqual(['GET'])
        expect(await exportsOf(patternRoutes[2])).toEqual(['POST'])
    })

    test('chart_pattern 표는 compose 만 읽고, 저장된 배치를 읽는 함수는 어떤 라우트·화면도 부르지 않는다(trace 의 패턴 정합 검사만 compose 에서 받는다)', () => {
        const tableUsers = sourceFiles.filter((path) => /\bchartPattern\b/.test(read(path)) && !path.endsWith('db/schema.ts'))
        expect(tableUsers.map((path) => relative('src', path))).toEqual(['server/compose/chart-pattern.ts'])
        const readers = sourceFiles.filter((path) => /\b(readPattern|readFeatures|findStored)\b/.test(read(path)))
        expect(readers.toSorted()).toEqual([
            'src/server/compose/chart-pattern.ts',
            'src/server/compose/trace.ts',
            'src/server/service/domain/chart-pattern/chart-pattern.service.ts',
        ])
        expect(read('src/server/compose/trace.ts')).toContain("chartPatternService: Pick<ChartPatternService, 'readPattern'>")
        const routeDeps = read('src/server/route/admin.ts')
        expect(routeDeps).toContain("Pick<ChartPatternService, 'upload' | 'coverage' | 'recomputeStale'>")
    })
})

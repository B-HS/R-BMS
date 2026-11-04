import { beforeEach, describe, expect, test } from 'bun:test'
import { trainingChartDetailSchema, trainingChartListSchema } from '@server/dto/training'
import type { AuthUser } from '@server/lib/auth-context'
import { RATE_LIMIT, resetRateLimits } from '@server/lib/with-rate-limit'
import { createTrainingDownloadRoute } from '@server/route/training'
import { createAiTrainingChartRoutes } from '@server/route/training-charts'
import { TRAINING_OTHER_USER, TRAINING_USER, createTrainingHarness } from '../support/training-fixture'

const ALICE: AuthUser = { id: TRAINING_USER, loginId: 'alice', name: 'Alice', role: 'user' }
const BOB: AuthUser = { id: TRAINING_OTHER_USER, loginId: 'bob', name: 'Bob', role: 'user' }
const ORIGIN = 'http://localhost:3000'
const CHARTS_PATH = '/api/fe/me/ai/training-charts'

const setup = async () => {
    const harness = createTrainingHarness()
    await harness.registerKey()
    const deps = {
        authService: {
            resolveSessionUser: async (request: Request) =>
                [ALICE, BOB].find((user) => user.loginId === request.headers.get('x-session-user')) ?? null,
        },
        trainingChartService: harness.service,
    }
    const noWait = async () => undefined
    const routes = createAiTrainingChartRoutes(
        () => deps,
        noWait,
        async () => 'ko' as const,
    )
    const download = createTrainingDownloadRoute(() => deps, noWait)
    return { harness, routes, download }
}

const call = (method: string, path: string, init: { user?: AuthUser; body?: unknown; headers?: Record<string, string> } = {}) =>
    new Request(`${ORIGIN}${path}`, {
        method,
        headers: {
            'content-type': 'application/json',
            ...(init.user ? { 'x-session-user': init.user.loginId } : {}),
            ...init.headers,
        },
        body: init.body === undefined ? undefined : JSON.stringify(init.body),
    })

const idContext = (id: string) => ({ params: Promise.resolve({ id }) })

const createChart = async (routes: Awaited<ReturnType<typeof setup>>['routes']) => {
    const response = await routes.create(call('POST', CHARTS_PATH, { user: ALICE, body: { focus_key: 'auto', intensity: 'push' } }), undefined)
    expect(response.status).toBe(201)
    return trainingChartDetailSchema.parse((await response.json()).data)
}

beforeEach(() => resetRateLimits())

describe('훈련곡 API 의 인증과 요청 검사', () => {
    test('세션이 없으면 다섯 경로가 모두 401 이다', async () => {
        const { routes } = await setup()
        const id = `tc_${'0'.repeat(32)}`
        expect((await routes.list(call('GET', CHARTS_PATH), undefined)).status).toBe(401)
        expect((await routes.create(call('POST', CHARTS_PATH, { body: { focus_key: 'auto', intensity: 'push' } }), undefined)).status).toBe(401)
        expect((await routes.detail(call('GET', `${CHARTS_PATH}/${id}`), idContext(id))).status).toBe(401)
        expect((await routes.removal(call('GET', `${CHARTS_PATH}/${id}/removal`), idContext(id))).status).toBe(401)
        expect((await routes.remove(call('DELETE', `${CHARTS_PATH}/${id}`), idContext(id))).status).toBe(401)
    })

    test('focus_key 65자·모르는 접두사·잘못된 태그·다른 필드는 400 이고 공급자를 부르지 않는다', async () => {
        const { routes, harness } = await setup()
        for (const body of [
            { focus_key: `tag:${'a'.repeat(61)}`, intensity: 'push' },
            { focus_key: 'weird:trill', intensity: 'push' },
            { focus_key: 'tag:nope', intensity: 'push' },
            { focus_key: 'auto', intensity: 'push', extra: 1 },
        ]) {
            const response = await routes.create(call('POST', CHARTS_PATH, { user: ALICE, body }), undefined)
            expect(response.status).toBe(400)
            expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
        }
        expect(harness.anthropic.generateCalls).toEqual([])
    })

    test('문법은 맞지만 프로필에 없는 키는 422 weakness_profile 이고 공급자 호출이 없다', async () => {
        const { routes, harness } = await setup()
        const response = await routes.create(
            call('POST', CHARTS_PATH, { user: ALICE, body: { focus_key: 'tag:stair', intensity: 'push' } }),
            undefined,
        )
        expect(response.status).toBe(422)
        expect((await response.json()).error).toMatchObject({ code: 'AI_NOT_ENOUGH_DATA', details: { need: 'weakness_profile' } })
        expect(harness.anthropic.generateCalls).toEqual([])
    })

    test('키가 없으면 404 AI_CREDENTIAL_NOT_FOUND, 20곡이면 409 TRAINING_CHART_LIMIT 이다', async () => {
        const { routes, harness } = await setup()
        const noKey = await routes.create(call('POST', CHARTS_PATH, { user: BOB, body: { focus_key: 'auto', intensity: 'push' } }), undefined)
        expect(noKey.status).toBe(404)
        expect((await noKey.json()).error.code).toBe('AI_CREDENTIAL_NOT_FOUND')
        for (let index = 0; index < 20; index += 1) {
            harness.state.now += 1
            expect((await harness.create({ focus_key: 'auto', intensity: 'push' })).ok).toBe(true)
        }
        const full = await routes.create(call('POST', CHARTS_PATH, { user: ALICE, body: { focus_key: 'auto', intensity: 'push' } }), undefined)
        expect(full.status).toBe(409)
        expect((await full.json()).error).toMatchObject({ code: 'TRAINING_CHART_LIMIT', details: { limit: 20, current: 20 } })
    })
})

describe('훈련곡 목록·상세·지우기', () => {
    test('만들면 201 상세를 private, no-store 로 주고 주소는 요청 origin 이다', async () => {
        const { routes } = await setup()
        const response = await routes.create(call('POST', CHARTS_PATH, { user: ALICE, body: { focus_key: 'auto', intensity: 'push' } }), undefined)
        expect(response.status).toBe(201)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        const detail = trainingChartDetailSchema.parse((await response.json()).data)
        expect(detail.download_url).toBe(`${ORIGIN}/api/training/${detail.id}/chart.zip`)
        const list = await routes.list(call('GET', CHARTS_PATH, { user: ALICE }), undefined)
        expect(trainingChartListSchema.parse((await list.json()).data).charts.map((chart) => chart.id)).toEqual([detail.id])
    })

    test('ALICE 의 훈련곡 id 로 BOB 이 상세·removal·DELETE 를 부르면 셋 다 404 이고 행이 남는다', async () => {
        const { routes, harness } = await setup()
        const chart = await createChart(routes)
        const path = `${CHARTS_PATH}/${chart.id}`
        for (const response of [
            await routes.detail(call('GET', path, { user: BOB }), idContext(chart.id)),
            await routes.removal(call('GET', `${path}/removal`, { user: BOB }), idContext(chart.id)),
            await routes.remove(call('DELETE', path, { user: BOB }), idContext(chart.id)),
        ]) {
            expect(response.status).toBe(404)
            expect((await response.json()).error.code).toBe('TRAINING_CHART_NOT_FOUND')
        }
        expect(harness.training.charts.has(chart.id)).toBe(true)
        expect((await routes.detail(call('GET', path, { user: ALICE }), idContext(chart.id))).status).toBe(200)
    })

    test('형식이 아닌 id 는 404 이고, removal 의 개수를 지운 뒤 같은 모양에 removed 를 더해 준다', async () => {
        const { routes } = await setup()
        expect((await routes.detail(call('GET', `${CHARTS_PATH}/nope`, { user: ALICE }), idContext('nope'))).status).toBe(404)
        const chart = await createChart(routes)
        const path = `${CHARTS_PATH}/${chart.id}`
        const removal = (await (await routes.removal(call('GET', `${path}/removal`, { user: ALICE }), idContext(chart.id))).json()).data
        expect(removal).toEqual({ removes: { training_chart: 1, folder_items: 1 }, keeps: { chart_pattern: 0, scores: 0, traces: 0, replays: 0 } })
        const removed = await routes.remove(call('DELETE', path, { user: ALICE }), idContext(chart.id))
        expect(removed.status).toBe(200)
        expect((await removed.json()).data).toEqual({ ...removal, removed: true })
        expect((await routes.detail(call('GET', path, { user: ALICE }), idContext(chart.id))).status).toBe(404)
    })

    test('분당 한도를 넘으면 429 이다', async () => {
        const { routes } = await setup()
        for (let index = 0; index < RATE_LIMIT.TABLE_RECORDS.limit; index += 1) {
            expect((await routes.list(call('GET', CHARTS_PATH, { user: ALICE }), undefined)).status).toBe(200)
        }
        expect((await routes.list(call('GET', CHARTS_PATH, { user: ALICE }), undefined)).status).toBe(429)
    })
})

describe('GET /api/training/[id]/chart.zip', () => {
    test('세션 없이 id 만으로 zip 을 주고 헤더가 맞다', async () => {
        const { routes, download } = await setup()
        const chart = await createChart(routes)
        const response = await download.download(call('GET', `/api/training/${chart.id}/chart.zip`), idContext(chart.id))
        expect(response.status).toBe(200)
        expect(response.headers.get('content-type')).toBe('application/zip')
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        expect(response.headers.get('content-disposition')).toBe(`attachment; filename="rbms-training-${chart.id.slice(3, 11)}.zip"`)
        const bytes = Buffer.from(await response.arrayBuffer())
        expect(response.headers.get('content-length')).toBe(String(bytes.length))
        expect(bytes.readUInt32LE(0)).toBe(0x04034b50)
    })

    test('형식이 아닌 id·없는 id·지운 곡은 404 이다', async () => {
        const { routes, download } = await setup()
        for (const id of ['tc_ABC', `tc_${'0'.repeat(31)}`, '../etc', `tc_${'f'.repeat(32)}`]) {
            const response = await download.download(call('GET', `/api/training/${id}/chart.zip`), idContext(id))
            expect(response.status).toBe(404)
        }
        const chart = await createChart(routes)
        await routes.remove(call('DELETE', `${CHARTS_PATH}/${chart.id}`, { user: ALICE }), idContext(chart.id))
        const gone = await download.download(call('GET', `/api/training/${chart.id}/chart.zip`), idContext(chart.id))
        expect(gone.status).toBe(404)
        expect((await gone.json()).error.code).toBe('TRAINING_CHART_NOT_FOUND')
    })

    test('IP 당 분당 30회를 넘으면 429 이다', async () => {
        const { download } = await setup()
        const id = `tc_${'f'.repeat(32)}`
        const request = () => call('GET', `/api/training/${id}/chart.zip`, { headers: { 'x-forwarded-for': '203.0.113.9' } })
        for (let index = 0; index < RATE_LIMIT.TRAINING_DOWNLOAD.limit; index += 1) {
            expect((await download.download(request(), idContext(id))).status).toBe(404)
        }
        expect((await download.download(request(), idContext(id))).status).toBe(429)
    })
})

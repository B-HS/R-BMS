import { afterEach, beforeEach, describe, expect, spyOn, test } from 'bun:test'
import type { SubmitResponseOutput } from '@server/dto/score'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createMyDanDisplayRoute, createMyDansRoute, type DanRouteDeps } from '@server/route/dan'
import { createCourseSubmitRoute, type CourseSubmitRouteDeps } from '@server/route/course'
import { createDanService, type DanService } from '@server/service/domain/dan/dan.service'
import { createMemoryDanDb } from '../support/dan-memory-db'
import { danRun, FIRST_DAN_COURSE, SECOND_DAN_COURSE } from '../support/dan-fixture'

const ALICE: AuthUser = { id: 'user-alice', loginId: 'alice', name: 'Alice', role: 'user' }
const BOB: AuthUser = { id: 'user-bob', loginId: 'bob', name: 'Bob', role: 'user' }
const USERS = new Map([ALICE, BOB].map((user) => [user.loginId, user]))
const ORIGIN = 'http://localhost:3000'
const TOKEN = 'rbms_test_token'
const SYSTEM_COUNT = 4
const OVERSIZED_BODY_BYTES = 4096
const SUBMITTED_PLAYED_AT = 1_760_000_000_000
const SECRET_QUERY = 'insert into user_dan values (secret-value)'

beforeEach(() => resetRateLimits())

const noWait = async () => undefined

const createDanRoutes = () => {
    const memory = createMemoryDanDb({ courses: [FIRST_DAN_COURSE, SECOND_DAN_COURSE] })
    const danService = createDanService({ db: memory.db })
    const revalidated: string[] = []
    const deps: DanRouteDeps = {
        authService: { resolveSessionUser: async (request) => USERS.get(request.headers.get('x-session-user') ?? '') ?? null },
        danService,
        revalidate: { onDanAward: ({ loginId }) => void revalidated.push(loginId) },
    }
    return { list: createMyDansRoute(() => deps, noWait), setDisplay: createMyDanDisplayRoute(() => deps), danService, revalidated }
}

const call = (method: string, path: string, init: { user?: AuthUser; body?: unknown; rawBody?: string } = {}) =>
    new Request(`${ORIGIN}${path}`, {
        method,
        headers: { 'content-type': 'application/json', ...(init.user ? { 'x-session-user': init.user.loginId } : {}) },
        body: init.rawBody ?? (init.body === undefined ? undefined : JSON.stringify(init.body)),
    })

const unauthorized = async (response: Response) => {
    expect(response.status).toBe(401)
    expect((await response.json()).error.code).toBe('UNAUTHORIZED')
}

describe('GET·PUT /api/fe/me/dans', () => {
    test('세션이 없으면 401 이다', async () => {
        const routes = createDanRoutes()
        await unauthorized(await routes.list(call('GET', '/api/fe/me/dans'), undefined))
        await unauthorized(await routes.setDisplay(call('PUT', '/api/fe/me/dans/display', { body: { system: 'stella', mode: 'auto' } }), undefined))
    })

    test('GET 은 네 체계를 모두 돌려주고 캐시하지 않는다', async () => {
        const routes = createDanRoutes()
        const response = await routes.list(call('GET', '/api/fe/me/dans', { user: ALICE }), undefined)
        expect(response.status).toBe(200)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        const body = await response.json()
        expect(body.success).toBe(true)
        expect(body.data.systems).toHaveLength(SYSTEM_COUNT)
    })

    test('PUT 은 잘못된 mode 나 체계, label 이 빠진 본문을 400 으로 거절한다', async () => {
        const routes = createDanRoutes()
        const invalid = [
            { system: 'stella', mode: 'everyone' },
            { system: 'unknown', mode: 'auto' },
            { system: 'stella', mode: 'label' },
            { system: 'stella', mode: 'label', label: '' },
            {},
        ]
        for (const body of invalid) {
            const response = await routes.setDisplay(call('PUT', '/api/fe/me/dans/display', { user: ALICE, body }), undefined)
            expect(response.status).toBe(400)
            expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
        }
        expect(routes.revalidated).toEqual([])
    })

    test('PUT 은 너무 큰 본문을 413 으로 거절한다', async () => {
        const routes = createDanRoutes()
        const rawBody = JSON.stringify({ system: 'stella', mode: 'label', label: 'x'.repeat(OVERSIZED_BODY_BYTES) })
        const response = await routes.setDisplay(call('PUT', '/api/fe/me/dans/display', { user: ALICE, rawBody }), undefined)
        expect(response.status).toBe(413)
    })

    test('PUT 뒤 GET 의 shown 이 바뀌고 프로필 캐시가 갱신된다', async () => {
        const routes = createDanRoutes()
        await routes.danService.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE.id }))
        await routes.danService.awardFromRun(danRun(SECOND_DAN_COURSE, { id: 'cs_2', userId: ALICE.id }))
        const shownOf = async () => {
            const body = await (await routes.list(call('GET', '/api/fe/me/dans', { user: ALICE }), undefined)).json()
            return body.data.systems[0].shown
        }
        expect(await shownOf()).toBe('二段')

        const put = await routes.setDisplay(
            call('PUT', '/api/fe/me/dans/display', { user: ALICE, body: { system: 'genoside2018_sp', mode: 'label', label: '初段' } }),
            undefined,
        )
        expect(put.status).toBe(200)
        expect((await put.json()).data.systems[0]).toMatchObject({ shown: '初段', display: { mode: 'label', label: '初段' } })
        expect(await shownOf()).toBe('初段')
        expect(routes.revalidated).toEqual(['alice'])
    })

    test('PUT 으로 획득하지 않은 label 을 고르면 400 이고 캐시를 갱신하지 않는다', async () => {
        const routes = createDanRoutes()
        await routes.danService.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE.id }))
        const response = await routes.setDisplay(
            call('PUT', '/api/fe/me/dans/display', { user: ALICE, body: { system: 'genoside2018_sp', mode: 'label', label: '二段' } }),
            undefined,
        )
        expect(response.status).toBe(400)
        expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
        expect(routes.revalidated).toEqual([])
    })

    test('다른 사용자의 표시 선택은 서로 섞이지 않는다', async () => {
        const routes = createDanRoutes()
        await routes.danService.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE.id }))
        await routes.danService.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_2', userId: BOB.id }))
        await routes.setDisplay(
            call('PUT', '/api/fe/me/dans/display', { user: ALICE, body: { system: 'genoside2018_sp', mode: 'hidden' } }),
            undefined,
        )
        const bob = await (await routes.list(call('GET', '/api/fe/me/dans', { user: BOB }), undefined)).json()
        expect(bob.data.systems[0]).toMatchObject({ shown: '初段', display: { mode: 'auto' } })
    })
})

describe('POST /api/courses 의 단위 판정', () => {
    const SCORE_ID = 'cs_route_1'
    const RESPONSE: SubmitResponseOutput = {
        accepted: true,
        rank: 1,
        previous_best: null,
        message: 'saved',
        ranked: true,
        flags: [],
        is_new_best: true,
        score_id: SCORE_ID,
    }
    const scoreRow = danRun(FIRST_DAN_COURSE, { id: SCORE_ID, userId: ALICE.id })

    const submissionBody = {
        course_hash: FIRST_DAN_COURSE.courseHash,
        player: { id: ALICE.loginId },
        clear: 'Normal',
        ex_score: 0,
        judge: { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 },
        played_at: SUBMITTED_PLAYED_AT,
    }

    const request = () =>
        new Request(`${ORIGIN}/api/courses`, {
            method: 'POST',
            headers: { 'content-type': 'application/json', authorization: `Bearer ${TOKEN}` },
            body: JSON.stringify(submissionBody),
        })

    const createSubmit = (awardFromRun: DanService['awardFromRun'], options: { duplicated?: boolean } = {}) => {
        const events: string[] = []
        const awardedRuns: unknown[] = []
        const deps: CourseSubmitRouteDeps = {
            authService: { resolveApiTokenUser: async (token) => (token === TOKEN ? ALICE : null) },
            courseService: {
                submit: async () => ({
                    courseScoreId: SCORE_ID,
                    duplicated: options.duplicated ?? false,
                    ranked: true,
                    flags: [],
                    response: RESPONSE,
                    scoreRow,
                }),
            },
            scoreService: { recordRejection: async () => undefined, recordAcceptance: async () => undefined },
            resolveBuildTrust: async () => 'trusted',
            danService: {
                awardFromRun: async (run) => {
                    awardedRuns.push(run)
                    return awardFromRun(run)
                },
            },
            revalidate: {
                onCourseSubmit: ({ courseHash }) => void events.push(`course:${courseHash}`),
                onDanAward: ({ loginId }) => void events.push(`dan:${loginId}`),
            },
        }
        return { submit: createCourseSubmitRoute(() => deps), events, awardedRuns }
    }

    const noContext = { params: Promise.resolve({}) }

    let errorLog: ReturnType<typeof spyOn<typeof console, 'error'>>

    beforeEach(() => {
        errorLog = spyOn(console, 'error').mockImplementation(() => undefined)
    })

    afterEach(() => errorLog.mockRestore())

    test('단위 판정이 던져도 코스 제출은 201 이고 판정 실패는 한 번 로그에 남는다', async () => {
        const { submit, events } = createSubmit(() => Promise.reject(new Error(SECRET_QUERY)))
        const response = await submit(request(), noContext)
        expect(response.status).toBe(201)
        expect((await response.json()).score_id).toBe(SCORE_ID)
        expect(errorLog).toHaveBeenCalledTimes(1)
        expect(errorLog.mock.calls[0][0]).toBe('dan award failed')
        expect(events).toEqual([`course:${FIRST_DAN_COURSE.courseHash}`])
    })

    test('판정 실패 로그에는 오류의 종류만 있고 메시지는 없다', async () => {
        const { submit } = createSubmit(() => Promise.reject(new Error(SECRET_QUERY)))
        await submit(request(), noContext)
        expect(JSON.stringify(errorLog.mock.calls)).not.toContain('secret-value')
        expect(errorLog.mock.calls[0][1]).toEqual({ error_name: 'Error', error_code: null })
    })

    test('저장된 판 그대로를 판정에 넘기고, 단위를 새로 얻으면 프로필 캐시를 갱신한다', async () => {
        const { submit, events, awardedRuns } = createSubmit(async () => ({ awarded: FIRST_DAN_COURSE.dan, reason: null }))
        expect((await submit(request(), noContext)).status).toBe(201)
        expect(awardedRuns).toEqual([scoreRow])
        expect(awardedRuns[0]).toBe(scoreRow)
        expect(events).toEqual(['dan:alice', `course:${FIRST_DAN_COURSE.courseHash}`])
        expect(errorLog).not.toHaveBeenCalled()
    })

    test('단위를 얻지 못한 판은 프로필 캐시를 갱신하지 않는다', async () => {
        const { submit, events } = createSubmit(async () => ({ awarded: null, reason: 'not_cleared' }))
        expect((await submit(request(), noContext)).status).toBe(201)
        expect(events).toEqual([`course:${FIRST_DAN_COURSE.courseHash}`])
    })

    test('같은 판의 재전송도 판정을 부르고 응답은 200 이다', async () => {
        const { submit, awardedRuns } = createSubmit(async () => ({ awarded: null, reason: null }), { duplicated: true })
        expect((await submit(request(), noContext)).status).toBe(200)
        expect(awardedRuns).toHaveLength(1)
    })

    test('인증에 실패한 제출은 판정하지 않는다', async () => {
        const { submit, awardedRuns } = createSubmit(async () => ({ awarded: null, reason: null }))
        const response = await submit(
            new Request(`${ORIGIN}/api/courses`, {
                method: 'POST',
                headers: { 'content-type': 'application/json', authorization: 'Bearer wrong' },
                body: JSON.stringify(submissionBody),
            }),
            noContext,
        )
        expect(response.status).toBe(401)
        expect(awardedRuns).toHaveLength(0)
    })
})

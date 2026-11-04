import { afterAll, beforeEach, describe, expect, mock, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import type { SessionUser } from '../../src/entities/auth.type'

const realServer = await import('next/server')
const realHeaders = await import('next/headers')
const realIntlServer = await import('next-intl/server')
const realSessionGate = await import('../../src/widgets/auth/session-gate')

const state = {
    isStubbing: true,
    cookie: '',
    user: null as SessionUser | null,
    readFails: false,
    sessionReads: 0,
}

const SESSION_COOKIE = 'better-auth.session_token=session-token'
const USER: SessionUser = { id: 'user-alice', login_id: 'alice', name: 'Alice', email: 'alice@example.test', role: 'user' }

mock.module('next/server', () => ({ ...realServer, connection: () => (state.isStubbing ? Promise.resolve() : realServer.connection()) }))
mock.module('next/headers', () => ({
    ...realHeaders,
    headers: () => (state.isStubbing ? Promise.resolve(new Headers({ cookie: state.cookie })) : realHeaders.headers()),
}))
mock.module('next-intl/server', () => ({
    ...realIntlServer,
    getTranslations: (...args: Parameters<typeof realIntlServer.getTranslations>) =>
        state.isStubbing ? Promise.resolve((key: string) => key) : realIntlServer.getTranslations(...args),
}))
mock.module('@widgets/auth/session-gate', () => ({
    ...realSessionGate,
    readSessionUser: async () => {
        if (!state.isStubbing) return realSessionGate.readSessionUser()
        state.sessionReads += 1
        if (state.readFails) throw new Error('database is down')
        return state.user
    },
}))
mock.module('@widgets/home/member-dashboard-content', () => ({
    MemberDashboardContent: ({ user }: { user: SessionUser }) => <p>member dashboard for {user.login_id}</p>,
}))
mock.module('@widgets/home/home-content', () => ({ HomeContent: () => <p>public home content</p> }))

const { HomeBranch } = await import('../../src/widgets/home/home-branch')

afterAll(() => {
    state.isStubbing = false
})

beforeEach(() => {
    state.cookie = ''
    state.user = null
    state.readFails = false
    state.sessionReads = 0
})

const renderBranch = async () => renderToStaticMarkup(await HomeBranch())

describe('홈의 손님/회원 분기', () => {
    test('세션 쿠키가 없으면 세션을 읽지 않고 손님 홈을 그린다', async () => {
        const markup = await renderBranch()
        expect(state.sessionReads).toBe(0)
        expect(markup).toContain('R-BMS 점수 서버')
        expect(markup).toContain('public home content')
        expect(markup).not.toContain('member dashboard')
    })

    test('세션 쿠키가 있고 세션이 읽히면 회원 대시보드를 그린다', async () => {
        state.cookie = SESSION_COOKIE
        state.user = USER
        const markup = await renderBranch()
        expect(state.sessionReads).toBe(1)
        expect(markup).toContain('member dashboard for alice')
        expect(markup).not.toContain('public home content')
        expect(markup).not.toContain('R-BMS 점수 서버')
    })

    test('세션 쿠키가 있어도 세션이 끝났으면 손님 홈을 그린다', async () => {
        state.cookie = SESSION_COOKIE
        const markup = await renderBranch()
        expect(state.sessionReads).toBe(1)
        expect(markup).toContain('public home content')
        expect(markup).not.toContain('member dashboard')
    })

    test('세션을 읽다 실패하면 오류 대신 손님 홈을 그린다', async () => {
        state.cookie = SESSION_COOKIE
        state.readFails = true
        const markup = await renderBranch()
        expect(state.sessionReads).toBe(1)
        expect(markup).toContain('public home content')
        expect(markup).not.toContain('member dashboard')
    })

    test('다른 이름의 쿠키만 있으면 손님으로 본다', async () => {
        state.cookie = 'theme=dark; rbms-theme=dark'
        state.user = USER
        const markup = await renderBranch()
        expect(state.sessionReads).toBe(0)
        expect(markup).toContain('public home content')
    })
})

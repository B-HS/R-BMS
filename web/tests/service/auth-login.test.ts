import { describe, expect, test } from 'bun:test'
import { loginSchema, sessionLoginSchema, sessionRegisterSchema } from '@server/dto/auth'
import { createAuthService, type AuthServiceDeps } from '@server/service/domain/auth/auth.service'

const KNOWN_USER = { id: 'user-1', email: 'kuro@example.com', name: '黒猫', loginId: 'kuro_neko', role: 'user' }
const CORRECT_PASSWORD = 'correct-horse'

const createFixture = () => {
    const issuedTokens: { userId: string; label: string }[] = []
    const deps: AuthServiceDeps = {
        db: {
            findUserByLoginId: async (loginId) => (loginId === KNOWN_USER.loginId ? KNOWN_USER : null),
            findUserById: async () => null,
            insertApiToken: async (row) => {
                issuedTokens.push({ userId: row.userId, label: row.label })
            },
            findApiToken: async () => null,
            touchApiToken: async () => {},
            revokeTokensForUser: async () => {},
        },
        provider: {
            signUpEmail: async ({ loginId, name }) => ({
                headers: new Headers({ 'set-cookie': 'session=new' }),
                user: { id: 'user-2', name, loginId, role: 'user' },
            }),
            signInEmail: async ({ email, password }) => {
                if (email !== KNOWN_USER.email || password !== CORRECT_PASSWORD) throw new Error('invalid credentials')
                return { headers: new Headers({ 'set-cookie': 'session=abc' }), user: KNOWN_USER }
            },
            getSessionUser: async () => null,
        },
        generateToken: () => 'plain-token',
        hashToken: (plain) => `hash:${plain}`,
        newId: (prefix) => `${prefix}id`,
        now: () => 0,
    }
    return { issuedTokens, service: createAuthService(deps) }
}

describe('authService.loginSession', () => {
    test('세션 쿠키만 돌려주고 API 토큰을 발급하지 않는다', async () => {
        const { issuedTokens, service } = createFixture()
        const result = await service.loginSession({ loginId: KNOWN_USER.loginId, password: CORRECT_PASSWORD })
        expect(result.user).toEqual({ id: 'user-1', loginId: 'kuro_neko', name: '黒猫', role: 'user' })
        expect(result.headers.get('set-cookie')).toBe('session=abc')
        expect(issuedTokens).toEqual([])
    })

    test('모르는 아이디와 틀린 비밀번호는 같은 UNAUTHORIZED 로 거절한다', async () => {
        const { issuedTokens, service } = createFixture()
        await expect(service.loginSession({ loginId: 'nobody', password: CORRECT_PASSWORD })).rejects.toMatchObject({ code: 'UNAUTHORIZED' })
        await expect(service.loginSession({ loginId: KNOWN_USER.loginId, password: 'wrong' })).rejects.toMatchObject({ code: 'UNAUTHORIZED' })
        expect(issuedTokens).toEqual([])
    })
})

describe('authService.login', () => {
    test('게임 클라이언트용 로그인은 기존대로 토큰을 한 개 발급한다', async () => {
        const { issuedTokens, service } = createFixture()
        const result = await service.login({ loginId: KNOWN_USER.loginId, password: CORRECT_PASSWORD })
        expect(result.token).toBe('plain-token')
        expect(issuedTokens).toEqual([{ userId: 'user-1', label: 'login' }])
    })
})

describe('authService.registerSession', () => {
    const newAccount = { loginId: 'new_player', password: 'local-password-1', email: 'new@example.com', name: null }

    test('세션 쿠키만 돌려주고 API 토큰을 발급하지 않는다', async () => {
        const { issuedTokens, service } = createFixture()
        const result = await service.registerSession(newAccount)
        expect(result.user).toEqual({ id: 'user-2', loginId: 'new_player', name: 'new_player', role: 'user' })
        expect(result.headers.get('set-cookie')).toBe('session=new')
        expect(issuedTokens).toEqual([])
    })

    test('이미 있는 아이디는 IR_ACCOUNT_EXISTS 로 거절한다', async () => {
        const { service } = createFixture()
        await expect(service.registerSession({ ...newAccount, loginId: KNOWN_USER.loginId })).rejects.toMatchObject({ code: 'IR_ACCOUNT_EXISTS' })
    })
})

describe('authService.register', () => {
    test('게임 클라이언트용 가입은 기존대로 토큰을 한 개 발급한다', async () => {
        const { issuedTokens, service } = createFixture()
        const result = await service.register({ loginId: 'new_player', password: 'local-password-1', email: 'new@example.com', name: 'New' })
        expect(result.token).toBe('plain-token')
        expect(issuedTokens).toEqual([{ userId: 'user-2', label: 'register' }])
    })
})

describe('sessionRegisterSchema', () => {
    test('api_version 을 받지 않고 아이디 형식과 예약어를 서버에서 검사한다', () => {
        const valid = { id: 'new_player', password: 'local-password-1', email: 'new@example.com' }
        expect(sessionRegisterSchema.parse(valid)).toEqual({ ...valid, name: null })
        expect(sessionRegisterSchema.safeParse({ ...valid, id: 'a b' }).success).toBe(false)
        expect(sessionRegisterSchema.safeParse({ ...valid, id: 'guest' }).success).toBe(false)
        expect(sessionRegisterSchema.safeParse({ ...valid, password: 'short' }).success).toBe(false)
    })
})

describe('sessionLoginSchema', () => {
    test('아이디와 비밀번호만 받고 나머지는 버린다', () => {
        const parsed = sessionLoginSchema.parse({ id: 'kuro_neko', password: 'x', email: 'a@b.c', api_version: 2 })
        expect(parsed).toEqual({ id: 'kuro_neko', password: 'x' })
    })

    test('아이디나 비밀번호가 비면 실패한다', () => {
        expect(sessionLoginSchema.safeParse({ id: '', password: 'x' }).success).toBe(false)
        expect(sessionLoginSchema.safeParse({ id: 'a', password: '' }).success).toBe(false)
    })

    test('게임용 loginSchema 는 그대로 api_version 기본값을 채운다', () => {
        expect(loginSchema.parse({ id: 'a', password: 'b' }).api_version).toBe(1)
    })
})

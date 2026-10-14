import { describe, expect, test } from 'bun:test'
import { APIError } from 'better-auth'
import { accountSchema, loginSchema, MAX_PASSWORD_LENGTH, sessionLoginSchema, sessionRegisterSchema } from '@server/dto/auth'
import { AUTH_PROVIDER_ERROR_CODE, createAuthService, type AuthServiceDeps } from '@server/service/domain/auth/auth.service'

const providerFailure = (code: string, status: ConstructorParameters<typeof APIError>[0] = 'UNPROCESSABLE_ENTITY') =>
    new APIError(status, { message: code, code })

const KNOWN_USER = { id: 'user-1', email: 'kuro@example.com', name: '黒猫', loginId: 'kuro_neko', role: 'user' }
const CORRECT_PASSWORD = 'correct-horse'

const createFixture = (
    overrides: { signUp?: AuthServiceDeps['provider']['signUpEmail']; signIn?: AuthServiceDeps['provider']['signInEmail'] } = {},
) => {
    const issuedTokens: { userId: string; label: string }[] = []
    const revokedHashes: string[] = []
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
            revokeTokenByHash: async (tokenHash) => {
                revokedHashes.push(tokenHash)
                return tokenHash === 'hash:live-token'
            },
        },
        provider: {
            signUpEmail:
                overrides.signUp ??
                (async ({ loginId, name }) => ({
                    headers: new Headers({ 'set-cookie': 'session=new' }),
                    user: { id: 'user-2', name, loginId, role: 'user' },
                })),
            signInEmail:
                overrides.signIn ??
                (async ({ email, password }) => {
                    if (email !== KNOWN_USER.email || password !== CORRECT_PASSWORD)
                        throw providerFailure(AUTH_PROVIDER_ERROR_CODE.INVALID_EMAIL_OR_PASSWORD, 'UNAUTHORIZED')
                    return { headers: new Headers({ 'set-cookie': 'session=abc' }), user: KNOWN_USER }
                }),
            getSessionUser: async () => null,
        },
        generateToken: () => 'plain-token',
        hashToken: (plain) => `hash:${plain}`,
        newId: (prefix) => `${prefix}id`,
        now: () => 0,
    }
    return { issuedTokens, revokedHashes, service: createAuthService(deps) }
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

describe('authService.register 의 실패 구분', () => {
    const newAccount = { loginId: 'new_player', password: 'local-password-1', email: 'new@example.com', name: null }
    const failWith = (error: unknown) => async () => {
        throw error
    }

    test('better-auth 가 이미 있는 이메일이라고 하면 IR_ACCOUNT_EXISTS(409)', async () => {
        const { service } = createFixture({ signUp: failWith(providerFailure(AUTH_PROVIDER_ERROR_CODE.USER_ALREADY_EXISTS_USE_ANOTHER_EMAIL)) })
        await expect(service.register(newAccount)).rejects.toMatchObject({ code: 'IR_ACCOUNT_EXISTS', statusCode: 409 })
    })

    test('연결 끊김이나 시크릿 누락 같은 원인 모를 오류는 409 로 바꾸지 않고 그대로 던진다', async () => {
        const secretMissing = new Error('You are using the default secret. Please set BETTER_AUTH_SECRET')
        const { service } = createFixture({ signUp: failWith(secretMissing) })
        await expect(service.register(newAccount)).rejects.toBe(secretMissing)
        const connectionLost = Object.assign(new Error('connect ECONNREFUSED'), { code: 'ECONNREFUSED' })
        const { service: other } = createFixture({ signUp: failWith(connectionLost) })
        await expect(other.register(newAccount)).rejects.toBe(connectionLost)
    })

    test('사용자 생성 실패는 그 사이 같은 아이디가 생겼을 때만 409 이고 아니면 그대로 던진다', async () => {
        const creationFailure = providerFailure(AUTH_PROVIDER_ERROR_CODE.FAILED_TO_CREATE_USER)
        const { service } = createFixture({ signUp: failWith(creationFailure) })
        await expect(service.register(newAccount)).rejects.toBe(creationFailure)
        await expect(service.register({ ...newAccount, loginId: KNOWN_USER.loginId })).rejects.toMatchObject({ code: 'IR_ACCOUNT_EXISTS' })
    })

    test('비밀번호 길이나 이메일 형식 오류는 어느 항목인지 담은 VALIDATION_ERROR(400)', async () => {
        const { service } = createFixture({ signUp: failWith(providerFailure(AUTH_PROVIDER_ERROR_CODE.PASSWORD_TOO_LONG, 'BAD_REQUEST')) })
        await expect(service.register(newAccount)).rejects.toMatchObject({
            code: 'VALIDATION_ERROR',
            statusCode: 400,
            details: { field: 'password' },
        })
        const { service: badEmail } = createFixture({ signUp: failWith(providerFailure(AUTH_PROVIDER_ERROR_CODE.INVALID_EMAIL, 'BAD_REQUEST')) })
        await expect(badEmail.registerSession(newAccount)).rejects.toMatchObject({ code: 'VALIDATION_ERROR', details: { field: 'email' } })
    })
})

describe('authService.login 의 실패 구분', () => {
    test('비밀번호가 틀리면 UNAUTHORIZED(401)', async () => {
        const { service } = createFixture()
        await expect(service.login({ loginId: KNOWN_USER.loginId, password: 'wrong' })).rejects.toMatchObject({
            code: 'UNAUTHORIZED',
            statusCode: 401,
        })
    })

    test('비밀번호와 무관한 오류는 401 로 바꾸지 않고 그대로 던진다', async () => {
        const databaseDown = new Error('Connection lost: The server closed the connection.')
        const { service, issuedTokens } = createFixture({
            signIn: async () => {
                throw databaseDown
            },
        })
        await expect(service.login({ loginId: KNOWN_USER.loginId, password: CORRECT_PASSWORD })).rejects.toBe(databaseDown)
        await expect(service.loginSession({ loginId: KNOWN_USER.loginId, password: CORRECT_PASSWORD })).rejects.toBe(databaseDown)
        expect(issuedTokens).toEqual([])
    })
})

describe('better-auth 오류 코드 이름', () => {
    test('서비스가 가려내는 코드가 better-auth 가 실제로 던지는 코드와 같다', async () => {
        const { betterAuth } = await import('better-auth')
        const { memoryAdapter } = await import('better-auth/adapters/memory')
        const auth = betterAuth({
            secret: 'a-test-secret-that-is-long-enough-for-better-auth',
            baseURL: 'http://localhost:3000',
            logger: { disabled: true },
            database: memoryAdapter({ user: [], session: [], account: [], verification: [] }),
            emailAndPassword: { enabled: true, minPasswordLength: 8, maxPasswordLength: MAX_PASSWORD_LENGTH },
            user: { additionalFields: { loginId: { type: 'string', required: true, input: true } } },
        })
        const body = { email: 'same@example.com', password: 'long-enough-1', name: 'same', loginId: 'same_player' }
        await auth.api.signUpEmail({ body })
        const caught = async (run: () => Promise<unknown>) => run().catch((error: unknown) => error)
        const duplicate = await caught(() => auth.api.signUpEmail({ body }))
        const tooLong = await caught(() =>
            auth.api.signUpEmail({ body: { ...body, email: 'long@example.com', password: 'x'.repeat(MAX_PASSWORD_LENGTH + 1) } }),
        )
        const wrongPassword = await caught(() => auth.api.signInEmail({ body: { email: body.email, password: 'not-the-password' } }))
        expect(duplicate).toMatchObject({ body: { code: AUTH_PROVIDER_ERROR_CODE.USER_ALREADY_EXISTS_USE_ANOTHER_EMAIL } })
        expect(tooLong).toMatchObject({ body: { code: AUTH_PROVIDER_ERROR_CODE.PASSWORD_TOO_LONG } })
        expect(wrongPassword).toMatchObject({ body: { code: AUTH_PROVIDER_ERROR_CODE.INVALID_EMAIL_OR_PASSWORD } })
    })
})

describe('API 토큰 폐기', () => {
    test('revokeApiToken 은 해시한 토큰 하나만 폐기하고 폐기됐는지 돌려준다', async () => {
        const { service, revokedHashes } = createFixture()
        expect(await service.revokeApiToken('live-token')).toBe(true)
        expect(await service.revokeApiToken('unknown-token')).toBe(false)
        expect(revokedHashes).toEqual(['hash:live-token', 'hash:unknown-token'])
    })

    test('로그인이 끝난 뒤에만 그 클라이언트가 들고 있던 이전 토큰을 폐기한다', async () => {
        const { service, revokedHashes, issuedTokens } = createFixture()
        await service.login({ loginId: KNOWN_USER.loginId, password: CORRECT_PASSWORD, previousToken: 'live-token' })
        expect(issuedTokens).toHaveLength(1)
        expect(revokedHashes).toEqual(['hash:live-token'])
    })

    test('비밀번호가 틀린 로그인은 이전 토큰을 건드리지 않는다', async () => {
        const { service, revokedHashes } = createFixture()
        await expect(service.login({ loginId: KNOWN_USER.loginId, password: 'wrong', previousToken: 'live-token' })).rejects.toMatchObject({
            code: 'UNAUTHORIZED',
        })
        expect(revokedHashes).toEqual([])
    })

    test('이전 토큰이 없으면 아무것도 폐기하지 않고 다른 기기의 토큰은 그대로다', async () => {
        const { service, revokedHashes } = createFixture()
        await service.login({ loginId: KNOWN_USER.loginId, password: CORRECT_PASSWORD })
        await service.register({ loginId: 'new_player', password: 'local-password-1', email: 'new@example.com', name: null })
        expect(revokedHashes).toEqual([])
    })
})

describe('비밀번호 길이 상한', () => {
    test('가입과 로그인 본문은 better-auth 가 받는 최대 길이를 넘으면 400 으로 거절한다', () => {
        const account = { id: 'new_player', email: 'new@example.com' }
        expect(accountSchema.safeParse({ ...account, password: 'x'.repeat(MAX_PASSWORD_LENGTH) }).success).toBe(true)
        expect(accountSchema.safeParse({ ...account, password: 'x'.repeat(MAX_PASSWORD_LENGTH + 1) }).success).toBe(false)
        expect(loginSchema.safeParse({ id: 'a', password: 'x'.repeat(MAX_PASSWORD_LENGTH + 1) }).success).toBe(false)
        expect(sessionLoginSchema.safeParse({ id: 'a', password: 'x'.repeat(MAX_PASSWORD_LENGTH + 1) }).success).toBe(false)
    })
})

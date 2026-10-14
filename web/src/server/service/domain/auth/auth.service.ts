import { z } from 'zod'
import { ERROR_CODE } from '@server/lib/error-code'
import { createAppError } from '@server/lib/error'
import type { AuthUser } from '@server/lib/auth-context'

const DEFAULT_ROLE = 'user'
const LAST_USED_REFRESH_MS = 60_000

const sessionUserSchema = z.object({
    id: z.string(),
    name: z.string().nullish(),
    loginId: z.string().nullish(),
    role: z.string().nullish(),
})

const signUpResultSchema = z.object({
    user: sessionUserSchema,
})

const providerErrorSchema = z.object({ body: z.object({ code: z.string() }) })

export const AUTH_PROVIDER_ERROR_CODE = {
    USER_ALREADY_EXISTS: 'USER_ALREADY_EXISTS',
    USER_ALREADY_EXISTS_USE_ANOTHER_EMAIL: 'USER_ALREADY_EXISTS_USE_ANOTHER_EMAIL',
    FAILED_TO_CREATE_USER: 'FAILED_TO_CREATE_USER',
    INVALID_EMAIL_OR_PASSWORD: 'INVALID_EMAIL_OR_PASSWORD',
    INVALID_EMAIL: 'INVALID_EMAIL',
    INVALID_PASSWORD: 'INVALID_PASSWORD',
    PASSWORD_TOO_SHORT: 'PASSWORD_TOO_SHORT',
    PASSWORD_TOO_LONG: 'PASSWORD_TOO_LONG',
} as const

const ACCOUNT_EXISTS_PROVIDER_CODES: ReadonlySet<string> = new Set([
    AUTH_PROVIDER_ERROR_CODE.USER_ALREADY_EXISTS,
    AUTH_PROVIDER_ERROR_CODE.USER_ALREADY_EXISTS_USE_ANOTHER_EMAIL,
])

const INVALID_ACCOUNT_FIELD_BY_PROVIDER_CODE: ReadonlyMap<string, string> = new Map([
    [AUTH_PROVIDER_ERROR_CODE.INVALID_EMAIL, 'email'],
    [AUTH_PROVIDER_ERROR_CODE.INVALID_PASSWORD, 'password'],
    [AUTH_PROVIDER_ERROR_CODE.PASSWORD_TOO_SHORT, 'password'],
    [AUTH_PROVIDER_ERROR_CODE.PASSWORD_TOO_LONG, 'password'],
])

const readProviderErrorCode = (error: unknown) => {
    const parsed = providerErrorSchema.safeParse(error)
    return parsed.success ? parsed.data.body.code : null
}

export type AuthServiceDb = {
    findUserByLoginId: (loginId: string) => Promise<{ id: string; email: string; name: string; loginId: string; role: string } | null>
    findUserById: (id: string) => Promise<{ id: string; name: string; loginId: string; role: string } | null>
    insertApiToken: (row: { id: string; userId: string; tokenHash: string; label: string }) => Promise<void>
    findApiToken: (tokenHash: string) => Promise<{ id: string; userId: string; lastUsedAt: number | null } | null>
    touchApiToken: (id: string, at: number) => Promise<void>
    revokeTokensForUser: (userId: string) => Promise<void>
    revokeTokenByHash: (tokenHash: string) => Promise<boolean>
}

export type AuthProvider = {
    signUpEmail: (body: { email: string; password: string; name: string; loginId: string }) => Promise<{ headers: Headers; user: unknown }>
    signInEmail: (body: { email: string; password: string }) => Promise<{ headers: Headers; user: unknown }>
    getSessionUser: (headers: Headers) => Promise<unknown>
}

type SignUpParams = { loginId: string; password: string; email: string; name: string | null }
type PreviousToken = { previousToken?: string | null }

export type AuthServiceDeps = {
    db: AuthServiceDb
    provider: AuthProvider
    generateToken: () => string
    hashToken: (plain: string) => string
    newId: (prefix: string) => string
    now: () => number
}

const toAuthUser = (raw: unknown): AuthUser | null => {
    const parsed = sessionUserSchema.safeParse(raw)
    if (!parsed.success) return null
    const { id, name, loginId, role } = parsed.data
    if (!loginId) return null
    return { id, loginId, name: name ?? loginId, role: role ?? DEFAULT_ROLE }
}

export const createAuthService = (deps: AuthServiceDeps) => {
    const issueToken = async (userId: string, label: string) => {
        const plain = deps.generateToken()
        await deps.db.insertApiToken({ id: deps.newId('tk_'), userId, tokenHash: deps.hashToken(plain), label })
        return plain
    }

    const retirePreviousToken = async (previousToken: string | null | undefined) => {
        if (previousToken) await deps.db.revokeTokenByHash(deps.hashToken(previousToken))
    }

    const signUp = async (params: SignUpParams) => {
        const duplicate = await deps.db.findUserByLoginId(params.loginId)
        if (duplicate) throw createAppError(ERROR_CODE.IR_ACCOUNT_EXISTS)
        const name = params.name ?? params.loginId
        const result = await deps.provider
            .signUpEmail({ email: params.email, password: params.password, name, loginId: params.loginId })
            .catch(async (error: unknown) => {
                const code = readProviderErrorCode(error)
                if (code === null) throw error
                if (ACCOUNT_EXISTS_PROVIDER_CODES.has(code)) throw createAppError(ERROR_CODE.IR_ACCOUNT_EXISTS)
                const invalidField = INVALID_ACCOUNT_FIELD_BY_PROVIDER_CODE.get(code)
                if (invalidField) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: invalidField })
                if (code === AUTH_PROVIDER_ERROR_CODE.FAILED_TO_CREATE_USER && (await deps.db.findUserByLoginId(params.loginId))) {
                    throw createAppError(ERROR_CODE.IR_ACCOUNT_EXISTS)
                }
                throw error
            })
        const parsed = signUpResultSchema.safeParse({ user: result.user })
        if (!parsed.success) throw createAppError(ERROR_CODE.INTERNAL_ERROR)
        return { user: parsed.data.user, name, headers: result.headers }
    }

    const signInByLoginId = async (params: { loginId: string; password: string }) => {
        const user = await deps.db.findUserByLoginId(params.loginId)
        if (!user) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        const result = await deps.provider.signInEmail({ email: user.email, password: params.password }).catch((error: unknown) => {
            if (readProviderErrorCode(error) === AUTH_PROVIDER_ERROR_CODE.INVALID_EMAIL_OR_PASSWORD) throw createAppError(ERROR_CODE.UNAUTHORIZED)
            throw error
        })
        return { user, headers: result.headers }
    }

    return {
        issueToken,

        register: async (params: SignUpParams & PreviousToken) => {
            const { user, name, headers } = await signUp(params)
            const token = await issueToken(user.id, 'register')
            await retirePreviousToken(params.previousToken)
            return { token, loginId: params.loginId, name, headers }
        },

        registerSession: async (params: SignUpParams) => {
            const { user, name, headers } = await signUp(params)
            return { user: { id: user.id, loginId: params.loginId, name, role: user.role ?? DEFAULT_ROLE }, headers }
        },

        login: async (params: { loginId: string; password: string } & PreviousToken) => {
            const { user, headers } = await signInByLoginId(params)
            const token = await issueToken(user.id, 'login')
            await retirePreviousToken(params.previousToken)
            return { token, loginId: user.loginId, name: user.name, headers }
        },

        loginSession: async (params: { loginId: string; password: string }) => {
            const { user, headers } = await signInByLoginId(params)
            return { user: { id: user.id, loginId: user.loginId, name: user.name, role: user.role }, headers }
        },

        revokeApiToken: async (plain: string) => deps.db.revokeTokenByHash(deps.hashToken(plain)),

        rotateToken: async (userId: string, label: string) => {
            await deps.db.revokeTokensForUser(userId)
            return issueToken(userId, label)
        },

        resolveApiTokenUser: async (plain: string) => {
            const record = await deps.db.findApiToken(deps.hashToken(plain))
            if (!record) return null
            const user = await deps.db.findUserById(record.userId)
            if (!user) return null
            const now = deps.now()
            if (!record.lastUsedAt || now - record.lastUsedAt > LAST_USED_REFRESH_MS) await deps.db.touchApiToken(record.id, now)
            return { id: user.id, loginId: user.loginId, name: user.name, role: user.role }
        },

        resolveSessionUser: async (request: Request) => toAuthUser(await deps.provider.getSessionUser(request.headers)),
    }
}

export type AuthService = ReturnType<typeof createAuthService>

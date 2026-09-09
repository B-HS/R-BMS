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

export type AuthServiceDb = {
    findUserByLoginId: (loginId: string) => Promise<{ id: string; email: string; name: string; loginId: string; role: string } | null>
    findUserById: (id: string) => Promise<{ id: string; name: string; loginId: string; role: string } | null>
    insertApiToken: (row: { id: string; userId: string; tokenHash: string; label: string }) => Promise<void>
    findApiToken: (tokenHash: string) => Promise<{ id: string; userId: string; lastUsedAt: number | null } | null>
    touchApiToken: (id: string, at: number) => Promise<void>
    revokeTokensForUser: (userId: string) => Promise<void>
}

export type AuthProvider = {
    signUpEmail: (body: { email: string; password: string; name: string; loginId: string }) => Promise<{ headers: Headers; user: unknown }>
    signInEmail: (body: { email: string; password: string }) => Promise<{ headers: Headers; user: unknown }>
    getSessionUser: (headers: Headers) => Promise<unknown>
}

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

    return {
        issueToken,

        register: async (params: { loginId: string; password: string; email: string; name: string | null }) => {
            const duplicate = await deps.db.findUserByLoginId(params.loginId)
            if (duplicate) throw createAppError(ERROR_CODE.IR_ACCOUNT_EXISTS)
            const name = params.name ?? params.loginId
            const result = await deps.provider
                .signUpEmail({ email: params.email, password: params.password, name, loginId: params.loginId })
                .catch(() => {
                    throw createAppError(ERROR_CODE.IR_ACCOUNT_EXISTS)
                })
            const parsed = signUpResultSchema.safeParse({ user: result.user })
            if (!parsed.success) throw createAppError(ERROR_CODE.INTERNAL_ERROR)
            const token = await issueToken(parsed.data.user.id, 'register')
            return { token, loginId: params.loginId, name, headers: result.headers }
        },

        login: async (params: { loginId: string; password: string }) => {
            const found = await deps.db.findUserByLoginId(params.loginId)
            if (!found) throw createAppError(ERROR_CODE.UNAUTHORIZED)
            const result = await deps.provider.signInEmail({ email: found.email, password: params.password }).catch(() => {
                throw createAppError(ERROR_CODE.UNAUTHORIZED)
            })
            const token = await issueToken(found.id, 'login')
            return { token, loginId: found.loginId, name: found.name, headers: result.headers }
        },

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

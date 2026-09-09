import { ERROR_CODE } from '@server/lib/error-code'
import { createAppError } from '@server/lib/error'
import { parseBearerToken } from '@server/lib/token'
import type { AuthUser, ResolveApiTokenUser, ResolveSessionUser } from '@server/lib/auth-context'

type AuthedHandler<TContext> = (request: Request, context: TContext, user: AuthUser) => Promise<Response> | Response

export const withApiToken =
    (deps: { resolveApiTokenUser: ResolveApiTokenUser }) =>
    <TContext>(handler: AuthedHandler<TContext>) =>
    async (request: Request, context: TContext) => {
        const plain = parseBearerToken(request)
        if (!plain) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        const user = await deps.resolveApiTokenUser(plain)
        if (!user) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        return handler(request, context, user)
    }

export const withBearerOrSession =
    (deps: { resolveApiTokenUser: ResolveApiTokenUser; resolveSessionUser: ResolveSessionUser }) =>
    <TContext>(handler: AuthedHandler<TContext>) =>
    async (request: Request, context: TContext) => {
        const plain = parseBearerToken(request)
        const user = plain ? await deps.resolveApiTokenUser(plain) : await deps.resolveSessionUser(request)
        if (!user) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        return handler(request, context, user)
    }

export const resolveOptionalApiTokenUser = async (request: Request, resolveApiTokenUser: ResolveApiTokenUser) => {
    const plain = parseBearerToken(request)
    if (!plain) return null
    const user = await resolveApiTokenUser(plain)
    if (!user) throw createAppError(ERROR_CODE.UNAUTHORIZED)
    return user
}

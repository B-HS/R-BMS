import { ERROR_CODE } from '@server/lib/error-code'
import { createAppError } from '@server/lib/error'
import type { AuthUser, ResolveSessionUser } from '@server/lib/auth-context'

const ADMIN_ROLE = 'admin'

type AuthedHandler<TContext> = (request: Request, context: TContext, user: AuthUser) => Promise<Response> | Response

export const withSession =
    (deps: { resolveSessionUser: ResolveSessionUser }) =>
    <TContext>(handler: AuthedHandler<TContext>) =>
    async (request: Request, context: TContext) => {
        const user = await deps.resolveSessionUser(request)
        if (!user) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        return handler(request, context, user)
    }

export const withAdmin =
    (deps: { resolveSessionUser: ResolveSessionUser }) =>
    <TContext>(handler: AuthedHandler<TContext>) =>
    async (request: Request, context: TContext) => {
        const user = await deps.resolveSessionUser(request)
        if (!user) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        if (user.role !== ADMIN_ROLE) throw createAppError(ERROR_CODE.FORBIDDEN)
        return handler(request, context, user)
    }

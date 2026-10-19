import { ERROR_CODE } from '@server/lib/error-code'
import { createAppError } from '@server/lib/error'
import { parseBearerToken } from '@server/lib/token'
import type { AuthUser, ResolveApiTokenUser } from '@server/lib/auth-context'

const ADMIN_ROLE = 'admin'

type AuthedHandler<TContext> = (request: Request, context: TContext, user: AuthUser) => Promise<Response> | Response

/**
 * Admits a request only by an API token whose user holds the admin role. The role is read with the token on every request, so
 * taking the role away stops the token at once. A session cookie alone is not enough: a route under this is called by a script.
 */
export const withAdminBearer =
    (deps: { resolveApiTokenUser: ResolveApiTokenUser }) =>
    <TContext>(handler: AuthedHandler<TContext>) =>
    async (request: Request, context: TContext) => {
        const plain = parseBearerToken(request)
        if (!plain) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        const user = await deps.resolveApiTokenUser(plain)
        if (!user) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        if (user.role !== ADMIN_ROLE) throw createAppError(ERROR_CODE.FORBIDDEN)
        return handler(request, context, user)
    }

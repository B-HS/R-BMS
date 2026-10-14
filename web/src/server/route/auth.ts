import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import { getAuth } from '@server/lib/auth'
import { accountSchema, loginSchema, sessionLoginSchema, sessionRegisterSchema, tokenIssueSchema } from '@server/dto/auth'
import { irRaw, successResponse } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseJsonBody } from '@server/lib/parse-request'
import { parseBearerToken } from '@server/lib/token'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { withBearerOrSession } from '@server/lib/with-api-token'
import { withSession } from '@server/lib/with-session'
import { RATE_LIMIT, consumeRateLimit, getClientIp, withRateLimit } from '@server/lib/with-rate-limit'
import type { AuthService } from '@server/service/domain/auth/auth.service'

const CREATED_STATUS = 201
const NO_CONTENT_STATUS = 204

const applySessionCookies = (response: Response, headers: Headers) => {
    const cookie = headers.get('set-cookie')
    if (cookie) response.headers.append('set-cookie', cookie)
    return response
}

const guardAuthRateLimit = (request: Request, scope: string) => {
    const { allowed } = consumeRateLimit(`${scope}:${getClientIp(request)}`, RATE_LIMIT.AUTH.limit, RATE_LIMIT.AUTH.windowMs)
    if (!allowed) throw createAppError(ERROR_CODE.RATE_LIMITED)
}

export const createRegisterRoute = () =>
    withErrorHandling(async (request: Request) => {
        guardAuthRateLimit(request, 'auth-register')
        const body = await parseJsonBody(request, accountSchema)
        const result = await compose().authService.register({
            loginId: body.id,
            password: body.password,
            email: body.email,
            name: body.name ?? null,
            previousToken: parseBearerToken(request),
        })
        return applySessionCookies(irRaw({ token: result.token, player: { id: result.loginId }, name: result.name }, CREATED_STATUS), result.headers)
    })

export const createLoginRoute = () =>
    withErrorHandling(async (request: Request) => {
        guardAuthRateLimit(request, 'auth-login')
        const body = await parseJsonBody(request, loginSchema)
        const result = await compose().authService.login({ loginId: body.id, password: body.password, previousToken: parseBearerToken(request) })
        return applySessionCookies(irRaw({ token: result.token, player: { id: result.loginId }, name: result.name }), result.headers)
    })

const toSessionUserBody = (user: { id: string; loginId: string; name: string; role: string }) =>
    successResponse({ user: { id: user.id, login_id: user.loginId, name: user.name, role: user.role } })

export const createSessionLoginRoute = () =>
    withErrorHandling(async (request: Request) => {
        guardAuthRateLimit(request, 'auth-login')
        const body = await parseJsonBody(request, sessionLoginSchema)
        const result = await compose().authService.loginSession({ loginId: body.id, password: body.password })
        return applySessionCookies(NextResponse.json(toSessionUserBody(result.user)), result.headers)
    })

export const createSessionRegisterRoute = () =>
    withErrorHandling(async (request: Request) => {
        guardAuthRateLimit(request, 'auth-register')
        const body = await parseJsonBody(request, sessionRegisterSchema)
        const result = await compose().authService.registerSession({ loginId: body.id, password: body.password, email: body.email, name: body.name })
        return applySessionCookies(NextResponse.json(toSessionUserBody(result.user), { status: CREATED_STATUS }), result.headers)
    })

export const createMeRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        await connection()
        const { authService, playerService } = compose()
        return withBearerOrSession({
            resolveApiTokenUser: authService.resolveApiTokenUser,
            resolveSessionUser: authService.resolveSessionUser,
        })(async (_request, _context, user) => {
            const profile = await playerService.getProfileById(user.id)
            if (!profile) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND)
            return irRaw(profile)
        })(request, context)
    })

export const createTokenRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService } = compose()
        return withSession({ resolveSessionUser: authService.resolveSessionUser })(async (authedRequest, _context, user) => {
            const raw = await authedRequest.json().catch(() => ({}))
            const parsed = tokenIssueSchema.safeParse(raw ?? {})
            const token = await authService.rotateToken(user.id, parsed.success ? parsed.data.label : '')
            return irRaw({ token, created_at: Date.now() })
        })(request, context)
    })

export const createTokenRevokeCurrentRoute = (resolveAuthService: () => Pick<AuthService, 'revokeApiToken'> = () => compose().authService) =>
    withErrorHandling(async (request: Request) => {
        guardAuthRateLimit(request, 'auth-token-revoke')
        const plain = parseBearerToken(request)
        if (!plain || !(await resolveAuthService().revokeApiToken(plain))) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        return new NextResponse(null, { status: NO_CONTENT_STATUS })
    })

const AUTH_BASE_PATH = '/api/auth'
const TRAILING_SLASHES = /\/+$/

export const ALLOWED_AUTH_HANDLER_ROUTES = ['GET /get-session', 'POST /sign-out'] as const

export const isAllowedAuthHandlerRoute = (method: string, pathname: string) =>
    (ALLOWED_AUTH_HANDLER_ROUTES as readonly string[]).includes(`${method} ${pathname.replace(AUTH_BASE_PATH, '').replace(TRAILING_SLASHES, '')}`)

export const createAuthHandlerRoute = (resolveHandler: () => (request: Request) => Promise<Response> = () => getAuth().handler) =>
    withErrorHandling(
        withRateLimit(
            'auth-handler',
            RATE_LIMIT.PUBLIC_READ,
        )(async (request: Request) => {
            if (!isAllowedAuthHandlerRoute(request.method, new URL(request.url).pathname)) throw createAppError(ERROR_CODE.NOT_FOUND)
            return resolveHandler()(request)
        }),
    )

import { compose } from '@server/compose'
import { accountSchema, loginSchema, tokenIssueSchema } from '@server/dto/auth'
import { irRaw } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseJsonBody } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { withBearerOrSession } from '@server/lib/with-api-token'
import { withSession } from '@server/lib/with-session'
import { RATE_LIMIT, consumeRateLimit, getClientIp } from '@server/lib/with-rate-limit'

const CREATED_STATUS = 201

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
        })
        return applySessionCookies(irRaw({ token: result.token, player: { id: result.loginId }, name: result.name }, CREATED_STATUS), result.headers)
    })

export const createLoginRoute = () =>
    withErrorHandling(async (request: Request) => {
        guardAuthRateLimit(request, 'auth-login')
        const body = await parseJsonBody(request, loginSchema)
        const result = await compose().authService.login({ loginId: body.id, password: body.password })
        return applySessionCookies(irRaw({ token: result.token, player: { id: result.loginId }, name: result.name }), result.headers)
    })

export const createMeRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
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

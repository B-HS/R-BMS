import { timingSafeEqual } from 'node:crypto'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'

const BEARER_PREFIX = 'Bearer '

/**
 * Admits a request only when its `Authorization` header is `Bearer <CRON_SECRET>`, which Vercel sends with every cron invocation. With no
 * secret configured nothing is admitted: the answer is SERVICE_NOT_CONFIGURED, so a missing setting reads differently from a wrong secret
 * in the logs. The secret is compared in constant time and is never written into an error.
 */
export const withCronSecret =
    (deps: { readSecret: () => string | undefined }) =>
    <TContext>(handler: (request: Request, context: TContext) => Promise<Response>) =>
    async (request: Request, context: TContext) => {
        const secret = deps.readSecret()
        if (!secret) throw createAppError(ERROR_CODE.SERVICE_NOT_CONFIGURED, { reason: 'cron_secret_missing' })
        const expected = Buffer.from(`${BEARER_PREFIX}${secret}`)
        const given = Buffer.from(request.headers.get('authorization') ?? '')
        if (given.length !== expected.length || !timingSafeEqual(given, expected)) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        return handler(request, context)
    }

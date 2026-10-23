import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import { composeDan } from '@server/compose/dan'
import { getDb } from '@server/db'
import { DAN_DISPLAY_BODY_MAX_BYTES, danDisplayInputSchema } from '@server/dto/dan'
import { successResponse } from '@server/lib/api-response'
import { parseLimitedJsonBody } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit } from '@server/lib/with-rate-limit'
import { withSession } from '@server/lib/with-session'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import type { DanService } from '@server/service/domain/dan/dan.service'
import { revalidateOnDanAward } from '@server/service/read-cache'

export type DanRouteDeps = {
    authService: Pick<AuthService, 'resolveSessionUser'>
    danService: Pick<DanService, 'listMine' | 'setDisplay'>
    revalidate: { onDanAward: typeof revalidateOnDanAward }
}

const PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }

let composedDan: ReturnType<typeof composeDan> | null = null

export const resolveDanService = () => {
    composedDan ??= composeDan(getDb())
    return composedDan.danService
}

const resolveDanRouteDeps = (): DanRouteDeps => ({
    authService: compose().authService,
    danService: resolveDanService(),
    revalidate: { onDanAward: revalidateOnDanAward },
})

const privateJson = (body: unknown) => NextResponse.json(body, { headers: PRIVATE_HEADERS })

/** `GET /api/fe/me/dans`: the signed-in player's acquired dans and the dan each system shows to others. */
export const createMyDansRoute = (resolveDeps: () => DanRouteDeps = resolveDanRouteDeps, waitForRequest: () => Promise<void> = connection) =>
    withErrorHandling(async (request: Request, context: unknown) => {
        await waitForRequest()
        const { authService, danService } = resolveDeps()
        return withSession({ resolveSessionUser: authService.resolveSessionUser })(async (_authedRequest, _authedContext, user) => {
            enforceUserRateLimit('dan-read', user.id, RATE_LIMIT.PUBLIC_READ)
            return privateJson(successResponse(await danService.listMine(user.id)))
        })(request, context)
    })

/** `PUT /api/fe/me/dans/display`: chooses which dan of one system others see (the highest, none, or one the player holds). */
export const createMyDanDisplayRoute = (resolveDeps: () => DanRouteDeps = resolveDanRouteDeps) =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService, danService, revalidate } = resolveDeps()
        return withSession({ resolveSessionUser: authService.resolveSessionUser })(async (authedRequest, _authedContext, user) => {
            enforceUserRateLimit('dan-write', user.id, RATE_LIMIT.SETTING_WRITE)
            const input = await parseLimitedJsonBody(authedRequest, danDisplayInputSchema, DAN_DISPLAY_BODY_MAX_BYTES)
            const dans = await danService.setDisplay(user.id, input)
            revalidate.onDanAward({ loginId: user.loginId })
            return privateJson(successResponse(dans))
        })(request, context)
    })

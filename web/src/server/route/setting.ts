import { connection } from 'next/server'
import { compose } from '@server/compose'
import { SETTING_MAX_BYTES, SETTING_NAME_MAX_LENGTH, settingNameSchema, settingPutSchema, type SettingPutOutput } from '@server/dto/setting'
import { irRaw, successResponse } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseLimitedJsonBody } from '@server/lib/parse-request'
import { withBearerOrSession } from '@server/lib/with-api-token'
import { withSession } from '@server/lib/with-session'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit } from '@server/lib/with-rate-limit'
import type { AuthUser } from '@server/lib/auth-context'

type SettingContext = { params: Promise<{ playerId: string; name: string }> }

const CONFLICT_STATUS = 409

const requireOwner = (user: AuthUser, playerId: string) => {
    if (user.loginId !== playerId) throw createAppError(ERROR_CODE.FORBIDDEN, { playerId })
}

const withOwner = <TContext>(handler: (request: Request, context: TContext, user: AuthUser) => Promise<Response>) => {
    const { authService } = compose()
    return withBearerOrSession({
        resolveApiTokenUser: authService.resolveApiTokenUser,
        resolveSessionUser: authService.resolveSessionUser,
    })(handler)
}

export const createSettingGetRoute = () =>
    withErrorHandling(async (request: Request, context: SettingContext) =>
        withOwner(async (_authedRequest, authedContext: SettingContext, user) => {
            const { playerId, name } = await authedContext.params
            requireOwner(user, playerId)
            const blob = await compose().settingService.get({ userId: user.id, name })
            if (!blob) throw createAppError(ERROR_CODE.IR_SETTING_NOT_FOUND, { name })
            return irRaw(blob)
        })(request, context),
    )

export const createSettingPutRoute = () =>
    withErrorHandling(async (request: Request, context: SettingContext) =>
        withOwner(async (authedRequest, authedContext: SettingContext, user) => {
            const { playerId, name: rawName } = await authedContext.params
            requireOwner(user, playerId)
            enforceUserRateLimit('setting-write', user.id, RATE_LIMIT.SETTING_WRITE)
            const name = settingNameSchema.safeParse(rawName)
            if (!name.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'name', max: SETTING_NAME_MAX_LENGTH })
            const input = await parseLimitedJsonBody(authedRequest, settingPutSchema, SETTING_MAX_BYTES)
            const result = await compose().settingService.put({ userId: user.id, name: name.data, input })
            if (result.status === 'conflict') return irRaw({ conflict: true, server: result.server }, CONFLICT_STATUS)
            if (result.status === 'too_many_names')
                throw createAppError(ERROR_CODE.IR_PAYLOAD_TOO_LARGE, { scope: 'setting_names', limit: result.limit })
            const stored: SettingPutOutput = { updated_at: result.updated_at }
            return irRaw(stored)
        })(request, context),
    )

export const createMySettingsRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        await connection()
        const { authService, settingService } = compose()
        return withSession({ resolveSessionUser: authService.resolveSessionUser })(async (_authedRequest, _authedContext, user) =>
            irRaw(successResponse({ keys: await settingService.listKeys(user.id) })),
        )(request, context)
    })

import { compose } from '@server/compose'
import { playerScoresQuerySchema, rivalPutSchema } from '@server/dto/fe'
import { irRaw } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { parseJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { withBearerOrSession } from '@server/lib/with-api-token'
import { getPlayerProfileCached, getPlayerScoresCached, getRivalsCached, revalidateOnRivalChange } from '@server/service/read-cache'

type PlayerContext = { params: Promise<{ playerId: string }> }

export const createPlayerProfileRoute = () =>
    withErrorHandling(async (_request: Request, context: PlayerContext) => {
        const { playerId } = await context.params
        const profile = await getPlayerProfileCached(playerId)
        if (!profile) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId })
        return irRaw(profile)
    })

export const createPlayerRivalsRoute = () =>
    withErrorHandling(async (_request: Request, context: PlayerContext) => {
        const { playerId } = await context.params
        const rivals = await getRivalsCached(playerId)
        if (!rivals) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId })
        return irRaw(rivals)
    })

export const createPlayerRivalsPutRoute = () =>
    withErrorHandling(async (request: Request, context: PlayerContext) => {
        const { authService, rivalService } = compose()
        return withBearerOrSession({
            resolveApiTokenUser: authService.resolveApiTokenUser,
            resolveSessionUser: authService.resolveSessionUser,
        })(async (authedRequest, authedContext: PlayerContext, user) => {
            const { playerId } = await authedContext.params
            if (user.loginId !== playerId) throw createAppError(ERROR_CODE.FORBIDDEN, { playerId })
            const body = await parseJsonBody(authedRequest, rivalPutSchema)
            const rivals = await rivalService.replaceByUserId({ userId: user.id, rivalLoginIds: body.rivals })
            revalidateOnRivalChange(playerId)
            return irRaw(rivals)
        })(request, context)
    })

export const createPlayerScoresRoute = () =>
    withErrorHandling(async (request: Request, context: PlayerContext) => {
        const query = parseSearchParams(request, playerScoresQuerySchema)
        const { playerId } = await context.params
        const userId = await compose().findUserIdByLoginId(playerId)
        if (!userId) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId })
        return irRaw(await getPlayerScoresCached({ loginId: playerId, userId, ...query }))
    })

import { compose } from '@server/compose'
import { clientBuildSchema } from '@server/dto/admin'
import { irRaw, successResponse } from '@server/lib/api-response'
import { parseJsonBody } from '@server/lib/parse-request'
import { withAdmin } from '@server/lib/with-session'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { getClientBuildsCached, revalidateOnBuildUpsert } from '@server/service/read-cache'

const CREATED_STATUS = 201

export const createAdminBuildListRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService } = compose()
        return withAdmin({ resolveSessionUser: authService.resolveSessionUser })(async () => irRaw(successResponse(await getClientBuildsCached())))(
            request,
            context,
        )
    })

export const createAdminBuildUpsertRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService, buildService } = compose()
        return withAdmin({ resolveSessionUser: authService.resolveSessionUser })(async (authedRequest) => {
            const input = await parseJsonBody(authedRequest, clientBuildSchema)
            const build = await buildService.upsert(input)
            revalidateOnBuildUpsert()
            return irRaw(successResponse(build), CREATED_STATUS)
        })(request, context)
    })

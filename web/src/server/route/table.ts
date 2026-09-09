import { compose } from '@server/compose'
import { tableUpsertSchema } from '@server/dto/table'
import { irRaw } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseJsonBody } from '@server/lib/parse-request'
import { withAdmin } from '@server/lib/with-session'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { getTableCached, getTablesCached, revalidateOnTableUpsert } from '@server/service/read-cache'

export const createTableListRoute = () => withErrorHandling(async () => irRaw(await getTablesCached()))

export const createTableDetailRoute = () =>
    withErrorHandling(async (_request: Request, context: { params: Promise<{ tableId: string }> }) => {
        const { tableId } = await context.params
        const table = await getTableCached(tableId)
        if (!table) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND, { tableId })
        return irRaw(table)
    })

export const createTableUpsertRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService, tableService } = compose()
        return withAdmin({ resolveSessionUser: authService.resolveSessionUser })(async (authedRequest) => {
            const input = await parseJsonBody(authedRequest, tableUpsertSchema)
            const table = await tableService.upsert(input)
            if (!table) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND, { tableId: input.id })
            revalidateOnTableUpsert(input.id)
            return irRaw(table)
        })(request, context)
    })

import { NextResponse } from 'next/server'
import { compose } from '@server/compose'
import { tableEntriesQuerySchema, tableIdParamSchema } from '@server/dto/table-snapshot'
import { successResponse } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseSearchParams } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit, withRateLimit } from '@server/lib/with-rate-limit'
import { withSession } from '@server/lib/with-session'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import type { TableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'

export type TableSnapshotRouteDeps = {
    authService: Pick<AuthService, 'resolveSessionUser'>
    tableSnapshotService: TableSnapshotService
}

type TableContext = { params: Promise<{ tableId: string }> }

const STATIC_JSON_CACHE = 'public, max-age=300, s-maxage=3600, stale-while-revalidate=86400'
const STANDARD_FORMAT_CACHE = 'public, max-age=3600, s-maxage=3600, stale-while-revalidate=86400'
const PRIVATE_CACHE = 'private, no-store'
const NOT_MODIFIED_STATUS = 304
const JSON_CONTENT_TYPE = 'application/json; charset=utf-8'

const STANDARD_FORMAT_HEADERS = {
    'Content-Type': JSON_CONTENT_TYPE,
    'Cache-Control': STANDARD_FORMAT_CACHE,
    'Access-Control-Allow-Origin': '*',
}

const parseTableId = async (context: TableContext) => {
    const { tableId } = await context.params
    const parsed = tableIdParamSchema.safeParse(tableId)
    if (!parsed.success) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND)
    return parsed.data
}

const requireTable = <T>(value: T | null, tableId: string) => {
    if (value === null) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND, { tableId })
    return value
}

const publicJson = (data: unknown) => NextResponse.json(successResponse(data), { headers: { 'Cache-Control': STATIC_JSON_CACHE } })

export const createTableSnapshotRoutes = (resolveDeps: () => TableSnapshotRouteDeps = compose) => {
    const withPublicRead = <TContext>(handler: (request: Request, context: TContext, deps: TableSnapshotRouteDeps) => Promise<Response>) =>
        withErrorHandling(
            withRateLimit('fe-read', RATE_LIMIT.PUBLIC_READ)((request: Request, context: TContext) => handler(request, context, resolveDeps())),
        )

    return {
        list: withPublicRead<unknown>(async (_request, _context, deps) => publicJson(await deps.tableSnapshotService.list())),

        detail: withPublicRead<TableContext>(async (_request, context, deps) => {
            const tableId = await parseTableId(context)
            return publicJson(requireTable(await deps.tableSnapshotService.getDetail(tableId), tableId))
        }),

        entries: withPublicRead<TableContext>(async (request, context, deps) => {
            const tableId = await parseTableId(context)
            const query = parseSearchParams(request, tableEntriesQuerySchema)
            const result = requireTable(await deps.tableSnapshotService.searchEntries(tableId, query), tableId)
            return NextResponse.json(
                {
                    ...successResponse(result.items),
                    pagination: {
                        page: query.page,
                        limit: query.limit,
                        total: result.total,
                        totalPages: Math.max(1, Math.ceil(result.total / query.limit)),
                    },
                },
                { headers: { 'Cache-Control': STATIC_JSON_CACHE } },
            )
        }),

        records: withErrorHandling(async (request: Request, context: TableContext) => {
            const deps = resolveDeps()
            return withSession({ resolveSessionUser: deps.authService.resolveSessionUser })(
                async (_authedRequest, authedContext: TableContext, user) => {
                    enforceUserRateLimit('table-records', user.id, RATE_LIMIT.TABLE_RECORDS)
                    const tableId = await parseTableId(authedContext)
                    const records = requireTable(await deps.tableSnapshotService.getRecords({ tableId, userId: user.id }), tableId)
                    return NextResponse.json(successResponse(records), { headers: { 'Cache-Control': PRIVATE_CACHE } })
                },
            )(request, context)
        }),

        standardHeader: withErrorHandling(async (request: Request, context: TableContext) => {
            const tableId = await parseTableId(context)
            const header = requireTable(await resolveDeps().tableSnapshotService.getStandardHeader(tableId, new URL(request.url).origin), tableId)
            return new NextResponse(JSON.stringify(header), { headers: STANDARD_FORMAT_HEADERS })
        }),

        standardData: withErrorHandling(async (request: Request, context: TableContext) => {
            const tableId = await parseTableId(context)
            const service = resolveDeps().tableSnapshotService
            const text = requireTable(await service.getStandardDataText(tableId), tableId)
            const etag = await service.getDataEtag(tableId)
            const headers = etag ? { ...STANDARD_FORMAT_HEADERS, ETag: etag } : STANDARD_FORMAT_HEADERS
            if (etag && request.headers.get('if-none-match') === etag) return new NextResponse(null, { status: NOT_MODIFIED_STATUS, headers })
            return new NextResponse(text, { headers })
        }),
    }
}

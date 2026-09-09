import { compose } from '@server/compose'
import { chartLookupQuerySchema, chartUpsertSchema } from '@server/dto/chart'
import { bestQuerySchema, rankingQuerySchema } from '@server/dto/score'
import { irRaw } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { withApiToken } from '@server/lib/with-api-token'
import { classifyChartHash } from '@server/service/domain/chart/chart.service'
import {
    getChartBestCached,
    getChartMetaCached,
    getChartRankingCached,
    resolveChartSha256Cached,
    revalidateOnChartUpsert,
} from '@server/service/read-cache'

type HashContext = { params: Promise<{ hash: string }> }

export const resolveKnownChart = async (hash: string, resolveSha256 = resolveChartSha256Cached) => {
    if (classifyChartHash(hash) === 'unknown') throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
    return resolveSha256(hash)
}

export const createChartDetailRoute = () =>
    withErrorHandling(async (_request: Request, context: HashContext) => {
        const { hash } = await context.params
        if (classifyChartHash(hash) === 'unknown') throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
        const meta = await getChartMetaCached(hash)
        if (!meta) throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
        return irRaw(meta)
    })

export const createChartUpsertRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService, chartService } = compose()
        return withApiToken({ resolveApiTokenUser: authService.resolveApiTokenUser })(async (authedRequest) => {
            const body = await parseJsonBody(authedRequest, chartUpsertSchema)
            const row = await chartService.upsertFromMeta(body.chart)
            if (!row) throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND)
            revalidateOnChartUpsert({ sha256: row.sha256, md5: row.md5 })
            const meta = await chartService.getMetaByHash(row.sha256)
            return irRaw(meta)
        })(request, context)
    })

export const createChartListRoute = () =>
    withErrorHandling(async (request: Request) => {
        const query = parseSearchParams(request, chartLookupQuerySchema)
        const hash = query.hash ?? query.sha256 ?? query.md5
        if (!hash) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'hash' })
        const meta = await getChartMetaCached(hash)
        if (!meta) throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
        return irRaw(meta)
    })

export const createChartRankingRoute = () =>
    withErrorHandling(async (request: Request, context: HashContext) => {
        const { hash } = await context.params
        const query = parseSearchParams(request, rankingQuerySchema)
        const resolved = await resolveKnownChart(hash)
        if (!resolved) return irRaw([])
        const rows = await getChartRankingCached({
            chartSha256: resolved.sha256,
            limit: query.limit,
            page: query.page,
            lnmode: query.lnmode,
            rivalOf: query.rival_of,
        })
        return irRaw(rows)
    })

export const createChartBestRoute = () =>
    withErrorHandling(async (request: Request, context: HashContext) => {
        const { hash } = await context.params
        const query = parseSearchParams(request, bestQuerySchema)
        const resolved = await resolveKnownChart(hash)
        if (!resolved) return irRaw(null)
        const record = await getChartBestCached({ chartSha256: resolved.sha256, loginId: query.player, lnmode: query.lnmode })
        return irRaw(record)
    })

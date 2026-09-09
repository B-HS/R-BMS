import { compose } from '@server/compose'
import { isSha256Hash } from '@server/dto/common'
import { replayListQuerySchema, replayUploadSchema, type ReplayUploadInput } from '@server/dto/replay'
import { irRaw } from '@server/lib/api-response'
import { getEnv } from '@server/lib/env'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { guardBodySize, parseLimitedJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { parseBearerToken } from '@server/lib/token'
import { withApiToken } from '@server/lib/with-api-token'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, consumeRateLimit } from '@server/lib/with-rate-limit'
import { classifyChartHash } from '@server/service/domain/chart/chart.service'
import { getChartReplaysCached, getReplayCached, resolveChartSha256Cached, revalidateOnReplayUpload } from '@server/service/read-cache'

type HashContext = { params: Promise<{ hash: string }> }

const CREATED_STATUS = 201

const resolveReplayChartSha256 = async (hash: string, input: ReplayUploadInput) => {
    if (classifyChartHash(hash) === 'sha256') return hash
    const resolved = await resolveChartSha256Cached(hash)
    if (resolved) return resolved.sha256
    const fromBody = input.chart?.sha256 ?? ''
    if (isSha256Hash(fromBody) && (await resolveChartSha256Cached(fromBody))) return fromBody
    throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
}

export const createReplayUploadRoute = () =>
    withErrorHandling(async (request: Request, context: HashContext) => {
        const maxBytes = getEnv().REPLAY_MAX_BYTES
        guardBodySize(request, maxBytes)
        if (!parseBearerToken(request)) throw createAppError(ERROR_CODE.UNAUTHORIZED)

        const { authService, replayService } = compose()
        return withApiToken({ resolveApiTokenUser: authService.resolveApiTokenUser })(async (authedRequest, authedContext: HashContext, user) => {
            const { allowed } = consumeRateLimit(`replay-upload:${user.id}`, RATE_LIMIT.REPLAY_UPLOAD.limit, RATE_LIMIT.REPLAY_UPLOAD.windowMs)
            if (!allowed) throw createAppError(ERROR_CODE.RATE_LIMITED)
            const { hash } = await authedContext.params
            const input = await parseLimitedJsonBody(authedRequest, replayUploadSchema, maxBytes)
            const chartSha256 = await resolveReplayChartSha256(hash, input)
            const result = await replayService.upload({ input, chartSha256, userId: user.id })
            revalidateOnReplayUpload({ chartSha256, loginId: user.loginId, scoreId: input.score_id ?? null })
            return irRaw(result, CREATED_STATUS)
        })(request, context)
    })

export const createChartReplayListRoute = () =>
    withErrorHandling(async (request: Request, context: HashContext) => {
        const query = parseSearchParams(request, replayListQuerySchema)
        const { hash } = await context.params
        if (classifyChartHash(hash) === 'unknown') throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
        const resolved = await resolveChartSha256Cached(hash)
        if (!resolved) return irRaw([])
        const userId = query.player ? await compose().findUserIdByLoginId(query.player) : null
        if (query.player && !userId) return irRaw([])
        return irRaw(await getChartReplaysCached({ chartSha256: resolved.sha256, userId: userId ?? undefined, limit: query.limit }))
    })

export const createReplayDetailRoute = () =>
    withErrorHandling(async (_request: Request, context: { params: Promise<{ replayId: string }> }) => {
        const { replayId } = await context.params
        const replay = await getReplayCached(replayId)
        if (!replay) throw createAppError(ERROR_CODE.IR_REPLAY_NOT_FOUND, { replayId })
        return irRaw(replay)
    })

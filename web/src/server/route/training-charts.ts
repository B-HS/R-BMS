import { connection } from 'next/server'
import { getLocale } from 'next-intl/server'
import { compose } from '@server/compose'
import { TRAINING_LIMIT, trainingChartCreateSchema, trainingChartIdSchema } from '@server/dto/training'
import { successResponse } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseLimitedJsonBody } from '@server/lib/parse-request'
import { RATE_LIMIT, enforceUserRateLimit } from '@server/lib/with-rate-limit'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import type { TrainingChartFailure, TrainingChartResult, TrainingChartService } from '@server/service/domain/training/training.service'
import { aiPrivateJson, createAiRouteWrappers, unwrapAiResult, type AiHandler } from '@server/route/ai'
import type { Locale } from '@shared/i18n/locale'

export type AiTrainingChartRouteDeps = {
    authService: Pick<AuthService, 'resolveSessionUser'>
    trainingChartService: Pick<TrainingChartService, 'list' | 'create' | 'detail' | 'removal' | 'remove'>
}

type ChartContext = { params: Promise<{ id: string }> }

const CREATED_STATUS = 201
const RATE_SCOPE = 'ai-training'

const notFound = () => createAppError(ERROR_CODE.TRAINING_CHART_NOT_FOUND)

/** A chart id that is not of the id's form is answered as a missing chart, without reading the database. */
const parseChartId = async (context: ChartContext) => {
    const parsed = trainingChartIdSchema.safeParse((await context.params).id)
    if (!parsed.success) throw notFound()
    return parsed.data
}

/** Another account's chart and a missing one answer alike (plan 2026-11-04 7.2): whether an id exists is never told. */
const requireOwned = <T>(value: T | null) => {
    if (value === null) throw notFound()
    return value
}

const isTrainingFailure = (result: { ok: boolean; reason?: string }): result is { ok: false } & TrainingChartFailure =>
    !result.ok && (result.reason === 'chart_limit' || result.reason === 'folder_unavailable' || result.reason === 'not_found')

const trainingFailureError = (failure: TrainingChartFailure) => {
    if (failure.reason === 'chart_limit') return createAppError(ERROR_CODE.TRAINING_CHART_LIMIT, { limit: failure.limit, current: failure.used })
    if (failure.reason === 'folder_unavailable') return createAppError(ERROR_CODE.TRAINING_FOLDER_UNAVAILABLE, { limit: failure.limit })
    return notFound()
}

const unwrapTrainingResult = <T>(result: TrainingChartResult<T>) => {
    if (isTrainingFailure(result)) throw trainingFailureError(result)
    return unwrapAiResult(result)
}

/**
 * The owner's training charts (plan 2026-11-04 7.2): list and make (`/api/fe/me/ai/training-charts`), read and delete one
 * (`/[id]`) and count what a delete takes and leaves (`/[id]/removal`). Every route needs the session, answers privately, counts
 * against one per-minute limit and reads only the session user's charts. Download links are built on the request's origin.
 */
export const createAiTrainingChartRoutes = (
    resolveDeps: () => AiTrainingChartRouteDeps = compose,
    waitForRequest: () => Promise<void> = connection,
    readLocale: () => Promise<Locale> = getLocale,
) => {
    const { withUser } = createAiRouteWrappers(resolveDeps, waitForRequest)

    const withLimited = <TContext>(handler: AiHandler<TContext, AiTrainingChartRouteDeps>) =>
        withUser<TContext>(async (request, context, user, deps) => {
            enforceUserRateLimit(RATE_SCOPE, user.id, RATE_LIMIT.TABLE_RECORDS)
            return handler(request, context, user, deps)
        })

    const ok = (data: unknown, status = 200) => aiPrivateJson(successResponse(data), status)
    const originOf = (request: Request) => new URL(request.url).origin

    return {
        list: withLimited<unknown>(async (request, _context, user, deps) =>
            ok(await deps.trainingChartService.list({ userId: user.id, origin: originOf(request) })),
        ),

        create: withLimited<unknown>(async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, trainingChartCreateSchema, TRAINING_LIMIT.WRITE_MAX_BYTES)
            const created = await deps.trainingChartService.create({
                userId: user.id,
                input,
                language: await readLocale(),
                origin: originOf(request),
            })
            return ok(unwrapTrainingResult(created), CREATED_STATUS)
        }),

        detail: withLimited<ChartContext>(async (request, context, user, deps) => {
            const id = await parseChartId(context)
            return ok(requireOwned(await deps.trainingChartService.detail({ userId: user.id, id, origin: originOf(request) })))
        }),

        removal: withLimited<ChartContext>(async (_request, context, user, deps) => {
            const id = await parseChartId(context)
            return ok(requireOwned(await deps.trainingChartService.removal({ userId: user.id, id })))
        }),

        remove: withLimited<ChartContext>(async (_request, context, user, deps) => {
            const id = await parseChartId(context)
            return ok(requireOwned(await deps.trainingChartService.remove({ userId: user.id, id })))
        }),
    }
}

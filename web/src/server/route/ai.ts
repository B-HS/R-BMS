import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import {
    AI_FAILURE_KIND,
    AI_GOAL_LIMIT,
    AI_LIMIT,
    AI_PRACTICE_LIMIT,
    AI_RECORD_QUERY_LIMIT,
    aiCredentialModelSchema,
    aiCredentialPutSchema,
    aiGoalIdSchema,
    aiGoalParamsSchema,
    aiOperatorLimitsPutSchema,
    aiPracticeFolderCreateSchema,
    aiPracticeSourceSchema,
    aiProviderParamSchema,
    aiRecordQueryAskSchema,
    aiRecordQueryRunSchema,
    aiTableQuerySchema,
    aiUserSettingPutSchema,
    aiWeekQuerySchema,
    type AiFailureKind,
} from '@server/dto/ai'
import { aiCompareQuerySchema, aiScoreParamSchema } from '@server/dto/ai-play'
import { FOLDER_LIMIT } from '@server/dto/folder'
import { successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { withSummarizedErrors } from '@server/lib/with-summarized-errors'
import { parseLimitedJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit } from '@server/lib/with-rate-limit'
import { withAdmin, withSession } from '@server/lib/with-session'
import type { AiServiceFailure, AiServiceResult } from '@server/service/domain/ai/ai.type'
import type { AiCredentialService } from '@server/service/domain/ai/credential.service'
import type { AiGaugeAdviceService } from '@server/service/domain/ai/gauge-advice.service'
import type { AiGoalFailure, AiGoalService } from '@server/service/domain/ai/goal-plan.service'
import type { AiJudgeHabitService } from '@server/service/domain/ai/judge-habit.service'
import type { AiPlayCompareService } from '@server/service/domain/ai/play-compare.service'
import type { AiPlayReportService } from '@server/service/domain/ai/play-report.service'
import type { AiPracticeFolderService, AiPracticeSourceFailure } from '@server/service/domain/ai/practice-folder.service'
import type { AiRecommendService } from '@server/service/domain/ai/recommend.service'
import type { AiWeeklySummaryService } from '@server/service/domain/ai/weekly-summary.service'
import type { AiRecordQueryService } from '@server/service/domain/ai/record-query.service'
import type { AiReplayCoachService } from '@server/service/domain/ai/replay-coach.service'
import type { AiSettingsService } from '@server/service/domain/ai/settings.service'
import type { AiUsageService } from '@server/service/domain/ai/usage.service'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import { AI_SCHEMA_UNSUPPORTED_ERROR_TYPE } from '@server/service/shared/ai-provider/http'
import type { AiFailureDetail } from '@server/service/shared/ai-provider/types'
import { unwrapFolderResult } from '@server/route/folder'

export type AiRouteDeps = {
    authService: Pick<AuthService, 'resolveSessionUser'>
    aiCredentialService: Pick<AiCredentialService, 'list' | 'register' | 'setModel' | 'remove' | 'revalidate' | 'listModels'>
    aiUsageService: Pick<AiUsageService, 'summary'>
    aiSettingsService: AiSettingsService
}

type ProviderContext = { params: Promise<{ provider: string }> }

type AiHandler<TContext, TDeps = AiRouteDeps> = (request: Request, context: TContext, user: AuthUser, deps: TDeps) => Promise<Response>

const NO_CONTENT_STATUS = 204
const AI_PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }

const PROVIDER_FAILURE_CODE = {
    [AI_FAILURE_KIND.INVALID_KEY]: ERROR_CODE.AI_CREDENTIAL_INVALID,
    [AI_FAILURE_KIND.FORBIDDEN]: ERROR_CODE.AI_PROVIDER_FORBIDDEN,
    [AI_FAILURE_KIND.MODEL_UNAVAILABLE]: ERROR_CODE.AI_MODEL_UNAVAILABLE,
    [AI_FAILURE_KIND.QUOTA]: ERROR_CODE.AI_PROVIDER_QUOTA,
    [AI_FAILURE_KIND.RATE_LIMITED]: ERROR_CODE.AI_PROVIDER_RATE_LIMITED,
    [AI_FAILURE_KIND.UNAVAILABLE]: ERROR_CODE.AI_PROVIDER_UNAVAILABLE,
    [AI_FAILURE_KIND.REFUSED]: ERROR_CODE.AI_PROVIDER_REFUSED,
    [AI_FAILURE_KIND.TRUNCATED]: ERROR_CODE.AI_OUTPUT_TRUNCATED,
    [AI_FAILURE_KIND.INVALID_OUTPUT]: ERROR_CODE.AI_OUTPUT_INVALID,
    [AI_FAILURE_KIND.BAD_REQUEST]: ERROR_CODE.AI_PROVIDER_BAD_REQUEST,
} as const satisfies Record<AiFailureKind, string>

/** A JSON response no shared cache may keep; every AI route answers with it. */
export const aiPrivateJson = (body: unknown, status = 200) => NextResponse.json(body, { status, headers: AI_PRIVATE_HEADERS })

const providerFailureError = (provider: string, detail: AiFailureDetail) => {
    const details = {
        reason: detail.kind,
        provider,
        request_id: detail.requestId,
        http_status: detail.httpStatus,
        provider_error_type: detail.providerErrorType,
        retry_after_seconds: detail.retryAfterSeconds,
    }
    if (detail.kind !== AI_FAILURE_KIND.BAD_REQUEST) return createAppError(PROVIDER_FAILURE_CODE[detail.kind], details)
    console.error('AI provider rejected the request', details)
    const isOurSchema = detail.providerErrorType === AI_SCHEMA_UNSUPPORTED_ERROR_TYPE
    return createAppError(isOurSchema ? ERROR_CODE.INTERNAL_ERROR : PROVIDER_FAILURE_CODE[detail.kind], details)
}

/** Turns an AI service failure into the API error a route throws. The details never carry a key, a prompt or a provider error body. */
export const toAiAppError = (failure: AiServiceFailure) => {
    if (failure.reason === 'credential_not_found') return createAppError(ERROR_CODE.AI_CREDENTIAL_NOT_FOUND, { reason: failure.reason })
    if (failure.reason === 'credential_undecryptable' && failure.cause === 'unknown_kek') {
        return createAppError(ERROR_CODE.SERVICE_NOT_CONFIGURED, { reason: 'ai_credential_kek_unknown' })
    }
    if (failure.reason === 'credential_undecryptable') return createAppError(ERROR_CODE.AI_CREDENTIAL_INVALID, { reason: failure.reason })
    if (failure.reason === 'model_not_selected') return createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'model', reason: failure.reason })
    if (failure.reason === 'usage_limit') return createAppError(ERROR_CODE.AI_USAGE_LIMIT, { reason: failure.limit })
    if (failure.reason === 'output_invalid') return createAppError(ERROR_CODE.AI_OUTPUT_INVALID, { reason: AI_FAILURE_KIND.INVALID_OUTPUT })
    if (failure.reason === 'not_enough_data') return createAppError(ERROR_CODE.AI_NOT_ENOUGH_DATA, { reason: failure.reason, need: failure.need })
    return providerFailureError(failure.provider, failure.detail)
}

export const unwrapAiResult = <T>(result: AiServiceResult<T>) => {
    if (result.ok) return result.value
    throw toAiAppError(result)
}

const parseProvider = async (context: ProviderContext) => {
    const parsed = aiProviderParamSchema.safeParse(await context.params)
    if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'provider' })
    return parsed.data.provider
}

type SessionDeps = { authService: Pick<AuthService, 'resolveSessionUser'> }

/** Unexpected errors of AI routes are logged by class and driver code only: a database error there carries ciphertext, a wrapped data key or prompt output. */
const withSafeAiErrors = withSummarizedErrors('AI route failed')

const createAiRouteWrappers = <TDeps extends SessionDeps>(resolveDeps: () => TDeps, waitForRequest: () => Promise<void>) => {
    const withUser = <TContext>(handler: AiHandler<TContext, TDeps>) =>
        withErrorHandling(
            withSafeAiErrors(async (request: Request, context: TContext) => {
                await waitForRequest()
                const deps = resolveDeps()
                return withSession({ resolveSessionUser: deps.authService.resolveSessionUser })((authedRequest, authedContext: TContext, user) =>
                    handler(authedRequest, authedContext, user, deps),
                )(request, context)
            }),
        )

    const withAdminUser = <TContext>(handler: AiHandler<TContext, TDeps>) =>
        withErrorHandling(
            withSafeAiErrors(async (request: Request, context: TContext) => {
                await waitForRequest()
                const deps = resolveDeps()
                return withAdmin({ resolveSessionUser: deps.authService.resolveSessionUser })((authedRequest, authedContext: TContext, user) =>
                    handler(authedRequest, authedContext, user, deps),
                )(request, context)
            }),
        )

    return { withUser, withAdminUser }
}

/**
 * `waitForRequest` keeps the GET handlers out of build-time prerendering (Cache Components prerenders GET route handlers that touch
 * no request data first); tests pass a no-op because `connection()` needs a Next request scope.
 */
export const createAiRoutes = (resolveDeps: () => AiRouteDeps = compose, waitForRequest: () => Promise<void> = connection) => {
    const { withUser, withAdminUser } = createAiRouteWrappers(resolveDeps, waitForRequest)

    const withKeyCheck = <TContext>(scope: string, handler: AiHandler<TContext>) =>
        withUser<TContext>(async (request, context, user, deps) => {
            enforceUserRateLimit(scope, user.id, RATE_LIMIT.AUTH)
            return handler(request, context, user, deps)
        })

    const withSettingWrite = <TContext>(wrap: typeof withUser, handler: AiHandler<TContext>) =>
        wrap<TContext>(async (request, context, user, deps) => {
            enforceUserRateLimit('ai-setting-write', user.id, RATE_LIMIT.SETTING_WRITE)
            return handler(request, context, user, deps)
        })

    return {
        listCredentials: withUser<unknown>(async (_request, _context, user, deps) =>
            aiPrivateJson(successResponse(await deps.aiCredentialService.list(user.id))),
        ),

        putCredential: withKeyCheck<ProviderContext>('ai-credential', async (request, context, user, deps) => {
            const provider = await parseProvider(context)
            const input = await parseLimitedJsonBody(request, aiCredentialPutSchema, AI_LIMIT.WRITE_MAX_BYTES)
            return aiPrivateJson(
                successResponse(unwrapAiResult(await deps.aiCredentialService.register({ userId: user.id, provider, apiKey: input.api_key }))),
            )
        }),

        patchCredential: withSettingWrite<ProviderContext>(withUser, async (request, context, user, deps) => {
            const provider = await parseProvider(context)
            const input = await parseLimitedJsonBody(request, aiCredentialModelSchema, AI_LIMIT.WRITE_MAX_BYTES)
            const updated = await deps.aiCredentialService.setModel({ userId: user.id, provider, model: input.model })
            if (!updated) throw createAppError(ERROR_CODE.AI_CREDENTIAL_NOT_FOUND, { reason: 'credential_not_found' })
            return aiPrivateJson(successResponse(updated))
        }),

        deleteCredential: withSettingWrite<ProviderContext>(withUser, async (_request, context, user, deps) => {
            const provider = await parseProvider(context)
            if (!(await deps.aiCredentialService.remove({ userId: user.id, provider }))) {
                throw createAppError(ERROR_CODE.AI_CREDENTIAL_NOT_FOUND, { reason: 'credential_not_found' })
            }
            return new NextResponse(null, { status: NO_CONTENT_STATUS, headers: AI_PRIVATE_HEADERS })
        }),

        validateCredential: withKeyCheck<ProviderContext>('ai-credential', async (_request, context, user, deps) => {
            const provider = await parseProvider(context)
            return aiPrivateJson(successResponse(unwrapAiResult(await deps.aiCredentialService.revalidate({ userId: user.id, provider }))))
        }),

        listModels: withKeyCheck<ProviderContext>('ai-models', async (_request, context, user, deps) => {
            const provider = await parseProvider(context)
            return aiPrivateJson(successResponse(unwrapAiResult(await deps.aiCredentialService.listModels({ userId: user.id, provider }))))
        }),

        usage: withUser<unknown>(async (_request, _context, user, deps) => {
            const limits = await deps.aiSettingsService.resolveLimits(user.id)
            return aiPrivateJson(successResponse(await deps.aiUsageService.summary({ userId: user.id, limits })))
        }),

        getSettings: withUser<unknown>(async (_request, _context, user, deps) =>
            aiPrivateJson(successResponse(await deps.aiSettingsService.getUserSetting(user.id))),
        ),

        putSettings: withSettingWrite<unknown>(withUser, async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, aiUserSettingPutSchema, AI_LIMIT.WRITE_MAX_BYTES)
            return aiPrivateJson(successResponse(await deps.aiSettingsService.putUserSetting({ userId: user.id, input })))
        }),

        getLimits: withAdminUser<unknown>(async (_request, _context, _user, deps) =>
            aiPrivateJson(successResponse(await deps.aiSettingsService.getOperatorLimits())),
        ),

        putLimits: withSettingWrite<unknown>(withAdminUser, async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, aiOperatorLimitsPutSchema, AI_LIMIT.WRITE_MAX_BYTES)
            return aiPrivateJson(successResponse(await deps.aiSettingsService.putOperatorLimits({ userId: user.id, input })))
        }),
    }
}

export type AiRecordQueryRouteDeps = SessionDeps & { aiRecordQueryService: Pick<AiRecordQueryService, 'ask' | 'run'> }

/** Feature 9: the AI turns a question into filters (`ask`), and the user's edited filters run again with no AI call (`run`). */
export const createAiRecordQueryRoutes = (resolveDeps: () => AiRecordQueryRouteDeps = compose, waitForRequest: () => Promise<void> = connection) => {
    const { withUser } = createAiRouteWrappers(resolveDeps, waitForRequest)

    return {
        ask: withUser<unknown>(async (request, _context, user, deps) => {
            enforceUserRateLimit('ai-record-query', user.id, RATE_LIMIT.TABLE_RECORDS)
            const input = await parseLimitedJsonBody(request, aiRecordQueryAskSchema, AI_LIMIT.WRITE_MAX_BYTES)
            return aiPrivateJson(successResponse(unwrapAiResult(await deps.aiRecordQueryService.ask({ userId: user.id, question: input.question }))))
        }),

        run: withUser<unknown>(async (request, _context, user, deps) => {
            enforceUserRateLimit('ai-record-query-run', user.id, RATE_LIMIT.TABLE_RECORDS)
            const plan = await parseLimitedJsonBody(request, aiRecordQueryRunSchema, AI_RECORD_QUERY_LIMIT.WRITE_MAX_BYTES)
            return aiPrivateJson(successResponse(await deps.aiRecordQueryService.run({ userId: user.id, plan })))
        }),
    }
}

export type AiScoreFeatureRouteDeps = SessionDeps & {
    aiRecommendService: Pick<AiRecommendService, 'view' | 'interpret'>
    aiPracticeFolderService: Pick<AiPracticeFolderService, 'view' | 'preview' | 'create'>
    aiGoalService: Pick<AiGoalService, 'list' | 'create' | 'archive' | 'planView' | 'interpret'>
    aiWeeklySummaryService: Pick<AiWeeklySummaryService, 'view' | 'interpret'>
    aiJudgeHabitService: Pick<AiJudgeHabitService, 'view' | 'interpret'>
}

type GoalContext = { params: Promise<{ goalId: string }> }

const CREATED_STATUS = 201

const requireTable = <T>(value: T | null) => {
    if (value === null) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND)
    return value
}

const requireGoal = <T>(value: T | null) => {
    if (value === null) throw createAppError(ERROR_CODE.NOT_FOUND, { reason: 'goal_not_found' })
    return value
}

const requireWeek = <T>(value: T | null) => {
    if (value === null) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'week', reason: 'not_a_past_monday' })
    return value
}

const parseGoalId = async (context: GoalContext) => {
    const parsed = aiGoalIdSchema.safeParse((await context.params).goalId)
    if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'goalId' })
    return parsed.data
}

const sourceFailureError = (failure: AiPracticeSourceFailure) => {
    if (failure.reason === 'table_not_found') return createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND)
    if (failure.reason === 'goal_not_found') return createAppError(ERROR_CODE.NOT_FOUND, { reason: failure.reason })
    return createAppError(ERROR_CODE.VALIDATION_ERROR, { field: failure.field, reason: failure.reason })
}

const goalFailureError = (failure: AiGoalFailure) => {
    if (failure.reason === 'table_not_found') return createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND)
    if (failure.reason === 'goal_not_found') return createAppError(ERROR_CODE.NOT_FOUND, { reason: failure.reason })
    if (failure.reason === 'goal_limit')
        return createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'goal', reason: 'active_goal_limit', limit: failure.limit })
    return createAppError(ERROR_CODE.VALIDATION_ERROR, { field: failure.field, reason: failure.reason })
}

const isSourceFailure = (result: { ok: boolean; reason?: string }): result is { ok: false } & AiPracticeSourceFailure =>
    !result.ok && (result.reason === 'table_not_found' || result.reason === 'goal_not_found' || result.reason === 'invalid_source')

/**
 * Features 4 (score-row part), 5, 6, 7 and 10. Each GET returns the deterministic digest with the latest stored AI answer and never
 * calls a provider; each POST is the user's "AI 해석 만들기" press and calls the shared runner once (no automatic retry). Every
 * response is private and read only for the session user.
 */
export const createAiScoreFeatureRoutes = (
    resolveDeps: () => AiScoreFeatureRouteDeps = compose,
    waitForRequest: () => Promise<void> = connection,
) => {
    const { withUser } = createAiRouteWrappers(resolveDeps, waitForRequest)

    const withLimited = <TContext>(scope: string, handler: AiHandler<TContext, AiScoreFeatureRouteDeps>) =>
        withUser<TContext>(async (request, context, user, deps) => {
            enforceUserRateLimit(scope, user.id, RATE_LIMIT.TABLE_RECORDS)
            return handler(request, context, user, deps)
        })

    const ok = (data: unknown, status = 200) => aiPrivateJson(successResponse(data), status)

    return {
        recommendView: withLimited<unknown>('ai-recommend', async (request, _context, user, deps) => {
            const query = parseSearchParams(request, aiTableQuerySchema)
            return ok(requireTable(await deps.aiRecommendService.view({ userId: user.id, tableId: query.table })))
        }),

        recommendInterpret: withLimited<unknown>('ai-recommend-run', async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, aiTableQuerySchema, AI_LIMIT.WRITE_MAX_BYTES)
            return ok(unwrapAiResult(requireTable(await deps.aiRecommendService.interpret({ userId: user.id, tableId: input.table }))))
        }),

        practiceView: withLimited<unknown>('ai-practice', async (request, _context, user, deps) => {
            const input = parseSearchParams(request, aiPracticeSourceSchema)
            const result = await deps.aiPracticeFolderService.view({ userId: user.id, input })
            if (!result.ok) throw sourceFailureError(result)
            return ok(result.value)
        }),

        practicePreview: withLimited<unknown>('ai-practice-run', async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, aiPracticeSourceSchema, AI_LIMIT.WRITE_MAX_BYTES)
            const result = await deps.aiPracticeFolderService.preview({ userId: user.id, input })
            if (isSourceFailure(result)) throw sourceFailureError(result)
            if (!result.ok && result.reason === 'folder_limit') {
                throw createAppError(ERROR_CODE.IR_FOLDER_LIMIT_EXCEEDED, {
                    scope: 'folders',
                    limit: result.limit,
                    current: result.used,
                    requested: 1,
                })
            }
            return ok(unwrapAiResult(result))
        }),

        practiceCreate: withUser<unknown>(async (request, _context, user, deps) => {
            enforceUserRateLimit('folder-write', user.id, RATE_LIMIT.FOLDER_WRITE)
            const input = await parseLimitedJsonBody(request, aiPracticeFolderCreateSchema, AI_PRACTICE_LIMIT.WRITE_MAX_BYTES)
            const result = await deps.aiPracticeFolderService.create({ userId: user.id, input })
            if (isSourceFailure(result)) throw sourceFailureError(result)
            if (!result.ok && result.reason === 'no_items') {
                throw createAppError(ERROR_CODE.VALIDATION_ERROR, {
                    field: 'items',
                    reason: 'no_candidate_items',
                    limit: FOLDER_LIMIT.ITEMS_PER_REQUEST,
                })
            }
            const value = unwrapFolderResult(result)
            return ok(value, value.outcome === 'created' ? CREATED_STATUS : 200)
        }),

        goalList: withLimited<unknown>('ai-goal', async (_request, _context, user, deps) => ok(await deps.aiGoalService.list(user.id))),

        goalCreate: withUser<unknown>(async (request, _context, user, deps) => {
            enforceUserRateLimit('ai-setting-write', user.id, RATE_LIMIT.SETTING_WRITE)
            const input = await parseLimitedJsonBody(request, aiGoalParamsSchema, AI_GOAL_LIMIT.WRITE_MAX_BYTES)
            const result = await deps.aiGoalService.create({ userId: user.id, input })
            if (!result.ok) throw goalFailureError(result)
            return ok(result.value, CREATED_STATUS)
        }),

        goalArchive: withUser<GoalContext>(async (_request, context, user, deps) => {
            enforceUserRateLimit('ai-setting-write', user.id, RATE_LIMIT.SETTING_WRITE)
            const goalId = await parseGoalId(context)
            if (!(await deps.aiGoalService.archive({ userId: user.id, goalId }))) throw goalFailureError({ reason: 'goal_not_found' })
            return new NextResponse(null, { status: NO_CONTENT_STATUS, headers: AI_PRIVATE_HEADERS })
        }),

        goalPlanView: withLimited<GoalContext>('ai-goal', async (_request, context, user, deps) => {
            const goalId = await parseGoalId(context)
            return ok(requireGoal(await deps.aiGoalService.planView({ userId: user.id, goalId })))
        }),

        goalPlanInterpret: withLimited<GoalContext>('ai-goal-run', async (_request, context, user, deps) => {
            const goalId = await parseGoalId(context)
            return ok(unwrapAiResult(requireGoal(await deps.aiGoalService.interpret({ userId: user.id, goalId }))))
        }),

        weeklyView: withLimited<unknown>('ai-weekly', async (request, _context, user, deps) => {
            const query = parseSearchParams(request, aiWeekQuerySchema)
            return ok(requireWeek(await deps.aiWeeklySummaryService.view({ userId: user.id, week: query.week })))
        }),

        weeklyInterpret: withLimited<unknown>('ai-weekly-run', async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, aiWeekQuerySchema, AI_LIMIT.WRITE_MAX_BYTES)
            return ok(unwrapAiResult(requireWeek(await deps.aiWeeklySummaryService.interpret({ userId: user.id, week: input.week }))))
        }),

        judgeHabitView: withLimited<unknown>('ai-judge-habit', async (_request, _context, user, deps) =>
            ok(await deps.aiJudgeHabitService.view(user.id)),
        ),

        judgeHabitInterpret: withLimited<unknown>('ai-judge-habit-run', async (_request, _context, user, deps) =>
            ok(unwrapAiResult(await deps.aiJudgeHabitService.interpret(user.id))),
        ),
    }
}

export type AiPlayFeatureRouteDeps = SessionDeps & {
    aiPlayReportService: Pick<AiPlayReportService, 'view' | 'interpret'>
    aiReplayCoachService: Pick<AiReplayCoachService, 'view' | 'interpret'>
    aiPlayCompareService: Pick<AiPlayCompareService, 'view' | 'interpret'>
    aiGaugeAdviceService: Pick<AiGaugeAdviceService, 'view' | 'interpret'>
}

type ScoreContext = { params: Promise<{ scoreId: string }> }

const parseScoreId = async (context: ScoreContext) => {
    const parsed = aiScoreParamSchema.safeParse(await context.params)
    if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'scoreId' })
    return parsed.data.scoreId
}

/** An own play the account does not have answers as a missing score, whoever's score the id is. */
const requirePlay = <T>(value: T | null) => {
    if (value === null) throw createAppError(ERROR_CODE.IR_SCORE_NOT_FOUND, { field: 'score_id' })
    return value
}

/**
 * Features 1, 2, 3, 4's trace part (through the judge-habit routes) and 8: they read the session user's own play analyses only, a
 * rival's segment totals only when that rival shares them. Each GET returns the deterministic result with the latest stored AI answer
 * and never calls a provider; each POST is the user's "AI 해석 만들기" press and calls the shared runner once (no automatic retry).
 */
export const createAiPlayFeatureRoutes = (resolveDeps: () => AiPlayFeatureRouteDeps = compose, waitForRequest: () => Promise<void> = connection) => {
    const { withUser } = createAiRouteWrappers(resolveDeps, waitForRequest)

    const withLimited = <TContext>(scope: string, handler: AiHandler<TContext, AiPlayFeatureRouteDeps>) =>
        withUser<TContext>(async (request, context, user, deps) => {
            enforceUserRateLimit(scope, user.id, RATE_LIMIT.TABLE_RECORDS)
            return handler(request, context, user, deps)
        })

    const ok = (data: unknown) => aiPrivateJson(successResponse(data))

    return {
        reportView: withLimited<unknown>('ai-report', async (_request, _context, user, deps) => ok(await deps.aiPlayReportService.view(user.id))),

        reportInterpret: withLimited<unknown>('ai-report-run', async (_request, _context, user, deps) =>
            ok(unwrapAiResult(await deps.aiPlayReportService.interpret(user.id))),
        ),

        coachView: withLimited<ScoreContext>('ai-coach', async (_request, context, user, deps) => {
            const scoreId = await parseScoreId(context)
            return ok(requirePlay(await deps.aiReplayCoachService.view({ userId: user.id, scoreId })))
        }),

        coachInterpret: withLimited<ScoreContext>('ai-coach-run', async (_request, context, user, deps) => {
            const scoreId = await parseScoreId(context)
            return ok(unwrapAiResult(requirePlay(await deps.aiReplayCoachService.interpret({ userId: user.id, scoreId }))))
        }),

        compareView: withLimited<ScoreContext>('ai-compare', async (request, context, user, deps) => {
            const scoreId = await parseScoreId(context)
            const query = parseSearchParams(request, aiCompareQuerySchema)
            return ok(requirePlay(await deps.aiPlayCompareService.view({ userId: user.id, scoreId, query })))
        }),

        compareInterpret: withLimited<ScoreContext>('ai-compare-run', async (request, context, user, deps) => {
            const scoreId = await parseScoreId(context)
            const query = await parseLimitedJsonBody(request, aiCompareQuerySchema, AI_LIMIT.WRITE_MAX_BYTES)
            return ok(unwrapAiResult(requirePlay(await deps.aiPlayCompareService.interpret({ userId: user.id, scoreId, query }))))
        }),

        gaugeAdviceView: withLimited<unknown>('ai-gauge-advice', async (_request, _context, user, deps) =>
            ok(await deps.aiGaugeAdviceService.view(user.id)),
        ),

        gaugeAdviceInterpret: withLimited<unknown>('ai-gauge-advice-run', async (_request, _context, user, deps) =>
            ok(unwrapAiResult(await deps.aiGaugeAdviceService.interpret(user.id))),
        ),
    }
}

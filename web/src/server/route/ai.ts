import { unstable_rethrow } from 'next/navigation'
import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import {
    AI_FAILURE_KIND,
    AI_LIMIT,
    AI_RECORD_QUERY_LIMIT,
    aiCredentialModelSchema,
    aiCredentialPutSchema,
    aiOperatorLimitsPutSchema,
    aiProviderParamSchema,
    aiRecordQueryAskSchema,
    aiRecordQueryRunSchema,
    aiUserSettingPutSchema,
    type AiFailureKind,
} from '@server/dto/ai'
import { successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { createAppError, isAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { summarizeError } from '@server/lib/error-summary'
import { parseLimitedJsonBody } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit } from '@server/lib/with-rate-limit'
import { withAdmin, withSession } from '@server/lib/with-session'
import type { AiServiceFailure, AiServiceResult } from '@server/service/domain/ai/ai.type'
import type { AiCredentialService } from '@server/service/domain/ai/credential.service'
import type { AiRecordQueryService } from '@server/service/domain/ai/record-query.service'
import type { AiSettingsService } from '@server/service/domain/ai/settings.service'
import type { AiUsageService } from '@server/service/domain/ai/usage.service'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import { AI_SCHEMA_UNSUPPORTED_ERROR_TYPE } from '@server/service/shared/ai-provider/http'
import type { AiFailureDetail } from '@server/service/shared/ai-provider/types'

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

/**
 * Keeps unexpected errors of AI routes out of `withErrorHandling`, which logs the raw error: a database error there carries the
 * query and its bound values (ciphertext, wrapped data key, prompt output). Only the error's class and driver code are logged.
 */
const withSafeAiErrors =
    <TContext>(run: (request: Request, context: TContext) => Promise<Response>) =>
    async (request: Request, context: TContext) => {
        try {
            return await run(request, context)
        } catch (error) {
            if (isAppError(error)) throw error
            unstable_rethrow(error)
            const summary = summarizeError(error)
            console.error('AI route failed', summary)
            throw createAppError(ERROR_CODE.INTERNAL_ERROR, summary)
        }
    }

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

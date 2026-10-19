import { AI_FAILURE_KIND, type AiFailureKind } from '@server/dto/ai'
import type { AiFailureDetail } from '@server/service/shared/ai-provider/types'

export const HTTP_STATUS = {
    BAD_REQUEST: 400,
    UNAUTHORIZED: 401,
    PAYMENT_REQUIRED: 402,
    FORBIDDEN: 403,
    NOT_FOUND: 404,
    REQUEST_TIMEOUT: 408,
    TOO_MANY_REQUESTS: 429,
    INTERNAL_SERVER_ERROR: 500,
} as const

export const REQUEST_ID_MAX_LENGTH = 128
export const PROVIDER_ERROR_TYPE_MAX_LENGTH = 64
export const RETRY_AFTER_MAX_SECONDS = 86_400

const MILLISECONDS_PER_SECOND = 1000
const REQUEST_ID_PATTERN = /^[A-Za-z0-9._:-]+$/
const PROVIDER_ERROR_TYPE_PATTERN = /^[A-Za-z0-9._-]+$/
const JSON_CONTENT_TYPE = 'application/json'
const MODEL_NOT_FOUND_ERROR_TYPES = new Set(['not_found_error', 'model_not_found'])

const normalizeToken = (value: string | null | undefined, pattern: RegExp, maxLength: number) => {
    const trimmed = value?.trim()
    if (!trimmed || trimmed.length > maxLength || !pattern.test(trimmed)) return null
    return trimmed
}

export const normalizeRequestId = (value: string | null | undefined) => normalizeToken(value, REQUEST_ID_PATTERN, REQUEST_ID_MAX_LENGTH)

export const normalizeProviderErrorType = (value: string | null | undefined) =>
    normalizeToken(value, PROVIDER_ERROR_TYPE_PATTERN, PROVIDER_ERROR_TYPE_MAX_LENGTH)

export const parseRetryAfterSeconds = (value: string | null | undefined, now = Date.now()) => {
    const trimmed = value?.trim()
    if (!trimmed) return null
    const seconds = /^\d+$/.test(trimmed) ? Number(trimmed) : Math.ceil((Date.parse(trimmed) - now) / MILLISECONDS_PER_SECOND)
    if (!Number.isFinite(seconds)) return null
    return Math.min(Math.max(seconds, 0), RETRY_AFTER_MAX_SECONDS)
}

export const tryParseJson = (text: string) => {
    try {
        return { ok: true as const, value: JSON.parse(text) as unknown }
    } catch {
        return { ok: false as const }
    }
}

export const createFailureDetail = (kind: AiFailureKind, overrides: Partial<Omit<AiFailureDetail, 'kind'>> = {}) =>
    ({
        kind,
        requestId: null,
        httpStatus: null,
        providerErrorType: null,
        retryAfterSeconds: null,
        ...overrides,
    }) satisfies AiFailureDetail

/**
 * Maps a non-2xx provider answer to a failure kind. Only 401 means the key is wrong; 403 means the key was accepted but may not
 * use this resource (region, project permission, model access); 404 or a provider "not found" type means the model is gone.
 */
export const classifyHttpFailure = ({
    status,
    quotaSignal,
    providerErrorType,
}: {
    status: number
    quotaSignal: boolean
    providerErrorType: string | null
}) => {
    if (status === HTTP_STATUS.UNAUTHORIZED) return AI_FAILURE_KIND.INVALID_KEY
    if (status === HTTP_STATUS.FORBIDDEN) return AI_FAILURE_KIND.FORBIDDEN
    if (status === HTTP_STATUS.PAYMENT_REQUIRED) return AI_FAILURE_KIND.QUOTA
    if (status === HTTP_STATUS.TOO_MANY_REQUESTS) return quotaSignal ? AI_FAILURE_KIND.QUOTA : AI_FAILURE_KIND.RATE_LIMITED
    if (status === HTTP_STATUS.BAD_REQUEST && quotaSignal) return AI_FAILURE_KIND.QUOTA
    if (status === HTTP_STATUS.NOT_FOUND || (providerErrorType !== null && MODEL_NOT_FOUND_ERROR_TYPES.has(providerErrorType))) {
        return AI_FAILURE_KIND.MODEL_UNAVAILABLE
    }
    if (status === HTTP_STATUS.REQUEST_TIMEOUT || status >= HTTP_STATUS.INTERNAL_SERVER_ERROR) return AI_FAILURE_KIND.UNAVAILABLE
    return AI_FAILURE_KIND.BAD_REQUEST
}

export type ProviderHttpResponse = {
    status: number
    ok: boolean
    requestId: string | null
    retryAfterSeconds: number | null
    body: unknown
}

type SendProviderRequestInput = {
    url: string
    method: 'GET' | 'POST'
    headers: Record<string, string>
    body?: unknown
    signal: AbortSignal
    requestIdHeader: string | null
}

/**
 * Sends one provider request. Returns null when the provider cannot be reached or the body cannot be read:
 * the thrown error is dropped on purpose because its message can carry the URL or request headers.
 * Redirects are refused so credential headers never follow a redirect to another host.
 */
export const sendProviderRequest = async ({ url, method, headers, body, signal, requestIdHeader }: SendProviderRequestInput) => {
    try {
        const response = await fetch(url, {
            method,
            headers: body === undefined ? headers : { ...headers, 'content-type': JSON_CONTENT_TYPE },
            body: body === undefined ? undefined : JSON.stringify(body),
            signal,
            redirect: 'error',
            cache: 'no-store',
        })
        const parsedBody = tryParseJson(await response.text())
        return {
            status: response.status,
            ok: response.ok,
            requestId: requestIdHeader ? normalizeRequestId(response.headers.get(requestIdHeader)) : null,
            retryAfterSeconds: parseRetryAfterSeconds(response.headers.get('retry-after')),
            body: parsedBody.ok ? parsedBody.value : undefined,
        } satisfies ProviderHttpResponse
    } catch {
        return null
    }
}

export const AI_SCHEMA_UNSUPPORTED_ERROR_TYPE = 'schema_unsupported'

type CallProviderInput = SendProviderRequestInput & {
    describeFailure: (response: ProviderHttpResponse) => AiFailureDetail
}

/**
 * Sends a request and folds every failure (unreachable, non-2xx) into one failure detail,
 * so each adapter only handles the 2xx body.
 */
export const callProvider = async ({ describeFailure, ...request }: CallProviderInput) => {
    const response = await sendProviderRequest(request)
    if (!response) return { ok: false as const, detail: createFailureDetail(AI_FAILURE_KIND.UNAVAILABLE) }
    if (!response.ok) return { ok: false as const, detail: describeFailure(response) }
    return { ok: true as const, response }
}

export const toHttpFailureDetail = ({
    response,
    quotaSignal,
    providerErrorType,
}: {
    response: ProviderHttpResponse
    quotaSignal: boolean
    providerErrorType: string | null | undefined
}) => {
    const normalizedType = normalizeProviderErrorType(providerErrorType)
    return createFailureDetail(classifyHttpFailure({ status: response.status, quotaSignal, providerErrorType: normalizedType }), {
        requestId: response.requestId,
        httpStatus: response.status,
        providerErrorType: normalizedType,
        retryAfterSeconds: response.retryAfterSeconds,
    })
}

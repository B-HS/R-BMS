export type ApiSuccessResponse<T> = { success: true; data: T }
export type ApiPagination = { page: number; limit: number; total: number; totalPages: number }
export type ApiErrorResponse = { success: false; error: { code: string; message: string; details?: Record<string, unknown> } }
export type ApiEnvelope<T> = ApiSuccessResponse<T> & { pagination?: ApiPagination }

export class ApiError extends Error {
    code: string
    status: number
    details?: Record<string, unknown>

    constructor(code: string, message: string, status: number, details?: Record<string, unknown>) {
        super(message)
        this.name = 'ApiError'
        this.code = code
        this.status = status
        this.details = details
    }
}

const isEnvelope = (value: unknown): value is ApiEnvelope<unknown> | ApiErrorResponse =>
    typeof value === 'object' && value !== null && 'success' in value && typeof (value as { success: unknown }).success === 'boolean'

const readBody = async (response: Response) => {
    const text = await response.text()
    if (!text) return null
    try {
        return JSON.parse(text) as unknown
    } catch {
        return text
    }
}

const failFromBody = (body: unknown, status: number) => {
    if (isEnvelope(body) && !body.success) return new ApiError(body.error.code, body.error.message, status, body.error.details)
    return new ApiError('HTTP_ERROR', `요청이 실패했습니다 (${status})`, status)
}

export const API_BASE_PATH = '/api'

const DEFAULT_DEV_PORT = '3000'

const resolveOrigin = () => {
    if (typeof window !== 'undefined') return ''
    if (process.env.NEXT_PUBLIC_APP_URL) return process.env.NEXT_PUBLIC_APP_URL
    if (process.env.VERCEL_URL) return `https://${process.env.VERCEL_URL}`
    return `http://localhost:${process.env.PORT ?? DEFAULT_DEV_PORT}`
}

export const buildApiUrl = (path: string, params?: Record<string, string | number | boolean | undefined | null>) => {
    const base = path.startsWith('http') ? path : `${resolveOrigin()}${API_BASE_PATH}${path}`
    if (!params) return base
    const search = new URLSearchParams()
    for (const [key, value] of Object.entries(params)) {
        if (value === undefined || value === null || value === '') continue
        search.set(key, String(value))
    }
    const query = search.toString()
    return query ? `${base}?${query}` : base
}

const runFetch = (url: string, init?: RequestInit) => fetch(url, { credentials: 'include', ...init })

/** Reads a FE route (`/api/fe/*`) that always answers with the `{ success, data }` envelope. */
export const fetchEnvelope = async <T>(url: string, init?: RequestInit) => {
    const response = await runFetch(url, init)
    const body = await readBody(response)
    if (!isEnvelope(body) || !body.success) throw failFromBody(body, response.status)
    return body.data as T
}

/** Reads a FE list route and keeps the pagination block alongside the rows. */
export const fetchEnvelopePage = async <T>(url: string, init?: RequestInit) => {
    const response = await runFetch(url, init)
    const body = await readBody(response)
    if (!isEnvelope(body) || !body.success) throw failFromBody(body, response.status)
    return { data: body.data as T, pagination: body.pagination }
}

/** Reads a native IR route that answers with raw JSON and no envelope. */
export const fetchRaw = async <T>(url: string, init?: RequestInit) => {
    const response = await runFetch(url, init)
    const body = await readBody(response)
    if (!response.ok) throw failFromBody(body, response.status)
    return body as T
}

/** Calls a route that answers `204 No Content`, so there is no body to decode. */
export const fetchNoContent = async (url: string, init?: RequestInit) => {
    const response = await runFetch(url, init)
    if (response.ok) return
    throw failFromBody(await readBody(response), response.status)
}

import { NextResponse } from 'next/server'
import type { ErrorCode } from '@server/lib/error-code'
import { getEnv } from '@server/lib/env'

export type Pagination = {
    page: number
    limit: number
    total: number
}

export const successResponse = <T>(data: T) => ({ success: true as const, data })

export const toPagination = (pagination: Pagination) => ({
    ...pagination,
    totalPages: Math.max(1, Math.ceil(pagination.total / pagination.limit)),
})

export const paginatedResponse = <T>(data: T[], pagination: Pagination) => ({
    success: true as const,
    data,
    pagination: toPagination(pagination),
})

const PRODUCTION_DETAIL_TYPES = {
    field: 'string',
    reason: 'string',
    provider: 'string',
    request_id: 'string',
    retry_after_seconds: 'number',
} as const

/**
 * Chooses the error `details` a response may carry.
 * Outside production they pass through unchanged. In production only the keys of `PRODUCTION_DETAIL_TYPES` with their own type
 * survive: a field name, a fixed reason token, and for AI errors the provider id, the provider's request id (already reduced to a
 * safe token) and the seconds to wait. The rest (Zod issues, paths, player ids, provider error types) can echo input.
 */
export const publicErrorDetails = (details: Record<string, unknown> | undefined, isProduction: boolean) => {
    if (!isProduction || !details) return details
    const exposed = Object.fromEntries(
        Object.entries(PRODUCTION_DETAIL_TYPES)
            .filter(([key, type]) => typeof details[key] === type)
            .map(([key]) => [key, details[key]]),
    )
    return Object.keys(exposed).length > 0 ? exposed : undefined
}

export const errorResponse = (code: ErrorCode, message: string, details?: Record<string, unknown>) => {
    const visibleDetails = publicErrorDetails(details, getEnv().NODE_ENV === 'production')
    return { success: false as const, error: { code, message, ...(visibleDetails ? { details: visibleDetails } : {}) } }
}

export const irRaw = <T>(data: T, status = 200) => NextResponse.json(data, { status })

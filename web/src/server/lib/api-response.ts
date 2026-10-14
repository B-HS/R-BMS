import { NextResponse } from 'next/server'
import type { ErrorCode } from '@server/lib/error-code'
import { getEnv } from '@server/lib/env'

export type Pagination = {
    page: number
    limit: number
    total: number
}

export const successResponse = <T>(data: T) => ({ success: true as const, data })

export const paginatedResponse = <T>(data: T[], pagination: Pagination) => ({
    success: true as const,
    data,
    pagination: { ...pagination, totalPages: Math.max(1, Math.ceil(pagination.total / pagination.limit)) },
})

const PRODUCTION_DETAIL_KEYS = ['field', 'reason'] as const

/**
 * Chooses the error `details` a response may carry.
 * Outside production they pass through unchanged. In production only a string `field` and a string `reason` survive,
 * because a field name and a fixed reason token are not secrets while the rest (Zod issues, paths, player ids) can echo input.
 */
export const publicErrorDetails = (details: Record<string, unknown> | undefined, isProduction: boolean) => {
    if (!isProduction || !details) return details
    const exposed = Object.fromEntries(PRODUCTION_DETAIL_KEYS.filter((key) => typeof details[key] === 'string').map((key) => [key, details[key]]))
    return Object.keys(exposed).length > 0 ? exposed : undefined
}

export const errorResponse = (code: ErrorCode, message: string, details?: Record<string, unknown>) => {
    const visibleDetails = publicErrorDetails(details, getEnv().NODE_ENV === 'production')
    return { success: false as const, error: { code, message, ...(visibleDetails ? { details: visibleDetails } : {}) } }
}

export const irRaw = <T>(data: T, status = 200) => NextResponse.json(data, { status })

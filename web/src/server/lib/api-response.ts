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

export const errorResponse = (code: ErrorCode, message: string, details?: Record<string, unknown>) => ({
    success: false as const,
    error: { code, message, ...(details && getEnv().NODE_ENV !== 'production' ? { details } : {}) },
})

export const irRaw = <T>(data: T, status = 200) => NextResponse.json(data, { status })

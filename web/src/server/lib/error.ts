import { ERROR_CODE, type ErrorCode } from '@server/lib/error-code'
import { ERROR_MESSAGE } from '@server/lib/error-message'

export type AppError = {
    code: ErrorCode
    message: string
    statusCode: number
    details?: Record<string, unknown>
}

const STATUS_MAP: Record<ErrorCode, number> = {
    [ERROR_CODE.VALIDATION_ERROR]: 400,
    [ERROR_CODE.UNAUTHORIZED]: 401,
    [ERROR_CODE.FORBIDDEN]: 403,
    [ERROR_CODE.NOT_FOUND]: 404,
    [ERROR_CODE.RATE_LIMITED]: 429,
    [ERROR_CODE.SERVICE_NOT_CONFIGURED]: 503,
    [ERROR_CODE.INTERNAL_ERROR]: 500,
    [ERROR_CODE.IR_CHART_NOT_FOUND]: 404,
    [ERROR_CODE.IR_PLAYER_NOT_FOUND]: 404,
    [ERROR_CODE.IR_SCORE_NOT_FOUND]: 404,
    [ERROR_CODE.IR_REPLAY_NOT_FOUND]: 404,
    [ERROR_CODE.IR_SETTING_NOT_FOUND]: 404,
    [ERROR_CODE.IR_COURSE_NOT_FOUND]: 404,
    [ERROR_CODE.IR_TABLE_NOT_FOUND]: 404,
    [ERROR_CODE.IR_ACCOUNT_EXISTS]: 409,
    [ERROR_CODE.IR_PAYLOAD_TOO_LARGE]: 413,
    [ERROR_CODE.IR_API_VERSION_UNSUPPORTED]: 400,
}

export const getStatusCode = (code: ErrorCode) => STATUS_MAP[code]

export const createAppError = (code: ErrorCode, details?: Record<string, unknown>): AppError => ({
    code,
    message: ERROR_MESSAGE[code],
    statusCode: STATUS_MAP[code],
    details,
})

export const isAppError = (error: unknown): error is AppError =>
    typeof error === 'object' && error !== null && 'code' in error && 'message' in error && 'statusCode' in error

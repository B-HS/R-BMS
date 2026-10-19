import { z } from 'zod'

const ERROR_CODE_PATTERN = /^[A-Z][A-Z0-9_]{0,63}$/
const ERROR_NAME_PATTERN = /^[A-Za-z][A-Za-z0-9]{0,63}$/

const errorCodeSchema = z.object({ code: z.string().regex(ERROR_CODE_PATTERN) })

const readCode = (value: unknown) => {
    const parsed = errorCodeSchema.safeParse(value)
    return parsed.success ? parsed.data.code : null
}

/**
 * The parts of an unexpected error that are safe to log: its class name and a driver error code such as `ER_DATA_TOO_LONG`.
 * The message is left out on purpose because a database error message carries the query and its bound values.
 */
export const summarizeError = (error: unknown) => {
    const name = error instanceof Error && ERROR_NAME_PATTERN.test(error.name) ? error.name : 'unknown'
    const cause = error instanceof Error ? error.cause : undefined
    return { error_name: name, error_code: readCode(cause) ?? readCode(error) }
}

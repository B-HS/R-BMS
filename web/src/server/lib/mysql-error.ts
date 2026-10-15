export const MYSQL_ERROR_CODE = {
    DUPLICATE_ENTRY: 'ER_DUP_ENTRY',
    LOCK_DEADLOCK: 'ER_LOCK_DEADLOCK',
    LOCK_WAIT_TIMEOUT: 'ER_LOCK_WAIT_TIMEOUT',
} as const

export type MysqlErrorCode = (typeof MYSQL_ERROR_CODE)[keyof typeof MYSQL_ERROR_CODE]

/**
 * Whether the error, or any error in its `cause` chain, is a MySQL error with one of these codes.
 * Drizzle wraps the driver's error as the `cause` of its own query error, so the code is looked up through the chain.
 */
export const hasMysqlErrorCode = (error: unknown, codes: readonly MysqlErrorCode[]): boolean => {
    if (typeof error !== 'object' || error === null) return false
    const code = 'code' in error ? error.code : undefined
    if (codes.some((candidate) => candidate === code)) return true
    return 'cause' in error ? hasMysqlErrorCode(error.cause, codes) : false
}

import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'

export const LOCK_RETRY_ATTEMPTS = 4
export const LOCK_RETRY_BASE_DELAY_MS = 10

const RETRYABLE_LOCK_ERRORS = [MYSQL_ERROR_CODE.LOCK_DEADLOCK, MYSQL_ERROR_CODE.LOCK_WAIT_TIMEOUT]

const waitBeforeAttempt = (attempt: number) => {
    const doubled = LOCK_RETRY_BASE_DELAY_MS * 2 ** (attempt - 1)
    return new Promise((resolve) => setTimeout(resolve, doubled + Math.random() * doubled))
}

const runWithLockRetry = async <Result>(write: () => Promise<Result>, attempt: number): Promise<Result> => {
    try {
        return await write()
    } catch (error) {
        if (attempt >= LOCK_RETRY_ATTEMPTS || !hasMysqlErrorCode(error, RETRYABLE_LOCK_ERRORS)) throw error
        await waitBeforeAttempt(attempt)
        return runWithLockRetry(write, attempt + 1)
    }
}

/**
 * Runs the write again when MySQL rolled it back for a deadlock (1213) or a lock wait timeout (1205), at most
 * LOCK_RETRY_ATTEMPTS times in all, waiting a short doubling time with jitter between attempts. Any other error, and the
 * last lock error, is thrown as it was.
 *
 * The write must be a unit that is safe to run again from the start: one statement, one `db.transaction`, or statements
 * that each leave the same result when repeated. It must not run inside an outer transaction, which MySQL rolls back as a whole.
 */
export const withLockRetry =
    <Args extends unknown[], Result>(write: (...args: Args) => Promise<Result>) =>
    async (...args: Args) =>
        runWithLockRetry(() => write(...args), 1)

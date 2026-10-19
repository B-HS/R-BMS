import { unstable_rethrow } from 'next/navigation'
import { createAppError, isAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { summarizeError } from '@server/lib/error-summary'

/**
 * Keeps an unexpected error of a route out of `withErrorHandling`, which logs the raw error: a database error carries the query and
 * its bound values (a trace body, a chart's note layout, ciphertext). Only the error's class and driver code are logged, under
 * `label`, and the route answers INTERNAL_ERROR with the same two fields.
 */
export const withSummarizedErrors =
    (label: string) =>
    <TContext>(run: (request: Request, context: TContext) => Promise<Response>) =>
    async (request: Request, context: TContext) => {
        try {
            return await run(request, context)
        } catch (error) {
            if (isAppError(error)) throw error
            unstable_rethrow(error)
            const summary = summarizeError(error)
            console.error(label, summary)
            throw createAppError(ERROR_CODE.INTERNAL_ERROR, summary)
        }
    }

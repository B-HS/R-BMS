import { NextResponse } from 'next/server'
import { ERROR_CODE } from '@server/lib/error-code'
import { ERROR_MESSAGE } from '@server/lib/error-message'
import { errorResponse } from '@server/lib/api-response'
import { isAppError } from '@server/lib/error'

type RouteHandler<TContext> = (request: Request, context: TContext) => Promise<Response> | Response

export const withErrorHandling =
    <TContext>(handler: RouteHandler<TContext>) =>
    async (request: Request, context: TContext) => {
        try {
            return await handler(request, context)
        } catch (error) {
            if (isAppError(error)) {
                return NextResponse.json(errorResponse(error.code, error.message, error.details), { status: error.statusCode })
            }
            console.error(error)
            return NextResponse.json(errorResponse(ERROR_CODE.INTERNAL_ERROR, ERROR_MESSAGE.INTERNAL_ERROR), { status: 500 })
        }
    }

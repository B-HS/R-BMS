import { describe, expect, test } from 'bun:test'
import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { ApiError } from '@shared/lib/fetch'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'

const NOT_FOUND_DIGEST = 'NEXT_HTTP_ERROR_FALLBACK;404'

const clientThatFails = (error: unknown) => ({ fetchQuery: async () => Promise.reject(error) }) as unknown as QueryClient

const options = queryOptions({ queryKey: ['x'], queryFn: async () => ({ ok: true }) })

describe('fetchQueryOrNotFound', () => {
    test('HTTP 404 를 받은 ApiError 는 not-found 화면이 된다', async () => {
        const failure = await fetchQueryOrNotFound(clientThatFails(new ApiError('IR_CHART_NOT_FOUND', 'x', 404)), options).catch(
            (error: unknown) => error,
        )
        expect(failure).toMatchObject({ digest: NOT_FOUND_DIGEST })
    })

    test('서버가 직접 읽다 던진 404 AppError 도 not-found 화면이 된다', async () => {
        const failure = await fetchQueryOrNotFound(clientThatFails(createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND)), options).catch(
            (error: unknown) => error,
        )
        expect(failure).toMatchObject({ digest: NOT_FOUND_DIGEST })
    })

    test('404 가 아닌 실패는 null 로 삼켜 클라이언트가 다시 시도하게 둔다', async () => {
        expect(await fetchQueryOrNotFound(clientThatFails(createAppError(ERROR_CODE.INTERNAL_ERROR)), options)).toBeNull()
        expect(await fetchQueryOrNotFound(clientThatFails(new ApiError('RATE_LIMITED', 'x', 429)), options)).toBeNull()
        expect(await fetchQueryOrNotFound(clientThatFails(new Error('connection lost')), options)).toBeNull()
    })

    test('성공하면 데이터를 그대로 돌려준다', async () => {
        const client = { fetchQuery: async () => ({ ok: true }) } as unknown as QueryClient
        expect(await fetchQueryOrNotFound(client, options)).toEqual({ ok: true })
    })
})

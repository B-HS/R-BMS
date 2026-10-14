import { describe, expect, spyOn, test } from 'bun:test'
import { notFound, redirect } from 'next/navigation'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { withErrorHandling } from '@server/lib/with-error-handling'

const request = new Request('http://localhost:3000/api/x')

describe('withErrorHandling', () => {
    test('AppError 는 코드의 상태와 봉투로 바꾼다', async () => {
        const response = await withErrorHandling(async () => {
            throw createAppError(ERROR_CODE.IR_FOLDER_NOT_FOUND)
        })(request, undefined)
        expect(response.status).toBe(404)
        expect((await response.json()).error.code).toBe('IR_FOLDER_NOT_FOUND')
    })

    test('알 수 없는 오류는 500 INTERNAL_ERROR 이고 로그를 남긴다', async () => {
        const spy = spyOn(console, 'error').mockImplementation(() => undefined)
        const response = await withErrorHandling(async () => {
            throw new Error('boom')
        })(request, undefined)
        expect(response.status).toBe(500)
        expect((await response.json()).error.code).toBe('INTERNAL_ERROR')
        expect(spy).toHaveBeenCalledTimes(1)
        spy.mockRestore()
    })

    test('Next 의 notFound 와 redirect 신호는 삼키지 않고 로그도 남기지 않는다', async () => {
        const spy = spyOn(console, 'error').mockImplementation(() => undefined)
        await expect(
            withErrorHandling(async () => {
                notFound()
            })(request, undefined),
        ).rejects.toBeDefined()
        await expect(
            withErrorHandling(async () => {
                redirect('/login')
            })(request, undefined),
        ).rejects.toBeDefined()
        expect(spy).not.toHaveBeenCalled()
        spy.mockRestore()
    })
})

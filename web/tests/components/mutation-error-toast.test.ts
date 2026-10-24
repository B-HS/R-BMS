import { afterEach, describe, expect, test } from 'bun:test'
import { MutationObserver } from '@tanstack/react-query'
import { toast } from 'sonner'
import { MESSAGES } from '../../src/shared/i18n/messages'
import { ApiError, HTTP_ERROR_CODE } from '../../src/shared/lib/fetch'
import { bindMutationErrorMessages, createQueryClient } from '../../src/shared/utils/get-query-client'

const failMutation = async (error: unknown, meta?: Record<string, unknown>) => {
    const mutationFn = async () => {
        throw error
    }
    await new MutationObserver(createQueryClient(), { mutationFn, meta, retry: false }).mutate().catch(() => undefined)
}

const toastTitles = () => toast.getToasts().flatMap((entry) => ('title' in entry ? [String(entry.title)] : []))

let unbind: () => void = () => undefined

afterEach(() => {
    unbind()
    toast.dismiss()
})

describe('변이 실패 토스트는 화면 언어로 문구를 고른다', () => {
    test('서버가 준 오류는 서버 문구를 그대로 보인다', async () => {
        unbind = bindMutationErrorMessages({ http: (status) => `http ${status}`, unknown: 'unknown' })
        await failMutation(new ApiError('IR_FOLDER_NAME_TAKEN', '같은 이름의 폴더가 이미 있습니다.', 409))

        expect(toastTitles()).toEqual(['같은 이름의 폴더가 이미 있습니다.'])
    })

    test('HTTP_ERROR 는 상태 코드를 넣은 화면 언어 문구로 바꾼다', async () => {
        const messages = MESSAGES.en.pages.errors
        unbind = bindMutationErrorMessages({ http: (status) => messages.http.replace('{status}', String(status)), unknown: messages.unknown })
        await failMutation(new ApiError(HTTP_ERROR_CODE, '요청이 실패했습니다 (502)', 502))

        expect(toastTitles()).toEqual(['Request failed (502)'])
    })

    test('오류 객체가 아닌 값은 화면 언어의 일반 문구를 쓴다', async () => {
        unbind = bindMutationErrorMessages({ http: (status) => `http ${status}`, unknown: MESSAGES.ja.pages.errors.unknown })
        await failMutation('boom')

        expect(toastTitles()).toEqual(['リクエストを処理できませんでした。'])
    })

    test('문구를 묶기 전에도 토스트는 오류 문구를 잃지 않는다', async () => {
        await failMutation(new ApiError(HTTP_ERROR_CODE, 'fallback sentence', 500))

        expect(toastTitles()).toEqual(['fallback sentence'])
    })

    test('suppressErrorToast 인 변이는 토스트를 띄우지 않는다', async () => {
        await failMutation(new ApiError('X', 'quiet', 400), { suppressErrorToast: true })

        expect(toastTitles()).toEqual([])
    })
})

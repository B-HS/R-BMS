import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { DISPLAY_TIME_ZONE } from '../../src/shared/lib/format'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const { cleanup, render, within } = await import('@testing-library/react')
const { NextIntlClientProvider } = await import('next-intl')
const { MESSAGES } = await import('../../src/shared/i18n/messages')
const { ApiError } = await import('../../src/shared/lib/fetch')
const { useApiErrorMessage } = await import('../../src/shared/lib/use-api-error-message')

type Locale = import('../../src/shared/i18n/locale').Locale

const FALLBACK = 'fallback'
const NOT_FOUND_STATUS = 404
const BAD_GATEWAY_STATUS = 502

const ErrorLine = ({ error }: { error: unknown }) => <p role='alert'>{useApiErrorMessage()(error) ?? FALLBACK}</p>

const renderError = (locale: Locale, error: unknown) => {
    const { container } = render(
        <NextIntlClientProvider locale={locale} messages={MESSAGES[locale]} timeZone={DISPLAY_TIME_ZONE}>
            <ErrorLine error={error} />
        </NextIntlClientProvider>,
    )
    return within(container).getByRole('alert').textContent
}

const SERVER_KOREAN = '난이도표를 찾을 수 없습니다.'
const TABLE_NOT_FOUND = new ApiError('IR_TABLE_NOT_FOUND', SERVER_KOREAN, NOT_FOUND_STATUS)

afterEach(() => cleanup())

describe('useApiErrorMessage', () => {
    test('서버가 준 오류 코드를 화면 언어로 보여 준다', () => {
        expect(renderError('ko', TABLE_NOT_FOUND)).toBe(SERVER_KOREAN)
        expect(renderError('en', TABLE_NOT_FOUND)).toBe('The difficulty table could not be found.')
        expect(renderError('ja', TABLE_NOT_FOUND)).toBe('難易度表が見つかりません。')
    })

    test('API 봉투가 없는 실패는 상태 번호와 함께 화면 언어로 보여 준다', () => {
        const httpError = new ApiError('HTTP_ERROR', `요청이 실패했습니다 (${BAD_GATEWAY_STATUS})`, BAD_GATEWAY_STATUS)
        expect(renderError('ko', httpError)).toBe('요청이 실패했습니다 (502)')
        expect(renderError('en', httpError)).toBe('The request failed (502)')
        expect(renderError('ja', httpError)).toBe('リクエストに失敗しました（502）')
    })

    test('카탈로그에 없는 코드는 서버 문구를 그대로 두고, ApiError 가 아니면 부르는 쪽 대체 문구를 쓴다', () => {
        expect(renderError('en', new ApiError('SOMETHING_NEW', '서버 문구', BAD_GATEWAY_STATUS))).toBe('서버 문구')
        expect(renderError('ja', new Error('network'))).toBe(FALLBACK)
    })
})

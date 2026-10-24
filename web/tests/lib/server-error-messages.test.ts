import { describe, expect, test } from 'bun:test'
import { ERROR_CODE } from '@server/lib/error-code'
import { ERROR_MESSAGE } from '@server/lib/error-message'
import { LOCALES } from '@shared/i18n/locale'
import { MESSAGES } from '@shared/i18n/messages'

describe('서버 오류 문구 카탈로그', () => {
    test('언어마다 서버의 모든 오류 코드를 하나씩 갖는다', () => {
        for (const locale of LOCALES) {
            expect(Object.keys(MESSAGES[locale].server.error).toSorted(), locale).toEqual(Object.values(ERROR_CODE).toSorted())
        }
    })

    test('한국어 문구는 API 가 보내는 message 와 글자 하나 다르지 않다', () => {
        expect(MESSAGES.ko.server.error).toEqual(ERROR_MESSAGE)
    })
})

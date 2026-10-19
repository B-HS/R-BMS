import { describe, expect, test } from 'bun:test'
import { CLEAR_LAMPS as SERVER_CLEAR_LAMPS, MIN_CLEAR_LAMP_ID, clearLampToId } from '@server/dto/common'
import { CLEAR_LAMPS as CLIENT_CLEAR_LAMPS, MIN_CLEAR_LAMP } from '../../src/shared/constants/clear-lamp'

const NOT_CLEARS = ['NoPlay', 'Failed']

describe('클리어 기준', () => {
    test('FAILED 가 아닌 램프는 모두 클리어이고 NO PLAY 와 FAILED 만 클리어가 아니다', () => {
        for (const lamp of SERVER_CLEAR_LAMPS) expect([lamp, clearLampToId(lamp) >= MIN_CLEAR_LAMP_ID]).toEqual([lamp, !NOT_CLEARS.includes(lamp)])
        expect(clearLampToId('AssistEasy')).toBeGreaterThanOrEqual(MIN_CLEAR_LAMP_ID)
        expect(clearLampToId('LightAssistEasy')).toBeGreaterThanOrEqual(MIN_CLEAR_LAMP_ID)
        expect(clearLampToId('Failed')).toBeLessThan(MIN_CLEAR_LAMP_ID)
    })

    test('화면 쪽 램프 목록과 클리어 기준이 서버와 같다', () => {
        expect([...CLIENT_CLEAR_LAMPS]).toEqual([...SERVER_CLEAR_LAMPS])
        expect(CLIENT_CLEAR_LAMPS.indexOf(MIN_CLEAR_LAMP)).toBe(MIN_CLEAR_LAMP_ID)
    })
})

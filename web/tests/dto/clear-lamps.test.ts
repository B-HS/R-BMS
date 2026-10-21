import { describe, expect, test } from 'bun:test'
import {
    ASSIST_GRADES as SERVER_ASSIST_GRADES,
    ASSIST_GRADE_NAMES,
    CLEAR_LAMPS as SERVER_CLEAR_LAMPS,
    MIN_CLEAR_LAMP_ID,
    clearLampToId,
} from '@server/dto/common'
import { readFileSync } from 'node:fs'
import {
    ASSIST_GRADES as CLIENT_ASSIST_GRADES,
    ASSIST_GRADE_BACKGROUND,
    ASSIST_GRADE_LABEL,
    CLEAR_LAMPS as CLIENT_CLEAR_LAMPS,
    CLEAR_LAMP_LABEL,
    DISPLAY_LAMPS,
    LAMP_BACKGROUND,
    MIN_CLEAR_LAMP,
    displayLampRank,
    lampLabel,
} from '../../src/shared/constants/clear-lamp'

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

describe('어시스트 등급 표시', () => {
    test('화면 쪽 등급 목록과 이름이 서버와 같다', () => {
        expect([...CLIENT_ASSIST_GRADES]).toEqual([...SERVER_ASSIST_GRADES])
        expect(CLIENT_ASSIST_GRADES.map((grade) => ASSIST_GRADE_LABEL[grade])).toEqual([...ASSIST_GRADE_NAMES])
    })

    test('ASSIST 는 id 2, LIGHT ASSIST 는 등급 0 이고 서로 이름이 겹치지 않는다', () => {
        expect(CLEAR_LAMP_LABEL.AssistEasy).toBe('ASSIST')
        expect(CLEAR_LAMP_LABEL.LightAssistEasy).toBe(ASSIST_GRADE_LABEL[0])
        const labels = DISPLAY_LAMPS.map(({ lamp, assistGrade }) => lampLabel(lamp, assistGrade))
        expect(new Set(labels).size).toBe(labels.length)
    })

    test('표시 램프는 11종에서 LIGHT ASSIST 를 다섯 등급으로 펼친 15칸이고 등급이 높을수록 위이며 EASY 아래다', () => {
        expect(DISPLAY_LAMPS).toHaveLength(SERVER_CLEAR_LAMPS.length + SERVER_ASSIST_GRADES.length - 1)
        const ranks = SERVER_ASSIST_GRADES.map((grade) => displayLampRank('LightAssistEasy', grade))
        expect(ranks).toEqual(ranks.toSorted((left, right) => left - right))
        expect(displayLampRank('AssistEasy')).toBeLessThan(ranks[0])
        expect(ranks[4]).toBeLessThan(displayLampRank('Easy'))
    })

    test('모든 램프와 등급의 색 토큰이 globals.css 에 서로 다른 값으로 정의돼 있다', () => {
        const css = readFileSync(new URL('../../src/app/globals.css', import.meta.url), 'utf8')
        const tokens = [...Object.values(LAMP_BACKGROUND), ...CLIENT_ASSIST_GRADES.map((grade) => ASSIST_GRADE_BACKGROUND[grade])]
        const values = [...new Set(tokens)].map((token) => {
            const name = token.replace(/^var\((.+)\)$/, '$1')
            const match = css.match(new RegExp(`${name}:\\s*(#[0-9a-f]{6});`))
            expect(match).not.toBeNull()
            return match?.[1]
        })
        expect(new Set(values).size).toBe(values.length)
    })
})

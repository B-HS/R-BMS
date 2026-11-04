import { describe, expect, test } from 'bun:test'
import { PLAY_DAYS_WINDOW, playDaysWindowOf, summarisePlayDays } from '@server/service/domain/fe/player-play-days'
import { DAY_MS, MINUTE_MS, TODAY_START, noonOf } from './dashboard-fixture'

const SAMPLE_PLAYS = [noonOf(40), noonOf(40, 3), noonOf(12), noonOf(1, 5), noonOf(0), noonOf(0, 4), noonOf(0, 6)]
const WIDE_WINDOW_DAYS = 371

describe('집계 창 길이 인자', () => {
    test('길이를 주지 않으면 30일 창이고 길이를 주면 그 길이만큼 거슬러 올라간다', () => {
        expect(playDaysWindowOf(TODAY_START)).toEqual({ from: TODAY_START - (PLAY_DAYS_WINDOW - 1) * DAY_MS, to: TODAY_START + DAY_MS })
        expect(playDaysWindowOf(TODAY_START, 1)).toEqual({ from: TODAY_START, to: TODAY_START + DAY_MS })
        expect(playDaysWindowOf(TODAY_START, WIDE_WINDOW_DAYS).from).toBe(TODAY_START - 370 * DAY_MS)
    })

    test('길이를 주지 않은 결과는 30일 길이를 명시한 결과와 같다', () => {
        const input = { playedAts: SAMPLE_PLAYS, previousPlayedAt: null, todayStart: TODAY_START }
        expect(summarisePlayDays(input)).toEqual(summarisePlayDays({ ...input, days: PLAY_DAYS_WINDOW }))
        expect(summarisePlayDays(input).days).toHaveLength(PLAY_DAYS_WINDOW)
    })

    test('넓은 창의 마지막 30일은 30일 창의 결과와 날마다 같다', () => {
        const narrow = summarisePlayDays({ playedAts: SAMPLE_PLAYS, previousPlayedAt: noonOf(40, 3), todayStart: TODAY_START })
        const wide = summarisePlayDays({ playedAts: SAMPLE_PLAYS, previousPlayedAt: noonOf(41), todayStart: TODAY_START, days: WIDE_WINDOW_DAYS })
        expect(wide.days).toHaveLength(WIDE_WINDOW_DAYS)
        expect(wide.days.slice(-PLAY_DAYS_WINDOW)).toEqual(narrow.days)
    })

    test('넓은 창은 30일 밖의 판도 날짜에 맞게 센다', () => {
        const wide = summarisePlayDays({ playedAts: SAMPLE_PLAYS, previousPlayedAt: null, todayStart: TODAY_START, days: WIDE_WINDOW_DAYS })
        expect(wide.total_plays).toBe(SAMPLE_PLAYS.length)
        expect(wide.days.at(-41)).toMatchObject({ date: '2026-09-26', plays: 2 })
        expect(wide.days.at(-41)?.play_ms).toBe(2 * MINUTE_MS + 3 * MINUTE_MS)
    })

    test('창 밖의 판은 세지 않고 길이 1 창은 오늘만 본다', () => {
        const only = summarisePlayDays({ playedAts: SAMPLE_PLAYS, previousPlayedAt: noonOf(1, 5), todayStart: TODAY_START, days: 1 })
        expect(only.days).toEqual([{ date: '2026-11-05', plays: 3, play_ms: 2 * MINUTE_MS + 4 * MINUTE_MS + 2 * MINUTE_MS }])
        expect(only.total_plays).toBe(3)
    })
})

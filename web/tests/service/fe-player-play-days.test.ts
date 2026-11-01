import { describe, expect, test } from 'bun:test'
import { createFeService, type FeServiceDb } from '@server/service/domain/fe/fe.service'
import {
    PLAY_DAYS_WINDOW,
    PLAY_DEFAULT_MS,
    PLAY_SESSION_GAP_MS,
    kstDayStartOf,
    playDaysWindowOf,
    summarisePlayDays,
} from '@server/service/domain/fe/player-play-days'
import { formatDateTime } from '@shared/lib/format'

const MINUTE_MS = 60_000
const HOUR_MS = 3_600_000
const DAY_MS = 86_400_000
const LAST_DAY_INDEX = PLAY_DAYS_WINDOW - 1
const YESTERDAY_INDEX = PLAY_DAYS_WINDOW - 2

const TODAY_START = Date.UTC(2026, 10, 1, 15, 0, 0)
const WINDOW_START = TODAY_START - LAST_DAY_INDEX * DAY_MS
const NOON = TODAY_START + 12 * HOUR_MS

const summarise = (playedAts: number[], previousPlayedAt: number | null = null) =>
    summarisePlayDays({ playedAts, previousPlayedAt, todayStart: TODAY_START })

describe('한국 시간 하루의 시작', () => {
    test('UTC 15:00 직전은 앞날이고 UTC 15:00 부터 새 날이다', () => {
        expect(kstDayStartOf(TODAY_START - 1)).toBe(TODAY_START - DAY_MS)
        expect(kstDayStartOf(TODAY_START)).toBe(TODAY_START)
        expect(kstDayStartOf(TODAY_START + DAY_MS - 1)).toBe(TODAY_START)
    })

    test('화면이 쓰는 표시 시간대의 자정과 같다', () => {
        for (const at of [Date.UTC(2026, 0, 15, 3, 20), Date.UTC(2026, 6, 15, 14, 59), Date.UTC(2026, 6, 15, 15, 1)]) {
            const start = kstDayStartOf(at)
            expect(formatDateTime(start).endsWith(' 00:00')).toBe(true)
            expect(formatDateTime(start - 1).endsWith(' 23:59')).toBe(true)
            expect(start <= at && at < start + DAY_MS).toBe(true)
        }
    })

    test('집계 창은 오늘을 포함한 최근 30일이다', () => {
        expect(playDaysWindowOf(TODAY_START)).toEqual({ from: WINDOW_START, to: TODAY_START + DAY_MS })
        expect(PLAY_DAYS_WINDOW).toBe(30)
    })
})

describe('summarisePlayDays 는 판 끝 시각 간격으로 하루별 플레이 시간을 추정한다', () => {
    test('앞 판과의 간격이 10분 이하면 그 간격을, 넘으면 기본 2분을 센다', () => {
        const result = summarise([NOON, NOON + 5 * MINUTE_MS, NOON + 15 * MINUTE_MS, NOON + 26 * MINUTE_MS])
        expect(PLAY_SESSION_GAP_MS).toBe(10 * MINUTE_MS)
        expect(PLAY_DEFAULT_MS).toBe(2 * MINUTE_MS)
        expect(result.days[LAST_DAY_INDEX]).toEqual({ date: '2026-11-02', plays: 4, play_ms: (2 + 5 + 10 + 2) * MINUTE_MS })
    })

    test('창 안의 첫 판은 앞 판이 없으면 기본 2분이다', () => {
        expect(summarise([NOON]).days[LAST_DAY_INDEX].play_ms).toBe(PLAY_DEFAULT_MS)
    })

    test('입력 순서와 상관없이 오름차순으로 센다', () => {
        const ordered = [NOON, NOON + 3 * MINUTE_MS, NOON + 4 * MINUTE_MS]
        expect(summarise(ordered.toReversed())).toEqual(summarise(ordered))
        expect(summarise(ordered).days[LAST_DAY_INDEX].play_ms).toBe((2 + 3 + 1) * MINUTE_MS)
    })

    test('같은 시각의 두 판은 뒤 판이 0분이다', () => {
        expect(summarise([NOON, NOON]).days[LAST_DAY_INDEX]).toMatchObject({ plays: 2, play_ms: PLAY_DEFAULT_MS })
    })

    test('창 바로 앞의 판은 첫날 첫 판의 간격 계산에만 쓰이고 세지 않는다', () => {
        const joined = summarise([WINDOW_START + 4 * MINUTE_MS], WINDOW_START - 3 * MINUTE_MS)
        expect(joined.days[0]).toMatchObject({ plays: 1, play_ms: 7 * MINUTE_MS })
        expect(joined.total_plays).toBe(1)
        const apart = summarise([WINDOW_START + MINUTE_MS], WINDOW_START - 10 * MINUTE_MS)
        expect(apart.days[0].play_ms).toBe(PLAY_DEFAULT_MS)
        expect(summarise([WINDOW_START + MINUTE_MS], null).days[0].play_ms).toBe(PLAY_DEFAULT_MS)
    })

    test('한국 시간 자정을 넘는 판은 판 끝 시각이 속한 날에 더한다', () => {
        const beforeMidnight = TODAY_START - MINUTE_MS
        const afterMidnight = TODAY_START + 4 * MINUTE_MS
        const result = summarise([beforeMidnight, afterMidnight])
        expect(result.days[YESTERDAY_INDEX]).toEqual({ date: '2026-11-01', plays: 1, play_ms: PLAY_DEFAULT_MS })
        expect(result.days[LAST_DAY_INDEX]).toEqual({ date: '2026-11-02', plays: 1, play_ms: 5 * MINUTE_MS })
    })

    test('UTC 15:00 전후 1밀리초는 날이 갈리고 UTC 날짜가 아닌 한국 날짜로 센다', () => {
        const result = summarise([TODAY_START - 1, TODAY_START, Date.UTC(2026, 10, 1, 16, 0, 0)])
        expect(result.days[YESTERDAY_INDEX]).toMatchObject({ date: '2026-11-01', plays: 1 })
        expect(result.days[LAST_DAY_INDEX]).toMatchObject({ date: '2026-11-02', plays: 2 })
    })

    test('판이 없는 날도 30칸 모두 날짜와 함께 0 으로 돌려준다', () => {
        const result = summarise([NOON])
        expect(result.days).toHaveLength(PLAY_DAYS_WINDOW)
        expect(result.days[0].date).toBe('2026-10-04')
        expect(result.days.slice(0, LAST_DAY_INDEX).every((day) => day.plays === 0 && day.play_ms === 0)).toBe(true)
        expect(new Set(result.days.map((day) => day.date)).size).toBe(PLAY_DAYS_WINDOW)
    })

    test('판이 하나도 없으면 합계가 0 이다', () => {
        expect(summarise([])).toMatchObject({ total_plays: 0, total_ms: 0 })
    })

    test('창 밖의 판은 세지 않는다', () => {
        const result = summarise([WINDOW_START - 1, NOON, TODAY_START + DAY_MS])
        expect(result.total_plays).toBe(1)
        expect(result.days[0].plays).toBe(0)
    })

    test('합계 판 수와 시간은 날별 값의 합이다', () => {
        const result = summarise([WINDOW_START + HOUR_MS, WINDOW_START + HOUR_MS + 4 * MINUTE_MS, NOON])
        expect(result.total_plays).toBe(3)
        expect(result.total_ms).toBe(result.days.reduce((sum, day) => sum + day.play_ms, 0))
        expect(result.total_ms).toBe((2 + 4 + 2) * MINUTE_MS)
    })

    test('코스 스테이지처럼 몇 분 간격으로 이어진 판은 곡별 판이 따로 있으므로 간격 그대로 센다', () => {
        const stages = [NOON, NOON + 2 * MINUTE_MS, NOON + 4 * MINUTE_MS, NOON + 7 * MINUTE_MS]
        expect(summarise(stages).total_ms).toBe((2 + 2 + 2 + 3) * MINUTE_MS)
    })
})

const stubDb = (overrides: Partial<FeServiceDb> = {}): FeServiceDb => ({
    searchCharts: async () => ({ rows: [], total: 0 }),
    listRecentActivity: async () => [],
    listPlayerRecent: async () => ({ rows: [], total: 0 }),
    listPlayerLeaderboard: async () => ({ rows: [], total: 0 }),
    countLampsByUser: async () => [],
    listPlayerPlayedAts: async () => ({ playedAts: [], previousPlayedAt: null }),
    summarise: async () => ({ charts: 0, players: 0, scores: 0 }),
    listTokens: async () => [],
    deleteToken: async () => false,
    ...overrides,
})

describe('feService.playerPlayDays', () => {
    test('오늘 시작 시각에서 정한 창을 DB 에 넘기고 돌려받은 판 끝 시각으로 집계한다', async () => {
        const seen: unknown[] = []
        const service = createFeService({
            listDisplayedDans: async () => new Map(),
            db: stubDb({
                listPlayerPlayedAts: async (params) => {
                    seen.push(params)
                    return { playedAts: [WINDOW_START + MINUTE_MS, NOON], previousPlayedAt: WINDOW_START - 2 * MINUTE_MS }
                },
            }),
        })
        const result = await service.playerPlayDays({ userId: 'u_alice', todayStart: TODAY_START })
        expect(seen).toEqual([{ userId: 'u_alice', from: WINDOW_START, to: TODAY_START + DAY_MS }])
        expect(result.days[0]).toEqual({ date: '2026-10-04', plays: 1, play_ms: 3 * MINUTE_MS })
        expect(result.days[LAST_DAY_INDEX]).toEqual({ date: '2026-11-02', plays: 1, play_ms: PLAY_DEFAULT_MS })
        expect(result).toMatchObject({ total_plays: 2, total_ms: 5 * MINUTE_MS })
    })

    test('판이 없는 플레이어는 30칸 모두 0 이다', async () => {
        const service = createFeService({ listDisplayedDans: async () => new Map(), db: stubDb() })
        const result = await service.playerPlayDays({ userId: 'u_nobody', todayStart: TODAY_START })
        expect(result).toMatchObject({ total_plays: 0, total_ms: 0 })
        expect(result.days).toHaveLength(PLAY_DAYS_WINDOW)
    })
})

import type {
    DashboardCalendar,
    DashboardCourses,
    DashboardOverview,
    DashboardReplays,
    DashboardRivals,
    DashboardTableProgress,
} from '../../src/entities/dashboard.type'
import type { PlayerPlayDays, PlayerProfile, PlayerRecentRow, PlayerStats } from '../../src/entities/player.type'

export const LOGIN_ID = 'alice'
export const USER_ID = 'user-alice'
export const LAST_PLAYED_AT = 1_780_000_000_000

const SHA = 'a'.repeat(64)
const MD5 = 'b'.repeat(32)
const DAY_MS = 86_400_000
const CALENDAR_FIRST_DAY = Date.UTC(2026, 4, 6)
const CALENDAR_DAYS = 182
const CALENDAR_PLAY_CYCLE = 12
const PLAY_DAY_COUNT = 30
const PLAY_DAY_FIRST_DAY = Date.UTC(2026, 9, 4)
const MINUTE_MS = 60_000

const GRADES = ['F', 'E', 'D', 'C', 'B', 'A', 'AA', 'AAA', 'MAX'] as const

export const overview = (overrides: Partial<DashboardOverview> = {}): DashboardOverview => ({
    tiles: {
        total_plays: 1234,
        played_charts: 400,
        cleared_charts: 300,
        hard_or_better: 120,
        full_combo_or_better: 40,
        play_ms_30d: 7_200_000,
        active_days_30d: 12,
        streak_days: 3,
        best_streak_days: 9,
        this_week_plays: 21,
        last_played_at: LAST_PLAYED_AT,
    },
    grades: GRADES.map((grade, index) => ({ grade, count: index + 1 })),
    rank: { position: 3, total_players: 120 },
    ...overrides,
})

export const emptyOverview = (): DashboardOverview =>
    overview({
        tiles: {
            ...overview().tiles,
            total_plays: 0,
            played_charts: 0,
            cleared_charts: 0,
            hard_or_better: 0,
            full_combo_or_better: 0,
            last_played_at: null,
        },
        rank: { position: null, total_players: 120 },
    })

export const calendarDay = (index: number) => new Date(CALENDAR_FIRST_DAY + index * DAY_MS).toISOString().slice(0, 10)

export const calendar = (): DashboardCalendar => {
    const days = Array.from({ length: CALENDAR_DAYS }, (_, index) => ({ date: calendarDay(index), plays: index % CALENDAR_PLAY_CYCLE, play_ms: 0 }))
    return {
        days,
        total_plays: days.reduce((sum, day) => sum + day.plays, 0),
        active_days: days.filter((day) => day.plays > 0).length,
        streak_days: 4,
        best_streak_days: 11,
    }
}

export const tableProgress = (): DashboardTableProgress => ({
    table_id: 'insane1',
    name: '発狂BMS難易度表',
    symbol: '★',
    total: 10,
    played: 6,
    lamps: [
        { clear: 'Hard', assist_grade: 0, count: 2 },
        { clear: 'Normal', assist_grade: 0, count: 3 },
        { clear: 'Failed', assist_grade: 0, count: 1 },
        { clear: 'NoPlay', assist_grade: 0, count: 4 },
    ],
    levels: [
        {
            level: '1',
            total: 4,
            played: 4,
            cleared: 3,
            hard_or_better: 1,
            lamps: [
                { clear: 'Hard', assist_grade: 0, count: 1 },
                { clear: 'Normal', assist_grade: 0, count: 2 },
                { clear: 'Failed', assist_grade: 0, count: 1 },
            ],
        },
        {
            level: '2',
            total: 6,
            played: 2,
            cleared: 2,
            hard_or_better: 1,
            lamps: [
                { clear: 'Hard', assist_grade: 0, count: 1 },
                { clear: 'Normal', assist_grade: 0, count: 1 },
                { clear: 'NoPlay', assist_grade: 0, count: 4 },
            ],
        },
    ],
})

export const courses = (): DashboardCourses => ({
    groups: [
        { id: 'stella', group: 'Stella', total: 10, played: 6, cleared: 4 },
        { id: 'satellite', group: 'Satellite', total: 8, played: 0, cleared: 0 },
    ],
    recent: [{ course_hash: 'c'.repeat(64), name: 'Stella st3', clear: 'Hard', ex_score: 2345, played_at: LAST_PLAYED_AT }],
})

export const rivals = (): DashboardRivals => ({
    rivals: [
        { login_id: 'bob', name: 'Bob' },
        { login_id: 'carol', name: 'Carol' },
    ],
    recent: [
        {
            score_id: 'score-bob',
            player: { id: 'bob' },
            player_name: 'Bob',
            clear: 'ExHard',
            assist_grade: 0,
            ex_score: 1500,
            minbp: 3,
            max_combo: 400,
            played_at: LAST_PLAYED_AT,
            scratch_auto: false,
            chart: { md5: MD5, sha256: SHA, title: 'Rival chart', artist: 'artist', level: 12 },
        },
    ],
    versus: [
        { login_id: 'bob', common: 10, wins: 6, losses: 3, draws: 1 },
        { login_id: 'carol', common: 0, wins: 0, losses: 0, draws: 0 },
    ],
})

export const replays = (): DashboardReplays => ({
    replays: [
        {
            id: 'replay-1',
            created_at: LAST_PLAYED_AT,
            chart: { md5: MD5, sha256: SHA, title: 'Replay chart', level: 12 },
            clear: 'Hard',
            ex_score: 1800,
        },
        {
            id: 'replay-2',
            created_at: LAST_PLAYED_AT - DAY_MS,
            chart: { md5: MD5, sha256: SHA, title: '', level: null },
            clear: null,
            ex_score: null,
        },
    ],
})

export const profile = (): PlayerProfile => ({
    id: LOGIN_ID,
    name: 'Alice',
    total_plays: 1234,
    rank_points: 0,
    extra: { dans: [{ system: 'stella', name: 'Stella', label: 'st3', assisted: false }] },
})

export const stats = (): PlayerStats => ({
    total_plays: 1234,
    cleared: 300,
    lamps: [
        { clear: 'Hard', assist_grade: 0, count: 120 },
        { clear: 'Normal', assist_grade: 0, count: 180 },
        { clear: 'Failed', assist_grade: 0, count: 100 },
    ],
    scratch_auto_charts: 7,
})

export const playDays = (): PlayerPlayDays => {
    const days = Array.from({ length: PLAY_DAY_COUNT }, (_, index) => ({
        date: new Date(PLAY_DAY_FIRST_DAY + index * DAY_MS).toISOString().slice(0, 10),
        plays: index === PLAY_DAY_COUNT - 1 ? 3 : 0,
        play_ms: index === PLAY_DAY_COUNT - 1 ? 65 * MINUTE_MS : 0,
    }))
    return { days, total_plays: 3, total_ms: 65 * MINUTE_MS }
}

export const recentRow = (scoreId: string, overrides: Partial<PlayerRecentRow> = {}): PlayerRecentRow => ({
    score_id: scoreId,
    player: { id: LOGIN_ID },
    player_name: 'Alice',
    clear: 'ExHard',
    assist_grade: 0,
    ex_score: 1500,
    minbp: 4,
    max_combo: 300,
    played_at: LAST_PLAYED_AT,
    scratch_auto: false,
    chart: { md5: MD5, sha256: SHA, title: scoreId, artist: 'artist', level: 12 },
    ...overrides,
})

export const envelope = (data: unknown) =>
    new Response(JSON.stringify({ success: true, data }), { status: 200, headers: { 'content-type': 'application/json' } })

export const failure = () =>
    new Response(JSON.stringify({ success: false, error: { code: 'INTERNAL_ERROR', message: '서버 오류' } }), {
        status: 500,
        headers: { 'content-type': 'application/json' },
    })

type FetchAnswer = Response | 'pending'

/** Replaces `fetch` for one test: `answer` decides per URL, `'pending'` never settles, and `calls` lists the URLs asked for. */
export const stubFetch = (answer: (url: string) => FetchAnswer) => {
    const original = globalThis.fetch
    const calls: string[] = []
    globalThis.fetch = Object.assign(async (input: RequestInfo | URL) => {
        const url = String(input)
        calls.push(url)
        const result = answer(url)
        return result === 'pending' ? new Promise<Response>(() => undefined) : result
    }, original)
    return { calls, restore: () => void (globalThis.fetch = original) }
}

type Observer = { callback: IntersectionObserverCallback; element: Element | null; disconnected: boolean }

/** A stand-in for `IntersectionObserver` whose `enter()` reports every observed element as on screen. */
export const installIntersectionObserver = () => {
    const original = globalThis.IntersectionObserver
    const observers: Observer[] = []
    class FakeIntersectionObserver {
        private readonly observer: Observer
        constructor(callback: IntersectionObserverCallback) {
            this.observer = { callback, element: null, disconnected: false }
            observers.push(this.observer)
        }
        observe(element: Element) {
            this.observer.element = element
        }
        disconnect() {
            this.observer.disconnected = true
        }
        unobserve() {}
        takeRecords() {
            return []
        }
    }
    globalThis.IntersectionObserver = FakeIntersectionObserver as unknown as typeof IntersectionObserver
    return {
        observers,
        enter: () => {
            for (const observer of observers.filter((candidate) => !candidate.disconnected && candidate.element)) {
                observer.callback(
                    [{ isIntersecting: true, target: observer.element } as IntersectionObserverEntry],
                    observer as unknown as IntersectionObserver,
                )
            }
        },
        restore: () => void (globalThis.IntersectionObserver = original),
    }
}

import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const { cleanup, fireEvent, render } = await import('@testing-library/react')
const { PlayerStatsPanel } = await import('../../src/widgets/player/player-stats-panel')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { withLocale } = await import('../support/browse-intl')

type PlayerPlayDays = import('../../src/entities/player.type').PlayerPlayDays

const PLAYER_ID = 'alice'
const DAY_COUNT = 30
const DAY_MS = 86_400_000
const FIRST_DAY = Date.UTC(2026, 9, 4)
const MINUTE_MS = 60_000

const playDays = (playedDays: Record<number, { plays: number; minutes: number }>): PlayerPlayDays => {
    const days = Array.from({ length: DAY_COUNT }, (_, index) => ({
        date: new Date(FIRST_DAY + index * DAY_MS).toISOString().slice(0, 10),
        plays: playedDays[index]?.plays ?? 0,
        play_ms: (playedDays[index]?.minutes ?? 0) * MINUTE_MS,
    }))
    return {
        days,
        total_plays: days.reduce((sum, day) => sum + day.plays, 0),
        total_ms: days.reduce((sum, day) => sum + day.play_ms, 0),
    }
}

const renderPanel = () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity, retry: false } } })
    queryClient.setQueryData(QUERY_KEY.PLAYER.STATS(PLAYER_ID), {
        total_plays: 2000,
        cleared: 900,
        lamps: [{ clear: 'Hard', assist_grade: 0, count: 1234 }],
        scratch_auto_charts: 0,
    })
    const view = render(<QueryClientProvider client={queryClient}>{withLocale(<PlayerStatsPanel playerId={PLAYER_ID} />)}</QueryClientProvider>)
    return { view, queryClient }
}

const playDaysKey = QUERY_KEY.PLAYER.PLAY_DAYS(PLAYER_ID)

afterEach(() => cleanup())

describe('플레이어 통계 패널의 모드 전환', () => {
    test('처음엔 램프 분포이고 일별 플레이 시간은 모드를 바꾼 뒤에야 조회해 그래프와 표로 보여 준다', async () => {
        const { view, queryClient } = renderPanel()
        expect(view.getByRole('heading', { name: '램프 분포' })).toBeTruthy()
        expect(view.getByRole('radio', { name: '램프 분포' }).getAttribute('aria-checked')).toBe('true')
        expect(view.getByText('클리어한 차트')).toBeTruthy()
        expect(view.queryByRole('heading', { name: '일별 플레이 시간' })).toBeNull()
        expect(queryClient.getQueryState(playDaysKey)).toBeUndefined()

        queryClient.setQueryData(playDaysKey, playDays({ 28: { plays: 2, minutes: 65 }, 29: { plays: 3, minutes: 7 } }))
        fireEvent.click(view.getByRole('radio', { name: '일별 플레이 시간' }))
        expect(view.getByRole('radio', { name: '일별 플레이 시간' }).getAttribute('aria-checked')).toBe('true')
        expect(view.getByRole('heading', { name: '일별 플레이 시간' })).toBeTruthy()
        expect(view.queryByText('클리어한 차트')).toBeNull()

        const chart = await view.findByLabelText('최근 30일 일별 플레이 시간 그래프')
        expect(chart.querySelectorAll('[role="img"]')).toHaveLength(DAY_COUNT)
        expect(view.getByRole('img', { name: '2026-11-01 · 추정 1:05' })).toBeTruthy()
        expect(view.getByRole('img', { name: '2026-10-04 · 추정 0:00' })).toBeTruthy()
        expect(view.getByText('최근 30일 합계 1:12')).toBeTruthy()
        expect(view.getAllByRole('row').map((row) => [...row.querySelectorAll('th, td')].map((cell) => cell.textContent))).toEqual([
            ['날짜', '판 수', '추정 시간'],
            ['2026-11-02', '3', '0:07'],
            ['2026-11-01', '2', '1:05'],
        ])
        expect(view.getByText('판과 판 사이 간격으로 추정한 시간입니다 (한국 시간 기준).')).toBeTruthy()

        fireEvent.click(view.getByRole('radio', { name: '램프 분포' }))
        expect(view.getByRole('heading', { name: '램프 분포' })).toBeTruthy()
        expect(view.queryByLabelText('최근 30일 일별 플레이 시간 그래프')).toBeNull()
    })

    test('최근 30일에 판이 하나도 없으면 빈 상태를 보여 준다', async () => {
        const { view, queryClient } = renderPanel()
        queryClient.setQueryData(playDaysKey, playDays({}))
        fireEvent.click(view.getByRole('radio', { name: '일별 플레이 시간' }))
        expect(await view.findByText('최근 30일 동안 기록이 없습니다.')).toBeTruthy()
        expect(view.queryAllByRole('row')).toHaveLength(0)
    })
})

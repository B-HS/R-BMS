import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const { cleanup, render } = await import('@testing-library/react')
const { RecentActivityPanel } = await import('../../src/widgets/home/recent-activity-panel')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { withLocale } = await import('../support/browse-intl')

const RECENT_LIMIT = 20
const SHA = 'a'.repeat(64)

type Row = import('../../src/entities/stats.type').ActivityRow

const activityRow = (scoreId: string, scratchAuto: boolean): Row => ({
    score_id: scoreId,
    player: { id: 'alice' },
    player_name: 'Alice',
    clear: 'ExHard',
    assist_grade: 0,
    ex_score: 1500,
    minbp: 4,
    max_combo: 300,
    played_at: 1_780_000_000_000,
    scratch_auto: scratchAuto,
    chart: { md5: 'b'.repeat(32), sha256: SHA, title: scoreId, artist: 'artist', level: 12 },
})

const renderPanel = (rows: Row[]) => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } })
    queryClient.setQueryData(QUERY_KEY.ACTIVITY.RECENT({ limit: RECENT_LIMIT }), rows)
    return render(<QueryClientProvider client={queryClient}>{withLocale(<RecentActivityPanel />)}</QueryClientProvider>)
}

afterEach(() => cleanup())

describe('홈 최근 활동 패널', () => {
    test('SCRATCH AUTO 판에만 A-SCR 표지를 달고 안내 문구에 표지 뜻을 적는다', () => {
        const view = renderPanel([activityRow('Scratch chart', true), activityRow('Normal chart', false)])
        const items = view.getAllByRole('listitem')
        expect(items).toHaveLength(2)
        expect(items[0]?.textContent).toContain('A-SCR')
        expect(items[1]?.textContent).not.toContain('A-SCR')
        expect(view.getByText('순위에 반영되는 제출만 표시됩니다. A-SCR 은 SCRATCH AUTO 로 친 기록입니다.')).toBeTruthy()
    })
})

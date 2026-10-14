import { describe, expect, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderToStaticMarkup } from 'react-dom/server'
import { ReplayDetailPanel } from '../../src/widgets/replay/replay-detail-panel'
import { QUERY_KEY } from '../../src/shared/constants/query-key'
import { SCRATCH_AUTO_BADGE_LABEL } from '../../src/shared/constants/scratch-auto'
import { SidebarProvider } from '../../src/shared/ui/sidebar'
import type { ReplayData } from '../../src/entities/replay.type'

const REPLAY_ID = 'replay-1'

const replayData = (scratchAuto: boolean): ReplayData => ({
    api_version: 1,
    id: REPLAY_ID,
    format: 'rbms-replay',
    chart: { md5: 'a'.repeat(32), sha256: 'b'.repeat(64) },
    score_id: null,
    mode: 'SP',
    random: 'NONRAN',
    random_p2: null,
    seed: 1,
    lntype: 0,
    offset_ms: 0,
    judge_rate: 100,
    scratch_auto: scratchAuto,
    constant: false,
    gauge: 'NORMAL',
    client_build_sha256: null,
    events: [],
    event_count: 0,
    duration_us: 1_000_000,
    size: 0,
    extra: {},
})

const renderPanel = (data: ReplayData) => {
    const queryClient = new QueryClient()
    queryClient.setQueryData(QUERY_KEY.REPLAY.DETAIL(REPLAY_ID), data)
    return renderToStaticMarkup(
        <QueryClientProvider client={queryClient}>
            <SidebarProvider>
                <ReplayDetailPanel replayId={REPLAY_ID} />
            </SidebarProvider>
        </QueryClientProvider>,
    )
}

describe('리플레이 상세 SCRATCH AUTO 표지', () => {
    test('SCRATCH AUTO 로 친 리플레이에는 A-SCR 배지를 붙인다', () => {
        expect(renderPanel(replayData(true))).toContain(SCRATCH_AUTO_BADGE_LABEL)
    })

    test('일반 판 리플레이에는 A-SCR 배지를 붙이지 않는다', () => {
        expect(renderPanel(replayData(false))).not.toContain(SCRATCH_AUTO_BADGE_LABEL)
    })
})

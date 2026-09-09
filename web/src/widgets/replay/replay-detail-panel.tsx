'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { MetaBadge } from '@features/badge/meta-badge'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { PageHeading } from '@features/shell/page-heading'
import { MAX_RENDERED_REPLAY_EVENTS, ReplayWaterfall } from '@widgets/replay/replay-waterfall'
import { formatCount, formatHash } from '@shared/lib/format'
import { useReplayDetail } from '@entities/replay.query'

const MICROSECONDS_PER_SECOND = 1_000_000
const META_ROW_CLASS = 'flex items-center justify-between gap-4 py-1'

export const ReplayDetailPanel: FC<{ replayId: string }> = ({ replayId }) => {
    const { data, isLoading, isError } = useReplayDetail(replayId)

    if (isLoading) return <PanelSkeleton height='page' />
    if (isError || !data) return <LoadErrorState title='리플레이를 불러오지 못했습니다' />

    return (
        <>
            <PageHeading>리플레이 {data.id}</PageHeading>
            <PanelCard title='리플레이' contentClassName='flex flex-col gap-2'>
                <div className='flex flex-wrap gap-1'>
                    <MetaBadge>{data.mode}</MetaBadge>
                    <MetaBadge>{data.random}</MetaBadge>
                    <MetaBadge>seed {data.seed}</MetaBadge>
                    <MetaBadge>lntype {data.lntype}</MetaBadge>
                </div>
                <dl className='flex flex-col text-xs'>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>차트</dt>
                        <dd>
                            <Link href={`/charts/${data.chart.sha256}`} className='font-mono hover:underline'>
                                {formatHash(data.chart.sha256)}
                            </Link>
                        </dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>스코어</dt>
                        <dd className='font-mono'>{data.score_id ?? '—'}</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>이벤트</dt>
                        <dd className='tabular-nums'>{formatCount(data.event_count)}</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>길이</dt>
                        <dd className='tabular-nums'>{(data.duration_us / MICROSECONDS_PER_SECOND).toFixed(3)}초</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>빌드</dt>
                        <dd className='font-mono'>{data.client_build_sha256 ? formatHash(data.client_build_sha256) : '—'}</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>크기</dt>
                        <dd className='tabular-nums'>{formatCount(data.size)} bytes</dd>
                    </div>
                </dl>
            </PanelCard>
            <PanelCard title='입력 워터폴' contentClassName='flex flex-col gap-2'>
                <ReplayWaterfall events={data.events} durationUs={data.duration_us} />
                <PanelFootnote>
                    이벤트 타임스탬프는 µs 원본입니다. 가로 1초 = 96px, 키보드 내비게이션은 지원하지 않습니다.
                    {data.events.length > MAX_RENDERED_REPLAY_EVENTS &&
                        ` 전체 ${formatCount(data.events.length)}건 중 앞 ${formatCount(MAX_RENDERED_REPLAY_EVENTS)}건만 그립니다.`}
                </PanelFootnote>
            </PanelCard>
        </>
    )
}

'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { MetaBadge } from '@features/badge/meta-badge'
import { ScratchAutoBadge } from '@features/score/scratch-auto-badge'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { MAX_RENDERED_REPLAY_EVENTS, ReplayWaterfall } from '@widgets/replay/replay-waterfall'
import { ApiError } from '@shared/lib/fetch'
import { formatCount, formatHash } from '@shared/lib/format'
import { useReplayDetail } from '@entities/replay.query'

const NOT_FOUND_STATUS = 404
const MICROSECONDS_PER_SECOND = 1_000_000
const META_ROW_CLASS = 'flex items-center justify-between gap-4 py-1'

export const ReplayDetailPanel: FC<{ replayId: string }> = ({ replayId }) => {
    const t = useTranslations('browse.replay')
    const { data, isLoading, isError, error, isFetching, refetch } = useReplayDetail(replayId)

    if (isLoading) return <ShellPageLoading title={t('pageTitle')} />

    if (error instanceof ApiError && error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={t('pageTitle')}>
                <NotFoundNotice title={t('notFoundTitle')} description={t('notFoundDescription')} />
            </ShellPage>
        )

    if (isError || !data)
        return (
            <ShellPage title={t('pageTitle')}>
                <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
            </ShellPage>
        )

    return (
        <ShellPage title={t('pageTitle')} description={data.id}>
            <PanelCard title={t('pageTitle')} contentClassName='flex flex-col gap-2'>
                <div className='flex flex-wrap items-center gap-1'>
                    <MetaBadge>{data.mode}</MetaBadge>
                    <MetaBadge>{data.random}</MetaBadge>
                    <MetaBadge>seed {data.seed}</MetaBadge>
                    <MetaBadge>lntype {data.lntype}</MetaBadge>
                    {data.scratch_auto && <ScratchAutoBadge />}
                </div>
                <dl className='flex flex-col text-xs'>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>{t('chart')}</dt>
                        <dd>
                            <Link href={`/charts/${data.chart.sha256}`} className='font-mono hover:underline'>
                                {formatHash(data.chart.sha256)}
                            </Link>
                        </dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>{t('score')}</dt>
                        <dd className='font-mono'>{data.score_id ?? '—'}</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>{t('events')}</dt>
                        <dd className='tabular-nums'>{formatCount(data.event_count)}</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>{t('length')}</dt>
                        <dd className='tabular-nums'>{t('seconds', { seconds: (data.duration_us / MICROSECONDS_PER_SECOND).toFixed(3) })}</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>{t('build')}</dt>
                        <dd className='font-mono'>{data.client_build_sha256 ? formatHash(data.client_build_sha256) : '—'}</dd>
                    </div>
                    <div className={META_ROW_CLASS}>
                        <dt className='text-muted-foreground'>{t('size')}</dt>
                        <dd className='tabular-nums'>{t('bytes', { size: data.size })}</dd>
                    </div>
                </dl>
            </PanelCard>
            <PanelCard title={t('waterfallTitle')} contentClassName='flex flex-col gap-2'>
                <ReplayWaterfall events={data.events} durationUs={data.duration_us} />
                <PanelFootnote>
                    {data.events.length > MAX_RENDERED_REPLAY_EVENTS
                        ? t('waterfallFootnoteLimited', { total: data.events.length, shown: MAX_RENDERED_REPLAY_EVENTS })
                        : t('waterfallFootnote')}
                </PanelFootnote>
            </PanelCard>
        </ShellPage>
    )
}

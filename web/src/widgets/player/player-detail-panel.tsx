'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DanBadgeList } from '@features/dan/dan-badge-list'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { PlayerRecentPanel } from '@widgets/player/player-recent-panel'
import { PlayerStatsPanel } from '@widgets/player/player-stats-panel'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { readDisplayedDans } from '@entities/dan.type'
import { usePlayer } from '@entities/player.query'
import type { PlayerRecentParams } from '@entities/player.type'
import { ApiError } from '@shared/lib/fetch'

const NOT_FOUND_STATUS = 404

type PlayerDetailPanelProps = {
    playerId: string
    recentParams: PlayerRecentParams
}

export const PlayerDetailPanel: FC<PlayerDetailPanelProps> = ({ playerId, recentParams }) => {
    const t = useTranslations('browse.player.detail')
    const { data, isLoading, isError, error, isFetching, refetch } = usePlayer(playerId)

    if (isLoading) return <ShellPageLoading title={t('pageTitle')} />

    if (error instanceof ApiError && error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={t('pageTitle')}>
                <NotFoundNotice
                    title={t('notFoundTitle')}
                    description={t('notFoundDescription')}
                    actionHref='/leaderboards'
                    actionLabel={t('notFoundAction')}
                />
            </ShellPage>
        )

    if (isError || !data)
        return (
            <ShellPage title={t('pageTitle')}>
                <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
            </ShellPage>
        )

    return (
        <ShellPage title={data.name} description={`@${data.id}`}>
            <div className='bg-card flex flex-col gap-1 p-3'>
                <p className='text-lg leading-tight font-semibold tracking-tight wrap-anywhere'>{data.name}</p>
                <p className='text-muted-foreground font-mono text-xs'>@{data.id}</p>
                <DanBadgeList dans={readDisplayedDans(data.extra)} />
                <p className='text-muted-foreground text-xs'>
                    {t.rich('plays', {
                        total: data.total_plays,
                        value: (chunks) => <span className='text-foreground font-medium tabular-nums'>{chunks}</span>,
                    })}
                </p>
            </div>
            <PlayerStatsPanel playerId={playerId} />
            <PlayerRecentPanel playerId={playerId} params={recentParams} />
        </ShellPage>
    )
}

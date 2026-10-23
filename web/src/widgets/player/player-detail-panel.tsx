'use client'

import type { FC } from 'react'
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
import { formatCount } from '@shared/lib/format'

const PAGE_TITLE = '플레이어'
const NOT_FOUND_STATUS = 404

type PlayerDetailPanelProps = {
    playerId: string
    recentParams: PlayerRecentParams
}

export const PlayerDetailPanel: FC<PlayerDetailPanelProps> = ({ playerId, recentParams }) => {
    const { data, isLoading, isError, error, isFetching, refetch } = usePlayer(playerId)

    if (isLoading) return <ShellPageLoading title={PAGE_TITLE} />

    if (error instanceof ApiError && error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={PAGE_TITLE}>
                <NotFoundNotice
                    title='없는 플레이어입니다'
                    description='아이디가 바뀌었거나 가입하지 않은 플레이어입니다.'
                    actionHref='/leaderboards'
                    actionLabel='순위로'
                />
            </ShellPage>
        )

    if (isError || !data)
        return (
            <ShellPage title={PAGE_TITLE}>
                <LoadErrorState title='플레이어를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />
            </ShellPage>
        )

    return (
        <ShellPage title={data.name} description={`@${data.id}`}>
            <div className='bg-card flex flex-col gap-1 p-3'>
                <p className='text-lg leading-tight font-semibold tracking-tight wrap-anywhere'>{data.name}</p>
                <p className='text-muted-foreground font-mono text-xs'>@{data.id}</p>
                <DanBadgeList dans={readDisplayedDans(data.extra)} />
                <p className='text-muted-foreground text-xs'>
                    제출 <span className='text-foreground font-medium tabular-nums'>{formatCount(data.total_plays)}</span>
                </p>
            </div>
            <PlayerStatsPanel playerId={playerId} />
            <PlayerRecentPanel playerId={playerId} params={recentParams} />
        </ShellPage>
    )
}

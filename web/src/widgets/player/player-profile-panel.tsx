'use client'

import type { FC } from 'react'
import { PanelCard } from '@features/shell/panel-card'
import { MetaBadge } from '@features/badge/meta-badge'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { usePlayer } from '@entities/player.query'
import { formatCount } from '@shared/lib/format'

export const PlayerProfilePanel: FC<{ playerId: string }> = ({ playerId }) => {
    const { data, isLoading, isError } = usePlayer(playerId)

    if (isLoading) return <PanelSkeleton />
    if (isError) return <LoadErrorState title='플레이어를 불러오지 못했습니다' />
    if (!data) return null

    const rank = typeof data.extra.rank === 'string' ? data.extra.rank : null

    return (
        <PanelCard contentClassName='flex flex-col gap-2'>
            <h1 className='text-xl font-semibold tracking-tight'>{data.name}</h1>
            <div className='flex flex-wrap items-center gap-1'>
                {rank && <MetaBadge>rank {rank}</MetaBadge>}
                <MetaBadge>{formatCount(data.total_plays)} plays</MetaBadge>
            </div>
            <p className='text-muted-foreground flex flex-wrap gap-3 text-xs'>
                <span className='font-mono'>id {data.id}</span>
                <span>rank point {data.rank_points.toFixed(2)}</span>
            </p>
        </PanelCard>
    )
}

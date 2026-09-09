'use client'

import type { FC } from 'react'
import { StatTile } from '@features/shell/stat-tile'
import { LoadErrorState } from '@features/state/load-error-state'
import { useStatsSummary } from '@entities/stats.query'
import { formatCount } from '@shared/lib/format'
import { Skeleton } from '@shared/ui/skeleton'

const TILE_COUNT = 3

export const StatsSummaryPanel: FC = () => {
    const { data, isLoading, isError } = useStatsSummary()

    if (isLoading)
        return (
            <div className='grid grid-cols-2 gap-px md:grid-cols-3'>
                {Array.from({ length: TILE_COUNT }, (_, index) => (
                    <div key={index} className='bg-card p-3'>
                        <Skeleton className='h-14 w-full' />
                    </div>
                ))}
            </div>
        )

    if (isError) return <LoadErrorState title='서버 통계를 불러오지 못했습니다' />

    if (!data) return null

    return (
        <div className='grid grid-cols-2 gap-px md:grid-cols-3'>
            <StatTile label='등록 차트' value={formatCount(data.charts)} />
            <StatTile label='플레이어' value={formatCount(data.players)} />
            <StatTile label='누적 제출' value={formatCount(data.scores)} />
        </div>
    )
}

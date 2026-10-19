'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { AiLimitsForm } from '@widgets/admin/ai-limits-form'
import { useAiOperatorLimits } from '@entities/ai.query'

export const AiLimitsPanel: FC = () => {
    const { data, isLoading, isError, isFetching, refetch } = useAiOperatorLimits()

    return (
        <PanelCard
            title='AI 사용 한도'
            action={
                <Link href='/settings' className='text-muted-foreground text-xs underline'>
                    설정으로
                </Link>
            }
            contentClassName='flex flex-col gap-3'>
            {isLoading && <TableSkeleton rows={3} label='AI 사용 한도 불러오는 중' />}
            {!isLoading && isError && (
                <LoadErrorState title='AI 사용 한도를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {data && <AiLimitsForm limits={data} />}
            <PanelFootnote>
                저장한 값은 다음 AI 호출부터 모든 사용자에게 적용됩니다. 사용자가 월 토큰 상한을 직접 정했다면 그 값이 기본 월 토큰 상한보다 먼저
                적용됩니다.
            </PanelFootnote>
        </PanelCard>
    )
}

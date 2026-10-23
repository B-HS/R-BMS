'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { PanelCard } from '@features/shell/panel-card'
import { DanSystemCard } from '@widgets/settings/dan-system-card'
import { useMyDans } from '@entities/dan.query'

const SKELETON_ROWS = 4

const ELIGIBILITY_RULES = [
    '게이지가 남은 채 코스를 끝낸 판입니다(FAILED·ASSIST·LIGHT ASSIST 램프는 제외).',
    '모든 스테이지를 코스에 적힌 곡으로 쳐야 합니다.',
    '코스 제약을 지켜야 합니다(미러·랜덤 제한, LN 종류).',
    'SCRATCH AUTO·ASSIST 판과 순위에 오르지 않는 판은 제외합니다.',
    '스테이지 기록을 함께 보내는 클라이언트의 판만 봅니다. 그 전의 기록은 소급하지 않습니다.',
    '게이지 모드(LR2·BEATORAJA·IIDX)는 가리지 않고 기록만 남깁니다.',
]

export const DanPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const { data, isLoading, isError, isFetching, refetch } = useMyDans()

    if (isLoading) return <TableSkeleton rows={SKELETON_ROWS} label='단위 불러오는 중' />
    if (isError || !data) return <LoadErrorState title='단위를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />

    const hasAcquired = data.systems.some((system) => system.acquired.length > 0)

    return (
        <>
            <PanelCard title='단위' contentClassName='flex flex-col gap-2'>
                <p className='text-xs'>
                    코스를 클리어하면 서버가 기록을 보고 단위를 자동으로 줍니다. 다른 플레이어에게는 체계마다 아래에서 고른 단위 하나만 프로필과
                    순위에 보입니다. 획득 목록은 나만 봅니다.
                </p>
                {!hasAcquired && (
                    <p className='text-xs font-medium'>
                        아직 획득한 단위가 없습니다. 코스를 클리어하면 자동으로 들어옵니다. 단위를 주는 코스는{' '}
                        <Link href='/courses' className='underline'>
                            코스 목록
                        </Link>
                        에서 볼 수 있습니다.
                    </p>
                )}
                <ul aria-label='단위 인정 조건' className='text-muted-foreground flex list-disc flex-col gap-0.5 pl-4 text-xs'>
                    {ELIGIBILITY_RULES.map((rule) => (
                        <li key={rule}>{rule}</li>
                    ))}
                </ul>
            </PanelCard>
            {data.systems.map((system) => (
                <DanSystemCard key={system.system} system={system} playerId={playerId} />
            ))}
        </>
    )
}

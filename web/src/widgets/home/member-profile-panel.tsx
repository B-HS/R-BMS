'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DanBadgeList } from '@features/dan/dan-badge-list'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { readDisplayedDans } from '@entities/dan.type'
import { useDashboardOverview } from '@entities/dashboard.query'
import { usePlayer } from '@entities/player.query'
import { formatDateTime } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

const ROW_CLASS_NAME = 'flex items-baseline justify-between gap-3'

export const MemberProfilePanel: FC<{ loginId: string }> = ({ loginId }) => {
    const t = useTranslations('pages.dashboard.profile')
    const profile = usePlayer(loginId)
    const overview = useDashboardOverview()
    const rank = overview.data?.rank
    const lastPlayedAt = overview.data?.tiles.last_played_at

    return (
        <DashboardPanel title={t('title')} bodyClassName='gap-3'>
            <DashboardQueryState
                isPending={profile.isPending || overview.isPending}
                isError={profile.isError || overview.isError}
                isFetching={profile.isFetching || overview.isFetching}
                onRetry={() => void Promise.all([profile.refetch(), overview.refetch()])}
                loadingLabel={t('loading')}
                errorMessage={t('loadError')}>
                {profile.data && (
                    <>
                        <div className='flex min-w-0 flex-col gap-0.5'>
                            <p className='text-lg leading-tight font-semibold tracking-tight wrap-anywhere'>{profile.data.name}</p>
                            <p className='text-muted-foreground font-mono text-xs wrap-anywhere'>@{profile.data.id}</p>
                        </div>
                        <DanBadgeList dans={readDisplayedDans(profile.data.extra)} />
                        <dl className='flex flex-col gap-1 text-xs'>
                            <div className={ROW_CLASS_NAME}>
                                <dt className='text-muted-foreground'>{t('rank')}</dt>
                                <dd className='font-medium tabular-nums'>
                                    {rank && rank.position !== null
                                        ? t('rankValue', { position: rank.position, total: rank.total_players })
                                        : t('rankNone')}
                                </dd>
                            </div>
                            <div className={ROW_CLASS_NAME}>
                                <dt className='text-muted-foreground'>{t('lastPlayed')}</dt>
                                <dd className='font-medium tabular-nums'>{lastPlayedAt ? formatDateTime(lastPlayedAt) : t('neverPlayed')}</dd>
                            </div>
                        </dl>
                        <div className='mt-auto flex flex-wrap gap-1'>
                            <Button asChild size='sm'>
                                <Link href={`/players/${encodeURIComponent(loginId)}`}>{t('myPage')}</Link>
                            </Button>
                            <Button asChild variant='outline' size='sm'>
                                <Link href='/settings'>{t('settings')}</Link>
                            </Button>
                        </div>
                    </>
                )}
            </DashboardQueryState>
        </DashboardPanel>
    )
}

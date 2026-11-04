'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiWeaknessList } from '@features/ai/ai-weakness-list'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { DensityLimitMini } from '@features/dashboard/density-limit-mini'
import { useAiProfile } from '@entities/ai.query'
import { aiHubHref } from '@entities/ai.type'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { useInView } from '@shared/hooks/use-in-view'
import { Button } from '@shared/ui/button'

const WEAKNESSES_SHOWN = 3

/**
 * The weakness panel of the signed-in home (plan 2026-11-04 8.2): the three strongest weaknesses of the profile with their grade
 * and a link to a training chart, and a small density limit curve. It asks for the profile only once it is near the screen, since
 * the server may have to compute it; no AI is called.
 */
export const MemberWeaknessPanel: FC = () => {
    const t = useTranslations('pages.dashboard.weakness')
    const tProfile = useTranslations('pages.ai.profile')
    const { ref, isInView } = useInView()
    const { data, isPending, isError, isFetching, refetch } = useAiProfile(isInView)

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_WIDE_PANEL_CLASS_NAME}
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href={aiHubHref('profile')}>{t('viewProfile')}</Link>
                </Button>
            }>
            <div ref={ref} className='@container flex min-h-0 flex-1 flex-col'>
                <DashboardQueryState
                    isPending={isPending}
                    isError={isError}
                    isFetching={isFetching}
                    onRetry={() => void refetch()}
                    loadingLabel={t('loading')}
                    errorMessage={t('loadError')}
                    isEmpty={data !== undefined && data.window.plays === 0}
                    emptyMessage={t('empty')}
                    loadingClassName='min-h-28'>
                    {data && (
                        <div className='grid min-w-0 gap-4 @xl:grid-cols-2'>
                            <div className='flex min-w-0 flex-col gap-2'>
                                {data.weaknesses.length === 0 ? (
                                    <p className='text-muted-foreground text-xs break-keep'>{t('noWeakness')}</p>
                                ) : (
                                    <AiWeaknessList
                                        label={t('listLabel')}
                                        items={data.weaknesses.slice(0, WEAKNESSES_SHOWN)}
                                        trainingHrefOf={(key) => aiHubHref('training', key)}
                                    />
                                )}
                                {data.notice && <AiDataNotice need={data.notice.need} />}
                            </div>
                            <div className='flex min-w-0 flex-col gap-1'>
                                <h3 className='rbms-micro-label'>{t('densityTitle')}</h3>
                                {data.density.curve.length === 0 ? (
                                    <p className='text-muted-foreground text-xs break-keep'>{tProfile('densityInsufficient')}</p>
                                ) : (
                                    <>
                                        <DensityLimitMini density={data.density} />
                                        <p className='text-muted-foreground text-xs tabular-nums'>
                                            {t('densityCaption', {
                                                comfort:
                                                    data.density.comfort_nps === null
                                                        ? tProfile('none')
                                                        : tProfile('npsValue', { nps: data.density.comfort_nps }),
                                                limit:
                                                    data.density.limit_nps_ex === null
                                                        ? tProfile('none')
                                                        : tProfile('npsValue', { nps: data.density.limit_nps_ex }),
                                            })}
                                        </p>
                                    </>
                                )}
                            </div>
                        </div>
                    )}
                </DashboardQueryState>
            </div>
        </DashboardPanel>
    )
}

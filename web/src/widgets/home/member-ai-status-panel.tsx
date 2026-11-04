'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useQuery } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiUsageMeter } from '@features/ai/ai-usage-meter'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { aiGaugeAdviceQueryOptions, aiJudgeHabitQueryOptions, useAiCredentialList, useAiUsageSummary } from '@entities/ai.query'
import { AI_GAUGE_LABEL, AI_PROVIDER_LABEL, type AiGaugeAdviceView, type AiGaugeName } from '@entities/ai.type'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { AI_SETTINGS_HREF } from '@shared/constants/navigation'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useInView } from '@shared/hooks/use-in-view'
import { Button } from '@shared/ui/button'

const MISSING = '—'
const OFFSET_DECIMALS = 1

const formatSignedMs = (ms: number) => `${ms > 0 ? '+' : ''}${ms.toFixed(OFFSET_DECIMALS)} ms`

const topRescueGaugeOf = (rescues: AiGaugeAdviceView['rescues']) => {
    const totals = new Map<AiGaugeName, number>()
    for (const rescue of rescues)
        for (const rescuedBy of rescue.rescued_by) totals.set(rescuedBy.gauge, (totals.get(rescuedBy.gauge) ?? 0) + rescuedBy.plays)
    return [...totals].toSorted((left, right) => right[1] - left[1])[0] ?? null
}

export const MemberAiStatusPanel: FC = () => {
    const t = useTranslations('pages.dashboard.aiStatus')
    const { ref, isInView } = useInView()
    const credentials = useAiCredentialList()
    const usage = useAiUsageSummary()
    const habit = useQuery({ ...aiJudgeHabitQueryOptions(), enabled: isInView })
    const gauge = useQuery({ ...aiGaugeAdviceQueryOptions(), enabled: isInView })
    const habitView = habit.data
    const gaugeView = gauge.data
    const providers = credentials.data?.map((credential) => AI_PROVIDER_LABEL[credential.provider]).join(', ') ?? ''
    const topRescue = gaugeView ? topRescueGaugeOf(gaugeView.rescues) : null
    const habitLine =
        habitView && !habitView.notice
            ? t('habitLine', {
                  fastShare: habitView.fast_slow.fast_share === null ? MISSING : `${Math.round(habitView.fast_slow.fast_share * PERCENT_SCALE)}%`,
                  meanOffset: habitView.offset.mean_offset_ms === null ? MISSING : formatSignedMs(habitView.offset.mean_offset_ms),
                  suggestion:
                      habitView.offset.suggested_shift_ms === null
                          ? t(`offsetDirection.${habitView.offset.direction}`)
                          : t('offsetShift', {
                                direction: t(`offsetDirection.${habitView.offset.direction}`),
                                shift: formatSignedMs(habitView.offset.suggested_shift_ms),
                            }),
              })
            : null

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_WIDE_PANEL_CLASS_NAME}
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href={AI_SETTINGS_HREF}>{t('settings')}</Link>
                </Button>
            }>
            <div ref={ref} className='@container flex min-h-0 flex-1 flex-col'>
                <div className='grid min-w-0 gap-4 @xl:grid-cols-2'>
                    <div className='flex min-w-0 flex-col gap-3'>
                        <DashboardQueryState
                            isPending={credentials.isPending}
                            isError={credentials.isError}
                            isFetching={credentials.isFetching}
                            onRetry={() => void credentials.refetch()}
                            loadingLabel={t('keyLoading')}
                            errorMessage={t('keyLoadError')}
                            loadingClassName='min-h-8'>
                            {providers ? (
                                <p className='text-xs'>{t('keyRegistered', { providers })}</p>
                            ) : (
                                <p className='text-xs'>
                                    {t('keyMissing')}{' '}
                                    <Link href={AI_SETTINGS_HREF} className='underline'>
                                        {t('keyRegister')}
                                    </Link>
                                </p>
                            )}
                        </DashboardQueryState>
                        <DashboardQueryState
                            isPending={usage.isPending}
                            isError={usage.isError}
                            isFetching={usage.isFetching}
                            onRetry={() => void usage.refetch()}
                            loadingLabel={t('usageLoading')}
                            errorMessage={t('usageLoadError')}
                            loadingClassName='min-h-16'>
                            {usage.data && (
                                <div className='flex flex-col gap-2'>
                                    <AiUsageMeter
                                        label={t('todayCalls')}
                                        used={usage.data.today.calls}
                                        limit={usage.data.limits.daily_calls}
                                        unit={t('callsUnit')}
                                    />
                                    <AiUsageMeter
                                        label={t('monthTokens')}
                                        used={usage.data.this_month.input_tokens + usage.data.this_month.output_tokens}
                                        limit={usage.data.limits.monthly_token_limit}
                                        unit={t('tokensUnit')}
                                    />
                                </div>
                            )}
                        </DashboardQueryState>
                    </div>
                    <div className='flex min-w-0 flex-col gap-3'>
                        <section className='flex flex-col gap-1'>
                            <h3 className='rbms-micro-label'>{t('habitTitle')}</h3>
                            <DashboardQueryState
                                isPending={habit.isPending}
                                isError={habit.isError}
                                isFetching={habit.isFetching}
                                onRetry={() => void habit.refetch()}
                                loadingLabel={t('habitLoading')}
                                errorMessage={t('habitLoadError')}
                                loadingClassName='min-h-10'>
                                {habitView?.notice && <AiDataNotice need={habitView.notice.need} />}
                                {habitLine && <p className='text-xs tabular-nums'>{habitLine}</p>}
                            </DashboardQueryState>
                        </section>
                        <section className='flex flex-col gap-1'>
                            <h3 className='rbms-micro-label'>{t('gaugeTitle')}</h3>
                            <DashboardQueryState
                                isPending={gauge.isPending}
                                isError={gauge.isError}
                                isFetching={gauge.isFetching}
                                onRetry={() => void gauge.refetch()}
                                loadingLabel={t('gaugeLoading')}
                                errorMessage={t('gaugeLoadError')}
                                loadingClassName='min-h-10'>
                                {gaugeView?.notice && <AiDataNotice need={gaugeView.notice.need} />}
                                {gaugeView && !gaugeView.notice && (
                                    <p className='text-xs tabular-nums'>
                                        {topRescue ? t('gaugeLine', { gauge: AI_GAUGE_LABEL[topRescue[0]], plays: topRescue[1] }) : t('gaugeNone')}
                                    </p>
                                )}
                            </DashboardQueryState>
                        </section>
                    </div>
                </div>
            </div>
        </DashboardPanel>
    )
}

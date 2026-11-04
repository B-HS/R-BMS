'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { ActivityCalendar } from '@features/dashboard/activity-calendar'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { PlayDayChart } from '@features/stats/play-day-chart'
import { useDashboardCalendar } from '@entities/dashboard.query'
import { DASHBOARD_CALENDAR_DAYS } from '@entities/dashboard.type'
import { usePlayerPlayDays } from '@entities/player.query'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { formatCount, formatHoursMinutes } from '@shared/lib/format'

export const MemberCalendarPanel: FC<{ loginId: string }> = ({ loginId }) => {
    const t = useTranslations('pages.dashboard.calendar')
    const calendar = useDashboardCalendar(DASHBOARD_CALENDAR_DAYS)
    const playDays = usePlayerPlayDays(loginId)
    const calendarData = calendar.data
    const playDaysData = playDays.data
    const calendarFigures = calendarData
        ? [
              { key: 'streak', label: t('streak'), value: t('days', { days: calendarData.streak_days }) },
              { key: 'bestStreak', label: t('bestStreak'), value: t('days', { days: calendarData.best_streak_days }) },
              { key: 'activeDays', label: t('activeDays'), value: t('days', { days: calendarData.active_days }) },
              { key: 'totalPlays', label: t('totalPlays'), value: formatCount(calendarData.total_plays) },
          ]
        : []

    return (
        <DashboardPanel title={t('title')} className={DASHBOARD_WIDE_PANEL_CLASS_NAME} bodyClassName='@container'>
            <div className='grid min-w-0 gap-4 @2xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]'>
                <section className='flex min-w-0 flex-col gap-2'>
                    <h3 className='rbms-micro-label'>{t('calendarTitle')}</h3>
                    <DashboardQueryState
                        isPending={calendar.isPending}
                        isError={calendar.isError}
                        isFetching={calendar.isFetching}
                        onRetry={() => void calendar.refetch()}
                        loadingLabel={t('calendarLoading')}
                        errorMessage={t('calendarLoadError')}
                        isEmpty={calendarData?.total_plays === 0}
                        emptyMessage={t('calendarEmpty')}
                        loadingClassName='min-h-32'>
                        {calendarData && (
                            <>
                                <ActivityCalendar
                                    days={calendarData.days}
                                    label={t('calendarLabel', {
                                        days: calendarData.days.length,
                                        activeDays: calendarData.active_days,
                                        plays: calendarData.total_plays,
                                    })}
                                    cellLabel={(day) => t('cellLabel', { date: day.date, plays: day.plays })}
                                    legend={{ less: t('less'), more: t('more') }}
                                />
                                <dl className='grid grid-cols-2 gap-x-4 gap-y-1 text-xs'>
                                    {calendarFigures.map((figure) => (
                                        <div key={figure.key} className='flex items-baseline justify-between gap-2'>
                                            <dt className='text-muted-foreground'>{figure.label}</dt>
                                            <dd className='font-medium tabular-nums'>{figure.value}</dd>
                                        </div>
                                    ))}
                                </dl>
                            </>
                        )}
                    </DashboardQueryState>
                </section>
                <section className='flex min-w-0 flex-col gap-2'>
                    <h3 className='rbms-micro-label'>{t('playTimeTitle')}</h3>
                    <DashboardQueryState
                        isPending={playDays.isPending}
                        isError={playDays.isError}
                        isFetching={playDays.isFetching}
                        onRetry={() => void playDays.refetch()}
                        loadingLabel={t('playTimeLoading')}
                        errorMessage={t('playTimeLoadError')}
                        isEmpty={playDaysData?.total_plays === 0}
                        emptyMessage={t('playTimeEmpty')}
                        loadingClassName='min-h-28'>
                        {playDaysData && (
                            <>
                                <PlayDayChart
                                    days={playDaysData.days.map((day) => ({
                                        date: day.date,
                                        playMs: day.play_ms,
                                        label: t('barLabel', { date: day.date, time: formatHoursMinutes(day.play_ms) }),
                                    }))}
                                    listLabel={t('playTimeLabel', { days: playDaysData.days.length })}
                                />
                                <p className='text-xs font-medium tabular-nums'>
                                    {t('playTimeTotal', { days: playDaysData.days.length, time: formatHoursMinutes(playDaysData.total_ms) })}
                                </p>
                                <p className='text-muted-foreground text-2xs'>{t('playTimeNote')}</p>
                            </>
                        )}
                    </DashboardQueryState>
                </section>
            </div>
        </DashboardPanel>
    )
}

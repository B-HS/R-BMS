'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DanBadgeList } from '@features/dan/dan-badge-list'
import { CourseProgressList } from '@features/dashboard/course-progress-list'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { useMyDans } from '@entities/dan.query'
import { useDashboardCourses } from '@entities/dashboard.query'
import { lampLabel } from '@shared/constants/clear-lamp'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

export const MemberDanCoursePanel: FC = () => {
    const t = useTranslations('pages.dashboard.danCourse')
    const dans = useMyDans()
    const courses = useDashboardCourses()
    const hasCourseRecords = courses.data ? courses.data.groups.some((group) => group.played > 0) || courses.data.recent.length > 0 : false

    return (
        <DashboardPanel
            title={t('title')}
            bodyClassName='gap-4'
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href='/courses'>{t('viewCourses')}</Link>
                </Button>
            }>
            <section className='flex flex-col gap-1.5'>
                <h3 className='rbms-micro-label'>{t('danTitle')}</h3>
                <DashboardQueryState
                    isPending={dans.isPending}
                    isError={dans.isError}
                    isFetching={dans.isFetching}
                    onRetry={() => void dans.refetch()}
                    loadingLabel={t('danLoading')}
                    errorMessage={t('danLoadError')}
                    loadingClassName='min-h-16'>
                    <ul aria-label={t('danLabel')} className='flex flex-col gap-1.5'>
                        {dans.data?.systems.map((system) => (
                            <li key={system.system} className='flex items-center justify-between gap-2 text-xs'>
                                {system.shown.length > 0 ? (
                                    <DanBadgeList dans={system.shown} />
                                ) : (
                                    <span className='text-muted-foreground truncate'>
                                        {system.name} · {system.acquired.length > 0 ? t('danHidden') : t('danNone')}
                                    </span>
                                )}
                                <span className='text-muted-foreground shrink-0 tabular-nums'>
                                    {t('danAcquired', { count: system.acquired.length })}
                                </span>
                            </li>
                        ))}
                    </ul>
                </DashboardQueryState>
            </section>
            <section className='flex flex-col gap-1.5'>
                <h3 className='rbms-micro-label'>{t('courseTitle')}</h3>
                <DashboardQueryState
                    isPending={courses.isPending}
                    isError={courses.isError}
                    isFetching={courses.isFetching}
                    onRetry={() => void courses.refetch()}
                    loadingLabel={t('courseLoading')}
                    errorMessage={t('courseLoadError')}
                    isEmpty={!hasCourseRecords}
                    emptyMessage={t('courseEmpty')}
                    loadingClassName='min-h-24'>
                    {courses.data && (
                        <>
                            <CourseProgressList
                                groups={courses.data.groups}
                                label={t('groupsLabel')}
                                describeGroup={(group) =>
                                    t('groupDescription', { group: group.group, cleared: group.cleared, played: group.played, total: group.total })
                                }
                            />
                            {courses.data.recent.length > 0 && (
                                <ul aria-label={t('recentLabel')} className='border-border flex flex-col border-t'>
                                    {courses.data.recent.map((record) => (
                                        <li
                                            key={`${record.course_hash}:${record.played_at}`}
                                            className='border-border flex items-center justify-between gap-2 border-b py-1.5'>
                                            <div className='flex min-w-0 flex-col'>
                                                <Link
                                                    href={`/courses/${encodeURIComponent(record.course_hash)}`}
                                                    className='truncate text-xs hover:underline'>
                                                    {record.name}
                                                </Link>
                                                <span className='text-muted-foreground text-2xs tabular-nums'>
                                                    {formatDateTime(record.played_at)}
                                                </span>
                                            </div>
                                            <span className='shrink-0 text-xs'>
                                                {lampLabel(record.clear)}{' '}
                                                <span className='font-medium tabular-nums'>{formatCount(record.ex_score)}</span>
                                            </span>
                                        </li>
                                    ))}
                                </ul>
                            )}
                        </>
                    )}
                </DashboardQueryState>
            </section>
        </DashboardPanel>
    )
}

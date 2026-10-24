'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { MetaBadge } from '@features/badge/meta-badge'
import { CourseSourceNotice } from '@features/course/course-source-notice'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { formatCount } from '@shared/lib/format'
import { DAN_SETTINGS_HREF } from '@shared/constants/navigation'
import { useCourseCatalog } from '@entities/course.query'
import type { CourseCatalogCourse } from '@entities/course.type'

const SKELETON_ROWS = 8

type CourseTranslator = ReturnType<typeof useTranslations<'browse.course'>>

const buildColumns = (t: CourseTranslator): DataTableColumn<CourseCatalogCourse>[] => [
    {
        key: 'name',
        label: t('catalog.course'),
        flex: true,
        cell: (row) => (
            <span className='flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5'>
                <Link href={`/courses/${encodeURIComponent(row.course_hash)}`} className='max-w-full truncate font-medium hover:underline'>
                    {row.name}
                </Link>
                {!row.active && <MetaBadge>{t('removedFromTable')}</MetaBadge>}
            </span>
        ),
    },
    { key: 'stage_count', label: 'STAGE', width: 80, align: 'right', cell: (row) => formatCount(row.stage_count) },
    { key: 'dan', label: t('catalog.dan'), width: 112, cell: (row) => (row.dan ? <MetaBadge>{row.dan.label}</MetaBadge> : '—') },
]

export const CourseCatalogPanel: FC = () => {
    const t = useTranslations('browse.course')
    const { data, isLoading, isError, isFetching, refetch } = useCourseCatalog()

    if (isLoading) return <TableSkeleton rows={SKELETON_ROWS} label={t('catalog.loading')} />
    if (isError) return <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />

    const groups = data?.groups ?? []
    if (groups.length === 0) return <EmptyState title={t('catalog.emptyTitle')} description={t('catalog.emptyDescription')} />

    const columns = buildColumns(t)

    return (
        <>
            {groups.map((group) => (
                <PanelCard
                    key={group.id}
                    title={t('catalog.groupTitle', { name: group.name, count: group.courses.length })}
                    action={group.keys > 0 ? <MetaBadge>{t('catalog.keys', { keys: group.keys })}</MetaBadge> : undefined}
                    contentClassName='flex flex-col gap-2'>
                    <CourseSourceNotice source={group.source} keys={group.keys} />
                    <DataTable
                        columns={columns}
                        rows={group.courses}
                        rowKey={(row) => row.course_hash}
                        rowMuted={(row) => !row.active}
                        caption={group.name}
                    />
                </PanelCard>
            ))}
            <div className='bg-card px-3 py-2'>
                <PanelFootnote>
                    {t.rich('catalog.footnote', {
                        link: (chunks) => (
                            <Link href={DAN_SETTINGS_HREF} className='underline'>
                                {chunks}
                            </Link>
                        ),
                    })}
                </PanelFootnote>
            </div>
        </>
    )
}

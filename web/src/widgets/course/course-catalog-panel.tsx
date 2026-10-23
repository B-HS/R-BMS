'use client'

import type { FC } from 'react'
import Link from 'next/link'
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

const COLUMNS: DataTableColumn<CourseCatalogCourse>[] = [
    {
        key: 'name',
        label: '코스',
        flex: true,
        cell: (row) => (
            <span className='flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5'>
                <Link href={`/courses/${encodeURIComponent(row.course_hash)}`} className='max-w-full truncate font-medium hover:underline'>
                    {row.name}
                </Link>
                {!row.active && <MetaBadge>표에서 빠짐</MetaBadge>}
            </span>
        ),
    },
    { key: 'stage_count', label: 'STAGE', width: 80, align: 'right', cell: (row) => formatCount(row.stage_count) },
    { key: 'dan', label: '단위', width: 112, cell: (row) => (row.dan ? <MetaBadge>{row.dan.label}</MetaBadge> : '—') },
]

export const CourseCatalogPanel: FC = () => {
    const { data, isLoading, isError, isFetching, refetch } = useCourseCatalog()

    if (isLoading) return <TableSkeleton rows={SKELETON_ROWS} label='코스 불러오는 중' />
    if (isError) return <LoadErrorState title='코스를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />

    const groups = data?.groups ?? []
    if (groups.length === 0)
        return (
            <EmptyState
                title='등록된 코스가 아직 없습니다'
                description='GENOSIDE 2018 과 Stella·Satellite 코스는 서버에 등록되면 여기에 나타납니다.'
            />
        )

    return (
        <>
            {groups.map((group) => (
                <PanelCard
                    key={group.id}
                    title={`${group.name} (${formatCount(group.courses.length)})`}
                    action={group.keys > 0 ? <MetaBadge>{group.keys}키</MetaBadge> : undefined}
                    contentClassName='flex flex-col gap-2'>
                    <CourseSourceNotice source={group.source} keys={group.keys} />
                    <DataTable
                        columns={COLUMNS}
                        rows={group.courses}
                        rowKey={(row) => row.course_hash}
                        rowMuted={(row) => !row.active}
                        caption={group.name}
                    />
                </PanelCard>
            ))}
            <div className='bg-card px-3 py-2'>
                <PanelFootnote>
                    단위 칸에 표시가 있는 코스를 조건대로 클리어하면 서버가 단위를 자동으로 줍니다. 조건과 표시할 단위는{' '}
                    <Link href={DAN_SETTINGS_HREF} className='underline'>
                        마이페이지 단위 탭
                    </Link>
                    에서 봅니다.
                </PanelFootnote>
            </div>
        </>
    )
}

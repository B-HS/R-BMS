'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { FolderReadRow } from '@features/dashboard/folder-read-row'
import { ReplayRow } from '@features/dashboard/replay-row'
import { useDashboardReplays } from '@entities/dashboard.query'
import { DASHBOARD_REPLAYS_LIMIT } from '@entities/dashboard.type'
import { useGetFolderList } from '@entities/folder.query'
import { DASHBOARD_FILL_PANEL_CLASS_NAME, DASHBOARD_FOLDERS_SHOWN } from '@shared/constants/dashboard'
import { formatChartTitle } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

export const MemberFolderReplayPanel: FC = () => {
    const t = useTranslations('pages.dashboard.folderReplay')
    const folders = useGetFolderList()
    const replays = useDashboardReplays(DASHBOARD_REPLAYS_LIMIT)
    const recentFolders = folders.data?.toSorted((left, right) => right.updated_at - left.updated_at).slice(0, DASHBOARD_FOLDERS_SHOWN) ?? []

    return (
        <DashboardPanel title={t('title')} className={DASHBOARD_FILL_PANEL_CLASS_NAME} bodyClassName='px-0 pb-0'>
            <section className='flex flex-col'>
                <div className='flex h-7 items-center justify-between gap-2 px-3'>
                    <h3 className='rbms-micro-label'>{t('folderTitle')}</h3>
                    <Button asChild variant='ghost' size='xs'>
                        <Link href='/folders'>{t('viewFolders')}</Link>
                    </Button>
                </div>
                <DashboardQueryState
                    isPending={folders.isPending}
                    isError={folders.isError}
                    isFetching={folders.isFetching}
                    onRetry={() => void folders.refetch()}
                    loadingLabel={t('folderLoading')}
                    errorMessage={t('folderLoadError')}
                    isEmpty={recentFolders.length === 0}
                    emptyMessage={t('folderEmpty')}
                    loadingClassName='mx-3 mb-3 min-h-20'>
                    <ul aria-label={t('folderLabel')} className='border-border flex flex-col border-t'>
                        {recentFolders.map((folder) => (
                            <FolderReadRow
                                key={folder.id}
                                folderId={folder.id}
                                name={folder.name}
                                updatedAt={folder.updated_at}
                                itemCountLabel={t('itemCount', { count: folder.item_count })}
                            />
                        ))}
                    </ul>
                </DashboardQueryState>
            </section>
            <section className='flex flex-col'>
                <h3 className='rbms-micro-label flex h-7 items-center px-3'>{t('replayTitle')}</h3>
                <DashboardQueryState
                    isPending={replays.isPending}
                    isError={replays.isError}
                    isFetching={replays.isFetching}
                    onRetry={() => void replays.refetch()}
                    loadingLabel={t('replayLoading')}
                    errorMessage={t('replayLoadError')}
                    isEmpty={replays.data?.replays.length === 0}
                    emptyMessage={t('replayEmpty')}
                    loadingClassName='mx-3 mb-3 min-h-20'>
                    <ul aria-label={t('replayLabel')} className='border-border flex flex-col border-t'>
                        {replays.data?.replays.map((replay) => (
                            <ReplayRow
                                key={replay.id}
                                replayId={replay.id}
                                title={formatChartTitle(replay.chart)}
                                createdAt={replay.created_at}
                                lamp={replay.clear}
                                exScore={replay.ex_score}
                            />
                        ))}
                    </ul>
                </DashboardQueryState>
            </section>
        </DashboardPanel>
    )
}

'use client'

import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { FolderEditorWorkspace } from '@widgets/folder/folder-editor-workspace'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { folderDetailQueryOptions } from '@entities/folder.query'
import type { FolderTableIndex } from '@entities/folder.type'
import { ApiError } from '@shared/lib/fetch'

const NOT_FOUND_STATUS = 404

type FolderEditorProps = {
    folderId: string
    tables: FolderTableIndex[]
}

/**
 * The server renders the first copy of the folder without the download links of its training charts (it has no request origin there),
 * so the page asks the folder route once more when it mounts; that answer carries the links.
 */
export const FolderEditor: FC<FolderEditorProps> = ({ folderId, tables }) => {
    const t = useTranslations('browse.folder')
    const { data, isLoading, error, isFetching, refetch } = useQuery({ ...folderDetailQueryOptions(folderId), refetchOnMount: 'always' })

    const reload = async () => {
        const result = await refetch()
        return result.status === 'success' ? result.data : undefined
    }

    if (data) return <FolderEditorWorkspace key={folderId} folder={data} tables={tables} onReload={reload} />

    if (isLoading) return <ShellPageLoading title={t('pageTitle')} />

    if (error instanceof ApiError && error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={t('pageTitle')}>
                <NotFoundNotice
                    title={t('notFoundTitle')}
                    description={t('notFoundDescription')}
                    actionHref='/folders'
                    actionLabel={t('notFoundAction')}
                />
            </ShellPage>
        )

    return (
        <ShellPage title={t('pageTitle')}>
            <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
        </ShellPage>
    )
}

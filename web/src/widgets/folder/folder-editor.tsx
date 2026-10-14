'use client'

import type { FC } from 'react'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { FolderEditorWorkspace } from '@widgets/folder/folder-editor-workspace'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { useGetFolderDetail } from '@entities/folder.query'
import type { FolderTableIndex } from '@entities/folder.type'
import { ApiError } from '@shared/lib/fetch'

const PAGE_TITLE = '폴더'
const NOT_FOUND_STATUS = 404

type FolderEditorProps = {
    folderId: string
    tables: FolderTableIndex[]
}

export const FolderEditor: FC<FolderEditorProps> = ({ folderId, tables }) => {
    const { data, isLoading, error, isFetching, refetch } = useGetFolderDetail(folderId)

    const reload = async () => {
        const result = await refetch()
        return result.status === 'success' ? result.data : undefined
    }

    if (data) return <FolderEditorWorkspace key={folderId} folder={data} tables={tables} onReload={reload} />

    if (isLoading) return <ShellPageLoading title={PAGE_TITLE} />

    if (error instanceof ApiError && error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={PAGE_TITLE}>
                <NotFoundNotice
                    title='폴더를 찾을 수 없습니다'
                    description='지워졌거나 다른 계정의 폴더입니다.'
                    actionHref='/folders'
                    actionLabel='내 폴더로'
                />
            </ShellPage>
        )

    return (
        <ShellPage title={PAGE_TITLE}>
            <LoadErrorState title='폴더를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />
        </ShellPage>
    )
}

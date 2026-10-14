import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchNoContent } from '@shared/lib/fetch'
import type { FolderDetail, FolderItemChange, FolderItemInput, FolderMetaInput, FolderSummary } from '@entities/folder.type'

const FOLDERS_PATH = '/fe/me/folders'

const folderPath = (folderId: string) => `${FOLDERS_PATH}/${encodeURIComponent(folderId)}`

const jsonInit = (method: string, body: unknown) => ({ method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })

export const folderListQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.FOLDER.LIST,
        queryFn: () => fetchEnvelope<FolderSummary[]>(buildApiUrl(FOLDERS_PATH)),
    })

export const folderDetailQueryOptions = (folderId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.FOLDER.DETAIL(folderId),
        queryFn: () => fetchEnvelope<FolderDetail>(buildApiUrl(folderPath(folderId))),
    })

export const folderContainingQueryOptions = (md5: string) =>
    queryOptions({
        queryKey: QUERY_KEY.FOLDER.CONTAINING(md5),
        queryFn: () => fetchEnvelope<string[]>(buildApiUrl(`${FOLDERS_PATH}/containing/${encodeURIComponent(md5)}`)),
    })

export const useGetFolderList = (enabled = true) => useQuery({ ...folderListQueryOptions(), enabled })

export const useGetFolderDetail = (folderId: string) => useQuery(folderDetailQueryOptions(folderId))

export const useGetFolderContaining = (md5: string, enabled = true) => useQuery({ ...folderContainingQueryOptions(md5), enabled })

export const useCreateFolder = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (input: FolderMetaInput) => fetchEnvelope<FolderSummary>(buildApiUrl(FOLDERS_PATH), jsonInit('POST', input)),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.LIST }),
        meta: { suppressErrorToast: true },
    })
}

export const useUpdateFolder = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (params: { folderId: string; input: Partial<FolderMetaInput> }) =>
            fetchEnvelope<FolderSummary>(buildApiUrl(folderPath(params.folderId)), jsonInit('PATCH', params.input)),
        onSuccess: (_, params) =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.LIST }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.DETAIL(params.folderId) }),
            ]),
        meta: { suppressErrorToast: true },
    })
}

export const useDeleteFolder = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (folderId: string) => fetchNoContent(buildApiUrl(folderPath(folderId)), { method: 'DELETE' }),
        onSuccess: (_, folderId) => {
            queryClient.removeQueries({ queryKey: QUERY_KEY.FOLDER.DETAIL(folderId) })
            return Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.LIST }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.CONTAINING_ALL }),
            ])
        },
    })
}

export const useReorderFolders = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (folderIds: string[]) =>
            fetchEnvelope<FolderSummary[]>(buildApiUrl(`${FOLDERS_PATH}/order`), jsonInit('PUT', { folder_ids: folderIds })),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.LIST }),
    })
}

/** Replaces the whole ordered item list, and the name and description when given, in one request; the server answers 409 when `baseUpdatedAt` is stale. */
export const useReplaceFolderItems = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (params: { folderId: string; items: FolderItemInput[]; baseUpdatedAt: number; meta?: Partial<FolderMetaInput> }) =>
            fetchEnvelope<FolderItemChange>(
                buildApiUrl(`${folderPath(params.folderId)}/items`),
                jsonInit('PUT', { items: params.items, base_updated_at: params.baseUpdatedAt, ...params.meta }),
            ),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.ALL }),
        meta: { suppressErrorToast: true },
    })
}

export const useAddFolderItems = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (params: { folderId: string; items: FolderItemInput[] }) =>
            fetchEnvelope<FolderItemChange>(buildApiUrl(`${folderPath(params.folderId)}/items`), jsonInit('POST', { items: params.items })),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.ALL }),
        meta: { suppressErrorToast: true },
    })
}

export const useRemoveFolderItems = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (params: { folderId: string; md5s: string[] }) =>
            fetchEnvelope<FolderItemChange>(buildApiUrl(`${folderPath(params.folderId)}/items/remove`), jsonInit('POST', { md5s: params.md5s })),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.ALL }),
        meta: { suppressErrorToast: true },
    })
}

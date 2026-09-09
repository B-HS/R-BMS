import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchRaw } from '@shared/lib/fetch'
import type { TableData } from '@entities/table.type'

export const tableListQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.TABLE.LIST,
        queryFn: () => fetchRaw<TableData[]>(buildApiUrl('/tables')),
    })

export const tableDetailQueryOptions = (tableId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.TABLE.DETAIL(tableId),
        queryFn: () => fetchRaw<TableData>(buildApiUrl(`/tables/${encodeURIComponent(tableId)}`)),
    })

export const useTableList = () => useQuery(tableListQueryOptions())

export const useTableDetail = (tableId: string) => useQuery(tableDetailQueryOptions(tableId))

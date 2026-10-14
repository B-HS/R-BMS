import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope } from '@shared/lib/fetch'
import type { TableSnapshotRecords } from '@entities/table-snapshot.type'

export const tableSnapshotRecordsQueryOptions = (tableId: string, userId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.TABLE_SNAPSHOT.RECORDS(tableId, userId),
        queryFn: async () => (await fetchEnvelope<TableSnapshotRecords>(buildApiUrl(`/fe/tables/${encodeURIComponent(tableId)}/records`))).records,
    })

/** Reads the signed-in player's best record per md5 for one bundled table; stays idle while nobody is signed in. */
export const useGetTableSnapshotRecords = (tableId: string, userId: string | null) =>
    useQuery({ ...tableSnapshotRecordsQueryOptions(tableId, userId ?? ''), enabled: userId !== null })

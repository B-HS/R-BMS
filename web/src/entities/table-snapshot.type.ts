import type { TableDetailOutput, TableEntryOutput, TableRecordOutput, TableRecordsOutput, TableSummaryOutput } from '@server/dto/table-snapshot'

export type TableSnapshotSummary = TableSummaryOutput
export type TableSnapshotDetail = TableDetailOutput
export type TableSnapshotLevel = TableDetailOutput['levels'][number]
export type TableSnapshotEntry = TableEntryOutput
export type TableSnapshotRecord = TableRecordOutput
export type TableSnapshotRecords = TableRecordsOutput

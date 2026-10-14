import type { FC } from 'react'
import { TableChartCard } from '@features/table/table-chart-card'
import type { TableSnapshotEntry, TableSnapshotRecord } from '@entities/table-snapshot.type'

const GRID_CLASS_NAME = 'bg-border grid min-w-0 grid-cols-2 gap-px sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6'

const EMPTY_CELL_LAYOUTS = [
    { columns: 2, className: 'block sm:hidden' },
    { columns: 3, className: 'hidden sm:block lg:hidden' },
    { columns: 4, className: 'hidden lg:block xl:hidden' },
    { columns: 5, className: 'hidden xl:block 2xl:hidden' },
    { columns: 6, className: 'hidden 2xl:block' },
] as const

type TableChartGridProps = {
    label: string
    symbol: string
    entries: TableSnapshotEntry[]
    records: Record<string, TableSnapshotRecord> | null
    isRecordPending: boolean
    onOpen: (entry: TableSnapshotEntry, trigger: HTMLButtonElement) => void
}

export const TableChartGrid: FC<TableChartGridProps> = ({ label, symbol, entries, records, isRecordPending, onOpen }) => (
    <ul aria-label={label} aria-busy={isRecordPending} className={GRID_CLASS_NAME}>
        {entries.map((entry) => (
            <li key={entry.md5} className='min-w-0'>
                <TableChartCard entry={entry} symbol={symbol} record={records?.[entry.md5]} isRecordPending={isRecordPending} onOpen={onOpen} />
            </li>
        ))}
        {EMPTY_CELL_LAYOUTS.flatMap((layout) =>
            Array.from({ length: (layout.columns - (entries.length % layout.columns)) % layout.columns }, (_, index) => (
                <li key={`${layout.columns}-${index}`} aria-hidden='true' className={`bg-card min-h-14 ${layout.className}`} />
            )),
        )}
    </ul>
)

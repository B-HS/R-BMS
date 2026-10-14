import type { ReactNode } from 'react'
import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from '@shared/ui/table'
import { cn } from '@shared/lib/cn'

const HIDE_BELOW_CLASS = { sm: 'hidden sm:table-cell', md: 'hidden md:table-cell', lg: 'hidden lg:table-cell' }

export type DataTableColumn<TRow> = {
    key: string
    label: string
    width?: number
    align?: 'left' | 'right'
    flex?: boolean
    mono?: boolean
    hideBelow?: keyof typeof HIDE_BELOW_CLASS
    cell: (row: TRow, index: number) => ReactNode
}

type DataTableProps<TRow> = {
    columns: DataTableColumn<TRow>[]
    rows: TRow[]
    rowKey: (row: TRow) => string
    rowMuted?: (row: TRow) => boolean
    caption?: string
}

export const DataTable = <TRow,>({ columns, rows, rowKey, rowMuted, caption }: DataTableProps<TRow>) => (
    <Table className='text-xs'>
        {caption && <TableCaption className='sr-only'>{caption}</TableCaption>}
        <TableHeader>
            <TableRow className='hover:bg-transparent'>
                {columns.map((column) => (
                    <TableHead
                        key={column.key}
                        style={column.width ? { width: `${column.width}px` } : undefined}
                        className={cn(
                            'text-muted-foreground h-auto p-2 font-medium',
                            column.align === 'right' && 'text-right tabular-nums',
                            column.hideBelow && HIDE_BELOW_CLASS[column.hideBelow],
                        )}>
                        {column.label}
                    </TableHead>
                ))}
            </TableRow>
        </TableHeader>
        <TableBody>
            {rows.map((row, index) => (
                <TableRow key={rowKey(row)} className={cn('border-border', rowMuted?.(row) && 'text-muted-foreground')}>
                    {columns.map((column) => (
                        <TableCell
                            key={column.key}
                            className={cn(
                                column.align === 'right' && 'text-right tabular-nums',
                                column.mono && 'font-mono',
                                column.flex && 'max-w-0 overflow-hidden text-ellipsis',
                                column.hideBelow && HIDE_BELOW_CLASS[column.hideBelow],
                            )}>
                            {column.cell(row, index)}
                        </TableCell>
                    ))}
                </TableRow>
            ))}
        </TableBody>
    </Table>
)

import type { FC } from 'react'
import { formatCount, formatHoursMinutes } from '@shared/lib/format'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@shared/ui/table'

type PlayDayTableRow = { date: string; plays: number; playMs: number }

type PlayDayTableProps = {
    rows: readonly PlayDayTableRow[]
    labels: { date: string; plays: string; time: string }
}

export const PlayDayTable: FC<PlayDayTableProps> = ({ rows, labels }) => (
    <Table className='text-xs'>
        <TableHeader>
            <TableRow>
                <TableHead>{labels.date}</TableHead>
                <TableHead className='text-right'>{labels.plays}</TableHead>
                <TableHead className='text-right'>{labels.time}</TableHead>
            </TableRow>
        </TableHeader>
        <TableBody>
            {rows.map((row) => (
                <TableRow key={row.date}>
                    <TableCell className='tabular-nums'>{row.date}</TableCell>
                    <TableCell className='text-right tabular-nums'>{formatCount(row.plays)}</TableCell>
                    <TableCell className='text-right tabular-nums'>{formatHoursMinutes(row.playMs)}</TableCell>
                </TableRow>
            ))}
        </TableBody>
    </Table>
)

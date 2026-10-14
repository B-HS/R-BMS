'use client'

import type { ComponentProps, FC } from 'react'
import { TableChartDetails } from '@widgets/table/table-chart-details'
import type { TableSnapshotEntry } from '@entities/table-snapshot.type'
import { Dialog, DialogContent } from '@shared/ui/dialog'

type TableChartDialogProps = Omit<ComponentProps<typeof TableChartDetails>, 'entry'> & {
    entry: TableSnapshotEntry | null
    isOpen: boolean
    onOpenChange: (isOpen: boolean) => void
    onCloseAutoFocus: ComponentProps<typeof DialogContent>['onCloseAutoFocus']
}

export const TableChartDialog: FC<TableChartDialogProps> = ({ isOpen, onOpenChange, onCloseAutoFocus, entry, ...bodyProps }) => (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
        <DialogContent
            onCloseAutoFocus={onCloseAutoFocus}
            className='flex max-h-[calc(100dvh-2rem)] flex-col gap-0 overflow-hidden rounded-none p-0 sm:max-w-md'>
            {entry && <TableChartDetails entry={entry} {...bodyProps} />}
        </DialogContent>
    </Dialog>
)

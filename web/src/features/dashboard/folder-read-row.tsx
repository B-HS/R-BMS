import type { FC } from 'react'
import Link from 'next/link'
import { formatDateTime } from '@shared/lib/format'

type FolderReadRowProps = {
    folderId: string
    name: string
    updatedAt: number
    itemCountLabel: string
}

export const FolderReadRow: FC<FolderReadRowProps> = ({ folderId, name, updatedAt, itemCountLabel }) => (
    <li className='border-border hover:bg-muted/50 relative flex min-h-10 items-center justify-between gap-3 border-b px-3 py-1.5'>
        <div className='flex min-w-0 flex-col'>
            <Link
                href={`/folders/${encodeURIComponent(folderId)}`}
                className='truncate text-xs font-medium outline-none after:absolute after:inset-0 focus-visible:underline'>
                {name}
            </Link>
            <span className='text-muted-foreground text-2xs tabular-nums'>{formatDateTime(updatedAt)}</span>
        </div>
        <span className='shrink-0 text-xs tabular-nums'>{itemCountLabel}</span>
    </li>
)

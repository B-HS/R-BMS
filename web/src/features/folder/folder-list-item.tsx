'use client'

import { useRef, type FC } from 'react'
import Link from 'next/link'
import { ArrowDown, ArrowUp, Ellipsis, Pencil, Trash2 } from 'lucide-react'
import type { FolderSummary } from '@entities/folder.type'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { Button } from '@shared/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'

type FolderListItemProps = {
    folder: FolderSummary
    href: string
    isFirst: boolean
    isLast: boolean
    onMove: (direction: 'up' | 'down') => void
    onEdit: (menuTrigger: HTMLButtonElement | null) => void
    onDelete: (menuTrigger: HTMLButtonElement | null) => void
}

export const FolderListItem: FC<FolderListItemProps> = ({ folder, href, isFirst, isLast, onMove, onEdit, onDelete }) => {
    const menuTriggerRef = useRef<HTMLButtonElement>(null)

    const countLabel = `${formatCount(folder.item_count)}곡`
    const updatedLabel = `${formatDateTime(folder.updated_at)} 수정`

    return (
        <li
            data-folder-id={folder.id}
            className='border-border bg-card hover:bg-muted/40 has-[a:focus-visible]:ring-ring/50 relative flex min-h-14 items-center gap-3 border-b px-3 py-2 has-[a:focus-visible]:ring-3 has-[a:focus-visible]:ring-inset'>
            <div className='flex min-w-0 flex-1 flex-col gap-0.5'>
                <Link href={href} className='truncate text-sm font-medium outline-none after:absolute after:inset-0'>
                    {folder.name}
                </Link>
                <p className='text-muted-foreground truncate text-xs'>{folder.description || '설명 없음'}</p>
                <p className='text-muted-foreground text-2xs tabular-nums sm:hidden'>
                    {countLabel} · {updatedLabel}
                </p>
            </div>
            <span className='hidden w-16 shrink-0 text-right text-xs tabular-nums sm:block'>{countLabel}</span>
            <span className='text-muted-foreground hidden w-36 shrink-0 text-right text-xs tabular-nums md:block'>{updatedLabel}</span>
            <div className='relative z-(--z-rail) flex shrink-0 items-center gap-0.5'>
                <Button
                    variant='ghost'
                    size='icon-sm'
                    aria-label={`${folder.name} 위로`}
                    aria-disabled={isFirst}
                    data-action='up'
                    className='aria-disabled:opacity-40'
                    onClick={() => !isFirst && onMove('up')}>
                    <ArrowUp />
                </Button>
                <Button
                    variant='ghost'
                    size='icon-sm'
                    aria-label={`${folder.name} 아래로`}
                    aria-disabled={isLast}
                    data-action='down'
                    className='aria-disabled:opacity-40'
                    onClick={() => !isLast && onMove('down')}>
                    <ArrowDown />
                </Button>
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <Button ref={menuTriggerRef} variant='ghost' size='icon-sm' aria-label={`${folder.name} 메뉴`}>
                            <Ellipsis />
                        </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align='end' className='w-44'>
                        <DropdownMenuItem onSelect={() => onEdit(menuTriggerRef.current)}>
                            <Pencil />
                            이름·설명 고치기
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant='destructive' onSelect={() => onDelete(menuTriggerRef.current)}>
                            <Trash2 />
                            폴더 지우기
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
            </div>
        </li>
    )
}

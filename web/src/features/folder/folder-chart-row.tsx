'use client'

import type { FC } from 'react'
import { ArrowDown, ArrowUp, ChevronsDown, ChevronsUp, Ellipsis, StickyNote, X } from 'lucide-react'
import type { FolderItem } from '@entities/folder.type'
import { formatHash } from '@shared/lib/format'
import { Button } from '@shared/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'

export type FolderChartMove = 'up' | 'down' | 'top' | 'bottom'

type FolderChartRowProps = {
    item: FolderItem
    position: number
    levelLabel?: string
    isNew: boolean
    isFirst: boolean
    isLast: boolean
    onMove: (move: FolderChartMove) => void
    onEditNote: () => void
    onRemove: () => void
}

const UNTITLED_LABEL = '제목 미등록'

export const FolderChartRow: FC<FolderChartRowProps> = ({ item, position, levelLabel, isNew, isFirst, isLast, onMove, onEditNote, onRemove }) => {
    const title = item.title.trim() || `${UNTITLED_LABEL} ${formatHash(item.md5)}`

    return (
        <li data-md5={item.md5} className='border-border bg-card flex min-h-10 items-center gap-2 border-b py-1 pr-1.5 pl-1'>
            <span className='text-muted-foreground w-8 shrink-0 text-right text-xs tabular-nums'>{position}</span>
            <span
                className='w-10 shrink-0 font-mono text-xs font-semibold tabular-nums'
                style={{ color: levelLabel ? 'var(--rbms-difficulty-insane)' : undefined }}>
                {levelLabel ?? <span className='text-muted-foreground font-normal'>—</span>}
            </span>
            <div className='flex min-w-0 flex-1 flex-col'>
                <span className='flex min-w-0 items-center gap-1.5'>
                    <span className='truncate text-xs font-medium'>{title}</span>
                    {isNew && <span className='border-foreground/40 text-2xs shrink-0 border px-1 leading-tight font-medium'>새로</span>}
                </span>
                <span className='text-muted-foreground text-2xs truncate'>
                    {item.artist || '아티스트 미등록'}
                    {item.note && <span className='text-foreground/80'> · 메모 {item.note}</span>}
                </span>
            </div>
            <span className='text-muted-foreground text-2xs hidden w-20 shrink-0 font-mono lg:block'>{formatHash(item.md5)}</span>
            <div className='flex shrink-0 items-center gap-0.5'>
                <Button
                    variant='ghost'
                    size='icon-xs'
                    aria-label={`${title} 위로`}
                    aria-disabled={isFirst}
                    data-action='up'
                    className='aria-disabled:opacity-40'
                    onClick={() => !isFirst && onMove('up')}>
                    <ArrowUp />
                </Button>
                <Button
                    variant='ghost'
                    size='icon-xs'
                    aria-label={`${title} 아래로`}
                    aria-disabled={isLast}
                    data-action='down'
                    className='aria-disabled:opacity-40'
                    onClick={() => !isLast && onMove('down')}>
                    <ArrowDown />
                </Button>
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <Button variant='ghost' size='icon-xs' aria-label={`${title} 메뉴`} data-action='menu'>
                            <Ellipsis />
                        </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align='end' className='w-40'>
                        <DropdownMenuItem disabled={isFirst} onSelect={() => onMove('top')}>
                            <ChevronsUp />맨 위로
                        </DropdownMenuItem>
                        <DropdownMenuItem disabled={isLast} onSelect={() => onMove('bottom')}>
                            <ChevronsDown />맨 아래로
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem onSelect={onEditNote}>
                            <StickyNote />
                            메모 고치기
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
                <Button variant='ghost' size='icon-xs' aria-label={`${title} 빼기`} data-action='remove' onClick={onRemove}>
                    <X />
                </Button>
            </div>
        </li>
    )
}

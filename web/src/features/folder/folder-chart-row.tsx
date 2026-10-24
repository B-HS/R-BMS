'use client'

import type { FC } from 'react'
import { ArrowDown, ArrowUp, ChevronsDown, ChevronsUp, Ellipsis, StickyNote, X } from 'lucide-react'
import { useTranslations } from 'next-intl'
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

export const FolderChartRow: FC<FolderChartRowProps> = ({ item, position, levelLabel, isNew, isFirst, isLast, onMove, onEditNote, onRemove }) => {
    const t = useTranslations('pages.folder.chartRow')
    const tShared = useTranslations('pages.shared')
    const title = item.title.trim() || tShared('untitledWithHash', { hash: formatHash(item.md5) })

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
                    {isNew && <span className='border-foreground/40 text-2xs shrink-0 border px-1 leading-tight font-medium'>{t('new')}</span>}
                </span>
                <span className='text-muted-foreground text-2xs truncate'>
                    {item.artist || tShared('artistMissing')}
                    {item.note && <span className='text-foreground/80'> · {t('note', { note: item.note })}</span>}
                </span>
            </div>
            <span className='text-muted-foreground text-2xs hidden w-20 shrink-0 font-mono lg:block'>{formatHash(item.md5)}</span>
            <div className='flex shrink-0 items-center gap-0.5'>
                <Button
                    variant='ghost'
                    size='icon-xs'
                    aria-label={t('moveUp', { title })}
                    aria-disabled={isFirst}
                    data-action='up'
                    className='aria-disabled:opacity-40'
                    onClick={() => !isFirst && onMove('up')}>
                    <ArrowUp />
                </Button>
                <Button
                    variant='ghost'
                    size='icon-xs'
                    aria-label={t('moveDown', { title })}
                    aria-disabled={isLast}
                    data-action='down'
                    className='aria-disabled:opacity-40'
                    onClick={() => !isLast && onMove('down')}>
                    <ArrowDown />
                </Button>
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <Button variant='ghost' size='icon-xs' aria-label={t('menu', { title })} data-action='menu'>
                            <Ellipsis />
                        </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align='end' className='w-40'>
                        <DropdownMenuItem disabled={isFirst} onSelect={() => onMove('top')}>
                            <ChevronsUp />
                            {t('moveTop')}
                        </DropdownMenuItem>
                        <DropdownMenuItem disabled={isLast} onSelect={() => onMove('bottom')}>
                            <ChevronsDown />
                            {t('moveBottom')}
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem onSelect={onEditNote}>
                            <StickyNote />
                            {t('editNote')}
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
                <Button variant='ghost' size='icon-xs' aria-label={t('remove', { title })} data-action='remove' onClick={onRemove}>
                    <X />
                </Button>
            </div>
        </li>
    )
}

'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'
import { formatCount } from '@shared/lib/format'
import { useRivalList, useSaveRivals } from '@entities/rival.query'
import type { PlayerProfile } from '@entities/player.type'

export const RivalPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const [draft, setDraft] = useState('')

    const t = useTranslations('settings.rivalPanel')
    const { data, isLoading, isError, isFetching, refetch } = useRivalList(playerId)
    const save = useSaveRivals(playerId)

    const rows = data ?? []

    const commit = (nextIds: string[], message: string) => {
        save.mutate(nextIds, {
            onSuccess: (saved) => {
                setDraft('')
                const missing = nextIds.filter((id) => !saved.some((rival) => rival.id === id))
                if (missing.length > 0) {
                    toast.warning(t('notFound', { ids: missing.join(', ') }))
                    return
                }
                toast.success(message)
            },
        })
    }

    const addRival = () => {
        const nextId = draft.trim()
        if (rows.some((rival) => rival.id === nextId)) {
            toast.warning(t('duplicate'))
            return
        }
        commit([...rows.map((rival) => rival.id), nextId], t('added'))
    }

    const columns: DataTableColumn<PlayerProfile>[] = [
        {
            key: 'name',
            label: t('columnPlayer'),
            flex: true,
            cell: (row) => (
                <Link href={`/players/${row.id}`} className='hover:underline'>
                    {row.name}
                </Link>
            ),
        },
        { key: 'id', label: 'ID', width: 128, mono: true, cell: (row) => row.id },
        { key: 'total_plays', label: t('columnPlays'), width: 96, align: 'right', cell: (row) => formatCount(row.total_plays) },
        {
            key: 'action',
            label: '',
            width: 72,
            align: 'right',
            cell: (row) => (
                <Button
                    variant='ghost'
                    size='sm'
                    disabled={save.isPending}
                    onClick={() =>
                        commit(
                            rows.filter((rival) => rival.id !== row.id).map((rival) => rival.id),
                            t('removed'),
                        )
                    }>
                    {t('remove')}
                </Button>
            ),
        },
    ]

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-2'>
            <div className='flex items-center gap-2'>
                <Input
                    aria-label={t('inputLabel')}
                    placeholder={t('inputPlaceholder')}
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                />
                <Button size='sm' disabled={draft.trim() === '' || save.isPending} onClick={addRival}>
                    {t('add')}
                </Button>
            </div>
            {isLoading && <TableSkeleton rows={3} />}
            {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
            {!isLoading && !isError && rows.length === 0 && <EmptyState title={t('emptyTitle')} />}
            {!isLoading && !isError && rows.length > 0 && <DataTable columns={columns} rows={rows} rowKey={(row) => row.id} caption={t('caption')} />}
            <PanelFootnote>{t('footnote')}</PanelFootnote>
        </PanelCard>
    )
}

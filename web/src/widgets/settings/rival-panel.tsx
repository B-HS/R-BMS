'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
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

    const { data, isLoading, isError } = useRivalList(playerId)
    const save = useSaveRivals(playerId)

    const rows = data ?? []

    const commit = (nextIds: string[], message: string) => {
        save.mutate(nextIds, {
            onSuccess: (saved) => {
                setDraft('')
                const missing = nextIds.filter((id) => !saved.some((rival) => rival.id === id))
                if (missing.length > 0) {
                    toast.warning(`해당 플레이어를 찾을 수 없습니다: ${missing.join(', ')}`)
                    return
                }
                toast.success(message)
            },
        })
    }

    const addRival = () => {
        const nextId = draft.trim()
        if (rows.some((rival) => rival.id === nextId)) {
            toast.warning('이미 등록된 라이벌입니다.')
            return
        }
        commit([...rows.map((rival) => rival.id), nextId], '라이벌을 추가했습니다.')
    }

    const columns: DataTableColumn<PlayerProfile>[] = [
        {
            key: 'name',
            label: '플레이어',
            flex: true,
            cell: (row) => (
                <Link href={`/players/${row.id}`} className='hover:underline'>
                    {row.name}
                </Link>
            ),
        },
        { key: 'id', label: 'ID', width: 128, mono: true, cell: (row) => row.id },
        { key: 'total_plays', label: '플레이', width: 96, align: 'right', cell: (row) => formatCount(row.total_plays) },
        { key: 'rank_points', label: '랭크 포인트', width: 112, align: 'right', cell: (row) => row.rank_points.toFixed(2) },
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
                            '라이벌을 삭제했습니다.',
                        )
                    }>
                    삭제
                </Button>
            ),
        },
    ]

    return (
        <PanelCard title='라이벌' contentClassName='flex flex-col gap-2'>
            <div className='flex items-center gap-2'>
                <Input
                    aria-label='추가할 라이벌 플레이어 ID'
                    placeholder='플레이어 ID'
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                />
                <Button size='sm' disabled={draft.trim() === '' || save.isPending} onClick={addRival}>
                    추가
                </Button>
            </div>
            {isLoading && <TableSkeleton rows={3} />}
            {!isLoading && isError && <LoadErrorState title='라이벌 목록을 불러오지 못했습니다' />}
            {!isLoading && !isError && rows.length === 0 && <EmptyState title='등록한 라이벌이 없습니다' />}
            {!isLoading && !isError && rows.length > 0 && <DataTable columns={columns} rows={rows} rowKey={(row) => row.id} caption='라이벌 목록' />}
            <PanelFootnote>라이벌은 클라이언트 랭킹 조회에서 `rival_of` 필터로 쓰입니다.</PanelFootnote>
        </PanelCard>
    )
}

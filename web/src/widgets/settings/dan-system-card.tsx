'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { toast } from 'sonner'
import { DanBadge } from '@features/dan/dan-badge'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { formatDateTime } from '@shared/lib/format'
import { useSetDanDisplay } from '@entities/dan.query'
import type { AcquiredDan, DanDisplay, DanDisplayRequest, DanSystem } from '@entities/dan.type'

const DISPLAY_AUTO = 'auto'
const DISPLAY_HIDDEN = 'hidden'
const DISPLAY_LABEL_PREFIX = 'label:'

const ACQUIRED_COLUMNS: DataTableColumn<AcquiredDan>[] = [
    { key: 'label', label: '단위', width: 96, cell: (row) => row.label },
    {
        key: 'course',
        label: '코스',
        flex: true,
        cell: (row) => (
            <Link href={`/courses/${encodeURIComponent(row.course_hash)}`} className='hover:underline'>
                {row.course_name}
            </Link>
        ),
    },
    { key: 'course_gauge', label: '게이지', width: 112, hideBelow: 'sm', cell: (row) => row.course_gauge },
    { key: 'acquired_at', label: '받은 날', width: 144, align: 'right', cell: (row) => formatDateTime(row.acquired_at) },
]

const selectValueOf = (display: DanDisplay) => {
    if (display.mode !== 'label') return display.mode
    return `${DISPLAY_LABEL_PREFIX}${display.label ?? ''}`
}

const toDisplayRequest = (system: DanSystem['system'], value: string): DanDisplayRequest => {
    if (value.startsWith(DISPLAY_LABEL_PREFIX)) return { system, mode: 'label', label: value.slice(DISPLAY_LABEL_PREFIX.length) }
    return { system, mode: value === DISPLAY_HIDDEN ? 'hidden' : 'auto' }
}

type DanSystemCardProps = {
    system: DanSystem
    playerId: string
}

export const DanSystemCard: FC<DanSystemCardProps> = ({ system, playerId }) => {
    const setDisplay = useSetDanDisplay(playerId)

    const handleValueChange = (value: string) =>
        setDisplay.mutate(toDisplayRequest(system.system, value), { onSuccess: () => toast.success('표시할 단위를 바꿨습니다') })

    return (
        <PanelCard
            title={system.name}
            action={
                <Select value={selectValueOf(system.display)} onValueChange={handleValueChange} disabled={setDisplay.isPending}>
                    <SelectTrigger size='sm' aria-label={`${system.name} 표시 단위`} className='w-52 max-w-full'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value={DISPLAY_AUTO}>자동 (가장 높은 단위)</SelectItem>
                        <SelectItem value={DISPLAY_HIDDEN}>숨김</SelectItem>
                        {system.acquired.map((acquired) => (
                            <SelectItem key={acquired.label} value={`${DISPLAY_LABEL_PREFIX}${acquired.label}`}>
                                {acquired.label}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            }
            contentClassName='flex flex-col gap-2'>
            <p className='flex flex-wrap items-center gap-1.5 text-xs'>
                다른 플레이어에게 보이는 단위
                {system.shown === null ? (
                    <span className='text-muted-foreground'>없음</span>
                ) : (
                    <DanBadge dan={{ system: system.system, name: system.name, label: system.shown }} />
                )}
            </p>
            {system.acquired.length === 0 ? (
                <PanelFootnote>획득한 단위가 없습니다.</PanelFootnote>
            ) : (
                <DataTable columns={ACQUIRED_COLUMNS} rows={system.acquired} rowKey={(row) => row.label} caption={`${system.name} 획득 단위`} />
            )}
        </PanelCard>
    )
}

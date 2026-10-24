'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
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
    const t = useTranslations('settings.danSystemCard')
    const setDisplay = useSetDanDisplay(playerId)

    const handleValueChange = (value: string) =>
        setDisplay.mutate(toDisplayRequest(system.system, value), { onSuccess: () => toast.success(t('changed')) })

    const acquiredColumns: DataTableColumn<AcquiredDan>[] = [
        { key: 'label', label: t('columnDan'), width: 96, cell: (row) => row.label },
        {
            key: 'course',
            label: t('columnCourse'),
            flex: true,
            cell: (row) => (
                <Link href={`/courses/${encodeURIComponent(row.course_hash)}`} className='hover:underline'>
                    {row.course_name}
                </Link>
            ),
        },
        { key: 'course_gauge', label: t('columnGauge'), width: 112, hideBelow: 'sm', cell: (row) => row.course_gauge },
        { key: 'acquired_at', label: t('columnAcquiredAt'), width: 144, align: 'right', cell: (row) => formatDateTime(row.acquired_at) },
    ]

    return (
        <PanelCard
            title={system.name}
            action={
                <Select value={selectValueOf(system.display)} onValueChange={handleValueChange} disabled={setDisplay.isPending}>
                    <SelectTrigger size='sm' aria-label={t('displayLabel', { system: system.name })} className='w-52 max-w-full'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value={DISPLAY_AUTO}>{t('displayAuto')}</SelectItem>
                        <SelectItem value={DISPLAY_HIDDEN}>{t('displayHidden')}</SelectItem>
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
                {t('shownToOthers')}
                {system.shown === null ? (
                    <span className='text-muted-foreground'>{t('shownNone')}</span>
                ) : (
                    <DanBadge dan={{ system: system.system, name: system.name, label: system.shown }} />
                )}
            </p>
            {system.acquired.length === 0 ? (
                <PanelFootnote>{t('noneAcquired')}</PanelFootnote>
            ) : (
                <DataTable
                    columns={acquiredColumns}
                    rows={system.acquired}
                    rowKey={(row) => row.label}
                    caption={t('caption', { system: system.name })}
                />
            )}
        </PanelCard>
    )
}

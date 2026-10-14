'use client'

import { useId, type FC } from 'react'
import { cn } from '@shared/lib/cn'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'

type FilterOption = { value: string; label: string; disabled?: boolean }

type TableFilterFieldsProps = {
    layout: 'inline' | 'stacked'
    levelOptions: FilterOption[]
    from: string
    to: string
    onFromChange: (value: string) => void
    onToChange: (value: string) => void
    lampOptions: FilterOption[]
    lamp: string
    isLampDisabled: boolean
    lampHint?: string
    onLampChange: (value: string) => void
    sortOptions: FilterOption[]
    sort: string
    onSortChange: (value: string) => void
}

const FIELD_LABEL_CLASS_NAME = 'rbms-micro-label shrink-0'

export const TableFilterFields: FC<TableFilterFieldsProps> = ({
    layout,
    levelOptions,
    from,
    to,
    onFromChange,
    onToChange,
    lampOptions,
    lamp,
    isLampDisabled,
    lampHint,
    onLampChange,
    sortOptions,
    sort,
    onSortChange,
}) => {
    const fieldId = useId()
    const isStacked = layout === 'stacked'
    const fieldClassName = cn('flex min-w-0 gap-2', isStacked ? 'flex-col' : 'items-center')
    const triggerSize = isStacked ? 'default' : 'sm'

    return (
        <div className={cn('flex min-w-0 gap-3', isStacked ? 'flex-col' : 'flex-wrap items-center')}>
            <div role='group' aria-labelledby={`${fieldId}-level`} className={fieldClassName}>
                <span id={`${fieldId}-level`} className={FIELD_LABEL_CLASS_NAME}>
                    레벨
                </span>
                <div className='flex items-center gap-1'>
                    <Select value={from} onValueChange={onFromChange}>
                        <SelectTrigger aria-label='시작 레벨' size={triggerSize} className={isStacked ? 'flex-1' : 'w-24'}>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {levelOptions.map((option) => (
                                <SelectItem key={option.value} value={option.value} disabled={option.disabled}>
                                    {option.label}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                    <span aria-hidden className='text-muted-foreground text-xs'>
                        ~
                    </span>
                    <Select value={to} onValueChange={onToChange}>
                        <SelectTrigger aria-label='끝 레벨' size={triggerSize} className={isStacked ? 'flex-1' : 'w-24'}>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {levelOptions.map((option) => (
                                <SelectItem key={option.value} value={option.value} disabled={option.disabled}>
                                    {option.label}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
            </div>
            <div className={fieldClassName}>
                <label htmlFor={`${fieldId}-lamp`} className={FIELD_LABEL_CLASS_NAME}>
                    램프
                </label>
                <Select value={lamp} onValueChange={onLampChange} disabled={isLampDisabled}>
                    <SelectTrigger
                        id={`${fieldId}-lamp`}
                        size={triggerSize}
                        className={isStacked ? 'w-full' : 'w-44'}
                        aria-describedby={lampHint ? `${fieldId}-lamp-hint` : undefined}>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {lampOptions.map((option) => (
                            <SelectItem key={option.value} value={option.value}>
                                {option.label}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                {lampHint && (
                    <span id={`${fieldId}-lamp-hint`} className={cn('text-muted-foreground text-xs', !isStacked && 'sr-only')}>
                        {lampHint}
                    </span>
                )}
            </div>
            <div className={fieldClassName}>
                <label htmlFor={`${fieldId}-sort`} className={FIELD_LABEL_CLASS_NAME}>
                    정렬
                </label>
                <Select value={sort} onValueChange={onSortChange}>
                    <SelectTrigger id={`${fieldId}-sort`} size={triggerSize} className={isStacked ? 'w-full' : 'w-36'}>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {sortOptions.map((option) => (
                            <SelectItem key={option.value} value={option.value} disabled={option.disabled}>
                                {option.label}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            </div>
        </div>
    )
}

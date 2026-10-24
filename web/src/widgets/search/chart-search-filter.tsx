'use client'

import { useState, type FC, type FormEvent } from 'react'
import { Search } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { FilterBar } from '@features/filter/filter-bar'
import { FilterField } from '@features/filter/filter-field'
import { ColumnToggle } from '@features/table/column-toggle'
import { SortSelect } from '@features/table/sort-select'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'
import { ToggleGroup, ToggleGroupItem } from '@shared/ui/toggle-group'

const ALL_MODES = 'all'

const MODE_OPTIONS = [
    { value: 'BEAT_7K', label: '7K' },
    { value: 'BEAT_14K', label: '14K' },
    { value: 'BEAT_5K', label: '5K' },
]

type ChartSearchFilterProps = {
    keyword: string
    mode: string
    level: string
    sort: string
    sortOptions: { value: string; label: string }[]
    columnOptions: { key: string; label: string }[]
    activeColumns: string[]
    summary: string
    onChange: (next: Record<string, string | undefined>) => void
    onToggleColumn: (key: string) => void
}

export const ChartSearchFilter: FC<ChartSearchFilterProps> = ({
    keyword,
    mode,
    level,
    sort,
    sortOptions,
    columnOptions,
    activeColumns,
    summary,
    onChange,
    onToggleColumn,
}) => {
    const [keywordDraft, setKeywordDraft] = useState(keyword)
    const [levelDraft, setLevelDraft] = useState(level)

    const t = useTranslations('browse.search.filter')

    const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault()
        onChange({ q: keywordDraft.trim(), level: levelDraft.trim() })
    }

    return (
        <FilterBar label={t('label')} summary={summary}>
            <form onSubmit={handleSubmit} role='search' className='flex w-full min-w-0 flex-wrap items-center gap-2 sm:w-auto'>
                <div className='relative min-w-0 flex-1 sm:w-64 sm:flex-none'>
                    <Search className='text-muted-foreground pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2' aria-hidden />
                    <Input
                        type='search'
                        aria-label={t('keywordLabel')}
                        placeholder={t('keywordPlaceholder')}
                        value={keywordDraft}
                        onChange={(event) => setKeywordDraft(event.target.value)}
                        className='h-7 pl-7 text-xs'
                    />
                </div>
                <FilterField label={t('levelLabel')} htmlFor='search-level'>
                    <Input
                        id='search-level'
                        type='number'
                        inputMode='numeric'
                        min={0}
                        placeholder={t('all')}
                        value={levelDraft}
                        onChange={(event) => setLevelDraft(event.target.value)}
                        className='h-7 w-16 text-xs tabular-nums'
                    />
                </FilterField>
                <Button type='submit' variant='secondary' size='sm'>
                    {t('submit')}
                </Button>
            </form>
            <ToggleGroup
                type='single'
                variant='outline'
                size='sm'
                spacing={0}
                aria-label={t('modeLabel')}
                value={mode || ALL_MODES}
                onValueChange={(value) => value && onChange({ mode: value === ALL_MODES ? '' : value })}>
                <ToggleGroupItem value={ALL_MODES} className='px-2 text-xs'>
                    {t('all')}
                </ToggleGroupItem>
                {MODE_OPTIONS.map((option) => (
                    <ToggleGroupItem key={option.value} value={option.value} className='px-2 text-xs'>
                        {option.label}
                    </ToggleGroupItem>
                ))}
            </ToggleGroup>
            <SortSelect label={t('sortLabel')} value={sort} options={sortOptions} onValueChange={(value) => onChange({ sort: value })} />
            <ColumnToggle options={columnOptions} selected={activeColumns} onToggle={onToggleColumn} />
        </FilterBar>
    )
}

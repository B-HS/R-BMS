'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { cn } from '@shared/lib/cn'
import {
    AI_RECORD_QUERY_FIELD_LABEL,
    AI_RECORD_QUERY_FIELD_OPS,
    AI_RECORD_QUERY_FORM_LIMIT,
    AI_RECORD_QUERY_ISSUE_LABEL,
    AI_RECORD_QUERY_OP_LABEL,
    AI_RECORD_QUERY_SORT_LABEL,
    AI_RECORD_QUERY_VALUE_HINT,
    type AiRecordQueryField,
    type AiRecordQueryFilter,
    type AiRecordQueryIssue,
    type AiRecordQueryOp,
    type AiRecordQueryPlan,
} from '@entities/ai.type'

type DraftFilter = AiRecordQueryFilter & { id: string }

const SORT_DIRS = ['desc', 'asc'] as const satisfies readonly AiRecordQueryPlan['sort_dir'][]

const isField = (value: string): value is AiRecordQueryField => value in AI_RECORD_QUERY_FIELD_OPS
const isOp = (value: string): value is AiRecordQueryOp => value in AI_RECORD_QUERY_OP_LABEL
const isSortField = (value: string): value is AiRecordQueryPlan['sort_field'] => value in AI_RECORD_QUERY_SORT_LABEL
const isSortDir = (value: string): value is AiRecordQueryPlan['sort_dir'] => SORT_DIRS.some((dir) => dir === value)

const FIELDS = Object.keys(AI_RECORD_QUERY_FIELD_OPS).filter(isField)
const SORT_FIELDS = Object.keys(AI_RECORD_QUERY_SORT_LABEL).filter(isSortField)
const DEFAULT_FIELD = 'lamp' satisfies AiRecordQueryField

const firstOp = (field: AiRecordQueryField): AiRecordQueryOp => AI_RECORD_QUERY_FIELD_OPS[field][0]

const toDraft = (filter: AiRecordQueryFilter): DraftFilter => ({ ...filter, id: crypto.randomUUID() })

type RecordQueryFilterEditorProps = {
    initialPlan: AiRecordQueryPlan
    issues: readonly AiRecordQueryIssue[]
    isRunning: boolean
    onRun: (plan: AiRecordQueryPlan) => void
}

export const RecordQueryFilterEditor: FC<RecordQueryFilterEditorProps> = ({ initialPlan, issues, isRunning, onRun }) => {
    const [filters, setFilters] = useState(() => initialPlan.filters.map(toDraft))
    const [sortField, setSortField] = useState(initialPlan.sort_field)
    const [sortDir, setSortDir] = useState(initialPlan.sort_dir)
    const [limitText, setLimitText] = useState(String(initialPlan.limit))
    const [submittedIds, setSubmittedIds] = useState(() => filters.map((filter) => filter.id))

    const t = useTranslations('ai.recordQueryEditor')
    const issueFor = (id: string) => issues.find((issue) => submittedIds[issue.index] === id)
    const updateFilter = (id: string, change: Partial<AiRecordQueryFilter>) =>
        setFilters((current) => current.map((filter) => (filter.id === id ? { ...filter, ...change } : filter)))
    const changeField = (id: string, field: AiRecordQueryField) =>
        setFilters((current) =>
            current.map((filter) => {
                if (filter.id !== id) return filter
                const isOpAllowed = AI_RECORD_QUERY_FIELD_OPS[field].some((op) => op === filter.op)
                return { ...filter, field, op: isOpAllowed ? filter.op : firstOp(field) }
            }),
        )
    const parsedLimit = Number.parseInt(limitText, 10)

    const handleRun = () => {
        setSubmittedIds(filters.map((filter) => filter.id))
        onRun({
            filters: filters.map(({ field, op, value }) => ({ field, op, value })),
            sort_field: sortField,
            sort_dir: sortDir,
            limit: Number.isNaN(parsedLimit) ? AI_RECORD_QUERY_FORM_LIMIT.ROWS_MAX : parsedLimit,
        })
    }

    return (
        <form
            aria-label={t('formLabel')}
            className='flex flex-col gap-3'
            noValidate
            onSubmit={(event) => {
                event.preventDefault()
                handleRun()
            }}>
            {filters.length === 0 && <p className='text-muted-foreground text-xs'>{t('noFilters')}</p>}
            <ul aria-label={t('listLabel')} className='flex flex-col gap-2'>
                {filters.map((filter, index) => {
                    const issue = issueFor(filter.id)
                    const position = index + 1
                    return (
                        <li
                            key={filter.id}
                            aria-label={t('conditionLabel', { index: position })}
                            className={cn('border-border flex flex-wrap items-center gap-2 border p-2', issue && 'border-destructive')}>
                            <Select value={filter.field} onValueChange={(value) => isField(value) && changeField(filter.id, value)}>
                                <SelectTrigger size='sm' className='w-36' aria-label={t('fieldLabel', { index: position })}>
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {FIELDS.map((field) => (
                                        <SelectItem key={field} value={field}>
                                            {AI_RECORD_QUERY_FIELD_LABEL[field]}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                            <Select value={filter.op} onValueChange={(value) => isOp(value) && updateFilter(filter.id, { op: value })}>
                                <SelectTrigger size='sm' className='w-28' aria-label={t('opLabel', { index: position })}>
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {AI_RECORD_QUERY_FIELD_OPS[filter.field].map((op) => (
                                        <SelectItem key={op} value={op}>
                                            {AI_RECORD_QUERY_OP_LABEL[op]}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                            <Input
                                aria-label={t('valueLabel', { index: position })}
                                aria-invalid={Boolean(issue)}
                                className='h-7 w-48 max-w-full'
                                value={filter.value}
                                placeholder={AI_RECORD_QUERY_VALUE_HINT[filter.field]}
                                onChange={(event) => updateFilter(filter.id, { value: event.target.value })}
                            />
                            <Button
                                type='button'
                                variant='ghost'
                                size='xs'
                                onClick={() => setFilters((current) => current.filter((candidate) => candidate.id !== filter.id))}>
                                {t('removeLabel', { index: position })}
                            </Button>
                            {issue && (
                                <p role='alert' className='text-destructive w-full text-xs'>
                                    {AI_RECORD_QUERY_ISSUE_LABEL[issue.reason]}
                                </p>
                            )}
                        </li>
                    )
                })}
            </ul>
            <div className='flex flex-wrap items-center gap-2'>
                <Button
                    type='button'
                    variant='outline'
                    size='xs'
                    disabled={filters.length >= AI_RECORD_QUERY_FORM_LIMIT.MAX_FILTERS}
                    onClick={() => setFilters((current) => [...current, toDraft({ field: DEFAULT_FIELD, op: firstOp(DEFAULT_FIELD), value: '' })])}>
                    {t('add')}
                </Button>
                <Select value={sortField} onValueChange={(value) => isSortField(value) && setSortField(value)}>
                    <SelectTrigger size='sm' className='w-28' aria-label={t('sortFieldLabel')}>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {SORT_FIELDS.map((field) => (
                            <SelectItem key={field} value={field}>
                                {AI_RECORD_QUERY_SORT_LABEL[field]}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                <Select value={sortDir} onValueChange={(value) => isSortDir(value) && setSortDir(value)}>
                    <SelectTrigger size='sm' className='w-28' aria-label={t('sortDirLabel')}>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {SORT_DIRS.map((dir) => (
                            <SelectItem key={dir} value={dir}>
                                {t(`sortDir.${dir}`)}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                <Input
                    aria-label={t('limitLabel')}
                    inputMode='numeric'
                    className='h-7 w-20'
                    value={limitText}
                    onChange={(event) => setLimitText(event.target.value)}
                />
                <Button type='submit' size='sm' disabled={isRunning}>
                    {t('run')}
                </Button>
            </div>
            <p className='text-muted-foreground text-xs'>
                {t('footnote', { min: AI_RECORD_QUERY_FORM_LIMIT.ROWS_MIN, max: AI_RECORD_QUERY_FORM_LIMIT.ROWS_MAX })}
            </p>
        </form>
    )
}

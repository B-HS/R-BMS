'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { AiChartCandidateList } from '@features/ai/ai-chart-candidate-list'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Button } from '@shared/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { formatCount } from '@shared/lib/format'
import { useAiPracticeFolder, useCreateAiPracticeFolder, usePreviewAiPracticeFolder } from '@entities/ai.query'
import { AI_PATTERN_TAG_LABEL, AI_SCORE_FEATURE_FORM_LIMIT, isAiPatternTag, type AiPatternTag, type AiPracticeSourceInput } from '@entities/ai.type'

const SOURCES = ['recommend', 'tag'] as const
type PanelSource = (typeof SOURCES)[number]

const isPanelSource = (value: string): value is PanelSource => SOURCES.some((source) => source === value)
const TAGS = Object.keys(AI_PATTERN_TAG_LABEL).filter(isAiPatternTag)
const DEFAULT_TAG = 'trill' satisfies AiPatternTag

export const AiPracticeFolderPanel: FC = () => {
    const [source, setSource] = useState<PanelSource>('recommend')
    const [tag, setTag] = useState<AiPatternTag>(DEFAULT_TAG)

    const t = useTranslations('ai.practiceFolder')
    const input: AiPracticeSourceInput = source === 'tag' ? { source, tag } : { source }
    const practice = useAiPracticeFolder(input)
    const preview = usePreviewAiPracticeFolder()
    const create = useCreateAiPracticeFolder()

    const view = practice.data
    const previewData = preview.data ?? view?.ai ?? null
    const byMd5 = new Map((view?.candidates ?? []).map((candidate) => [candidate.md5, candidate]))
    const items = (previewData?.output.items ?? []).flatMap((item) => {
        const candidate = byMd5.get(item.md5)
        return candidate ? [{ candidate, memo: item.memo }] : []
    })
    const memos = new Map(items.map((item) => [item.candidate.md5, item.memo]))
    const isFull = view ? !view.folders.can_create : false

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            <div className='flex flex-wrap items-center gap-2'>
                <Select
                    value={source}
                    onValueChange={(value) => {
                        if (isPanelSource(value)) setSource(value)
                        preview.reset()
                    }}>
                    <SelectTrigger size='sm' className='w-40' aria-label={t('sourceLabel')}>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {SOURCES.map((value) => (
                            <SelectItem key={value} value={value}>
                                {t(`source.${value}`)}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                {source === 'tag' && (
                    <Select
                        value={tag}
                        onValueChange={(value) => {
                            if (isAiPatternTag(value)) setTag(value)
                            preview.reset()
                        }}>
                        <SelectTrigger size='sm' className='w-44' aria-label={t('tagLabel')}>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {TAGS.map((value) => (
                                <SelectItem key={value} value={value}>
                                    {AI_PATTERN_TAG_LABEL[value]}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                )}
            </div>
            {practice.isPending && <PanelSkeleton label={t('loading')} />}
            {practice.isError && <LoadErrorState title={t('loadError')} isRetrying={practice.isFetching} onRetry={() => void practice.refetch()} />}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>
                        {t('info', {
                            candidates: view.candidates.length,
                            used: formatCount(view.folders.used),
                            limit: formatCount(view.folders.limit),
                            prefix: AI_SCORE_FEATURE_FORM_LIMIT.PRACTICE_NAME_PREFIX,
                        })}
                    </p>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {isFull && (
                        <p role='alert' className='text-destructive text-xs'>
                            {t('full', { limit: view.folders.limit })}
                        </p>
                    )}
                    <AiInterpretSection
                        label={t('aiLabel')}
                        meta={previewData}
                        isPending={preview.isPending}
                        error={preview.error}
                        isBlocked={view.notice !== null || isFull}
                        onInterpret={() => preview.mutate(input)}>
                        {previewData && items.length > 0 && (
                            <div className='flex flex-col gap-2'>
                                <p className='text-sm font-medium'>
                                    {AI_SCORE_FEATURE_FORM_LIMIT.PRACTICE_NAME_PREFIX}
                                    {previewData.output.name_suffix}
                                </p>
                                {previewData.output.description && <p className='text-muted-foreground text-xs'>{previewData.output.description}</p>}
                                <AiChartCandidateList
                                    label={t('previewLabel')}
                                    symbol=''
                                    candidates={items.map((item) => item.candidate)}
                                    renderExtra={(candidate) => memos.get(candidate.md5) && <p className='text-xs'>{memos.get(candidate.md5)}</p>}
                                />
                                <div className='flex flex-wrap items-center gap-2'>
                                    <Button
                                        type='button'
                                        size='sm'
                                        disabled={create.isPending || isFull}
                                        onClick={() =>
                                            create.mutate({
                                                ...input,
                                                name_suffix: previewData.output.name_suffix,
                                                description: previewData.output.description,
                                                items: items.map((item) => ({ md5: item.candidate.md5, memo: item.memo })),
                                            })
                                        }>
                                        {t('create')}
                                    </Button>
                                    {create.data && (
                                        <Button asChild variant='link' size='sm'>
                                            <Link href={`/folders/${create.data.folder_id}`}>{t('openFolder')}</Link>
                                        </Button>
                                    )}
                                </div>
                            </div>
                        )}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}

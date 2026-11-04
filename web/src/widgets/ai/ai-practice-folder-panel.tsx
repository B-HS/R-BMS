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
import { ApiError } from '@shared/lib/fetch'
import { formatCount } from '@shared/lib/format'
import { useAiPracticeFolder, useAiProfile, useAiWeaknessLabel, useCreateAiPracticeFolder, usePreviewAiPracticeFolder } from '@entities/ai.query'
import { AI_PATTERN_TAG_LABEL, AI_SCORE_FEATURE_FORM_LIMIT, isAiPatternTag, type AiPatternTag, type AiPracticeSourceInput } from '@entities/ai.type'

const SOURCES = ['recommend', 'tag', 'weakness'] as const
type PanelSource = (typeof SOURCES)[number]

const isPanelSource = (value: string): value is PanelSource => SOURCES.some((source) => source === value)
const TAGS = Object.keys(AI_PATTERN_TAG_LABEL).filter(isAiPatternTag)
const DEFAULT_TAG = 'trill' satisfies AiPatternTag

const needOf = (error: unknown) => {
    const need = error instanceof ApiError ? error.details?.need : undefined
    return typeof need === 'string' ? need : null
}

/**
 * Feature 6: a practice folder from the next challenge candidates, one pattern tag, or one weakness of the profile (`initialFocusKey`
 * opens it on that weakness, as the profile's links do). The candidates come from the server; the AI only orders them on a press.
 */
export const AiPracticeFolderPanel: FC<{ initialFocusKey?: string | null }> = ({ initialFocusKey = null }) => {
    const [source, setSource] = useState<PanelSource>(initialFocusKey ? 'weakness' : 'recommend')
    const [tag, setTag] = useState<AiPatternTag>(DEFAULT_TAG)
    const [pickedFocusKey, setPickedFocusKey] = useState<string | null>(initialFocusKey)

    const t = useTranslations('ai.practiceFolder')
    const weaknessLabel = useAiWeaknessLabel()
    const profile = useAiProfile(source === 'weakness')
    const focusKeys = (profile.data?.weaknesses ?? []).filter((item) => item.selectable_for_folder).map((item) => item.key)
    const shownFocusKeys = pickedFocusKey && !focusKeys.includes(pickedFocusKey) ? [pickedFocusKey, ...focusKeys] : focusKeys
    const focusKey = pickedFocusKey ?? focusKeys[0] ?? null
    const input: AiPracticeSourceInput = {
        source,
        ...(source === 'tag' ? { tag } : {}),
        ...(source === 'weakness' && focusKey ? { focus_key: focusKey } : {}),
    }
    const isWaitingForKey = source === 'weakness' && focusKey === null
    const practice = useAiPracticeFolder(input, !isWaitingForKey)
    const preview = usePreviewAiPracticeFolder()
    const create = useCreateAiPracticeFolder()

    const errorNeed = needOf(practice.error)
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
                {source === 'weakness' && focusKey !== null && (
                    <Select
                        value={focusKey}
                        onValueChange={(value) => {
                            setPickedFocusKey(value)
                            preview.reset()
                        }}>
                        <SelectTrigger size='sm' className='w-52' aria-label={t('weaknessLabel')}>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {shownFocusKeys.map((value) => (
                                <SelectItem key={value} value={value}>
                                    {weaknessLabel(value)}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                )}
            </div>
            {isWaitingForKey && profile.isPending && <PanelSkeleton label={t('loading')} />}
            {isWaitingForKey && profile.isError && (
                <LoadErrorState title={t('loadError')} isRetrying={profile.isFetching} onRetry={() => void profile.refetch()} />
            )}
            {isWaitingForKey && profile.data && <p className='text-muted-foreground text-xs break-keep'>{t('weaknessEmpty')}</p>}
            {!isWaitingForKey && practice.isPending && <PanelSkeleton label={t('loading')} />}
            {practice.isError && errorNeed && <AiDataNotice need={errorNeed} />}
            {practice.isError && !errorNeed && (
                <LoadErrorState title={t('loadError')} isRetrying={practice.isFetching} onRetry={() => void practice.refetch()} />
            )}
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

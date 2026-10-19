'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
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

const SOURCE_LABEL = { recommend: '다음 도전 후보', tag: '패턴 태그' } as const
type PanelSource = keyof typeof SOURCE_LABEL

const isPanelSource = (value: string): value is PanelSource => value in SOURCE_LABEL
const SOURCES = Object.keys(SOURCE_LABEL).filter(isPanelSource)
const TAGS = Object.keys(AI_PATTERN_TAG_LABEL).filter(isAiPatternTag)
const DEFAULT_TAG = 'trill' satisfies AiPatternTag

export const AiPracticeFolderPanel: FC = () => {
    const [source, setSource] = useState<PanelSource>('recommend')
    const [tag, setTag] = useState<AiPatternTag>(DEFAULT_TAG)

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
        <PanelCard title='AI 연습 폴더' contentClassName='flex flex-col gap-4'>
            <div className='flex flex-wrap items-center gap-2'>
                <Select
                    value={source}
                    onValueChange={(value) => {
                        if (isPanelSource(value)) setSource(value)
                        preview.reset()
                    }}>
                    <SelectTrigger size='sm' className='w-40' aria-label='후보 출처'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {SOURCES.map((value) => (
                            <SelectItem key={value} value={value}>
                                {SOURCE_LABEL[value]}
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
                        <SelectTrigger size='sm' className='w-44' aria-label='연습할 패턴'>
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
            {practice.isPending && <PanelSkeleton label='연습 후보 계산 중' />}
            {practice.isError && (
                <LoadErrorState title='연습 후보를 불러오지 못했습니다' isRetrying={practice.isFetching} onRetry={() => void practice.refetch()} />
            )}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>
                        후보 {formatCount(view.candidates.length)}곡 · 내 폴더 {formatCount(view.folders.used)}/{formatCount(view.folders.limit)}.
                        미리보기는 폴더를 만들지 않고, 아래 만들기를 눌러야 &quot;{AI_SCORE_FEATURE_FORM_LIMIT.PRACTICE_NAME_PREFIX}&quot; 폴더가
                        생깁니다. 게임은 다음 새로 받기에서 MY FOLDERS 에 보입니다.
                    </p>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {isFull && (
                        <p role='alert' className='text-destructive text-xs'>
                            폴더가 {formatCount(view.folders.limit)}개로 가득 찼습니다. 폴더를 지운 뒤 다시 시도하세요.
                        </p>
                    )}
                    <AiInterpretSection
                        label='AI 연습 폴더 미리보기'
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
                                    label='미리보기 곡'
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
                                        이 미리보기로 폴더 만들기
                                    </Button>
                                    {create.data && (
                                        <Button asChild variant='link' size='sm'>
                                            <Link href={`/folders/${create.data.folder_id}`}>만든 폴더 열기</Link>
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

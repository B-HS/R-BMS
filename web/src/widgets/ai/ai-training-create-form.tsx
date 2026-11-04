'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiFailureNotice } from '@features/ai/ai-failure-notice'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { ApiError } from '@shared/lib/fetch'
import { useApiErrorMessage } from '@shared/lib/use-api-error-message'
import { Button } from '@shared/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { ToggleGroup, ToggleGroupItem } from '@shared/ui/toggle-group'
import { useAiProfile, useAiWeaknessLabel, useCreateAiTrainingChart } from '@entities/ai.query'
import {
    AI_TRAINING_AUTO_FOCUS,
    AI_TRAINING_DEFAULT_INTENSITY,
    AI_TRAINING_ERROR_CODE,
    AI_TRAINING_INTENSITIES,
    aiDataNeedOf,
    aiTrainingHref,
    canAiTrainingPickFocus,
    isAiTrainingPositionalKey,
    trainableAiWeaknessesOf,
    type AiTrainingIntensity,
} from '@entities/ai.type'

const isIntensity = (value: string): value is AiTrainingIntensity => AI_TRAINING_INTENSITIES.some((intensity) => intensity === value)

const failureKindOf = (error: unknown) => {
    if (error instanceof ApiError && error.code === AI_TRAINING_ERROR_CODE.LIMIT) return 'limit'
    if (error instanceof ApiError && error.code === AI_TRAINING_ERROR_CODE.FOLDER_UNAVAILABLE) return 'folder'
    return aiDataNeedOf(error) === null ? 'ai' : 'need'
}

/**
 * Makes one training chart (plan 2026-11-04 3.1, 7.3): the weakness to train (the server's own pick, or one of the profile's weaknesses a
 * chart can be laid out for, which a link from the profile preselects) and how hard. One press asks the provider once and can take
 * minutes. The reasons the server refuses (the limit raced by another tab, no room for the folder, too few plays, the AI call) are worded here.
 */
export const AiTrainingCreateForm: FC<{ initialFocusKey: string | null }> = ({ initialFocusKey }) => {
    const [pickedFocusKey, setPickedFocusKey] = useState<string | null>(initialFocusKey)
    const [intensity, setIntensity] = useState<AiTrainingIntensity>(AI_TRAINING_DEFAULT_INTENSITY)

    const t = useTranslations('pages.ai.training')
    const weaknessLabel = useAiWeaknessLabel()
    const apiErrorMessage = useApiErrorMessage()
    const profile = useAiProfile()
    const create = useCreateAiTrainingChart()

    const weaknesses = profile.data?.weaknesses ?? []
    const trainableKeys = trainableAiWeaknessesOf(weaknesses).map((item) => item.key)
    const focusOptions = [...(canAiTrainingPickFocus(weaknesses) ? [AI_TRAINING_AUTO_FOCUS] : []), ...trainableKeys]
    const focusKey = pickedFocusKey !== null && focusOptions.includes(pickedFocusKey) ? pickedFocusKey : (focusOptions[0] ?? null)
    const isLinkedKeyMissing = profile.data !== undefined && initialFocusKey !== null && !focusOptions.includes(initialFocusKey)
    const failureKind = failureKindOf(create.error)

    return (
        <form
            aria-label={t('form.label')}
            className='flex flex-col gap-3'
            noValidate
            onSubmit={(event) => {
                event.preventDefault()
                if (focusKey !== null && !create.isPending) create.mutate({ focus_key: focusKey, intensity })
            }}>
            <p className='text-muted-foreground text-xs break-keep'>{t('form.description')}</p>
            {profile.isPending && <PanelSkeleton label={t('form.loading')} />}
            {profile.isError && <LoadErrorState title={t('form.loadError')} isRetrying={profile.isFetching} onRetry={() => void profile.refetch()} />}
            {profile.data && focusOptions.length === 0 && <AiDataNotice need={profile.data.notice?.need ?? 'weakness_profile'} />}
            {isLinkedKeyMissing && <p className='text-muted-foreground text-xs break-keep'>{t('form.linkedKeyMissing')}</p>}
            {focusKey !== null && (
                <div className='flex flex-wrap items-center gap-2'>
                    <Select value={focusKey} onValueChange={setPickedFocusKey}>
                        <SelectTrigger size='sm' className='w-56' aria-label={t('form.focusLabel')}>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {focusOptions.map((value) => (
                                <SelectItem key={value} value={value}>
                                    {value === AI_TRAINING_AUTO_FOCUS ? t('form.autoFocus') : weaknessLabel(value)}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                    <ToggleGroup
                        type='single'
                        variant='outline'
                        size='sm'
                        spacing={0}
                        aria-label={t('form.intensityLabel')}
                        value={intensity}
                        onValueChange={(value) => isIntensity(value) && setIntensity(value)}>
                        {AI_TRAINING_INTENSITIES.map((value) => (
                            <ToggleGroupItem key={value} value={value} className='px-3 text-xs'>
                                {t(`form.intensity.${value}.name`)}
                            </ToggleGroupItem>
                        ))}
                    </ToggleGroup>
                    <Button type='submit' size='sm' disabled={create.isPending}>
                        {create.isPending ? t('form.submitting') : t('form.submit')}
                    </Button>
                </div>
            )}
            {focusKey !== null && <p className='text-muted-foreground text-xs break-keep'>{t(`form.intensity.${intensity}.description`)}</p>}
            {focusKey !== null && isAiTrainingPositionalKey(focusKey) && <p className='text-xs break-keep'>{t('optionNotice')}</p>}
            {create.isPending && (
                <p role='status' className='text-muted-foreground text-xs break-keep'>
                    {t('form.pending')}
                </p>
            )}
            {create.isError && failureKind === 'limit' && (
                <p role='alert' className='text-destructive text-xs break-keep'>
                    {t('limitRaced')}
                </p>
            )}
            {create.isError && failureKind === 'folder' && (
                <p role='alert' className='text-destructive text-xs break-keep'>
                    {apiErrorMessage(create.error)}
                </p>
            )}
            {create.isError && failureKind === 'need' && <AiDataNotice need={aiDataNeedOf(create.error) ?? ''} />}
            {create.isError && failureKind === 'ai' && <AiFailureNotice error={create.error} />}
            {create.data && (
                <p role='status' className='flex flex-wrap items-center gap-2 text-xs'>
                    {t('form.created', { title: create.data.title })}
                    <Button asChild variant='outline' size='xs'>
                        <Link href={aiTrainingHref(create.data.id)}>{t('form.openDetail')}</Link>
                    </Button>
                </p>
            )}
        </form>
    )
}

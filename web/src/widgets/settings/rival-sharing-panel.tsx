'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Switch } from '@shared/ui/switch'
import { useAiShareSetting, useSaveAiShareSetting } from '@entities/ai.query'

const SWITCH_ID = 'rival-segment-sharing'

/**
 * Whether players who are rivals of mine and keep me as a rival too may compare their play with mine segment by segment (ADR Q4, W5
 * decision: a one-sided registration gives nothing). Off until turned on: they send only my plays' segment totals (EX, BP and object
 * counts per segment, never notes, timing or lanes) to their own AI.
 */
export const RivalSharingPanel: FC = () => {
    const t = useTranslations('settings.rivalSharingPanel')
    const setting = useAiShareSetting()
    const save = useSaveAiShareSetting()

    const isOn = setting.data?.rival_segments ?? false

    const handleChange = (next: boolean) => save.mutate(next, { onSuccess: () => toast.success(t(next ? 'enabled' : 'disabled')) })

    return (
        <PanelCard title={t('title')}>
            {setting.isPending && <PanelSkeleton label={t('loading')} />}
            {setting.isError && <LoadErrorState title={t('loadError')} isRetrying={setting.isFetching} onRetry={() => void setting.refetch()} />}
            {setting.data && (
                <Field orientation='horizontal'>
                    <Switch id={SWITCH_ID} checked={isOn} disabled={save.isPending} onCheckedChange={handleChange} />
                    <div className='flex flex-col gap-1'>
                        <FieldLabel htmlFor={SWITCH_ID}>{t('label')}</FieldLabel>
                        <FieldDescription className='break-keep'>{t('description')}</FieldDescription>
                    </div>
                </Field>
            )}
        </PanelCard>
    )
}

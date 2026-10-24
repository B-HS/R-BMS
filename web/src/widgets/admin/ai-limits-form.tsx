'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { AiLimitNumberField } from '@features/ai/ai-limit-number-field'
import { parseLimitInput } from '@features/ai/parse-limit-input'
import { Button } from '@shared/ui/button'
import { formatDateTime } from '@shared/lib/format'
import { useSaveAiOperatorLimits } from '@entities/ai.query'
import { AI_FORM_LIMIT, type AiOperatorLimits } from '@entities/ai.type'

export const AiLimitsForm: FC<{ limits: AiOperatorLimits }> = ({ limits }) => {
    const [dailyCallsDraft, setDailyCallsDraft] = useState<string | null>(null)
    const [concurrentRunsDraft, setConcurrentRunsDraft] = useState<string | null>(null)
    const [monthlyTokenLimitDraft, setMonthlyTokenLimitDraft] = useState<string | null>(null)

    const t = useTranslations('browse.admin.aiLimits')
    const save = useSaveAiOperatorLimits()

    const dailyCallsText = dailyCallsDraft ?? String(limits.daily_calls)
    const concurrentRunsText = concurrentRunsDraft ?? String(limits.concurrent_runs)
    const monthlyTokenLimitText = monthlyTokenLimitDraft ?? limits.default_monthly_token_limit?.toString() ?? ''
    const dailyCalls = parseLimitInput(dailyCallsText, { min: 1, max: AI_FORM_LIMIT.DAILY_CALLS_MAX })
    const concurrentRuns = parseLimitInput(concurrentRunsText, { min: 1, max: AI_FORM_LIMIT.CONCURRENT_RUNS_MAX })
    const isMonthlyTokenLimitBlank = monthlyTokenLimitText.trim() === ''
    const monthlyTokenLimit = isMonthlyTokenLimitBlank
        ? null
        : parseLimitInput(monthlyTokenLimitText, { min: 1, max: AI_FORM_LIMIT.MONTHLY_TOKEN_LIMIT_MAX })
    const hasMonthlyTokenLimitError = !isMonthlyTokenLimitBlank && monthlyTokenLimit === null
    const isValid = dailyCalls !== null && concurrentRuns !== null && !hasMonthlyTokenLimitError
    const isDirty =
        dailyCalls !== limits.daily_calls || concurrentRuns !== limits.concurrent_runs || monthlyTokenLimit !== limits.default_monthly_token_limit

    const handleSubmit = () => {
        if (dailyCalls === null || concurrentRuns === null || hasMonthlyTokenLimitError) return
        save.mutate(
            { daily_calls: dailyCalls, concurrent_runs: concurrentRuns, default_monthly_token_limit: monthlyTokenLimit },
            {
                onSuccess: () => {
                    setDailyCallsDraft(null)
                    setConcurrentRunsDraft(null)
                    setMonthlyTokenLimitDraft(null)
                    toast.success(t('saved'))
                },
            },
        )
    }

    return (
        <form
            className='flex max-w-xl flex-col gap-3'
            noValidate
            onSubmit={(event) => {
                event.preventDefault()
                handleSubmit()
            }}>
            <AiLimitNumberField
                id='ai-limit-daily-calls'
                label={t('dailyCalls')}
                value={dailyCallsText}
                onValueChange={setDailyCallsDraft}
                description={t('dailyCallsDescription')}
                error={dailyCalls === null ? t('rangeError', { max: AI_FORM_LIMIT.DAILY_CALLS_MAX }) : undefined}
            />
            <AiLimitNumberField
                id='ai-limit-concurrent-runs'
                label={t('concurrentRuns')}
                value={concurrentRunsText}
                onValueChange={setConcurrentRunsDraft}
                description={t('concurrentRunsDescription')}
                error={concurrentRuns === null ? t('rangeError', { max: AI_FORM_LIMIT.CONCURRENT_RUNS_MAX }) : undefined}
            />
            <AiLimitNumberField
                id='ai-limit-monthly-token-limit'
                label={t('monthlyTokenLimit')}
                value={monthlyTokenLimitText}
                onValueChange={setMonthlyTokenLimitDraft}
                placeholder={t('monthlyTokenLimitPlaceholder')}
                description={t('monthlyTokenLimitDescription')}
                error={hasMonthlyTokenLimitError ? t('rangeError', { max: AI_FORM_LIMIT.MONTHLY_TOKEN_LIMIT_MAX }) : undefined}
            />
            <div className='flex flex-wrap items-center gap-3'>
                <Button type='submit' size='sm' disabled={!isValid || !isDirty || save.isPending}>
                    {t('save')}
                </Button>
                <span className='text-muted-foreground text-xs'>
                    {limits.updated_at === null ? t('neverChanged') : t('lastChanged', { time: formatDateTime(limits.updated_at) })}
                </span>
            </div>
        </form>
    )
}

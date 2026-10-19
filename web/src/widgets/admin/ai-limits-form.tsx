'use client'

import { useState, type FC } from 'react'
import { toast } from 'sonner'
import { AiLimitNumberField } from '@features/ai/ai-limit-number-field'
import { parseLimitInput } from '@features/ai/parse-limit-input'
import { Button } from '@shared/ui/button'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { useSaveAiOperatorLimits } from '@entities/ai.query'
import { AI_FORM_LIMIT, type AiOperatorLimits } from '@entities/ai.type'

const toRangeError = (max: number) => `1 이상 ${formatCount(max)} 이하의 정수를 입력하세요.`

export const AiLimitsForm: FC<{ limits: AiOperatorLimits }> = ({ limits }) => {
    const [dailyCallsDraft, setDailyCallsDraft] = useState<string | null>(null)
    const [concurrentRunsDraft, setConcurrentRunsDraft] = useState<string | null>(null)
    const [monthlyTokenLimitDraft, setMonthlyTokenLimitDraft] = useState<string | null>(null)

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
                    toast.success('AI 사용 한도를 저장했습니다.')
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
                label='사용자당 하루 호출 수'
                value={dailyCallsText}
                onValueChange={setDailyCallsDraft}
                description='사용자 한 명이 하루(UTC)에 공급자를 부를 수 있는 횟수입니다.'
                error={dailyCalls === null ? toRangeError(AI_FORM_LIMIT.DAILY_CALLS_MAX) : undefined}
            />
            <AiLimitNumberField
                id='ai-limit-concurrent-runs'
                label='사용자당 동시 실행 수'
                value={concurrentRunsText}
                onValueChange={setConcurrentRunsDraft}
                description='사용자 한 명이 동시에 진행할 수 있는 AI 호출 수입니다.'
                error={concurrentRuns === null ? toRangeError(AI_FORM_LIMIT.CONCURRENT_RUNS_MAX) : undefined}
            />
            <AiLimitNumberField
                id='ai-limit-monthly-token-limit'
                label='기본 월 토큰 상한'
                value={monthlyTokenLimitText}
                onValueChange={setMonthlyTokenLimitDraft}
                placeholder='비워 두면 제한 없음'
                description='사용자가 자기 월 상한을 정하지 않았을 때 쓰는 값입니다. 입력과 출력 토큰의 합이며, 비워 두면 제한하지 않습니다.'
                error={hasMonthlyTokenLimitError ? toRangeError(AI_FORM_LIMIT.MONTHLY_TOKEN_LIMIT_MAX) : undefined}
            />
            <div className='flex flex-wrap items-center gap-3'>
                <Button type='submit' size='sm' disabled={!isValid || !isDirty || save.isPending}>
                    저장
                </Button>
                <span className='text-muted-foreground text-xs'>
                    {limits.updated_at === null ? '아직 바꾼 적이 없어 기본값입니다.' : `마지막 변경 ${formatDateTime(limits.updated_at)}`}
                </span>
            </div>
        </form>
    )
}

'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiFailureNotice } from '@features/ai/ai-failure-notice'
import { AiTokenUsageLine } from '@features/ai/ai-token-usage-line'
import { PanelCard } from '@features/shell/panel-card'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { useAskAiRecordQuery, useRunAiRecordQuery } from '@entities/ai.query'
import { AI_PROVIDER_LABEL, AI_RECORD_QUERY_FORM_LIMIT, type AiRecordQueryAnswer } from '@entities/ai.type'
import { RecordQueryFilterEditor } from '@widgets/ai/record-query-filter-editor'
import { RecordQueryResults } from '@widgets/ai/record-query-results'

export const RecordQueryPanel: FC = () => {
    const [question, setQuestion] = useState('')
    const [answer, setAnswer] = useState<AiRecordQueryAnswer | null>(null)
    const [planVersion, setPlanVersion] = useState(0)

    const t = useTranslations('ai.recordQuery')
    const ask = useAskAiRecordQuery()
    const run = useRunAiRecordQuery()

    const trimmedQuestion = question.trim()
    const isBusy = ask.isPending || run.isPending

    const handleAsk = () =>
        ask.mutate(trimmedQuestion, {
            onSuccess: (data) => {
                setAnswer(data)
                setPlanVersion((version) => version + 1)
            },
        })

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            <form
                className='flex max-w-xl flex-col gap-2'
                noValidate
                onSubmit={(event) => {
                    event.preventDefault()
                    handleAsk()
                }}>
                <Field>
                    <FieldLabel htmlFor='ai-record-question'>{t('questionLabel')}</FieldLabel>
                    <Input
                        id='ai-record-question'
                        value={question}
                        maxLength={AI_RECORD_QUERY_FORM_LIMIT.QUESTION_MAX_CHARS}
                        placeholder={t('questionPlaceholder')}
                        autoComplete='off'
                        aria-describedby='ai-record-question-description'
                        onChange={(event) => setQuestion(event.target.value)}
                    />
                    <FieldDescription id='ai-record-question-description'>
                        {t('questionDescription', { max: AI_RECORD_QUERY_FORM_LIMIT.QUESTION_MAX_CHARS })}
                    </FieldDescription>
                </Field>
                <div>
                    <Button type='submit' size='sm' disabled={trimmedQuestion === '' || isBusy}>
                        {ask.isPending ? t('interpreting') : t('makeConditions')}
                    </Button>
                </div>
            </form>
            {ask.isError && <AiFailureNotice error={ask.error} />}
            {answer && (
                <section aria-label={t('interpretationLabel')} className='flex flex-col gap-2'>
                    {answer.plan.explanation && <p className='text-sm'>{answer.plan.explanation}</p>}
                    {answer.ai && (
                        <div className='flex flex-wrap items-center gap-2'>
                            <span className='text-muted-foreground text-xs'>
                                {AI_PROVIDER_LABEL[answer.ai.provider]} · <code className='font-mono'>{answer.ai.model}</code>
                            </span>
                            {answer.ai.cached && <Badge variant='outline'>{t('cachedBadge')}</Badge>}
                            <AiTokenUsageLine {...answer.ai.usage} />
                        </div>
                    )}
                    {!answer.ai && <p className='text-muted-foreground text-xs'>{t('editedRun')}</p>}
                    {answer.issues.length > 0 && (
                        <p role='alert' className='text-destructive text-xs'>
                            {t('issuesAlert')}
                        </p>
                    )}
                </section>
            )}
            {answer && (
                <RecordQueryFilterEditor
                    key={planVersion}
                    initialPlan={answer.plan}
                    issues={answer.issues}
                    isRunning={isBusy}
                    onRun={(plan) => run.mutate(plan, { onSuccess: setAnswer })}
                />
            )}
            {answer?.result && <RecordQueryResults result={answer.result} />}
        </PanelCard>
    )
}

'use client'

import { useState, type FC } from 'react'
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
        <PanelCard title='기록에 묻기' contentClassName='flex flex-col gap-4'>
            <form
                className='flex max-w-xl flex-col gap-2'
                noValidate
                onSubmit={(event) => {
                    event.preventDefault()
                    handleAsk()
                }}>
                <Field>
                    <FieldLabel htmlFor='ai-record-question'>질문</FieldLabel>
                    <Input
                        id='ai-record-question'
                        value={question}
                        maxLength={AI_RECORD_QUERY_FORM_LIMIT.QUESTION_MAX_CHARS}
                        placeholder='예: 지난달 ★12 에서 하드 못 깬 곡'
                        autoComplete='off'
                        aria-describedby='ai-record-question-description'
                        onChange={(event) => setQuestion(event.target.value)}
                    />
                    <FieldDescription id='ai-record-question-description'>
                        AI 는 질문을 검색 조건으로만 바꿉니다. 조건은 서버가 검사한 뒤 내 기록에서만 찾고, 질문 한 번에 AI 호출은 한 번입니다(같은 날
                        같은 질문은 저장된 해석을 씁니다). 최대 {AI_RECORD_QUERY_FORM_LIMIT.QUESTION_MAX_CHARS}자.
                    </FieldDescription>
                </Field>
                <div>
                    <Button type='submit' size='sm' disabled={trimmedQuestion === '' || isBusy}>
                        {ask.isPending ? '해석하는 중' : 'AI 로 조건 만들기'}
                    </Button>
                </div>
            </form>
            {ask.isError && <AiFailureNotice error={ask.error} />}
            {answer && (
                <section aria-label='해석' className='flex flex-col gap-2'>
                    {answer.plan.explanation && <p className='text-sm'>{answer.plan.explanation}</p>}
                    {answer.ai && (
                        <div className='flex flex-wrap items-center gap-2'>
                            <span className='text-muted-foreground text-xs'>
                                {AI_PROVIDER_LABEL[answer.ai.provider]} · <code className='font-mono'>{answer.ai.model}</code>
                            </span>
                            {answer.ai.cached && <Badge variant='outline'>저장된 해석</Badge>}
                            <AiTokenUsageLine {...answer.ai.usage} />
                        </div>
                    )}
                    {!answer.ai && <p className='text-muted-foreground text-xs'>직접 고친 조건으로 실행했습니다(AI 호출 없음).</p>}
                    {answer.issues.length > 0 && (
                        <p role='alert' className='text-destructive text-xs'>
                            검사를 통과하지 못한 조건이 있어 실행하지 않았습니다. 표시된 조건을 고친 뒤 다시 실행하세요.
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

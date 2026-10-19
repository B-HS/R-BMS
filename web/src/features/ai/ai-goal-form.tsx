'use client'

import { useState, type FC } from 'react'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { CLEAR_LAMPS, CLEAR_LAMP_LABEL, MIN_CLEAR_LAMP, type ClearLamp } from '@shared/constants/clear-lamp'
import { AI_GOAL_KIND_LABEL, AI_SCORE_FEATURE_FORM_LIMIT, type AiGoalKind, type AiGoalParams } from '@entities/ai.type'

const GOAL_LAMPS = CLEAR_LAMPS.slice(CLEAR_LAMPS.indexOf(MIN_CLEAR_LAMP))
const DEFAULT_LAMP = 'Hard' satisfies ClearLamp
const DEFAULT_COUNT = '5'
const DEFAULT_EX_RATE = '90'
const MD5_PATTERN = /^[0-9a-f]{32}$/

const isGoalKind = (value: string): value is AiGoalKind => value in AI_GOAL_KIND_LABEL
const isGoalLamp = (value: string): value is ClearLamp => GOAL_LAMPS.some((lamp) => lamp === value)
const GOAL_KINDS = Object.keys(AI_GOAL_KIND_LABEL).filter(isGoalKind)

type AiGoalFormProps = {
    tableId: string
    symbol: string
    levels: readonly string[]
    isSubmitting: boolean
    onSubmit: (params: AiGoalParams) => void
}

/** Builds one goal of the fixed kinds (plan 5.7, no free sentence). The server checks it again against the table. */
export const AiGoalForm: FC<AiGoalFormProps> = ({ tableId, symbol, levels, isSubmitting, onSubmit }) => {
    const [kind, setKind] = useState<AiGoalKind>('lamp_count_at_level')
    const [level, setLevel] = useState(levels[0] ?? '')
    const [lamp, setLamp] = useState<ClearLamp>(DEFAULT_LAMP)
    const [countText, setCountText] = useState(DEFAULT_COUNT)
    const [md5, setMd5] = useState('')
    const [exRateText, setExRateText] = useState(DEFAULT_EX_RATE)

    const count = Number.parseInt(countText, 10)
    const exRate = Number.parseFloat(exRateText)
    const trimmedMd5 = md5.trim().toLowerCase()
    const isValid =
        (kind === 'clear_all_level' && level !== '') ||
        (kind === 'lamp_count_at_level' && level !== '' && count >= 1 && count <= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_COUNT_MAX) ||
        (kind === 'ex_rate_on_chart' && MD5_PATTERN.test(trimmedMd5) && exRate > 0 && exRate <= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_EX_RATE_MAX)

    const handleSubmit = () => {
        if (kind === 'clear_all_level') onSubmit({ kind, table: tableId, level })
        if (kind === 'lamp_count_at_level') onSubmit({ kind, table: tableId, level, lamp, count })
        if (kind === 'ex_rate_on_chart') onSubmit({ kind, table: tableId, md5: trimmedMd5, ex_rate: exRate })
    }

    return (
        <form
            aria-label='새 목표'
            className='flex flex-wrap items-center gap-2'
            noValidate
            onSubmit={(event) => {
                event.preventDefault()
                if (isValid) handleSubmit()
            }}>
            <Select value={kind} onValueChange={(value) => isGoalKind(value) && setKind(value)}>
                <SelectTrigger size='sm' className='w-44' aria-label='목표 종류'>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    {GOAL_KINDS.map((value) => (
                        <SelectItem key={value} value={value}>
                            {AI_GOAL_KIND_LABEL[value]}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            {kind !== 'ex_rate_on_chart' && (
                <Select value={level} onValueChange={setLevel}>
                    <SelectTrigger size='sm' className='w-24' aria-label='레벨'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {levels.map((value) => (
                            <SelectItem key={value} value={value}>
                                {symbol}
                                {value}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            )}
            {kind === 'lamp_count_at_level' && (
                <>
                    <Select value={lamp} onValueChange={(value) => isGoalLamp(value) && setLamp(value)}>
                        <SelectTrigger size='sm' className='w-32' aria-label='램프'>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {GOAL_LAMPS.map((value) => (
                                <SelectItem key={value} value={value}>
                                    {CLEAR_LAMP_LABEL[value]} 이상
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                    <Input
                        aria-label='곡 수'
                        inputMode='numeric'
                        className='h-7 w-20'
                        value={countText}
                        onChange={(event) => setCountText(event.target.value)}
                    />
                </>
            )}
            {kind === 'ex_rate_on_chart' && (
                <>
                    <Input
                        aria-label='차트 md5'
                        className='h-7 w-72 max-w-full font-mono'
                        value={md5}
                        placeholder='표 곡의 md5'
                        onChange={(event) => setMd5(event.target.value)}
                    />
                    <Input
                        aria-label='목표 EX 비율(%)'
                        inputMode='decimal'
                        className='h-7 w-20'
                        value={exRateText}
                        onChange={(event) => setExRateText(event.target.value)}
                    />
                </>
            )}
            <Button type='submit' size='sm' disabled={!isValid || isSubmitting}>
                목표 추가
            </Button>
        </form>
    )
}

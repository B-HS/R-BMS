'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { CLEAR_LAMPS, CLEAR_LAMP_LABEL, MIN_CLEAR_LAMP, type ClearLamp } from '@shared/constants/clear-lamp'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useAiWeaknessLabel } from '@entities/ai.query'
import { AI_GOAL_KINDS, AI_SCORE_FEATURE_FORM_LIMIT, type AiGoalInput, type AiGoalKind } from '@entities/ai.type'

const GOAL_LAMPS = CLEAR_LAMPS.slice(CLEAR_LAMPS.indexOf(MIN_CLEAR_LAMP))
const DEFAULT_LAMP = 'Hard' satisfies ClearLamp
const DEFAULT_COUNT = '5'
const DEFAULT_EX_RATE = '90'
const DEFAULT_TARGET_RESIDUAL_PERCENT = '-2'
const DEFAULT_TARGET_NPS = '12'
const MD5_PATTERN = /^[0-9a-f]{32}$/
const LEVEL_KINDS = ['lamp_count_at_level', 'clear_all_level'] as const satisfies readonly AiGoalKind[]

const isGoalKind = (value: string): value is AiGoalKind => AI_GOAL_KINDS.some((kind) => kind === value)
const isGoalLamp = (value: string): value is ClearLamp => GOAL_LAMPS.some((lamp) => lamp === value)

type AiGoalFormProps = {
    tableId: string
    symbol: string
    levels: readonly string[]
    weaknessKeys: readonly string[]
    isSubmitting: boolean
    onSubmit: (params: AiGoalInput) => void
}

/**
 * Builds one goal of the fixed kinds (plan 5.7, no free sentence). A weakness goal picks one of the profile's weaknesses whose value is
 * an EX ratio difference and a target in percentage points; a density goal a target density limit in notes per second. The server
 * checks every goal again against the table and the profile.
 */
export const AiGoalForm: FC<AiGoalFormProps> = ({ tableId, symbol, levels, weaknessKeys, isSubmitting, onSubmit }) => {
    const t = useTranslations('pages.ai.goalForm')
    const [kind, setKind] = useState<AiGoalKind>('lamp_count_at_level')
    const [level, setLevel] = useState(levels[0] ?? '')
    const [lamp, setLamp] = useState<ClearLamp>(DEFAULT_LAMP)
    const [countText, setCountText] = useState(DEFAULT_COUNT)
    const [md5, setMd5] = useState('')
    const [exRateText, setExRateText] = useState(DEFAULT_EX_RATE)
    const [pickedKey, setPickedKey] = useState<string | null>(null)
    const [residualText, setResidualText] = useState(DEFAULT_TARGET_RESIDUAL_PERCENT)
    const [npsText, setNpsText] = useState(DEFAULT_TARGET_NPS)

    const weaknessLabel = useAiWeaknessLabel()
    const count = Number.parseInt(countText, 10)
    const exRate = Number.parseFloat(exRateText)
    const trimmedMd5 = md5.trim().toLowerCase()
    const weaknessKey = weaknessKeys.find((key) => key === pickedKey) ?? weaknessKeys[0] ?? null
    const targetResidual = Number.parseFloat(residualText) / PERCENT_SCALE
    const targetNps = Number.parseFloat(npsText)
    const isValid =
        (kind === 'clear_all_level' && level !== '') ||
        (kind === 'lamp_count_at_level' && level !== '' && count >= 1 && count <= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_COUNT_MAX) ||
        (kind === 'ex_rate_on_chart' && MD5_PATTERN.test(trimmedMd5) && exRate > 0 && exRate <= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_EX_RATE_MAX) ||
        (kind === 'weakness_residual' &&
            weaknessKey !== null &&
            targetResidual >= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_RESIDUAL_MIN &&
            targetResidual <= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_RESIDUAL_MAX) ||
        (kind === 'density_limit' &&
            targetNps >= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_NPS_MIN &&
            targetNps <= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_NPS_MAX)

    const handleSubmit = () => {
        if (kind === 'clear_all_level') onSubmit({ kind, table: tableId, level })
        if (kind === 'lamp_count_at_level') onSubmit({ kind, table: tableId, level, lamp, count })
        if (kind === 'ex_rate_on_chart') onSubmit({ kind, table: tableId, md5: trimmedMd5, ex_rate: exRate })
        if (kind === 'weakness_residual' && weaknessKey !== null)
            onSubmit({ kind, table: tableId, key: weaknessKey, target_residual: targetResidual })
        if (kind === 'density_limit') onSubmit({ kind, table: tableId, target_nps: targetNps })
    }

    return (
        <form
            aria-label={t('ariaLabel')}
            className='flex flex-wrap items-center gap-2'
            noValidate
            onSubmit={(event) => {
                event.preventDefault()
                if (isValid) handleSubmit()
            }}>
            <Select value={kind} onValueChange={(value) => isGoalKind(value) && setKind(value)}>
                <SelectTrigger size='sm' className='w-44' aria-label={t('kind')}>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    {AI_GOAL_KINDS.map((value) => (
                        <SelectItem key={value} value={value}>
                            {t(`goalKind.${value}`)}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            {LEVEL_KINDS.some((levelKind) => levelKind === kind) && (
                <Select value={level} onValueChange={setLevel}>
                    <SelectTrigger size='sm' className='w-24' aria-label={t('level')}>
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
                        <SelectTrigger size='sm' className='w-32' aria-label={t('lamp')}>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {GOAL_LAMPS.map((value) => (
                                <SelectItem key={value} value={value}>
                                    {t('lampOrAbove', { lamp: CLEAR_LAMP_LABEL[value] })}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                    <Input
                        aria-label={t('count')}
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
                        aria-label={t('md5')}
                        className='h-7 w-72 max-w-full font-mono'
                        value={md5}
                        placeholder={t('md5Placeholder')}
                        onChange={(event) => setMd5(event.target.value)}
                    />
                    <Input
                        aria-label={t('exRate')}
                        inputMode='decimal'
                        className='h-7 w-20'
                        value={exRateText}
                        onChange={(event) => setExRateText(event.target.value)}
                    />
                </>
            )}
            {kind === 'weakness_residual' && weaknessKey === null && <p className='text-muted-foreground text-xs'>{t('noWeaknessKeys')}</p>}
            {kind === 'weakness_residual' && weaknessKey !== null && (
                <>
                    <Select value={weaknessKey} onValueChange={setPickedKey}>
                        <SelectTrigger size='sm' className='w-48' aria-label={t('weaknessKey')}>
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {weaknessKeys.map((key) => (
                                <SelectItem key={key} value={key}>
                                    {weaknessLabel(key)}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                    <Input
                        aria-label={t('targetResidual', {
                            min: AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_RESIDUAL_MIN * PERCENT_SCALE,
                            max: AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_RESIDUAL_MAX * PERCENT_SCALE,
                        })}
                        inputMode='decimal'
                        className='h-7 w-20'
                        value={residualText}
                        onChange={(event) => setResidualText(event.target.value)}
                    />
                </>
            )}
            {kind === 'density_limit' && (
                <Input
                    aria-label={t('targetNps', {
                        min: AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_NPS_MIN,
                        max: AI_SCORE_FEATURE_FORM_LIMIT.GOAL_TARGET_NPS_MAX,
                    })}
                    inputMode='decimal'
                    className='h-7 w-20'
                    value={npsText}
                    onChange={(event) => setNpsText(event.target.value)}
                />
            )}
            <Button type='submit' size='sm' disabled={!isValid || isSubmitting}>
                {t('submit')}
            </Button>
        </form>
    )
}

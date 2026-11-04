import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { useAiTrainingFocusLabel } from '@entities/ai.query'
import { isAiTrainingCorrectionField, isAiTrainingPatternKind, type AiTrainingChartDetail } from '@entities/ai.type'

type AiTrainingCorrection = AiTrainingChartDetail['corrections'][number]

/**
 * The values the server changed from the AI's plan before laying the chart out (plan 2026-11-04 3.5, 7.3): a section the plan named
 * that the player's profile does not allow is dropped, a pattern kind that cannot train its weakness is swapped, and every number is
 * held to its range. Each line says which section, which value, and what it was and became.
 */
export const AiTrainingCorrectionList: FC<{ corrections: readonly AiTrainingCorrection[] }> = ({ corrections }) => {
    const t = useTranslations('pages.ai.training.corrections')
    const tKind = useTranslations('pages.ai.training.kind')
    const focusLabel = useAiTrainingFocusLabel()
    const valueText = (field: string, value: string | number | null) => {
        if (value === null) return t('dropped')
        if (field === 'focus_key') return focusLabel(String(value))
        if (field === 'kind' && typeof value === 'string' && isAiTrainingPatternKind(value)) return tKind(value)
        return String(value)
    }

    return (
        <ul aria-label={t('label')} className='flex flex-col gap-px'>
            {corrections.map((correction) => (
                <li
                    key={`${correction.section ?? 'chart'}:${correction.field}`}
                    className='border-border border px-2 py-1 text-xs tabular-nums break-keep'>
                    {correction.section === null ? t('chart') : t('section', { index: correction.section + 1 })}{' '}
                    {isAiTrainingCorrectionField(correction.field) ? t(`field.${correction.field}`) : correction.field}:{' '}
                    {valueText(correction.field, correction.from)} → {valueText(correction.field, correction.to)}
                </li>
            ))}
        </ul>
    )
}

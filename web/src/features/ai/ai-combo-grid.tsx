import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { cn } from '@shared/lib/cn'
import { AI_COMBO_TAGS, aiComboKeyOf, formatAiWeaknessValue, type AiWeaknessProfileCell } from '@entities/ai.type'

const RESIDUAL_FULL_SCALE = 0.1
const MAX_MIX_PERCENT = 85
const MIX_DECIMALS = 0

/**
 * The tag pairs of the profile as the upper triangle of a 15 × 15 grid: the deeper a cell, the more EX that pair loses against the
 * play's own level. Only losses are shaded; a pair below its sample conditions stays an empty cell. CSS grid colours, no animation.
 */
export const AiComboGrid: FC<{ cells: readonly AiWeaknessProfileCell[] }> = ({ cells }) => {
    const t = useTranslations('pages.ai')
    const byKey = new Map(cells.map((cell) => [cell.key, cell]))
    const tagName = (tag: (typeof AI_COMBO_TAGS)[number]) => t(`patternTag.${tag}`)

    return (
        <div className='overflow-x-auto'>
            <table className='text-2xs border-separate border-spacing-px tabular-nums'>
                <caption className='sr-only'>{t('profile.comboCaption')}</caption>
                <thead>
                    <tr>
                        <td />
                        {AI_COMBO_TAGS.map((tag, index) => (
                            <th key={tag} scope='col' className='text-muted-foreground size-5 font-normal'>
                                <abbr title={tagName(tag)} className='no-underline'>
                                    {index + 1}
                                </abbr>
                            </th>
                        ))}
                    </tr>
                </thead>
                <tbody>
                    {AI_COMBO_TAGS.map((first, row) => (
                        <tr key={first}>
                            <th scope='row' className='pr-2 text-left font-normal whitespace-nowrap'>
                                {row + 1}. {tagName(first)}
                            </th>
                            {AI_COMBO_TAGS.map((second, column) => {
                                if (column <= row) return <td key={second} aria-hidden className='size-5' />
                                const cell = byKey.get(aiComboKeyOf(first, second))
                                const isSampled = cell !== undefined && cell.evidence !== 'insufficient' && cell.value !== null
                                const loss = isSampled && cell.value !== null && cell.value < 0 ? Math.min(1, -cell.value / RESIDUAL_FULL_SCALE) : 0
                                return (
                                    <td
                                        key={second}
                                        title={isSampled ? `${tagName(first)} + ${tagName(second)}` : undefined}
                                        className={cn(
                                            'border-border size-5 border',
                                            isSampled && cell.leaning === 'weakness' && 'border-chart-1 border-2',
                                        )}
                                        style={
                                            loss > 0
                                                ? {
                                                      background: `color-mix(in oklch, var(--chart-1) ${(loss * MAX_MIX_PERCENT).toFixed(MIX_DECIMALS)}%, transparent)`,
                                                  }
                                                : undefined
                                        }>
                                        {isSampled && (
                                            <span className='sr-only'>
                                                {t('profile.comboCell', {
                                                    first: tagName(first),
                                                    second: tagName(second),
                                                    value: formatAiWeaknessValue(cell.key, cell.unit, cell.value),
                                                    evidence: t(`evidence.${cell.evidence}`),
                                                })}
                                            </span>
                                        )}
                                    </td>
                                )
                            })}
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    )
}

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { LampLabel } from '@features/lamp/lamp-label'
import { formatCount } from '@shared/lib/format'
import type { AiCompareView } from '@entities/ai.type'

const JUDGE_COLUMNS = ['pgreat', 'great', 'good', 'bad', 'poor'] as const

/** The public judgement totals of two plays of one chart, side by side (plan 5.3 with no segment comparison). */
export const AiCompareTotalsTable: FC<{ totals: NonNullable<AiCompareView['totals']> }> = ({ totals }) => {
    const t = useTranslations('pages.ai.compareTotals')

    return (
        <table className='w-full text-xs tabular-nums'>
            <thead>
                <tr className='text-muted-foreground text-left'>
                    <th className='font-normal'>{t('play')}</th>
                    <th className='font-normal'>{t('lamp')}</th>
                    <th className='font-normal'>EX</th>
                    <th className='font-normal'>BP</th>
                    {JUDGE_COLUMNS.map((column) => (
                        <th key={column} className='font-normal uppercase'>
                            {column}
                        </th>
                    ))}
                </tr>
            </thead>
            <tbody>
                {[
                    { key: 'thisPlay', label: t('thisPlay'), row: totals.base },
                    { key: 'comparedPlay', label: t('comparedPlay'), row: totals.target },
                ].map(({ key, label, row }) => (
                    <tr key={key}>
                        <td>{label}</td>
                        <td>
                            <LampLabel lamp={row.lamp} assistGrade={row.assist_grade} />
                        </td>
                        <td>{formatCount(row.ex_score)}</td>
                        <td>{formatCount(row.minbp)}</td>
                        {JUDGE_COLUMNS.map((column) => (
                            <td key={column}>{formatCount(row[column])}</td>
                        ))}
                    </tr>
                ))}
            </tbody>
        </table>
    )
}

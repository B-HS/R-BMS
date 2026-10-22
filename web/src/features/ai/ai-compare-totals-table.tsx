import type { FC } from 'react'
import { LampLabel } from '@features/lamp/lamp-label'
import { formatCount } from '@shared/lib/format'
import type { AiCompareView } from '@entities/ai.type'

const JUDGE_COLUMNS = ['pgreat', 'great', 'good', 'bad', 'poor'] as const

/** The public judgement totals of two plays of one chart, side by side (plan 5.3 with no segment comparison). */
export const AiCompareTotalsTable: FC<{ totals: NonNullable<AiCompareView['totals']> }> = ({ totals }) => (
    <table className='w-full text-xs tabular-nums'>
        <thead>
            <tr className='text-muted-foreground text-left'>
                <th className='font-normal'>판</th>
                <th className='font-normal'>램프</th>
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
                { label: '이 판', row: totals.base },
                { label: '비교 판', row: totals.target },
            ].map(({ label, row }) => (
                <tr key={label}>
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

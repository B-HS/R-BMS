import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useAiTrainingFocusLabel } from '@entities/ai.query'
import { AI_TRAINING_RANDOM_OFF, isAiTrainingPositionalKey, type AiTrainingChartDetail } from '@entities/ai.type'

type AiTrainingSectionTrendProps = {
    sections: AiTrainingChartDetail['sections']
    plays: AiTrainingChartDetail['plays']
}

const VIEW = { WIDTH: 120, HEIGHT: 40 } as const
const PADDING = { X: 6, Y: 5 } as const
const POINT_RADIUS = 2
const EX_RANGE_PADDING = 0.02
const EX_RANGE_MIN_SPAN = 0.1
const PERCENT_DECIMALS = 1
const HALF = 0.5

const clampRatio = (value: number) => Math.min(1, Math.max(0, value))

/**
 * How each section's EX ratio moved over the plays of a training chart (plan 2026-11-04 7.3): one small line per section, a point
 * per play from the oldest to the newest. On a section that leans on a lane or a hand, a play with a RANDOM-family option moved
 * the notes to other lanes, so its point is drawn faint and the line leaves it out (plan 7.1).
 */
export const AiTrainingSectionTrend: FC<AiTrainingSectionTrendProps> = ({ sections, plays }) => {
    const t = useTranslations('pages.ai.training.trend')
    const focusLabel = useAiTrainingFocusLabel()
    const ordered = plays.toReversed()
    const hasSections = ordered.some((play) => play.sections !== null)
    const percent = (ratio: number) => `${(ratio * PERCENT_SCALE).toFixed(PERCENT_DECIMALS)}%`

    if (!hasSections) return <p className='text-muted-foreground text-xs break-keep'>{t('empty')}</p>

    return (
        <ul aria-label={t('listLabel')} className='grid grid-cols-1 gap-px sm:grid-cols-2 lg:grid-cols-3'>
            {sections.map((section) => {
                const isPositional = isAiTrainingPositionalKey(section.focus_key)
                const points = ordered.flatMap((play, position) => {
                    const ratio = play.sections?.find((stat) => stat.index === section.index)?.ex_ratio ?? null
                    return ratio === null ? [] : [{ position, ratio, isDimmed: isPositional && play.random !== AI_TRAINING_RANDOM_OFF }]
                })
                const trend = points.filter((point) => !point.isDimmed)
                const ratios = points.map((point) => point.ratio)
                const low = clampRatio(Math.min(...ratios) - EX_RANGE_PADDING)
                const high = clampRatio(Math.max(low + EX_RANGE_MIN_SPAN, Math.max(...ratios) + EX_RANGE_PADDING))
                const xOf = (position: number) =>
                    ordered.length < 2 ? VIEW.WIDTH * HALF : PADDING.X + (position / (ordered.length - 1)) * (VIEW.WIDTH - 2 * PADDING.X)
                const yOf = (ratio: number) => VIEW.HEIGHT - PADDING.Y - ((ratio - low) / (high - low)) * (VIEW.HEIGHT - 2 * PADDING.Y)
                const first = trend[0]
                const last = trend[trend.length - 1]
                const summary =
                    first && last ? t('summary', { first: percent(first.ratio), last: percent(last.ratio), plays: trend.length }) : t('noTrend')

                return (
                    <li key={section.index} className='border-border flex flex-col gap-1 border p-2'>
                        <p className='text-xs font-medium break-keep'>
                            {t('sectionTitle', { index: section.index + 1, focus: focusLabel(section.focus_key) })}
                        </p>
                        {points.length === 0 ? (
                            <p className='text-muted-foreground text-2xs'>{t('noPoints')}</p>
                        ) : (
                            <svg
                                role='img'
                                aria-label={t('chartLabel', { index: section.index + 1, summary })}
                                viewBox={`0 0 ${VIEW.WIDTH} ${VIEW.HEIGHT}`}
                                className='h-10 w-full'>
                                {trend.length > 1 && (
                                    <polyline
                                        points={trend.map((point) => `${xOf(point.position)},${yOf(point.ratio)}`).join(' ')}
                                        fill='none'
                                        className='stroke-chart-1'
                                        vectorEffect='non-scaling-stroke'
                                    />
                                )}
                                {points.map((point) => (
                                    <circle
                                        key={point.position}
                                        cx={xOf(point.position)}
                                        cy={yOf(point.ratio)}
                                        r={POINT_RADIUS}
                                        data-dimmed={point.isDimmed}
                                        className={point.isDimmed ? 'fill-muted-foreground/30' : 'fill-chart-1'}
                                    />
                                ))}
                            </svg>
                        )}
                        <p className='text-muted-foreground text-2xs tabular-nums'>{summary}</p>
                    </li>
                )
            })}
        </ul>
    )
}

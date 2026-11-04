import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { cn } from '@shared/lib/cn'
import { PERCENT_SCALE } from '@shared/constants/ui'

type AiSegmentChartSegment = { id: string; bp: number; gauge: number | null; isProblem: boolean; hasBpRun?: boolean }

type AiSegmentChartProps = {
    segments: readonly AiSegmentChartSegment[]
    gaugeMax: number
    selectedId: string | null
    onSelect: (id: string) => void
}

const MIN_BAR_PERCENT = 2
const BAR_CENTER = 0.5
const CHART_HEIGHT_CLASS = 'h-28'
const MARKER_VIEW = { WIDTH: 8, HEIGHT: 6 } as const
const MARKER_POINTS = `0,0 ${MARKER_VIEW.WIDTH},0 ${MARKER_VIEW.WIDTH / 2},${MARKER_VIEW.HEIGHT}`

/**
 * The deterministic picture of one play (plan 5.2): a bar per segment for its BP and the played gauge at each segment's end as a line,
 * with a small triangle over each segment where a BAD/POOR run starts (plan 2026-11-04 2.7). Each bar is a button that selects its segment.
 */
export const AiSegmentChart: FC<AiSegmentChartProps> = ({ segments, gaugeMax, selectedId, onSelect }) => {
    const t = useTranslations('pages.ai')
    const maxBp = Math.max(1, ...segments.map((segment) => segment.bp))
    const points = segments.flatMap((segment, index) =>
        segment.gauge === null || gaugeMax <= 0
            ? []
            : [
                  `${((index + BAR_CENTER) / segments.length) * PERCENT_SCALE},${PERCENT_SCALE - (Math.min(segment.gauge, gaugeMax) / gaugeMax) * PERCENT_SCALE}`,
              ],
    )

    return (
        <div className={cn('relative w-full', CHART_HEIGHT_CLASS)}>
            <ul aria-label={t('segmentChart.bpBySegment')} className='flex h-full items-end gap-px'>
                {segments.map((segment) => (
                    <li key={segment.id} className='relative flex h-full flex-1 items-end'>
                        {segment.hasBpRun && (
                            <svg
                                aria-hidden
                                viewBox={`0 0 ${MARKER_VIEW.WIDTH} ${MARKER_VIEW.HEIGHT}`}
                                className='fill-destructive absolute top-0 left-1/2 h-1.5 w-2 -translate-x-1/2'>
                                <polygon points={MARKER_POINTS} />
                            </svg>
                        )}
                        <button
                            type='button'
                            aria-label={
                                segment.hasBpRun
                                    ? `${segment.id} BP ${segment.bp} · ${t('segmentChart.bpRunStart')}`
                                    : `${segment.id} BP ${segment.bp}`
                            }
                            aria-pressed={selectedId === segment.id}
                            onClick={() => onSelect(segment.id)}
                            className={cn(
                                'w-full outline-none focus-visible:ring-2',
                                segment.isProblem ? 'bg-destructive/70' : 'bg-muted-foreground/40',
                                selectedId === segment.id && 'ring-primary ring-2',
                            )}
                            style={{ height: `${Math.max(MIN_BAR_PERCENT, (segment.bp / maxBp) * PERCENT_SCALE)}%` }}
                        />
                    </li>
                ))}
            </ul>
            {points.length > 1 && (
                <svg
                    aria-hidden
                    viewBox={`0 0 ${PERCENT_SCALE} ${PERCENT_SCALE}`}
                    preserveAspectRatio='none'
                    className='pointer-events-none absolute inset-0 h-full w-full'>
                    <polyline points={points.join(' ')} fill='none' className='stroke-primary' strokeWidth={1} vectorEffect='non-scaling-stroke' />
                </svg>
            )}
        </div>
    )
}

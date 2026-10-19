import type { FC } from 'react'
import { cn } from '@shared/lib/cn'
import { PERCENT_SCALE } from '@shared/constants/ui'

type AiSegmentChartSegment = { id: string; bp: number; gauge: number | null; isProblem: boolean }

type AiSegmentChartProps = {
    segments: readonly AiSegmentChartSegment[]
    gaugeMax: number
    selectedId: string | null
    onSelect: (id: string) => void
}

const MIN_BAR_PERCENT = 2
const BAR_CENTER = 0.5
const CHART_HEIGHT_CLASS = 'h-28'

/**
 * The deterministic picture of one play (plan 5.2): a bar per segment for its BP and the played gauge at each segment's end as a line.
 * Each bar is a button that selects its segment.
 */
export const AiSegmentChart: FC<AiSegmentChartProps> = ({ segments, gaugeMax, selectedId, onSelect }) => {
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
            <ul aria-label='구간별 BP' className='flex h-full items-end gap-px'>
                {segments.map((segment) => (
                    <li key={segment.id} className='flex h-full flex-1 items-end'>
                        <button
                            type='button'
                            aria-label={`${segment.id} BP ${segment.bp}`}
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

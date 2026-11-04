import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import type { AiWeaknessProfile } from '@entities/ai.type'

type DensityProfile = AiWeaknessProfile['density']

const VIEW = { WIDTH: 160, HEIGHT: 56 } as const
const PLOT = { LEFT: 4, RIGHT: 156, TOP: 4, BOTTOM: 52 } as const
const DEFAULT_BIN_WIDTH = 2
const EX_RANGE_PADDING = 0.02
const EX_RANGE_MIN_SPAN = 0.1
const POINT_RADIUS = 1.5
const DASH = '3 3'
const HALF = 0.5

const clampRatio = (value: number) => Math.min(1, Math.max(0, value))

/**
 * A small version of the profile's density limit chart for a dashboard panel (plan 2026-11-04 8.2): the fitted EX line by notes per
 * second, the player's baseline as a dashed line, and the comfort and EX-limit densities as vertical lines. Inline SVG with no
 * animation; the limits are written in its label.
 */
export const DensityLimitMini: FC<{ density: DensityProfile }> = ({ density }) => {
    const t = useTranslations('pages.ai.profile')
    const npsText = (value: number | null) => (value === null ? t('none') : t('npsValue', { nps: value }))

    const bins = density.curve.map((bin) => ({ ...bin, center: (bin.from_nps + (bin.to_nps ?? bin.from_nps + DEFAULT_BIN_WIDTH)) * HALF }))
    const lines = [
        { key: 'comfort', value: density.comfort_nps, className: 'stroke-chart-2' },
        { key: 'limitEx', value: density.limit_nps_ex, className: 'stroke-chart-1' },
    ].flatMap((line) => (line.value === null ? [] : [{ ...line, value: line.value }]))
    const xMax = Math.max(DEFAULT_BIN_WIDTH, ...bins.map((bin) => bin.center), ...lines.map((line) => line.value))
    const xOf = (nps: number) => PLOT.LEFT + (nps / xMax) * (PLOT.RIGHT - PLOT.LEFT)
    const exValues = [...bins.map((bin) => bin.ex_fit), ...(density.baseline_ex === null ? [] : [density.baseline_ex])]
    const exLow = clampRatio(Math.min(...exValues) - EX_RANGE_PADDING)
    const exHigh = clampRatio(Math.max(exLow + EX_RANGE_MIN_SPAN, Math.max(...exValues) + EX_RANGE_PADDING))
    const yOf = (ratio: number) => PLOT.BOTTOM - ((ratio - exLow) / (exHigh - exLow)) * (PLOT.BOTTOM - PLOT.TOP)

    return (
        <svg
            role='img'
            aria-label={t('densityChartLabel', {
                comfort: npsText(density.comfort_nps),
                limitEx: npsText(density.limit_nps_ex),
                limitBp: npsText(density.limit_nps_bp),
                maxSeen: npsText(density.max_seen_nps),
            })}
            viewBox={`0 0 ${VIEW.WIDTH} ${VIEW.HEIGHT}`}
            className='h-auto w-full max-w-sm'>
            <line x1={PLOT.LEFT} y1={PLOT.BOTTOM} x2={PLOT.RIGHT} y2={PLOT.BOTTOM} className='stroke-border' vectorEffect='non-scaling-stroke' />
            {density.baseline_ex !== null && (
                <line
                    x1={PLOT.LEFT}
                    y1={yOf(density.baseline_ex)}
                    x2={PLOT.RIGHT}
                    y2={yOf(density.baseline_ex)}
                    strokeDasharray={DASH}
                    className='stroke-muted-foreground'
                    vectorEffect='non-scaling-stroke'
                />
            )}
            {lines.map((line) => (
                <line
                    key={line.key}
                    x1={xOf(line.value)}
                    y1={PLOT.TOP}
                    x2={xOf(line.value)}
                    y2={PLOT.BOTTOM}
                    className={line.className}
                    vectorEffect='non-scaling-stroke'
                />
            ))}
            {bins.length > 1 && (
                <polyline
                    points={bins.map((bin) => `${xOf(bin.center)},${yOf(bin.ex_fit)}`).join(' ')}
                    fill='none'
                    className='stroke-chart-1'
                    vectorEffect='non-scaling-stroke'
                />
            )}
            {bins.map((bin) => (
                <circle key={bin.from_nps} cx={xOf(bin.center)} cy={yOf(bin.ex_ratio)} r={POINT_RADIUS} className='fill-chart-1/50' />
            ))}
        </svg>
    )
}

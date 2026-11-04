import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { PERCENT_SCALE } from '@shared/constants/ui'
import type { AiWeaknessProfile } from '@entities/ai.type'

type DensityProfile = AiWeaknessProfile['density']

const VIEW = { WIDTH: 320, HEIGHT: 180 } as const
const PLOT = { LEFT: 30, RIGHT: 312, EX_TOP: 10, EX_BOTTOM: 118, BP_TOP: 132, BP_BOTTOM: 162, X_LABEL_Y: 174 } as const
const POINT_RADIUS = { MIN: 1.5, MAX: 6 } as const
const LABEL_FONT_SIZE = 8
const LABEL_LINE_GAP = 9
const LABEL_OFFSET = 2
const DEFAULT_BIN_WIDTH = 2
const X_TICK_STEP = 10
const EX_RANGE_PADDING = 0.02
const EX_RANGE_MIN_SPAN = 0.1
const BP_SCALE_MIN = 0.1
const BAR_GAP = 1
const DASH = '3 3'
const PERCENT_DECIMALS = 0
const HALF = 0.5

const clampRatio = (value: number) => Math.min(1, Math.max(0, value))

/**
 * The density limit curve of the profile (plan 2026-11-04 2.7): EX ratio by local NPS bin as points sized by their notes, the fitted
 * never-rising line, the player's baseline as a dashed line, the comfort and limit densities as vertical lines, and the BP rate of
 * each bin as small bars underneath. Inline SVG with no animation; the three limits are written out in the label.
 */
export const AiDensityLimitChart: FC<{ density: DensityProfile }> = ({ density }) => {
    const t = useTranslations('pages.ai.profile')
    const npsText = (value: number | null) => (value === null ? t('none') : t('npsValue', { nps: value }))

    const bins = density.curve.map((bin) => ({ ...bin, to: bin.to_nps ?? bin.from_nps + DEFAULT_BIN_WIDTH }))
    const lines = [
        { key: 'comfort', value: density.comfort_nps, className: 'stroke-chart-2', textClassName: 'fill-chart-2', label: t('chart.comfort') },
        { key: 'limitEx', value: density.limit_nps_ex, className: 'stroke-chart-1', textClassName: 'fill-chart-1', label: t('chart.limitEx') },
        { key: 'limitBp', value: density.limit_nps_bp, className: 'stroke-chart-3', textClassName: 'fill-chart-3', label: t('chart.limitBp') },
    ].flatMap((line) => (line.value === null ? [] : [{ ...line, value: line.value }]))
    const xMax = Math.max(DEFAULT_BIN_WIDTH, ...bins.map((bin) => bin.to), ...lines.map((line) => line.value))
    const xOf = (nps: number) => PLOT.LEFT + (nps / xMax) * (PLOT.RIGHT - PLOT.LEFT)
    const exValues = [...bins.flatMap((bin) => [bin.ex_ratio, bin.ex_fit]), ...(density.baseline_ex === null ? [] : [density.baseline_ex])]
    const exLow = clampRatio(Math.min(...exValues) - EX_RANGE_PADDING)
    const exHigh = clampRatio(Math.max(exLow + EX_RANGE_MIN_SPAN, Math.max(...exValues) + EX_RANGE_PADDING))
    const yOf = (ratio: number) => PLOT.EX_BOTTOM - ((ratio - exLow) / (exHigh - exLow)) * (PLOT.EX_BOTTOM - PLOT.EX_TOP)
    const maxNotes = Math.max(1, ...bins.map((bin) => bin.notes))
    const radiusOf = (notes: number) => POINT_RADIUS.MIN + Math.sqrt(notes / maxNotes) * (POINT_RADIUS.MAX - POINT_RADIUS.MIN)
    const bpScale = Math.max(BP_SCALE_MIN, ...bins.map((bin) => bin.bp_rate))
    const centerOf = (bin: (typeof bins)[number]) => xOf((bin.from_nps + bin.to) * HALF)
    const ticks = Array.from({ length: Math.floor(xMax / X_TICK_STEP) + 1 }, (_, index) => index * X_TICK_STEP)
    const percent = (ratio: number) => `${(ratio * PERCENT_SCALE).toFixed(PERCENT_DECIMALS)}%`

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
            className='h-auto w-full max-w-2xl'>
            <line
                x1={PLOT.LEFT}
                y1={PLOT.EX_BOTTOM}
                x2={PLOT.RIGHT}
                y2={PLOT.EX_BOTTOM}
                className='stroke-border'
                vectorEffect='non-scaling-stroke'
            />
            <line
                x1={PLOT.LEFT}
                y1={PLOT.BP_BOTTOM}
                x2={PLOT.RIGHT}
                y2={PLOT.BP_BOTTOM}
                className='stroke-border'
                vectorEffect='non-scaling-stroke'
            />
            <text x={PLOT.LEFT - LABEL_OFFSET} y={PLOT.EX_TOP} textAnchor='end' fontSize={LABEL_FONT_SIZE} className='fill-muted-foreground'>
                {percent(exHigh)}
            </text>
            <text x={PLOT.LEFT - LABEL_OFFSET} y={PLOT.EX_BOTTOM} textAnchor='end' fontSize={LABEL_FONT_SIZE} className='fill-muted-foreground'>
                {percent(exLow)}
            </text>
            <text
                x={PLOT.LEFT - LABEL_OFFSET}
                y={PLOT.BP_TOP + LABEL_FONT_SIZE}
                textAnchor='end'
                fontSize={LABEL_FONT_SIZE}
                className='fill-muted-foreground'>
                BP
            </text>
            {ticks.map((tick) => (
                <text key={tick} x={xOf(tick)} y={PLOT.X_LABEL_Y} textAnchor='middle' fontSize={LABEL_FONT_SIZE} className='fill-muted-foreground'>
                    {tick}
                </text>
            ))}
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
            {lines.map((line, index) => (
                <g key={line.key}>
                    <line
                        x1={xOf(line.value)}
                        y1={PLOT.EX_TOP}
                        x2={xOf(line.value)}
                        y2={PLOT.BP_BOTTOM}
                        className={line.className}
                        vectorEffect='non-scaling-stroke'
                    />
                    <text
                        x={xOf(line.value) + LABEL_OFFSET}
                        y={PLOT.EX_TOP + LABEL_FONT_SIZE + index * LABEL_LINE_GAP}
                        fontSize={LABEL_FONT_SIZE}
                        className={line.textClassName}>
                        {line.label} {line.value}
                    </text>
                </g>
            ))}
            {bins.length > 1 && (
                <polyline
                    points={bins.map((bin) => `${centerOf(bin)},${yOf(bin.ex_fit)}`).join(' ')}
                    fill='none'
                    className='stroke-chart-1'
                    vectorEffect='non-scaling-stroke'
                />
            )}
            {bins.map((bin) => (
                <circle
                    key={bin.from_nps}
                    cx={centerOf(bin)}
                    cy={yOf(bin.ex_ratio)}
                    r={radiusOf(bin.notes)}
                    className='fill-chart-1/40 stroke-chart-1'
                />
            ))}
            {bins.map((bin) => {
                const height = (bin.bp_rate / bpScale) * (PLOT.BP_BOTTOM - PLOT.BP_TOP)
                return (
                    <rect
                        key={bin.from_nps}
                        x={xOf(bin.from_nps) + BAR_GAP}
                        y={PLOT.BP_BOTTOM - height}
                        width={Math.max(BAR_GAP, xOf(bin.to) - xOf(bin.from_nps) - 2 * BAR_GAP)}
                        height={height}
                        className='fill-chart-3/70'
                    />
                )
            })}
        </svg>
    )
}

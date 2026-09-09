'use client'

import type { FC } from 'react'
import type { ReplayEvent } from '@entities/replay.type'

const LANE_HEIGHT_PX = 26
const LANE_GAP_PX = 2
const HEADER_WIDTH_PX = 64
const PIXELS_PER_SECOND = 96
const MIN_TRACK_WIDTH_PX = 720
const EVENT_WIDTH_PX = 3
const MICROSECONDS_PER_SECOND = 1_000_000
const GRID_INTERVAL_SECONDS = 1

export const MAX_RENDERED_REPLAY_EVENTS = 4000

const laneLabel = (lane: number) => (lane === 0 ? 'SC' : String(lane))

type ReplayWaterfallProps = {
    events: ReplayEvent[]
    durationUs: number
}

export const ReplayWaterfall: FC<ReplayWaterfallProps> = ({ events, durationUs }) => {
    const renderedEvents = events.slice(0, MAX_RENDERED_REPLAY_EVENTS)
    const lanes = Array.from(new Set(renderedEvents.map((event) => event.lane))).sort((a, b) => a - b)
    const durationSeconds = Math.max(durationUs / MICROSECONDS_PER_SECOND, 1)
    const trackWidth = Math.max(MIN_TRACK_WIDTH_PX, Math.ceil(durationSeconds * PIXELS_PER_SECOND))
    const svgWidth = HEADER_WIDTH_PX + trackWidth
    const svgHeight = lanes.length * (LANE_HEIGHT_PX + LANE_GAP_PX)
    const gridCount = Math.ceil(durationSeconds / GRID_INTERVAL_SECONDS)
    const laneIndex = new Map(lanes.map((lane, index) => [lane, index]))

    return (
        <div className='overflow-x-auto'>
            <svg
                width={svgWidth}
                height={svgHeight}
                viewBox={`0 0 ${svgWidth} ${svgHeight}`}
                role='img'
                aria-label={`레인 ${lanes.length}개에 걸친 입력 이벤트 ${renderedEvents.length}건, 총 길이 ${durationSeconds.toFixed(1)}초`}>
                {lanes.map((lane, index) => (
                    <g key={lane}>
                        <rect
                            x={HEADER_WIDTH_PX}
                            y={index * (LANE_HEIGHT_PX + LANE_GAP_PX)}
                            width={trackWidth}
                            height={LANE_HEIGHT_PX}
                            fill='var(--color-muted)'
                        />
                        <text
                            x={0}
                            y={index * (LANE_HEIGHT_PX + LANE_GAP_PX) + LANE_HEIGHT_PX / 2}
                            dominantBaseline='middle'
                            fontSize={11}
                            fontFamily='var(--font-mono)'
                            fill='var(--color-muted-foreground)'>
                            {laneLabel(lane)}
                        </text>
                    </g>
                ))}
                {Array.from({ length: gridCount }, (_, index) => (
                    <line
                        key={index}
                        x1={HEADER_WIDTH_PX + index * GRID_INTERVAL_SECONDS * PIXELS_PER_SECOND}
                        x2={HEADER_WIDTH_PX + index * GRID_INTERVAL_SECONDS * PIXELS_PER_SECOND}
                        y1={0}
                        y2={svgHeight}
                        stroke='var(--color-border)'
                        strokeWidth={1}
                    />
                ))}
                {renderedEvents.map((event, index) => (
                    <rect
                        key={`${index}-${event.lane}-${event.t_us}`}
                        x={HEADER_WIDTH_PX + (event.t_us / MICROSECONDS_PER_SECOND) * PIXELS_PER_SECOND}
                        y={(laneIndex.get(event.lane) ?? 0) * (LANE_HEIGHT_PX + LANE_GAP_PX)}
                        width={EVENT_WIDTH_PX}
                        height={LANE_HEIGHT_PX}
                        fill={event.press ? 'var(--color-chart-2)' : 'var(--color-chart-4)'}
                    />
                ))}
            </svg>
        </div>
    )
}

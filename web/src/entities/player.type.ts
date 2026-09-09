import type { ClearLamp } from '@shared/constants/clear-lamp'
import type { ActivityRow } from '@entities/stats.type'

export type PlayerProfile = {
    id: string
    name: string
    total_plays: number
    rank_points: number
    extra: Record<string, unknown>
}

export type PlayerRecentRow = ActivityRow

export type PlayerStats = {
    total_plays: number
    cleared: number
    lamps: { clear: ClearLamp; count: number }[]
}

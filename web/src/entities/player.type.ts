import type { AssistGrade, ClearLamp } from '@shared/constants/clear-lamp'
import type { ActivityRow } from '@entities/stats.type'

export type PlayerProfile = {
    id: string
    name: string
    total_plays: number
    rank_points: number
    extra: Record<string, unknown>
}

export type PlayerRecentRow = ActivityRow

export type PlayerRecentParams = {
    page: number
    limit: number
}

export type PlayerStats = {
    total_plays: number
    cleared: number
    lamps: { clear: ClearLamp; assist_grade: AssistGrade; count: number }[]
    scratch_auto_charts: number
}

export type PlayerPlayDay = {
    date: string
    plays: number
    play_ms: number
}

export type PlayerPlayDays = {
    days: PlayerPlayDay[]
    total_plays: number
    total_ms: number
}

import type { AssistGrade, ClearLamp } from '@shared/constants/clear-lamp'
import type { DisplayedDan } from '@entities/dan.type'
import type { PlayerProfile } from '@entities/player.type'
import type { PlayerId } from '@entities/score.type'

export type StatsSummary = {
    charts: number
    players: number
    scores: number
}

export type ActivityChartRef = {
    md5: string
    sha256: string
    title: string
    artist: string
    level: number | null
}

export type ActivityRow = {
    score_id: string
    player: PlayerId
    player_name: string
    clear: ClearLamp
    assist_grade: AssistGrade
    ex_score: number
    minbp: number
    max_combo: number
    played_at: number
    scratch_auto: boolean
    chart: ActivityChartRef
}

export type PlayerLeaderboardParams = {
    sort: string
    page: number
    limit: number
    include_scratch_auto: boolean
}

export type PlayerLeaderboardRow = PlayerProfile & {
    dans: DisplayedDan[]
    cleared_charts: number
    played_charts: number
}

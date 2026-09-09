import type { ClearLamp } from '@shared/constants/clear-lamp'
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
    ex_score: number
    minbp: number
    max_combo: number
    played_at: number
    chart: ActivityChartRef
}

export type PlayerLeaderboardRow = PlayerProfile

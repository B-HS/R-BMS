import type { PlayerId } from '@entities/score.type'

export type ReplayEvent = {
    t_us: number
    lane: number
    press: boolean
}

export type ReplayChartRef = {
    md5: string
    sha256: string
}

export type ReplayMeta = {
    id: string
    url: string
    player: PlayerId
    player_name: string
    chart_sha256: string
    score_id: string | null
    format: string
    mode: string
    seed: number
    lntype: number
    event_count: number
    duration_us: number
    size: number
    client_build_sha256: string | null
    created_at: number
}

export type ReplayData = {
    api_version: number
    id: string
    format: string
    chart: ReplayChartRef
    score_id: string | null
    mode: string
    random: string
    random_p2: string | null
    seed: number
    lntype: number
    offset_ms: number
    judge_rate: number
    scratch_auto: boolean
    constant: boolean
    client_build_sha256: string | null
    events: ReplayEvent[]
    event_count: number
    duration_us: number
    size: number
    extra: Record<string, unknown>
}

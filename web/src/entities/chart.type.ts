import type { ScoreRecord } from '@entities/score.type'

export type ChartMeta = {
    md5: string
    sha256: string
    title: string
    subtitle: string
    genre: string
    artist: string
    subartist: string
    level: number | null
    total: number | null
    mode: string
    lntype: number
    judge: number
    minbpm: number
    maxbpm: number
    notes: number
    has_ln: boolean
    has_cn: boolean
    has_hcn: boolean
    has_mine: boolean
    has_random: boolean
    has_stop: boolean
    url: string | null
    appendurl: string | null
    extra: Record<string, unknown>
}

export type ChartLeaderboard = {
    chart: ChartMeta | null
    ranking: ScoreRecord[]
}

export type ChartSearchRow = ChartMeta

export type ChartSearchParams = {
    q?: string
    mode?: string
    level?: string
    sort?: string
    page: number
    limit: number
}

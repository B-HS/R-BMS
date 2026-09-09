import type { ClearLamp } from '@shared/constants/clear-lamp'

export type JudgeBreakdown = {
    pgreat: number
    great: number
    good: number
    bad: number
    poor: number
    miss: number
    fast: number
    slow: number
    combobreak: number
    epg: number
    lpg: number
    egr: number
    lgr: number
    egd: number
    lgd: number
    ebd: number
    lbd: number
    epr: number
    lpr: number
    ems: number
    lms: number
    avgjudge: number
    empty_poor: number
}

export type PlayerId = { id: string }

export type ScoreRecord = {
    player: PlayerId
    player_name: string
    clear: ClearLamp
    ex_score: number
    max_combo: number
    minbp: number
    rank: number | null
    played_at: number
    lntype: number
    option: number
    total_notes: number
    judge: JudgeBreakdown | null
    extra: Record<string, unknown>
}

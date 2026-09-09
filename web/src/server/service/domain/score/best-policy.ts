import type { JudgeBreakdownInput } from '@server/dto/common'

export type BestCandidate = {
    clear: number
    exScore: number
    minbp: number
}

export const deriveExScore = (judge: JudgeBreakdownInput) => {
    const pgreat = judge.epg + judge.lpg || judge.pgreat
    const great = judge.egr + judge.lgr || judge.great
    return pgreat * 2 + great
}

export const deriveMinbp = (judge: JudgeBreakdownInput) => {
    const bad = judge.ebd + judge.lbd || judge.bad
    const poor = judge.epr + judge.lpr || judge.poor
    const miss = judge.ems + judge.lms || judge.miss
    return bad + poor + miss
}

export const isBetterBest = (candidate: BestCandidate, current: BestCandidate | null) => {
    if (!current) return true
    if (candidate.clear !== current.clear) return candidate.clear > current.clear
    if (candidate.exScore !== current.exScore) return candidate.exScore > current.exScore
    return candidate.minbp < current.minbp
}

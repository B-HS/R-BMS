import type { JudgeBreakdownInput } from '@server/dto/common'

export type BestCandidate = {
    clear: number
    assistGrade: number
    exScore: number
    minbp: number
}

export const EX_PER_PGREAT = 2

export type JudgeCountFields = Omit<JudgeBreakdownInput, 'fast' | 'slow' | 'combobreak' | 'avgjudge' | 'empty_poor'>

/**
 * The six judge counts a body stands for: each tier's early/late split when it sent one, its total otherwise.
 * EX, BP, the note bound and the lamp check all read these, so a split and a total that disagree are never read one way
 * for the score and another way for the checks.
 */
export const effectiveJudgeCounts = (judge: JudgeCountFields) => ({
    pgreat: judge.epg + judge.lpg || judge.pgreat,
    great: judge.egr + judge.lgr || judge.great,
    good: judge.egd + judge.lgd || judge.good,
    bad: judge.ebd + judge.lbd || judge.bad,
    poor: judge.epr + judge.lpr || judge.poor,
    miss: judge.ems + judge.lms || judge.miss,
})

export type EffectiveJudgeCounts = ReturnType<typeof effectiveJudgeCounts>

export const deriveExScore = (judge: JudgeCountFields) => {
    const counts = effectiveJudgeCounts(judge)
    return counts.pgreat * EX_PER_PGREAT + counts.great
}

export const deriveMinbp = (judge: JudgeCountFields) => {
    const counts = effectiveJudgeCounts(judge)
    return counts.bad + counts.poor + counts.miss
}

export const isBetterBest = (candidate: BestCandidate, current: BestCandidate | null) => {
    if (!current) return true
    if (candidate.clear !== current.clear) return candidate.clear > current.clear
    if (candidate.assistGrade !== current.assistGrade) return candidate.assistGrade > current.assistGrade
    if (candidate.exScore !== current.exScore) return candidate.exScore > current.exScore
    return candidate.minbp < current.minbp
}

import type { JudgeBreakdownInput } from '@server/dto/common'
import { effectiveJudgeCounts } from '@server/service/domain/score/best-policy'

export type JudgeColumns = {
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
    emptyPoor: number
}

/**
 * A stored score's judge counts as the API returns them. The six totals are the ones the server checked and scored
 * (`effectiveJudgeCounts`): a tier with an early/late split answers with that split's sum, never with a total the body sent
 * beside it and no check read.
 */
export const toJudgeBreakdown = (row: JudgeColumns): JudgeBreakdownInput => ({
    ...effectiveJudgeCounts(row),
    fast: row.fast,
    slow: row.slow,
    combobreak: row.combobreak,
    epg: row.epg,
    lpg: row.lpg,
    egr: row.egr,
    lgr: row.lgr,
    egd: row.egd,
    lgd: row.lgd,
    ebd: row.ebd,
    lbd: row.lbd,
    epr: row.epr,
    lpr: row.lpr,
    ems: row.ems,
    lms: row.lms,
    avgjudge: row.avgjudge,
    empty_poor: row.emptyPoor,
})

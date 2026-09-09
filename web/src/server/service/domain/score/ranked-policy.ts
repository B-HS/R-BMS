import type { PlayOptionsInput } from '@server/dto/common'

export const SCORE_FLAG = {
    AUTOPLAY: 'AUTOPLAY',
    SCRATCH_AUTO: 'SCRATCH_AUTO',
    ASSIST: 'ASSIST',
    JUDGE_WIDTH: 'JUDGE_WIDTH',
    TOTAL_OVERRIDE: 'TOTAL_OVERRIDE',
    UNKNOWN_BUILD: 'UNKNOWN_BUILD',
    GUEST: 'GUEST',
} as const

export type ScoreFlag = (typeof SCORE_FLAG)[keyof typeof SCORE_FLAG]

export const BUILD_TRUST = {
    TRUSTED: 'trusted',
    UNKNOWN: 'unknown',
    UNTRUSTED: 'untrusted',
} as const

export type BuildTrust = (typeof BUILD_TRUST)[keyof typeof BUILD_TRUST]

const NEUTRAL_JUDGE_RATE = 100

export type RankedPolicyInput = {
    options: PlayOptionsInput
    buildTrust: BuildTrust
    isGuest: boolean
    requireTrustedBuild: boolean
}

export const rankedPolicy = ({ options, buildTrust, isGuest, requireTrustedBuild }: RankedPolicyInput) => {
    const flags: ScoreFlag[] = []
    if (options.autoplay) flags.push(SCORE_FLAG.AUTOPLAY)
    if (options.scratch_auto) flags.push(SCORE_FLAG.SCRATCH_AUTO)
    if (options.assist.length > 0) flags.push(SCORE_FLAG.ASSIST)
    if (options.judge_rate > NEUTRAL_JUDGE_RATE) flags.push(SCORE_FLAG.JUDGE_WIDTH)
    if (options.total_override > 0) flags.push(SCORE_FLAG.TOTAL_OVERRIDE)
    if (buildTrust !== BUILD_TRUST.TRUSTED) flags.push(SCORE_FLAG.UNKNOWN_BUILD)
    if (isGuest) flags.push(SCORE_FLAG.GUEST)
    const buildBlocksRank = buildTrust === BUILD_TRUST.UNTRUSTED || (buildTrust === BUILD_TRUST.UNKNOWN && requireTrustedBuild)
    const rankBlockingFlags = flags.filter((flag) => flag !== SCORE_FLAG.UNKNOWN_BUILD)
    return { ranked: rankBlockingFlags.length === 0 && !buildBlocksRank, flags }
}

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

export const AUTO_SCRATCH_ASSIST_TAG = 'AUTO_SCRATCH'

const RANK_NEUTRAL_FLAGS: ScoreFlag[] = [SCORE_FLAG.SCRATCH_AUTO, SCORE_FLAG.UNKNOWN_BUILD]

type AssistOptions = Pick<PlayOptionsInput, 'scratch_auto' | 'assist'>

export const isScratchAutoPlay = ({ scratch_auto, assist }: AssistOptions) => scratch_auto || assist.includes(AUTO_SCRATCH_ASSIST_TAG)

export const assistFlags = (options: AssistOptions) => [
    ...(isScratchAutoPlay(options) ? [SCORE_FLAG.SCRATCH_AUTO] : []),
    ...(options.assist.some((tag) => tag !== AUTO_SCRATCH_ASSIST_TAG) ? [SCORE_FLAG.ASSIST] : []),
]

export const rankBlockingFlags = (flags: ScoreFlag[]) => flags.filter((flag) => !RANK_NEUTRAL_FLAGS.includes(flag))

export const submissionMessage = (ranked: boolean, flags: ScoreFlag[]) => {
    if (!ranked)
        return `recorded (unranked: ${flags
            .filter((flag) => flag !== SCORE_FLAG.SCRATCH_AUTO)
            .join(',')
            .toLowerCase()})`
    return flags.includes(SCORE_FLAG.SCRATCH_AUTO) ? 'saved (scratch auto)' : 'saved'
}

export type RankedPolicyInput = {
    options: PlayOptionsInput
    buildTrust: BuildTrust
    isGuest: boolean
    requireTrustedBuild: boolean
}

export const rankedPolicy = ({ options, buildTrust, isGuest, requireTrustedBuild }: RankedPolicyInput) => {
    const flags: ScoreFlag[] = []
    if (options.autoplay) flags.push(SCORE_FLAG.AUTOPLAY)
    flags.push(...assistFlags(options))
    if (options.judge_rate > NEUTRAL_JUDGE_RATE) flags.push(SCORE_FLAG.JUDGE_WIDTH)
    if (options.total_override > 0) flags.push(SCORE_FLAG.TOTAL_OVERRIDE)
    if (buildTrust !== BUILD_TRUST.TRUSTED) flags.push(SCORE_FLAG.UNKNOWN_BUILD)
    if (isGuest) flags.push(SCORE_FLAG.GUEST)
    const buildBlocksRank = buildTrust === BUILD_TRUST.UNTRUSTED || (buildTrust === BUILD_TRUST.UNKNOWN && requireTrustedBuild)
    return { ranked: rankBlockingFlags(flags).length === 0 && !buildBlocksRank, flags, scratchAuto: isScratchAutoPlay(options) }
}

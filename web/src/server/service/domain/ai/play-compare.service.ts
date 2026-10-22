import { AI_TEXT_LIMIT, type AiEffort } from '@server/dto/ai'
import {
    aiCompareOutputSchema,
    type AiCompareMode,
    type AiCompareOutput,
    type AiCompareQuery,
    type AiCompareReason,
    type AiCompareView,
} from '@server/dto/ai-play'
import { clearLampFromId } from '@server/dto/common'
import {
    DIGEST_IS_DATA_RULE,
    DIGEST_KEY_RULE,
    cutText,
    fitsLine,
    notEnoughData,
    readStoredInterpretation,
    runAiFeature,
    type AiFeatureSpec,
} from '@server/service/domain/ai/ai-digest'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import { COMPARE_GAP_SEGMENTS, PLAY_FEATURE_ANALYSIS_VERSION } from '@server/service/domain/ai/play-feature.constants'
import { isShownTag, segmentKey } from '@server/service/domain/ai/play-digest'
import type { PlayScoreDb, PlayScoreRow } from '@server/service/domain/ai/replay-coach.service'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import { deriveAssistGrade } from '@server/service/domain/score/assist-grade'
import type { AnalyzedPlay, RivalSegmentPlay, TraceService } from '@server/service/domain/trace/trace.service'

export const AI_PLAY_COMPARE_FEATURE = 'play_compare'
export const AI_PLAY_COMPARE_PROMPT_VERSION = 2
export const AI_PLAY_COMPARE_MAX_OUTPUT_TOKENS = 6_000
export const AI_PLAY_COMPARE_EFFORT = 'low' satisfies AiEffort

export const AI_PLAY_COMPARE_SYSTEM_PROMPT = [
    'You explain where one play of a rhythm game (BMS) chart gained or lost against another play of the same chart.',
    DIGEST_IS_DATA_RULE,
    '`gaps` are the time segments with the largest differences: `ex_diff` and `bp_diff` are this play minus the other (a positive `ex_diff` means this play scored more EX there), `cumulative_ex_diff` is the difference summed up to the end of the segment, and `top_tags` are the main pattern tags of this play there.',
    '`overall.ex_diff` is the sum of `ex_diff` over the `overall.compared_segments` segments both plays reached. A segment one of the plays never reached (it left or its gauge ran out first) is left out of that sum and of `gaps`; do not say anything about such segments.',
    "The other play is the player's own best or a rival who chose to share segment totals; nothing else of the rival is known. Lane options of the two plays may differ.",
    `For each gap you talk about write \`segment_id\` exactly as given and one Korean sentence \`explanation\`; at most ${COMPARE_GAP_SEGMENTS}. \`takeaway\` is one Korean sentence on what to practise.`,
    DIGEST_KEY_RULE,
    'All text is Korean; `summary` is two sentences at most.',
].join('\n')

export const AI_PLAY_COMPARE_SPEC = {
    feature: AI_PLAY_COMPARE_FEATURE,
    analysisVersion: PLAY_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_PLAY_COMPARE_PROMPT_VERSION,
    system: AI_PLAY_COMPARE_SYSTEM_PROMPT,
    schemaName: 'play_compare',
    schema: aiCompareOutputSchema,
    maxOutputTokens: AI_PLAY_COMPARE_MAX_OUTPUT_TOKENS,
    effort: AI_PLAY_COMPARE_EFFORT,
} satisfies AiFeatureSpec<typeof aiCompareOutputSchema>

/** A player's public best on a chart (`chart_best` and its score): the judgement totals anyone may read. */
export type PublicBestRow = {
    scoreId: string
    playedAt: number
    clear: number
    gauge: number
    exScore: number
    minbp: number
    random: string
    scratchAuto: boolean
    pgreat: number
    great: number
    good: number
    bad: number
    poor: number
}

export type RivalRow = { userId: string; loginId: string; name: string; allowsSegments: boolean }

export type AiPlayCompareServiceDb = PlayScoreDb & {
    /** The public best of `userId` on a chart with that SCRATCH AUTO, or null. */
    findPublicBest: (params: { userId: string; chartSha256: string; scratchAuto: boolean }) => Promise<PublicBestRow | null>
    /** The players `userId` keeps as rivals, and whether each shares segment totals. */
    listRivals: (userId: string) => Promise<RivalRow[]>
    findLoginId: (userId: string) => Promise<string | null>
}

type AiPlayCompareServiceDeps = {
    db: AiPlayCompareServiceDb
    plays: Pick<TraceService, 'analyzedPlayOfScore' | 'bestAnalyzedPlayOnChart' | 'rivalSegmentsOnChart'>
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
}

type SegmentCounts = { id: string; chart_notes: number; notes: number; ex_score: number; bp: number }
type BaseSegment = SegmentCounts & { from_ms: number; to_ms: number; top_tags: { tag: string; notes: number }[] }

const totalsOf = (row: Pick<PublicBestRow, 'clear' | 'gauge' | 'exScore' | 'minbp' | 'pgreat' | 'great' | 'good' | 'bad' | 'poor'>) => ({
    lamp: clearLampFromId(row.clear),
    assist_grade: deriveAssistGrade(row),
    ex_score: row.exScore,
    minbp: row.minbp,
    pgreat: row.pgreat,
    great: row.great,
    good: row.good,
    bad: row.bad,
    poor: row.poor,
})

/**
 * How many leading segments a play reached: every segment before the first one whose judged objects did not all take a judgement. A
 * play that left (Escape, quit) or whose gauge ran out never judged the objects after that point, so what its later segments hold
 * says nothing about how it would have played them.
 */
const reachedSegmentCountOf = (segments: readonly SegmentCounts[]) => {
    const firstUnreached = segments.findIndex((segment) => segment.notes < segment.chart_notes)
    return firstUnreached === -1 ? segments.length : firstUnreached
}

/**
 * Lines two plays of one chart up segment by segment (plan 5.3). Lane options never move a note in time, so the same chart cut into
 * the same segments has the same judged objects in each; when any segment's count differs (another LN type, SCRATCH AUTO on one
 * side) the plays are not comparable and this is null. `ex_diff` and `bp_diff` are the base minus the other. A segment either play
 * did not reach has null differences and no part in the running total (`cumulative_ex_diff`).
 */
export const compareSegments = (base: readonly BaseSegment[], other: readonly SegmentCounts[]) => {
    if (base.length !== other.length || base.some((segment, index) => segment.chart_notes !== other[index]?.chart_notes)) return null
    const reachedByBoth = Math.min(reachedSegmentCountOf(base), reachedSegmentCountOf(other))
    let cumulative = 0
    return base.map((segment, index) => {
        const shared = {
            key: segmentKey(segment.id),
            id: segment.id,
            from_ms: segment.from_ms,
            to_ms: segment.to_ms,
            chart_notes: segment.chart_notes,
        }
        const topTags = segment.top_tags.filter((tag) => isShownTag(tag.tag))
        if (index >= reachedByBoth) return { ...shared, ex_diff: null, bp_diff: null, cumulative_ex_diff: null, top_tags: topTags }
        const exDiff = segment.ex_score - (other[index]?.ex_score ?? 0)
        cumulative += exDiff
        return { ...shared, ex_diff: exDiff, bp_diff: segment.bp - (other[index]?.bp ?? 0), cumulative_ex_diff: cumulative, top_tags: topTags }
    })
}

export type ComparedSegment = NonNullable<ReturnType<typeof compareSegments>>[number]

/** The segments with the largest EX difference first (BP difference next, the earlier on a tie), those both plays reached and that differ only. */
export const pickGapSegments = (segments: readonly ComparedSegment[]) =>
    segments
        .flatMap(({ id, ex_diff, bp_diff }) =>
            ex_diff === null || bp_diff === null || (ex_diff === 0 && bp_diff === 0)
                ? []
                : [{ id, exGap: Math.abs(ex_diff), bpGap: Math.abs(bp_diff) }],
        )
        .toSorted((left, right) => right.exGap - left.exGap || right.bpGap - left.bpGap)
        .slice(0, COMPARE_GAP_SEGMENTS)
        .map(({ id }) => id)

const baseSegmentsOf = (play: AnalyzedPlay | null) => play?.summary.metrics?.segments ?? null

export const compareSubjectKey = (params: { scoreId: string; against: AiCompareQuery['against']; targetScoreId: string }) =>
    `cmp:${params.scoreId}:${params.against}:${params.targetScoreId}`

/** Keeps the explanations of gap segments that fit one line; the answer is rejected with no gap left. */
export const refineCompareOutput = (output: AiCompareOutput, gapIds: ReadonlySet<string>) => {
    const gaps = output.gaps
        .filter((gap) => gapIds.has(gap.segment_id) && fitsLine(gap.explanation, AI_TEXT_LIMIT.REASON_MAX_CHARS))
        .slice(0, COMPARE_GAP_SEGMENTS)
        .map((gap) => ({ segment_id: gap.segment_id, explanation: gap.explanation.trim() }))
    if (output.summary.trim() === '' || gaps.length === 0) return null
    return {
        summary: cutText(output.summary, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
        gaps,
        takeaway: cutText(output.takeaway, AI_TEXT_LIMIT.REASON_MAX_CHARS),
    }
}

const modeOf = (params: { hasSegments: boolean; hasTotals: boolean }): AiCompareMode => {
    if (params.hasSegments) return 'segments'
    return params.hasTotals ? 'totals' : 'none'
}

/** What the AI is told of the other play: an own play's result and option, and of a rival nothing but that it is one (ADR Q4). */
const targetDigestOf = (side: CompareSide) =>
    side.who === 'rival'
        ? { key: 'target', who: side.who }
        : { key: 'target', who: side.who, ex_score: side.ex_score, lamp: side.lamp, random: side.random }

type CompareSide = NonNullable<AiCompareView['target']>

/** What one other play offers a comparison: its segments (to line up with the base play's) and, separately, its judgement totals. */
type CompareTarget = {
    segments: { side: CompareSide; counts: readonly SegmentCounts[]; scoreId: string } | null
    totals: { side: CompareSide; judgements: ReturnType<typeof totalsOf>; scoreId: string } | null
}

const publicBestSide = (params: { who: CompareSide['who']; loginId: string; scoreId: string | null; row: PublicBestRow }): CompareSide => ({
    who: params.who,
    login_id: params.loginId,
    score_id: params.scoreId,
    played_at: params.row.playedAt,
    lamp: clearLampFromId(params.row.clear),
    ex_score: params.row.exScore,
    random: params.row.random,
    scratch_auto: params.row.scratchAuto,
})

export const createAiPlayCompareService = (deps: AiPlayCompareServiceDeps) => {
    const ownTarget = async (userId: string, loginId: string, row: PlayScoreRow): Promise<CompareTarget | null> => {
        const best = await deps.plays.bestAnalyzedPlayOnChart({
            userId,
            chartSha256: row.chartSha256,
            scratchAuto: row.scratchAuto,
            excludeScoreId: row.scoreId,
        })
        const bestSegments = baseSegmentsOf(best)
        if (best && best.summary.metrics && bestSegments) {
            const side = {
                who: 'me',
                login_id: loginId,
                score_id: best.scoreId,
                played_at: best.playedAt,
                lamp: clearLampFromId(best.score.clear),
                ex_score: best.score.exScore,
                random: best.score.random,
                scratch_auto: best.score.scratchAuto,
            } satisfies CompareSide
            return {
                segments: { side, counts: bestSegments, scoreId: best.scoreId },
                totals: { side, judgements: totalsOf({ ...best.score, ...best.summary.metrics.overall }), scoreId: best.scoreId },
            }
        }
        const publicBest = await deps.db.findPublicBest({ userId, chartSha256: row.chartSha256, scratchAuto: row.scratchAuto })
        if (!publicBest || publicBest.scoreId === row.scoreId) return null
        return {
            segments: null,
            totals: {
                side: publicBestSide({ who: 'me', loginId, scoreId: publicBest.scoreId, row: publicBest }),
                judgements: totalsOf(publicBest),
                scoreId: publicBest.scoreId,
            },
        }
    }

    /**
     * A rival's side is one of two plays that need not be the same: the best traced play whose segment totals they share (the segment
     * comparison), and their public best (`chart_best`, the judgement totals). The caller shows only the one it compares, so the header
     * and the table always name the same play.
     */
    const rivalTarget = async (
        userId: string,
        rival: RivalRow,
        row: PlayScoreRow,
    ): Promise<{ target: CompareTarget; reason: AiCompareReason | null }> => {
        const [shared, publicBest] = await Promise.all([
            deps.plays.rivalSegmentsOnChart({
                requesterId: userId,
                rivalId: rival.userId,
                chartSha256: row.chartSha256,
                scratchAuto: row.scratchAuto,
            }),
            deps.db.findPublicBest({ userId: rival.userId, chartSha256: row.chartSha256, scratchAuto: row.scratchAuto }),
        ])
        const sharedPlay: RivalSegmentPlay | null = shared.allowed ? shared.play : null
        const target = {
            segments: sharedPlay
                ? {
                      side: {
                          who: 'rival',
                          login_id: rival.loginId,
                          score_id: null,
                          played_at: sharedPlay.playedAt,
                          lamp: clearLampFromId(sharedPlay.clear),
                          ex_score: sharedPlay.exScore,
                          random: sharedPlay.random,
                          scratch_auto: sharedPlay.scratchAuto,
                      },
                      counts: sharedPlay.segments,
                      scoreId: sharedPlay.scoreId,
                  }
                : null,
            totals: publicBest
                ? {
                      side: publicBestSide({ who: 'rival', loginId: rival.loginId, scoreId: null, row: publicBest }),
                      judgements: totalsOf(publicBest),
                      scoreId: publicBest.scoreId,
                  }
                : null,
        } satisfies CompareTarget
        if (!target.segments && !target.totals) return { target, reason: 'rival_no_record' }
        if (target.segments) return { target, reason: null }
        return { target, reason: shared.allowed ? 'rival_no_trace' : 'rival_not_sharing' }
    }

    const resolveTarget = async (
        params: { userId: string; query: AiCompareQuery },
        row: PlayScoreRow,
        rivals: readonly RivalRow[],
        loginId: string,
    ): Promise<{ target: CompareTarget | null; reason: AiCompareReason | null }> => {
        if (params.query.against === 'best') {
            const target = await ownTarget(params.userId, loginId, row)
            return { target, reason: target ? null : 'no_other_play' }
        }
        const rival = rivals.find((candidate) => candidate.loginId === params.query.rival)
        if (!rival) return { target: null, reason: 'not_a_rival' }
        return rivalTarget(params.userId, rival, row)
    }

    /**
     * Plan 5.3's deterministic comparison of an own play with the player's other best play on the chart or a rival's. Segment
     * differences need both plays' analyses, and for a rival their permission (ADR Q4: only the segment totals of a rival who turned
     * `rival_segments` on); otherwise the public judgement totals are set side by side (`totals`). Null when the account has no such
     * score.
     */
    const build = async (params: { userId: string; scoreId: string; query: AiCompareQuery }) => {
        const row = await deps.db.findOwnedPlayScore(params)
        if (!row) return null
        const [basePlay, rivals, loginId] = await Promise.all([
            deps.plays.analyzedPlayOfScore(params),
            deps.db.listRivals(params.userId),
            deps.db.findLoginId(params.userId),
        ])
        const base = {
            who: 'me' as const,
            login_id: loginId ?? '',
            score_id: row.scoreId,
            played_at: row.playedAt,
            lamp: clearLampFromId(row.clear),
            ex_score: row.exScore,
            random: row.random,
            scratch_auto: row.scratchAuto,
        }
        const resolved = await resolveTarget(params, row, rivals, loginId ?? '')
        const baseSegments = baseSegmentsOf(basePlay)
        const compared = baseSegments && resolved.target?.segments ? compareSegments(baseSegments, resolved.target.segments.counts) : null
        const hasSegments = compared !== null
        const shown = hasSegments ? resolved.target?.segments : resolved.target?.totals
        const totals = !hasSegments && resolved.target?.totals ? { base: totalsOf(row), target: resolved.target.totals.judgements } : null
        const layoutReason: AiCompareReason | null = baseSegments && resolved.target?.segments && !hasSegments ? 'note_layout_differs' : null
        const mode = modeOf({ hasSegments, hasTotals: totals !== null })
        const segments = compared ?? []
        const gaps = pickGapSegments(segments)
        const reachedDiffs = segments.flatMap((segment) => (segment.ex_diff === null ? [] : [segment.ex_diff]))
        const digest =
            hasSegments && shown
                ? {
                      base: { key: 'base', ex_score: base.ex_score, lamp: base.lamp, random: base.random, scratch_auto: base.scratch_auto },
                      target: targetDigestOf(shown.side),
                      overall: {
                          key: 'overall',
                          ex_diff: reachedDiffs.reduce((sum, diff) => sum + diff, 0),
                          compared_segments: reachedDiffs.length,
                      },
                      gaps: segments
                          .filter((segment) => gaps.includes(segment.id))
                          .map(({ key, id, from_ms, to_ms, ex_diff, bp_diff, cumulative_ex_diff, top_tags }) => ({
                              key,
                              segment_id: id,
                              from_ms,
                              to_ms,
                              ex_diff,
                              bp_diff,
                              cumulative_ex_diff,
                              top_tags,
                          })),
                  }
                : null
        const reason = layoutReason ?? (hasSegments ? null : resolved.reason)
        const view = {
            against: params.query.against,
            base,
            target: shown?.side ?? null,
            mode,
            reason,
            segments,
            gap_segments: gaps,
            totals,
            rivals: rivals.map((candidate) => ({ login_id: candidate.loginId, name: candidate.name, allows_segments: candidate.allowsSegments })),
            notice: basePlay ? null : { need: 'play_trace' },
        }
        const subjectKey = shown ? compareSubjectKey({ ...params, against: params.query.against, targetScoreId: shown.scoreId }) : null
        return { view, digest: digest && gaps.length > 0 ? digest : null, subjectKey, gapIds: new Set(gaps) }
    }

    return {
        /** The comparison and its stored explanation; null when the account has no such score. No provider is called. */
        view: async (params: { userId: string; scoreId: string; query: AiCompareQuery }) => {
            const built = await build(params)
            if (!built) return null
            const ai =
                built.digest && built.subjectKey
                    ? await readStoredInterpretation({
                          results: deps.results,
                          spec: AI_PLAY_COMPARE_SPEC,
                          userId: params.userId,
                          subjectKey: built.subjectKey,
                          digest: built.digest,
                      })
                    : null
            return { ...built.view, ai } satisfies AiCompareView
        },

        /** Asks the user's AI to explain the segment differences; with no segment comparison (or no difference) no call is made. */
        interpret: async (params: { userId: string; scoreId: string; query: AiCompareQuery }) => {
            const built = await build(params)
            if (!built) return null
            if (!built.digest || !built.subjectKey) return notEnoughData('comparable_play')
            const { digest, gapIds } = built
            return runAiFeature(deps.runner, {
                ...AI_PLAY_COMPARE_SPEC,
                userId: params.userId,
                subjectKey: built.subjectKey,
                digest,
                refine: (output) => refineCompareOutput(output, gapIds),
            })
        },
    }
}

export type AiPlayCompareService = ReturnType<typeof createAiPlayCompareService>

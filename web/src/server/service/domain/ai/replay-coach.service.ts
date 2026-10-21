import { AI_TEXT_LIMIT, type AiEffort } from '@server/dto/ai'
import { aiCoachOutputSchema, aiGaugeNameOf, type AiCoachOutput, type AiCoachView } from '@server/dto/ai-play'
import { clearLampFromId, gaugeTypeFromId } from '@server/dto/common'
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
import { COACH_PROBLEM_SEGMENTS, PLAY_FEATURE_ANALYSIS_VERSION } from '@server/service/domain/ai/play-feature.constants'
import { exRatioOf, isShownTag, segmentKey } from '@server/service/domain/ai/play-digest'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import { deriveAssistGrade } from '@server/service/domain/score/assist-grade'
import type { AnalyzedPlay, TraceService } from '@server/service/domain/trace/trace.service'

export const AI_REPLAY_COACH_FEATURE = 'replay_coach'
export const AI_REPLAY_COACH_PROMPT_VERSION = 1
export const AI_REPLAY_COACH_MAX_OUTPUT_TOKENS = 8_000
export const AI_REPLAY_COACH_EFFORT = 'low' satisfies AiEffort

export const AI_REPLAY_COACH_SYSTEM_PROMPT = [
    "You coach a rhythm game (BMS, 7 keys and a scratch) player through one play, segment by segment, from the server's analysis of it.",
    DIGEST_IS_DATA_RULE,
    'The play is cut into equal time segments. `problems` are the segments where the player lost the most (BP is BAD plus POOR plus empty POOR), each with its main pattern tags, FAST/SLOW of GREAT to BAD, the gauge at the end of the segment before it and any gauge drop inside it.',
    `For each problem segment you talk about write \`segment_id\` exactly as given, one Korean sentence \`diagnosis\` and one Korean sentence \`advice\`; at most ${COACH_PROBLEM_SEGMENTS}.`,
    'Pattern tags describe the notes as the player saw them after lane options. `next_try` is one Korean sentence for the next attempt of this chart.',
    DIGEST_KEY_RULE,
    'All text is Korean; `summary` is two sentences at most.',
].join('\n')

export const AI_REPLAY_COACH_SPEC = {
    feature: AI_REPLAY_COACH_FEATURE,
    analysisVersion: PLAY_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_REPLAY_COACH_PROMPT_VERSION,
    system: AI_REPLAY_COACH_SYSTEM_PROMPT,
    schemaName: 'replay_coach',
    schema: aiCoachOutputSchema,
    maxOutputTokens: AI_REPLAY_COACH_MAX_OUTPUT_TOKENS,
    effort: AI_REPLAY_COACH_EFFORT,
} satisfies AiFeatureSpec<typeof aiCoachOutputSchema>

/** The account's own score row of one play with its chart's title, what the coach and the comparison show of the play. */
export type PlayScoreRow = {
    scoreId: string
    playedAt: number
    chartSha256: string
    chartMd5: string | null
    title: string
    artist: string
    clear: number
    exScore: number
    maxExScore: number
    minbp: number
    random: string
    gauge: number
    scratchAuto: boolean
    pgreat: number
    great: number
    good: number
    bad: number
    poor: number
}

export type PlayScoreDb = {
    /** The score `scoreId` names when the account owns it; another account's score is as absent as a missing one. */
    findOwnedPlayScore: (params: { userId: string; scoreId: string }) => Promise<PlayScoreRow | null>
}

type AiReplayCoachServiceDeps = {
    db: PlayScoreDb
    plays: Pick<TraceService, 'analyzedPlayOfScore'>
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
}

export const coachSubjectKey = (scoreId: string) => `score:${scoreId}`

export const toPlayView = (row: PlayScoreRow) => ({
    score_id: row.scoreId,
    played_at: row.playedAt,
    chart: { sha256: row.chartSha256, md5: row.chartMd5, title: row.title, artist: row.artist },
    lamp: clearLampFromId(row.clear),
    assist_grade: deriveAssistGrade(row),
    ex_score: row.exScore,
    max_ex_score: row.maxExScore,
    minbp: row.minbp,
    random: row.random,
    gauge: gaugeTypeFromId(row.gauge),
    scratch_auto: row.scratchAuto,
})

/**
 * Plan 5.2's deterministic part for one analysed play: every segment with its BP, combo breaks, FAST/SLOW, EX ratio, main tags and
 * the played gauge at its end; the played gauge's drops; how each gauge ended (`unknown` past an empty gauge's failure); and the
 * problem segments the AI is told about: the highest BP first (the earlier on a tie), then segments where a drop ends, at most
 * COACH_PROBLEM_SEGMENTS. Null for a play with no per-note figures (a mode other than 7-key single play).
 */
export const computeCoachAnalysis = (play: AnalyzedPlay) => {
    const { metrics, gauge } = play.summary
    if (!metrics) return null
    const segments = metrics.segments.map((segment, index) => ({
        key: segmentKey(segment.id),
        id: segment.id,
        from_ms: segment.from_ms,
        to_ms: segment.to_ms,
        chart_notes: segment.chart_notes,
        notes: segment.notes,
        ex_ratio: exRatioOf(segment),
        bp: segment.bp,
        combo_breaks: segment.combo_breaks,
        fast: segment.fast,
        slow: segment.slow,
        top_tags: segment.top_tags.filter((tag) => isShownTag(tag.tag)),
        gauge: gauge.segment_values[index] ?? null,
    }))
    const dropEndsIn = (segment: (typeof segments)[number]) => gauge.drops.some((drop) => drop.to_ms >= segment.from_ms && drop.to_ms < segment.to_ms)
    const byBp = segments.filter((segment) => segment.bp > 0).toSorted((left, right) => right.bp - left.bp)
    const withDrop = segments.filter((segment) => segment.bp === 0 && dropEndsIn(segment))
    const problems = [...byBp, ...withDrop].slice(0, COACH_PROBLEM_SEGMENTS).map((segment) => segment.id)
    return {
        run_end: play.summary.run_end,
        overall: { notes: metrics.overall.notes, ex_ratio: metrics.overall.ex_ratio },
        segments,
        problem_segments: problems,
        drops: gauge.drops.map((drop, index) => ({ key: `drop.${index}`, ...drop })),
        gauge: {
            selected: aiGaugeNameOf(gauge.selected),
            interval_ms: gauge.interval_ms,
            curves: gauge.curves.flatMap((curve, index) => {
                const name = aiGaugeNameOf(index)
                return name ? [{ gauge: name, end: curve.end, cleared: curve.cleared, outcome: curve.outcome }] : []
            }),
        },
        pattern_join: play.summary.pattern.join,
    }
}

export type CoachAnalysis = NonNullable<ReturnType<typeof computeCoachAnalysis>>

/** The problem segments with the gauge before each and its drop, and the play's conditions; the chart title is data only. */
export const toCoachDigest = (row: PlayScoreRow, analysis: CoachAnalysis) => {
    const byId = new Map(analysis.segments.map((segment, index) => [segment.id, { segment, before: analysis.segments[index - 1] ?? null }]))
    return {
        chart: { title: row.title, artist: row.artist },
        play: {
            key: 'play',
            lamp: clearLampFromId(row.clear),
            ex_ratio: analysis.overall.ex_ratio,
            minbp: row.minbp,
            random: row.random,
            gauge: gaugeTypeFromId(row.gauge),
            scratch_auto: row.scratchAuto,
            run_end: analysis.run_end,
        },
        problems: analysis.problem_segments.flatMap((id) => {
            const found = byId.get(id)
            if (!found) return []
            const { segment, before } = found
            const drop = analysis.drops.find((candidate) => candidate.to_ms >= segment.from_ms && candidate.to_ms < segment.to_ms)
            return [
                {
                    key: segment.key,
                    segment_id: segment.id,
                    from_ms: segment.from_ms,
                    to_ms: segment.to_ms,
                    bp: segment.bp,
                    combo_breaks: segment.combo_breaks,
                    fast: segment.fast,
                    slow: segment.slow,
                    ex_ratio: segment.ex_ratio,
                    top_tags: segment.top_tags,
                    gauge_before: before?.gauge ?? null,
                    gauge_drop: drop?.drop ?? null,
                },
            ]
        }),
        other_gauges: analysis.gauge.curves
            .filter((curve) => curve.gauge !== analysis.gauge.selected)
            .map(({ gauge, outcome }) => ({ gauge, outcome })),
    }
}

export type CoachDigest = ReturnType<typeof toCoachDigest>

/** Keeps the segment notes on problem segments that fit one line; the answer is rejected when a problem was named and none is left. */
export const refineCoachOutput = (output: AiCoachOutput, digest: CoachDigest) => {
    const problems = new Set(digest.problems.map((problem) => problem.segment_id))
    const segments = output.segments
        .filter(
            (item) =>
                problems.has(item.segment_id) &&
                fitsLine(item.diagnosis, AI_TEXT_LIMIT.REASON_MAX_CHARS) &&
                fitsLine(item.advice, AI_TEXT_LIMIT.REASON_MAX_CHARS),
        )
        .slice(0, COACH_PROBLEM_SEGMENTS)
        .map((item) => ({ segment_id: item.segment_id, diagnosis: item.diagnosis.trim(), advice: item.advice.trim() }))
    if (output.summary.trim() === '' || (problems.size > 0 && segments.length === 0)) return null
    return {
        summary: cutText(output.summary, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
        segments,
        next_try: cutText(output.next_try, AI_TEXT_LIMIT.REASON_MAX_CHARS),
    }
}

export const createAiReplayCoachService = (deps: AiReplayCoachServiceDeps) => {
    const build = async (params: { userId: string; scoreId: string }) => {
        const row = await deps.db.findOwnedPlayScore(params)
        if (!row) return null
        const play = await deps.plays.analyzedPlayOfScore(params)
        const analysis = play ? computeCoachAnalysis(play) : null
        const notice = analysis ? null : { need: play ? 'play_metrics' : 'play_trace' }
        return { row, analysis, notice, digest: analysis ? toCoachDigest(row, analysis) : null }
    }

    return {
        /** One own play's segments and the stored coaching; null when the account has no such score. No provider is called. */
        view: async (params: { userId: string; scoreId: string }) => {
            const built = await build(params)
            if (!built) return null
            const ai = built.digest
                ? await readStoredInterpretation({
                      results: deps.results,
                      spec: AI_REPLAY_COACH_SPEC,
                      userId: params.userId,
                      subjectKey: coachSubjectKey(params.scoreId),
                      digest: built.digest,
                  })
                : null
            return { play: toPlayView(built.row), analysis: built.analysis, notice: built.notice, ai } satisfies AiCoachView
        },

        /** Asks the user's AI to coach the play; a play with no trace (or no per-note figures) makes no call. Null for no such score. */
        interpret: async (params: { userId: string; scoreId: string }) => {
            const built = await build(params)
            if (!built) return null
            if (!built.digest) return notEnoughData(built.notice?.need ?? 'play_trace')
            const digest = built.digest
            return runAiFeature(deps.runner, {
                ...AI_REPLAY_COACH_SPEC,
                userId: params.userId,
                subjectKey: coachSubjectKey(params.scoreId),
                digest,
                refine: (output) => refineCoachOutput(output, digest),
            })
        },
    }
}

export type AiReplayCoachService = ReturnType<typeof createAiReplayCoachService>

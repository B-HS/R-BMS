import { AI_TEXT_LIMIT, type AiEffort } from '@server/dto/ai'
import {
    AI_GAUGE_NAMES,
    aiGaugeAdviceOutputSchema,
    aiGaugeNameOf,
    aiOptionNameOf,
    type AiGaugeAdviceOutput,
    type AiGaugeAdviceView,
    type AiGaugeName,
} from '@server/dto/ai-play'
import { isClearLampId } from '@server/service/domain/ai/table-progress'
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
import {
    GAUGE_ADVICE_OPTION_MIN_PLAYS,
    GAUGE_ADVICE_OPTION_TAGS,
    GAUGE_ADVICE_RECENT_PLAYS,
    PLAY_FEATURE_ANALYSIS_VERSION,
    PLAY_REPORT_MIN_PLAYS,
} from '@server/service/domain/ai/play-feature.constants'
import { exRatioOf, isShownTag, playsWithMetrics } from '@server/service/domain/ai/play-digest'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import { GAUGE_OUTCOME, RUN_END } from '@server/service/domain/analysis/play-metrics'
import { LAYOUT_DEPENDENT_TAGS } from '@server/service/domain/analysis/pattern-tags'
import { aggregateTagEvidence } from '@server/service/domain/analysis/sample'
import type { AnalyzedPlay, TraceService } from '@server/service/domain/trace/trace.service'

export const AI_GAUGE_ADVICE_FEATURE = 'gauge_advice'
export const AI_GAUGE_ADVICE_PROMPT_VERSION = 1
export const AI_GAUGE_ADVICE_MAX_OUTPUT_TOKENS = 6_000
export const AI_GAUGE_ADVICE_EFFORT = 'low' satisfies AiEffort
export const AI_GAUGE_ADVICE_SUBJECT = 'recent'
export const AI_GAUGE_ADVICE_ITEMS_MAX = 5

const UNKNOWN_LEVEL = '?'

export const AI_GAUGE_ADVICE_SYSTEM_PROMPT = [
    'You advise a rhythm game (BMS) player which gauge and which lane option to choose, from how their recent plays ended.',
    DIGEST_IS_DATA_RULE,
    'The game logs all nine gauges during a play, so `rescues` counts plays that failed on the gauge played but that another gauge would have cleared (`rescued_by`). After an empty gauge ends a play the other gauges cannot be known; those plays are counted in `unknown_runs` and never as rescued.',
    '`options` lists each lane option with the EX ratio of its plays and the pattern tags whose residual (the tag notes\' EX ratio less the whole play\'s) moves most under it; a tag with grade other than "strong" is only a hint.',
    `Write at most ${AI_GAUGE_ADVICE_ITEMS_MAX} \`gauge_advice\` items and ${AI_GAUGE_ADVICE_ITEMS_MAX} \`option_advice\` items, each naming a gauge or an option that appears in the input and one Korean sentence \`reasoning\`.`,
    DIGEST_KEY_RULE,
    'All text is Korean; `summary` is two sentences at most.',
].join('\n')

export const AI_GAUGE_ADVICE_SPEC = {
    feature: AI_GAUGE_ADVICE_FEATURE,
    analysisVersion: PLAY_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_GAUGE_ADVICE_PROMPT_VERSION,
    system: AI_GAUGE_ADVICE_SYSTEM_PROMPT,
    schemaName: 'gauge_option_advice',
    schema: aiGaugeAdviceOutputSchema,
    maxOutputTokens: AI_GAUGE_ADVICE_MAX_OUTPUT_TOKENS,
    effort: AI_GAUGE_ADVICE_EFFORT,
} satisfies AiFeatureSpec<typeof aiGaugeAdviceOutputSchema>

export type AiGaugeAdviceServiceDb = {
    /** The gauge and lamp of the account's newest score rows (autoplay left out), at most `limit`. */
    listRecentGaugeRows: (params: { userId: string; limit: number }) => Promise<{ gauge: number; clear: number }[]>
}

type AiGaugeAdviceServiceDeps = {
    db: AiGaugeAdviceServiceDb
    plays: Pick<TraceService, 'recentAnalyzedPlays'>
    levelLabelOf: (md5: string) => string | null
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
}

const isLayoutTag = (tag: string) => (LAYOUT_DEPENDENT_TAGS as readonly string[]).includes(tag)

const playedOutcomeOf = (play: AnalyzedPlay) => play.summary.gauge.curves[play.summary.gauge.selected]?.outcome ?? null

/**
 * Plan 5.8's deterministic part from the newest analysed plays: each played gauge with its clears; per level and played gauge, the
 * failed plays and which other gauges would have cleared them (the client's own curves, read up to an empty gauge's failure, so a
 * gauge that cannot be known is never counted); and per lane option the plays, their pooled EX ratio and the lane-dependent tags
 * whose residual moves most (from GAUGE_ADVICE_OPTION_MIN_PLAYS plays of the option).
 */
export const computeGaugeAdvice = (plays: readonly AnalyzedPlay[], levelLabelOf: (md5: string) => string | null) => {
    const gaugeRows = AI_GAUGE_NAMES.flatMap((gauge, index) => {
        const played = plays.filter((play) => play.summary.gauge.selected === index)
        if (played.length === 0) return []
        const cleared = played.filter((play) => playedOutcomeOf(play) === GAUGE_OUTCOME.CLEARED).length
        return [{ key: `gauge.${gauge}`, gauge, plays: played.length, cleared }]
    })
    const failed = plays
        .filter((play) => playedOutcomeOf(play) === GAUGE_OUTCOME.FAILED)
        .map((play) => ({ play, level: (play.chartMd5 ? levelLabelOf(play.chartMd5) : null) ?? UNKNOWN_LEVEL, played: play.summary.gauge.selected }))
    const groups = Map.groupBy(failed, (entry) => `${entry.level}|${entry.played}`)
    const rescues = [...groups.values()].flatMap((group) => {
        const first = group[0]
        const played = first ? aiGaugeNameOf(first.played) : null
        if (!first || !played) return []
        const rescuedBy = AI_GAUGE_NAMES.flatMap((gauge, index) => {
            const count = group.filter(({ play }) => play.summary.gauge.curves[index]?.outcome === GAUGE_OUTCOME.CLEARED).length
            return index === first.played || count === 0 ? [] : [{ gauge, plays: count }]
        })
        return [{ key: `rescue.${first.level}.${played}`, level: first.level, played, failed: group.length, rescued_by: rescuedBy }]
    })
    const measured = playsWithMetrics(plays)
    const options = [...Map.groupBy(measured, ({ play }) => play.score.random).entries()].flatMap(([random, inOption]) => {
        const option = aiOptionNameOf(random)
        if (!option) return []
        const pool = inOption.reduce(
            (sum, { metrics }) => ({ notes: sum.notes + metrics.overall.notes, ex_score: sum.ex_score + metrics.overall.ex_score }),
            { notes: 0, ex_score: 0 },
        )
        const evidence =
            inOption.length >= GAUGE_ADVICE_OPTION_MIN_PLAYS
                ? aggregateTagEvidence(
                      inOption.map(({ play, metrics }) => ({
                          scratchAuto: play.summary.scratch_auto,
                          overall: metrics.overall,
                          byTag: metrics.by_tag,
                      })),
                  )
                : {}
        const tags = Object.entries(evidence)
            .flatMap(([tag, row]) =>
                row && row.notes > 0 && isLayoutTag(tag) && isShownTag(tag)
                    ? [{ tag, residual: row.residual, grade: row.grade, notes: row.notes }]
                    : [],
            )
            .toSorted((left, right) => Math.abs(right.residual) - Math.abs(left.residual))
            .slice(0, GAUGE_ADVICE_OPTION_TAGS)
        return [{ key: `option.${option}`, option, plays: inOption.length, ex_ratio: exRatioOf(pool), tags }]
    })
    return {
        plays: { analyzed: plays.length, unknown_runs: plays.filter((play) => play.summary.run_end === RUN_END.GAUGE_EMPTY).length },
        gauges: gaugeRows,
        rescues,
        options,
    }
}

export type GaugeAdvice = ReturnType<typeof computeGaugeAdvice>

/** The score rows' gauges and lamps, what is left to show for plays with no trace (plan 5.8 "데이터 없음"). */
export const summarizeGaugeRows = (rows: readonly { gauge: number; clear: number }[]) =>
    AI_GAUGE_NAMES.flatMap((gauge, index) => {
        const played = rows.filter((row) => row.gauge === index)
        return played.length === 0 ? [] : [{ gauge, plays: played.length, cleared: played.filter((row) => isClearLampId(row.clear)).length }]
    })

export const toGaugeAdviceDigest = (advice: GaugeAdvice) => ({
    plays: { key: 'plays', ...advice.plays },
    gauges: advice.gauges,
    rescues: advice.rescues,
    options: advice.options,
})

export type GaugeAdviceDigest = ReturnType<typeof toGaugeAdviceDigest>

/** Keeps advice on gauges and options the digest names, one fitting line each; an answer with neither left is rejected. */
export const refineGaugeAdviceOutput = (output: AiGaugeAdviceOutput, digest: GaugeAdviceDigest) => {
    const gauges = new Set<AiGaugeName>([
        ...digest.gauges.map((row) => row.gauge),
        ...digest.rescues.flatMap((row) => [row.played, ...row.rescued_by.map((rescue) => rescue.gauge)]),
    ])
    const options = new Set(digest.options.map((row) => row.option))
    const gaugeAdvice = output.gauge_advice
        .filter((item) => gauges.has(item.gauge) && fitsLine(item.reasoning, AI_TEXT_LIMIT.REASON_MAX_CHARS))
        .slice(0, AI_GAUGE_ADVICE_ITEMS_MAX)
        .map((item) => ({ gauge: item.gauge, reasoning: item.reasoning.trim() }))
    const optionAdvice = output.option_advice
        .filter((item) => options.has(item.option) && fitsLine(item.reasoning, AI_TEXT_LIMIT.REASON_MAX_CHARS))
        .slice(0, AI_GAUGE_ADVICE_ITEMS_MAX)
        .map((item) => ({ option: item.option, reasoning: item.reasoning.trim() }))
    if (output.summary.trim() === '' || gaugeAdvice.length + optionAdvice.length === 0) return null
    return { summary: cutText(output.summary, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS), gauge_advice: gaugeAdvice, option_advice: optionAdvice }
}

export const createAiGaugeAdviceService = (deps: AiGaugeAdviceServiceDeps) => {
    const build = async (userId: string) => {
        const [{ plays, stale, failed }, gaugeRows] = await Promise.all([
            deps.plays.recentAnalyzedPlays({ userId, limit: GAUGE_ADVICE_RECENT_PLAYS }),
            deps.db.listRecentGaugeRows({ userId, limit: GAUGE_ADVICE_RECENT_PLAYS }),
        ])
        const advice = computeGaugeAdvice(plays, deps.levelLabelOf)
        return { advice, stale, failed, scoreRows: summarizeGaugeRows(gaugeRows), digest: toGaugeAdviceDigest(advice) }
    }

    const noticeOf = (advice: GaugeAdvice) => (advice.plays.analyzed < PLAY_REPORT_MIN_PLAYS ? { need: 'traced_plays' } : null)

    return {
        /** The deterministic gauge and option figures and the stored advice. No provider is called. */
        view: async (userId: string) => {
            const built = await build(userId)
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: AI_GAUGE_ADVICE_SPEC,
                userId,
                subjectKey: AI_GAUGE_ADVICE_SUBJECT,
                digest: built.digest,
            })
            return {
                ...built.advice,
                plays: { ...built.advice.plays, stale: built.stale, failed: built.failed },
                score_rows: built.scoreRows,
                notice: noticeOf(built.advice),
                ai,
            } satisfies AiGaugeAdviceView
        },

        /** Asks the user's AI for gauge and option advice; fewer than PLAY_REPORT_MIN_PLAYS analysed plays make no call. */
        interpret: async (userId: string) => {
            const built = await build(userId)
            const notice = noticeOf(built.advice)
            if (notice) return notEnoughData(notice.need)
            return runAiFeature(deps.runner, {
                ...AI_GAUGE_ADVICE_SPEC,
                userId,
                subjectKey: AI_GAUGE_ADVICE_SUBJECT,
                digest: built.digest,
                refine: (output) => refineGaugeAdviceOutput(output, built.digest),
            })
        },
    }
}

export type AiGaugeAdviceService = ReturnType<typeof createAiGaugeAdviceService>

import type { TraceUploadInput } from '@server/dto/trace'
import { finishTally, type Tally } from '@server/service/domain/analysis/play-metrics'
import { analyzeTrace } from '@server/service/domain/trace/trace-analysis'
import type { AnalyzedPlay, TracedPlayScore } from '@server/service/domain/trace/trace.service'
import { DEFAULT_PLAY_SCORE } from './trace-fixture'

type Counts = { pgreat?: number; great?: number; good?: number; bad?: number; poor?: number; fast?: number; slow?: number }

/** A finished tally of these judgement counts; `fast` and `slow` are GREATs early and late. */
export const tallyOf = (counts: Counts): Tally =>
    finishTally({
        pgreat: counts.pgreat ?? 0,
        great: counts.great ?? 0,
        good: counts.good ?? 0,
        bad: counts.bad ?? 0,
        poor: counts.poor ?? 0,
        fast: { pgreat: 0, great: counts.fast ?? 0, good: 0, bad: 0 },
        slow: { pgreat: 0, great: counts.slow ?? 0, good: 0, bad: 0 },
    })

/** One analysed play of `trace` as the trace service hands it out, its summary computed by the real analysis. */
export const analyzedPlayOf = (
    trace: TraceUploadInput,
    options: { scoreId?: string; playedAt?: number; chartMd5?: string | null; score?: Partial<TracedPlayScore> } = {},
): AnalyzedPlay => ({
    traceId: `tr_${options.scoreId ?? trace.score_id}`,
    scoreId: options.scoreId ?? trace.score_id,
    chartSha256: trace.chart.sha256,
    chartMd5: options.chartMd5 === undefined ? trace.chart.md5 || null : options.chartMd5,
    playedAt: options.playedAt ?? 0,
    score: { ...DEFAULT_PLAY_SCORE, ...options.score },
    analysis: null,
    summary: analyzeTrace({ trace, chartSha256: trace.chart.sha256, pattern: null }).summary,
})

type Metrics = NonNullable<AnalyzedPlay['summary']['metrics']>

/** The play with parts of its per-play figures replaced; a play with none (not 7-key single play) is left as it is. */
export const withMetrics = (play: AnalyzedPlay, patch: Partial<Metrics>): AnalyzedPlay => {
    const metrics = play.summary.metrics
    return metrics ? { ...play, summary: { ...play.summary, metrics: { ...metrics, ...patch } } } : play
}

/** The play with its gauge summary replaced. */
export const withGauge = (play: AnalyzedPlay, patch: Partial<AnalyzedPlay['summary']['gauge']>, runEnd = play.summary.run_end): AnalyzedPlay => ({
    ...play,
    summary: { ...play.summary, run_end: runEnd, gauge: { ...play.summary.gauge, ...patch } },
})

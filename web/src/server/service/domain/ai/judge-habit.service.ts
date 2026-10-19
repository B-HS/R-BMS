import { AI_TEXT_LIMIT, aiJudgeHabitOutputSchema, type AiEffort, type AiJudgeHabitOutput, type AiJudgeHabitView } from '@server/dto/ai'
import {
    DIGEST_IS_DATA_RULE,
    DIGEST_KEY_RULE,
    collectDigestKeys,
    cutText,
    fitsLine,
    notEnoughData,
    readStoredInterpretation,
    runAiFeature,
    type AiFeatureSpec,
} from '@server/service/domain/ai/ai-digest'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import {
    AVGJUDGE_MIN_HALF_PLAYS,
    GREEN_BUCKET_WIDTH,
    JUDGE_HABIT_MIN_FAST_SLOW,
    JUDGE_HABIT_RECENT_PLAYS,
    MILLIS_DECIMALS,
    OFFSET_SUGGEST_MIN_MS,
    PERCENT_DECIMALS,
    REFERENCE_RULE_VERSION,
    SCORE_FEATURE_ANALYSIS_VERSION,
} from '@server/service/domain/ai/score-feature.constants'
import { GREEN_DISPLAY_FACTOR, MIN_TIMING_PLAYS } from '@server/service/domain/analysis/analysis-constants'
import { roundRatio } from '@server/service/domain/analysis/chart-features'
import { describeGreenNumber, readTraceGreenNumber } from '@server/service/domain/analysis/play-metrics'
import { EVIDENCE, timingEvidence, wilsonInterval } from '@server/service/domain/analysis/sample'
import { PLAY_LANE_BIAS_TOP } from '@server/service/domain/ai/play-feature.constants'
import { WILSON_Z_95 } from '@server/service/domain/analysis/analysis-constants'

export const AI_JUDGE_HABIT_FEATURE = 'judge_habit'
export const AI_JUDGE_HABIT_PROMPT_VERSION = 2
export const AI_JUDGE_HABIT_MAX_OUTPUT_TOKENS = 8_000
export const AI_JUDGE_HABIT_EFFORT = 'low' satisfies AiEffort
export const AI_JUDGE_HABIT_NOTES_MAX = 5
export const AI_JUDGE_HABIT_SUBJECT = 'recent'

const MICROS_PER_MILLI = 1000
const PERCENT = 100
const EVEN_SHARE = 0.5
const MEDIAN_HALF = 2

export const AI_JUDGE_HABIT_SYSTEM_PROMPT = [
    'You explain the timing habits of a rhythm game (BMS) player from the totals of their recent plays.',
    DIGEST_IS_DATA_RULE,
    '`fast_slow` counts early (FAST) and late (SLOW) judgments, `offset` is the average signed timing error (positive means early) with the server suggestion (`source` says whether it rests on note-by-note traces or on score rows), `trace_timing` is the note-by-note timing of PGREAT and GREAT from traces with its 95% interval and the lanes (as played) that stray furthest from it, `avgjudge` the average distance from the note in the older and newer half of the plays, `green` the GREEN numbers the plays used.',
    'JUDGE OFFSET moves only where notes are drawn, never the judgment: say "노트가 보이는 위치를 옮긴다", never that it moves the judgment. Raising it makes notes arrive earlier on screen, lowering it later.',
    'Set `offset_direction` to the `offset.direction` given; explain it in `offset_reasoning`. When `offset.grade` is not "strong", say the evidence is not enough.',
    'Write GREEN numbers as the GREEN value first with the lane time in milliseconds after it, for example "GREEN 300 (500 ms)". A green number with `is_estimated` true is worked out from the score row, not read from the skin: call it an estimate.',
    `Write at most ${AI_JUDGE_HABIT_NOTES_MAX} \`notes\`, each with the \`key\` of the digest item it rests on.`,
    DIGEST_KEY_RULE,
    'All text is Korean; `summary` is two sentences at most.',
].join('\n')

export const AI_JUDGE_HABIT_SPEC = {
    feature: AI_JUDGE_HABIT_FEATURE,
    analysisVersion: SCORE_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_JUDGE_HABIT_PROMPT_VERSION,
    system: AI_JUDGE_HABIT_SYSTEM_PROMPT,
    schemaName: 'judge_habit',
    schema: aiJudgeHabitOutputSchema,
    maxOutputTokens: AI_JUDGE_HABIT_MAX_OUTPUT_TOKENS,
    effort: AI_JUDGE_HABIT_EFFORT,
} satisfies AiFeatureSpec<typeof aiJudgeHabitOutputSchema>

/**
 * One recent play's timing columns. `greenNumber` is the lane travel time in milliseconds the score row keeps (`options.green_number`),
 * `traceGreen` the green value and lane time of the play's trace when one was uploaded.
 */
export type TimingScoreRow = {
    playedAt: number
    pgreat: number
    great: number
    good: number
    bad: number
    fast: number
    slow: number
    egr: number
    lgr: number
    egd: number
    lgd: number
    ebd: number
    lbd: number
    avgjudge: number
    meanOffsetUs: number | null
    ruleVersion: number | null
    autoOffset: boolean
    greenNumber: number | null
    traceGreen?: { green: number; greenMs: number } | null
    /** The PGREAT and GREAT timing of the play's trace (`metrics.timing`, `metrics.timing_by_lane`), when it has one. */
    traceTiming?: { notes: number; meanUs: number; stdevUs: number; byLane: Readonly<Record<string, { notes: number; mean_us: number }>> } | null
    constant: boolean
    exScore: number
    maxExScore: number
}

export type AiJudgeHabitServiceDb = {
    /** The user's newest plays first, at most `limit`, autoplay left out. */
    listRecentTimingRows: (params: { userId: string; limit: number }) => Promise<TimingScoreRow[]>
}

type AiJudgeHabitServiceDeps = {
    db: AiJudgeHabitServiceDb
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
}

const roundTo = (value: number, decimals: number) => {
    const scale = 10 ** decimals
    return Math.round(value * scale) / scale
}

const sum = (values: readonly number[]) => values.reduce((total, value) => total + value, 0)

const median = (values: readonly number[]) => {
    const sorted = values.toSorted((left, right) => left - right)
    const middle = Math.floor(sorted.length / MEDIAN_HALF)
    if (sorted.length === 0) return null
    return sorted.length % MEDIAN_HALF === 1 ? (sorted[middle] ?? null) : ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / MEDIAN_HALF
}

const greenOfRow = (row: TimingScoreRow) => {
    if (row.traceGreen) return readTraceGreenNumber(row.traceGreen)
    return (row.greenNumber ?? 0) > 0 ? describeGreenNumber(row.greenNumber ?? 0) : null
}

const isReference = (row: TimingScoreRow) => (row.ruleVersion ?? 0) >= REFERENCE_RULE_VERSION

const fastShareOf = (rows: readonly TimingScoreRow[]) => {
    const fast = sum(rows.map((row) => row.fast))
    const slow = sum(rows.map((row) => row.slow))
    return { fast, slow, share: fast + slow > 0 ? fast / (fast + slow) : null }
}

/** EX of the plays as a percent of their maximum, pooled (not a mean of percents). */
const pooledExRate = (rows: readonly TimingScoreRow[]) => {
    const max = sum(rows.map((row) => row.maxExScore))
    return max > 0 ? roundTo((sum(rows.map((row) => row.exScore)) / max) * PERCENT, PERCENT_DECIMALS) : null
}

const traceGradeOf = (isEnough: boolean, leavesOutZero: boolean) => {
    if (!isEnough) return EVIDENCE.INSUFFICIENT
    return leavesOutZero ? EVIDENCE.STRONG : EVIDENCE.UNCERTAIN
}

/**
 * The note-by-note timing of the plays that have a trace (AUTO ADJUST plays apart): the pooled mean of PGREAT and GREAT errors with
 * its 95% interval (pooled variance over the plays), graded `strong` only with MIN_TIMING_PLAYS plays and MIN_TIMING_NOTES notes and an
 * interval that leaves out 0, and the PLAY_LANE_BIAS_TOP lanes (as played) whose mean strays furthest from it.
 */
export const computeTraceTiming = (rows: readonly TimingScoreRow[]) => {
    const timed = rows.flatMap((row) => (row.traceTiming && row.traceTiming.notes > 0 && !row.autoOffset ? [row.traceTiming] : []))
    const notes = sum(timed.map((timing) => timing.notes))
    const meanUs = notes > 0 ? sum(timed.map((timing) => timing.meanUs * timing.notes)) / notes : null
    const variance = meanUs === null ? 0 : sum(timed.map((timing) => timing.notes * (timing.stdevUs ** 2 + (timing.meanUs - meanUs) ** 2))) / notes
    const margin = notes > 0 ? WILSON_Z_95 * Math.sqrt(variance / notes) : 0
    const leavesOutZero = meanUs !== null && (meanUs - margin > 0 || meanUs + margin < 0)
    const grade = traceGradeOf(timingEvidence({ plays: timed.length, notes }) === EVIDENCE.STRONG, leavesOutZero)
    const lanePools = new Map<string, { notes: number; total: number }>()
    for (const timing of timed) {
        for (const [lane, row] of Object.entries(timing.byLane)) {
            const pool = lanePools.get(lane) ?? { notes: 0, total: 0 }
            lanePools.set(lane, { notes: pool.notes + row.notes, total: pool.total + row.mean_us * row.notes })
        }
    }
    const lanes = [...lanePools.entries()]
        .map(([lane, pool]) => {
            const laneMeanUs = pool.total / pool.notes
            return {
                key: `timing.lane.${lane}`,
                lane,
                notes: pool.notes,
                mean_ms: roundTo(laneMeanUs / MICROS_PER_MILLI, MILLIS_DECIMALS),
                deviation_ms: roundTo((laneMeanUs - (meanUs ?? 0)) / MICROS_PER_MILLI, MILLIS_DECIMALS),
            }
        })
        .toSorted((left, right) => Math.abs(right.deviation_ms) - Math.abs(left.deviation_ms))
        .slice(0, PLAY_LANE_BIAS_TOP)
    return {
        plays: timed.length,
        notes,
        mean_ms: meanUs === null ? null : roundTo(meanUs / MICROS_PER_MILLI, MILLIS_DECIMALS),
        ci_low_ms: meanUs === null ? null : roundTo((meanUs - margin) / MICROS_PER_MILLI, MILLIS_DECIMALS),
        ci_high_ms: meanUs === null ? null : roundTo((meanUs + margin) / MICROS_PER_MILLI, MILLIS_DECIMALS),
        grade,
        lanes,
    }
}

/**
 * Timing habits (plan 5.4). From score rows: the FAST share with its 95% Wilson interval, early and late
 * counts per judgment, the signed timing bias of reference-rule plays (`extra.mean_offset_us`, AUTO ADJUST plays apart) and the
 * offset change it suggests, the average judge distance of the older and newer half, and the GREEN numbers used (a trace's own GREEN
 * when there is one). From traces: `trace_timing` (`computeTraceTiming`). The offset change rests on the trace timing when it is
 * `strong` (`source: "trace"`); otherwise on score rows, only with MIN_TIMING_PLAYS plays and MIN_TIMING_NOTES PGREAT and GREAT notes, a
 * FAST share whose interval leaves out one half, and a bias on the same side (`source: "score"`).
 */
export const computeJudgeHabit = (rows: readonly TimingScoreRow[]) => {
    const plays = rows.length
    const totals = fastShareOf(rows)
    const interval = wilsonInterval(totals.fast, totals.fast + totals.slow)
    const isEnoughFastSlow = plays >= MIN_TIMING_PLAYS && totals.fast + totals.slow >= JUDGE_HABIT_MIN_FAST_SLOW
    const isSeparated = interval.low > EVEN_SHARE || interval.high < EVEN_SHARE
    const fastSlowGrade = isEnoughFastSlow ? (isSeparated ? EVIDENCE.STRONG : EVIDENCE.UNCERTAIN) : EVIDENCE.INSUFFICIENT
    const leaning =
        fastSlowGrade === EVIDENCE.STRONG && totals.share !== null ? (totals.share > EVEN_SHARE ? ('fast' as const) : ('slow' as const)) : null

    const timed = rows.filter((row) => isReference(row) && row.meanOffsetUs !== null && !row.autoOffset)
    const weights = timed.map((row) => row.pgreat + row.great + row.good + row.bad)
    const weight = sum(weights)
    const meanOffsetMs =
        weight > 0
            ? roundTo(sum(timed.map((row, index) => (row.meanOffsetUs ?? 0) * (weights[index] ?? 0))) / weight / MICROS_PER_MILLI, MILLIS_DECIMALS)
            : null
    const timingNotes = sum(timed.map((row) => row.pgreat + row.great))
    const offsetGrade = timingEvidence({ plays: timed.length, notes: timingNotes })
    const isSameSide = meanOffsetMs !== null && leaning !== null && meanOffsetMs > 0 === (leaning === 'fast')
    const traceTiming = computeTraceTiming(rows)
    const isTraceBased = traceTiming.grade === EVIDENCE.STRONG && traceTiming.mean_ms !== null
    const biasMs = isTraceBased ? traceTiming.mean_ms : meanOffsetMs
    const shift = biasMs === null ? 0 : -Math.round(biasMs)
    const canSuggest =
        Math.abs(shift) >= OFFSET_SUGGEST_MIN_MS &&
        (isTraceBased || (offsetGrade === EVIDENCE.STRONG && fastSlowGrade === EVIDENCE.STRONG && isSameSide))
    const direction = canSuggest ? (shift > 0 ? ('earlier' as const) : ('later' as const)) : ('keep' as const)

    const judged = rows.filter((row) => isReference(row) && row.avgjudge > 0).toSorted((left, right) => left.playedAt - right.playedAt)
    const half = Math.floor(judged.length / MEDIAN_HALF)
    const meanAvgjudgeMs = (part: readonly TimingScoreRow[]) =>
        part.length > 0 ? roundTo(sum(part.map((row) => row.avgjudge)) / part.length / MICROS_PER_MILLI, MILLIS_DECIMALS) : null
    const older = meanAvgjudgeMs(judged.slice(0, half))
    const newer = meanAvgjudgeMs(judged.slice(judged.length - half))

    const greens = rows.flatMap((row) => {
        const green = greenOfRow(row)
        return green ? [{ row, green }] : []
    })
    const bucketOf = (green: number) => Math.floor(green / GREEN_BUCKET_WIDTH) * GREEN_BUCKET_WIDTH
    const buckets = new Map<number, TimingScoreRow[]>()
    for (const entry of greens) buckets.set(bucketOf(entry.green.green), [...(buckets.get(bucketOf(entry.green.green)) ?? []), entry.row])
    const laneMsOfGreen = (green: number) => Math.round(green / GREEN_DISPLAY_FACTOR)
    const medianGreen = median(greens.map((entry) => entry.green.green))
    const medianLaneMs = median(greens.map((entry) => entry.green.lane_ms))

    return {
        plays,
        from: rows.at(-1)?.playedAt ?? null,
        to: rows[0]?.playedAt ?? null,
        fast_slow: {
            fast: totals.fast,
            slow: totals.slow,
            fast_share: totals.share === null ? null : roundRatio(totals.share),
            grade: fastSlowGrade,
            leaning,
            by_grade: [
                { grade: 'great' as const, early: sum(rows.map((row) => row.egr)), late: sum(rows.map((row) => row.lgr)) },
                { grade: 'good' as const, early: sum(rows.map((row) => row.egd)), late: sum(rows.map((row) => row.lgd)) },
                { grade: 'bad' as const, early: sum(rows.map((row) => row.ebd)), late: sum(rows.map((row) => row.lbd)) },
            ],
        },
        offset: {
            plays: timed.length,
            notes: timingNotes,
            mean_offset_ms: meanOffsetMs,
            grade: offsetGrade,
            direction,
            suggested_shift_ms: canSuggest ? shift : null,
            auto_adjust_plays: rows.filter((row) => row.autoOffset).length,
            source: isTraceBased ? ('trace' as const) : ('score' as const),
        },
        trace_timing: traceTiming,
        avgjudge: {
            plays: judged.length,
            older_ms: older,
            newer_ms: newer,
            change_ms: older !== null && newer !== null ? roundTo(newer - older, MILLIS_DECIMALS) : null,
            grade: half >= AVGJUDGE_MIN_HALF_PLAYS ? EVIDENCE.STRONG : EVIDENCE.INSUFFICIENT,
        },
        green: {
            plays: greens.length,
            latest: greens[0]?.green ?? null,
            median:
                medianGreen === null || medianLaneMs === null
                    ? null
                    : {
                          green: Math.round(medianGreen),
                          lane_ms: Math.round(medianLaneMs),
                          is_estimated: greens.some((entry) => entry.green.is_estimated),
                      },
            constant_plays: greens.filter((entry) => entry.row.constant).length,
            buckets: [...buckets.entries()]
                .toSorted(([left], [right]) => left - right)
                .map(([from, inBucket]) => ({
                    key: `green.${from}`,
                    from: { green: from, lane_ms: laneMsOfGreen(from) },
                    to: { green: from + GREEN_BUCKET_WIDTH, lane_ms: laneMsOfGreen(from + GREEN_BUCKET_WIDTH) },
                    plays: inBucket.length,
                    fast_share: fastShareOf(inBucket).share === null ? null : roundRatio(fastShareOf(inBucket).share ?? 0),
                    ex_rate: pooledExRate(inBucket),
                })),
        },
    }
}

export type JudgeHabit = ReturnType<typeof computeJudgeHabit>

/** The habit with a `key` on each item the AI may point at. */
export const toJudgeHabitDigest = (habit: JudgeHabit) => ({
    plays: { key: 'plays', count: habit.plays },
    fast_slow: {
        key: 'timing.fast_slow',
        ...habit.fast_slow,
        by_grade: habit.fast_slow.by_grade.map((row) => ({ key: `timing.${row.grade}`, ...row })),
    },
    offset: { key: 'timing.offset', ...habit.offset },
    trace_timing: { key: 'timing.trace', ...habit.trace_timing },
    avgjudge: { key: 'timing.avgjudge', ...habit.avgjudge },
    green: { key: 'green', ...habit.green },
})

/**
 * Pins the offset direction to the server's suggestion (the AI explains it, it does not decide it) and keeps notes that point at
 * a digest item and fit; the answer is rejected when the reasoning texts are empty.
 */
export const refineJudgeHabitOutput = (output: AiJudgeHabitOutput, params: { digest: unknown; direction: JudgeHabit['offset']['direction'] }) => {
    const keys = collectDigestKeys(params.digest)
    if (output.offset_reasoning.trim() === '' || output.summary.trim() === '') return null
    const notes = output.notes
        .filter(
            (item) =>
                keys.has(item.key) &&
                fitsLine(item.title, AI_TEXT_LIMIT.INSIGHT_TITLE_MAX_CHARS) &&
                fitsLine(item.body, AI_TEXT_LIMIT.INSIGHT_BODY_MAX_CHARS),
        )
        .slice(0, AI_JUDGE_HABIT_NOTES_MAX)
        .map((item) => ({ ...item, title: item.title.trim(), body: item.body.trim() }))
    return {
        summary: cutText(output.summary, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
        offset_direction: params.direction,
        offset_reasoning: cutText(output.offset_reasoning, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
        green_direction: output.green_direction,
        green_reasoning: cutText(output.green_reasoning, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
        notes,
    }
}

export const createAiJudgeHabitService = (deps: AiJudgeHabitServiceDeps) => {
    const build = async (userId: string) => {
        const habit = computeJudgeHabit(await deps.db.listRecentTimingRows({ userId, limit: JUDGE_HABIT_RECENT_PLAYS }))
        return { habit, digest: toJudgeHabitDigest(habit) }
    }

    return {
        /** The deterministic habit and the latest stored AI explanation of it. No provider is called. */
        view: async (userId: string) => {
            const built = await build(userId)
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: AI_JUDGE_HABIT_SPEC,
                userId,
                subjectKey: AI_JUDGE_HABIT_SUBJECT,
                digest: built.digest,
            })
            return { ...built.habit, notice: built.habit.plays < MIN_TIMING_PLAYS ? { need: 'recent_plays' } : null, ai } satisfies AiJudgeHabitView
        },

        /** Asks the user's AI to explain the habit; fewer than MIN_TIMING_PLAYS recent plays make no call. */
        interpret: async (userId: string) => {
            const built = await build(userId)
            if (built.habit.plays < MIN_TIMING_PLAYS) return notEnoughData('recent_plays')
            return runAiFeature(deps.runner, {
                ...AI_JUDGE_HABIT_SPEC,
                userId,
                subjectKey: AI_JUDGE_HABIT_SUBJECT,
                digest: built.digest,
                refine: (output) => refineJudgeHabitOutput(output, { digest: built.digest, direction: built.habit.offset.direction }),
            })
        },
    }
}

export type AiJudgeHabitService = ReturnType<typeof createAiJudgeHabitService>

import { TRACE_NOTE_KIND, TRACE_NOTE_STATE, type TraceUploadInput } from '@server/dto/trace'
import { badTakesNote, type ScoreInsertRow } from '@server/service/domain/score/score.service'

/** The score columns a trace is checked against: the judgements, their early/late split and the empty POORs. */
export const TRACE_CHECKED_COUNTS = [
    'pgreat',
    'great',
    'good',
    'bad',
    'poor',
    'epg',
    'lpg',
    'egr',
    'lgr',
    'egd',
    'lgd',
    'ebd',
    'lbd',
    'emptyPoor',
] as const satisfies readonly (keyof ScoreInsertRow)[]

type CheckedCount = (typeof TRACE_CHECKED_COUNTS)[number]

export type TraceJudgeCounts = Record<CheckedCount, number>

/** What a trace is checked against: the stored score row it names. Its `random` is the column as stored, a plain string. */
export type TraceScoreRow = Pick<
    ScoreInsertRow,
    CheckedCount | 'id' | 'userId' | 'chartSha256' | 'chartMd5' | 'mode' | 'lntype' | 'scratchAuto' | 'playedAt'
> & { random: string }

/** In a mode where a BAD leaves its note to be judged again (POPN_9K), BAD and POOR are not compared (main decision 2026-10-20). */
const COUNTS_A_KEPT_NOTE_DISTORTS = new Set<CheckedCount>(['bad', 'poor', 'ebd', 'lbd'])

const WIRE_NAME: Record<CheckedCount, string> = {
    pgreat: 'judge.pgreat',
    great: 'judge.great',
    good: 'judge.good',
    bad: 'judge.bad',
    poor: 'judge.poor',
    epg: 'judge.epg',
    lpg: 'judge.lpg',
    egr: 'judge.egr',
    lgr: 'judge.lgr',
    egd: 'judge.egd',
    lgd: 'judge.lgd',
    ebd: 'judge.ebd',
    lbd: 'judge.lbd',
    emptyPoor: 'judge.empty_poor',
}

const STATE_GRADE = {
    [TRACE_NOTE_STATE.PGREAT]: { total: 'pgreat', early: 'epg', late: 'lpg' },
    [TRACE_NOTE_STATE.GREAT]: { total: 'great', early: 'egr', late: 'lgr' },
    [TRACE_NOTE_STATE.GOOD]: { total: 'good', early: 'egd', late: 'lgd' },
    [TRACE_NOTE_STATE.BAD]: { total: 'bad', early: 'ebd', late: 'lbd' },
} as const satisfies Record<number, { total: CheckedCount; early: CheckedCount; late: CheckedCount }>

const isTimedState = (state: number): state is keyof typeof STATE_GRADE => state in STATE_GRADE

/**
 * The judgement counts a trace adds up to, as the score row counts them. A note's state is its judgement; an early one has
 * `err_us >= 0` (`matcher.rs`). A judged object the run never reached (state 0, any kind but a plain LN end) is a late POOR when
 * the run was left before its end (`PlaySummary::left_unfinished`) and nothing when an empty gauge ended it (`failed_at_ms` set),
 * as the score of such a run does not count the notes it did not get to.
 */
export const countTraceJudgements = (trace: Pick<TraceUploadInput, 'notes' | 'empty_poors' | 'failed_at_ms'>) => {
    const counts = {
        pgreat: 0,
        great: 0,
        good: 0,
        bad: 0,
        poor: 0,
        epg: 0,
        lpg: 0,
        egr: 0,
        lgr: 0,
        egd: 0,
        lgd: 0,
        ebd: 0,
        lbd: 0,
        emptyPoor: trace.empty_poors.lane.length,
    } satisfies TraceJudgeCounts
    const isLeftBeforeItsEnd = trace.failed_at_ms === null
    trace.notes.state.forEach((state, index) => {
        if (state === TRACE_NOTE_STATE.POOR) {
            counts.poor += 1
            return
        }
        if (isTimedState(state)) {
            const grade = STATE_GRADE[state]
            counts[grade.total] += 1
            counts[trace.notes.err_us[index] >= 0 ? grade.early : grade.late] += 1
            return
        }
        if (isLeftBeforeItsEnd && trace.notes.kind[index] !== TRACE_NOTE_KIND.LONG_END) counts.poor += 1
    })
    return counts
}

/**
 * The run conditions a trace states and its score row must state the same way (the client fills both from one run): the mode, the
 * LN type, the lane option and whether SCRATCH AUTO played any note (`auto_lanes` has a played lane exactly when the score says so).
 */
const runConditionMismatch = (trace: TraceUploadInput, score: TraceScoreRow) => {
    if (trace.mode !== score.mode) return { field: 'mode', reason: `score ${score.mode}, trace ${trace.mode}` }
    if (trace.lntype !== score.lntype) return { field: 'lntype', reason: `score ${score.lntype}, trace ${trace.lntype}` }
    if (trace.random !== score.random) return { field: 'random', reason: `score ${score.random}, trace ${trace.random}` }
    const isTraceScratchAuto = trace.auto_lanes.some(Boolean)
    if (isTraceScratchAuto !== score.scratchAuto) return { field: 'auto_lanes', reason: `score ${score.scratchAuto}, trace ${isTraceScratchAuto}` }
    return null
}

/**
 * The first way a trace and the score it names disagree, or null: the chart (the path's hash and the body's chart must be the
 * score's), the run conditions (`runConditionMismatch`), then every judgement count, its early/late split and the empty POORs,
 * with no tolerance. The 9-key exception follows the score's mode, which the trace's must equal. `field` is the wire name and
 * `reason` says both values.
 */
export const findTraceMismatch = (params: { trace: TraceUploadInput; pathHash: string; score: TraceScoreRow }) => {
    const { trace, pathHash, score } = params
    const chartMd5 = score.chartMd5 ?? trace.chart.md5
    if (trace.chart.sha256 !== score.chartSha256) return { field: 'chart.sha256', reason: 'the trace names another chart than its score' }
    if (pathHash !== score.chartSha256 && pathHash !== chartMd5) return { field: 'hash', reason: 'the path names another chart than the score' }
    const condition = runConditionMismatch(trace, score)
    if (condition) return condition
    const counted = countTraceJudgements(trace)
    const compared = badTakesNote(score.mode) ? TRACE_CHECKED_COUNTS : TRACE_CHECKED_COUNTS.filter((name) => !COUNTS_A_KEPT_NOTE_DISTORTS.has(name))
    const differing = compared.find((name) => counted[name] !== score[name])
    if (!differing) return null
    return { field: WIRE_NAME[differing], reason: `score ${score[differing]}, trace ${counted[differing]}` }
}

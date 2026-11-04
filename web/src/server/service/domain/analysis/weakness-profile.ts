import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import type { ProfileCellUnit, ProfileSampleLevel, WeaknessItem, WeaknessProfileCell, WeaknessProfileView } from '@server/dto/ai-play'
import {
    CHART_WEIGHT_CAP_NOTES,
    COMBO_INTERACTION_MIN,
    COMFORT_EX_DROP,
    CONTEXT_LIFT_MIN,
    CONTEXT_LIFT_PSEUDO_COUNT,
    FDR_Q,
    LIMIT_BP_RATE,
    LIMIT_EX_DROP,
    MIN_BIN_CHARTS,
    MIN_BIN_CONTEXT_NOTES,
    MIN_BIN_NOTES,
    MIN_CELL_CHARTS,
    MIN_CELL_NOTES,
    MIN_CELL_PLAYS,
    MIN_CONTEXT_NOTES,
    PROFILE_DENSITY_MIN_BINS,
    PROFILE_MIN_PLAYS,
    PROFILE_NPS_BIN_WIDTH,
    PROFILE_STRENGTH_TOP,
    PROFILE_TEST_FAMILY,
    PROFILE_VERSION,
    PROFILE_WEAKNESS_TOP,
    SAMPLE_LEVEL_EDGES,
    SEVERITY_NOTES_CAP,
    SEVERITY_UNIT_SCALE,
} from '@server/service/domain/ai/play-feature.constants'
import {
    BPM_BANDS,
    HAND_KEYS,
    NPS_BIN_TOP,
    NPS_SUMMARY_DECIMALS,
    PEAK_NPS_DECIMALS,
    RATIO_DECIMALS,
    SP7_LANE_NAMES,
} from '@server/service/domain/analysis/analysis-constants'
import { PATTERN_TAGS, SCRATCH_TAGS, type PatternTag } from '@server/service/domain/analysis/pattern-tags'
import { roundToDecimals, type TimingMoments } from '@server/service/domain/analysis/play-cells'
import {
    MONOTONE,
    benjaminiHochberg,
    clusterRobustTest,
    firstCrossingOf,
    isotonicRegression,
    type ClusterValue,
} from '@server/service/domain/analysis/profile-stats'
import { EVIDENCE, TAG_LEANING, type Evidence } from '@server/service/domain/analysis/sample'
import { COMBO_TAGS } from '@server/service/domain/analysis/tag-combos'
import {
    SOFLAN_KEY_PHASES,
    WEAKNESS_KEY,
    isFolderSelectableKind,
    parseWeaknessKey,
    type ParsedWeaknessKey,
    type WeaknessKind,
} from '@server/service/domain/analysis/weakness-key'
import { trainingKindsOfKey } from '@server/service/domain/training/training-constants'
import type { TraceAnalysisSummary } from '@server/service/domain/trace/trace-analysis'

/** One analysed play a profile reads: its chart, when it was played, whether the chart is one of the player's training charts, and its summary. */
export type ProfilePlay = { chartSha256: string; playedAt: number; isTraining: boolean; summary: TraceAnalysisSummary }

/** Where the plays come from: the window's edges, the start of its recent part (the trend), its length and the plays left out as stale. */
export type ProfileWindow = { fromMs: number; toMs: number; recentFromMs: number; days: number; stale: number }

type Metrics = NonNullable<TraceAnalysisSummary['metrics']>
type Counts = { notes: number; ex_score: number; bp: number }

/** Every training chart play is one cluster, so different training charts add at most one chart's weight to a cell (plan 2.5.2). */
export const TRAINING_CLUSTER = 'training'

const EX_PER_NOTE = 2
const MICROS_PER_MILLI = 1000
const HALF = 0.5
const P_VALUE_DECIMALS = 6
const PEAK_WINDOW_KEY = '2000'

type MeasuredPlay = {
    cluster: string
    chartSha256: string
    playedAt: number
    isTraining: boolean
    auto: boolean
    exRatio: number
    metrics: Metrics
    summary: TraceAnalysisSummary
}

const measuredPlaysOf = (plays: readonly ProfilePlay[]) =>
    plays.flatMap((play): MeasuredPlay[] => {
        const metrics = play.summary.metrics
        if (!metrics || metrics.overall.notes <= 0) return []
        return [
            {
                cluster: play.isTraining ? TRAINING_CLUSTER : play.chartSha256,
                chartSha256: play.chartSha256,
                playedAt: play.playedAt,
                isTraining: play.isTraining,
                auto: play.summary.scratch_auto,
                exRatio: metrics.overall.ex_score / (EX_PER_NOTE * metrics.overall.notes),
                metrics,
                summary: play.summary,
            },
        ]
    })

const roundRatio = (value: number) => roundToDecimals(value, RATIO_DECIMALS)
const exRatioOf = (counts: Pick<Counts, 'notes' | 'ex_score'>) => (counts.notes > 0 ? counts.ex_score / (EX_PER_NOTE * counts.notes) : 0)
const sumOf = (values: readonly number[]) => values.reduce((total, value) => total + value, 0)

/** Orders text by code unit, the same on every runtime (keys and chart hashes are ASCII). */
const compareText = (left: string, right: string) => (left < right ? -1 : Number(left > right))

/** Groups plays by cluster, the clusters in key order so the same plays give the same sums whatever order they come in. */
const byCluster = <T extends { play: MeasuredPlay }>(entries: readonly T[]) =>
    [...Map.groupBy(entries, (entry) => entry.play.cluster).entries()].toSorted(([left], [right]) => compareText(left, right))

const weightOf = (notes: number) => Math.min(notes, CHART_WEIGHT_CAP_NOTES)

const isScratchTag = (tag: PatternTag) => (SCRATCH_TAGS as readonly PatternTag[]).includes(tag)

/** The cells a SCRATCH AUTO play stays out of: the scratch lane and hand, and every cell naming a scratch tag (plan 2.5.2). */
const isScratchKey = (parsed: ParsedWeaknessKey) =>
    (parsed.kind === 'lane' && parsed.arg === 'S') || (parsed.kind === 'hand' && parsed.arg === 'scratch') || parsed.tags.some(isScratchTag)

type ProfileTiming = NonNullable<WeaknessProfileCell['timing']>

const addMoments = (total: TimingMoments, part: TimingMoments) => ({
    n: total.n + part.n,
    sum_us: total.sum_us + part.sum_us,
    sumsq_ms2: total.sumsq_ms2 + part.sumsq_ms2,
    fast_n: total.fast_n + part.fast_n,
    fast_sum_us: total.fast_sum_us + part.fast_sum_us,
    slow_n: total.slow_n + part.slow_n,
    slow_sum_us: total.slow_sum_us + part.slow_sum_us,
})

const NO_MOMENTS: TimingMoments = { n: 0, sum_us: 0, sumsq_ms2: 0, fast_n: 0, fast_sum_us: 0, slow_n: 0, slow_sum_us: 0 }

const poolMoments = (parts: readonly (TimingMoments | undefined)[]) =>
    parts.reduce<TimingMoments>((total, part) => (part ? addMoments(total, part) : total), NO_MOMENTS)

/** The mean, spread and FAST/SLOW of pooled timing moments in milliseconds; null with no note. */
const timingOf = (moments: TimingMoments): ProfileTiming | null => {
    if (moments.n <= 0) return null
    const meanMs = moments.sum_us / moments.n / MICROS_PER_MILLI
    return {
        mean_ms: roundRatio(meanMs),
        sd_ms: roundRatio(Math.sqrt(Math.max(0, moments.sumsq_ms2 / moments.n - meanMs ** 2))),
        fast_mean_ms: moments.fast_n > 0 ? roundRatio(moments.fast_sum_us / moments.fast_n / MICROS_PER_MILLI) : 0,
        slow_mean_ms: moments.slow_n > 0 ? roundRatio(moments.slow_sum_us / moments.slow_n / MICROS_PER_MILLI) : 0,
        fast_share: roundRatio(moments.fast_n / moments.n),
    }
}

/** A cell before its test: its key, how it is read, the cluster values its test runs on and the counts behind it. */
type CellDraft = {
    key: string
    parsed: ParsedWeaknessKey
    unit: ProfileCellUnit
    display: number | null
    clusters: ClusterValue[]
    notes: number
    charts: number
    plays: number
    autoPlays: number
    exRatio: number | null
    bpRate: number | null
    timing: ProfileTiming | null
    minNotes: number
    candidate: boolean
}

const parsedOf = (key: string) => {
    const parsed = parseWeaknessKey(key)
    if (!parsed) throw createAppError(ERROR_CODE.INTERNAL_ERROR, { reason: 'not_a_weakness_key', key })
    return parsed
}

const draftOf = (key: string, unit: ProfileCellUnit, parts: Partial<CellDraft>) =>
    ({
        key,
        parsed: parsedOf(key),
        unit,
        display: null,
        clusters: [],
        notes: 0,
        charts: 0,
        plays: 0,
        autoPlays: 0,
        exRatio: null,
        bpRate: null,
        timing: null,
        minNotes: MIN_CELL_NOTES,
        candidate: true,
        ...parts,
    }) satisfies CellDraft

/** The plays a cell reads: every play, or every play but those under SCRATCH AUTO for a scratch cell, with how many were left out. */
const scopeOf = (plays: readonly MeasuredPlay[], key: string) => {
    if (!isScratchKey(parsedOf(key))) return { plays, autoPlays: 0 }
    const kept = plays.filter((play) => !play.auto)
    return { plays: kept, autoPlays: plays.length - kept.length }
}

/**
 * An EX residual cell (plan 2.5.2): per chart cluster k the notes N, EX score E and B = Σ notes × the play's own EX ratio, so the
 * cluster's residual `E / 2N − B / N` cancels how hard the charts were.
 */
const residualCell = (key: string, plays: readonly MeasuredPlay[], countsOf: (metrics: Metrics) => Counts | undefined) => {
    const scope = scopeOf(plays, key)
    const used = scope.plays.flatMap((play) => {
        const counts = countsOf(play.metrics)
        return counts && counts.notes > 0 ? [{ play, counts }] : []
    })
    const clusters = byCluster(used).map(([, entries]) => {
        const notes = sumOf(entries.map(({ counts }) => counts.notes))
        const exScore = sumOf(entries.map(({ counts }) => counts.ex_score))
        const baseline = sumOf(entries.map(({ counts, play }) => counts.notes * play.exRatio))
        return { value: exScore / (EX_PER_NOTE * notes) - baseline / notes, weight: weightOf(notes) }
    })
    const notes = sumOf(used.map(({ counts }) => counts.notes))
    return draftOf(key, 'ex_residual', {
        clusters,
        notes,
        charts: clusters.length,
        plays: used.length,
        autoPlays: scope.autoPlays,
        exRatio: notes > 0 ? roundRatio(sumOf(used.map(({ counts }) => counts.ex_score)) / (EX_PER_NOTE * notes)) : null,
        bpRate: notes > 0 ? roundRatio(sumOf(used.map(({ counts }) => counts.bp)) / notes) : null,
    })
}

const tallyCounts = (tally: { notes: number; ex_score: number; bad: number; poor: number } | undefined) =>
    tally ? { notes: tally.notes, ex_score: tally.ex_score, bp: tally.bad + tally.poor } : undefined

/** The PGREAT to BAD timing of the whole play: its hands' moments added, each note in exactly one hand. */
const playMomentsOf = (metrics: Metrics) => poolMoments(HAND_KEYS.map((hand) => metrics.timing_by_hand[hand]))

/**
 * A tag's timing cell (plan 2.5.4): per cluster the tag notes' mean error less the whole plays' mean error (PGREAT to BAD, from
 * `timing_by_hand`, so a note with several tags counts once in the whole), in milliseconds.
 */
const timingCell = (tag: PatternTag, plays: readonly MeasuredPlay[]) => {
    const key = WEAKNESS_KEY.timing(tag)
    const scope = scopeOf(plays, key)
    const used = scope.plays.flatMap((play) => {
        const moments = play.metrics.timing_by_tag[tag]
        return moments && moments.n > 0 ? [{ play, moments }] : []
    })
    const clusters = byCluster(used).flatMap(([, entries]) => {
        const tagMoments = poolMoments(entries.map(({ moments }) => moments))
        const whole = poolMoments(entries.map(({ play }) => playMomentsOf(play.metrics)))
        if (whole.n <= 0) return []
        return [{ value: (tagMoments.sum_us / tagMoments.n - whole.sum_us / whole.n) / MICROS_PER_MILLI, weight: weightOf(tagMoments.n) }]
    })
    const pooled = poolMoments(used.map(({ moments }) => moments))
    return draftOf(key, 'ms', {
        clusters,
        notes: pooled.n,
        charts: clusters.length,
        plays: used.length,
        autoPlays: scope.autoPlays,
        timing: timingOf(pooled),
    })
}

/** A soflan phase against the phase before the event (plan 2.5.4): per cluster the phase's EX ratio less the `before` EX ratio. */
const soflanCell = (phase: (typeof SOFLAN_KEY_PHASES)[number], plays: readonly MeasuredPlay[]) => {
    const used = plays.flatMap((play) => {
        const { before, [phase]: during } = play.metrics.soflan_phase
        return before.notes > 0 && during.notes > 0 ? [{ play, before, during }] : []
    })
    const clusters = byCluster(used).map(([, entries]) => {
        const during = { notes: sumOf(entries.map((entry) => entry.during.notes)), ex_score: sumOf(entries.map((entry) => entry.during.ex_score)) }
        const before = { notes: sumOf(entries.map((entry) => entry.before.notes)), ex_score: sumOf(entries.map((entry) => entry.before.ex_score)) }
        return { value: exRatioOf(during) - exRatioOf(before), weight: weightOf(during.notes) }
    })
    const notes = sumOf(used.map((entry) => entry.during.notes))
    return draftOf(WEAKNESS_KEY.soflan(phase), 'ratio', {
        clusters,
        notes,
        charts: clusters.length,
        plays: used.length,
        exRatio: notes > 0 ? roundRatio(sumOf(used.map((entry) => entry.during.ex_score)) / (EX_PER_NOTE * notes)) : null,
        bpRate: notes > 0 ? roundRatio(sumOf(used.map((entry) => entry.during.bp)) / notes) : null,
    })
}

/** The fatigue cell: per cluster the note-weighted mean of the plays' last-third less first-third EX ratio. */
const fatigueCell = (plays: readonly MeasuredPlay[]) => {
    const used = plays.flatMap((play) => {
        const delta = play.metrics.fatigue.delta
        return delta === null ? [] : [{ play, delta, notes: play.metrics.overall.notes }]
    })
    const clusters = byCluster(used).map(([, entries]) => {
        const notes = sumOf(entries.map((entry) => entry.notes))
        return { value: sumOf(entries.map((entry) => entry.delta * entry.notes)) / notes, weight: weightOf(notes) }
    })
    return draftOf(WEAKNESS_KEY.fatigue(), 'ratio', {
        clusters,
        notes: sumOf(used.map((entry) => entry.notes)),
        charts: clusters.length,
        plays: used.length,
    })
}

/** The profile bin of a stored local NPS bin key (`"0"` to `"39"`, `"40+"`): PROFILE_NPS_BIN_WIDTH wide, the top bin from NPS_BIN_TOP. */
const profileBinOf = (storedKey: string) => {
    const lower = Number.parseInt(storedKey, 10)
    return Math.min(Math.floor(lower / PROFILE_NPS_BIN_WIDTH) * PROFILE_NPS_BIN_WIDTH, NPS_BIN_TOP)
}

type BinCounts = { all: Counts; burst: Counts; sustained: Counts }

const emptyCounts = (): Counts => ({ notes: 0, ex_score: 0, bp: 0 })
const addCounts = (total: Counts, part: Counts) => ({
    notes: total.notes + part.notes,
    ex_score: total.ex_score + part.ex_score,
    bp: total.bp + part.bp,
})

/** One play's local NPS cells pooled into profile bins. */
const playBinsOf = (metrics: Metrics) => {
    const bins = new Map<number, BinCounts>()
    for (const [storedKey, cell] of Object.entries(metrics.by_nps)) {
        const bin = profileBinOf(storedKey)
        const total = bins.get(bin) ?? { all: emptyCounts(), burst: emptyCounts(), sustained: emptyCounts() }
        bins.set(bin, {
            all: addCounts(total.all, cell),
            burst: addCounts(total.burst, cell.burst),
            sustained: addCounts(total.sustained, cell.sustained),
        })
    }
    return bins
}

/**
 * Sustained against burst density (plan 2.5.4, plays under SCRATCH AUTO left out): in each profile bin where the window holds
 * MIN_BIN_CONTEXT_NOTES notes of both contexts, the sustained notes' EX ratio less the burst notes', averaged per cluster with the
 * two contexts' notes as weights.
 */
const sustainedCell = (plays: readonly MeasuredPlay[]) => {
    const used = plays.map((play) => ({ play, bins: playBinsOf(play.metrics) }))
    const pooled = new Map<number, { burst: number; sustained: number }>()
    for (const { bins } of used) {
        for (const [bin, counts] of bins) {
            const total = pooled.get(bin) ?? { burst: 0, sustained: 0 }
            pooled.set(bin, { burst: total.burst + counts.burst.notes, sustained: total.sustained + counts.sustained.notes })
        }
    }
    const eligible = [...pooled.entries()]
        .filter(([, counts]) => counts.burst >= MIN_BIN_CONTEXT_NOTES && counts.sustained >= MIN_BIN_CONTEXT_NOTES)
        .map(([bin]) => bin)
        .toSorted((left, right) => left - right)
    const clusters = byCluster(used).flatMap(([, entries]) => {
        const rows = eligible.flatMap((bin) => {
            const burst = entries.reduce((total, { bins }) => addCounts(total, bins.get(bin)?.burst ?? emptyCounts()), emptyCounts())
            const sustained = entries.reduce((total, { bins }) => addCounts(total, bins.get(bin)?.sustained ?? emptyCounts()), emptyCounts())
            return burst.notes > 0 && sustained.notes > 0
                ? [{ diff: exRatioOf(sustained) - exRatioOf(burst), notes: burst.notes + sustained.notes }]
                : []
        })
        const notes = sumOf(rows.map((row) => row.notes))
        return notes > 0 ? [{ value: sumOf(rows.map((row) => row.diff * row.notes)) / notes, weight: weightOf(notes), notes }] : []
    })
    return draftOf(WEAKNESS_KEY.density('sustained'), 'ratio', {
        clusters: clusters.map(({ value, weight }) => ({ value, weight })),
        notes: sumOf(clusters.map((cluster) => cluster.notes)),
        charts: clusters.length,
        plays: used.filter(({ bins }) => eligible.some((bin) => (bins.get(bin)?.burst.notes ?? 0) > 0 && (bins.get(bin)?.sustained.notes ?? 0) > 0))
            .length,
    })
}

type ContextSource = {
    key: (tag: PatternTag) => string
    tagCount: (metrics: Metrics, tag: PatternTag) => number
    notes: (metrics: Metrics) => number
}

const CONTEXT_SOURCES = {
    bp_context: {
        key: WEAKNESS_KEY.bpContext,
        tagCount: (metrics, tag) => metrics.bp_runs.context_tags[tag] ?? 0,
        notes: (metrics) => metrics.bp_runs.context_notes,
    },
    gauge_context: {
        key: WEAKNESS_KEY.gaugeContext,
        tagCount: (metrics, tag) => metrics.gauge_drop_tag_totals[tag] ?? 0,
        notes: (metrics) => metrics.gauge_drop_notes,
    },
} as const satisfies Record<string, ContextSource>

const liftOf = (contextTag: number, contextNotes: number, tagNotes: number, allNotes: number) =>
    contextNotes > 0 && tagNotes > 0 && allNotes > 0 ? contextTag / contextNotes / (tagNotes / allNotes) : 0

/**
 * A failure or gauge-drop context cell (plan 2.5.4): how much more often the tag comes right before a BAD/POOR run (or inside a
 * gauge drop) than among all the notes, `lift`. It is listed only from MIN_CONTEXT_NOTES context notes and a lift of CONTEXT_LIFT_MIN
 * over the window; the test runs on each cluster's log lift, its counts each eased by CONTEXT_LIFT_PSEUDO_COUNT.
 */
const contextCell = (source: ContextSource, tag: PatternTag, plays: readonly MeasuredPlay[]) => {
    const key = source.key(tag)
    const scope = scopeOf(plays, key)
    const used = scope.plays.map((play) => ({
        play,
        contextTag: source.tagCount(play.metrics, tag),
        contextNotes: source.notes(play.metrics),
        tagNotes: play.metrics.by_tag[tag]?.notes ?? 0,
        allNotes: play.metrics.overall.notes,
    }))
    const total = (pick: (entry: (typeof used)[number]) => number) => sumOf(used.map(pick))
    const contextTag = total((entry) => entry.contextTag)
    const lift = liftOf(
        contextTag,
        total((entry) => entry.contextNotes),
        total((entry) => entry.tagNotes),
        total((entry) => entry.allNotes),
    )
    const withContext = used.filter((entry) => entry.contextNotes > 0)
    const clusters = byCluster(withContext).map(([, entries]) => {
        const sum = (pick: (entry: (typeof used)[number]) => number) => sumOf(entries.map(pick))
        const contextShare = (sum((entry) => entry.contextTag) + CONTEXT_LIFT_PSEUDO_COUNT) / (sum((entry) => entry.contextNotes) + 1)
        const baseShare = (sum((entry) => entry.tagNotes) + CONTEXT_LIFT_PSEUDO_COUNT) / (sum((entry) => entry.allNotes) + 1)
        return { value: Math.log(contextShare / baseShare), weight: weightOf(sum((entry) => entry.contextNotes)) }
    })
    return draftOf(key, 'ratio', {
        display: roundRatio(lift),
        clusters,
        notes: contextTag,
        charts: clusters.length,
        plays: withContext.length,
        autoPlays: scope.autoPlays,
        minNotes: MIN_CONTEXT_NOTES,
        candidate: contextTag >= MIN_CONTEXT_NOTES && lift >= CONTEXT_LIFT_MIN,
    })
}

const laneTiming = (lane: string) => (metrics: Metrics) => metrics.timing_by_lane[lane]?.moments

/** Every cell of the profile in the order it is listed, before its test. */
const draftCells = (plays: readonly MeasuredPlay[]) => {
    const keyPlays = plays.filter((play) => !play.auto)
    const withTiming = (draft: CellDraft, momentsOf: (metrics: Metrics) => TimingMoments | undefined) => ({
        ...draft,
        timing: timingOf(poolMoments(scopeOf(plays, draft.key).plays.map((play) => momentsOf(play.metrics)))),
    })
    return [
        ...PATTERN_TAGS.map((tag) => residualCell(WEAKNESS_KEY.tag(tag), plays, (metrics) => tallyCounts(metrics.by_tag[tag]))),
        ...COMBO_TAGS.map((tag) => residualCell(WEAKNESS_KEY.solo(tag), plays, (metrics) => metrics.by_tag_solo[tag])),
        ...COMBO_TAGS.flatMap((first, index) =>
            COMBO_TAGS.slice(index + 1).map((second) =>
                residualCell(WEAKNESS_KEY.combo(first, second), plays, (metrics) => metrics.by_combo[`${first}+${second}`]),
            ),
        ),
        ...SP7_LANE_NAMES.map((lane) =>
            withTiming(
                residualCell(WEAKNESS_KEY.lane(lane), plays, (metrics) => tallyCounts(metrics.by_lane[lane])),
                laneTiming(lane),
            ),
        ),
        ...HAND_KEYS.map((hand) =>
            withTiming(
                residualCell(WEAKNESS_KEY.hand(hand), plays, (metrics) => metrics.by_hand[hand]),
                (metrics) => metrics.timing_by_hand[hand],
            ),
        ),
        ...PATTERN_TAGS.map((tag) => timingCell(tag, plays)),
        ...BPM_BANDS.map((band) => residualCell(WEAKNESS_KEY.bpm(band), plays, (metrics) => metrics.by_bpm_band[band])),
        ...SOFLAN_KEY_PHASES.map((phase) => soflanCell(phase, plays)),
        fatigueCell(plays),
        sustainedCell(keyPlays),
        ...PATTERN_TAGS.map((tag) => contextCell(CONTEXT_SOURCES.bp_context, tag, plays)),
        ...PATTERN_TAGS.map((tag) => contextCell(CONTEXT_SOURCES.gauge_context, tag, plays)),
    ].filter((draft) => draft.notes > 0)
}

const meetsSample = (draft: CellDraft) => draft.notes >= draft.minNotes && draft.charts >= MIN_CELL_CHARTS && draft.plays >= MIN_CELL_PLAYS

const sampleLevelOf = (isSampled: boolean, charts: number, notes: number): ProfileSampleLevel => {
    if (!isSampled) return 'insufficient'
    if (charts >= SAMPLE_LEVEL_EDGES.HIGH_CHARTS && notes >= SAMPLE_LEVEL_EDGES.HIGH_NOTES) return 'high'
    if (charts >= SAMPLE_LEVEL_EDGES.MEDIUM_CHARTS && notes >= SAMPLE_LEVEL_EDGES.MEDIUM_NOTES) return 'medium'
    return 'low'
}

const isContextKind = (kind: WeaknessKind) => kind === 'bp_context' || kind === 'gauge_context'

const evidenceOf = (isSampled: boolean, correction: { passed: boolean } | undefined) => {
    if (!isSampled || !correction) return EVIDENCE.INSUFFICIENT
    return correction.passed ? EVIDENCE.STRONG : EVIDENCE.UNCERTAIN
}

/**
 * Which way a strong cell leans. A residual or a difference below 0 is a weakness and above 0 a strength; a context lift above 1
 * (log lift above 0) is a weakness; a strong timing deviation either way is a weakness (a habit to train), never a strength.
 */
const leaningOf = (kind: WeaknessKind, evidence: Evidence, estimate: number | null) => {
    if (evidence !== EVIDENCE.STRONG || estimate === null || estimate === 0) return null
    if (kind === 'timing') return TAG_LEANING.WEAKNESS
    if (isContextKind(kind)) return estimate > 0 ? TAG_LEANING.WEAKNESS : null
    return estimate < 0 ? TAG_LEANING.WEAKNESS : TAG_LEANING.STRENGTH
}

const severityScaleOf = (draft: CellDraft) => (isContextKind(draft.parsed.kind) ? SEVERITY_UNIT_SCALE.lift : SEVERITY_UNIT_SCALE[draft.unit])

const severityOf = (magnitude: number, scale: number, notes: number) =>
    roundRatio(Math.abs(magnitude) * scale * Math.sqrt(Math.min(notes, SEVERITY_NOTES_CAP) / SEVERITY_NOTES_CAP))

/** Each sampled cell's estimate, by key: what the trend compares between the recent and the older plays. */
const estimatesOf = (plays: readonly MeasuredPlay[]) =>
    new Map(
        draftCells(plays).flatMap((draft) => {
            const estimate = clusterRobustTest(draft.clusters).estimate
            return meetsSample(draft) && estimate !== null ? [[draft.key, estimate] as const] : []
        }),
    )

/**
 * Tests every cell: the cluster-robust t test of its cluster values, then Benjamini–Hochberg at FDR_Q within its family
 * (PROFILE_TEST_FAMILY) over the cells that meet the sample conditions. `strong` passes, `uncertain` does not, `insufficient` was not
 * tested. A tag pair also gets its `interaction`: its estimate less the lower of its two tags'.
 */
const finishCells = (drafts: readonly CellDraft[], trends: { recent: ReadonlyMap<string, number>; older: ReadonlyMap<string, number> }) => {
    const listed = drafts.filter((draft) => draft.candidate)
    const tested = listed.map((draft) => ({ draft, test: clusterRobustTest(draft.clusters), isSampled: meetsSample(draft) }))
    const corrections = new Map<CellDraft, { passed: boolean; threshold: number }>()
    for (const family of new Set(Object.values(PROFILE_TEST_FAMILY))) {
        const members = tested.filter((entry) => entry.isSampled && PROFILE_TEST_FAMILY[entry.draft.parsed.kind] === family)
        const results = benjaminiHochberg(
            members.map((entry) => entry.test.pValue),
            FDR_Q,
        )
        members.forEach((entry, index) => corrections.set(entry.draft, results[index]))
    }
    const estimates = new Map(tested.map(({ draft, test }) => [draft.key, test.estimate]))
    return tested.map(({ draft, test, isSampled }) => {
        const correction = corrections.get(draft)
        const evidence = evidenceOf(isSampled, correction)
        const recent = trends.recent.get(draft.key)
        const older = trends.older.get(draft.key)
        const [first, second] = draft.parsed.tags
        const tagEstimates =
            draft.parsed.kind === 'combo' && first && second ? [estimates.get(WEAKNESS_KEY.tag(first)), estimates.get(WEAKNESS_KEY.tag(second))] : []
        const knownTagEstimates = tagEstimates.flatMap((estimate) => (estimate === undefined || estimate === null ? [] : [estimate]))
        const interaction =
            test.estimate !== null && knownTagEstimates.length === 2 ? roundRatio(test.estimate - Math.min(...knownTagEstimates)) : null
        const cell = {
            key: draft.key,
            kind: draft.parsed.kind,
            value: draft.display ?? (test.estimate === null ? null : roundRatio(test.estimate)),
            unit: draft.unit,
            ex_ratio: draft.exRatio,
            bp_rate: draft.bpRate,
            timing: draft.timing,
            notes: draft.notes,
            charts: draft.charts,
            plays: draft.plays,
            auto_plays: draft.autoPlays,
            p_value: isSampled ? roundToDecimals(test.pValue, P_VALUE_DECIMALS) : null,
            q_threshold: isSampled && correction ? roundToDecimals(correction.threshold, P_VALUE_DECIMALS) : null,
            evidence,
            leaning: leaningOf(draft.parsed.kind, evidence, test.estimate),
            sample_level: sampleLevelOf(isSampled, draft.charts, draft.notes),
            trend_delta: recent !== undefined && older !== undefined ? roundRatio(recent - older) : null,
            interaction,
        } satisfies WeaknessProfileCell
        return { cell, severity: test.estimate === null ? 0 : severityOf(test.estimate, severityScaleOf(draft), draft.notes), parsed: draft.parsed }
    })
}

type FinishedCell = ReturnType<typeof finishCells>[number]

/**
 * The density limit curve (plan 2.5.4, plays under SCRATCH AUTO left out): the local NPS cells pooled into PROFILE_NPS_BIN_WIDTH
 * bins, the bins with MIN_BIN_NOTES notes over MIN_BIN_CHARTS charts used, EX ratio fitted never rising and BP rate never falling
 * (weighted isotonic regression), against the EX ratio of the bins at or below the player's median note NPS. The EX and BP limits are
 * where the fitted curve crosses its level, read by straight lines between bin middles (the top bin at its lower edge); a curve that
 * is past the level in its first bin gives that bin's middle, one that never reaches it gives none. The comfortable density is the
 * upper edge of the last bin within COMFORT_EX_DROP of the baseline, never above either limit.
 */
const densityLimitOf = (plays: readonly MeasuredPlay[]) => {
    const bins = new Map<number, { counts: Counts; clusters: Set<string> }>()
    for (const play of plays) {
        for (const [bin, counts] of playBinsOf(play.metrics)) {
            const total = bins.get(bin) ?? { counts: emptyCounts(), clusters: new Set<string>() }
            total.clusters.add(play.cluster)
            bins.set(bin, { counts: addCounts(total.counts, counts.all), clusters: total.clusters })
        }
    }
    const ordered = [...bins.entries()].toSorted(([left], [right]) => left - right)
    const used = ordered.filter(([, bin]) => bin.counts.notes >= MIN_BIN_NOTES && bin.clusters.size >= MIN_BIN_CHARTS)
    const exRatios = used.map(([, bin]) => exRatioOf(bin.counts))
    const bpRates = used.map(([, bin]) => bin.counts.bp / bin.counts.notes)
    const weights = used.map(([, bin]) => bin.counts.notes)
    const exFit = isotonicRegression(exRatios, weights, MONOTONE.DECREASING)
    const bpFit = isotonicRegression(bpRates, weights, MONOTONE.INCREASING)
    const curve = used.map(([from, bin], index) => ({
        from_nps: from,
        to_nps: from >= NPS_BIN_TOP ? null : from + PROFILE_NPS_BIN_WIDTH,
        notes: bin.counts.notes,
        charts: bin.clusters.size,
        ex_ratio: roundRatio(exRatios[index]),
        ex_fit: roundRatio(exFit[index]),
        bp_rate: roundRatio(bpRates[index]),
        bp_fit: roundRatio(bpFit[index]),
    }))
    const allNotes = sumOf(ordered.map(([, bin]) => bin.counts.notes))
    const reached = ordered.map((_, index) => sumOf(ordered.slice(0, index + 1).map(([, bin]) => bin.counts.notes)))
    const medianBin = ordered[reached.findIndex((notes) => notes * 2 >= allNotes)]?.[0]
    const below = used.flatMap(([from], index) => (medianBin !== undefined && from <= medianBin ? [index] : []))
    const belowNotes = sumOf(below.map((index) => weights[index]))
    const baseline = belowNotes > 0 ? sumOf(below.map((index) => exRatios[index] * weights[index])) / belowNotes : null
    const comfortIndex = baseline === null ? -1 : exFit.findLastIndex((fit) => fit >= baseline - COMFORT_EX_DROP)
    const centers = curve.map((bin) => (bin.from_nps + (bin.to_nps ?? bin.from_nps)) * HALF)
    const pointsOf = (fits: readonly number[]) => centers.map((x, index) => ({ x, y: fits[index] }))
    const limitEx = baseline === null ? null : firstCrossingOf(pointsOf(exFit), baseline - LIMIT_EX_DROP, MONOTONE.DECREASING)
    const limitBp = firstCrossingOf(pointsOf(bpFit), LIMIT_BP_RATE, MONOTONE.INCREASING)
    const limitIndex = limitEx?.index ?? -1
    const lastIndex = used.length - 1
    const allClusters = new Set(plays.map((play) => play.cluster))
    const isStrong = limitIndex > 0 && allClusters.size >= MIN_CELL_CHARTS
    const evidence = used.length < PROFILE_DENSITY_MIN_BINS ? EVIDENCE.INSUFFICIENT : evidenceOf(true, { passed: isStrong })
    const peaks = plays.map((play) => play.metrics.peak_nps[PEAK_WINDOW_KEY] ?? 0)
    const limitNpsEx = limitEx === null ? null : roundToDecimals(limitEx.x, NPS_SUMMARY_DECIMALS)
    const limitNpsBp = limitBp === null ? null : roundToDecimals(limitBp.x, NPS_SUMMARY_DECIMALS)
    const limits = [limitNpsEx, limitNpsBp].filter((limit) => limit !== null)
    return {
        curve,
        baseline_ex: baseline === null ? null : roundRatio(baseline),
        comfort_nps: comfortIndex < 0 ? null : Math.min(curve[comfortIndex].to_nps ?? curve[comfortIndex].from_nps, ...limits),
        limit_nps_ex: limitNpsEx,
        limit_nps_bp: limitNpsBp,
        max_seen_nps: lastIndex < 0 ? null : curve[lastIndex].from_nps,
        peak_seen_2s: peaks.length > 0 ? roundToDecimals(Math.max(...peaks), PEAK_NPS_DECIMALS) : null,
        ceiling_reached_ex: limitIndex >= 0,
        evidence,
        severity: baseline === null || lastIndex < 0 ? 0 : roundRatio(Math.max(0, baseline - exFit[lastIndex])),
        sample: { notes: sumOf(plays.map((play) => play.metrics.overall.notes)), charts: allClusters.size, plays: plays.length },
    }
}

type DensityLimit = ReturnType<typeof densityLimitOf>

const itemOf = (parts: Omit<WeaknessItem, 'kind' | 'label_tags' | 'selectable_for_folder' | 'training_kinds'>) => {
    const parsed = parsedOf(parts.key)
    return {
        ...parts,
        kind: parsed.kind,
        label_tags: [...parsed.tags],
        selectable_for_folder: isFolderSelectableKind(parsed.kind),
        training_kinds: [...trainingKindsOfKey(parts.key)],
    } satisfies WeaknessItem
}

const cellItem = ({ cell, severity }: FinishedCell) =>
    itemOf({
        key: cell.key,
        value: cell.value,
        unit: cell.unit,
        evidence: cell.evidence,
        sample_level: cell.sample_level,
        notes: cell.notes,
        charts: cell.charts,
        plays: cell.plays,
        severity,
        trend_delta: cell.trend_delta,
    })

const densityItem = (density: DensityLimit, trend: number | null) =>
    itemOf({
        key: WEAKNESS_KEY.density('limit'),
        value: density.limit_nps_ex,
        unit: 'nps',
        evidence: density.evidence,
        sample_level: sampleLevelOf(true, density.sample.charts, density.sample.notes),
        notes: density.sample.notes,
        charts: density.sample.charts,
        plays: density.sample.plays,
        severity: density.severity,
        trend_delta: trend,
    })

const bySeverity = (left: WeaknessItem, right: WeaknessItem) => right.severity - left.severity || compareText(left.key, right.key)

const isComboWeakness = (cell: WeaknessProfileCell) =>
    cell.kind !== 'combo' || (cell.interaction !== null && cell.interaction <= -COMBO_INTERACTION_MIN)

const median = (values: readonly number[]) => {
    const sorted = values.toSorted((left, right) => left - right)
    if (sorted.length === 0) return null
    const middle = Math.floor(sorted.length / 2)
    return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) * HALF
}

/** The BP runs of the window: per chart the mean longest run, averaged over charts; the run length histogram; the median recovery. */
const bpRunsOf = (plays: readonly MeasuredPlay[]) => {
    const clusterMeans = byCluster(plays.map((play) => ({ play }))).map(
        ([, entries]) => sumOf(entries.map(({ play }) => play.metrics.bp_runs.longest)) / entries.length,
    )
    const histogram: Record<string, number> = {}
    for (const play of plays) {
        for (const [bucket, count] of Object.entries(play.metrics.bp_runs.histogram)) histogram[bucket] = (histogram[bucket] ?? 0) + count
    }
    const recoveries = plays.flatMap((play) => play.metrics.bp_runs.runs.flatMap((run) => (run.recovery_notes === null ? [] : [run.recovery_notes])))
    return {
        longest_mean: clusterMeans.length > 0 ? roundToDecimals(sumOf(clusterMeans) / clusterMeans.length, NPS_SUMMARY_DECIMALS) : null,
        histogram,
        recovery_median: median(recoveries),
    }
}

/** The mean local NPS of a play's judged notes, read off its stored NPS bins at their middles (the top bin at its lower edge). */
const npsSumOf = (metrics: Metrics) =>
    Object.entries(metrics.by_nps).reduce(
        (total, [storedKey, cell]) => {
            const lower = Number.parseInt(storedKey, 10)
            const middle = lower >= NPS_BIN_TOP ? lower : lower + HALF
            return { notes: total.notes + cell.notes, sum: total.sum + middle * cell.notes }
        },
        { notes: 0, sum: 0 },
    )

const gaugeOf = (plays: readonly MeasuredPlay[]) => {
    const contexts = plays.flatMap((play) => play.metrics.gauge_drop_contexts)
    const contextNotes = sumOf(contexts.map((context) => context.chart_notes))
    const nps = plays.map((play) => npsSumOf(play.metrics))
    const npsNotes = sumOf(nps.map((entry) => entry.notes))
    return {
        drops_per_play: plays.length > 0 ? roundRatio(sumOf(plays.map((play) => play.summary.gauge.drops.length)) / plays.length) : null,
        drop_mean_nps:
            contextNotes > 0
                ? roundToDecimals(sumOf(contexts.map((context) => context.mean_nps * context.chart_notes)) / contextNotes, NPS_SUMMARY_DECIMALS)
                : null,
        overall_mean_nps: npsNotes > 0 ? roundToDecimals(sumOf(nps.map((entry) => entry.sum)) / npsNotes, NPS_SUMMARY_DECIMALS) : null,
    }
}

const soflanOf = (plays: readonly MeasuredPlay[]) => {
    const events = sumOf(plays.map((play) => play.summary.speed.soflan_adjust.events))
    const adjusted = sumOf(plays.map((play) => play.summary.speed.soflan_adjust.adjusted))
    return { events, adjusted_share: events > 0 ? roundRatio(adjusted / events) : null }
}

const buildProfile = (plays: readonly ProfilePlay[], window: ProfileWindow) => {
    const measured = measuredPlaysOf(plays)
    const keyPlays = measured.filter((play) => !play.auto)
    const recent = measured.filter((play) => play.playedAt >= window.recentFromMs)
    const older = measured.filter((play) => play.playedAt < window.recentFromMs)
    const cells = finishCells(draftCells(measured), { recent: estimatesOf(recent), older: estimatesOf(older) })
    const density = densityLimitOf(keyPlays)
    const recentLimit = densityLimitOf(recent.filter((play) => !play.auto))
    const olderLimit = densityLimitOf(older.filter((play) => !play.auto))
    const densityTrend =
        recentLimit.limit_nps_ex !== null &&
        olderLimit.limit_nps_ex !== null &&
        recentLimit.evidence !== EVIDENCE.INSUFFICIENT &&
        olderLimit.evidence !== EVIDENCE.INSUFFICIENT
            ? roundToDecimals(recentLimit.limit_nps_ex - olderLimit.limit_nps_ex, NPS_SUMMARY_DECIMALS)
            : null
    const sustained = cells.find(({ cell }) => cell.key === WEAKNESS_KEY.density('sustained'))?.cell
    const weaknesses = [
        ...cells.filter(({ cell }) => cell.leaning === TAG_LEANING.WEAKNESS && isComboWeakness(cell)).map(cellItem),
        ...(density.evidence === EVIDENCE.STRONG ? [densityItem(density, densityTrend)] : []),
    ]
        .toSorted(bySeverity)
        .slice(0, PROFILE_WEAKNESS_TOP)
    const strengths = cells
        .filter(({ cell }) => cell.leaning === TAG_LEANING.STRENGTH)
        .map(cellItem)
        .toSorted(bySeverity)
        .slice(0, PROFILE_STRENGTH_TOP)
    const view = {
        profile_version: PROFILE_VERSION,
        window: {
            from: window.fromMs,
            to: window.toMs,
            days: window.days,
            plays: measured.length,
            charts: new Set(measured.map((play) => play.chartSha256)).size,
            notes: sumOf(measured.map((play) => play.metrics.overall.notes)),
            stale: window.stale,
            auto_plays: measured.filter((play) => play.auto).length,
            training_plays: measured.filter((play) => play.isTraining).length,
        },
        density: {
            curve: density.curve,
            baseline_ex: density.baseline_ex,
            comfort_nps: density.comfort_nps,
            limit_nps_ex: density.limit_nps_ex,
            limit_nps_bp: density.limit_nps_bp,
            max_seen_nps: density.max_seen_nps,
            peak_seen_2s: density.peak_seen_2s,
            ceiling_reached_ex: density.ceiling_reached_ex,
            evidence: density.evidence,
            sustained_minus_burst: {
                value: sustained?.value ?? null,
                evidence: sustained?.evidence ?? EVIDENCE.INSUFFICIENT,
                sample_level: sustained?.sample_level ?? 'insufficient',
            },
        },
        cells: cells.map(({ cell }) => cell),
        weaknesses,
        strengths,
        bp_runs: bpRunsOf(measured),
        gauge: gaugeOf(measured),
        soflan: soflanOf(measured),
        notice: measured.length < PROFILE_MIN_PLAYS ? { need: 'traced_plays' } : null,
    } satisfies WeaknessProfileView
    return { view, density }
}

/**
 * The weakness profile of a player (plan 2026-11-04 2.5) from their analysed plays of one window: every cell (tags, tags alone, tag
 * pairs, lanes, hands, tag timing, BPM bands, soflan phases, fatigue, sustained density, failure and gauge-drop contexts) with its
 * value, sample and grade, the density limit curve, the strongest weaknesses and strengths with the keys other features act on, and
 * the window's BP runs, gauge drops and soflan adjustment. Plays of the player's training charts are one cluster; plays under SCRATCH
 * AUTO stay out of the density and scratch cells. The same plays give the same profile whatever their order.
 */
export const computeWeaknessProfile = (plays: readonly ProfilePlay[], window: ProfileWindow) => buildProfile(plays, window).view

/** What one weakness key measures over some plays: its value, unit and sample, or null when its sample conditions are not met. */
export const measureProfileKeys = (plays: readonly ProfilePlay[], window: ProfileWindow, keys: readonly string[]) => {
    const { view, density } = buildProfile(plays, window)
    const cells = new Map(view.cells.map((cell) => [cell.key, cell]))
    return Object.fromEntries(
        keys.map((key) => {
            if (key === WEAKNESS_KEY.density('limit')) {
                const measured =
                    density.evidence === EVIDENCE.INSUFFICIENT || density.limit_nps_ex === null
                        ? null
                        : { value: density.limit_nps_ex, unit: 'nps' as const, ...density.sample, evidence: density.evidence }
                return [key, measured]
            }
            const cell = cells.get(key)
            const measured =
                cell && cell.evidence !== EVIDENCE.INSUFFICIENT && cell.value !== null
                    ? { value: cell.value, unit: cell.unit, notes: cell.notes, charts: cell.charts, plays: cell.plays, evidence: cell.evidence }
                    : null
            return [key, measured]
        }),
    )
}

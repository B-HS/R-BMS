import {
    TRAINING_SHAPES,
    type AiTrainingPlanOutput,
    type TrainingIntensity,
    type TrainingPatternKind,
    type TrainingShape,
} from '../../../../web/src/server/dto/training'
import { localDensitiesOf } from '../../../../web/src/server/service/domain/analysis/local-density'
import type { PatternNote } from '../../../../web/src/server/service/domain/analysis/pattern-tags'
import { writeTrainingBms } from '../../../../web/src/server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '../../../../web/src/server/service/domain/training/pattern-document'
import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf } from '../../../../web/src/server/service/domain/training/spec'
import { synthesizeTraining } from '../../../../web/src/server/service/domain/training/synthesize'

const PROFILES = [
    { name: 'P1', limit: 10 },
    { name: 'P2', limit: 16 },
    { name: 'P3', limit: 24 },
] as const
const INTENSITIES: TrainingIntensity[] = ['build', 'push', 'over']
const KIND_KEY: Record<Exclude<TrainingPatternKind, 'rest'>, string> = {
    stream: 'tag:stream',
    chord_stream: 'tag:chord_3',
    trill: 'tag:trill',
    stair: 'tag:stair',
    jack: 'tag:jack',
    chord_jack: 'tag:chord_3',
    scratch_solo: 'tag:scratch_solo',
    scratch_mixed: 'tag:scratch_mixed',
    scratch_run: 'tag:scratch_run',
    ln_hold: 'tag:ln_head',
    ln_release: 'tag:ln_release_then_hit',
    burst: 'tag:dense',
}
const KINDS = Object.keys(KIND_KEY) as (keyof typeof KIND_KEY)[]
const SEEDS = [1, 2, 3, 4, 5]
const BASE_BPM = Number(process.env.BASE_BPM ?? 160)
const LEVELS = (process.env.LEVELS ?? '0,0.5,1').split(',').map(Number)
const SCENARIO = process.env.SCENARIO ?? 'typical'
const BLOCKS = 12

const KIND_NUM = ['normal', 'ln_head', 'ln_end'] as const
const RANGE = {
    build: [0.86, 1.05],
    push: [0.95, 1.15],
    over: [1.1, 1.36],
} as const

const rank = (sorted: number[], q: number) => sorted[Math.max(0, Math.ceil(q * sorted.length) - 1)] ?? 0
const r2 = (v: number) => Math.round(v * 100) / 100
const mean = (values: number[]) => values.reduce((a, b) => a + b, 0) / Math.max(1, values.length)

const section = (focus_key: string, kind: TrainingPatternKind, weight: number) => ({
    focus_key,
    kind,
    weight,
    lane_hint: 'all' as const,
    chord_size: kind === 'chord_stream' || kind === 'chord_jack' ? 3 : 1,
    scratch_ratio: 0,
    ln_ratio: 0,
    bpm_ratio: 1,
    intent: '',
})

const planOf = (kind: keyof typeof KIND_KEY, level: number, shape: TrainingShape): AiTrainingPlanOutput => {
    const key = KIND_KEY[kind]
    const focus = [0, 1, 2].map(() => section(key, kind, 3))
    const sections = SCENARIO === 'focus' ? focus : [section('warmup', 'stream', 1), ...focus]
    return {
        description: '',
        advice: '',
        base_bpm: BASE_BPM,
        level,
        shape,
        sections,
    }
}

const measure = (
    profile: (typeof PROFILES)[number],
    intensity: TrainingIntensity,
    kind: keyof typeof KIND_KEY,
    shape: TrainingShape,
    level: number,
    seed: number,
) => {
    const bounds = trainingBoundsOf({ limit_nps_ex: profile.limit, max_seen_nps: null }, intensity)
    const allowed = {
        kinds: [],
        lane_hints: [],
        keys: [KIND_KEY[kind], 'warmup'],
    }
    const corrected = correctTrainingPlan(planOf(kind, level, shape), {
        allowed,
        bounds,
    })
    const id = `tc_${'0'.repeat(31)}${seed}`
    const input = trainingSpecInputOf({
        id,
        userId: 'diag',
        createdAt: 1_790_000_000_000 + seed,
        intensity,
        plan: corrected.spec,
        corrections: corrected.corrections,
    })
    const { spec, notes } = synthesizeTraining(input)
    const written = writeTrainingBms(spec, notes)
    const doc = buildTrainingPatternDocument({
        spec,
        lines: written.lines,
        md5: written.md5,
        sha256: written.sha256,
    })
    const placed: PatternNote[] = doc.notes.time_us.map((timeUs, i) => ({
        timeUs,
        lane: doc.notes.lane[i],
        kind: KIND_NUM[doc.notes.kind[i]],
    }))
    const dens = localDensitiesOf(placed)
    const body = spec.sections.filter((s) => s.focus_key !== 'warmup')
    const bodyFrom = body[0].from_us
    const bodyTo = Math.min(59_000_000, body[body.length - 1].to_us)
    const bodyNotes = placed.flatMap((n, i) => (n.kind !== 'ln_end' && n.timeUs >= bodyFrom ? [{ t: n.timeUs, nps2: dens[i].nps2 }] : []))
    const nps2 = bodyNotes.map((n) => n.nps2).toSorted((a, b) => a - b)
    const blockUs = (bodyTo - bodyFrom) / BLOCKS
    const blocks = Array.from(
        { length: BLOCKS },
        (_, j) => bodyNotes.filter((n) => n.t >= bodyFrom + j * blockUs && n.t < bodyFrom + (j + 1) * blockUs).length,
    )
    const blockMean = mean(blocks)
    const cv = Math.sqrt(mean(blocks.map((b) => (b - blockMean) ** 2))) / blockMean
    const p50 = rank(nps2, 0.5)
    const p90 = rank(nps2, 0.9)
    const max = nps2.at(-1) ?? 0
    const ratios = body.map((s) => s.actual_nps / s.target_nps)
    return {
        p90L: p90 / profile.limit,
        maxP50: max / p50,
        cv,
        ratioMin: Math.min(...ratios),
        ratioMax: Math.max(...ratios),
        avgL: doc.playable_notes / 60 / profile.limit,
        p90,
        max,
        p50,
        rps: mean(body.map((s) => s.actual_rps ?? 0)),
        level: spec.play_level,
        bpm: spec.base_bpm,
        mu: spec.bounds.mean_nps,
        target: spec.bounds.p90_target,
        chordFills: spec.corrections.filter((correction) => correction.field === 'chord_fill').length,
        notesPerRowMax: Math.max(
            0,
            ...spec.corrections.flatMap((correction) =>
                correction.field === 'chord_fill' && typeof correction.to === 'number' ? [correction.to] : [],
            ),
        ),
    }
}

type Run = ReturnType<typeof measure>
const runs: {
    profile: string
    intensity: TrainingIntensity
    kind: string
    shape: TrainingShape
    level: number
    run: Run
}[] = []
for (const profile of PROFILES)
    for (const intensity of INTENSITIES)
        for (const kind of KINDS)
            for (const shape of TRAINING_SHAPES)
                for (const level of LEVELS)
                    for (const seed of SEEDS)
                        runs.push({
                            profile: profile.name,
                            intensity,
                            kind,
                            shape,
                            level,
                            run: measure(profile, intensity, kind, shape, level, seed),
                        })

const summarize = (label: string, picked: typeof runs) => {
    const of = (pick: (r: Run) => number) => picked.map((entry) => pick(entry.run))
    const lo = (pick: (r: Run) => number) => r2(Math.min(...of(pick)))
    const hi = (pick: (r: Run) => number) => r2(Math.max(...of(pick)))
    const av = (pick: (r: Run) => number) => r2(mean(of(pick)))
    return `${label}\tn=${picked.length}\tp90/L ${lo((r) => r.p90L)}~${hi((r) => r.p90L)} (avg ${av((r) => r.p90L)})\tmax/p50 ${lo((r) => r.maxP50)}~${hi((r) => r.maxP50)} (avg ${av((r) => r.maxP50)})\tcv ${lo((r) => r.cv)}~${hi((r) => r.cv)} (avg ${av((r) => r.cv)})\tactual/target ${lo((r) => r.ratioMin)}~${hi((r) => r.ratioMax)}\tavg/L ${lo((r) => r.avgL)}~${hi((r) => r.avgL)} (avg ${av((r) => r.avgL)})\trps ${av((r) => r.rps)}\tbpm ${lo((r) => r.bpm)}~${hi((r) => r.bpm)}\tlevel ${lo((r) => r.level)}~${hi((r) => r.level)}`
}

const failures = (entry: (typeof runs)[number]) => {
    const [low, high] = RANGE[entry.intensity]
    const isFlat = entry.shape === 'flat'
    return [
        entry.run.p90L < low || entry.run.p90L > high ? 'c1' : null,
        entry.run.maxP50 < 1.25 ? 'c2max' : null,
        entry.run.cv < (isFlat ? 0.12 : 0.15) || entry.run.cv > 0.35 ? 'c2cv' : null,
        entry.run.ratioMin < 0.93 || entry.run.ratioMax > 1.07 ? 'c3' : null,
        entry.intensity === 'push' && entry.run.avgL < 0.7 ? 'c4' : null,
    ].filter((item) => item !== null)
}

const lines: string[] = []
lines.push(`# scenario=${SCENARIO} base_bpm=${BASE_BPM} levels=${LEVELS.join(',')} seeds=${SEEDS.length}`)
lines.push('## profile x intensity')
for (const profile of PROFILES)
    for (const intensity of INTENSITIES)
        lines.push(
            summarize(
                `${profile.name} ${intensity}`,
                runs.filter((e) => e.profile === profile.name && e.intensity === intensity),
            ),
        )
lines.push('## intensity x level')
for (const intensity of INTENSITIES)
    for (const level of LEVELS)
        lines.push(
            summarize(
                `${intensity} level ${level}`,
                runs.filter((e) => e.intensity === intensity && e.level === level),
            ),
        )
lines.push('## kind')
for (const kind of KINDS)
    lines.push(
        summarize(
            kind,
            runs.filter((e) => e.kind === kind),
        ),
    )
lines.push('## shape')
for (const shape of TRAINING_SHAPES)
    lines.push(
        summarize(
            shape,
            runs.filter((e) => e.shape === shape),
        ),
    )
const failed = runs.map((entry) => ({ entry, reasons: failures(entry) })).filter((item) => item.reasons.length > 0)
lines.push(`## failures ${failed.length} / ${runs.length}`)
const REASONS = ['c1', 'c2max', 'c2cv', 'c3', 'c4'] as const
lines.push(`by reason: ${REASONS.map((reason) => `${reason} ${failed.filter((item) => item.reasons.includes(reason)).length}`).join(', ')}`)
for (const intensity of INTENSITIES)
    lines.push(
        `by reason ${intensity} (${runs.filter((e) => e.intensity === intensity).length}): ${REASONS.map((reason) => `${reason} ${failed.filter((item) => item.entry.intensity === intensity && item.reasons.includes(reason)).length}`).join(', ')}`,
    )
for (const intensity of INTENSITIES)
    for (const level of LEVELS)
        lines.push(
            `by reason ${intensity} level ${level}: ${REASONS.map((reason) => `${reason} ${failed.filter((item) => item.entry.intensity === intensity && item.entry.level === level && item.reasons.includes(reason)).length}`).join(', ')}`,
        )
for (const kind of KINDS)
    lines.push(
        `by reason ${kind}: ${REASONS.map((reason) => `${reason} ${failed.filter((item) => item.entry.kind === kind && item.reasons.includes(reason)).length}`).join(', ')}`,
    )
for (const profile of PROFILES)
    for (const intensity of INTENSITIES)
        lines.push(
            `by reason ${profile.name} ${intensity} (${runs.filter((e) => e.profile === profile.name && e.intensity === intensity).length}): ${REASONS.map((reason) => `${reason} ${failed.filter((item) => item.entry.profile === profile.name && item.entry.intensity === intensity && item.reasons.includes(reason)).length}`).join(', ')}`,
        )
const chordFilled = runs.filter((entry) => entry.run.chordFills > 0)
lines.push(
    `chord_fill: charts ${chordFilled.length} / ${runs.length}, most notes per row ${r2(Math.min(...chordFilled.map((e) => e.run.notesPerRowMax)))}~${r2(Math.max(...chordFilled.map((e) => e.run.notesPerRowMax)))}`,
)
for (const profile of PROFILES)
    lines.push(
        `by reason ${profile.name}: ${REASONS.map((reason) => `${reason} ${failed.filter((item) => item.entry.profile === profile.name && item.reasons.includes(reason)).length}`).join(', ')}`,
    )
if (process.env.DETAIL !== '1') {
    console.log(lines.join('\n'))
    process.exit(0)
}
const groups = Map.groupBy(failed, (item) => `${item.entry.profile} ${item.entry.intensity} ${item.entry.kind} ${item.reasons.join('+')}`)
for (const [key, items] of groups) {
    const sample = items[0].entry.run
    lines.push(
        `${key}\t${items.length}\tp90/L ${r2(sample.p90L)} max/p50 ${r2(sample.maxP50)} cv ${r2(sample.cv)} ratio ${r2(sample.ratioMin)}~${r2(sample.ratioMax)} avg/L ${r2(sample.avgL)} bpm ${sample.bpm}`,
    )
}
console.log(lines.join('\n'))

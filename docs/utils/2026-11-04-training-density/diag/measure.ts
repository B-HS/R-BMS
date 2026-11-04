import type { AiTrainingPlanOutput, TrainingIntensity, TrainingPatternKind } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/dto/training'
import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/spec'
import { synthesizeTraining } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/synthesize'
import { writeTrainingBms } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/pattern-document'
import { localDensitiesOf, peakNpsOf, peakRowsPerSecondOf } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/analysis/local-density'
import type { PatternNote } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/analysis/pattern-tags'

const PROFILES = [
    { name: 'P1', comfort: 8, limit: 10, trueLimit: 11 },
    { name: 'P2', comfort: 12, limit: 16, trueLimit: 16 },
    { name: 'P3', comfort: 18, limit: 24, trueLimit: 24 },
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
const SCENARIO = process.env.SCENARIO ?? 'typical'
const TARGET = process.env.TARGET ?? 'max'

const KIND_NUM = ['normal', 'ln_head', 'ln_end'] as const

const pct = (sorted: number[], p: number) => sorted[Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)] ?? 0
const r2 = (v: number) => Math.round(v * 100) / 100

const section = (focus_key: string, kind: TrainingPatternKind, weight: number, target_nps: number) => ({
    focus_key,
    kind,
    weight,
    target_nps,
    lane_hint: 'all' as const,
    chord_size: kind === 'chord_stream' || kind === 'chord_jack' ? 3 : 1,
    scratch_ratio: 0,
    ln_ratio: 0,
    bpm_ratio: 1,
    intent: '',
})

const planOf = (kind: keyof typeof KIND_KEY, bounds: ReturnType<typeof trainingBoundsOf>): AiTrainingPlanOutput => {
    const key = KIND_KEY[kind]
    const focusTarget = TARGET === 'max' ? 999 : (bounds.nps_min + bounds.nps_max) / 2
    if (SCENARIO === 'focus')
        return { description: '', advice: '', base_bpm: BASE_BPM, sections: [0, 1, 2].map(() => section(key, kind, 3, focusTarget)) }
    return {
        description: '',
        advice: '',
        base_bpm: BASE_BPM,
        sections: [
            section('warmup', 'stream', 2, bounds.comfort_nps),
            section(key, kind, 3, focusTarget),
            section(key, kind, 3, focusTarget),
            section(key, kind, 3, focusTarget),
            section('cooldown', 'rest', 1, 2),
        ],
    }
}

const measure = (profile: (typeof PROFILES)[number], intensity: TrainingIntensity, kind: keyof typeof KIND_KEY, seed: number) => {
    const bounds = trainingBoundsOf({ comfort_nps: profile.comfort, limit_nps_ex: profile.limit, max_seen_nps: null }, intensity)
    const plan = planOf(kind, bounds)
    const allowed = { kinds: [], lane_hints: [], keys: [KIND_KEY[kind], 'warmup', 'cooldown'] }
    const corrected = correctTrainingPlan(plan, { allowed: allowed as never, bounds })
    const id = `tc_${'0'.repeat(31)}${seed}`
    const input = trainingSpecInputOf({ id, userId: 'diag', createdAt: 1_790_000_000_000 + seed, intensity, plan: corrected.spec, corrections: corrected.corrections })
    const { spec, notes } = synthesizeTraining(input)
    const written = writeTrainingBms(spec, notes)
    const doc = buildTrainingPatternDocument({ spec, lines: written.lines, md5: written.md5, sha256: written.sha256 })
    const patternNotes: PatternNote[] = doc.notes.time_us.map((timeUs, i) => ({ timeUs, lane: doc.notes.lane[i], kind: KIND_NUM[doc.notes.kind[i]] }))
    const dens = localDensitiesOf(patternNotes)
    const judged = patternNotes.flatMap((n, i) => (n.kind === 'ln_end' ? [] : [dens[i].nps2]))
    const nps2 = judged.toSorted((a, b) => a - b)
    const focus = spec.sections.filter((s) => s.focus_key === KIND_KEY[kind])
    const focusUs = focus.reduce((sum, s) => sum + (s.to_us - s.from_us), 0)
    const focusNotes = focus.reduce((sum, s) => sum + s.notes, 0)
    const focusTimes = patternNotes.filter((n) => n.kind !== 'ln_end').map((n) => n.timeUs)
    const focusRows = new Set(
        focusTimes.filter((t) => focus.some((s) => t >= s.from_us && t < s.to_us)),
    ).size
    const edgeUs = spec.sections.filter((s) => s.focus_key === 'warmup' || s.focus_key === 'cooldown').reduce((sum, s) => sum + (s.to_us - s.from_us), 0)
    const firstUs = focusTimes[0] ?? 0
    const lastUs = focusTimes.at(-1) ?? 0
    const nps1Max = Math.max(...patternNotes.flatMap((n, i) => (n.kind === 'ln_end' ? [] : [dens[i].nps1])))
    return {
        bounds,
        target: focus[0]?.target_nps ?? 0,
        focusActual: focusUs > 0 ? (focusNotes * 1e6) / focusUs : 0,
        focusRowsPerSec: focusUs > 0 ? (focusRows * 1e6) / focusUs : 0,
        focusShare: focusUs / 60e6,
        edgeShare: edgeUs / 60e6,
        countInUs: spec.sections[0].from_us,
        grid: focus[0]?.grid ?? 0,
        p50: pct(nps2, 50),
        p90: pct(nps2, 90),
        max: nps2.at(-1) ?? 0,
        nps1Max,
        peak2s: peakNpsOf(patternNotes)['2000'],
        peakRps2s: peakRowsPerSecondOf(patternNotes),
        avg60: doc.playable_notes / 60,
        avgSpan: lastUs > firstUs ? (doc.playable_notes * 1e6) / (lastUs - firstUs) : 0,
        notes: doc.playable_notes,
        playLevel: spec.play_level,
        total: doc.total,
        corrections: corrected.corrections.filter((c) => c.field === 'target_nps' && c.section !== null).map((c) => `${c.from}->${c.to}`),
    }
}

const mean = (values: number[]) => values.reduce((a, b) => a + b, 0) / values.length

const rows: Record<string, unknown>[] = []
for (const profile of PROFILES) {
    for (const intensity of INTENSITIES) {
        for (const kind of KINDS) {
            const runs = SEEDS.map((seed) => measure(profile, intensity, kind, seed))
            const m = (pick: (r: (typeof runs)[number]) => number) => r2(mean(runs.map(pick)))
            const levels = runs.map((r) => r.playLevel)
            rows.push({
                profile: profile.name,
                intensity,
                kind,
                nps_min: runs[0].bounds.nps_min,
                nps_max: runs[0].bounds.nps_max,
                target: r2(runs[0].target),
                actual: m((r) => r.focusActual),
                act_over_target: r2(mean(runs.map((r) => r.focusActual / r.target))),
                rows_s: m((r) => r.focusRowsPerSec),
                grid: runs[0].grid,
                p50: m((r) => r.p50),
                p90: m((r) => r.p90),
                max: m((r) => r.max),
                nps1max: m((r) => r.nps1Max),
                peak2s: m((r) => r.peak2s),
                peakRps2s: m((r) => r.peakRps2s),
                avg60: m((r) => r.avg60),
                avgSpan: m((r) => r.avgSpan),
                p50_L: r2(mean(runs.map((r) => r.p50)) / profile.limit),
                p90_L: r2(mean(runs.map((r) => r.p90)) / profile.limit),
                max_L: r2(mean(runs.map((r) => r.max)) / profile.limit),
                act_L: r2(mean(runs.map((r) => r.focusActual)) / profile.limit),
                act_trueL: r2(mean(runs.map((r) => r.focusActual)) / profile.trueLimit),
                notes: m((r) => r.notes),
                level: `${Math.min(...levels)}-${Math.max(...levels)}`,
                total: m((r) => r.total),
                focusShare: m((r) => r.focusShare),
                edgeShare: m((r) => r.edgeShare),
                countInMs: Math.round(runs[0].countInUs / 1000),
                corr: runs[0].corrections.join(' '),
            })
        }
    }
}
console.log(JSON.stringify(rows))

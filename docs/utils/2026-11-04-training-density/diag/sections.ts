import type { AiTrainingPlanOutput, TrainingIntensity, TrainingPatternKind, TrainingShape } from '../../../../web/src/server/dto/training'
import { localDensitiesOf } from '../../../../web/src/server/service/domain/analysis/local-density'
import type { PatternNote } from '../../../../web/src/server/service/domain/analysis/pattern-tags'
import { writeTrainingBms } from '../../../../web/src/server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '../../../../web/src/server/service/domain/training/pattern-document'
import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf } from '../../../../web/src/server/service/domain/training/spec'
import { synthesizeTraining } from '../../../../web/src/server/service/domain/training/synthesize'

const KIND_NUM = ['normal', 'ln_head', 'ln_end'] as const
const sec = (focus_key: string, kind: TrainingPatternKind, weight: number, chord = 1) => ({
    focus_key,
    kind,
    weight,
    lane_hint: 'all' as const,
    chord_size: chord,
    scratch_ratio: 0,
    ln_ratio: 0,
    bpm_ratio: 1,
    intent: '',
})

const run = (
    label: string,
    intensity: TrainingIntensity,
    limit: number,
    level: number,
    shape: TrainingShape,
    sections: ReturnType<typeof sec>[],
    keys: string[],
) => {
    const bounds = trainingBoundsOf({ limit_nps_ex: limit, max_seen_nps: null }, intensity)
    const plan: AiTrainingPlanOutput = {
        description: '',
        advice: '',
        base_bpm: 160,
        level,
        shape,
        sections,
    }
    const corrected = correctTrainingPlan(plan, {
        allowed: { kinds: [], lane_hints: [], keys: [...keys, 'warmup'] },
        bounds,
    })
    const input = trainingSpecInputOf({
        id: `tc_${'0'.repeat(31)}1`,
        userId: 'diag',
        createdAt: 1_790_000_000_001,
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
    const pn: PatternNote[] = doc.notes.time_us.map((timeUs, i) => ({
        timeUs,
        lane: doc.notes.lane[i],
        kind: KIND_NUM[doc.notes.kind[i]],
    }))
    const dens = localDensitiesOf(pn)
    console.log(
        `## ${label} bounds=${JSON.stringify(spec.bounds)} base_bpm=${spec.base_bpm} play_level=${spec.play_level} notes=${doc.playable_notes} total=${doc.total}`,
    )
    for (const s of spec.sections) {
        const inside = pn
            .flatMap((n, i) => (n.kind !== 'ln_end' && n.timeUs >= s.from_us && n.timeUs < s.to_us ? [dens[i].nps2] : []))
            .toSorted((a, b) => a - b)
        const p = (q: number) => inside[Math.max(0, Math.ceil(q * inside.length) - 1)] ?? 0
        console.log(
            [
                s.index,
                s.focus_key,
                s.kind,
                s.measure_count,
                ((s.to_us - s.from_us) / 1e6).toFixed(2),
                s.grid,
                s.target_nps,
                s.actual_nps,
                s.actual_rps,
                s.notes,
                p(0.5),
                p(0.9),
                inside.at(-1) ?? 0,
            ].join('\t'),
        )
    }
    const series = Array.from({ length: 15 }, (_, k) => {
        const t = (k * 4 + 2) * 1e6
        const at = pn.findIndex((n) => n.timeUs >= t && n.kind !== 'ln_end')
        return at < 0 ? '-' : dens[at].nps2
    })
    console.log(`nps2@2,6,10..58s: ${series.join(' ')}`)
}

const focus = (key: string, kind: TrainingPatternKind, chord = 1) => [
    sec('warmup', 'stream', 1),
    sec(key, kind, 3, chord),
    sec(key, kind, 3, chord),
    sec(key, kind, 3, chord),
]
run('P2 push stream flat level 0.5', 'push', 16, 0.5, 'flat', focus('tag:stream', 'stream'), ['tag:stream'])
run('P2 push stream dip level 0.5', 'push', 16, 0.5, 'dip', focus('tag:stream', 'stream'), ['tag:stream'])
run('P2 push chord_stream x3 rising level 0.5', 'push', 16, 0.5, 'rising', focus('tag:chord_3', 'chord_stream', 3), ['tag:chord_3'])
run('P2 over burst front_dip level 1', 'over', 16, 1, 'front_dip', focus('tag:dense', 'burst'), ['tag:dense'])
run('P2 over ln_release flat level 1', 'over', 16, 1, 'flat', focus('tag:ln_release_then_hit', 'ln_release'), ['tag:ln_release_then_hit'])
run('P2 over stair flat level 1', 'over', 16, 1, 'flat', focus('tag:stair', 'stair'), ['tag:stair'])

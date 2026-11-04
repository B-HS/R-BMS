import type { AiTrainingPlanOutput, TrainingIntensity, TrainingPatternKind } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/dto/training'
import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/spec'
import { synthesizeTraining } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/synthesize'
import { writeTrainingBms } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/pattern-document'
import { localDensitiesOf } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/analysis/local-density'
import type { PatternNote } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/analysis/pattern-tags'

const KIND_NUM = ['normal', 'ln_head', 'ln_end'] as const
const sec = (focus_key: string, kind: TrainingPatternKind, weight: number, target_nps: number, chord = 1) => ({
    focus_key, kind, weight, target_nps, lane_hint: 'all' as const, chord_size: chord, scratch_ratio: 0, ln_ratio: 0, bpm_ratio: 1, intent: '',
})

const run = (label: string, intensity: TrainingIntensity, density: { comfort_nps: number; limit_nps_ex: number }, sections: ReturnType<typeof sec>[], keys: string[]) => {
    const bounds = trainingBoundsOf({ ...density, max_seen_nps: null }, intensity)
    const plan: AiTrainingPlanOutput = { description: '', advice: '', base_bpm: 160, sections }
    const corrected = correctTrainingPlan(plan, { allowed: { kinds: [], lane_hints: [], keys: [...keys, 'warmup', 'cooldown'] } as never, bounds })
    const input = trainingSpecInputOf({ id: `tc_${'0'.repeat(31)}1`, userId: 'diag', createdAt: 1_790_000_000_001, intensity, plan: corrected.spec, corrections: corrected.corrections })
    const { spec, notes } = synthesizeTraining(input)
    const written = writeTrainingBms(spec, notes)
    const doc = buildTrainingPatternDocument({ spec, lines: written.lines, md5: written.md5, sha256: written.sha256 })
    const pn: PatternNote[] = doc.notes.time_us.map((timeUs, i) => ({ timeUs, lane: doc.notes.lane[i], kind: KIND_NUM[doc.notes.kind[i]] }))
    const dens = localDensitiesOf(pn)
    console.log(`## ${label} bounds=${JSON.stringify({ min: bounds.nps_min, max: bounds.nps_max, comfort: bounds.comfort_nps })} play_level=${spec.play_level} notes=${doc.playable_notes} total=${doc.total}`)
    for (const s of spec.sections) {
        const inside = pn.flatMap((n, i) => (n.kind !== 'ln_end' && n.timeUs >= s.from_us && n.timeUs < s.to_us ? [dens[i].nps2] : [])).toSorted((a, b) => a - b)
        const p = (q: number) => inside[Math.max(0, Math.ceil(q * inside.length) - 1)] ?? 0
        console.log([s.index, s.focus_key, s.kind, s.measure_count, ((s.to_us - s.from_us) / 1e6).toFixed(2), s.grid, Math.round(s.target_nps * 100) / 100, s.actual_nps, s.notes, p(0.5), p(0.9), inside.at(-1) ?? 0].join('\t'))
    }
    const series = Array.from({ length: 15 }, (_, k) => {
        const t = (k * 4 + 2) * 1e6
        const at = pn.findIndex((n) => n.timeUs >= t && n.kind !== 'ln_end')
        return at < 0 ? '-' : dens[at].nps2
    })
    console.log(`nps2@2,6,10..58s: ${series.join(' ')}`)
}

const P2 = { comfort_nps: 12, limit_nps_ex: 16 }
run('P2 push stream (focus=max)', 'push', P2, [sec('warmup', 'stream', 2, 12), sec('tag:stream', 'stream', 3, 99), sec('tag:stream', 'stream', 3, 99), sec('tag:stream', 'stream', 3, 99), sec('cooldown', 'rest', 1, 2)], ['tag:stream'])
run('P2 push stream (LLM-like ramp 11/13/15)', 'push', P2, [sec('warmup', 'stream', 2, 10), sec('tag:stream', 'stream', 3, 11), sec('tag:stream', 'stream', 3, 13), sec('tag:stream', 'stream', 3, 15), sec('cooldown', 'rest', 1, 2)], ['tag:stream'])
run('P2 push chord_stream x3 (max)', 'push', P2, [sec('warmup', 'stream', 2, 12), sec('tag:chord_3', 'chord_stream', 3, 99, 3), sec('tag:chord_3', 'chord_jack', 3, 99, 3), sec('tag:chord_3', 'chord_stream', 3, 99, 3), sec('cooldown', 'rest', 1, 2)], ['tag:chord_3'])
run('P2 over burst (max)', 'over', P2, [sec('warmup', 'stream', 2, 12), sec('tag:dense', 'burst', 3, 99), sec('tag:dense', 'burst', 3, 99), sec('tag:dense', 'burst', 3, 99), sec('cooldown', 'rest', 1, 2)], ['tag:dense'])
run('P2 over ln_release (max)', 'over', P2, [sec('warmup', 'stream', 2, 12), sec('tag:ln_release_then_hit', 'ln_release', 3, 99), sec('tag:ln_release_then_hit', 'ln_release', 3, 99), sec('tag:ln_release_then_hit', 'ln_release', 3, 99), sec('cooldown', 'rest', 1, 2)], ['tag:ln_release_then_hit'])
run('P2 over stair (max)', 'over', P2, [sec('warmup', 'stream', 2, 12), sec('tag:stair', 'stair', 3, 99), sec('tag:stair', 'stair', 3, 99), sec('tag:stair', 'stair', 3, 99), sec('cooldown', 'rest', 1, 2)], ['tag:stair'])

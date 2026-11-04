import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/spec'
import { synthesizeTraining } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/synthesize'
for (const [intensity, c, l] of [['build', 8, 10], ['push', 12, 16], ['over', 12, 16]] as const) {
  const sec = (f: string, k: string, t: number) => ({ focus_key: f, kind: k as never, weight: 3, target_nps: t, lane_hint: 'all' as const, chord_size: 1, scratch_ratio: 0, ln_ratio: 0, bpm_ratio: 1, intent: '' })
  const bounds = trainingBoundsOf({ comfort_nps: c, limit_nps_ex: l, max_seen_nps: null }, intensity)
  const cc = correctTrainingPlan({ description: '', advice: '', base_bpm: 160, sections: [0, 1, 2].map(() => sec('tag:scratch_solo', 'scratch_solo', 99)) }, { allowed: { kinds: [], lane_hints: [], keys: ['tag:scratch_solo'] } as never, bounds })
  const { spec, notes } = synthesizeTraining(trainingSpecInputOf({ id: 'tc_' + '0'.repeat(32), userId: 'd', createdAt: 1, intensity, plan: cc.spec, corrections: cc.corrections }))
  const scratch = notes.filter((n) => n.lane === 7).length
  console.log(intensity, c, l, 'target', spec.sections[0].target_nps, 'grid', spec.sections[0].grid, 'scratch', scratch, 'keys', notes.length - scratch)
}

import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/spec'
import { synthesizeTraining } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/synthesize'
import { writeTrainingBms } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/training/pattern-document'
import { localDensitiesOf } from '/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/analysis/local-density'
const KIND_NUM = ['normal', 'ln_head', 'ln_end'] as const
for (const [kind, key] of [['burst', 'tag:dense'], ['stream', 'tag:stream']] as const) {
  const sec = (f: string, k: string, w: number, t: number) => ({ focus_key: f, kind: k as never, weight: w, target_nps: t, lane_hint: 'all' as const, chord_size: 1, scratch_ratio: 0, ln_ratio: 0, bpm_ratio: 1, intent: '' })
  const bounds = trainingBoundsOf({ comfort_nps: 12, limit_nps_ex: 16, max_seen_nps: null }, 'push')
  const c = correctTrainingPlan({ description: '', advice: '', base_bpm: 160, sections: [sec('warmup','stream',2,12), sec(key,kind,3,99), sec(key,kind,3,99), sec(key,kind,3,99), sec('cooldown','rest',1,2)] }, { allowed: { kinds: [], lane_hints: [], keys: [key, 'warmup', 'cooldown'] } as never, bounds })
  const { spec, notes } = synthesizeTraining(trainingSpecInputOf({ id: 'tc_' + '0'.repeat(31) + '1', userId: 'diag', createdAt: 1_790_000_000_001, intensity: 'push', plan: c.spec, corrections: c.corrections }))
  const w = writeTrainingBms(spec, notes)
  const doc = buildTrainingPatternDocument({ spec, lines: w.lines, md5: w.md5, sha256: w.sha256 })
  const pn = doc.notes.time_us.map((timeUs, i) => ({ timeUs, lane: doc.notes.lane[i], kind: KIND_NUM[doc.notes.kind[i]] }))
  const d = localDensitiesOf(pn)
  const counts: Record<string, number> = {}
  d.forEach((x, i) => { if (pn[i].kind !== 'ln_end') counts[x.context] = (counts[x.context] ?? 0) + 1 })
  const nps1 = d.map((x) => x.nps1).toSorted((a, b) => a - b)
  console.log(kind, JSON.stringify(counts), 'nps1 p50', nps1[Math.floor(nps1.length / 2)], 'nps1 max', nps1.at(-1))
}

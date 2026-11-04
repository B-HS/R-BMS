import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf } from '../../../../web/src/server/service/domain/training/spec'
import { synthesizeTraining } from '../../../../web/src/server/service/domain/training/synthesize'

for (const [intensity, limit] of [
    ['build', 6],
    ['build', 10],
    ['push', 16],
    ['over', 16],
] as const) {
    const sec = () => ({
        focus_key: 'tag:scratch_solo',
        kind: 'scratch_solo' as const,
        weight: 3,
        lane_hint: 'all' as const,
        chord_size: 1,
        scratch_ratio: 0,
        ln_ratio: 0,
        bpm_ratio: 1,
        intent: '',
    })
    const bounds = trainingBoundsOf({ limit_nps_ex: limit, max_seen_nps: null }, intensity)
    const cc = correctTrainingPlan(
        {
            description: '',
            advice: '',
            base_bpm: 160,
            level: 0.5,
            shape: 'flat',
            sections: [sec(), sec(), sec()],
        },
        {
            allowed: { kinds: [], lane_hints: [], keys: ['tag:scratch_solo'] },
            bounds,
        },
    )
    const { spec, notes } = synthesizeTraining(
        trainingSpecInputOf({
            id: `tc_${'0'.repeat(32)}`,
            userId: 'd',
            createdAt: 1,
            intensity,
            plan: cc.spec,
            corrections: cc.corrections,
        }),
    )
    const scratch = notes.filter((n) => n.lane === 7).length
    console.log(
        intensity,
        limit,
        'targets',
        spec.sections.map((s) => s.target_nps).join('/'),
        'grid',
        spec.sections[0].grid,
        'scratch',
        scratch,
        'keys',
        notes.length - scratch,
    )
}

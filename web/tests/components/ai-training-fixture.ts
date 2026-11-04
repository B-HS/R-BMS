import type {
    AiTrainingChart,
    AiTrainingChartDetail,
    AiTrainingChartList,
    AiTrainingPlay,
    AiWeaknessItem,
    AiWeaknessProfile,
} from '../../src/entities/ai.type'

export const TRAINING_CREATED_AT = 1_780_000_000_000
const DAY_MS = 86_400_000
const HEX_ID_WIDTH = 32
const SHA256_WIDTH = 64

export const trainingId = (index: number) => `tc_${index.toString(16).padStart(HEX_ID_WIDTH, '0')}`
export const trainingSha256 = (index: number) => index.toString(16).padStart(SHA256_WIDTH, '0')
export const trainingDownloadUrl = (index: number) => `http://localhost:3000/api/training/${trainingId(index)}/chart.zip`

export const trainingChart = (index: number, overrides: Partial<AiTrainingChart> = {}): AiTrainingChart => ({
    id: trainingId(index),
    sha256: trainingSha256(index),
    title: `R-BMS Training · 2026-11-0${index} · PUSH`,
    created_at: TRAINING_CREATED_AT - index * DAY_MS,
    focus_keys: ['tag:trill'],
    kinds: ['trill'],
    note_count: 600,
    bpm: 160,
    play_level: 12,
    length_ms: 55_000,
    download_url: trainingDownloadUrl(index),
    plays: 0,
    best_ex_ratio: null,
    last_played_at: null,
    ...overrides,
})

export const trainingList = (overrides: Partial<AiTrainingChartList> = {}): AiTrainingChartList => {
    const charts = overrides.charts ?? [
        trainingChart(1, { plays: 3, best_ex_ratio: 0.877 }),
        trainingChart(2, { focus_keys: ['tag:jack', 'lane:1'] }),
    ]
    return { charts, used: charts.length, limit: 20, can_create: charts.length < 20, needs_key: false, ...overrides }
}

const section = (index: number, focusKey: string, kind: AiTrainingChartDetail['sections'][number]['kind'], laneHint: 'all' | 'left') => ({
    index,
    focus_key: focusKey,
    kind,
    from_ms: index * 12_000,
    to_ms: (index + 1) * 12_000,
    target_nps: 8 + index,
    actual_nps: 8.4 + index,
    actual_rps: index === 0 ? null : 6.2 + index,
    lane_hint: laneHint,
    intent: `intent ${index}`,
})

const playSection = (index: number, exRatio: number | null) => ({ index, ex_ratio: exRatio, bp: 2, mean_err_ms: 1.5, fast: 3, slow: 4 })

export const trainingPlay = (scoreId: string, overrides: Partial<AiTrainingPlay> = {}): AiTrainingPlay => ({
    score_id: scoreId,
    played_at: TRAINING_CREATED_AT + DAY_MS,
    clear: 6,
    ex_score: 900,
    ex_ratio: 0.9,
    bp: 12,
    random: 'Off',
    sections: null,
    ...overrides,
})

export const trainingDetail = (overrides: Partial<AiTrainingChartDetail> = {}): AiTrainingChartDetail => ({
    ...trainingChart(1),
    focus_keys: ['tag:trill', 'lane:1'],
    play_count: 4,
    description: 'Trill and left lane work',
    advice: 'Keep the wrists loose',
    sections: [
        section(0, 'warmup', 'stream', 'all'),
        section(1, 'tag:trill', 'trill', 'all'),
        section(2, 'lane:1', 'stream', 'left'),
        section(3, 'cooldown', 'stream', 'all'),
    ],
    corrections: [
        { section: 1, field: 'kind', from: 'jack', to: 'trill' },
        { section: 4, field: 'focus_key', from: 'tag:unknown', to: null },
        { section: null, field: 'base_bpm', from: 151, to: 152 },
    ],
    plays: [
        trainingPlay('score-newest', {
            played_at: TRAINING_CREATED_AT + 4 * DAY_MS,
            random: 'Mirror',
            ex_ratio: 0.93,
            sections: [playSection(1, 0.9), playSection(2, 0.6)],
        }),
        trainingPlay('score-middle', {
            played_at: TRAINING_CREATED_AT + 3 * DAY_MS,
            ex_ratio: 0.88,
            sections: [playSection(1, 0.86), playSection(2, 0.8)],
        }),
        trainingPlay('score-oldest', {
            played_at: TRAINING_CREATED_AT + 2 * DAY_MS,
            ex_ratio: 0.82,
            sections: [playSection(1, 0.8), playSection(2, 0.75)],
        }),
        trainingPlay('score-no-trace', { played_at: TRAINING_CREATED_AT + DAY_MS, ex_ratio: null, sections: null }),
    ],
    before_after: [
        {
            focus_key: 'tag:trill',
            unit: 'ex_residual',
            before: { value: -0.08, unit: 'ex_residual', notes: 900, charts: 12, evidence: 'strong' },
            after: { value: -0.04, unit: 'ex_residual', notes: 400, charts: 3, evidence: 'uncertain', plays: 6 },
            delta: 0.04,
        },
        {
            focus_key: 'lane:1',
            unit: 'ex_residual',
            before: { value: -0.05, unit: 'ex_residual', notes: 700, charts: 9, evidence: 'strong' },
            after: null,
            delta: null,
        },
    ],
    ...overrides,
})

export const trainingWeakness = (overrides: Partial<AiWeaknessItem> & Pick<AiWeaknessItem, 'key' | 'kind'>): AiWeaknessItem => ({
    label_tags: [],
    value: -0.06,
    unit: 'ex_residual',
    evidence: 'strong',
    sample_level: 'high',
    notes: 4200,
    charts: 25,
    plays: 60,
    severity: 0.06,
    trend_delta: null,
    selectable_for_folder: true,
    training_kinds: ['trill'],
    ...overrides,
})

const curveBin = (from: number, exRatio: number) => ({
    from_nps: from,
    to_nps: from + 2,
    notes: 400,
    charts: 6,
    ex_ratio: exRatio,
    ex_fit: exRatio,
    bp_rate: 0.02,
    bp_fit: 0.02,
})

export const trainingProfile = (overrides: Partial<AiWeaknessProfile> = {}): AiWeaknessProfile => ({
    profile_version: 1,
    window: { from: 0, to: 1, days: 90, plays: 120, charts: 40, notes: 98_000, stale: 0, auto_plays: 0, training_plays: 0 },
    density: {
        curve: [curveBin(4, 0.92), curveBin(8, 0.88), curveBin(12, 0.8), curveBin(16, 0.72)],
        baseline_ex: 0.89,
        comfort_nps: 10,
        limit_nps_ex: 16,
        limit_nps_bp: 16,
        max_seen_nps: 16,
        peak_seen_2s: 21.5,
        ceiling_reached_ex: true,
        evidence: 'strong',
        sustained_minus_burst: { value: -0.03, evidence: 'uncertain', sample_level: 'low' },
    },
    cells: [],
    weaknesses: [
        trainingWeakness({ key: 'tag:trill', kind: 'tag', label_tags: ['trill'] }),
        trainingWeakness({ key: 'lane:1', kind: 'lane', value: -0.04, training_kinds: ['stream'] }),
        trainingWeakness({ key: 'bp_context:stair', kind: 'bp_context', training_kinds: [], selectable_for_folder: false }),
        trainingWeakness({ key: 'tag:jack', kind: 'tag', label_tags: ['jack'], sample_level: 'insufficient', training_kinds: ['jack'] }),
    ],
    strengths: [],
    bp_runs: { longest_mean: null, histogram: {}, recovery_median: null },
    gauge: { drops_per_play: null, drop_mean_nps: null, overall_mean_nps: null },
    soflan: { events: 0, adjusted_share: null },
    notice: null,
    ...overrides,
})

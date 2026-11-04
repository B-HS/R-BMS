import { describe, expect, test } from 'bun:test'
import { FDR_Q, PROFILE_TEST_FAMILY } from '@server/service/domain/ai/play-feature.constants'
import type { CompactCell, TimingMoments } from '@server/service/domain/analysis/play-cells'
import { parseWeaknessKey } from '@server/service/domain/analysis/weakness-key'
import { computeWeaknessProfile, measureProfileKeys, type ProfilePlay, type ProfileWindow } from '@server/service/domain/analysis/weakness-profile'
import { analyzedPlayOf, tallyOf } from '../../support/play-feature-fixture'
import { readContractTrace } from '../../support/trace-fixture'

const DAY_MS = 86_400_000
const NOW = Date.UTC(2026, 10, 4)
const WINDOW: ProfileWindow = { fromMs: NOW - 90 * DAY_MS, toMs: NOW, recentFromMs: NOW - 30 * DAY_MS, days: 90, stale: 0 }
const RECENT = NOW - DAY_MS
const OLDER = NOW - 60 * DAY_MS

const BASE = analyzedPlayOf(readContractTrace('trace-normal.json')).summary
type Metrics = NonNullable<typeof BASE.metrics>

const cell = (notes: number, exScore: number, bp = 0): CompactCell => ({ notes, ex_score: exScore, bp, fast: 0, slow: 0 })
const NO_CELL = cell(0, 0)

/** Timing moments of `n` notes all `errUs` off (positive is early). */
const moments = (n: number, errUs: number): TimingMoments => ({
    n,
    sum_us: n * errUs,
    sumsq_ms2: n * (errUs / 1000) ** 2,
    fast_n: errUs >= 0 ? n : 0,
    fast_sum_us: errUs >= 0 ? n * errUs : 0,
    slow_n: errUs < 0 ? n : 0,
    slow_sum_us: errUs < 0 ? n * errUs : 0,
})

const BLANK: Metrics = {
    ...(BASE.metrics as Metrics),
    overall: tallyOf({}),
    by_tag: {},
    by_lane: {},
    by_nps: {},
    timing_by_tag: {},
    timing_by_lane: {},
    by_hand: {},
    timing_by_hand: {},
    lane_bp: {},
    by_combo: {},
    by_tag_solo: {},
    by_bpm_band: {},
    soflan_phase: { events: 0, before: NO_CELL, after: NO_CELL, settled: NO_CELL },
    fatigue: { first_third_ex_ratio: null, last_third_ex_ratio: null, delta: null },
    bp_runs: { histogram: { '1': 0, '2': 0, '3-4': 0, '5-9': 0, '10+': 0 }, longest: 0, runs: [], context_tags: {}, context_notes: 0 },
    gauge_drop_contexts: [],
    gauge_drop_tag_totals: {},
    gauge_drop_notes: 0,
    peak_nps: { '500': 0, '1000': 0, '2000': 0, '4000': 0, '8000': 0 },
}

/** A play of 400 notes at EX ratio 0.8 (320 PGREAT, 80 GOOD) unless `metrics` says otherwise. */
const play = (chart: string, metrics: Partial<Metrics>, options: { playedAt?: number; isTraining?: boolean; auto?: boolean } = {}): ProfilePlay => ({
    chartSha256: chart,
    playedAt: options.playedAt ?? RECENT,
    isTraining: options.isTraining ?? false,
    summary: { ...BASE, scratch_auto: options.auto ?? false, metrics: { ...BLANK, overall: tallyOf({ pgreat: 320, good: 80 }), ...metrics } },
})

/** A tag tally of 100 notes at this EX ratio (PGREAT and GOOD only, so the ratio is PGREAT over notes). */
const tagAt = (exRatio: number, notes = 100) => tallyOf({ pgreat: Math.round(exRatio * notes), good: notes - Math.round(exRatio * notes) })

const chartOf = (index: number) => `${String(index).padStart(2, '0')}`.padEnd(64, 'c')

const cellOf = (profile: ReturnType<typeof computeWeaknessProfile>, key: string) => profile.cells.find((entry) => entry.key === key)

/** One play per chart of `count` charts, its trill notes at `ratios[i]` against the play's 0.8. */
const trillPlays = (ratios: readonly number[], notes = 100, options: { playedAt?: number } = {}) =>
    ratios.map((ratio, index) => play(chartOf(index), { by_tag: { trill: tagAt(ratio, notes) } }, options))

describe('약점 프로필 — 표본·가중·결정론(기획 2.5.2)', () => {
    test('곡 군집 잔차는 판 EX 비율을 빼고, 한 곡 반복은 군집 가중 상한 500 노트로 묶인다', () => {
        const repeated = Array.from({ length: 10 }, (_, index) => play(chartOf(0), { by_tag: { trill: tagAt(0.5) } }, { playedAt: RECENT - index }))
        const others = [1, 2, 3, 4, 5].map((index) => play(chartOf(index), { by_tag: { trill: tagAt(0.8) } }))
        const trill = cellOf(computeWeaknessProfile([...repeated, ...others], WINDOW), 'tag:trill')
        expect(trill).toMatchObject({ value: -0.15, notes: 1500, charts: 6, plays: 15, ex_ratio: 0.6, unit: 'ex_residual' })
    })

    test('같은 판 목록은 순서가 달라도 같은 프로필이다', () => {
        const plays = [...trillPlays([0.5, 0.52, 0.48, 0.51, 0.49, 0.5]), ...trillPlays([0.7, 0.75], 100, { playedAt: OLDER })]
        expect(computeWeaknessProfile(plays.toReversed(), WINDOW)).toEqual(computeWeaknessProfile(plays, WINDOW))
    })

    test('표본 등급은 곡 수와 노트 수의 경계로 정하고 조건을 못 채우면 insufficient 이다', () => {
        const levelOf = (charts: number, notes: number) =>
            cellOf(
                computeWeaknessProfile(
                    trillPlays(
                        Array.from({ length: charts }, (_, index) => 0.5 + (index % 3) * 0.01),
                        notes,
                    ),
                    WINDOW,
                ),
                'tag:trill',
            )
        expect(levelOf(20, 150)).toMatchObject({ sample_level: 'high', notes: 3000, charts: 20 })
        expect(levelOf(19, 160)?.sample_level).toBe('medium')
        expect(levelOf(10, 100)).toMatchObject({ sample_level: 'medium', notes: 1000 })
        expect(levelOf(10, 99)?.sample_level).toBe('low')
        expect(levelOf(5, 60)).toMatchObject({ sample_level: 'low', notes: 300 })
        expect(levelOf(5, 59)).toMatchObject({ sample_level: 'insufficient', evidence: 'insufficient', p_value: null })
        expect(levelOf(4, 100)).toMatchObject({ sample_level: 'insufficient', evidence: 'insufficient' })
    })

    test('훈련곡 판은 곡이 달라도 군집 하나라 서로 다른 훈련곡 20곡이 K 에 1 만 보탠다', () => {
        const training = Array.from({ length: 20 }, (_, index) =>
            play(chartOf(50 + index), { by_tag: { trill: tagAt(0.5) } }, { isTraining: true, playedAt: RECENT - index }),
        )
        const profile = computeWeaknessProfile([...training, ...trillPlays([0.5, 0.52, 0.48, 0.51])], WINDOW)
        expect(cellOf(profile, 'tag:trill')).toMatchObject({ charts: 5, plays: 24, notes: 2400 })
        expect(profile.window).toMatchObject({ plays: 24, charts: 24, training_plays: 20 })
        const onlyTraining = computeWeaknessProfile(training, WINDOW)
        expect(cellOf(onlyTraining, 'tag:trill')).toMatchObject({ charts: 1, evidence: 'insufficient' })
    })

    test('7키 판이 PROFILE_MIN_PLAYS 판 아래면 안내가 붙는다', () => {
        expect(computeWeaknessProfile(trillPlays([0.5, 0.5, 0.5]), WINDOW).notice).toEqual({ need: 'traced_plays' })
        expect(computeWeaknessProfile(trillPlays(Array.from({ length: 10 }, () => 0.5)), WINDOW).notice).toBeNull()
        expect(computeWeaknessProfile([], WINDOW)).toMatchObject({ cells: [], weaknesses: [], strengths: [], notice: { need: 'traced_plays' } })
    })
})

describe('약점 프로필 — 검정·등급·키(기획 2.5.3, 2.5.5)', () => {
    test('곡마다 일관된 음의 잔차는 strong weakness 가 되어 키·연습 폴더 여부·훈련 종류와 함께 약점 목록에 오른다', () => {
        const profile = computeWeaknessProfile(trillPlays([0.5, 0.52, 0.48, 0.51, 0.49, 0.5]), WINDOW)
        expect(cellOf(profile, 'tag:trill')).toMatchObject({ evidence: 'strong', leaning: 'weakness', sample_level: 'low' })
        expect(profile.weaknesses[0]).toMatchObject({
            key: 'tag:trill',
            kind: 'tag',
            label_tags: ['trill'],
            unit: 'ex_residual',
            evidence: 'strong',
            selectable_for_folder: true,
            training_kinds: ['trill'],
        })
        expect(profile.weaknesses[0]?.severity).toBeGreaterThan(0)
        expect(profile.strengths).toEqual([])
    })

    test('곡마다 부호가 엇갈리면 표본은 충분해도 uncertain 이고 목록에 오르지 않는다', () => {
        const profile = computeWeaknessProfile(trillPlays([0.6, 1, 0.7, 0.95, 0.75, 0.85]), WINDOW)
        expect(cellOf(profile, 'tag:trill')).toMatchObject({ evidence: 'uncertain', leaning: null })
        expect(profile.weaknesses).toEqual([])
    })

    test('셀은 정확도와 타이밍 두 가족으로 나뉘어 가족마다 따로 순위를 매긴다', () => {
        const plays = [0, 1, 2, 3, 4, 5].map((index) =>
            play(chartOf(index), {
                by_tag: { trill: tagAt(0.5 + index * 0.01), stream: tagAt(0.79 + index * 0.004), jack: tagAt(0.7) },
                by_hand: { left: cell(200, 300), right: cell(200, 340) },
                timing_by_hand: { left: moments(200, 0), right: moments(200, 0) },
                timing_by_tag: { trill: moments(100, 3000 + index * 100), stream: moments(100, 200 * index), jack: moments(100, -2500) },
            }),
        )
        const profile = computeWeaknessProfile(plays, WINDOW)
        const sampled = profile.cells.filter((entry) => entry.sample_level !== 'insufficient')
        const familyOf = (kind: string) => PROFILE_TEST_FAMILY[kind as keyof typeof PROFILE_TEST_FAMILY]
        expect(new Set(sampled.map((entry) => familyOf(entry.kind)))).toEqual(new Set(['accuracy', 'timing']))
        for (const family of ['accuracy', 'timing']) {
            const members = sampled.filter((entry) => familyOf(entry.kind) === family)
            const ranks = members
                .map((entry) => Math.round(((entry.q_threshold ?? 0) * members.length) / FDR_Q))
                .toSorted((left, right) => left - right)
            expect(ranks, family).toEqual(members.map((_, index) => index + 1))
        }
        expect(sampled.filter((entry) => familyOf(entry.kind) === 'timing').map((entry) => entry.kind)).toEqual(['timing', 'timing', 'timing'])
    })

    test('타이밍 셀의 판 전체 평균은 손 4 개의 모멘트 합이라 여러 태그를 가진 노트가 한 번만 들어간다', () => {
        const plays = [-2, -1, 0, 1, 2].map((shift, index) =>
            play(chartOf(index), {
                timing_by_hand: { left: moments(100, 0), right: moments(100, 0) },
                timing_by_tag: { trill: moments(100, 2000 + shift * 1000), stream: moments(100, 2000 + shift * 1000) },
            }),
        )
        const profile = computeWeaknessProfile(plays, WINDOW)
        expect(cellOf(profile, 'timing:trill')).toMatchObject({
            value: 2,
            unit: 'ms',
            notes: 500,
            timing: { mean_ms: 2, sd_ms: 1.4142, fast_share: 1 },
        })
        expect(cellOf(profile, 'timing:stream')?.value).toBe(2)
    })

    test('조합은 두 태그 중 낮은 쪽보다 COMBO_INTERACTION_MIN 이상 더 잃을 때만 조합 약점으로 오른다', () => {
        const comboPlays = (comboRatio: number) =>
            [0, 1, 2, 3, 4, 5].map((index) =>
                play(chartOf(index), {
                    by_tag: { chord_2: tagAt(0.75 + index * 0.002), trill: tagAt(0.75 - index * 0.002) },
                    by_combo: { 'chord_2+trill': cell(100, Math.round(2 * 100 * (comboRatio + (index % 2) * 0.005))) },
                }),
            )
        const interacting = computeWeaknessProfile(comboPlays(0.6), WINDOW)
        expect(cellOf(interacting, 'combo:chord_2+trill')).toMatchObject({ evidence: 'strong', leaning: 'weakness' })
        expect(cellOf(interacting, 'combo:chord_2+trill')?.interaction).toBeLessThanOrEqual(-0.02)
        expect(interacting.weaknesses.map((item) => item.key)).toContain('combo:chord_2+trill')
        const additive = computeWeaknessProfile(comboPlays(0.74), WINDOW)
        expect(cellOf(additive, 'combo:chord_2+trill')).toMatchObject({ leaning: 'weakness' })
        expect(cellOf(additive, 'combo:chord_2+trill')?.interaction).toBeGreaterThan(-0.02)
        expect(additive.weaknesses.map((item) => item.key)).not.toContain('combo:chord_2+trill')
    })

    test('추세는 최근 30일 판과 그 이전 판의 추정 차이고 한쪽이라도 표본이 모자라면 null 이다', () => {
        const recent = trillPlays([0.5, 0.52, 0.48, 0.51, 0.49])
        const older = [10, 11, 12, 13, 14].map((index, at) =>
            play(chartOf(index), { by_tag: { trill: tagAt(0.7 + (at % 2) * 0.01) } }, { playedAt: OLDER }),
        )
        const trill = cellOf(computeWeaknessProfile([...recent, ...older], WINDOW), 'tag:trill')
        expect(trill?.trend_delta).toBeCloseTo(-0.204, 3)
        expect(cellOf(computeWeaknessProfile([...recent, ...older.slice(0, 4)], WINDOW), 'tag:trill')?.trend_delta).toBeNull()
    })

    test('약점 목록의 모든 키는 키 문법을 지킨다', () => {
        const profile = computeWeaknessProfile(trillPlays([0.5, 0.52, 0.48, 0.51, 0.49, 0.5]), WINDOW)
        expect([...profile.cells, ...profile.weaknesses].every((entry) => parseWeaknessKey(entry.key) !== null)).toBe(true)
    })
})

describe('약점 프로필 — 밀도 한계와 SCRATCH AUTO(기획 2.5.2, 2.5.4)', () => {
    const DENSITY_BINS = [
        { key: '4', exScore: 90, bp: 0 },
        { key: '8', exScore: 88, bp: 1 },
        { key: '12', exScore: 85, bp: 2 },
        { key: '16', exScore: 75, bp: 5 },
        { key: '20', exScore: 60, bp: 8 },
    ]
    const NOTES_PER_BIN = 50
    const densityMetrics = (shift = 0): Partial<Metrics> => ({
        overall: tallyOf({ pgreat: 200, good: 50 }),
        by_nps: Object.fromEntries(
            DENSITY_BINS.map(({ key, exScore, bp }) => [key, { ...cell(NOTES_PER_BIN, exScore + shift, bp), burst: NO_CELL, sustained: NO_CELL }]),
        ),
        peak_nps: { '500': 22, '1000': 21, '2000': 21.5, '4000': 20, '8000': 18 },
    })

    test('등위회귀 곡선에서 편한 밀도·EX 한계·BP 한계·본 최대 밀도를 읽고 한계가 strong 이면 약점에 오른다', () => {
        const plays = [0, 1, 2, 3, 4].map((index) => play(chartOf(index), densityMetrics()))
        const profile = computeWeaknessProfile(plays, WINDOW)
        expect(profile.density).toMatchObject({
            baseline_ex: 0.8767,
            comfort_nps: 14,
            limit_nps_ex: 15.9,
            limit_nps_bp: 15.7,
            max_seen_nps: 20,
            peak_seen_2s: 21.5,
            ceiling_reached_ex: true,
            evidence: 'strong',
        })
        expect(profile.density.curve.map((bin) => [bin.from_nps, bin.to_nps, bin.notes, bin.charts, bin.ex_fit, bin.bp_fit])).toEqual([
            [4, 6, 250, 5, 0.9, 0],
            [8, 10, 250, 5, 0.88, 0.02],
            [12, 14, 250, 5, 0.85, 0.04],
            [16, 18, 250, 5, 0.75, 0.1],
            [20, 22, 250, 5, 0.6, 0.16],
        ])
        expect(profile.weaknesses.find((item) => item.key === 'density:limit')).toMatchObject({
            value: 15.9,
            unit: 'nps',
            evidence: 'strong',
            severity: 0.2767,
            selectable_for_folder: true,
            training_kinds: ['stream', 'chord_stream'],
        })
    })

    test('EX 가 기준에서 크게 떨어지지 않으면 한계가 없고 uncertain, 쓸 칸이 3 개 미만이면 insufficient 이다', () => {
        const flat = [0, 1, 2, 3, 4].map((index) =>
            play(chartOf(index), {
                by_nps: {
                    '4': { ...cell(50, 90), burst: NO_CELL, sustained: NO_CELL },
                    '8': { ...cell(50, 89), burst: NO_CELL, sustained: NO_CELL },
                    '12': { ...cell(50, 88), burst: NO_CELL, sustained: NO_CELL },
                },
            }),
        )
        expect(computeWeaknessProfile(flat, WINDOW).density).toMatchObject({
            limit_nps_ex: null,
            limit_nps_bp: null,
            ceiling_reached_ex: false,
            evidence: 'uncertain',
        })
        const twoCharts = [0, 1].map((index) => play(chartOf(index), densityMetrics()))
        expect(computeWeaknessProfile(twoCharts, WINDOW).density).toMatchObject({ curve: [], evidence: 'insufficient', limit_nps_ex: null })
    })

    const curvePlays = (bins: readonly { key: string; exScore: number; bp?: number }[], options: { playedAt?: number } = {}) =>
        [0, 1, 2, 3, 4].map((index) =>
            play(
                chartOf(index),
                {
                    by_nps: Object.fromEntries(
                        bins.map(({ key, exScore, bp }) => [key, { ...cell(NOTES_PER_BIN, exScore, bp ?? 0), burst: NO_CELL, sustained: NO_CELL }]),
                    ),
                },
                options,
            ),
        )
    const limitsOf = (bins: Parameters<typeof curvePlays>[0]) => {
        const { limit_nps_ex, limit_nps_bp } = computeWeaknessProfile(curvePlays(bins), WINDOW).density
        return { limit_nps_ex, limit_nps_bp }
    }

    test('한계는 맞춤 곡선이 기준선을 지나는 점을 이웃 구간 중심 사이 직선으로 읽고, 쓰지 않은 구간이 끼어도 쓴 두 중심을 잇는다', () => {
        expect(
            limitsOf([
                { key: '4', exScore: 90 },
                { key: '8', exScore: 90 },
                { key: '12', exScore: 70 },
            ]),
        ).toEqual({ limit_nps_ex: 11, limit_nps_bp: null })
        expect(
            limitsOf([
                { key: '4', exScore: 90 },
                { key: '8', exScore: 90 },
                { key: '16', exScore: 70 },
            ]),
        ).toEqual({ limit_nps_ex: 13, limit_nps_bp: null })
        expect(
            limitsOf([
                { key: '4', exScore: 90, bp: 0 },
                { key: '8', exScore: 90, bp: 2 },
                { key: '12', exScore: 90, bp: 6 },
            ]),
        ).toEqual({ limit_nps_ex: null, limit_nps_bp: 11 })
    })

    test('마지막 구간에서 지나면 앞 구간 중심과 마지막 구간 중심 사이이고, 가장 높은 구간은 아래 끝을 중심으로 본다', () => {
        expect(
            limitsOf([
                { key: '32', exScore: 90 },
                { key: '36', exScore: 90 },
                { key: '40+', exScore: 70 },
            ]).limit_nps_ex,
        ).toBe(38.5)
    })

    test('첫 구간에서 이미 BP 한계를 넘었으면 앞 이웃이 없어 첫 구간 중심이고, 떨어진 적 없는 쪽은 null 이다', () => {
        expect(
            limitsOf([
                { key: '4', exScore: 90, bp: 5 },
                { key: '8', exScore: 90, bp: 5 },
                { key: '12', exScore: 90, bp: 5 },
            ]),
        ).toEqual({ limit_nps_ex: null, limit_nps_bp: 5 })
    })

    test('편한 밀도(구간 위 끝)가 보간한 EX·BP 한계보다 크면 둘 중 작은 한계로 자른다', () => {
        const densityOf = (bins: Parameters<typeof curvePlays>[0]) => {
            const { comfort_nps, limit_nps_ex, limit_nps_bp } = computeWeaknessProfile(curvePlays(bins), WINDOW).density
            return { comfort_nps, limit_nps_ex, limit_nps_bp }
        }
        expect(
            densityOf([
                { key: '4', exScore: 90 },
                { key: '8', exScore: 90 },
                { key: '10', exScore: 30 },
            ]),
        ).toEqual({ comfort_nps: 9.3, limit_nps_ex: 9.3, limit_nps_bp: null })
        expect(
            densityOf([
                { key: '4', exScore: 90 },
                { key: '8', exScore: 90 },
                { key: '10', exScore: 90, bp: 20 },
            ]),
        ).toEqual({ comfort_nps: 9.4, limit_nps_ex: null, limit_nps_bp: 9.4 })
    })

    test('밀도 한계 추세는 최근과 이전 한계의 차이를 소수 한 자리로 맞춘다', () => {
        const recent = curvePlays(DENSITY_BINS)
        const older = curvePlays(
            [
                { key: '4', exScore: 90 },
                { key: '8', exScore: 90 },
                { key: '12', exScore: 70 },
            ],
            { playedAt: OLDER },
        )
        const profile = computeWeaknessProfile([...recent, ...older], WINDOW)
        expect(profile.weaknesses.find((item) => item.key === 'density:limit')?.trend_delta).toBe(4.9)
    })

    test('SCRATCH AUTO 판만 있으면 밀도는 insufficient 이고, 섞이면 밀도·스크래치 셀은 auto 판을 빼고 세며 건반 셀은 넣는다', () => {
        const autoOnly = [0, 1, 2, 3, 4].map((index) => play(chartOf(index), densityMetrics(), { auto: true }))
        const onlyAuto = computeWeaknessProfile(autoOnly, WINDOW)
        expect(onlyAuto.density).toMatchObject({ curve: [], evidence: 'insufficient', peak_seen_2s: null })
        expect(onlyAuto.window.auto_plays).toBe(5)

        const keyed = (index: number, auto: boolean) =>
            play(
                chartOf(index),
                {
                    ...densityMetrics(auto ? -40 : 0),
                    by_tag: { trill: tagAt(0.5), scratch_chord: tagAt(0.5) },
                    by_lane: { '1': tagAt(0.7), S: tagAt(0.6) },
                },
                { auto },
            )
        const mixed = computeWeaknessProfile(
            [0, 1, 2, 3, 4].map((index) => keyed(index, false)).concat([5, 6, 7].map((index) => keyed(index, true))),
            WINDOW,
        )
        expect(cellOf(mixed, 'tag:trill')).toMatchObject({ plays: 8, charts: 8, auto_plays: 0 })
        expect(cellOf(mixed, 'tag:scratch_chord')).toMatchObject({ plays: 5, charts: 5, auto_plays: 3 })
        expect(cellOf(mixed, 'lane:S')).toMatchObject({ plays: 5, auto_plays: 3 })
        expect(cellOf(mixed, 'lane:1')).toMatchObject({ plays: 8, auto_plays: 0 })
        expect(mixed.density.curve.every((bin) => bin.notes === 250 && bin.charts === 5)).toBe(true)
        expect(mixed.density.limit_nps_ex).toBe(15.9)
        expect(mixed.window).toMatchObject({ plays: 8, auto_plays: 3 })
    })
})

describe('약점 프로필 — 실패·게이지 문맥과 그 밖의 셀(기획 2.5.4)', () => {
    test('실패 직전 태그는 문맥 노트 30 이상·lift 1.5 이상만 후보이고 로그 lift 로 검정한다', () => {
        const contextPlays = (contextTrill: number) =>
            [0, 1, 2, 3, 4, 5].map((index) =>
                play(chartOf(index), {
                    by_tag: { trill: tagAt(0.8, 40), stream: tagAt(0.8, 200) },
                    bp_runs: { ...BLANK.bp_runs, context_tags: { trill: contextTrill + (index % 2), stream: 8 }, context_notes: 20 },
                }),
            )
        const profile = computeWeaknessProfile(contextPlays(10), WINDOW)
        expect(cellOf(profile, 'bp_context:trill')).toMatchObject({ unit: 'ratio', notes: 63, charts: 6, evidence: 'strong', leaning: 'weakness' })
        expect(cellOf(profile, 'bp_context:trill')?.value).toBeCloseTo(5.25, 2)
        expect(cellOf(profile, 'bp_context:stream')).toBeUndefined()
        expect(profile.weaknesses.map((item) => item.key)).toContain('bp_context:trill')
        expect(cellOf(computeWeaknessProfile(contextPlays(3), WINDOW), 'bp_context:trill')).toBeUndefined()
    })

    test('소프란 국면·피로·BPM·지속 대 버스트 셀은 각자의 군집 값으로 검정한다', () => {
        const plays = [0, 1, 2, 3, 4, 5].map((index) =>
            play(chartOf(index), {
                soflan_phase: { events: 2, before: cell(100, 180), after: cell(100, 120 + index), settled: cell(100, 170 + index) },
                fatigue: { first_third_ex_ratio: 0.85, last_third_ex_ratio: 0.75, delta: -0.1 - index * 0.005 },
                by_bpm_band: { '150-179': cell(400, 640) },
                by_nps: {
                    '10': { ...cell(120, 190), burst: cell(60, 105 + index), sustained: cell(60, 90) },
                },
            }),
        )
        const profile = computeWeaknessProfile(plays, WINDOW)
        expect(cellOf(profile, 'soflan:after')).toMatchObject({ unit: 'ratio', evidence: 'strong', leaning: 'weakness' })
        expect(cellOf(profile, 'soflan:after')?.value).toBeCloseTo(-0.2875, 3)
        expect(cellOf(profile, 'soflan:settled')?.value).toBeCloseTo(-0.0375, 3)
        expect(cellOf(profile, 'fatigue')).toMatchObject({ evidence: 'strong', leaning: 'weakness', notes: 2400 })
        expect(cellOf(profile, 'bpm:150-179')).toMatchObject({ value: 0, evidence: 'uncertain' })
        expect(cellOf(profile, 'density:sustained')).toMatchObject({ notes: 720, charts: 6, leaning: 'weakness' })
        expect(profile.density.sustained_minus_burst.value).toBeCloseTo(-0.1458, 3)
    })

    test('이번 판 묶음의 피로·소프란 판정 비율·게이지·실패 요약을 셈한다', () => {
        const plays = [0, 1].map((index) =>
            play(chartOf(index), {
                bp_runs: {
                    ...BLANK.bp_runs,
                    histogram: { '1': 2, '2': 1, '3-4': index, '5-9': 0, '10+': 0 },
                    longest: 3 + index,
                    runs: [{ from_ms: 0, to_ms: 10, length: 3, start_nps: 9, recovery_notes: 10 + index * 10, preceding_tags: [] }],
                },
                gauge_drop_contexts: [{ from_ms: 0, to_ms: 100, chart_notes: 10, bp: 3, mean_nps: 12 + index * 2, top_tags: [] }],
                by_nps: { '6': { ...cell(400, 640), burst: NO_CELL, sustained: NO_CELL } },
            }),
        )
        const profile = computeWeaknessProfile(plays, WINDOW)
        expect(profile.bp_runs).toEqual({ longest_mean: 3.5, histogram: { '1': 4, '2': 2, '3-4': 1, '5-9': 0, '10+': 0 }, recovery_median: 15 })
        expect(profile.gauge).toMatchObject({ drop_mean_nps: 13, overall_mean_nps: 6.5 })
    })

    test('측정은 표본 조건을 채운 키의 값만 주고 나머지는 null 이다', () => {
        const plays = [0, 1, 2, 3, 4].map((index) => play(chartOf(index), { by_tag: { trill: tagAt(0.5 + (index % 2) * 0.01) } }))
        const measured = measureProfileKeys(plays, WINDOW, ['tag:trill', 'tag:jack', 'density:limit'])
        expect(measured['tag:trill']).toMatchObject({ unit: 'ex_residual', notes: 500, charts: 5, plays: 5 })
        expect(measured['tag:trill']?.value).toBeCloseTo(-0.296, 3)
        expect(measured['tag:jack']).toBeNull()
        expect(measured['density:limit']).toBeNull()
    })
})

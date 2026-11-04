import { describe, expect, test } from 'bun:test'
import {
    challengeWindowOf,
    findFrontierIndex,
    isDensityOver,
    practicePoolOf,
    rankWeaknessCharts,
    toCandidate,
    rankChallengeCandidates,
    readTableProgress,
    summarizeLevels,
    toChartProgress,
    topTagsOf,
    weaknessShareOf,
    type ChartFeatureExtras,
} from '@server/service/domain/ai/table-progress'
import { levelRecords, makeTable, md5Of, recordOf } from '../../support/ai-score-fixture'

const table = makeTable()

describe('발광 표 진행도와 경계 레벨', () => {
    test('클리어 비율 0.5 이상인 가장 높은 레벨의 다음이 경계다(아래 레벨을 안 쳤어도 끌려 내려가지 않는다)', () => {
        const records = levelRecords(table, { '3': 'Hard', '4': 'Easy' })
        const { levels, frontierIndex, frontier } = readTableProgress(table, records)
        expect(levels.map((level) => [level.level, level.cleared, level.clear_rate])).toEqual([
            ['1', 0, 0],
            ['2', 0, 0],
            ['3', 4, 1],
            ['4', 4, 1],
            ['5', 0, 0],
            ['6', 0, 0],
        ])
        expect(frontierIndex).toBe(4)
        expect(frontier?.level).toBe('5')
    })

    test('아무것도 깨지 못했으면 첫 레벨, 전부 깼으면 마지막 레벨, 레벨이 없으면 null', () => {
        expect(findFrontierIndex(readTableProgress(table, {}).levels)).toBe(0)
        const allClear = levelRecords(table, { '1': 'Easy', '2': 'Easy', '3': 'Easy', '4': 'Easy', '5': 'Easy', '6': 'Easy' })
        expect(findFrontierIndex(readTableProgress(table, allClear).levels)).toBe(5)
        expect(findFrontierIndex([])).toBeNull()
    })

    test('정확히 절반이면 깬 레벨로 보고 절반 아래면 아니다', () => {
        const half = { [md5Of(100)]: recordOf('Easy'), [md5Of(101)]: recordOf('AssistEasy'), [md5Of(102)]: recordOf('Failed') }
        expect(readTableProgress(table, half).levels[0]).toMatchObject({ played: 3, cleared: 2, clear_rate: 0.5 })
        expect(findFrontierIndex(readTableProgress(table, half).levels)).toBe(1)
        const below = { [md5Of(100)]: recordOf('Easy'), [md5Of(101)]: recordOf('Failed') }
        expect(findFrontierIndex(readTableProgress(table, below).levels)).toBe(0)
    })

    test('같은 md5 가 표에 두 번 있어도 한 번만 센다', () => {
        const repeated = { ...table, entries: [...table.entries, { md5: md5Of(100), level: '6', title: 'again', artist: '' }] }
        const charts = toChartProgress(repeated, {})
        expect(charts.filter((chart) => chart.md5 === md5Of(100))).toHaveLength(1)
        expect(summarizeLevels(repeated, charts).at(-1)?.charts).toBe(4)
    })
})

describe('다음 도전 후보 점수', () => {
    test('경계 ±1 레벨의 실패·미플레이·HARD 아래 클리어만 후보이고, 실패 > 미플레이 > 램프 갱신, 레벨 거리만큼 낮아진다', () => {
        const records = {
            ...levelRecords(table, { '1': 'Hard', '2': 'Hard', '3': 'Hard' }),
            [md5Of(400)]: recordOf('Failed', 90),
            [md5Of(401)]: recordOf('Failed', 60),
            [md5Of(402)]: recordOf('Easy'),
            [md5Of(300)]: recordOf('Normal'),
            [md5Of(500)]: recordOf('Failed', 70),
            [md5Of(403)]: recordOf('Failed'),
            [md5Of(301)]: recordOf('ExHard'),
        }
        const progress = readTableProgress(table, records)
        expect(progress.frontier?.level).toBe('4')
        const ranked = rankChallengeCandidates({
            charts: progress.charts,
            levels: progress.levels,
            frontierIndex: progress.frontierIndex ?? 0,
            limit: 40,
        })
        expect(ranked.slice(0, 4).map((entry) => [entry.chart.md5, entry.kind, entry.score])).toEqual([
            [md5Of(400), 'failed', 3.9],
            [md5Of(403), 'failed', 3.8],
            [md5Of(401), 'failed', 3.6],
            [md5Of(500), 'failed', 2.2],
        ])
        expect(ranked.find((entry) => entry.chart.md5 === md5Of(402))).toMatchObject({ kind: 'lamp_up', score: 1 })
        expect(ranked.find((entry) => entry.chart.md5 === md5Of(300))).toMatchObject({ kind: 'lamp_up', score: -0.5 })
        expect(ranked.some((entry) => entry.chart.md5 === md5Of(301))).toBe(false)
        expect(ranked.some((entry) => entry.chart.level === '2' || entry.chart.level === '6')).toBe(false)
        expect(ranked.filter((entry) => entry.kind === 'unplayed').map((entry) => entry.chart.level)).toEqual(['5', '5', '5'])
    })

    test('limit 으로 자르고 같은 점수는 표 순서로 둔다', () => {
        const progress = readTableProgress(table, {})
        const ranked = rankChallengeCandidates({ charts: progress.charts, levels: progress.levels, frontierIndex: 0, limit: 3 })
        expect(ranked.map((entry) => entry.chart.md5)).toEqual([md5Of(100), md5Of(101), md5Of(102)])
    })

    test('곡별 특징의 상위 태그는 비율 순 3개, 5% 아래와 설명이 약한 태그는 뺀다', () => {
        expect(topTagsOf({ trill: 0.2, jack: 0.3, ln_end: 0.5, with_scratch: 0.4, dense: 0.1, stair: 0.04, chord_2: 0.1 })).toEqual([
            { tag: 'jack', ratio: 0.3 },
            { tag: 'trill', ratio: 0.2 },
            { tag: 'chord_2', ratio: 0.1 },
        ])
        expect(topTagsOf(undefined)).toEqual([])
    })
})

describe('어시스트 등급', () => {
    test('기록의 등급을 차트 진행도와 후보에 싣고 기록이 없으면 0 이다', () => {
        const records = { [md5Of(100)]: { ...recordOf('LightAssistEasy'), assist_grade: 3 as const } }
        const charts = toChartProgress(table, records)
        expect(charts.find((chart) => chart.md5 === md5Of(100))).toMatchObject({ lamp: 'LightAssistEasy', lampId: 3, assistGrade: 3 })
        expect(charts.find((chart) => chart.md5 === md5Of(101))).toMatchObject({ lamp: 'NoPlay', assistGrade: 0 })
        const candidate = toCandidate(charts[0], new Map())
        expect([candidate.lamp, candidate.assist_grade]).toEqual(['LightAssistEasy', 3])
    })

    test('등급이 달라도 램프 id 는 같아 클리어 수와 HARD 이상 수는 그대로다', () => {
        const records = {
            [md5Of(100)]: { ...recordOf('LightAssistEasy'), assist_grade: 4 as const },
            [md5Of(101)]: { ...recordOf('LightAssistEasy'), assist_grade: 0 as const },
        }
        expect(readTableProgress(table, records).levels[0]).toMatchObject({ played: 2, cleared: 2, hard_or_better: 0 })
    })
})

const extrasOf = (patch: Partial<ChartFeatureExtras> & { p90?: number; sustainedS?: number } = {}): ChartFeatureExtras => ({
    nps: { p50_2s: 5, p90_2s: patch.p90 ?? 10, max_2s: 14, max_8s: 12, sustained_s: patch.sustainedS ?? 0 },
    combos: patch.combos ?? {},
    bpmMainBand: patch.bpmMainBand ?? '150-179',
    soflanEvents: patch.soflanEvents ?? 0,
    lengthMs: patch.lengthMs ?? 60_000,
})

describe('약점 키와 곡의 일치 (기획 2026-11-04 2.6 기능 5·6)', () => {
    test('키 종류마다 곡 특징의 몫을 읽고, 곡을 고르지 못하는 키와 잘못된 키는 0 이다', () => {
        const input = {
            tags: { trill: 0.25, chord_2: 0.4 },
            extras: extrasOf({ p90: 12.5, sustainedS: 15, combos: { 'chord_2+trill': 0.12 }, bpmMainBand: '180-209', soflanEvents: 2 }),
            limitNps: 12,
        }
        expect(weaknessShareOf('tag:trill', input)).toBe(0.25)
        expect(weaknessShareOf('solo:trill', input)).toBe(0.25)
        expect(weaknessShareOf('combo:chord_2+trill', input)).toBe(0.12)
        expect(weaknessShareOf('density:limit', input)).toBe(1)
        expect(weaknessShareOf('density:sustained', input)).toBe(0.25)
        expect(weaknessShareOf('bpm:180-209', input)).toBe(1)
        expect(weaknessShareOf('bpm:<120', input)).toBe(0)
        expect(weaknessShareOf('soflan:after', input)).toBe(1)
        for (const key of ['lane:3', 'hand:left', 'timing:trill', 'fatigue', 'bp_context:trill', 'nope', 'tag:nope'])
            expect(weaknessShareOf(key, input)).toBe(0)
    })

    test('밀도 한계 구간은 한계의 0.9~1.15 배이고, 한계가 없거나 새 특징이 없으면 일치·감점이 모두 0 이다', () => {
        const at = (p90: number) => weaknessShareOf('density:limit', { tags: undefined, extras: extrasOf({ p90 }), limitNps: 10 })
        expect([at(8.9), at(9), at(11.5), at(11.6)]).toEqual([0, 1, 1, 0])
        expect(weaknessShareOf('density:limit', { tags: undefined, extras: extrasOf({ p90: 10 }), limitNps: null })).toBe(0)
        const v1 = { tags: { trill: 0.3 }, extras: undefined, limitNps: 10 }
        expect(
            ['combo:chord_2+trill', 'density:limit', 'density:sustained', 'bpm:150-179', 'soflan:after'].map((key) => weaknessShareOf(key, v1)),
        ).toEqual([0, 0, 0, 0, 0])
        expect(weaknessShareOf('tag:trill', v1)).toBe(0.3)
        expect([isDensityOver(extrasOf({ p90: 12.5 }), 10), isDensityOver(extrasOf({ p90: 12.6 }), 10)]).toEqual([false, true])
        expect([isDensityOver(undefined, 10), isDensityOver(extrasOf({ p90: 30 }), null)]).toEqual([false, false])
    })

    test('약점 곡 순위는 몫 0.1 이상만 몫 큰 순, 같으면 표 순서이고 limit 으로 자른다', () => {
        const progress = readTableProgress(table, {})
        const load = {
            shares: new Map([
                [md5Of(100), { trill: 0.2 }],
                [md5Of(101), { trill: 0.4 }],
                [md5Of(102), { trill: 0.2 }],
                [md5Of(103), { trill: 0.05 }],
            ]),
            staleMd5s: new Set<string>(),
        }
        const ranked = rankWeaknessCharts({ pool: progress.charts, key: 'tag:trill', load, limitNps: null, limit: 2 })
        expect(ranked.map((entry) => [entry.chart.md5, entry.share])).toEqual([
            [md5Of(101), 0.4],
            [md5Of(100), 0.2],
        ])
    })

    test('추천 창은 경계 ±1 레벨, 연습 풀은 경계 −2~+1 레벨에서 FULL COMBO 아래만이다', () => {
        const records = { ...levelRecords(table, { '1': 'Hard', '2': 'Hard', '3': 'Hard' }), [md5Of(300)]: recordOf('FullCombo') }
        const progress = readTableProgress(table, records)
        expect(progress.frontier?.level).toBe('4')
        expect([...new Set(challengeWindowOf(progress).map((chart) => chart.level))]).toEqual(['3', '4', '5'])
        const pool = practicePoolOf(progress)
        expect([...new Set(pool.map((chart) => chart.level))]).toEqual(['2', '3', '4', '5'])
        expect(pool.some((chart) => chart.md5 === md5Of(300))).toBe(false)
    })

    test('점수 가산(bonusOf)은 기존 점수에 더해져 순서를 바꾼다', () => {
        const progress = readTableProgress(table, {})
        const ranked = rankChallengeCandidates({
            charts: progress.charts,
            levels: progress.levels,
            frontierIndex: 0,
            limit: 2,
            bonusOf: (chart) => (chart.md5 === md5Of(103) ? 0.5 : 0),
        })
        expect(ranked.map((entry) => [entry.chart.md5, entry.score])).toEqual([
            [md5Of(103), 2.5],
            [md5Of(100), 2],
        ])
    })
})

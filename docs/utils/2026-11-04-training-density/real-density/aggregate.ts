import { readFileSync, writeFileSync } from 'node:fs'

const [dir] = process.argv.slice(2)
if (!dir) throw new Error('usage: aggregate.ts <real-density dir>')

type Rec = any
const recs: Rec[] = readFileSync(`${dir}/charts.ndjson`, 'utf8').trim().split('\n').map((l) => JSON.parse(l))
const meta = JSON.parse(readFileSync(`${dir}/charts.meta.json`, 'utf8'))
const tables: Record<string, Record<string, string>> = JSON.parse(readFileSync(`${dir}/tables.json`, 'utf8'))
const TAGS: string[] = meta.tags

const quantile = (a: readonly number[], p: number) => {
    if (a.length === 0) return NaN
    const s = a.toSorted((x, y) => x - y)
    const pos = (p / 100) * (s.length - 1)
    const lo = Math.floor(pos)
    const hi = Math.ceil(pos)
    return s[lo] + (s[hi] - s[lo]) * (pos - lo)
}
const mean = (a: readonly number[]) => (a.length === 0 ? NaN : a.reduce((s, x) => s + x, 0) / a.length)
const f = (v: number, d = 2) => (Number.isFinite(v) ? v.toFixed(d) : '-')
const pct = (v: number, d = 0) => (Number.isFinite(v) ? `${(v * 100).toFixed(d)}%` : '-')

const levelOf = (rec: Rec, key: string) => tables[key][rec.md5]
const insaneNum = (rec: Rec) => {
    const l = levelOf(rec, 'insane')
    return l === undefined ? undefined : l === '???' ? 99 : Number(l)
}
const inAnyTable = (rec: Rec) => ['insane', 'st', 'sl'].some((k) => tables[k][rec.md5] !== undefined)

type Group = { id: string; label: string; recs: Rec[] }
const range = (key: string, from: number, to: number, id: string, label: string): Group => ({
    id,
    label,
    recs: recs.filter((r) => {
        const l = levelOf(r, key)
        return l !== undefined && Number(l) >= from && Number(l) <= to
    }),
})
const insaneGroups: Group[] = [
    range('insane', 1, 5, 'i1-5', '발광 ★1~5'),
    range('insane', 6, 10, 'i6-10', '발광 ★6~10'),
    range('insane', 11, 15, 'i11-15', '발광 ★11~15'),
    range('insane', 16, 20, 'i16-20', '발광 ★16~20'),
    range('insane', 21, 25, 'i21-25', '발광 ★21~25'),
    { id: 'i???', label: '발광 ★???', recs: recs.filter((r) => levelOf(r, 'insane') === '???') },
]
const stellaGroups: Group[] = [
    range('st', 0, 2, 'st0-2', 'Stella st0~2'),
    range('st', 3, 5, 'st3-5', 'Stella st3~5'),
    range('st', 6, 8, 'st6-8', 'Stella st6~8'),
    range('st', 9, 12, 'st9-12', 'Stella st9~12'),
]
const satGroups: Group[] = [
    range('sl', 0, 2, 'sl0-2', 'Satellite sl0~2'),
    range('sl', 3, 5, 'sl3-5', 'Satellite sl3~5'),
    range('sl', 6, 8, 'sl6-8', 'Satellite sl6~8'),
    range('sl', 9, 12, 'sl9-12', 'Satellite sl9~12'),
]
const outRecs = recs.filter((r) => !inAnyTable(r))
const outGroup: Group = { id: 'out', label: '표 밖(3개 표 어디에도 없음)', recs: outRecs }
const p90Bins: [number, number][] = [
    [0, 6],
    [6, 9],
    [9, 12],
    [12, 15],
    [15, 18],
    [18, 22],
    [22, 999],
]
const p90Groups: Group[] = p90Bins.map(([a, b]) => ({
    id: `p90_${a}-${b}`,
    label: b === 999 ? `nps2 p90 ${a}+` : `nps2 p90 ${a}~${b}`,
    recs: recs.filter((r) => r.nps2.p90 >= a && r.nps2.p90 < b),
}))
const allTableRecs = recs.filter(inAnyTable)
const tableP90Groups: Group[] = p90Bins.map(([a, b]) => ({
    id: `tp90_${a}-${b}`,
    label: b === 999 ? `표 곡 nps2 p90 ${a}+` : `표 곡 nps2 p90 ${a}~${b}`,
    recs: allTableRecs.filter((r) => r.nps2.p90 >= a && r.nps2.p90 < b),
}))

const stAll: Group = { id: 'stAll', label: 'Stella 전체(st0~12)', recs: recs.filter((r) => levelOf(r, 'st') !== undefined) }
const slAll: Group = { id: 'slAll', label: 'Satellite 전체(sl0~12)', recs: recs.filter((r) => levelOf(r, 'sl') !== undefined) }
const tableAll: Group = { id: 'tableAll', label: '표 3종 합침', recs: allTableRecs }
const CONDENSED: Group[] = [...insaneGroups.slice(0, 5), stAll, slAll, outGroup]
const MAIN_GROUPS = [...insaneGroups, ...stellaGroups, ...satGroups, outGroup]

type Metric = { name: string; get: (r: Rec) => number | null | undefined; d?: number }
const SCALARS: Metric[] = [
    { name: '평균 NPS(avg, 판정 노트/첫~끝 노트)', get: (r) => r.avgNps },
    { name: '선곡 표시 avg(1/4 평균 이상 초)', get: (r) => r.selectAvg },
    { name: '선곡 표시 peak(최대 1초 칸)', get: (r) => r.selectPeak },
    { name: 'nps2 p50(노트 기준)', get: (r) => r.nps2.p50 },
    { name: 'nps2 p75', get: (r) => r.nps2.p75 },
    { name: 'nps2 p90', get: (r) => r.nps2.p90 },
    { name: 'nps2 max', get: (r) => r.nps2.max },
    { name: 'nps8 max', get: (r) => r.nps8Max },
    { name: '최대/평균 (nps2 max / avg)', get: (r) => r.nps2.max / r.avgNps },
    { name: 'p90/평균 (nps2 p90 / avg)', get: (r) => r.nps2.p90 / r.avgNps },
    { name: 'p50/평균', get: (r) => r.nps2.p50 / r.avgNps },
    { name: 'peak(1초)/평균', get: (r) => r.selectPeak / r.avgNps },
    { name: '곡 길이(마지막 노트까지, 초)', get: (r) => r.lengthS, d: 0 },
    { name: '활동 구간(첫~끝 판정 노트, 초)', get: (r) => r.spanS, d: 0 },
    { name: '첫 노트 시각(초)', get: (r) => r.firstS, d: 1 },
    { name: '판정 노트 수', get: (r) => r.n, d: 0 },
    { name: '노트/행(동시치기 평균)', get: (r) => r.notesPerRow },
    { name: '평균 행/초(평균 NPS / 노트/행)', get: (r) => r.avgNps / r.notesPerRow },
    { name: '16분 연타 대비 평균 행 밀도(행/초 / (노트 BPM 중앙값/15))', get: (r) => r.avgNps / r.notesPerRow / (r.bpm.p50 / 15), d: 3 },
    { name: 'nps2 p50 / p90', get: (r) => r.nps2.p50 / r.nps2.p90 },
    { name: 'nps2 p75 / p90', get: (r) => r.nps2.p75 / r.nps2.p90 },
    { name: 'nps2 max / p90', get: (r) => r.nps2.max / r.nps2.p90 },
    { name: 'nps2 p90 / p50', get: (r) => r.nps2.p90 / r.nps2.p50 },
    { name: 'nps2 max / p50', get: (r) => r.nps2.max / r.nps2.p50 },
    { name: '2초 최대 행/초', get: (r) => r.peakRows2s, d: 1 },
    { name: '스크래치 비율', get: (r) => r.scratchShare, d: 3 },
    { name: 'LN 비율', get: (r) => r.lnRatio, d: 3 },
    { name: '노트 BPM 중앙값', get: (r) => r.bpm.p50, d: 0 },
    { name: '고점 근처 시간 비율(r>=0.8 max)', get: (r) => r.shareTimeAtLeast[2], d: 3 },
    { name: '평균 125% 이상 시간 비율', get: (r) => r.shareTimeAboveMean125, d: 3 },
    { name: '평균 75% 미만 시간 비율', get: (r) => r.shareTimeBelowMean75, d: 3 },
    { name: '고점 노트(nps2>=0.9 max) 비율', get: (r) => r.shareNotesAtLeast[3], d: 3 },
    { name: '가장 어려운 1분 NPS', get: (r) => r.minute?.hardNps },
    { name: '가장 어려운 1분 / 평균', get: (r) => (r.minute ? r.minute.hardNps / r.avgNps : null) },
    { name: '가장 어려운 1분 / nps2 max', get: (r) => (r.minute ? r.minute.hardNps / r.nps2.max : null) },
    { name: '가장 어려운 1분 / nps2 p90', get: (r) => (r.minute ? r.minute.hardNps / r.nps2.p90 : null) },
    { name: '1분 내 5초 구간 변동계수', get: (r) => r.minute?.hardCv12, d: 3 },
    { name: '1분 내 5초 구간 (최대-최소)/평균', get: (r) => r.minute?.hardRange12 },
    { name: '1분 내 5초 구간 최솟값/평균', get: (r) => r.minute?.hardMin12 },
    { name: '1분 내 5초 구간 최댓값/평균', get: (r) => r.minute?.hardMax12 },
    { name: '1분 내 평균 70% 미만 5초 구간 수(12 중)', get: (r) => r.minute?.hardDips, d: 1 },
    { name: '1분 내 nps2(0.5초 격자) p90/p10', get: (r) => (r.minute ? r.minute.hardNps2.p90 / Math.max(r.minute.hardNps2.p10, 0.5) : null) },
    { name: '1분 내 nps2 max / 1분 평균', get: (r) => (r.minute ? r.minute.hardNps2.max / r.minute.hardNps : null) },
    { name: '1분 내 노트 nps2 p50 / 1분 평균', get: (r) => (r.minute ? r.minute.hardNoteNps2.p50 / r.minute.hardNps : null) },
    { name: '1분 내 노트 nps2 p90 / 1분 평균', get: (r) => (r.minute ? r.minute.hardNoteNps2.p90 / r.minute.hardNps : null) },
    { name: '1분 내 노트 nps2 max / 1분 평균', get: (r) => (r.minute ? r.minute.hardNoteNps2.max / r.minute.hardNps : null) },
    { name: '대표(중앙) 1분 NPS / 평균', get: (r) => (r.minute ? r.minute.medNps / r.avgNps : null) },
    { name: '대표(중앙) 1분 5초 변동계수', get: (r) => r.minute?.medCv12, d: 3 },
    ...[2, 4, 8, 16, 30, 60].map((w): Metric => ({ name: `최대 ${w}초 평균 NPS`, get: (r) => r.peakAvg[String(w)] })),
    ...[4, 8, 16, 30, 60].map((w): Metric => ({ name: `최대 ${w}초 평균 / 최대 2초`, get: (r) => (r.peakAvg[String(w)] === null ? null : r.peakAvg[String(w)] / r.peakAvg['2']), d: 3 })),
    ...[4, 8, 16, 30, 60].map((w): Metric => ({ name: `최대 ${w}초 평균 / 곡 평균`, get: (r) => (r.peakAvg[String(w)] === null ? null : r.peakAvg[String(w)] / r.avgNps), d: 3 })),
    { name: '고점 구간 수 / 분', get: (r) => (r.hiZonesS.length / r.spanS) * 60, d: 1 },
    { name: '고점 구간 중 가장 긴 길이(초)', get: (r) => Math.max(0, ...r.hiZonesS), d: 1 },
    { name: '휴식(<0.5 평균, >=2초) 횟수 / 분', get: (r) => (r.restsS[1].length / r.spanS) * 60, d: 2 },
    { name: '휴식(<0.5 평균) 시간 비율', get: (r) => r.restShare[1], d: 3 },
    { name: '깊은 휴식(<0.25 평균, >=2초) 횟수 / 분', get: (r) => (r.restsS[0].length / r.spanS) * 60, d: 2 },
    { name: '깊은 휴식 시간 비율', get: (r) => r.restShare[0], d: 3 },
    { name: '밀도 파동 수 / 분 (돌출 >=0.25 평균)', get: (r) => (r.waves[0].count / r.spanS) * 60, d: 1 },
    { name: '밀도 파동 수 / 분 (돌출 >=0.5 평균)', get: (r) => (r.waves[1].count / r.spanS) * 60, d: 1 },
    { name: '가장 높은 격자 시점 위치(0~1)', get: (r) => r.rMaxPos, d: 2 },
    { name: '첫 5%(도입) 구간 / 평균', get: (r) => (r.seg20[0] * SEG / r.n) , d: 2 },
    { name: '끝 5% 구간 / 평균', get: (r) => (r.seg20[SEG - 1] * SEG) / r.n, d: 2 },
]
const SEG = 20

const statRow = (m: Metric, rs: Rec[]) => {
    const vals = rs.map(m.get).filter((v): v is number => v !== null && v !== undefined && Number.isFinite(v))
    return { n: vals.length, p10: quantile(vals, 10), p25: quantile(vals, 25), p50: quantile(vals, 50), p75: quantile(vals, 75), p90: quantile(vals, 90), mean: mean(vals) }
}

const out: Record<string, any> = { meta: { measured: recs.length, skipped: meta.skipped.length, crossCheckMaxDiff: meta.crossCheckMaxDiff } }
const md: string[] = []

const countsTable = (groups: Group[]) => {
    md.push('| 대역 | 곡 수 | 1분 이상(가장 어려운 1분 통계 가능) |')
    md.push('| --- | ---: | ---: |')
    for (const g of groups) md.push(`| ${g.label} | ${g.recs.length} | ${g.recs.filter((r) => r.minute).length} |`)
}

const scalarTable = (title: string, groups: Group[], names: string[], useMedianIqr = true) => {
    md.push(`\n#### ${title}\n`)
    md.push(`| 지표 (중앙값 [p25~p75]) | ${groups.map((g) => g.label).join(' | ')} |`)
    md.push(`| --- | ${groups.map(() => '---:').join(' | ')} |`)
    for (const name of names) {
        const m = SCALARS.find((s) => s.name === name)
        if (!m) throw new Error(`no metric ${name}`)
        const d = m.d ?? 2
        const cells = groups.map((g) => {
            const s = statRow(m, g.recs)
            return useMedianIqr ? `${f(s.p50, d)} [${f(s.p25, d)}~${f(s.p75, d)}]` : f(s.mean, d)
        })
        md.push(`| ${name} | ${cells.join(' | ')} |`)
    }
}

const scalarsJson: Record<string, Record<string, ReturnType<typeof statRow>>> = {}
for (const g of [...MAIN_GROUPS, ...p90Groups, ...tableP90Groups]) {
    scalarsJson[g.id] = {}
    for (const m of SCALARS) scalarsJson[g.id][m.name] = statRow(m, g.recs)
}
out.scalars = scalarsJson

md.push('### 표본 수')
md.push('\n발광 표')
countsTable(insaneGroups)
md.push('\nStella / Satellite')
countsTable([...stellaGroups, ...satGroups])
md.push('\n표 밖')
countsTable([outGroup])

const NAMES_BASIC = [
    '판정 노트 수',
    '곡 길이(마지막 노트까지, 초)',
    '활동 구간(첫~끝 판정 노트, 초)',
    '첫 노트 시각(초)',
    '평균 NPS(avg, 판정 노트/첫~끝 노트)',
    '선곡 표시 avg(1/4 평균 이상 초)',
    '선곡 표시 peak(최대 1초 칸)',
    'nps2 p50(노트 기준)',
    'nps2 p75',
    'nps2 p90',
    'nps2 max',
    'nps8 max',
    '최대/평균 (nps2 max / avg)',
    'p90/평균 (nps2 p90 / avg)',
    'p50/평균',
    'peak(1초)/평균',
]
md.push('\n### 3.1 곡 단위 밀도 요약')
scalarTable('발광 표 대역', insaneGroups, NAMES_BASIC)
scalarTable('Stella·Satellite 대역', [...stellaGroups, ...satGroups], NAMES_BASIC)
scalarTable('전체 7,848곡을 nps2 p90 구간으로 나눈 것(표 안팎 합침)', p90Groups, NAMES_BASIC)

const NAMES_MINUTE = [
    '가장 어려운 1분 NPS',
    '가장 어려운 1분 / 평균',
    '가장 어려운 1분 / nps2 max',
    '가장 어려운 1분 / nps2 p90',
    '1분 내 5초 구간 변동계수',
    '1분 내 5초 구간 (최대-최소)/평균',
    '1분 내 5초 구간 최솟값/평균',
    '1분 내 5초 구간 최댓값/평균',
    '1분 내 평균 70% 미만 5초 구간 수(12 중)',
    '1분 내 nps2(0.5초 격자) p90/p10',
    '1분 내 nps2 max / 1분 평균',
    '1분 내 노트 nps2 p50 / 1분 평균',
    '1분 내 노트 nps2 p90 / 1분 평균',
    '1분 내 노트 nps2 max / 1분 평균',
    '대표(중앙) 1분 NPS / 평균',
    '대표(중앙) 1분 5초 변동계수',
]
md.push('\n### 3.4 가장 어려운 1분')
scalarTable('발광 표 대역', insaneGroups, NAMES_MINUTE)
scalarTable('Stella·Satellite 대역', [...stellaGroups, ...satGroups], NAMES_MINUTE)
scalarTable('전체 곡을 nps2 p90 구간으로 나눈 것', p90Groups, NAMES_MINUTE)

const NAMES_PEAKDECAY = [
    '최대 2초 평균 NPS',
    '최대 4초 평균 / 최대 2초',
    '최대 8초 평균 / 최대 2초',
    '최대 16초 평균 / 최대 2초',
    '최대 30초 평균 / 최대 2초',
    '최대 60초 평균 / 최대 2초',
    '최대 8초 평균 / 곡 평균',
    '최대 16초 평균 / 곡 평균',
    '최대 30초 평균 / 곡 평균',
    '최대 60초 평균 / 곡 평균',
]
md.push('\n### 3.3 고점 지속')
scalarTable('최대 w초 평균의 감쇠', [...CONDENSED, tableAll], NAMES_PEAKDECAY)

const NAMES_FLOW = [
    '고점 근처 시간 비율(r>=0.8 max)',
    '평균 125% 이상 시간 비율',
    '평균 75% 미만 시간 비율',
    '고점 노트(nps2>=0.9 max) 비율',
    '고점 구간 수 / 분',
    '고점 구간 중 가장 긴 길이(초)',
    '휴식(<0.5 평균) 시간 비율',
    '휴식(<0.5 평균, >=2초) 횟수 / 분',
    '깊은 휴식 시간 비율',
    '깊은 휴식(<0.25 평균, >=2초) 횟수 / 분',
    '밀도 파동 수 / 분 (돌출 >=0.25 평균)',
    '밀도 파동 수 / 분 (돌출 >=0.5 평균)',
    '가장 높은 격자 시점 위치(0~1)',
    '첫 5%(도입) 구간 / 평균',
    '끝 5% 구간 / 평균',
]
md.push('\n### 3.2 흐름 모양 스칼라')
scalarTable('흐름 지표', [...CONDENSED, tableAll], NAMES_FLOW)

const pooled = (groupRecs: Rec[], pick: (r: Rec) => number[]) => groupRecs.flatMap(pick)
const poolRow = (label: string, vals: number[], d = 1) =>
    `| ${label} | ${vals.length} | ${f(quantile(vals, 10), d)} | ${f(quantile(vals, 25), d)} | ${f(quantile(vals, 50), d)} | ${f(quantile(vals, 75), d)} | ${f(quantile(vals, 90), d)} | ${f(Math.max(...vals), d)} |`
const POOL_HEAD = ['| 대역 | 개수 | p10 | p25 | p50 | p75 | p90 | max |', '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
const poolGroups = [...insaneGroups, ...stellaGroups, ...satGroups, outGroup]
md.push('\n#### 고점 구간(r>=0.8 max, 간격 1초 이하 병합) 길이 분포(초, 모든 곡의 구간을 합쳐서)\n', ...POOL_HEAD)
for (const g of poolGroups) md.push(poolRow(g.label, pooled(g.recs, (r) => r.hiZonesS)))
md.push('\n#### 휴식(r<0.5 평균, 2초 이상) 길이 분포(초)\n', ...POOL_HEAD)
for (const g of poolGroups) md.push(poolRow(g.label, pooled(g.recs, (r) => r.restsS[1])))
md.push('\n#### 깊은 휴식(r<0.25 평균, 2초 이상) 길이 분포(초)\n', ...POOL_HEAD)
for (const g of poolGroups) md.push(poolRow(g.label, pooled(g.recs, (r) => r.restsS[0])))
md.push('\n#### 밀도 파동 간격(돌출 >=0.25 평균인 인접 두 고점 사이, 초)\n', ...POOL_HEAD)
for (const g of poolGroups) md.push(poolRow(g.label, pooled(g.recs, (r) => r.waves[0].intervalsS)))
md.push('\n#### 밀도 파동 간격(돌출 >=0.5 평균, 초)\n', ...POOL_HEAD)
for (const g of poolGroups) md.push(poolRow(g.label, pooled(g.recs, (r) => r.waves[1].intervalsS)))
md.push('\n#### 밀도 파동 돌출 크기(고점-양쪽 골 중 높은 쪽, 곡 평균 NPS 대비 배수, 돌출 >=0.25 평균인 것)\n', ...POOL_HEAD.map((l) => l))
for (const g of poolGroups) md.push(poolRow(g.label, pooled(g.recs, (r) => r.waves[0].promRel), 2))

const curveStats = (rs: Rec[], k: number, mode: 'mean' | 'peak') => {
    const per = k === SEG ? 1 : SEG / k
    const curves = rs.map((r) => {
        const pooledSeg = Array.from({ length: k }, (_, j) => r.seg20.slice(j * per, (j + 1) * per).reduce((s: number, x: number) => s + x, 0))
        const base = mode === 'mean' ? r.n / k : Math.max(...pooledSeg)
        return pooledSeg.map((c) => c / base)
    })
    return Array.from({ length: k }, (_, j) => {
        const col = curves.map((c) => c[j])
        return { mean: mean(col), p25: quantile(col, 25), p50: quantile(col, 50), p75: quantile(col, 75) }
    })
}
const curveGroups = [...insaneGroups, ...stellaGroups, ...satGroups, outGroup]
out.curves = {}
md.push('\n### 3.2 곡 전체 정규화 곡선 (활동 구간을 K 등분, 구간 NPS / 곡 평균 NPS, 곡별 값의 중앙값)\n')
for (const k of [10, 20]) {
    md.push(`\n#### K=${k} 중앙값 곡선(곡 평균=1.00)\n`)
    md.push(`| 대역 | ${Array.from({ length: k }, (_, j) => `${j + 1}`).join(' | ')} |`)
    md.push(`| --- | ${Array.from({ length: k }, () => '---:').join(' | ')} |`)
    for (const g of curveGroups) {
        const cs = curveStats(g.recs, k, 'mean')
        out.curves[`${g.id}_K${k}_mean`] = cs
        md.push(`| ${g.label} | ${cs.map((c) => f(c.p50)).join(' | ')} |`)
    }
}
md.push('\n#### K=10 평균 곡선(곡 평균=1.00, 평균값)\n')
md.push(`| 대역 | ${Array.from({ length: 10 }, (_, j) => `${j + 1}`).join(' | ')} |`)
md.push(`| --- | ${Array.from({ length: 10 }, () => '---:').join(' | ')} |`)
for (const g of curveGroups) {
    const cs = curveStats(g.recs, 10, 'mean')
    md.push(`| ${g.label} | ${cs.map((c) => f(c.mean)).join(' | ')} |`)
}
md.push('\n#### K=10 최고 구간 대비 곡선(곡 내 가장 센 구간=1.00, 중앙값)\n')
md.push(`| 대역 | ${Array.from({ length: 10 }, (_, j) => `${j + 1}`).join(' | ')} |`)
md.push(`| --- | ${Array.from({ length: 10 }, () => '---:').join(' | ')} |`)
for (const g of curveGroups) {
    const cs = curveStats(g.recs, 10, 'peak')
    out.curves[`${g.id}_K10_peak`] = cs
    md.push(`| ${g.label} | ${cs.map((c) => f(c.p50)).join(' | ')} |`)
}
md.push('\n#### K=10 p25~p75 (곡 평균=1.00) -- 곡마다 편차\n')
md.push(`| 대역 | ${Array.from({ length: 10 }, (_, j) => `${j + 1}`).join(' | ')} |`)
md.push(`| --- | ${Array.from({ length: 10 }, () => '---:').join(' | ')} |`)
for (const g of curveGroups) {
    const cs = curveStats(g.recs, 10, 'mean')
    md.push(`| ${g.label} | ${cs.map((c) => `${f(c.p25)}~${f(c.p75)}`).join(' | ')} |`)
}

md.push('\n#### 최고 구간(K=10 중 가장 센 구간) 위치 분포(곡 비율)\n')
md.push(`| 대역 | ${Array.from({ length: 10 }, (_, j) => `${j + 1}`).join(' | ')} |`)
md.push(`| --- | ${Array.from({ length: 10 }, () => '---:').join(' | ')} |`)
for (const g of curveGroups) {
    const hist = new Array(10).fill(0)
    for (const r of g.recs) {
        const pooledSeg = Array.from({ length: 10 }, (_, j) => r.seg20[2 * j] + r.seg20[2 * j + 1])
        hist[pooledSeg.indexOf(Math.max(...pooledSeg))] += 1
    }
    md.push(`| ${g.label} | ${hist.map((h) => pct(h / g.recs.length)).join(' | ')} |`)
}

const levelRows = (title: string, key: string, levels: string[], label: (l: string) => string) => {
    md.push(`\n#### ${title}\n`)
    const names = ['평균 NPS(avg, 판정 노트/첫~끝 노트)', 'nps2 p50(노트 기준)', 'nps2 p90', 'nps2 max', '최대/평균 (nps2 max / avg)', '가장 어려운 1분 NPS', '가장 어려운 1분 / 평균', '1분 내 5초 구간 변동계수', '활동 구간(첫~끝 판정 노트, 초)']
    md.push(`| 레벨 | 곡 수 | ${names.join(' | ')} |`)
    md.push(`| --- | ---: | ${names.map(() => '---:').join(' | ')} |`)
    for (const l of levels) {
        const rs = recs.filter((r) => levelOf(r, key) === l)
        if (rs.length === 0) continue
        const cells = names.map((nm) => {
            const m = SCALARS.find((s) => s.name === nm)!
            return f(statRow(m, rs).p50, m.d ?? 2)
        })
        md.push(`| ${label(l)} | ${rs.length} | ${cells.join(' | ')} |`)
    }
}
levelRows('발광 표 레벨별 중앙값', 'insane', [...Array.from({ length: 25 }, (_, i) => String(i + 1)), '???'], (l) => `★${l}`)
levelRows('Stella 레벨별 중앙값', 'st', Array.from({ length: 13 }, (_, i) => String(i)), (l) => `st${l}`)
levelRows('Satellite 레벨별 중앙값', 'sl', Array.from({ length: 13 }, (_, i) => String(i)), (l) => `sl${l}`)

md.push('\n### 3.5 BPM 분포 (노트 기준 BPM 중앙값, 곡 비율)\n')
const BPM_EDGES = [0, 100, 120, 140, 160, 180, 200, 220, 250, 300, 99999]
md.push(`| 대역 | ${BPM_EDGES.slice(0, -1).map((e, i) => (i === BPM_EDGES.length - 2 ? `${e}+` : `${e}~${BPM_EDGES[i + 1]}`)).join(' | ')} | p10 | p50 | p90 | 소프랑(BPM 점프>=1) 곡 비율 |`)
md.push(`| --- | ${BPM_EDGES.slice(0, -1).map(() => '---:').join(' | ')} | ---: | ---: | ---: | ---: |`)
for (const g of curveGroups) {
    const bpms = g.recs.map((r) => r.bpm.p50)
    const hist = BPM_EDGES.slice(0, -1).map((e, i) => bpms.filter((b) => b >= e && b < BPM_EDGES[i + 1]).length / bpms.length)
    const soflan = g.recs.filter((r) => r.soflanJumps + r.longStops > 0).length / g.recs.length
    md.push(`| ${g.label} | ${hist.map((h) => pct(h)).join(' | ')} | ${f(quantile(bpms, 10), 0)} | ${f(quantile(bpms, 50), 0)} | ${f(quantile(bpms, 90), 0)} | ${pct(soflan)} |`)
}

const TAG_SHOW = TAGS
md.push('\n### 3.6 패턴 태그 비율(곡마다 판정 노트 대비 비율의 평균)\n')
md.push(`| 태그 | ${curveGroups.map((g) => g.label).join(' | ')} |`)
md.push(`| --- | ${curveGroups.map(() => '---:').join(' | ')} |`)
for (let t = 0; t < TAG_SHOW.length; t += 1) {
    const cells = curveGroups.map((g) => pct(mean(g.recs.map((r) => r.tagsAll[t] / r.n)), 1))
    md.push(`| ${TAG_SHOW[t]} | ${cells.join(' | ')} |`)
}
md.push('\n#### 고점 노트(곡 안 nps2 상위 10%)의 태그 비율 / 곡 전체 비율 (배율 lift, 곡 평균끼리)\n')
const liftGroups = [insaneGroups[0], insaneGroups[1], insaneGroups[2], insaneGroups[3], insaneGroups[4], stellaGroups[3], satGroups[3], outGroup]
md.push(`| 태그 | ${liftGroups.map((g) => `${g.label} 전체 → 고점 (배율)`).join(' | ')} |`)
md.push(`| --- | ${liftGroups.map(() => '---:').join(' | ')} |`)
const liftJson: Record<string, Record<string, { all: number; top: number; minute: number; lift: number }>> = {}
for (let t = 0; t < TAG_SHOW.length; t += 1) {
    const cells = liftGroups.map((g) => {
        const all = mean(g.recs.map((r) => r.tagsAll[t] / r.n))
        const top = mean(g.recs.map((r) => r.tagTop[t] / r.topSize))
        const minute = mean(g.recs.filter((r) => r.minute).map((r) => r.minute.hardTagCounts[t] / r.minute.hardNotes))
        liftJson[g.id] = liftJson[g.id] ?? {}
        liftJson[g.id][TAG_SHOW[t]] = { all, top, minute, lift: all > 0 ? top / all : NaN }
        return `${pct(all, 1)} → ${pct(top, 1)} (${all > 0.001 ? f(top / all, 2) : '-'})`
    })
    md.push(`| ${TAG_SHOW[t]} | ${cells.join(' | ')} |`)
}
out.tagLift = liftJson
md.push('\n#### 가장 어려운 1분 안의 태그 비율(곡 평균)\n')
md.push(`| 태그 | ${liftGroups.map((g) => g.label).join(' | ')} |`)
md.push(`| --- | ${liftGroups.map(() => '---:').join(' | ')} |`)
for (let t = 0; t < TAG_SHOW.length; t += 1) {
    md.push(`| ${TAG_SHOW[t]} | ${liftGroups.map((g) => pct(liftJson[g.id][TAG_SHOW[t]].minute, 1)).join(' | ')} |`)
}

// mean nps2 quantile function by group
md.push('\n### 3.1b 노트 기준 nps2 분포의 곡 평균(분위수: p5 p10 p25 p50 p75 p90 p95 p99, 곡별 값의 중앙값)\n')
md.push(`| 대역 | p5 | p10 | p25 | p50 | p75 | p90 | p95 | p99 |`)
md.push(`| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |`)
for (const g of curveGroups) {
    md.push(`| ${g.label} | ${[0, 1, 2, 3, 4, 5, 6, 7].map((i) => f(quantile(g.recs.map((r) => r.nps2Q[i]), 50), 1)).join(' | ')} |`)
}

// ---------- k-means of hardest minute shapes ----------
const mulberry32 = (seed: number) => () => {
    seed |= 0
    seed = (seed + 0x6d2b79f5) | 0
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
}
const dist2 = (a: number[], b: number[]) => a.reduce((s, x, i) => s + (x - b[i]) ** 2, 0)
const kmeans = (points: number[][], k: number, seed = 1, restarts = 20) => {
    let best: { centers: number[][]; assign: number[]; inertia: number } | null = null
    for (let rs = 0; rs < restarts; rs += 1) {
        const rnd = mulberry32(seed * 1000 + rs)
        const centers: number[][] = [points[Math.floor(rnd() * points.length)].slice()]
        while (centers.length < k) {
            const d = points.map((p) => Math.min(...centers.map((c) => dist2(p, c))))
            const total = d.reduce((s, x) => s + x, 0)
            let target = rnd() * total
            let idx = 0
            for (; idx < d.length - 1; idx += 1) {
                target -= d[idx]
                if (target <= 0) break
            }
            centers.push(points[idx].slice())
        }
        let assign = new Array(points.length).fill(0)
        for (let iter = 0; iter < 100; iter += 1) {
            const next = points.map((p) => {
                let bi = 0
                let bd = Infinity
                centers.forEach((c, i) => {
                    const d = dist2(p, c)
                    if (d < bd) {
                        bd = d
                        bi = i
                    }
                })
                return bi
            })
            const changed = next.some((v, i) => v !== assign[i])
            assign = next
            for (let c = 0; c < k; c += 1) {
                const members = points.filter((_, i) => assign[i] === c)
                if (members.length > 0) centers[c] = centers[c].map((_, d) => mean(members.map((m) => m[d])))
            }
            if (!changed) break
        }
        const inertia = points.reduce((s, p, i) => s + dist2(p, centers[assign[i]]), 0)
        if (!best || inertia < best.inertia) best = { centers: centers.map((c) => c.slice()), assign: assign.slice(), inertia }
    }
    return best!
}

const shapeOf = (c: number[]) => {
    const first = mean(c.slice(0, 4))
    const last = mean(c.slice(8))
    const mid = mean(c.slice(4, 8))
    return { first, mid, last }
}

const kmeansBlock = (title: string, rs: Rec[], k: number, groupsForMembership: Group[], key: 'hardRel12' | 'medRel12') => {
    const sel = rs.filter((r) => r.minute)
    const pts = sel.map((r) => r.minute[key] as number[])
    const res = kmeans(pts, k)
    const order = res.centers.map((_, i) => i).toSorted((a, b) => res.assign.filter((x) => x === b).length - res.assign.filter((x) => x === a).length)
    md.push(`\n#### ${title} (곡 ${sel.length}개, k=${k}, 5초 구간 12개, 1분 평균=1.00)\n`)
    md.push(`| 번호 | 비율 | ${Array.from({ length: 12 }, (_, j) => `${j * 5}~${j * 5 + 5}s`).join(' | ')} | 최솟값 | 최댓값 | 변동계수 |`)
    md.push(`| --- | ---: | ${Array.from({ length: 12 }, () => '---:').join(' | ')} | ---: | ---: | ---: |`)
    const extraRows: string[] = []
    const clusters = order.map((ci, rank) => {
        const members = sel.filter((_, i) => res.assign[i] === ci)
        const c = res.centers[ci]
        const sd = Math.sqrt(mean(c.map((x) => (x - mean(c)) ** 2)))
        md.push(`| ${String.fromCharCode(65 + rank)} | ${pct(members.length / sel.length)} | ${c.map((x) => f(x)).join(' | ')} | ${f(Math.min(...c))} | ${f(Math.max(...c))} | ${f(sd / mean(c))} |`)
        const residual = Math.sqrt(mean(members.flatMap((m) => (m.minute[key] as number[]).map((x, j) => (x - c[j]) ** 2))))
        const memberCv = quantile(members.map((m) => m.minute.hardCv12 as number), 50)
        const memberStep = quantile(members.flatMap((m) => m.minute.inMin.stepAbs12 as number[]), 50)
        extraRows.push(`| ${String.fromCharCode(65 + rank)} | ${f(residual)} | ${f(memberCv, 3)} | ${f(memberStep, 3)} | ${f(quantile(members.map((m) => m.avgNps), 50), 1)} | ${f(quantile(members.map((m) => m.minute.hardNps), 50), 1)} |`)
        return { label: String.fromCharCode(65 + rank), share: members.length / sel.length, centroid: c, residual, memberCv, memberStep }
    })
    md.push('\n| 원형 | 개별 곡이 중심에서 벗어나는 정도(잔차 표준편차) | 개별 곡의 5초 변동계수 중앙값 | 개별 곡의 인접 5초 변화 중앙값 | 평균 NPS 중앙값 | 가장 어려운 1분 NPS 중앙값 |', '| --- | --- | --- | --- | --- | --- |')
    md.push(...extraRows)
    md.push(`\n| 대역 | ${clusters.map((c) => `${c.label} 비율`).join(' | ')} |`)
    md.push(`| --- | ${clusters.map(() => '---:').join(' | ')} |`)
    for (const g of groupsForMembership) {
        const gs = g.recs.filter((r) => r.minute)
        const nearest = (r: Rec) => {
            let bi = 0
            let bd = Infinity
            clusters.forEach((c, i) => {
                const d = dist2(r.minute[key] as number[], c.centroid)
                if (d < bd) {
                    bd = d
                    bi = i
                }
            })
            return bi
        }
        const counts = new Array(clusters.length).fill(0)
        for (const r of gs) counts[nearest(r)] += 1
        const cells = counts.map((c) => pct(c / Math.max(1, gs.length)))
        md.push(`| ${g.label} (${gs.length}) | ${cells.join(' | ')} |`)
    }
    return clusters.map((c) => ({ label: c.label, share: c.share, centroid: c.centroid }))
}

out.kmeans = {}
const membershipGroups = [...insaneGroups.slice(0, 5), ...stellaGroups, ...satGroups, outGroup]
md.push('\n### 4.1 가장 어려운 1분의 밀도 곡선 원형')
for (const k of [4, 6]) {
    out.kmeans[`pooledHard_k${k}`] = kmeansBlock(`표 3종 곡 전체(발광+Stella+Satellite) 가장 어려운 1분 원형`, allTableRecs, k, membershipGroups, 'hardRel12')
}
out.kmeans.pooledHard_out_k4 = kmeansBlock('표 밖 곡 가장 어려운 1분 원형', outRecs, 4, membershipGroups, 'hardRel12')
md.push('\n### 4.2 대표(중앙) 1분의 밀도 곡선 원형')
out.kmeans.pooledMed_k4 = kmeansBlock('표 3종 곡 전체 대표 1분 원형', allTableRecs, 4, membershipGroups, 'medRel12')

// band mean of 12-point hardest/median minute
md.push('\n#### 가장 어려운 1분 12구간 중앙값 곡선 (대역별, 1분 평균=1.00)\n')
md.push(`| 대역 | ${Array.from({ length: 12 }, (_, j) => `${j * 5}~${j * 5 + 5}s`).join(' | ')} |`)
md.push(`| --- | ${Array.from({ length: 12 }, () => '---:').join(' | ')} |`)
const minuteCurves: Record<string, number[]> = {}
for (const g of curveGroups) {
    const gs = g.recs.filter((r) => r.minute)
    const med = Array.from({ length: 12 }, (_, j) => quantile(gs.map((r) => r.minute.hardRel12[j]), 50))
    minuteCurves[g.id] = med
    md.push(`| ${g.label} | ${med.map((x) => f(x)).join(' | ')} |`)
}
out.minuteMedianCurves = minuteCurves
md.push('\n#### 가장 어려운 1분이 곡의 어디에서 시작하는가(곡 길이 대비 시작 위치, 곡 분포)\n')
md.push(`| 대역 | p10 | p25 | p50 | p75 | p90 |`)
md.push(`| --- | ---: | ---: | ---: | ---: | ---: |`)
for (const g of curveGroups) {
    const v = g.recs.filter((r) => r.minute).map((r) => r.minute.hardStartFrac)
    md.push(`| ${g.label} | ${[10, 25, 50, 75, 90].map((p) => f(quantile(v, p))).join(' | ')} |`)
}

// ---------- additions: condensed groups, in-minute stats, composition, regression ----------
const hardRecs = (g: Group) => g.recs.filter((r) => r.minute)

md.push('\n### 3.7 가장 어려운 1분 안쪽 (1분 평균 = 1.00 으로 정규화)\n')
md.push('\n#### 0.5초 격자 nps2 / 1분 평균 의 분위수 (곡별 값의 중앙값)\n')
md.push(`| 대역 | p5 | p10 | p25 | p50 | p75 | p90 | p95 | max |`)
md.push(`| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |`)
for (const g of [...CONDENSED, tableAll]) md.push(`| ${g.label} | ${[0, 1, 2, 3, 4, 5, 6, 7].map((i) => f(quantile(hardRecs(g).map((r) => r.minute.inMin.relQ[i]), 50))).join(' | ')} |`)

md.push('\n#### 노트 기준 nps2 / 1분 평균 의 분위수 (곡별 값의 중앙값)\n')
md.push(`| 대역 | p10 | p25 | p50 | p75 | p90 | p95 | p99 | max |`)
md.push(`| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |`)
for (const g of [...CONDENSED, tableAll]) md.push(`| ${g.label} | ${[0, 1, 2, 3, 4, 5, 6, 7].map((i) => f(quantile(hardRecs(g).map((r) => r.minute.inMin.noteQ[i]), 50))).join(' | ')} |`)

md.push('\n#### 노트 중 nps2 가 1분 평균의 x 배 이상인 노트 비율 (곡별 값의 중앙값)\n')
md.push(`| 대역 | >=1.0 | >=1.1 | >=1.2 | >=1.3 | >=1.5 |`)
md.push(`| --- | ---: | ---: | ---: | ---: | ---: |`)
for (const g of [...CONDENSED, tableAll]) md.push(`| ${g.label} | ${[0, 1, 2, 3, 4].map((i) => pct(quantile(hardRecs(g).map((r) => r.minute.inMin.noteShareAtLeast[i]), 50))).join(' | ')} |`)

md.push('\n#### 시간 비율 구간 (0.5초 격자 nps2 / 1분 평균 기준, 곡별 값의 중앙값)\n')
md.push(`| 대역 | <0.7 | 0.7~1.0 | 1.0~1.25 | >=1.25 |`)
md.push(`| --- | ---: | ---: | ---: | ---: |`)
for (const g of [...CONDENSED, tableAll]) md.push(`| ${g.label} | ${[0, 1, 2, 3].map((i) => pct(quantile(hardRecs(g).map((r) => r.minute.inMin.timeShare[i]), 50))).join(' | ')} |`)

md.push('\n#### 1분 안 휴식·고점·파동의 곡당 개수 (중앙값 [p25~p75])\n')
const countStat = (g: Group, pick: (r: Rec) => number) => {
  const v = hardRecs(g).map(pick)
  return `${f(quantile(v, 50), 1)} [${f(quantile(v, 25), 1)}~${f(quantile(v, 75), 1)}]`
}
const inMinCountRows: [string, (r: Rec) => number][] = [
    ['휴식(<0.5 평균, >=2초) 개수', (r) => r.minute.inMin.rests50.length],
    ['얕은 휴식(<0.7 평균, >=2초) 개수', (r) => r.minute.inMin.rests70.length],
    ['고점(>=1.25 평균, 1초 이하 간격 병합) 구간 개수', (r) => r.minute.inMin.hi125.length],
    ['강한 고점(>=1.4 평균) 구간 개수', (r) => r.minute.inMin.hi140.length],
    ['밀도 파동(돌출 >=0.25 평균) 개수', (r) => r.minute.inMin.waves25.count],
    ['밀도 파동(돌출 >=0.5 평균) 개수', (r) => r.minute.inMin.waves50.count],
]
md.push(`| 지표 | ${CONDENSED.map((g) => g.label).join(' | ')} |`)
md.push(`| --- | ${CONDENSED.map(() => '---:').join(' | ')} |`)
for (const [name, pick] of inMinCountRows) md.push(`| ${name} | ${CONDENSED.map((g) => countStat(g, pick)).join(' | ')} |`)

const poolMin = (g: Group, pick: (r: Rec) => number[]) => hardRecs(g).flatMap(pick)
const POOL_LABEL = ['| 대역 | 개수 | p10 | p25 | p50 | p75 | p90 | max |', '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
const poolBlock = (title: string, pick: (r: Rec) => number[], d = 1) => {
    md.push(`\n#### ${title}\n`, ...POOL_LABEL)
    for (const g of [...CONDENSED, tableAll]) md.push(poolRow(g.label, poolMin(g, pick), d))
}
poolBlock('1분 안 휴식(<0.5 평균) 길이(초)', (r) => r.minute.inMin.rests50)
poolBlock('1분 안 얕은 휴식(<0.7 평균) 길이(초)', (r) => r.minute.inMin.rests70)
poolBlock('1분 안 고점(>=1.25 평균) 구간 길이(초)', (r) => r.minute.inMin.hi125)
poolBlock('1분 안 강한 고점(>=1.4 평균) 구간 길이(초)', (r) => r.minute.inMin.hi140)
poolBlock('1분 안 밀도 파동 간격(돌출 >=0.25 평균, 초)', (r) => r.minute.inMin.waves25.intervalsS)
poolBlock('1분 안 밀도 파동 간격(돌출 >=0.5 평균, 초)', (r) => r.minute.inMin.waves50.intervalsS)
poolBlock('1분 안 밀도 파동 돌출 크기(1분 평균 대비, 돌출 >=0.25)', (r) => r.minute.inMin.waves25.promRel, 2)
poolBlock('인접 5초 구간 사이 변화량 |Δ|/1분 평균 (11개 간격)', (r) => r.minute.inMin.stepAbs12, 2)

md.push('\n#### nps2 분위수 사이 비율 (곡 안에서 p90 을 1.00 으로, 곡별 값의 중앙값 [p25~p75])\n')
scalarTable('p90 기준 환산', [...CONDENSED, tableAll], ['nps2 p50 / p90', 'nps2 p75 / p90', 'nps2 max / p90', 'nps2 p90 / p50', 'nps2 max / p50'])
md.push('\n### 3.8 구성 (동시치기·스크래치·LN)\n')
const COMPOSITION = ['노트/행(동시치기 평균)', '평균 행/초(평균 NPS / 노트/행)', '16분 연타 대비 평균 행 밀도(행/초 / (노트 BPM 중앙값/15))', '2초 최대 행/초', '스크래치 비율', 'LN 비율']
scalarTable('구성 지표', [...CONDENSED, tableAll], COMPOSITION)

md.push('\n### 3.9 레벨과 밀도의 선형 관계 (최소제곱, 곡 단위)\n')
const fit = (xs: number[], ys: number[]) => {
    const mx = mean(xs)
    const my = mean(ys)
    const b = xs.reduce((s, x, i) => s + (x - mx) * (ys[i] - my), 0) / xs.reduce((s, x) => s + (x - mx) ** 2, 0)
    const a = my - b * mx
    const ssTot = ys.reduce((s, y) => s + (y - my) ** 2, 0)
    const ssRes = ys.reduce((s, y, i) => s + (y - (a + b * xs[i])) ** 2, 0)
    return { a, b, r2: 1 - ssRes / ssTot }
}
md.push('| 표 | 지표 | 절편 | 기울기(레벨 1당) | R^2 | 곡 수 |')
md.push('| --- | --- | ---: | ---: | ---: | ---: |')
const fitRows: [string, string, Rec[], (r: Rec) => number, (r: Rec) => number][] = []
const insaneRecs = recs.filter((r) => levelOf(r, 'insane') !== undefined && levelOf(r, 'insane') !== '???')
for (const [lab, key, rs] of [['발광', 'insane', insaneRecs], ['Stella', 'st', stAll.recs], ['Satellite', 'sl', slAll.recs]] as [string, string, Rec[]][]) {
    const lvl = (r: Rec) => Number(levelOf(r, key))
    for (const [mname, pick] of [['평균 NPS', (r: Rec) => r.avgNps], ['nps2 p50', (r: Rec) => r.nps2.p50], ['nps2 p90', (r: Rec) => r.nps2.p90], ['nps2 max', (r: Rec) => r.nps2.max], ['가장 어려운 1분 NPS', (r: Rec) => r.minute?.hardNps ?? NaN]] as [string, (r: Rec) => number][]) {
        const use = rs.filter((r) => Number.isFinite(pick(r)))
        const fr = fit(use.map(lvl), use.map(pick))
        md.push(`| ${lab} | ${mname} | ${f(fr.a)} | ${f(fr.b, 3)} | ${f(fr.r2, 3)} | ${use.length} |`)
        out.fit = out.fit ?? {}
        out.fit[`${key}:${mname}`] = fr
    }
}

md.push('\n### 3.10 nps2 p90 구간별 곡선(전체 곡, 표 안팎 합침)\n')
for (const k of [10]) {
    md.push(`\n#### K=${k} 중앙값 곡선(곡 평균=1.00)\n`)
    md.push(`| 구간 | 곡 수 | ${Array.from({ length: k }, (_, j) => `${j + 1}`).join(' | ')} |`)
    md.push(`| --- | ---: | ${Array.from({ length: k }, () => '---:').join(' | ')} |`)
    for (const g of p90Groups) {
        const cs = curveStats(g.recs, k, 'mean')
        md.push(`| ${g.label} | ${g.recs.length} | ${cs.map((c) => f(c.p50)).join(' | ')} |`)
    }
}
md.push('\n#### 가장 어려운 1분 12구간 중앙값 곡선 (nps2 p90 구간별, 1분 평균=1.00)\n')
md.push(`| 구간 | 곡 수 | ${Array.from({ length: 12 }, (_, j) => `${j * 5}~${j * 5 + 5}s`).join(' | ')} |`)
md.push(`| --- | ---: | ${Array.from({ length: 12 }, () => '---:').join(' | ')} |`)
for (const g of p90Groups) {
    const gs = hardRecs(g)
    md.push(`| ${g.label} | ${gs.length} | ${Array.from({ length: 12 }, (_, j) => f(quantile(gs.map((r) => r.minute.hardRel12[j]), 50))).join(' | ')} |`)
}
md.push('\n#### 1분 원형 A~D 소속 비율 (표 3종 합친 k=4 중심에 가장 가까운 것, nps2 p90 구간별 전체 곡)\n')
const poolK4 = out.kmeans.pooledHard_k4 as { label: string; centroid: number[] }[]
md.push(`| 구간 | 곡 수 | ${poolK4.map((c) => c.label).join(' | ')} |`)
md.push(`| --- | ---: | ${poolK4.map(() => '---:').join(' | ')} |`)
for (const g of [...p90Groups, tableAll]) {
    const gs = hardRecs(g)
    const counts = new Array(poolK4.length).fill(0)
    for (const r of gs) {
        let bi = 0
        let bd = Infinity
        poolK4.forEach((c, i) => {
            const d = dist2(r.minute.hardRel12, c.centroid)
            if (d < bd) {
                bd = d
                bi = i
            }
        })
        counts[bi] += 1
    }
    md.push(`| ${g.label} | ${gs.length} | ${counts.map((c) => pct(c / gs.length)).join(' | ')} |`)
}

writeFileSync(`${dir}/agg.json`, JSON.stringify(out))
writeFileSync(`${dir}/agg.md`, md.join('\n') + '\n')
console.log('groups', MAIN_GROUPS.map((g) => `${g.id}:${g.recs.length}`).join(' '))
console.log('skipped by table', JSON.stringify(
    ['insane', 'st', 'sl'].map((k) => [k, meta.skipped.filter((s: { md5: string }) => tables[k][s.md5] !== undefined).map((s: { md5: string; reason: string }) => `${tables[k][s.md5]}:${s.reason}`)]),
))

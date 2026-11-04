import { readFileSync, readdirSync, writeFileSync } from 'node:fs'

const ROOT = '/Users/hyunseokbyun/development/R-BMS/web/src/server'
const { chartPatternDocumentSchema } = await import(`${ROOT}/dto/chart-pattern.ts`)
const { computeChartFeatures, toPatternInput } = await import(`${ROOT}/service/domain/analysis/chart-features.ts`)
const { localDensitiesOf, peakRowsPerSecondOf } = await import(`${ROOT}/service/domain/analysis/local-density.ts`)
const { classifyPattern, isJudgedNote, createTempoLookup, PATTERN_TAGS } = await import(`${ROOT}/service/domain/analysis/pattern-tags.ts`)

const [patternDir, outPath] = process.argv.slice(2)
if (!patternDir || !outPath) throw new Error('usage: measure.ts <pattern dir> <out ndjson>')

const US = 1_000_000
const GRID_STEP_US = 500_000
const GRID_HALF_US = 1_000_000
const SEGMENTS = 20
const MINUTE_US = 60 * US
const MINUTE_SEGMENTS = 12
const MINUTE_SEGMENT_US = MINUTE_US / MINUTE_SEGMENTS
const PEAK_WINDOWS_S = [2, 4, 8, 16, 30, 60]
const HI_RATIO = 0.8
const REST_RATIOS = [0.25, 0.5]
const REST_MIN_S = 2
const HI_MERGE_GAP_POINTS = 2
const WAVE_PROMINENCE = [0.25, 0.5]
const TOP_PERCENTILE = 90
const SHARE_THRESHOLDS = [0.5, 0.7, 0.8, 0.9]
const MIN_NOTES = 200
const MIN_SPAN_S = 30
const MEDIAN_MINUTE_STRIDE_US = 5 * US
const QUANTILES = [5, 10, 25, 50, 75, 90, 95, 99]

const round = (v: number, d = 3) => Math.round(v * 10 ** d) / 10 ** d
const nearestRank = (sorted: readonly number[], p: number) => sorted[Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)] ?? 0
const lowerBound = (a: readonly number[], v: number) => {
    let lo = 0
    let hi = a.length
    while (lo < hi) {
        const m = (lo + hi) >> 1
        if (a[m] < v) lo = m + 1
        else hi = m
    }
    return lo
}
const mean = (a: readonly number[]) => (a.length === 0 ? 0 : a.reduce((s, x) => s + x, 0) / a.length)
const std = (a: readonly number[]) => {
    const m = mean(a)
    return Math.sqrt(mean(a.map((x) => (x - m) ** 2)))
}

const runsOf = (mask: readonly boolean[], mergeGap: number) => {
    const runs: [number, number][] = []
    let start = -1
    mask.forEach((on, i) => {
        if (on && start < 0) start = i
        if (!on && start >= 0) {
            runs.push([start, i - 1])
            start = -1
        }
    })
    if (start >= 0) runs.push([start, mask.length - 1])
    const merged: [number, number][] = []
    for (const run of runs) {
        const last = merged.at(-1)
        if (last && run[0] - last[1] - 1 <= mergeGap) last[1] = run[1]
        else merged.push([run[0], run[1]])
    }
    return merged
}

const prominentPeaks = (series: readonly number[], minProminence: number) => {
    const compressed: { v: number; pos: number }[] = []
    let i = 0
    while (i < series.length) {
        let j = i
        while (j + 1 < series.length && series[j + 1] === series[i]) j += 1
        compressed.push({ v: series[i], pos: (i + j) / 2 })
        i = j + 1
    }
    const peaks: { pos: number; prom: number }[] = []
    compressed.forEach((point, idx) => {
        const left = compressed[idx - 1]
        const right = compressed[idx + 1]
        if ((left && left.v >= point.v) || (right && right.v >= point.v)) return
        let leftMin = point.v
        for (let k = idx - 1; k >= 0; k -= 1) {
            if (compressed[k].v > point.v) break
            leftMin = Math.min(leftMin, compressed[k].v)
        }
        let rightMin = point.v
        for (let k = idx + 1; k < compressed.length; k += 1) {
            if (compressed[k].v > point.v) break
            rightMin = Math.min(rightMin, compressed[k].v)
        }
        const prom = point.v - Math.max(leftMin, rightMin)
        if (prom >= minProminence) peaks.push({ pos: point.pos, prom })
    })
    return peaks
}

const files = readdirSync(patternDir).filter((n: string) => n.endsWith('.json') && !n.startsWith('_'))
const lines: string[] = []
const skipped: { md5: string; reason: string }[] = []
let crossCheckMaxDiff = 0
let crossChecked = 0

for (const file of files) {
    const doc = chartPatternDocumentSchema.parse(JSON.parse(readFileSync(`${patternDir}/${file}`, 'utf8')))
    const input = toPatternInput(doc)
    const notes = input.notes
    const judgedIdx: number[] = notes.flatMap((note: { kind: string }, i: number) => (isJudgedNote(note) ? [i] : []))
    const T: number[] = judgedIdx.map((i) => notes[i].timeUs)
    const n = T.length
    if (n < MIN_NOTES) {
        skipped.push({ md5: doc.md5, reason: `notes ${n} < ${MIN_NOTES}` })
        continue
    }
    const startUs = T[0]
    const endUs = T[n - 1] + 1
    const spanUs = endUs - startUs
    const spanS = spanUs / US
    if (spanS < MIN_SPAN_S) {
        skipped.push({ md5: doc.md5, reason: `span ${round(spanS, 1)}s < ${MIN_SPAN_S}` })
        continue
    }

    const feat = computeChartFeatures(doc)
    const tagLists: string[][] = classifyPattern(input)
    const dens = localDensitiesOf(notes)
    const nps2 = judgedIdx.map((i) => dens[i].nps2)
    const nps2Sorted = nps2.toSorted((a, b) => a - b)
    const p50 = nearestRank(nps2Sorted, 50)
    const p75 = nearestRank(nps2Sorted, 75)
    const p90 = nearestRank(nps2Sorted, 90)
    const max2 = nps2Sorted.at(-1) ?? 0
    const diff = Math.max(Math.abs(p50 - feat.nps.p50_2s), Math.abs(p90 - feat.nps.p90_2s), Math.abs(max2 - feat.nps.max_2s))
    crossCheckMaxDiff = Math.max(crossCheckMaxDiff, diff)
    crossChecked += 1
    const avgNps = n / spanS

    const lookup = createTempoLookup(input.tempo, input.initBpm)
    const bpmSorted = T.map((t) => lookup.bpmAt(t)).toSorted((a, b) => a - b)

    const gridCount = Math.floor(spanUs / GRID_STEP_US) + 1
    const r: number[] = []
    let lo = 0
    let hi = 0
    for (let k = 0; k < gridCount; k += 1) {
        const g = startUs + k * GRID_STEP_US
        const a = g - GRID_HALF_US
        const b = g + GRID_HALF_US
        while (lo < n && T[lo] < a) lo += 1
        while (hi < n && T[hi] < b) hi += 1
        const clipped = (Math.min(b, endUs) - Math.max(a, startUs)) / US
        r.push((hi - lo) / clipped)
    }
    const rSorted = r.toSorted((a, b) => a - b)
    const rMax = rSorted.at(-1) ?? 0
    const rMaxAt = r.indexOf(rMax)

    const seg = new Array<number>(SEGMENTS).fill(0)
    for (const t of T) seg[Math.min(SEGMENTS - 1, Math.floor(((t - startUs) / spanUs) * SEGMENTS))] += 1
    const segMaxIdx = seg.indexOf(Math.max(...seg))

    const hiMask = r.map((v) => v >= HI_RATIO * rMax)
    const hiZones = runsOf(hiMask, HI_MERGE_GAP_POINTS).map(([s, e]) => (e - s + 1) * (GRID_STEP_US / US))
    const restOf = (ratio: number) =>
        runsOf(
            r.map((v) => v < ratio * avgNps),
            0,
        )
            .map(([s, e]) => (e - s + 1) * (GRID_STEP_US / US))
            .filter((d) => d >= REST_MIN_S)
    const rests = REST_RATIOS.map(restOf)
    const restTotals = rests.map((list) => list.reduce((s, d) => s + d, 0))
    const waves = WAVE_PROMINENCE.map((p) => {
        const peaks = prominentPeaks(r, p * avgNps)
        return {
            count: peaks.length,
            intervalsS: peaks.slice(1).map((peak, i) => round((peak.pos - peaks[i].pos) * (GRID_STEP_US / US), 1)),
            promRel: peaks.map((peak) => round(peak.prom / avgNps, 2)),
        }
    })

    const peakAvg: Record<string, number | null> = {}
    for (const w of PEAK_WINDOWS_S) {
        if (spanS < w) {
            peakAvg[String(w)] = null
            continue
        }
        const wUs = w * US
        let high = 0
        let best = 0
        for (let low = 0; low < n; low += 1) {
            while (high < n && T[high] < T[low] + wUs) high += 1
            best = Math.max(best, high - low)
        }
        peakAvg[String(w)] = round(best / w, 3)
    }

    const countIn = (a: number, b: number) => lowerBound(T, b) - lowerBound(T, a)
    let minute: Record<string, unknown> | null = null
    if (spanUs >= MINUTE_US) {
        const candidates = [...T.filter((t) => t + MINUTE_US <= endUs), endUs - MINUTE_US]
        let bestStart = candidates[0]
        let bestCount = -1
        for (const s of candidates) {
            const c = countIn(s, s + MINUTE_US)
            if (c > bestCount || (c === bestCount && s < bestStart)) {
                bestCount = c
                bestStart = s
            }
        }
        const minuteProfile = (s: number) => {
            const c = countIn(s, s + MINUTE_US)
            const nps12 = Array.from({ length: MINUTE_SEGMENTS }, (_, j) => countIn(s + j * MINUTE_SEGMENT_US, s + (j + 1) * MINUTE_SEGMENT_US) / (MINUTE_SEGMENT_US / US))
            const mu = c / 60
            const rel12 = nps12.map((v) => (mu > 0 ? v / mu : 0))
            return { s, c, mu, nps12, rel12 }
        }
        const hard = minuteProfile(bestStart)
        const inMinuteJudged = judgedIdx.filter((_, k) => T[k] >= hard.s && T[k] < hard.s + MINUTE_US)
        const minuteNps2 = inMinuteJudged.map((i) => dens[i].nps2).toSorted((a, b) => a - b)
        const gridInMinute = r.filter((_, k) => startUs + k * GRID_STEP_US >= hard.s && startUs + k * GRID_STEP_US < hard.s + MINUTE_US).toSorted((a, b) => a - b)
        const tagCounts = PATTERN_TAGS.map((tag: string) => inMinuteJudged.filter((i) => tagLists[i].includes(tag)).length)
        const minuteTimes = [...new Set(inMinuteJudged.map((i) => notes[i].timeUs))]
        const counts: number[] = []
        for (let s = startUs; s + MINUTE_US <= endUs; s += MEDIAN_MINUTE_STRIDE_US) counts.push(countIn(s, s + MINUTE_US))
        const sortedCounts = counts.toSorted((a, b) => a - b)
        const medianCount = sortedCounts[Math.floor(sortedCounts.length / 2)]
        let medianStart = startUs
        let medianBest = Infinity
        for (let s = startUs, idx = 0; s + MINUTE_US <= endUs; s += MEDIAN_MINUTE_STRIDE_US, idx += 1) {
            const d = Math.abs(counts[idx] - medianCount)
            if (d < medianBest) {
                medianBest = d
                medianStart = s
            }
        }
        const med = minuteProfile(medianStart)
        const relGrid = gridInMinute.length === 0 ? [] : r.flatMap((v, k) => {
            const g = startUs + k * GRID_STEP_US
            return g >= hard.s && g < hard.s + MINUTE_US ? [v / hard.mu] : []
        })
        const relSorted = relGrid.toSorted((a, b) => a - b)
        const stepSec = GRID_STEP_US / US
        const lensOf = (mask: boolean[], gap: number, minS: number) =>
            runsOf(mask, gap)
                .map(([s, e]) => (e - s + 1) * stepSec)
                .filter((d) => d >= minS)
        const noteRel = inMinuteJudged.map((i) => dens[i].nps2 / hard.mu).toSorted((a, b) => a - b)
        const inMin = {
            relQ: [5, 10, 25, 50, 75, 90, 95, 100].map((p) => round(nearestRank(relSorted, p), 3)),
            timeShare: [
                round(relGrid.filter((v) => v < 0.7).length / relGrid.length, 4),
                round(relGrid.filter((v) => v >= 0.7 && v < 1).length / relGrid.length, 4),
                round(relGrid.filter((v) => v >= 1 && v < 1.25).length / relGrid.length, 4),
                round(relGrid.filter((v) => v >= 1.25).length / relGrid.length, 4),
            ],
            rests50: lensOf(relGrid.map((v) => v < 0.5), 0, REST_MIN_S),
            rests70: lensOf(relGrid.map((v) => v < 0.7), 0, REST_MIN_S),
            hi125: lensOf(relGrid.map((v) => v >= 1.25), HI_MERGE_GAP_POINTS, 0),
            hi140: lensOf(relGrid.map((v) => v >= 1.4), HI_MERGE_GAP_POINTS, 0),
            waves25: ((peaks) => ({ count: peaks.length, intervalsS: peaks.slice(1).map((peak, i) => round((peak.pos - peaks[i].pos) * stepSec, 1)), promRel: peaks.map((peak) => round(peak.prom, 2)) }))(prominentPeaks(relGrid, 0.25)),
            waves50: ((peaks) => ({ count: peaks.length, intervalsS: peaks.slice(1).map((peak, i) => round((peak.pos - peaks[i].pos) * stepSec, 1)) }))(prominentPeaks(relGrid, 0.5)),
            noteQ: [10, 25, 50, 75, 90, 95, 99, 100].map((p) => round(nearestRank(noteRel, p), 3)),
            noteShareAtLeast: [1, 1.1, 1.2, 1.3, 1.5].map((x) => round(noteRel.filter((v) => v >= x).length / noteRel.length, 4)),
            stepAbs12: hard.rel12.slice(1).map((v, j) => round(Math.abs(v - hard.rel12[j]), 3)),
        }
        minute = {
            inMin,
            hardNps: round(hard.mu, 3),
            hardStartFrac: round((hard.s - startUs) / spanUs, 3),
            hardRel12: hard.rel12.map((v) => round(v, 3)),
            hardCv12: round(hard.mu > 0 ? std(hard.nps12) / hard.mu : 0, 3),
            hardRange12: round(hard.mu > 0 ? (Math.max(...hard.nps12) - Math.min(...hard.nps12)) / hard.mu : 0, 3),
            hardMin12: round(Math.min(...hard.rel12), 3),
            hardMax12: round(Math.max(...hard.rel12), 3),
            hardDips: hard.rel12.filter((v) => v < 0.7).length,
            hardNps2: { p10: round(nearestRank(gridInMinute, 10), 2), p50: round(nearestRank(gridInMinute, 50), 2), p90: round(nearestRank(gridInMinute, 90), 2), max: round(gridInMinute.at(-1) ?? 0, 2), min: round(gridInMinute[0] ?? 0, 2) },
            hardNoteNps2: { p50: round(nearestRank(minuteNps2, 50), 2), p90: round(nearestRank(minuteNps2, 90), 2), max: round(minuteNps2.at(-1) ?? 0, 2) },
            hardNotes: hard.c,
            hardTagCounts: tagCounts,
            hardRows: minuteTimes.length,
            medNps: round(med.mu, 3),
            medRel12: med.rel12.map((v) => round(v, 3)),
            medCv12: round(med.mu > 0 ? std(med.nps12) / med.mu : 0, 3),
            minuteCountP50: medianCount,
        }
    }

    const topIdx = judgedIdx.filter((_, k) => nps2[k] >= p90)
    const tagTop = PATTERN_TAGS.map((tag: string) => topIdx.filter((i) => tagLists[i].includes(tag)).length)
    const rows = new Set(T).size

    const record = {
        md5: doc.md5,
        playLevelHeader: doc.play_level,
        declaredLn: doc.declared_ln,
        n,
        lengthS: round(feat.length_ms / 1000, 1),
        firstS: round(startUs / US, 2),
        lastS: round((endUs - 1) / US, 2),
        spanS: round(spanS, 2),
        avgNps: round(avgNps, 3),
        selectAvg: feat.density.average,
        selectPeak: feat.density.peak,
        selectLateAvg: feat.density.late_average,
        engine: { p50_2s: feat.nps.p50_2s, p90_2s: feat.nps.p90_2s, max_2s: feat.nps.max_2s, max_8s: feat.nps.max_8s, sustained_s: feat.nps.sustained_s },
        nps2: { p50: round(p50, 3), p75: round(p75, 3), p90: round(p90, 3), max: round(max2, 3) },
        nps2Q: QUANTILES.map((p) => round(nearestRank(nps2Sorted, p), 2)),
        nps1Max: round(Math.max(...judgedIdx.map((i) => dens[i].nps1)), 2),
        nps8Max: feat.nps.max_8s,
        rMax: round(rMax, 3),
        rMaxPos: round((rMaxAt * GRID_STEP_US) / spanUs, 3),
        rQ: [10, 25, 50, 75, 90].map((p) => round(nearestRank(rSorted, p), 2)),
        peakAvg,
        seg20: seg,
        segMaxPos: round((segMaxIdx + 0.5) / SEGMENTS, 3),
        shareNotesAtLeast: SHARE_THRESHOLDS.map((x) => round(nps2.filter((v) => v >= x * max2).length / n, 4)),
        shareTimeAtLeast: SHARE_THRESHOLDS.map((x) => round(r.filter((v) => v >= x * rMax).length / r.length, 4)),
        shareTimeAboveMean125: round(r.filter((v) => v >= 1.25 * avgNps).length / r.length, 4),
        shareTimeBelowMean75: round(r.filter((v) => v < 0.75 * avgNps).length / r.length, 4),
        hiZonesS: hiZones,
        hiZonesShare: round(hiMask.filter(Boolean).length / r.length, 4),
        restsS: rests,
        restShare: restTotals.map((t) => round(t / spanS, 4)),
        waves,
        minute,
        tagsAll: PATTERN_TAGS.map((tag: string) => feat.tags[tag].notes),
        tagTop,
        topSize: topIdx.length,
        bpm: { init: input.initBpm, p10: round(nearestRank(bpmSorted, 10), 1), p50: round(nearestRank(bpmSorted, 50), 1), p90: round(nearestRank(bpmSorted, 90), 1), mainBand: feat.bpm_main_band, min: feat.soflan.bpm.min, max: feat.soflan.bpm.max },
        soflanJumps: feat.soflan.tempo_jumps,
        longStops: feat.soflan.long_stops,
        rows,
        notesPerRow: round(n / rows, 3),
        peakRows2s: peakRowsPerSecondOf(notes),
        scratchShare: round(judgedIdx.filter((i) => notes[i].lane === 7).length / n, 4),
        lnRatio: feat.ln_ratio,
    }
    lines.push(JSON.stringify(record))
}

writeFileSync(outPath, lines.join('\n') + '\n')
writeFileSync(
    outPath.replace(/\.ndjson$/, '.meta.json'),
    JSON.stringify({ files: files.length, measured: lines.length, skipped, tags: PATTERN_TAGS, crossChecked, crossCheckMaxDiff }, null, 1),
)
console.log(`files ${files.length}, measured ${lines.length}, skipped ${skipped.length}, engine cross-check max diff ${crossCheckMaxDiff} over ${crossChecked}`)

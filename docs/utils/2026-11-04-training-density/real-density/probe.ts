import { readFileSync, readdirSync } from 'node:fs'
const ROOT = '/Users/hyunseokbyun/development/R-BMS/web/src/server'
const { chartPatternDocumentSchema } = await import(`${ROOT}/dto/chart-pattern.ts`)
const { computeChartFeatures, toPatternInput } = await import(`${ROOT}/service/domain/analysis/chart-features.ts`)
const { localDensitiesOf } = await import(`${ROOT}/service/domain/analysis/local-density.ts`)
const dir = process.argv[2]
const f = readdirSync(dir).filter((n: string) => !n.startsWith('_'))[0]
const doc = chartPatternDocumentSchema.parse(JSON.parse(readFileSync(`${dir}/${f}`, 'utf8')))
const t0 = performance.now()
const feat = computeChartFeatures(doc)
console.log(f, feat.judged_notes, feat.length_ms, feat.nps, feat.density, feat.bpm_main_band, (performance.now() - t0).toFixed(1) + 'ms')
const input = toPatternInput(doc)
console.log(input.notes.length, localDensitiesOf(input.notes).slice(0, 2))

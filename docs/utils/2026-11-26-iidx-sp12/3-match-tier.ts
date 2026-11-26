import { readFile, writeFile } from 'node:fs/promises'
const [tierPath, chartsPath, outPath] = process.argv.slice(2)
const tiers: any[] = JSON.parse(await readFile(tierPath, 'utf8'))
const charts: any[] = JSON.parse(await readFile(chartsPath, 'utf8'))
const VERS = ['5th','6th','7th','8th','9th','10th','IIDXRED','HAPPY SKY','DistorteD','GOLD','DJ TROOPERS','EMPRESS','SIRIUS','Resort Anthem','Lincle','tricoro','SPADA','PENDUAL','copula']
const WORD: Record<string, string> = { beginner: 'B', light: 'N', normal: 'N', hyper: 'H', another: 'A', leggendaria: 'L', leg: 'L', n: 'N', h: 'H', a: 'A', l: 'L', b: 'B' }
const parseName = (raw: string) => {
  const s = raw.replace(/\.(bms|bme|bml|pms|bmson)$/i, '')
  let m
  if ((m = /\[(?:\d+\s+)?(SP|DP)\s+(BEGINNER|NORMAL|HYPER|ANOTHER|LEGGENDARIA)\]/i.exec(s))) return { side: m[1].toUpperCase(), diff: WORD[m[2].toLowerCase()] }
  if ((m = /\[(Light|Normal|Hyper|Another|Beginner)(7|14)\]/i.exec(s))) return { side: m[2] === '7' ? 'SP' : 'DP', diff: WORD[m[1].toLowerCase()] }
  if ((m = /\[(7|14)key\]/i.exec(s))) return { side: m[1] === '7' ? 'SP' : 'DP', diff: 'H' }
  if ((m = /\[(?:\d+\s+)?([1D])([BNHAL])(?:\s+[^\]]*)?\]/.exec(s))) return { side: m[1] === 'D' ? 'DP' : 'SP', diff: m[2] }
  if ((m = /\(\d+\s+(D?)([BNHAL])\)/.exec(s))) return { side: m[1] ? 'DP' : 'SP', diff: m[2] }
  if ((m = /^(leg|[bnhal])(7|14)?(?:\s*\(\d+\))?$/i.exec(s.trim()))) return { side: m[2] === '14' ? 'DP' : m[2] === '7' ? 'SP' : null, diff: WORD[m[1].toLowerCase()] }
  return null
}
const HEADER_DIFF: Record<string, string> = { B: 'B', N: 'N', H: 'H', A: 'A', L: 'L' }
const describe = (c: any) => {
  const file = c.path.split('/').pop()
  const named = parseName(file) ?? parseName(c.title)
  const parsed = named?.diff ?? HEADER_DIFF[c.difficulty] ?? '?'
  const diff = /leggendaria/i.test(file + ' ' + c.title) && (parsed === 'A' || parsed === '?' || parsed === 'H') ? 'L' : parsed
  const side = named?.side ?? (c.player === '3' ? 'DP' : c.player === '1' ? 'SP' : null)
  return { diff, side }
}
const stripTitle = (t: string) => t.replace(/\s*[\[(（][^\])）]*(?:another|hyper|normal|light|beginner|leggendaria|7key|14key|sp |dp )[^\])）]*[\])）]\s*/gi, ' ').replace(/†\s*(leggendaria)?\s*$/i, '')
const looseKey = (t: string) => stripTitle(t).normalize('NFKC').normalize('NFKD').replace(/\p{L}\p{M}+/gu, '').toLowerCase().replace(/[^\p{L}\p{N}]/gu, '')
const key = (t: string) => stripTitle(t).normalize('NFKC').normalize('NFKD').replace(/\p{M}/gu, '').toLowerCase().replace(/[^\p{L}\p{N}]/gu, '')
const ROOT = '/Volumes/SSD/iidxbms/'
const DP_CHANNEL = /^#\d{3}2[1-9]:/m
const sideByChannels = async (c: any) => (DP_CHANNEL.test(await Bun.file(ROOT + c.path).text()) ? 'DP' : 'SP')
const described = charts.map((c) => ({ ...c, ...describe(c), key: key(c.title) }))
const enriched = await Promise.all(described.map(async (c) => (c.side === null ? { ...c, side: await sideByChannels(c), sideFrom: 'channels' } : c)))
const sp = enriched.filter((c) => c.side !== 'DP')
const folderOf = (c: any) => c.path.split('/').slice(0, -1).join('/')
const songOfFolder = (folder: string) => key(folder.split('/').pop()?.split(' - ').slice(1).pop() ?? '')
const folderKeys = new Map([...Map.groupBy(enriched, folderOf)].map(([folder, list]) => [folder, new Set([...list.map((c) => c.key), songOfFolder(folder)].filter((k) => k.length > 0))]))
const byKey = new Map<string, any[]>()
for (const c of sp) for (const k of folderKeys.get(folderOf(c)) ?? []) byKey.set(k, [...(byKey.get(k) ?? []), c])
const scoped = tiers.filter((t) => VERS.includes(t.version))
const matched: any[] = [], unmatched: any[] = []
for (const t of scoped) {
  const cands = [...new Set([...(byKey.get(key(t.title)) ?? []), ...(byKey.get(looseKey(t.title)) ?? [])])].filter((c) => c.diff === t.difficulty && c.side === 'SP')
  if (cands.length) matched.push({ tier: t, charts: cands }); else unmatched.push(t)
}
const matchedMd5 = new Set(matched.flatMap((m) => m.charts.map((c: any) => c.md5)))
const header12 = sp.filter((c) => c.level === '12' && c.side === 'SP')
const header12Only = header12.filter((c) => !matchedMd5.has(c.md5))
await writeFile(outPath, JSON.stringify({ matched, unmatched, header12Only, enriched }, null, 1))
console.log('tiers <=23', scoped.length, 'matched', matched.length, 'charts', matchedMd5.size, 'unmatched', unmatched.length)
console.log('side', JSON.stringify(Object.fromEntries(Object.entries(Object.groupBy(enriched, (c) => String(c.side))).map(([k, v]) => [k, v?.length]))), 'diff?', enriched.filter((c) => c.diff === '?').length)
console.log('header SP12 total', header12.length, 'not in tier', header12Only.length)
for (const t of unmatched) {
  const k = key(t.title).slice(0, 5)
  const near = [...new Set(enriched.filter((c) => c.key.includes(k)).map((c) => c.title))].slice(0, 3)
  console.log(`  ${t.version} ${t.difficulty} | ${t.title} | near: ${near.join(' / ')}`)
}

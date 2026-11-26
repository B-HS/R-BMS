import { readFile, writeFile } from 'node:fs/promises'
const [joinPath, blockedPath, outPath] = process.argv.slice(2)
const { matched, header12Only } = JSON.parse(await readFile(joinPath, 'utf8'))
const listed = new Set((JSON.parse(await readFile(blockedPath, 'utf8')) as { md5: string }[]).map((e) => e.md5))
const NAME: Record<string, string> = { B: 'BEGINNER', N: 'NORMAL', H: 'HYPER', A: 'ANOTHER', L: 'LEGGENDARIA' }
const entries = new Map<string, string>()
for (const m of matched) for (const c of m.charts) entries.set(c.md5, `${m.tier.title} [${NAME[m.tier.difficulty]}]`)
for (const c of header12Only) if (!entries.has(c.md5)) entries.set(c.md5, `${c.title.replace(/\s*\[[^\]]*\]\s*$/, '')}${NAME[c.diff] ? ` [${NAME[c.diff]}]` : ''}`)
const charts = [...entries].map(([md5, title]) => ({ md5, title })).toSorted((a, b) => a.title.localeCompare(b.title, 'ja'))
const notListed = charts.filter((c) => !listed.has(c.md5))
await writeFile(outPath, `${JSON.stringify({ label: 'SP 12', source: 'https://iidx.hyns.dev/table (SP 12 tier list, matched 2026-11-26) and #PLAYLEVEL 12 SP charts of the collected IIDX folder', charts }, null, 4)}\n`)
console.log('charts', charts.length, 'from tier', matched.reduce((s: number, m: any) => s + m.charts.length, 0), 'header only', header12Only.length, 'not in blocklist', notListed.length)

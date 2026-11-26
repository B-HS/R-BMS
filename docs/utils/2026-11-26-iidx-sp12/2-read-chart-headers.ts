import { readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { findChartFiles } from '../../../web/scripts/chart-blocklist'
const [tierPath, root, outPath] = process.argv.slice(2)
type Tier = { id: string; title: string; difficulty: string; version: string; normalRank: string; hardRank: string }
const tiers: Tier[] = JSON.parse(await readFile(tierPath, 'utf8'))
const sjis = new TextDecoder('shift_jis')
const utf8 = new TextDecoder('utf-8', { fatal: true })
const decode = (bytes: Uint8Array) => { try { return utf8.decode(bytes) } catch { return sjis.decode(bytes) } }
const header = (text: string, key: string) => text.match(new RegExp(`^#${key}[ \\t]+(.*)$`, 'mi'))?.[1]?.trim() ?? ''
const DIFFICULTY_NAMES: Record<string, string> = { '1': 'B', '2': 'N', '3': 'H', '4': 'A', '5': 'L' }
const norm = (title: string) => title.normalize('NFKC').toLowerCase().replace(/[\s\-~〜・･'’"“”!！?？.,:;()（）\[\]［］†＊*&＆]/g, '')
const charts = []
for (const path of await findChartFiles(root)) {
  const bytes = new Uint8Array(await readFile(path))
  const text = decode(bytes.slice(0, 8192))
  const md5 = new Bun.CryptoHasher('md5').update(bytes).digest('hex')
  charts.push({ md5, path: path.slice(root.length + 1), title: header(text, 'TITLE'), subtitle: header(text, 'SUBTITLE'), level: header(text, 'PLAYLEVEL'), difficulty: DIFFICULTY_NAMES[header(text, 'DIFFICULTY')] ?? '?', player: header(text, 'PLAYER') })
}
await writeFile(outPath, JSON.stringify(charts, null, 1))
const stat = (key: string) => Object.entries(Object.groupBy(charts, (c: any) => c[key])).map(([k, v]) => `${k}:${v?.length}`).sort().join(' ')
console.log('charts', charts.length); console.log('player', stat('player')); console.log('difficulty', stat('difficulty')); console.log('level', stat('level'))

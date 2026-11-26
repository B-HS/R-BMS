import { readFile, writeFile } from 'node:fs/promises'
const [htmlPath, outPath] = process.argv.slice(2)
const html = (await readFile(htmlPath, 'utf8')).replaceAll('\\"', '"').replaceAll('\\\\', '\\')
const pattern = /\{"id":"(chart-[0-9a-f]+)","title":"((?:[^"\\]|\\.)*)","difficulty":"([A-Z]+)","version":"([^"]*)","normalRank":"([^"]*)","hardRank":"([^"]*)"/g
const rows = [...html.matchAll(pattern)].map((m) => ({ id: m[1], title: JSON.parse(`"${m[2]}"`), difficulty: m[3], version: m[4], normalRank: m[5], hardRank: m[6] }))
const unique = [...new Map(rows.map((row) => [row.id, row])).values()]
await writeFile(outPath, JSON.stringify(unique, null, 1))
const count = (key: keyof (typeof unique)[number]) => Object.entries(Object.groupBy(unique, (row) => String(row[key]))).map(([k, v]) => `${k}:${v?.length}`).join(' ')
console.log('rows', rows.length, 'unique', unique.length)
console.log('difficulty', count('difficulty'))
console.log('normalRank', count('normalRank'))
console.log('hardRank', count('hardRank'))
console.log('version', count('version'))

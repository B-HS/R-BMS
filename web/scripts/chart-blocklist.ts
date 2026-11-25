import { createHash } from 'node:crypto'
import { readdir, readFile, writeFile } from 'node:fs/promises'
import { extname, join, relative, resolve, sep } from 'node:path'
import { CHART_BLOCKLIST_ID_PATTERN, chartBlocklistSourceSchema, type ChartBlocklistEntry } from '../src/server/dto/chart-blocklist'

/** The chart files the client scans (`rbms-library::is_chart_file`), compared without case. */
export const CHART_FILE_EXTENSIONS = new Set(['.bms', '.bme', '.bml', '.pms', '.bmson'])
export const BLOCKLIST_DIR = resolve(import.meta.dirname, '../data/chart-blocklist')

const HIDDEN_NAME_PREFIX = '.'
const EXIT_CODE = { OK: 0, FAILED: 1, USAGE: 2 } as const
const USAGE = `usage: bun scripts/chart-blocklist.ts <id> <label> <folder>
  <id>: kebab-case file name under web/data/chart-blocklist/ (e.g. iidx-18-resort-anthem)
  <label>: what the charts are (e.g. "beatmania IIDX 18 Resort Anthem")
  <folder>: the folder whose chart files (.bms .bme .bml .pms .bmson) are hashed`

export type BlocklistArgs = { id: string; label: string; folder: string }

export const parseBlocklistArgs = (argv: readonly string[]): BlocklistArgs | null => {
    const [id, label, folder, ...rest] = argv
    if (!id || !label || !folder || rest.length > 0 || !CHART_BLOCKLIST_ID_PATTERN.test(id)) return null
    return { id, label: label.trim(), folder }
}

export const isChartFileName = (name: string) => !name.startsWith(HIDDEN_NAME_PREFIX) && CHART_FILE_EXTENSIONS.has(extname(name).toLowerCase())

/** Every chart file under `root`, depth first, without following links. Hidden files and folders (`.DS_Store`, `._x.bms`) are left out. */
export const findChartFiles = async (root: string): Promise<string[]> => {
    const entries = await readdir(root, { withFileTypes: true })
    const nested = await Promise.all(
        entries
            .filter((entry) => !entry.name.startsWith(HIDDEN_NAME_PREFIX))
            .map(async (entry) => {
                const path = join(root, entry.name)
                if (entry.isDirectory()) return findChartFiles(path)
                return entry.isFile() && isChartFileName(entry.name) ? [path] : []
            }),
    )
    return nested.flat()
}

const hex = (algorithm: 'md5' | 'sha256', bytes: Buffer) => createHash(algorithm).update(bytes).digest('hex')

/** The md5 and sha256 of the raw bytes, as the client hashes a chart (`rbms-parser`), and the path under `root` with `/` and NFC. */
export const hashChartFile = async (root: string, path: string): Promise<ChartBlocklistEntry> => {
    const bytes = await readFile(path)
    return { md5: hex('md5', bytes), sha256: hex('sha256', bytes), path: relative(root, path).split(sep).join('/').normalize('NFC') }
}

/** The blocklist source of one folder: one entry per distinct chart (the first path of identical files), sorted by path. */
export const buildBlocklistSource = async (args: BlocklistArgs) => {
    const root = resolve(args.folder)
    const hashed = await Promise.all((await findChartFiles(root)).map((path) => hashChartFile(root, path)))
    const sorted = hashed.toSorted((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0))
    const charts = [...Map.groupBy(sorted, (entry) => entry.sha256).values()].map(([first]) => first)
    return chartBlocklistSourceSchema.parse({ id: args.id, label: args.label, charts })
}

const main = async () => {
    const args = parseBlocklistArgs(process.argv.slice(2))
    if (!args) {
        console.error(USAGE)
        return EXIT_CODE.USAGE
    }
    const source = await buildBlocklistSource(args)
    const target = join(BLOCKLIST_DIR, `${source.id}.json`)
    await writeFile(target, `${JSON.stringify(source, null, 4)}\n`)
    console.log(`${relative(process.cwd(), target)}: ${source.charts.length} charts`)
    return EXIT_CODE.OK
}

if (import.meta.main) process.exit(await main())

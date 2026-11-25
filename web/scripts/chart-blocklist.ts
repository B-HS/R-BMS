import { createHash } from 'node:crypto'
import { readdir, readFile, writeFile } from 'node:fs/promises'
import { basename, extname, join, relative, resolve, sep } from 'node:path'
import { format, resolveConfig } from 'prettier'
import {
    CHART_BLOCKLIST_ID_PATTERN,
    chartBlocklistSourceSchema,
    type ChartBlocklistEntry,
    type ChartBlocklistSource,
} from '../src/server/dto/chart-blocklist'

/** The chart files the client scans (`rbms-library::is_chart_file`), compared without case. */
export const CHART_FILE_EXTENSIONS = new Set(['.bms', '.bme', '.bml', '.pms', '.bmson'])
const WEB_DIR = resolve(import.meta.dirname, '..')
export const BLOCKLIST_DIR = resolve(import.meta.dirname, '../data/chart-blocklist')
export const DATA_MODULE_PATH = resolve(import.meta.dirname, '../src/server/service/domain/chart-blocklist/chart-blocklist.data.ts')

const DATA_IMPORT_DIR = '../../../../../data/chart-blocklist'
const HIDDEN_NAME_PREFIX = '.'
const SOURCE_FILE_EXTENSION = '.json'
const HASH_BATCH_SIZE = 32
const TESTS_TO_RUN = ['tests/service/chart-blocklist.test.ts', 'tests/scripts/chart-blocklist.test.ts']
const PURGE_COUNT_ACTION = 'chart-blocklist-purge-count'
const EXIT_CODE = { OK: 0, FAILED: 1, USAGE: 2 } as const
const IIDX_FOLDER_PATTERN = /iidx\s*(\d+)(st|nd|rd|th)?\b\s*(.*)$/i

const USAGE = `usage:
  bun run blocklist:add <folder>... [--id <id>] [--label <label>] [--publish]
      Hashes the chart files (.bms .bme .bml .pms .bmson) of each folder into web/data/chart-blocklist/<id>.json,
      leaving out charts another list already has and merging into the list of the same id, then writes the
      server's list module again. The id and label come from the folder name ("Beatmania IIDX20 ~tricoro~" ->
      iidx-20-tricoro, "beatmania IIDX 20 tricoro"); --id and --label override them for a single folder.
      --publish then runs the blocklist tests, commits only the blocklist files, pushes, and starts the
      web-db ${PURGE_COUNT_ACTION} workflow.
  bun run blocklist:check
      Fails when the list module and the files of web/data/chart-blocklist/ disagree.`

export type AddCommand = { kind: 'add'; folders: string[]; id: string | null; label: string | null; shouldPublish: boolean }
export type BlocklistCommand = AddCommand | { kind: 'check' }

export const parseBlocklistCommand = (argv: readonly string[]): BlocklistCommand | null => {
    const [kind, ...rest] = argv
    if (kind === 'check') return rest.length === 0 ? { kind: 'check' } : null
    if (kind !== 'add') return null
    const command: AddCommand = { kind: 'add', folders: [], id: null, label: null, shouldPublish: false }
    const queue = [...rest]
    while (queue.length > 0) {
        const arg = queue.shift() ?? ''
        if (arg === '--') continue
        if (arg === '--publish') command.shouldPublish = true
        else if (arg === '--id') command.id = queue.shift() ?? ''
        else if (arg === '--label') command.label = (queue.shift() ?? '').trim()
        else if (arg.startsWith('--')) return null
        else command.folders.push(arg)
    }
    if (command.folders.length === 0) return null
    if (command.id !== null && !CHART_BLOCKLIST_ID_PATTERN.test(command.id)) return null
    if (command.label === '') return null
    if ((command.id !== null || command.label !== null) && command.folders.length > 1) return null
    return command
}

const slugOf = (text: string) =>
    text
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-+|-+$/g, '')

/** The id and label a pack folder is listed under, or null when its name gives no usable id. */
export const deriveSourceName = (folderName: string) => {
    const name = folderName.normalize('NFC').trim()
    const iidx = IIDX_FOLDER_PATTERN.exec(name)
    if (iidx) {
        const [, version, ordinal, rest] = iidx
        const title = rest.replace(/[~〜]/g, ' ').replace(/\s+/g, ' ').trim()
        const id = slugOf(`iidx ${version} ${title}`)
        return CHART_BLOCKLIST_ID_PATTERN.test(id) ? { id, label: `beatmania IIDX ${version}${ordinal ?? ''} ${title}`.trim() } : null
    }
    const id = slugOf(name)
    return CHART_BLOCKLIST_ID_PATTERN.test(id) ? { id, label: name } : null
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

const byPath = (a: ChartBlocklistEntry, b: ChartBlocklistEntry) => Number(a.path > b.path) - Number(a.path < b.path)

const distinctBySha256 = (entries: readonly ChartBlocklistEntry[]) =>
    [...Map.groupBy(entries, (entry) => entry.sha256).values()].map(([first]) => first)

/** One entry per distinct chart file of the folder (the first path of identical files), sorted by path. */
export const scanFolder = async (folder: string) => {
    const root = resolve(folder)
    const paths = await findChartFiles(root)
    const batches = Array.from({ length: Math.ceil(paths.length / HASH_BATCH_SIZE) }, (_, index) =>
        paths.slice(index * HASH_BATCH_SIZE, (index + 1) * HASH_BATCH_SIZE),
    )
    const hashed: ChartBlocklistEntry[][] = []
    for (const batch of batches) hashed.push(await Promise.all(batch.map((path) => hashChartFile(root, path))))
    return distinctBySha256(hashed.flat().toSorted(byPath))
}

export const readSources = async (dir = BLOCKLIST_DIR) => {
    const names = (await readdir(dir)).filter((name) => name.endsWith(SOURCE_FILE_EXTENSION)).toSorted()
    return Promise.all(names.map(async (name) => chartBlocklistSourceSchema.parse(JSON.parse(await readFile(join(dir, name), 'utf8')))))
}

/**
 * The list `id` becomes once `scanned` is added: the charts it already had, plus the scanned ones no other list has. A chart is
 * already listed when its sha256 or its md5 is in another list.
 */
export const mergeSource = (params: {
    id: string
    label: string
    scanned: readonly ChartBlocklistEntry[]
    sources: readonly ChartBlocklistSource[]
}) => {
    const { id, label, scanned, sources } = params
    const existing = sources.find((source) => source.id === id)
    const others = sources.filter((source) => source.id !== id).flatMap((source) => source.charts)
    const otherSha256s = new Set(others.map((chart) => chart.sha256))
    const otherMd5s = new Set(others.map((chart) => chart.md5))
    const kept = existing?.charts ?? []
    const keptSha256s = new Set(kept.map((chart) => chart.sha256))
    const fresh = scanned.filter((chart) => !otherSha256s.has(chart.sha256) && !otherMd5s.has(chart.md5))
    const added = fresh.filter((chart) => !keptSha256s.has(chart.sha256))
    const charts = distinctBySha256([...kept, ...added].toSorted(byPath))
    return { source: { id, label: existing?.label ?? label, charts }, added: added.length, alreadyListed: scanned.length - fresh.length }
}

const identifierOf = (id: string) => id.replace(/-([a-z0-9])/g, (_, next: string) => next.toUpperCase())

/** The text of `chart-blocklist.data.ts` for the lists `ids`, formatted as the repository's prettier does. */
export const renderDataModule = async (ids: readonly string[]) => {
    const sorted = ids.toSorted()
    const imports = sorted.map((id) => `import ${identifierOf(id)} from '${DATA_IMPORT_DIR}/${id}${SOURCE_FILE_EXTENSION}'`).join('\n')
    const text = `${imports}

/** Every committed file of \`web/data/chart-blocklist/\`, unparsed. Written by \`scripts/chart-blocklist.ts\`: run \`bun run blocklist:add\`, never edit. */
export const CHART_BLOCKLIST_RAW_SOURCES: readonly unknown[] = [${sorted.map(identifierOf).join(', ')}]
`
    const config = await resolveConfig(DATA_MODULE_PATH)
    return format(text, { ...config, filepath: DATA_MODULE_PATH })
}

/** What is wrong between the list files and the list module: an id that is not its file name, or a module that is not the one `add` writes. */
export const checkBlocklist = async (dir = BLOCKLIST_DIR, modulePath = DATA_MODULE_PATH) => {
    const names = (await readdir(dir)).filter((name) => name.endsWith(SOURCE_FILE_EXTENSION)).toSorted()
    const sources = await readSources(dir)
    const misnamed = sources.filter((source, index) => `${source.id}${SOURCE_FILE_EXTENSION}` !== names[index]).map((source) => source.id)
    const expected = await renderDataModule(sources.map((source) => source.id))
    const current = await readFile(modulePath, 'utf8')
    return [
        ...misnamed.map((id) => `${id}: the id is not its file name`),
        ...(expected === current ? [] : [`${relative(process.cwd(), modulePath)} is out of date: run bun run blocklist:add or write it again`]),
    ]
}

const run = (command: string[]) => Bun.spawnSync(command, { cwd: WEB_DIR, stdout: 'inherit', stderr: 'inherit' }).exitCode === 0

const output = (command: string[]) => Bun.spawnSync(command, { cwd: WEB_DIR, stdout: 'pipe', stderr: 'inherit' }).stdout.toString().trim()

const publish = (params: { changedIds: string[]; total: number }) => {
    const { changedIds, total } = params
    if (!run(['git', 'diff', '--cached', '--quiet'])) {
        console.error('publish stopped: other changes are staged. Commit or unstage them first')
        return EXIT_CODE.FAILED
    }
    if (!run(['bun', 'test', ...TESTS_TO_RUN])) return EXIT_CODE.FAILED
    const paths = [...changedIds.map((id) => join(BLOCKLIST_DIR, `${id}${SOURCE_FILE_EXTENSION}`)), DATA_MODULE_PATH]
    if (!run(['git', 'add', '--', ...paths])) return EXIT_CODE.FAILED
    const message = `feat(web): 차트 차단 목록에 ${changedIds.join('·')} 추가(${total}개)`
    if (!run(['git', 'commit', '-q', '-m', message])) return EXIT_CODE.FAILED
    const branch = output(['git', 'rev-parse', '--abbrev-ref', 'HEAD'])
    if (!run(['git', 'push', '-q', 'origin', branch])) return EXIT_CODE.FAILED
    console.log(`pushed ${output(['git', 'rev-parse', '--short', 'HEAD'])} to ${branch}: the web deploy starts from it`)
    if (!run(['gh', 'workflow', 'run', 'web-db.yml', '--ref', branch, '-f', `action=${PURGE_COUNT_ACTION}`])) return EXIT_CODE.FAILED
    console.log(`started web-db ${PURGE_COUNT_ACTION}: gh run list --workflow web-db.yml --limit 1`)
    console.log('if it counts rows, delete them with: gh workflow run web-db.yml --ref dev -f action=chart-blocklist-purge-delete')
    return EXIT_CODE.OK
}

const add = async (command: AddCommand) => {
    const changedIds: string[] = []
    let total = 0
    for (const folder of command.folders) {
        const derived = deriveSourceName(basename(resolve(folder)))
        const id = command.id ?? derived?.id
        const label = command.label ?? derived?.label
        if (!id || !label) {
            console.error(`${folder}: no id could be made from the folder name, give --id and --label`)
            return EXIT_CODE.USAGE
        }
        const scanned = await scanFolder(folder)
        const { source, added, alreadyListed } = mergeSource({ id, label, scanned, sources: await readSources() })
        console.log(
            `${id} (${source.label}): ${scanned.length} charts found, ${added} added, ${alreadyListed} already in another list, ${source.charts.length} in the list`,
        )
        if (added === 0) continue
        await writeFile(
            join(BLOCKLIST_DIR, `${id}${SOURCE_FILE_EXTENSION}`),
            `${JSON.stringify(chartBlocklistSourceSchema.parse(source), null, 4)}\n`,
        )
        changedIds.push(id)
        total += added
    }
    const sources = await readSources()
    await writeFile(DATA_MODULE_PATH, await renderDataModule(sources.map((source) => source.id)))
    console.log(`blocklist: ${sources.length} lists, ${sources.reduce((sum, source) => sum + source.charts.length, 0)} charts`)
    if (changedIds.length === 0) {
        console.log('nothing new to add')
        return EXIT_CODE.OK
    }
    return command.shouldPublish ? publish({ changedIds, total }) : EXIT_CODE.OK
}

const main = async () => {
    const command = parseBlocklistCommand(process.argv.slice(2))
    if (!command) {
        console.error(USAGE)
        return EXIT_CODE.USAGE
    }
    if (command.kind === 'add') return add(command)
    const problems = await checkBlocklist()
    problems.forEach((problem) => console.error(problem))
    return problems.length === 0 ? EXIT_CODE.OK : EXIT_CODE.FAILED
}

if (import.meta.main) process.exit(await main())

import { createHash } from 'node:crypto'
import { readdir, readFile } from 'node:fs/promises'
import { basename, extname, join, relative, resolve, sep } from 'node:path'
import { z } from 'zod'
import { CHART_BLOCKLIST_BATCH_MAX, CHART_BLOCKLIST_ID_PATTERN, type ChartBlocklistEntry } from '../src/server/dto/chart-blocklist'
import { DEFAULT_SERVER, readEnvToken, readKeychainToken, safeServerOrigin } from './upload-chart-patterns'

/** The chart files the client scans (`rbms-library::is_chart_file`), compared without case. */
export const CHART_FILE_EXTENSIONS = new Set(['.bms', '.bme', '.bml', '.pms', '.bmson'])
export const BLOCKLIST_PATH = '/api/admin/chart-blocklist'

const HIDDEN_NAME_PREFIX = '.'
const HASH_BATCH_SIZE = 32
const REQUEST_TIMEOUT_MS = 60_000
const EXIT_CODE = { OK: 0, FAILED: 1, USAGE: 2 } as const
const IIDX_FOLDER_PATTERN = /iidx\s*(\d+)(st|nd|rd|th)?\b\s*(.*)$/i
const VERSION_SUBTITLE_PATTERN = /[~〜]([^~〜]+)[~〜]/
const SONG_NAME_SEPARATOR = ' - '

const USAGE = `usage:
  bun run blocklist:add <folder>... [--id <id>] [--label <label>] [--dry-run] [--server <url>] [--token-env <NAME>]
      Hashes the chart files (.bms .bme .bml .pms .bmson) of each folder and registers them on the server
      (PUT ${BLOCKLIST_PATH}, admin API token). Each chart goes to the IIDX version its folder or song folder
      names ("Beatmania IIDX20 ~tricoro~", "IIDX20 ~Tricoro~ - ALBIDA" -> iidx-20-tricoro), so a folder that
      gathers several versions is split by version; else to the folder's own name. --id and --label put
      every chart of a single folder in one list. Charts the server lists already are counted, not added again.
      --dry-run only hashes and prints the counts.
  bun run blocklist:list [--server <url>] [--token-env <NAME>]
      Prints each list on the server with its chart count.
  The token is read from the macOS keychain (as scripts/upload-chart-patterns.ts does) or from --token-env.`

type ServerOptions = { server: string; tokenEnv: string | null }
export type AddCommand = ServerOptions & { kind: 'add'; folders: string[]; id: string | null; label: string | null; isDryRun: boolean }
export type BlocklistCommand = AddCommand | (ServerOptions & { kind: 'list' })

export const parseBlocklistCommand = (argv: readonly string[]): BlocklistCommand | null => {
    const [kind, ...rest] = argv
    if (kind !== 'add' && kind !== 'list') return null
    const parsed: Omit<AddCommand, 'kind'> = { folders: [], id: null, label: null, isDryRun: false, server: DEFAULT_SERVER, tokenEnv: null }
    const queue = [...rest]
    while (queue.length > 0) {
        const arg = queue.shift() ?? ''
        if (arg === '--') continue
        if (arg === '--dry-run') parsed.isDryRun = true
        else if (arg === '--id') parsed.id = queue.shift() ?? ''
        else if (arg === '--label') parsed.label = (queue.shift() ?? '').trim()
        else if (arg === '--server') parsed.server = queue.shift() ?? ''
        else if (arg === '--token-env') parsed.tokenEnv = queue.shift() ?? ''
        else if (arg.startsWith('--')) return null
        else parsed.folders.push(arg)
    }
    const server = safeServerOrigin(parsed.server)
    if (!server || parsed.tokenEnv === '') return null
    if (kind === 'list') {
        const hasAddOptions = parsed.folders.length > 0 || parsed.id !== null || parsed.label !== null || parsed.isDryRun
        return hasAddOptions ? null : { kind, server, tokenEnv: parsed.tokenEnv }
    }
    if (parsed.folders.length === 0) return null
    if (parsed.id !== null && !CHART_BLOCKLIST_ID_PATTERN.test(parsed.id)) return null
    if (parsed.label === '') return null
    if ((parsed.id !== null || parsed.label !== null) && parsed.folders.length > 1) return null
    return { kind, ...parsed, server }
}

const slugOf = (text: string) =>
    text
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-+|-+$/g, '')

/**
 * The id and label a folder is listed under, or null when its name gives no usable id. An IIDX version folder
 * ("Beatmania IIDX18 ~Resort Anthem~") and an IIDX song folder ("IIDX20 ~Tricoro~ - ALBIDA") both name their version.
 */
export const deriveSourceName = (folderName: string) => {
    const name = folderName.normalize('NFC').trim()
    const iidx = IIDX_FOLDER_PATTERN.exec(name)
    if (iidx) {
        const [, version, ordinal, rest] = iidx
        const subtitle = VERSION_SUBTITLE_PATTERN.exec(rest)?.[1] ?? rest.split(SONG_NAME_SEPARATOR)[0]
        const title = subtitle.replace(/\s+/g, ' ').trim()
        const id = slugOf(`iidx ${version} ${title}`)
        return CHART_BLOCKLIST_ID_PATTERN.test(id) ? { id, label: `beatmania IIDX ${version}${ordinal ?? ''} ${title}`.trim() } : null
    }
    const id = slugOf(name)
    return CHART_BLOCKLIST_ID_PATTERN.test(id) ? { id, label: name } : null
}

export const isChartFileName = (name: string) => !name.startsWith(HIDDEN_NAME_PREFIX) && CHART_FILE_EXTENSIONS.has(extname(name).toLowerCase())

/**
 * The list a chart under the folder `rootName` goes to: the first IIDX version named by the folder or by a folder on the chart's path
 * (so a folder that gathers songs of several versions is split by version), else the folder itself.
 */
export const sourceOfChart = (rootName: string, path: string) => {
    const folders = [rootName, ...path.split('/').slice(0, -1)]
    return deriveSourceName(folders.find((folder) => IIDX_FOLDER_PATTERN.test(folder)) ?? rootName)
}

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

const chunksOf = <T>(items: readonly T[], size: number) =>
    Array.from({ length: Math.ceil(items.length / size) }, (_, index) => items.slice(index * size, (index + 1) * size))

/** One entry per distinct chart file of the folder (the first path of identical files), sorted by path. */
export const scanFolder = async (folder: string) => {
    const root = resolve(folder)
    const hashed: ChartBlocklistEntry[][] = []
    for (const batch of chunksOf(await findChartFiles(root), HASH_BATCH_SIZE)) {
        hashed.push(await Promise.all(batch.map((path) => hashChartFile(root, path))))
    }
    return [...Map.groupBy(hashed.flat().toSorted(byPath), (entry) => entry.sha256).values()].map(([first]) => first)
}

const registeredSchema = z.object({ data: z.object({ received: z.number(), added: z.number(), already_listed: z.number() }) })
const listedSchema = z.object({ data: z.array(z.object({ id: z.string(), label: z.string(), charts: z.number() })) })
const errorSchema = z.object({ error: z.object({ code: z.string() }) })

export type Fetch = (input: string, init: RequestInit) => Promise<Response>

const errorCodeOf = (text: string) => {
    try {
        const parsed = errorSchema.safeParse(JSON.parse(text))
        return parsed.success ? ` ${parsed.data.error.code}` : ''
    } catch {
        return ''
    }
}

const failureOf = async (response: Response) => `server answered ${response.status}${errorCodeOf(await response.text())}`

/** Sends the charts of one source in batches of at most `CHART_BLOCKLIST_BATCH_MAX` and answers the summed counts. */
export const registerCharts = async (params: {
    fetch: Fetch
    server: string
    token: string
    source: { id: string; label: string }
    charts: readonly ChartBlocklistEntry[]
}) => {
    const { fetch, server, token, source, charts } = params
    const answers: z.infer<typeof registeredSchema>['data'][] = []
    for (const batch of chunksOf(charts, CHART_BLOCKLIST_BATCH_MAX)) {
        const response = await fetch(`${server}${BLOCKLIST_PATH}`, {
            method: 'PUT',
            headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
            body: JSON.stringify({ source, charts: batch }),
            signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
        })
        if (!response.ok) throw new Error(await failureOf(response))
        answers.push(registeredSchema.parse(await response.json()).data)
    }
    return {
        received: answers.reduce((sum, answer) => sum + answer.received, 0),
        added: answers.reduce((sum, answer) => sum + answer.added, 0),
        alreadyListed: answers.reduce((sum, answer) => sum + answer.already_listed, 0),
    }
}

export const listSources = async (params: { fetch: Fetch; server: string; token: string }) => {
    const response = await params.fetch(`${params.server}${BLOCKLIST_PATH}`, {
        method: 'GET',
        headers: { authorization: `Bearer ${params.token}` },
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    })
    if (!response.ok) throw new Error(await failureOf(response))
    return listedSchema.parse(await response.json()).data
}

const readToken = (tokenEnv: string | null) => (tokenEnv ? readEnvToken(tokenEnv) : readKeychainToken())

const printSources = (sources: Awaited<ReturnType<typeof listSources>>) => {
    sources.forEach((source) => console.log(`  ${source.id} (${source.label}): ${source.charts}`))
    console.log(`blocklist on the server: ${sources.length} lists, ${sources.reduce((sum, source) => sum + source.charts, 0)} charts`)
}

const planSources = async (command: AddCommand) => {
    const planned: { folder: string; id: string; label: string; charts: ChartBlocklistEntry[] }[] = []
    for (const folder of command.folders) {
        const rootName = basename(resolve(folder))
        const rootSource = deriveSourceName(rootName)
        const isOverridden = command.id !== null || command.label !== null
        const sourced = (await scanFolder(folder)).map((chart) => ({
            chart,
            source: isOverridden
                ? { id: command.id ?? rootSource?.id ?? '', label: command.label ?? rootSource?.label ?? '' }
                : sourceOfChart(rootName, chart.path),
        }))
        if (sourced.some(({ source }) => !source?.id || !source.label)) return { unnamedFolder: folder, planned }
        Map.groupBy(sourced, ({ source }) => source?.id ?? '').forEach((group, id) =>
            planned.push({ folder, id, label: group[0].source?.label ?? id, charts: group.map(({ chart }) => chart) }),
        )
    }
    return { unnamedFolder: null, planned }
}

const add = async (command: AddCommand) => {
    const { unnamedFolder, planned } = await planSources(command)
    if (unnamedFolder) {
        console.error(`${unnamedFolder}: no id could be made from the folder names, give --id and --label`)
        return EXIT_CODE.USAGE
    }
    if (planned.length === 0) console.log('no chart files found')
    const token = command.isDryRun || planned.length === 0 ? '' : await readToken(command.tokenEnv)
    for (const { folder, id, label, charts } of planned) {
        if (command.isDryRun) {
            console.log(`${id} (${label}): ${charts.length} charts found in ${folder}, nothing sent`)
            continue
        }
        const totals = await registerCharts({ fetch, server: command.server, token, source: { id, label }, charts })
        console.log(`${id} (${label}): ${charts.length} charts found in ${folder}, ${totals.added} added, ${totals.alreadyListed} already listed`)
    }
    if (command.isDryRun || planned.length === 0) return EXIT_CODE.OK
    printSources(await listSources({ fetch, server: command.server, token }))
    console.log(
        'the server refuses these charts from now on. Rows left from before: gh workflow run web-db.yml --ref dev -f action=chart-blocklist-purge-count',
    )
    return EXIT_CODE.OK
}

const main = async () => {
    const command = parseBlocklistCommand(process.argv.slice(2))
    if (!command) {
        console.error(USAGE)
        return EXIT_CODE.USAGE
    }
    try {
        if (command.kind === 'add') return await add(command)
        printSources(await listSources({ fetch, server: command.server, token: await readToken(command.tokenEnv) }))
        return EXIT_CODE.OK
    } catch (error: unknown) {
        console.error(error instanceof Error ? error.message : String(error))
        return EXIT_CODE.FAILED
    }
}

if (import.meta.main) process.exit(await main())

import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { z } from 'zod'

const REQUEST_TIMEOUT_MS = 30_000
const MAX_RESPONSE_BYTES = 64 * 1024 * 1024
const MAX_DATA_URLS = 16
const HEADER_FILE = 'header.json'
const DATA_FILE = 'data.json'
const SOURCE_FILE = 'source.json'
const SOURCE_SCHEMA_VERSION = 1
const HEADER_JSON_INDENT = 2
const MD5_PATTERN = /^[0-9a-f]{32}$/
const META_TAG_PATTERN = /<meta\b[^>]*>/gi
const HTML_COMMENT_PATTERN = /<!--[\s\S]*?(?:-->|$)/g
const ATTRIBUTE_PATTERN = /([^\s=/>]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/g
const BMSTABLE_META_NAME = 'bmstable'
const EXIT_CODE = { SAME: 0, DIFFERENT: 1, FAILED: 2 } as const

type Fetched = {
    requestedUrl: string
    finalUrl: string
    status: number
    contentType: string | null
    lastModified: string | null
    etag: string | null
    bytes: Uint8Array
    sha256: string
}

type Body = { fetched: Fetched; entries: unknown[] }

type Snapshot = {
    headerText: string
    dataText: string
    pageUrl: string
    page: Fetched
    metaContent: string
    header: Fetched
    bodies: Body[]
    entryCount: number
    levelCount: number
}

const objectSchema = z.record(z.string(), z.unknown())

const sourceSchema = z.object({
    schema: z.literal(SOURCE_SCHEMA_VERSION),
    snapshot: z.object({ header_file_sha256: z.string(), data_file_sha256: z.string(), entries: z.number() }),
})

const sha256Of = (bytes: Uint8Array | string) => createHash('sha256').update(bytes).digest('hex')

const stripBom = (bytes: Uint8Array) => (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? bytes.subarray(3) : bytes)

const decodeUtf8 = (bytes: Uint8Array) => new TextDecoder('utf-8', { fatal: true }).decode(stripBom(bytes))

const parseJson = (bytes: Uint8Array) => JSON.parse(decodeUtf8(bytes)) as unknown

const get = async (url: string): Promise<Fetched> => {
    const response = await fetch(url, { redirect: 'follow', signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) })
    if (!response.ok) throw new Error(`${url} answered ${response.status}`)
    const bytes = new Uint8Array(await response.arrayBuffer())
    if (bytes.length > MAX_RESPONSE_BYTES) throw new Error(`${url} is larger than ${MAX_RESPONSE_BYTES} bytes`)
    return {
        requestedUrl: url,
        finalUrl: response.url,
        status: response.status,
        contentType: response.headers.get('content-type'),
        lastModified: response.headers.get('last-modified'),
        etag: response.headers.get('etag'),
        bytes,
        sha256: sha256Of(bytes),
    }
}

const bmstableAddressOf = (html: string) => {
    const visible = html.replace(HTML_COMMENT_PATTERN, '')
    for (const tag of visible.match(META_TAG_PATTERN) ?? []) {
        const attributes = new Map<string, string>()
        for (const match of tag.matchAll(ATTRIBUTE_PATTERN)) {
            attributes.set(match[1].toLowerCase(), (match[2] ?? match[3] ?? match[4] ?? '').trim())
        }
        const content = attributes.get('content')?.replaceAll('&amp;', '&')
        if (attributes.get('name')?.toLowerCase() === BMSTABLE_META_NAME && content) return content
    }
    return null
}

const dataUrlsOf = (header: Record<string, unknown>) => {
    const named = header.data_url
    const urls = Array.isArray(named) ? named : [named]
    const texts = urls.filter((url): url is string => typeof url === 'string' && url.length > 0)
    if (texts.length === 0) throw new Error('the header names no data_url')
    if (texts.length > MAX_DATA_URLS) throw new Error(`the header names more than ${MAX_DATA_URLS} data_url values`)
    return texts
}

const SHAPE_BY_FIRST_CHARACTER: Partial<Record<string, 'header' | 'body'>> = { '{': 'header', '[': 'body' }

const pageHeaderOrBody = (bytes: Uint8Array) => SHAPE_BY_FIRST_CHARACTER[decodeUtf8(bytes).trimStart()[0] ?? ''] ?? 'page'

const levelsOf = (entries: unknown[]) => new Set(entries.map((entry) => objectSchema.safeParse(entry).data?.level)).size

const normalizeHeader = (header: Record<string, unknown>) => `${JSON.stringify(header, null, HEADER_JSON_INDENT)}\n`

const normalizeData = (entries: unknown[]) => (entries.length === 0 ? '[]\n' : `[\n${entries.map((entry) => JSON.stringify(entry)).join(',\n')}\n]\n`)

const collect = async (tableUrl: string): Promise<Snapshot> => {
    const first = await get(tableUrl)
    const shape = pageHeaderOrBody(first.bytes)
    if (shape === 'body') throw new Error('the address is a data body; give the table page or its header.json')
    const metaContent = shape === 'page' ? bmstableAddressOf(decodeUtf8(first.bytes)) : null
    if (shape === 'page' && !metaContent) throw new Error('the page has no bmstable meta tag')
    const header = shape === 'header' ? first : await get(new URL(metaContent ?? '', first.finalUrl).href)
    const headerObject = objectSchema.parse(parseJson(header.bytes))
    const bodies: Body[] = []
    for (const dataUrl of dataUrlsOf(headerObject)) {
        const fetched = await get(new URL(dataUrl, header.finalUrl).href)
        const entries = z.array(z.unknown()).parse(parseJson(fetched.bytes))
        bodies.push({ fetched, entries })
    }
    const entries = bodies.flatMap((body) => body.entries)
    return {
        headerText: normalizeHeader(headerObject),
        dataText: normalizeData(entries),
        pageUrl: tableUrl,
        page: first,
        metaContent: metaContent ?? '',
        header,
        bodies,
        entryCount: entries.length,
        levelCount: levelsOf(entries),
    }
}

const httpOf = (fetched: Fetched) => ({
    requested_url: fetched.requestedUrl,
    final_url: fetched.finalUrl,
    status: fetched.status,
    content_type: fetched.contentType,
    last_modified: fetched.lastModified,
    etag: fetched.etag,
    bytes: fetched.bytes.length,
})

const sourceJsonOf = (snapshot: Snapshot, fetchedAt: string, tableUrl: string, outDir: string) => {
    const md5Count = snapshot.bodies
        .flatMap((body) => body.entries)
        .filter((entry) => MD5_PATTERN.test(String(objectSchema.safeParse(entry).data?.md5))).length
    return `${JSON.stringify(
        {
            schema: SOURCE_SCHEMA_VERSION,
            origin: {
                page_url: snapshot.pageUrl,
                bmstable_meta_content: snapshot.metaContent,
                header_url: snapshot.header.finalUrl,
                data_urls: snapshot.bodies.map((body) => body.fetched.finalUrl),
            },
            fetched_at: fetchedAt,
            http: {
                page: { ...httpOf(snapshot.page), sha256_volatile: sha256Of(snapshot.page.bytes) },
                header: { ...httpOf(snapshot.header), sha256: snapshot.header.sha256 },
                data: snapshot.bodies.map((body) => ({ ...httpOf(body.fetched), sha256: body.fetched.sha256, entries: body.entries.length })),
            },
            snapshot: {
                header_file_sha256: sha256Of(snapshot.headerText),
                data_file_sha256: sha256Of(snapshot.dataText),
                entries: snapshot.entryCount,
                levels: snapshot.levelCount,
                entries_with_valid_md5: md5Count,
            },
            method: {
                command: `bun scripts/snapshot-table.ts ${tableUrl} ${outDir}`,
                normalization: [
                    'UTF-8 without BOM, LF line endings, a single trailing newline',
                    'header.json: JSON.stringify with 2-space indent, key order kept',
                    'data.json: one entry per line, entry order and key order kept, no value changed',
                    'the page html is not stored: Cloudflare injects a per-request script, so its hash changes on every request',
                ],
            },
        },
        null,
        HEADER_JSON_INDENT,
    )}\n`
}

const readIfExists = async (path: string) => readFile(path, 'utf-8').catch(() => null)

const write = async (tableUrl: string, outDir: string) => {
    const snapshot = await collect(tableUrl)
    await mkdir(outDir, { recursive: true })
    await writeFile(join(outDir, HEADER_FILE), snapshot.headerText)
    await writeFile(join(outDir, DATA_FILE), snapshot.dataText)
    await writeFile(join(outDir, SOURCE_FILE), sourceJsonOf(snapshot, new Date().toISOString(), tableUrl, outDir))
    console.log(`wrote ${snapshot.entryCount} entries across ${snapshot.levelCount} levels to ${outDir}`)
    return EXIT_CODE.SAME
}

const check = async (tableUrl: string, outDir: string) => {
    const storedHeader = await readIfExists(join(outDir, HEADER_FILE))
    const storedData = await readIfExists(join(outDir, DATA_FILE))
    const storedSource = await readIfExists(join(outDir, SOURCE_FILE))
    if (storedHeader === null || storedData === null || storedSource === null) {
        console.error('the snapshot folder is missing a file')
        return EXIT_CODE.FAILED
    }
    const recorded = sourceSchema.parse(JSON.parse(storedSource))
    const localProblems = [
        sha256Of(storedHeader) === recorded.snapshot.header_file_sha256 ? null : 'header.json differs from the hash in source.json',
        sha256Of(storedData) === recorded.snapshot.data_file_sha256 ? null : 'data.json differs from the hash in source.json',
    ].filter((problem) => problem !== null)
    const remote = await collect(tableUrl)
    const problems = [
        ...localProblems,
        remote.headerText === storedHeader ? null : 'the header differs from the origin',
        remote.dataText === storedData
            ? null
            : `the data differs from the origin (stored ${recorded.snapshot.entries} entries, origin ${remote.entryCount})`,
    ].filter((problem) => problem !== null)
    if (problems.length === 0) {
        console.log(`same: ${remote.entryCount} entries, header and data match the origin after normalization`)
        return EXIT_CODE.SAME
    }
    problems.forEach((problem) => console.error(problem))
    return EXIT_CODE.DIFFERENT
}

const main = async () => {
    const args = process.argv.slice(2)
    const shouldCheck = args.includes('--check')
    const [tableUrl, outDir] = args.filter((arg) => !arg.startsWith('--'))
    if (!tableUrl || !outDir) {
        console.error('usage: bun scripts/snapshot-table.ts <table-url> <output-dir> [--check]')
        return EXIT_CODE.FAILED
    }
    return shouldCheck ? check(tableUrl, outDir) : write(tableUrl, outDir)
}

process.exitCode = await main().catch((error: unknown) => {
    console.error(error instanceof Error ? error.message : String(error))
    return EXIT_CODE.FAILED
})

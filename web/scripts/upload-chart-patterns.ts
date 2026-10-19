import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { z } from 'zod'
import { CHART_PATTERN_WRITE_RATE_LIMIT, chartPatternDocumentSchema } from '../src/server/dto/chart-pattern'
import { parseRetryAfterSeconds } from '../src/server/service/shared/ai-provider/http'

export const DEFAULT_SERVER = 'https://bms.hyuns.uk'
export const UPLOAD_CONCURRENCY = 4
export const KEYCHAIN_ACCOUNT = 'rbms-admin'
export const KEYCHAIN_SERVICE = 'rbms-pattern-upload'
export const SKIPPED_FILE_PREFIX = '_'

/** The share of the server's per-minute write limit a run may use, so a clock skew or a retry never pushes a window over it. */
export const UPLOAD_RATE_SHARE = 0.9
/** The least time between two request starts of the whole run (all workers together): about 56 ms, 1,071 requests a minute against 1,200. */
export const UPLOAD_REQUEST_INTERVAL_MS = Math.ceil(
    CHART_PATTERN_WRITE_RATE_LIMIT.windowMs / (CHART_PATTERN_WRITE_RATE_LIMIT.limit * UPLOAD_RATE_SHARE),
)
/** The wait after a 429 with no `Retry-After`: the server's whole limit window, after which its counter is certainly new. */
export const RATE_LIMIT_FALLBACK_PAUSE_MS = CHART_PATTERN_WRITE_RATE_LIMIT.windowMs
/** The longest one wait after a 429 lasts, whatever `Retry-After` asks for. */
export const RATE_LIMIT_MAX_PAUSE_MS = 300_000
/** After this many waits in a row with no request getting through, the run stops and reports the file as failed. */
export const MAX_CONSECUTIVE_RATE_LIMIT_PAUSES = 5

const PATTERN_FILE = /^([0-9a-f]{32})\.json$/
const UPLOAD_PATH = '/api/admin/chart-patterns'
const STATUS = { OK: 200, CREATED: 201, UNAUTHORIZED: 401, FORBIDDEN: 403, TOO_MANY_REQUESTS: 429 } as const
const MILLIS_PER_SECOND = 1000
const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]'])
const EXIT_CODE = { OK: 0, FAILED: 1, USAGE: 2 } as const

const USAGE = `bun scripts/upload-chart-patterns.ts <패턴 폴더> [--server <주소>] [--token-env <환경변수 이름>] [--dry-run]
  토큰: 기본은 macOS 키체인(계정 ${KEYCHAIN_ACCOUNT}, 서비스 ${KEYCHAIN_SERVICE}), --token-env 를 주면 그 환경변수
  --dry-run: 서버에 보내지 않고 파일만 검사한다`

export type UploadArgs = { dir: string; server: string; tokenEnv: string | null; dryRun: boolean }

/** Reads the command line. A server must be https, or http on this machine, so the admin token never crosses the network in clear. */
export const parseUploadArgs = (argv: readonly string[]) => {
    const rest = [...argv]
    const args: UploadArgs = { dir: '', server: DEFAULT_SERVER, tokenEnv: null, dryRun: false }
    while (rest.length > 0) {
        const arg = rest.shift() ?? ''
        if (arg === '--server') args.server = rest.shift() ?? ''
        else if (arg === '--token-env') args.tokenEnv = rest.shift() ?? ''
        else if (arg === '--dry-run') args.dryRun = true
        else if (arg.startsWith('--')) throw new Error(`알 수 없는 옵션: ${arg}`)
        else if (!args.dir) args.dir = arg
        else throw new Error(`인자가 너무 많습니다: ${arg}`)
    }
    if (!args.dir) throw new Error('패턴 폴더를 주세요')
    if (args.tokenEnv === '') throw new Error('--token-env 에 환경변수 이름을 주세요')
    const url = URL.parse(args.server)
    if (!url || !(url.protocol === 'https:' || (url.protocol === 'http:' && LOCAL_HOSTS.has(url.hostname)))) {
        throw new Error('--server 는 https 주소(또는 이 컴퓨터의 http 주소)여야 합니다')
    }
    return { ...args, server: url.origin }
}

export type PatternFile = { md5: string; path: string }

/** The `<md5>.json` files of the folder in name order. Names starting with `_` (the CLI's `_coverage.json`) and any other name are skipped. */
export const listPatternFiles = async (dir: string) => {
    const names = (await readdir(dir)).toSorted()
    const files = names.flatMap((name) => {
        const match = PATTERN_FILE.exec(name)
        return !name.startsWith(SKIPPED_FILE_PREFIX) && match ? [{ md5: match[1], path: join(dir, name) }] : []
    })
    return { files, skipped: names.length - files.length }
}

export type Spawn = (command: string[]) => { exited: Promise<number>; stdout: ReadableStream<Uint8Array> }

const spawnQuietly: Spawn = (command) => Bun.spawn(command, { stdin: 'ignore', stdout: 'pipe', stderr: 'ignore' })

/** The admin token from the macOS keychain, read while the script runs. It is never printed, logged or written. */
export const readKeychainToken = async (spawn: Spawn = spawnQuietly) => {
    const child = spawn(['security', 'find-generic-password', '-a', KEYCHAIN_ACCOUNT, '-s', KEYCHAIN_SERVICE, '-w'])
    const [exitCode, text] = await Promise.all([child.exited, new Response(child.stdout).text()])
    const token = text.trim()
    if (exitCode !== 0 || !token)
        throw new Error(`키체인에서 토큰을 읽지 못했습니다(계정 ${KEYCHAIN_ACCOUNT}, 서비스 ${KEYCHAIN_SERVICE}, 종료 코드 ${exitCode})`)
    return token
}

export const readEnvToken = (name: string, env: Record<string, string | undefined> = process.env) => {
    const token = env[name]?.trim()
    if (!token) throw new Error(`환경변수 ${name} 에 토큰이 없습니다`)
    return token
}

const responseSchema = z.object({ data: z.object({ outcome: z.enum(['created', 'replaced', 'unchanged']) }) })
const errorSchema = z.object({
    error: z.object({ code: z.string(), details: z.object({ field: z.string().optional(), reason: z.string().optional() }).optional() }),
})

export type UploadFailure = { md5: string; status: number | null; reason: string }

export type UploadSummary = {
    files: number
    skipped: number
    created: number
    replaced: number
    unchanged: number
    checked: number
    failures: UploadFailure[]
    stoppedByAuth: boolean
    stoppedByRateLimit: boolean
    rateLimitPauses: number
}

const STOP_CAUSE = { AUTH: 'auth', RATE_LIMIT: 'rate_limit' } as const
type StopCause = (typeof STOP_CAUSE)[keyof typeof STOP_CAUSE]

const STOP_BY_STATUS: Partial<Record<number, StopCause>> = {
    [STATUS.UNAUTHORIZED]: STOP_CAUSE.AUTH,
    [STATUS.FORBIDDEN]: STOP_CAUSE.AUTH,
    [STATUS.TOO_MANY_REQUESTS]: STOP_CAUSE.RATE_LIMIT,
}

type FileResult = { kind: 'created' | 'replaced' | 'unchanged' | 'checked' } | { kind: 'failed'; failure: UploadFailure; stop: StopCause | null }

const failed = (md5: string, status: number | null, reason: string, stop: StopCause | null = null) =>
    ({ kind: 'failed', failure: { md5, status, reason }, stop }) satisfies FileResult

const checkLocally = (file: PatternFile, text: string) => {
    let body: unknown
    try {
        body = JSON.parse(text)
    } catch {
        return 'JSON 이 아닙니다'
    }
    const parsed = chartPatternDocumentSchema.safeParse(body)
    if (!parsed.success) {
        const [issue] = parsed.error.issues
        return `형식 오류: ${issue?.path.join('.') ?? ''} ${issue?.message ?? ''}`.trim()
    }
    return parsed.data.md5 === file.md5 ? null : '파일 이름의 md5 와 본문의 md5 가 다릅니다'
}

const describeError = async (response: Response) => {
    const parsed = errorSchema.safeParse(await response.json().catch(() => null))
    if (!parsed.success) return `HTTP ${response.status}`
    const { code, details } = parsed.data.error
    return [code, details?.field, details?.reason].filter(Boolean).join(' ')
}

export type UploadClock = { now: () => number; sleep: (ms: number) => Promise<void> }

const realClock: UploadClock = { now: () => Date.now(), sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)) }

export type UploadOptions = {
    files: readonly PatternFile[]
    skipped: number
    server: string
    token: string | null
    dryRun: boolean
    fetch: (url: string, init: RequestInit) => Promise<Response>
    readText?: (path: string) => Promise<string>
    concurrency?: number
    clock?: UploadClock
}

/**
 * Keeps the whole run (every worker) under the server's write limit. Each request start reserves the next slot UPLOAD_REQUEST_INTERVAL_MS
 * after the previous one. A 429 pauses every worker until `Retry-After` (RATE_LIMIT_FALLBACK_PAUSE_MS without it, at most
 * RATE_LIMIT_MAX_PAUSE_MS); the workers whose requests were already in flight when the pause began join it and do not count as a new one.
 */
const createRequestGate = (clock: UploadClock) => {
    let nextSlotAt = 0
    let pausedUntil = 0
    let consecutivePauses = 0
    let totalPauses = 0

    const sleepUntil = async (at: number) => {
        const wait = at - clock.now()
        if (wait > 0) await clock.sleep(wait)
    }

    return {
        beforeRequest: async () => {
            for (;;) {
                await sleepUntil(pausedUntil)
                const slot = Math.max(clock.now(), nextSlotAt)
                nextSlotAt = slot + UPLOAD_REQUEST_INTERVAL_MS
                await sleepUntil(slot)
                if (pausedUntil <= clock.now()) return
            }
        },
        gotThrough: () => {
            consecutivePauses = 0
        },
        rateLimited: (sentAt: number, retryAfterSeconds: number | null) => {
            if (pausedUntil > sentAt) return true
            if (consecutivePauses >= MAX_CONSECUTIVE_RATE_LIMIT_PAUSES) return false
            consecutivePauses += 1
            totalPauses += 1
            const askedMs = retryAfterSeconds === null ? RATE_LIMIT_FALLBACK_PAUSE_MS : retryAfterSeconds * MILLIS_PER_SECOND
            pausedUntil = clock.now() + Math.min(askedMs, RATE_LIMIT_MAX_PAUSE_MS)
            return true
        },
        pauses: () => totalPauses,
    }
}

/**
 * Checks every file with the server's own schema and, unless it is a dry run, PUTs it to the admin route, UPLOAD_CONCURRENCY at a
 * time and never faster than UPLOAD_REQUEST_INTERVAL_MS between two request starts. A 200 (`unchanged` or `replaced`) and a 201
 * (`created`) both count as uploaded, so running it again after a failure only redoes what is missing. A 401 or 403 stops the
 * remaining files, since every one of them would fail the same way. A 429 pauses every worker and sends the same file again (see
 * `createRequestGate`); MAX_CONSECUTIVE_RATE_LIMIT_PAUSES waits in a row with no request getting through fail the file and stop the run.
 */
export const uploadChartPatterns = async (options: UploadOptions) => {
    const readText = options.readText ?? ((path: string) => readFile(path, 'utf8'))
    const clock = options.clock ?? realClock
    const gate = createRequestGate(clock)
    const queue = [...options.files]
    const results: FileResult[] = []
    const hasStopped = (stop: StopCause) => results.some((result) => result.kind === 'failed' && result.stop === stop)
    const isStopped = () => hasStopped(STOP_CAUSE.AUTH) || hasStopped(STOP_CAUSE.RATE_LIMIT)

    const send = async (file: PatternFile, text: string) => {
        for (;;) {
            await gate.beforeRequest()
            const sentAt = clock.now()
            const response = await options
                .fetch(`${options.server}${UPLOAD_PATH}/${file.md5}`, {
                    method: 'PUT',
                    headers: { authorization: `Bearer ${options.token}`, 'content-type': 'application/json' },
                    body: text,
                })
                .catch(() => null)
            if (response?.status !== STATUS.TOO_MANY_REQUESTS) {
                gate.gotThrough()
                return response
            }
            if (!gate.rateLimited(sentAt, parseRetryAfterSeconds(response.headers.get('retry-after'), clock.now()))) return response
        }
    }

    const uploadOne = async (file: PatternFile) => {
        const text = await readText(file.path)
        const localProblem = checkLocally(file, text)
        if (localProblem) return failed(file.md5, null, localProblem)
        if (options.dryRun || !options.token) return { kind: 'checked' } as const
        const response = await send(file, text)
        if (!response) return failed(file.md5, null, '서버에 연결하지 못했습니다')
        if (response.status === STATUS.OK || response.status === STATUS.CREATED) {
            const parsed = responseSchema.safeParse(await response.json().catch(() => null))
            return parsed.success ? { kind: parsed.data.data.outcome } : failed(file.md5, response.status, '응답 형식이 다릅니다')
        }
        return failed(file.md5, response.status, await describeError(response), STOP_BY_STATUS[response.status] ?? null)
    }

    const worker = async () => {
        while (!isStopped()) {
            const file = queue.shift()
            if (!file) return
            results.push(await uploadOne(file))
        }
    }

    await Promise.all(Array.from({ length: Math.max(1, options.concurrency ?? UPLOAD_CONCURRENCY) }, worker))

    const countOf = (kind: FileResult['kind']) => results.filter((result) => result.kind === kind).length
    return {
        files: options.files.length,
        skipped: options.skipped,
        created: countOf('created'),
        replaced: countOf('replaced'),
        unchanged: countOf('unchanged'),
        checked: countOf('checked'),
        failures: results.flatMap((result) => (result.kind === 'failed' ? [result.failure] : [])).toSorted((a, b) => a.md5.localeCompare(b.md5)),
        stoppedByAuth: hasStopped(STOP_CAUSE.AUTH),
        stoppedByRateLimit: hasStopped(STOP_CAUSE.RATE_LIMIT),
        rateLimitPauses: gate.pauses(),
    } satisfies UploadSummary
}

/** The printed summary: counts first, then each failure on its own line. The token is not part of it. */
export const formatSummary = (summary: UploadSummary, dryRun: boolean) =>
    [
        `파일 ${summary.files} (건너뜀 ${summary.skipped})`,
        dryRun ? `검사 통과 ${summary.checked}` : `새로 올림 ${summary.created}, 바꿈 ${summary.replaced}, 이미 같음 ${summary.unchanged}`,
        ...(summary.rateLimitPauses > 0 ? [`속도 제한(429)으로 ${summary.rateLimitPauses}번 기다림`] : []),
        `실패 ${summary.failures.length}${summary.stoppedByAuth ? ' (인증 실패로 나머지를 멈춤)' : ''}${summary.stoppedByRateLimit ? ' (속도 제한이 이어져 나머지를 멈춤)' : ''}`,
        ...summary.failures.map((failure) => `  ${failure.md5} ${failure.status ?? '-'} ${failure.reason}`),
    ].join('\n')

const main = async () => {
    const parsedArgs = (() => {
        try {
            return parseUploadArgs(process.argv.slice(2))
        } catch (error) {
            console.error(error instanceof Error ? error.message : String(error))
            console.error(USAGE)
            return null
        }
    })()
    if (!parsedArgs) return EXIT_CODE.USAGE
    const { files, skipped } = await listPatternFiles(parsedArgs.dir)
    const token = parsedArgs.dryRun ? null : parsedArgs.tokenEnv ? readEnvToken(parsedArgs.tokenEnv) : await readKeychainToken()
    console.log(`${parsedArgs.dryRun ? '검사' : '업로드'}: ${files.length}개 → ${parsedArgs.dryRun ? '(보내지 않음)' : parsedArgs.server}`)
    const summary = await uploadChartPatterns({ files, skipped, server: parsedArgs.server, token, dryRun: parsedArgs.dryRun, fetch })
    console.log(formatSummary(summary, parsedArgs.dryRun))
    return summary.failures.length === 0 ? EXIT_CODE.OK : EXIT_CODE.FAILED
}

if (import.meta.main) {
    process.exitCode = await main().catch((error: unknown) => {
        console.error(error instanceof Error ? error.message : '업로드를 끝내지 못했습니다')
        return EXIT_CODE.FAILED
    })
}

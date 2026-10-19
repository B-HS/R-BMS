import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createAdminChartPatternRoutes } from '@server/route/admin'
import { createChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import {
    DEFAULT_SERVER,
    KEYCHAIN_ACCOUNT,
    KEYCHAIN_SERVICE,
    formatSummary,
    listPatternFiles,
    parseUploadArgs,
    readEnvToken,
    readKeychainToken,
    uploadChartPatterns,
    type Spawn,
} from '../../scripts/upload-chart-patterns'
import { createMemoryChartPatternDb, makePatternDocument, type FixtureNote } from '../support/chart-pattern-fixture'

const ADMIN_TOKEN = 'rbms_script_admin_secret_Q7'
const PLAYER_TOKEN = 'rbms_script_player_secret_P3'
const USERS = new Map<string, AuthUser>([
    [ADMIN_TOKEN, { id: 'user-admin', loginId: 'admin', name: 'Admin', role: 'admin' }],
    [PLAYER_TOKEN, { id: 'user-player', loginId: 'player', name: 'Player', role: 'user' }],
])
const SERVER = 'http://127.0.0.1:3999'
const MD5S = ['1'.repeat(32), '2'.repeat(32), '3'.repeat(32)]
const BROKEN_MD5 = '4'.repeat(32)
const NOTES: FixtureNote[] = Array.from({ length: 32 }, (_, index): FixtureNote => [index * 100_000, index % 8, 0])
const PUT_PATH = /^\/api\/admin\/chart-patterns\/([^/]+)$/

let dir = ''

beforeAll(async () => {
    dir = await mkdtemp(join(tmpdir(), 'rbms-pattern-upload-'))
    await Promise.all(MD5S.map((md5) => writeFile(join(dir, `${md5}.json`), JSON.stringify(makePatternDocument(NOTES, { md5 })))))
    await writeFile(join(dir, `${BROKEN_MD5}.json`), JSON.stringify({ ...makePatternDocument(NOTES, { md5: BROKEN_MD5 }), version: 9 }))
    await writeFile(join(dir, '_coverage.json'), '{"format":"rbms-chart-pattern-coverage"}')
    await writeFile(join(dir, 'notes.txt'), 'not a pattern')
})

afterAll(async () => {
    await rm(dir, { recursive: true, force: true })
})

beforeEach(() => resetRateLimits())

/** A `fetch` that hands each request to the real route handlers over an in-memory table, with no server and no database. */
const createServer = () => {
    const memory = createMemoryChartPatternDb()
    const routes = createAdminChartPatternRoutes(
        () => ({
            authService: { resolveApiTokenUser: async (plain) => USERS.get(plain) ?? null },
            chartPatternService: createChartPatternService({ db: memory.db, now: () => 0, listCoverageTables: () => [] }),
        }),
        async () => undefined,
    )
    const seen: { url: string; authorization: string | null }[] = []
    const fakeFetch = async (url: string, init: RequestInit) => {
        const request = new Request(url, init)
        seen.push({ url: request.url, authorization: request.headers.get('authorization') })
        const match = PUT_PATH.exec(new URL(request.url).pathname)
        if (!match || request.method !== 'PUT') return new Response(null, { status: 404 })
        return routes.put(request, { params: Promise.resolve({ md5: match[1] }) })
    }
    return { memory, fakeFetch, seen }
}

type FakeFetch = (url: string, init: RequestInit) => Promise<Response>

const run = async (fakeFetch: FakeFetch, token: string | null, dryRun = false) => {
    const { files, skipped } = await listPatternFiles(dir)
    return uploadChartPatterns({ files, skipped, server: SERVER, token, dryRun, fetch: fakeFetch })
}

describe('업로드 스크립트 — 폴더 읽기', () => {
    test('<md5>.json 만 이름 순서로 고르고 _ 로 시작하는 파일과 다른 파일은 건너뛴다', async () => {
        const { files, skipped } = await listPatternFiles(dir)
        expect(files.map((file) => file.md5)).toEqual([...MD5S, BROKEN_MD5])
        expect(skipped).toBe(2)
    })
})

describe('업로드 스크립트 — 서비스 단위(서버·DB 없이 실제 라우트로)', () => {
    test('처음은 새로 올리고, 다시 돌리면 200 으로 이미 같음을 세며, 형식이 틀린 파일은 보내지 않고 실패로 남긴다', async () => {
        const { memory, fakeFetch, seen } = createServer()
        const first = await run(fakeFetch, ADMIN_TOKEN)
        expect(first).toMatchObject({ files: 4, skipped: 2, created: 3, replaced: 0, unchanged: 0, stoppedByAuth: false })
        expect(first.failures).toEqual([{ md5: BROKEN_MD5, status: null, reason: expect.stringContaining('형식 오류: version') }])
        expect(memory.rows.size).toBe(3)
        expect(seen.map((entry) => entry.url).toSorted()).toEqual(MD5S.map((md5) => `${SERVER}/api/admin/chart-patterns/${md5}`))
        expect(seen.every((entry) => entry.authorization === `Bearer ${ADMIN_TOKEN}`)).toBe(true)

        const second = await run(fakeFetch, ADMIN_TOKEN)
        expect(second).toMatchObject({ created: 0, replaced: 0, unchanged: 3 })
        expect(memory.calls).toMatchObject({ insert: 3, replace: 0 })
    })

    test('관리자가 아닌 토큰이면 403 에서 나머지를 멈추고, 요약에 토큰이 나오지 않는다', async () => {
        const { memory, fakeFetch, seen } = createServer()
        const files = (await listPatternFiles(dir)).files.filter((file) => file.md5 !== BROKEN_MD5)
        const summary = await uploadChartPatterns({
            files,
            skipped: 0,
            server: SERVER,
            token: PLAYER_TOKEN,
            dryRun: false,
            fetch: fakeFetch,
            concurrency: 1,
        })
        expect(summary.stoppedByAuth).toBe(true)
        expect(summary.failures).toEqual([{ md5: MD5S[0], status: 403, reason: 'FORBIDDEN' }])
        expect(seen).toHaveLength(1)
        expect(memory.rows.size).toBe(0)
        const text = formatSummary(summary, false)
        expect(text).toContain('인증 실패로 나머지를 멈춤')
        expect(text).not.toContain(PLAYER_TOKEN)
    })

    test('--dry-run 은 서버에 보내지 않고 같은 스키마로 검사만 한다', async () => {
        const { fakeFetch, seen } = createServer()
        const summary = await run(fakeFetch, null, true)
        expect(summary).toMatchObject({ checked: 3, created: 0 })
        expect(summary.failures.map((failure) => failure.md5)).toEqual([BROKEN_MD5])
        expect(seen).toHaveLength(0)
    })

    test('파일을 읽지 못하면 그 파일만 실패로 남기고 나머지는 올린다', async () => {
        const { fakeFetch } = createServer()
        const { files, skipped } = await listPatternFiles(dir)
        const unreadable = files[0]
        const summary = await uploadChartPatterns({
            files,
            skipped,
            server: SERVER,
            token: ADMIN_TOKEN,
            dryRun: false,
            fetch: fakeFetch,
            readText: async (path) => {
                if (path === unreadable.path) throw new Error('EACCES: permission denied')
                return Bun.file(path).text()
            },
        })
        expect(summary.failures.find((failure) => failure.md5 === unreadable.md5)).toEqual({
            md5: unreadable.md5,
            status: null,
            reason: '파일을 읽지 못했습니다',
        })
        expect(summary.created).toBe(MD5S.length - 1)
    })

    test('서버에 닿지 못하면 그 파일을 실패로 남긴다', async () => {
        const unreachable: FakeFetch = async () => {
            throw new Error('connect ECONNREFUSED')
        }
        const summary = await run(unreachable, ADMIN_TOKEN)
        expect(summary.failures.filter((failure) => failure.reason === '서버에 연결하지 못했습니다')).toHaveLength(3)
    })
})

describe('업로드 스크립트 — 인자와 토큰', () => {
    test('서버 기본값은 운영 주소이고 바꿀 수 있지만, 이 컴퓨터가 아닌 http 주소는 거절한다', () => {
        expect(parseUploadArgs(['/tmp/patterns'])).toEqual({ dir: '/tmp/patterns', server: DEFAULT_SERVER, tokenEnv: null, dryRun: false })
        expect(parseUploadArgs(['/tmp/p', '--server', 'http://localhost:3000/', '--token-env', 'RBMS_TOKEN']).server).toBe('http://localhost:3000')
        expect(() => parseUploadArgs(['/tmp/p', '--server', 'http://bms.example.com'])).toThrow()
        expect(() => parseUploadArgs(['/tmp/p', '--token-env'])).toThrow()
        expect(() => parseUploadArgs([])).toThrow()
        expect(() => parseUploadArgs(['/tmp/p', '--bogus'])).toThrow()
    })

    test('키체인 토큰은 security find-generic-password -a rbms-admin -s rbms-pattern-upload -w 로 읽는다', async () => {
        const commands: string[][] = []
        const spawn: Spawn = (command) => {
            commands.push(command)
            return { exited: Promise.resolve(0), stdout: new Response(`${ADMIN_TOKEN}\n`).body ?? new ReadableStream() }
        }
        expect(await readKeychainToken(spawn)).toBe(ADMIN_TOKEN)
        expect(commands).toEqual([['security', 'find-generic-password', '-a', KEYCHAIN_ACCOUNT, '-s', KEYCHAIN_SERVICE, '-w']])
        const missing: Spawn = () => ({ exited: Promise.resolve(44), stdout: new Response('').body ?? new ReadableStream() })
        const error = await readKeychainToken(missing).catch((caught: unknown) => caught)
        expect(error instanceof Error ? error.message : '').toContain('종료 코드 44')
    })

    test('--token-env 는 그 환경변수에서 읽고 비어 있으면 실패한다', () => {
        expect(readEnvToken('RBMS_PATTERN_TOKEN', { RBMS_PATTERN_TOKEN: ` ${ADMIN_TOKEN} ` })).toBe(ADMIN_TOKEN)
        expect(() => readEnvToken('RBMS_PATTERN_TOKEN', {})).toThrow('RBMS_PATTERN_TOKEN')
    })
})

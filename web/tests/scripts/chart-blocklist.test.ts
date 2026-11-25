import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { CHART_BLOCKLIST_BATCH_MAX } from '@server/dto/chart-blocklist'
import type { AuthUser } from '@server/lib/auth-context'
import { createChartBlocklistRoutes } from '@server/route/chart-blocklist'
import { createChartBlocklistService } from '@server/service/domain/chart-blocklist/chart-blocklist.service'
import { parsePurgeCommand } from '../../scripts/chart-blocklist-purge'
import {
    deriveSourceName,
    isChartFileName,
    listSources,
    parseBlocklistCommand,
    registerCharts,
    scanFolder,
    sourceOfChart,
    type Fetch,
} from '../../scripts/chart-blocklist'
import { createMemoryChartBlocklistDb } from '../support/chart-blocklist-memory-db'

const SERVER = 'https://bms.hyuns.uk'
const ADMIN: AuthUser = { id: 'user-admin', loginId: 'admin', name: 'Admin', role: 'admin' }
const PLAYER: AuthUser = { id: 'user-player', loginId: 'player', name: 'Player', role: 'user' }
const TOKENS = new Map([
    ['rbms_admin_token', ADMIN],
    ['rbms_player_token', PLAYER],
])
const SOURCE = { id: 'iidx-20-tricoro', label: 'beatmania IIDX 20 tricoro' }
const CHART_TEXT = '#TITLE sample\n#00111:01\n'
const OTHER_TEXT = '#TITLE other\n'
const DECOMPOSED_NAME = 'ユミル'.normalize('NFD')

const hexOf = (index: number, length: number) => index.toString(16).padStart(length, '0')
const chartAt = (index: number) => ({ md5: hexOf(index, 32), sha256: hexOf(index, 64), path: `song/${index}.bme` })

/** A `fetch` that hands each request to the real admin route handlers over an in-memory `chart_blocklist`. */
const createServer = () => {
    const memory = createMemoryChartBlocklistDb()
    const routes = createChartBlocklistRoutes(
        () => ({
            authService: { resolveApiTokenUser: async (plain) => TOKENS.get(plain) ?? null },
            chartBlocklistService: createChartBlocklistService({ db: memory.db }),
        }),
        async () => undefined,
    )
    const requests: string[] = []
    const fetch: Fetch = async (input, init) => {
        requests.push(init.method ?? 'GET')
        const request = new Request(input, init)
        return init.method === 'PUT' ? routes.put(request, {}) : routes.list(request, {})
    }
    return { memory, fetch, requests }
}

describe('parseBlocklistCommand', () => {
    test('add 는 폴더 여럿과 --dry-run 을 받고 bun run 의 -- 를 건너뛰며 서버 기본값은 운영이다', () => {
        expect(parseBlocklistCommand(['add', '--', '/a', '/b', '--dry-run'])).toEqual({
            kind: 'add',
            folders: ['/a', '/b'],
            id: null,
            label: null,
            isDryRun: true,
            server: SERVER,
            tokenEnv: null,
        })
        expect(parseBlocklistCommand(['list', '--server', 'http://localhost:3000', '--token-env', 'RBMS_ADMIN_TOKEN'])).toEqual({
            kind: 'list',
            server: 'http://localhost:3000',
            tokenEnv: 'RBMS_ADMIN_TOKEN',
        })
    })

    test('--id·--label 은 폴더 하나일 때만, id 는 kebab-case 만, 서버는 https 나 이 컴퓨터만 받는다', () => {
        expect(parseBlocklistCommand(['add', '/a', '--id', 'iidx-20-tricoro', '--label', ' beatmania IIDX 20 tricoro '])).toMatchObject({
            id: 'iidx-20-tricoro',
            label: 'beatmania IIDX 20 tricoro',
        })
        expect(parseBlocklistCommand(['add', '/a', '/b', '--id', 'x'])).toBeNull()
        expect(parseBlocklistCommand(['add', '/a', '--id', 'IIDX 20'])).toBeNull()
        expect(parseBlocklistCommand(['add', '/a', '--server', 'http://bms.hyuns.uk'])).toBeNull()
        expect(parseBlocklistCommand(['add'])).toBeNull()
        expect(parseBlocklistCommand(['add', '/a', '--force'])).toBeNull()
        expect(parseBlocklistCommand(['list', '/a'])).toBeNull()
        expect(parseBlocklistCommand([])).toBeNull()
    })
})

describe('deriveSourceName', () => {
    test('IIDX 폴더 이름에서 버전과 부제를 뽑는다', () => {
        expect(deriveSourceName('Beatmania IIDX18 ~Resort Anthem~')).toEqual({
            id: 'iidx-18-resort-anthem',
            label: 'beatmania IIDX 18 Resort Anthem',
        })
        expect(deriveSourceName('Beatmania IIDX19 ~Lincle~')).toEqual({ id: 'iidx-19-lincle', label: 'beatmania IIDX 19 Lincle' })
        expect(deriveSourceName('beatmania IIDX 25 CANNON BALLERS')).toEqual({
            id: 'iidx-25-cannon-ballers',
            label: 'beatmania IIDX 25 CANNON BALLERS',
        })
        expect(deriveSourceName('beatmania IIDX 9th style')).toEqual({ id: 'iidx-9-style', label: 'beatmania IIDX 9th style' })
    })

    test('IIDX 곡 폴더 이름에서는 곡명을 빼고 버전만 뽑는다', () => {
        expect(deriveSourceName('IIDX20 ~Tricoro~ - ALBIDA')).toEqual({ id: 'iidx-20-tricoro', label: 'beatmania IIDX 20 Tricoro' })
        expect(deriveSourceName('IIDX18 ~Resort Anthem~ - IIDXAegis')).toEqual({
            id: 'iidx-18-resort-anthem',
            label: 'beatmania IIDX 18 Resort Anthem',
        })
        expect(deriveSourceName('beatmania IIDX 25 CANNON BALLERS - Song')).toEqual({
            id: 'iidx-25-cannon-ballers',
            label: 'beatmania IIDX 25 CANNON BALLERS',
        })
    })

    test('IIDX 가 아닌 이름은 이름 그대로를 kebab-case 로, 만들 수 없으면 null', () => {
        expect(deriveSourceName('Pop n Music 20 fantasia')).toEqual({ id: 'pop-n-music-20-fantasia', label: 'Pop n Music 20 fantasia' })
        expect(deriveSourceName('ユミル')).toBeNull()
    })
})

describe('sourceOfChart', () => {
    test('여러 버전을 모은 폴더는 곡 폴더의 버전으로, 버전 폴더는 그 버전으로, 둘 다 없으면 폴더 이름으로 나눈다', () => {
        expect(sourceOfChart('iidxbms', 'IIDX20 ~Tricoro~ - ALBIDA/ALBIDA [SPA].bme')?.id).toBe('iidx-20-tricoro')
        expect(sourceOfChart('iidxbms', 'IIDX19 ~Lincle~ - F/F [SPA].bme')?.id).toBe('iidx-19-lincle')
        expect(sourceOfChart('Beatmania IIDX18 ~Resort Anthem~', 'IIDX18 ~Resort Anthem~ - ALBIDA/a.bml')?.id).toBe('iidx-18-resort-anthem')
        expect(sourceOfChart('My Pack', 'song/a.bms')).toEqual({ id: 'my-pack', label: 'My Pack' })
        expect(sourceOfChart('ユミル', 'a.bms')).toBeNull()
    })
})

describe('isChartFileName', () => {
    test('클라이언트가 읽는 확장자만 대소문자 없이 고르고 숨김 파일은 뺀다', () => {
        expect(['a.bms', 'a.BME', 'a.bml', 'a.pms', 'a.bmson'].every(isChartFileName)).toBe(true)
        expect(['a.ogg', 'a.dtx', 'a.txt', '._a.bms', '.a.bme'].some(isChartFileName)).toBe(false)
    })
})

describe('scanFolder', () => {
    let root = ''

    beforeAll(async () => {
        root = await mkdtemp(join(tmpdir(), 'chart-blocklist-'))
        await mkdir(join(root, `song ${DECOMPOSED_NAME}`))
        await mkdir(join(root, '.hidden'))
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, 'b [HYPER].BME'), CHART_TEXT)
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, 'a [NORMAL].bms'), OTHER_TEXT)
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, 'copy.bml'), CHART_TEXT)
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, '01.ogg'), 'audio')
        await writeFile(join(root, `song ${DECOMPOSED_NAME}`, '._a [NORMAL].bms'), 'apple double')
        await writeFile(join(root, '.hidden', 'x.bms'), 'hidden')
    })

    afterAll(async () => {
        await rm(root, { recursive: true, force: true })
    })

    test('원본 바이트의 md5·sha256 을 path 순으로 담고 같은 파일은 앞 경로 하나만 남긴다', async () => {
        const folder = `song ${DECOMPOSED_NAME.normalize('NFC')}`
        expect(await scanFolder(root)).toEqual([
            {
                md5: createHash('md5').update(OTHER_TEXT).digest('hex'),
                sha256: createHash('sha256').update(OTHER_TEXT).digest('hex'),
                path: `${folder}/a [NORMAL].bms`,
            },
            {
                md5: createHash('md5').update(CHART_TEXT).digest('hex'),
                sha256: createHash('sha256').update(CHART_TEXT).digest('hex'),
                path: `${folder}/b [HYPER].BME`,
            },
        ])
    })
})

describe('registerCharts·listSources (실제 관리자 라우트, 메모리 DB)', () => {
    test('한 요청의 상한을 넘는 폴더는 나눠 보내고, 다시 보내면 모두 이미 있는 것으로 센다', async () => {
        const { fetch, memory, requests } = createServer()
        const charts = Array.from({ length: CHART_BLOCKLIST_BATCH_MAX + 1 }, (_, index) => chartAt(index + 1))
        const first = await registerCharts({ fetch, server: SERVER, token: 'rbms_admin_token', source: SOURCE, charts })
        expect(first).toEqual({ received: charts.length, added: charts.length, alreadyListed: 0 })
        expect(requests).toEqual(['PUT', 'PUT'])
        expect(memory.rows.get(charts[0].sha256)).toMatchObject({ sourceId: SOURCE.id, sourceLabel: SOURCE.label, addedBy: ADMIN.id })
        const again = await registerCharts({ fetch, server: SERVER, token: 'rbms_admin_token', source: SOURCE, charts: charts.slice(0, 2) })
        expect(again).toEqual({ received: 2, added: 0, alreadyListed: 2 })
        expect(await listSources({ fetch, server: SERVER, token: 'rbms_admin_token' })).toEqual([{ ...SOURCE, charts: charts.length }])
    })

    test('관리자가 아닌 토큰은 403, 토큰이 틀리면 401 로 멈추고 아무것도 넣지 않는다', async () => {
        const { fetch, memory } = createServer()
        const charts = [chartAt(1)]
        await expect(registerCharts({ fetch, server: SERVER, token: 'rbms_player_token', source: SOURCE, charts })).rejects.toThrow(
            'server answered 403 FORBIDDEN',
        )
        await expect(registerCharts({ fetch, server: SERVER, token: 'wrong', source: SOURCE, charts })).rejects.toThrow(
            'server answered 401 UNAUTHORIZED',
        )
        expect(memory.rows.size).toBe(0)
    })

    test('형식이 틀린 차트가 섞이면 400 으로 그 요청을 통째로 거절한다', async () => {
        const { fetch, memory } = createServer()
        const charts = [chartAt(1), { ...chartAt(2), md5: 'A'.repeat(32) }]
        await expect(registerCharts({ fetch, server: SERVER, token: 'rbms_admin_token', source: SOURCE, charts })).rejects.toThrow(
            'server answered 400 VALIDATION_ERROR',
        )
        expect(memory.rows.size).toBe(0)
    })
})

describe('parsePurgeCommand', () => {
    test('count 와 delete 하나만 받는다', () => {
        expect(parsePurgeCommand(['count'])).toBe('count')
        expect(parsePurgeCommand(['delete'])).toBe('delete')
        expect(parsePurgeCommand(['delete', 'now'])).toBeUndefined()
        expect(parsePurgeCommand(['drop'])).toBeUndefined()
        expect(parsePurgeCommand([])).toBeUndefined()
    })
})

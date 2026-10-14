import { describe, expect, test } from 'bun:test'
import { chartMetaSchema } from '@server/dto/chart'
import { submissionChartSchema } from '@server/dto/score'
import { createChartService, fillBlankText, type ChartRow } from '@server/service/domain/chart/chart.service'
import { createFeService, toActivityItem, type ActivityRow, type FeServiceDb } from '@server/service/domain/fe/fe.service'

const SHA = 'a'.repeat(64)
const MD5 = 'b'.repeat(32)

const blankRow = (overrides: Partial<ChartRow> = {}): ChartRow => ({
    sha256: SHA,
    md5: MD5,
    title: '',
    subtitle: '',
    genre: '',
    artist: '',
    subartist: '',
    level: null,
    total: null,
    mode: '',
    lntype: 0,
    judge: 0,
    minbpm: 0,
    maxbpm: 0,
    notes: 0,
    hasLn: false,
    hasCn: false,
    hasHcn: false,
    hasMine: false,
    hasRandom: false,
    hasStop: false,
    url: null,
    appendurl: null,
    extra: {},
    ...overrides,
})

const createService = (seed: ChartRow[] = [], lookup?: (md5: string) => { title: string; artist: string } | null) => {
    const rows = new Map(seed.map((row) => [row.sha256, row]))
    return {
        rows,
        writes: [] as string[],
        service: createChartService({
            lookupTableText: lookup,
            db: {
                findBySha256: async (sha256) => rows.get(sha256) ?? null,
                findByMd5: async (md5) => [...rows.values()].find((row) => row.md5 === md5) ?? null,
                upsert: async (row) => {
                    rows.set(row.sha256, row)
                },
            },
        }),
    }
}

describe('chartService.ensureFromSubmission 의 메타 채우기', () => {
    const params = { md5: MD5, sha256: SHA, title: 'Sent Title', artist: 'Sent Artist', level: 12, mode: '7K', notes: 500, lntype: 0 }

    test('새 차트는 보낸 제목, 아티스트, 레벨로 만든다', async () => {
        const { service, rows } = createService()
        expect(await service.ensureFromSubmission(params)).toEqual({ sha256: SHA, md5: MD5, changed: true })
        expect(rows.get(SHA)).toMatchObject({ title: 'Sent Title', artist: 'Sent Artist', level: 12, notes: 500 })
    })

    test('공란인 행만 채우고 값이 있는 필드는 건드리지 않는다', async () => {
        const { service, rows } = createService([blankRow({ title: 'Existing Title', artist: '', level: 7 })])
        const result = await service.ensureFromSubmission(params)
        expect(result).toEqual({ sha256: SHA, md5: MD5, changed: true })
        expect(rows.get(SHA)).toMatchObject({ title: 'Existing Title', artist: 'Sent Artist', level: 7 })
    })

    test('이미 다 찬 행은 쓰지 않는다', async () => {
        const { service, rows } = createService([blankRow({ title: 'T', artist: 'A', level: 3 })])
        const before = rows.get(SHA)
        expect(await service.ensureFromSubmission(params)).toEqual({ sha256: SHA, md5: MD5, changed: false })
        expect(rows.get(SHA)).toBe(before)
    })

    test('메타 없이 보낸 옛 클라이언트의 요청도 그대로 통과한다', async () => {
        const { service, rows } = createService()
        const legacy = await service.ensureFromSubmission({ md5: MD5, sha256: SHA, title: '', mode: '7K', notes: 10, lntype: 0 })
        expect(legacy).toEqual({ sha256: SHA, md5: MD5, changed: true })
        expect(rows.get(SHA)).toMatchObject({ title: '', artist: '', level: null })
    })

    test('md5 가 비어 있던 행은 md5 도 채운다', async () => {
        const { service, rows } = createService([blankRow({ md5: null })])
        const result = await service.ensureFromSubmission({ ...params, title: '', artist: '', level: null })
        expect(result).toEqual({ sha256: SHA, md5: MD5, changed: true })
        expect(rows.get(SHA)?.md5).toBe(MD5)
    })

    test('sha256 도 기존 행도 없으면 null 이다', async () => {
        const { service } = createService()
        expect(await service.ensureFromSubmission({ ...params, sha256: '' })).toBeNull()
    })
})

describe('chartService.upsertFromMeta 는 덮어쓰지 않는다', () => {
    const incoming = chartMetaSchema.parse({ sha256: SHA, md5: MD5, title: 'Vandal', artist: 'Vandal', level: 99, notes: 1 })

    test('값이 있는 기존 행의 제목과 레벨은 그대로이고 공란만 채운다', async () => {
        const { service, rows } = createService([blankRow({ title: 'Real Title', artist: '', level: 10, notes: 800 })])
        await service.upsertFromMeta(incoming)
        expect(rows.get(SHA)).toMatchObject({ title: 'Real Title', artist: 'Vandal', level: 10, notes: 800 })
    })

    test('새 차트는 보낸 값 그대로 만든다', async () => {
        const { service, rows } = createService()
        await service.upsertFromMeta(incoming)
        expect(rows.get(SHA)).toMatchObject({ title: 'Vandal', level: 99 })
    })

    test('sha256 도 기존 행도 없으면 null 이다', async () => {
        const { service } = createService()
        expect(await service.upsertFromMeta(chartMetaSchema.parse({ md5: MD5, title: 'x' }))).toBeNull()
    })
})

describe('표 제목으로 읽을 때 보강', () => {
    const lookup = (md5: string) => (md5 === MD5 ? { title: 'Table Title', artist: 'Table Artist' } : null)

    test('fillBlankText 는 공란인 필드만 채운다', () => {
        expect(fillBlankText({ md5: MD5, title: '', artist: 'Mine' }, lookup)).toEqual({ md5: MD5, title: 'Table Title', artist: 'Mine' })
        const full = { md5: MD5, title: 'T', artist: 'A' }
        expect(fillBlankText(full, lookup)).toBe(full)
        const unknown = { md5: 'c'.repeat(32), title: '', artist: '' }
        expect(fillBlankText(unknown, lookup)).toBe(unknown)
        const noMd5 = { md5: '', title: '', artist: '' }
        expect(fillBlankText(noMd5, lookup)).toBe(noMd5)
        expect(fillBlankText({ md5: MD5, title: '', artist: '' }, undefined).title).toBe('')
    })

    test('getMetaByHash 는 DB 를 고치지 않고 응답만 보강한다', async () => {
        const { service, rows } = createService([blankRow()], lookup)
        const meta = await service.getMetaByHash(MD5)
        expect(meta).toMatchObject({ title: 'Table Title', artist: 'Table Artist' })
        expect(rows.get(SHA)?.title).toBe('')
    })

    test('활동 항목의 제목도 보강한다', () => {
        const row: ActivityRow = {
            scoreId: 's1',
            loginId: 'p1',
            playerName: 'P1',
            clear: 5,
            exScore: 1,
            minbp: 1,
            maxCombo: 1,
            playedAt: 1,
            chartSha256: SHA,
            chartMd5: MD5,
            title: '',
            artist: '',
            level: null,
        }
        expect(toActivityItem(row, lookup).chart).toMatchObject({ title: 'Table Title', artist: 'Table Artist' })
        expect(toActivityItem(row).chart.title).toBe('')
    })

    test('검색은 표 제목에 맞는 md5 를 DB 쿼리에 함께 넘기고 결과를 보강한다', async () => {
        const seen: string[][] = []
        const db: FeServiceDb = {
            searchCharts: async (_query, options) => {
                seen.push(options.extraMd5s)
                return { rows: [blankRow()], total: 1 }
            },
            listRecentActivity: async () => [],
            listPlayerRecent: async () => [],
            listPlayerLeaderboard: async () => ({ rows: [], total: 0 }),
            countLampsByUser: async () => [],
            summarise: async () => ({ charts: 0, players: 0, scores: 0 }),
            listTokens: async () => [],
            deleteToken: async () => false,
        }
        const service = createFeService({ db, lookupTableText: lookup, searchTableMd5s: (query) => (query === 'table' ? [MD5] : []) })
        const withQuery = await service.searchCharts({ q: 'table', sort: 'title', page: 1, limit: 50 })
        expect(seen[0]).toEqual([MD5])
        expect(withQuery.items[0]).toMatchObject({ title: 'Table Title' })
        await service.searchCharts({ sort: 'title', page: 1, limit: 50 })
        expect(seen[1]).toEqual([])
    })
})

describe('제출 본문의 chart 메타', () => {
    test('title, artist, level 은 선택이고 기본은 빈 문자열과 null 이다', () => {
        expect(submissionChartSchema.parse({ md5: MD5, sha256: SHA })).toEqual({ md5: MD5, sha256: SHA, title: '', artist: '', level: null })
    })

    test('긴 문자열은 잘라서 받고 이상한 레벨은 null 로 받아 제출을 막지 않는다', () => {
        const parsed = submissionChartSchema.parse({ md5: MD5, sha256: SHA, title: ` ${'가'.repeat(300)} `, artist: null, level: 12.5 })
        expect([...parsed.title]).toHaveLength(255)
        expect(parsed.artist).toBe('')
        expect(parsed.level).toBeNull()
        expect(submissionChartSchema.parse({ md5: MD5, sha256: SHA, level: -3 }).level).toBeNull()
        expect(submissionChartSchema.parse({ md5: MD5, sha256: SHA, level: 12 }).level).toBe(12)
    })
})

describe('POST /api/charts 의 문자열 상한', () => {
    test('255자를 넘는 제목과 512자를 넘는 url, 16KB 를 넘는 extra 를 거부한다', () => {
        expect(chartMetaSchema.safeParse({ title: 't'.repeat(256) }).success).toBe(false)
        expect(chartMetaSchema.safeParse({ title: 't'.repeat(255) }).success).toBe(true)
        expect(chartMetaSchema.safeParse({ url: 'u'.repeat(513) }).success).toBe(false)
        expect(chartMetaSchema.safeParse({ extra: { blob: 'x'.repeat(17 * 1024) } }).success).toBe(false)
        expect(chartMetaSchema.safeParse({ mode: 'm'.repeat(17) }).success).toBe(false)
    })
})

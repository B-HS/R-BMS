import { describe, expect, test } from 'bun:test'
import { createRemoteSnapshotCache, createTableLoader } from '@server/compose/table-snapshot'
import { createChartBlocklistService } from '@server/service/domain/chart-blocklist/chart-blocklist.service'
import { BLOCKED_TABLE_ID, blockedChartTitle, createBlockedTableCache, toBlockedTable } from '@server/service/domain/table-snapshot/blocked-table'
import { createTableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { createMemoryChartBlocklistDb } from '../support/chart-blocklist-memory-db'

const listed = (seed: string, sourceId: string, sourceLabel: string, path: string) => ({ md5: seed.repeat(32), sourceId, sourceLabel, path })

const CHARTS = [
    listed('c', 'iidx-20-tricoro', 'beatmania IIDX 20 Tricoro', 'IIDX20 ~Tricoro~ - A/A [SPA].bme'),
    listed('a', 'iidx-9-style', 'beatmania IIDX 9th style', 'x/B [SPN].bms'),
    listed('b', 'iidx-18-resort-anthem', 'beatmania IIDX 18 Resort Anthem', 'IIDX18 ~Resort Anthem~ - ALBIDA/ALBIDA [SP NORMAL].bml'),
]
const VERSION = { charts: CHARTS.length, lastAddedAt: Date.UTC(2026, 10, 25, 12) }

const noRemoteTables = createRemoteSnapshotCache({ listVersions: async () => [], loadSnapshot: async () => null })

describe('toBlockedTable', () => {
    test('묶음마다 한 레벨(이름)로, 묶음 id 의 숫자를 수로 비교한 순서로 놓고 제목은 파일 이름이다', () => {
        const table = toBlockedTable(CHARTS, VERSION)
        expect(table?.id).toBe(BLOCKED_TABLE_ID)
        expect(table?.header.level_order).toEqual(['beatmania IIDX 9th style', 'beatmania IIDX 18 Resort Anthem', 'beatmania IIDX 20 Tricoro'])
        expect(table?.entries.map((entry) => entry.title)).toEqual(['B [SPN]', 'ALBIDA [SP NORMAL]', 'A [SPA]'])
        expect(table?.header.last_update).toBe('2026-11-25')
        expect(table?.header.symbol).toBe('')
        expect(JSON.parse(table?.standardData() ?? '[]')).toHaveLength(CHARTS.length)
        expect(table?.source.snapshot.data_file_sha256).toMatch(/^[0-9a-f]{64}$/)
    })

    test('목록이 비면 표가 없다', () => {
        expect(toBlockedTable([], { charts: 0, lastAddedAt: 0 })).toBeNull()
    })

    test('제목은 경로의 마지막 파일 이름에서 확장자만 뺀다', () => {
        expect(blockedChartTitle('song/Song v1.0 [DPA].bme')).toBe('Song v1.0 [DPA]')
        expect(blockedChartTitle('a.bmson')).toBe('a')
    })
})

describe('createBlockedTableCache', () => {
    test('목록의 버전이 바뀔 때만 다시 만들고, 새 차트를 등록하면 다음 읽기부터 표에 나온다', async () => {
        const memory = createMemoryChartBlocklistDb()
        const service = createChartBlocklistService({ db: memory.db })
        const reads = { charts: 0 }
        const cache = createBlockedTableCache({
            listCharts: async () => {
                reads.charts += 1
                return service.listCharts()
            },
            version: service.version,
        })
        expect(await cache.load()).toBeNull()
        await service.register({
            source: { id: 'iidx-20-tricoro', label: 'beatmania IIDX 20 Tricoro' },
            charts: [{ md5: 'a'.repeat(32), sha256: 'b'.repeat(64), path: 'A/A [SPA].bme' }],
            addedBy: 'admin',
        })
        expect((await cache.load())?.entries).toHaveLength(1)
        expect((await cache.load())?.entries).toHaveLength(1)
        expect(reads.charts).toBe(2)
    })
})

describe('표 서비스에 실린 차단 표', () => {
    test('표 목록과 표준 header·data 로 나오고, header 의 data_url 은 그 사이트의 data.json 이다', async () => {
        const blocked = { load: async () => toBlockedTable(CHARTS, VERSION) }
        const service = createTableSnapshotService({ loadTables: createTableLoader(noRemoteTables, blocked), db: { listBestByMd5: async () => [] } })
        const summary = (await service.list()).find((table) => table.id === BLOCKED_TABLE_ID)
        expect(summary).toMatchObject({ entry_count: CHARTS.length, level_count: 3, standard_paths: { page: '/tables/blocked' } })
        expect(await service.getStandardHeader(BLOCKED_TABLE_ID, 'https://bms.hyuns.uk')).toMatchObject({
            data_url: 'https://bms.hyuns.uk/tables/blocked/data.json',
        })
        expect(JSON.parse((await service.getStandardDataText(BLOCKED_TABLE_ID)) ?? '[]')[0]).toMatchObject({ md5: 'a'.repeat(32) })
    })

    test('차단 목록이 비면 표 목록에 나오지 않는다', async () => {
        const service = createTableSnapshotService({ loadTables: createTableLoader(noRemoteTables), db: { listBestByMd5: async () => [] } })
        expect((await service.list()).some((table) => table.id === BLOCKED_TABLE_ID)).toBe(false)
    })
})

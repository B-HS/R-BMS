import { describe, expect, test } from 'bun:test'
import { getStatusCode } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { rejectBlockedChart } from '@server/route/chart'
import { createChartBlocklistService } from '@server/service/domain/chart-blocklist/chart-blocklist.service'
import { createMemoryChartBlocklistDb } from '../support/chart-blocklist-memory-db'

const LISTED = {
    sha256: 'b'.repeat(64),
    md5: 'a'.repeat(32),
    sourceId: 'iidx-18-resort-anthem',
    sourceLabel: 'IIDX 18',
    path: 'x.bml',
    addedBy: 'admin',
}
const OTHER_MD5 = 'c'.repeat(32)
const OTHER_SHA256 = 'd'.repeat(64)
const SOURCE = { id: 'iidx-19-lincle', label: 'beatmania IIDX 19 Lincle' }

const createService = () => {
    const memory = createMemoryChartBlocklistDb([LISTED])
    return { memory, service: createChartBlocklistService({ db: memory.db }) }
}

describe('chartBlocklistService.isBlocked', () => {
    test('sha256 이나 md5 하나만 맞아도 차단이다', async () => {
        const { service } = createService()
        expect(await service.isBlocked({ sha256: LISTED.sha256, md5: '' })).toBe(true)
        expect(await service.isBlocked({ md5: LISTED.md5, sha256: OTHER_SHA256 })).toBe(true)
        expect(await service.isBlocked({ md5: OTHER_MD5, sha256: OTHER_SHA256 })).toBe(false)
    })

    test('해시 꼴이 아닌 값만 있으면 DB 를 묻지 않고 차단하지 않는다', async () => {
        const { service, memory } = createService()
        expect(await service.isBlocked({ md5: '', sha256: '' })).toBe(false)
        expect(await service.isBlocked({ md5: 'A'.repeat(32), sha256: 'not-a-hash' })).toBe(false)
        expect(await service.isBlocked({})).toBe(false)
        expect(memory.calls.isListed).toBe(0)
    })
})

describe('chartBlocklistService.register', () => {
    test('어느 목록에든 sha256 이나 md5 로 이미 있는 차트는 세기만 하고 새 차트만 그 출처로 넣는다', async () => {
        const { service, memory } = createService()
        const charts = [
            { sha256: LISTED.sha256, md5: OTHER_MD5, path: 'same-sha.bme' },
            { sha256: OTHER_SHA256, md5: LISTED.md5, path: 'same-md5.bme' },
            { sha256: 'e'.repeat(64), md5: 'f'.repeat(32), path: 'new.bme' },
        ]
        expect(await service.register({ source: SOURCE, charts, addedBy: 'admin' })).toEqual({ received: 3, added: 1, alreadyListed: 2 })
        expect(memory.rows.get('e'.repeat(64))).toEqual({ ...charts[2], sourceId: SOURCE.id, sourceLabel: SOURCE.label, addedBy: 'admin' })
    })

    test('같은 차트를 다시 보내면 아무것도 넣지 않는다', async () => {
        const { service } = createService()
        const charts = [{ sha256: 'e'.repeat(64), md5: 'f'.repeat(32), path: 'new.bme' }]
        await service.register({ source: SOURCE, charts, addedBy: 'admin' })
        expect(await service.register({ source: SOURCE, charts, addedBy: 'admin' })).toEqual({ received: 1, added: 0, alreadyListed: 1 })
        expect(await service.listSources()).toEqual([
            { sourceId: 'iidx-18-resort-anthem', sourceLabel: 'IIDX 18', charts: 1 },
            { sourceId: SOURCE.id, sourceLabel: SOURCE.label, charts: 1 },
        ])
    })
})

describe('rejectBlockedChart', () => {
    test('차단된 차트는 422 IR_CHART_BLOCKED 를 던지고 아니면 지나간다', async () => {
        const { service } = createService()
        await expect(rejectBlockedChart({ sha256: LISTED.sha256 }, service)).rejects.toMatchObject({
            code: ERROR_CODE.IR_CHART_BLOCKED,
            statusCode: 422,
        })
        expect(await rejectBlockedChart({ md5: OTHER_MD5, sha256: OTHER_SHA256 }, service)).toBeUndefined()
        expect(getStatusCode(ERROR_CODE.IR_CHART_BLOCKED)).toBe(422)
    })
})

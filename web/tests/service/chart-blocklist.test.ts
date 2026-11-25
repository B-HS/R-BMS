import { describe, expect, test } from 'bun:test'
import { readdir } from 'node:fs/promises'
import { join } from 'node:path'
import { getStatusCode } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { rejectBlockedChart } from '@server/route/chart'
import { createChartBlocklist, getChartBlocklist } from '@server/service/domain/chart-blocklist/chart-blocklist.service'

const BLOCKLIST_DIR = join(import.meta.dirname, '..', '..', 'data', 'chart-blocklist')
const IIDX18_CHARTS = 391
const IIDX19_CHARTS = 353
const LISTED = { md5: 'a'.repeat(32), sha256: 'b'.repeat(64), path: 'x/y.bms' }
const OTHER_MD5 = 'c'.repeat(32)
const OTHER_SHA256 = 'd'.repeat(64)

describe('커밋된 차트 차단 목록', () => {
    test('data/chart-blocklist 의 파일이 모두 서버에 실리고 id 가 파일 이름과 같다', async () => {
        const files = (await readdir(BLOCKLIST_DIR)).filter((name) => name.endsWith('.json')).toSorted()
        const ids = getChartBlocklist()
            .sources.map((source) => `${source.id}.json`)
            .toSorted()
        expect(ids).toEqual(files)
    })

    test('IIDX18 Resort Anthem 391개, IIDX19 Lincle 353개이고 sha256·md5 가 겹치지 않는다', () => {
        const blocklist = getChartBlocklist()
        const counts = Object.fromEntries(blocklist.sources.map((source) => [source.id, source.charts.length]))
        expect(counts).toEqual({ 'iidx-18-resort-anthem': IIDX18_CHARTS, 'iidx-19-lincle': IIDX19_CHARTS })
        expect(blocklist.sha256s).toHaveLength(IIDX18_CHARTS + IIDX19_CHARTS)
        expect(blocklist.md5s).toHaveLength(IIDX18_CHARTS + IIDX19_CHARTS)
    })

    test('목록의 차트는 sha256 이나 md5 하나만 맞아도 차단이다', () => {
        const [first] = getChartBlocklist().sources[0].charts
        expect(getChartBlocklist().isBlocked({ sha256: first.sha256, md5: '' })).toBe(true)
        expect(getChartBlocklist().isBlocked({ md5: first.md5 })).toBe(true)
    })
})

describe('createChartBlocklist', () => {
    const blocklist = createChartBlocklist([{ id: 'sample', label: 'Sample', charts: [LISTED] }])

    test('목록에 없는 해시, 빈 해시, 해시 없음은 차단하지 않는다', () => {
        expect(blocklist.isBlocked({ md5: OTHER_MD5, sha256: OTHER_SHA256 })).toBe(false)
        expect(blocklist.isBlocked({ md5: '', sha256: '' })).toBe(false)
        expect(blocklist.isBlocked({})).toBe(false)
    })

    test('형식이 틀린 파일은 서버가 읽을 때 실패한다', () => {
        expect(() => createChartBlocklist([{ id: 'Bad Id', label: 'x', charts: [LISTED] }])).toThrow()
        expect(() => createChartBlocklist([{ id: 'bad', label: 'x', charts: [{ ...LISTED, md5: 'A'.repeat(32) }] }])).toThrow()
        expect(() => createChartBlocklist([{ id: 'empty', label: 'x', charts: [] }])).toThrow()
    })
})

describe('rejectBlockedChart', () => {
    const blocklist = createChartBlocklist([{ id: 'sample', label: 'Sample', charts: [LISTED] }])

    test('차단된 차트는 422 IR_CHART_BLOCKED 를 던진다', () => {
        expect(() => rejectBlockedChart({ sha256: LISTED.sha256 }, blocklist)).toThrow(
            expect.objectContaining({ code: ERROR_CODE.IR_CHART_BLOCKED, statusCode: 422 }),
        )
        expect(getStatusCode(ERROR_CODE.IR_CHART_BLOCKED)).toBe(422)
    })

    test('차단되지 않은 차트는 그대로 지나간다', () => {
        expect(() => rejectBlockedChart({ md5: OTHER_MD5, sha256: OTHER_SHA256 }, blocklist)).not.toThrow()
    })
})

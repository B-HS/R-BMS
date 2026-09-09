import { describe, expect, test } from 'bun:test'
import { replayUploadSchema } from '@server/dto/replay'
import { createReplayService, measureDurationUs, type ReplayInsertRow, type ReplayRow } from '@server/service/domain/replay/replay.service'
import { createMysqlReplayStorage } from '@server/service/shared/storage/replay-storage'

const CHART_SHA256 = 'a'.repeat(64)
const USER_ID = 'user-1'

const MICROSECOND_EVENTS = [
    { t_us: 1_234_567, lane: 0, press: true },
    { t_us: 1_289_001, lane: 0, press: false },
    { t_us: 9_007_199_254_740, lane: 7, press: true },
]

const createStubDb = () => {
    const stored: ReplayInsertRow[] = []
    const links: { scoreId: string; userId: string; replayId: string }[] = []
    return {
        stored,
        links,
        db: {
            insertReplay: async (row: ReplayInsertRow) => {
                stored.push(row)
            },
            findReplayById: async (id: string): Promise<ReplayRow | null> => {
                const row = stored.find((entry) => entry.id === id)
                return row ? { ...row, loginId: 'alice', playerName: 'Alice', createdAt: 1 } : null
            },
            listReplaysByChart: async ({ limit }: { chartSha256: string; userId?: string; limit: number }): Promise<ReplayRow[]> =>
                stored.slice(0, limit).map((row) => ({ ...row, loginId: 'alice', playerName: 'Alice', createdAt: 1 })),
            linkScoreReplay: async (params: { scoreId: string; userId: string; replayId: string }) => {
                links.push(params)
            },
        },
    }
}

const createService = () => {
    const stub = createStubDb()
    let counter = 0
    const service = createReplayService({
        db: stub.db,
        storage: createMysqlReplayStorage(),
        newId: (prefix) => `${prefix}${(counter += 1)}`,
    })
    return { stub, service }
}

describe('replay 업로드/다운로드 왕복', () => {
    test('µs 타임스탬프가 라운딩 없이 그대로 되돌아온다', async () => {
        const { service } = createService()
        const input = replayUploadSchema.parse({ format: 'rbms-replay-ron-v1', events: MICROSECOND_EVENTS, seed: 12345 })
        const uploaded = await service.upload({ input, chartSha256: CHART_SHA256, userId: USER_ID })
        const downloaded = await service.getById(uploaded.id)
        expect(JSON.stringify(downloaded?.events)).toBe(JSON.stringify(MICROSECOND_EVENTS))
    })

    test('event_count 와 duration_us 를 이벤트에서 유도한다', async () => {
        const { service } = createService()
        const input = replayUploadSchema.parse({ format: 'rbms-replay-ron-v1', events: MICROSECOND_EVENTS })
        const uploaded = await service.upload({ input, chartSha256: CHART_SHA256, userId: USER_ID })
        expect(uploaded.event_count).toBe(3)
        const downloaded = await service.getById(uploaded.id)
        expect(downloaded?.duration_us).toBe(9_007_199_254_740 - 1_234_567)
    })

    test('score_id 가 오면 그 스코어에 리플레이를 연결한다', async () => {
        const { stub, service } = createService()
        const input = replayUploadSchema.parse({ format: 'f', events: [], score_id: 'sc_9' })
        const uploaded = await service.upload({ input, chartSha256: CHART_SHA256, userId: USER_ID })
        expect(stub.links).toEqual([{ scoreId: 'sc_9', userId: USER_ID, replayId: uploaded.id }])
    })

    test('없는 리플레이 id 는 null 이다', async () => {
        const { service } = createService()
        expect(await service.getById('rp_missing')).toBeNull()
    })

    test('목록은 events 없이 메타만 돌려준다', async () => {
        const { service } = createService()
        const input = replayUploadSchema.parse({ format: 'f', events: MICROSECOND_EVENTS })
        await service.upload({ input, chartSha256: CHART_SHA256, userId: USER_ID })
        const [meta] = await service.listByChart({ chartSha256: CHART_SHA256, limit: 10 })
        expect(meta.event_count).toBe(3)
        expect(Object.keys(meta)).not.toContain('events')
    })
})

describe('measureDurationUs', () => {
    test('이벤트가 없으면 0 이다', () => {
        expect(measureDurationUs([])).toBe(0)
    })

    test('가장 이른 이벤트와 늦은 이벤트의 간격이다', () => {
        expect(
            measureDurationUs([
                { t_us: 500, lane: 0, press: true },
                { t_us: 120, lane: 1, press: true },
            ]),
        ).toBe(380)
    })
})

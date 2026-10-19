import { describe, expect, test } from 'bun:test'
import { clearLampToId } from '@server/dto/common'
import { createFeService, type ActivityRow, type FeServiceDb, type LampCountRow } from '@server/service/domain/fe/fe.service'

const SHA = 'a'.repeat(64)
const MD5 = 'b'.repeat(32)

const activityRow = (overrides: Partial<ActivityRow> = {}): ActivityRow => ({
    scoreId: 'sc_1',
    loginId: 'alice',
    playerName: 'Alice',
    clear: clearLampToId('Hard'),
    exScore: 100,
    minbp: 1,
    maxCombo: 10,
    playedAt: 1_780_000_000_000,
    scratchAuto: false,
    chartSha256: SHA,
    chartMd5: MD5,
    title: '',
    artist: '',
    level: null,
    ...overrides,
})

const lampRow = (lamp: Parameters<typeof clearLampToId>[0], count: number, scratchAuto = false): LampCountRow => ({
    clear: clearLampToId(lamp),
    scratchAuto,
    count,
})

const stubDb = (overrides: Partial<FeServiceDb> = {}): FeServiceDb => ({
    searchCharts: async () => ({ rows: [], total: 0 }),
    listRecentActivity: async () => [],
    listPlayerRecent: async () => ({ rows: [], total: 0 }),
    listPlayerLeaderboard: async () => ({ rows: [], total: 0 }),
    countLampsByUser: async () => [],
    summarise: async () => ({ charts: 0, players: 0, scores: 0 }),
    listTokens: async () => [],
    deleteToken: async () => false,
    ...overrides,
})

describe('feService.playerRecent 는 플레이 기록을 페이지로 준다', () => {
    test('페이지와 개수를 DB 에 그대로 넘기고 전체 판 수를 함께 돌려준다', async () => {
        const seen: unknown[] = []
        const service = createFeService({
            db: stubDb({
                listPlayerRecent: async (params) => {
                    seen.push(params)
                    return { rows: [activityRow({ scoreId: 'sc_21' }), activityRow({ scoreId: 'sc_22' })], total: 42 }
                },
            }),
        })
        const result = await service.playerRecent({ userId: 'u_alice', page: 2, limit: 20 })
        expect(seen).toEqual([{ userId: 'u_alice', page: 2, limit: 20 }])
        expect(result.total).toBe(42)
        expect(result.items.map((item) => item.score_id)).toEqual(['sc_21', 'sc_22'])
    })

    test('기록이 없는 범위는 빈 목록과 전체 판 수를 돌려준다', async () => {
        const service = createFeService({ db: stubDb({ listPlayerRecent: async () => ({ rows: [], total: 3 }) }) })
        expect(await service.playerRecent({ userId: 'u_alice', page: 9, limit: 20 })).toEqual({ items: [], total: 3 })
    })

    test('표 제목으로 빈 제목을 채운다', async () => {
        const service = createFeService({
            db: stubDb({ listPlayerRecent: async () => ({ rows: [activityRow()], total: 1 }) }),
            lookupTableText: (md5) => (md5 === MD5 ? { title: 'Table Title', artist: 'Table Artist' } : null),
        })
        const { items } = await service.playerRecent({ userId: 'u_alice', page: 1, limit: 20 })
        expect(items[0].chart).toMatchObject({ title: 'Table Title', artist: 'Table Artist' })
    })
})

describe('feService.playerStats 의 클리어 수', () => {
    test('ASSIST 와 L-ASSIST 도 클리어로 세고 NO PLAY 와 FAILED 만 뺀다', async () => {
        const service = createFeService({
            db: stubDb({
                countLampsByUser: async () => [
                    lampRow('NoPlay', 3),
                    lampRow('Failed', 5),
                    lampRow('AssistEasy', 2),
                    lampRow('LightAssistEasy', 4, true),
                    lampRow('Easy', 1),
                    lampRow('Max', 6),
                ],
            }),
        })
        const stats = await service.playerStats({ userId: 'u_alice', totalPlays: 21 })
        expect(stats.cleared).toBe(2 + 4 + 1 + 6)
        expect(stats.scratch_auto_charts).toBe(4)
        expect(stats.lamps.map((entry) => entry.clear)).toEqual(['NoPlay', 'Failed', 'AssistEasy', 'LightAssistEasy', 'Easy', 'Max'])
    })

    test('FAILED 만 있으면 클리어는 0 이다', async () => {
        const service = createFeService({ db: stubDb({ countLampsByUser: async () => [lampRow('Failed', 8)] }) })
        expect((await service.playerStats({ userId: 'u_alice', totalPlays: 8 })).cleared).toBe(0)
    })
})

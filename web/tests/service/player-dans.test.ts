import { describe, expect, test } from 'bun:test'
import type { DisplayedDan } from '@server/dto/dan'
import { createFeService, type FeServiceDb, type LeaderboardRow } from '@server/service/domain/fe/fe.service'
import { createPlayerService, type PlayerRow } from '@server/service/domain/player/player.service'

const ALICE: PlayerRow = { id: 'u_alice', loginId: 'alice', name: 'Alice', rank: '', rankPoints: 0, totalPlays: 0, role: 'user' }
const BOB: PlayerRow = { id: 'u_bob', loginId: 'bob', name: 'Bob', rank: '', rankPoints: 0, totalPlays: 0, role: 'user' }
const ALICE_PLAYS = 12

const GENOSIDE_SP_DAN: DisplayedDan = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: '発狂五段', assisted: false }
const GENOSIDE_SP_ASSISTED_DAN: DisplayedDan = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: 'A-発狂六段', assisted: true }
const STELLA_DAN: DisplayedDan = { system: 'stella', name: 'Stella', label: 'st3', assisted: false }

const leaderboardRow = (row: PlayerRow): LeaderboardRow => ({ ...row, plays: 1, clearedCharts: 1, playedCharts: 1 })

const feDb = (rows: LeaderboardRow[]): FeServiceDb => ({
    searchCharts: async () => ({ rows: [], total: 0 }),
    listRecentActivity: async () => [],
    listPlayerRecent: async () => ({ rows: [], total: 0 }),
    listPlayerLeaderboard: async () => ({ rows, total: rows.length }),
    countLampsByUser: async () => [],
    summarise: async () => ({ charts: 0, players: 0, scores: 0 }),
    listTokens: async () => [],
    deleteToken: async () => false,
})

describe('플레이어 프로필의 표시 단위', () => {
    test('플레이어 프로필 extra 에 표시 단위가 들어간다', async () => {
        const asked: string[] = []
        const service = createPlayerService({
            db: { findByLoginId: async () => ALICE, findById: async () => ALICE, countScores: async () => ALICE_PLAYS },
            listDisplayedDans: async (userId) => {
                asked.push(userId)
                return [GENOSIDE_SP_DAN, GENOSIDE_SP_ASSISTED_DAN, STELLA_DAN]
            },
        })

        const profile = await service.getProfileByLoginId('alice')

        expect(asked).toEqual([ALICE.id])
        expect(profile).toEqual({
            id: 'alice',
            name: 'Alice',
            total_plays: ALICE_PLAYS,
            rank_points: 0,
            extra: { rank: '', dans: [GENOSIDE_SP_DAN, GENOSIDE_SP_ASSISTED_DAN, STELLA_DAN] },
        })
    })

    test('단위가 없으면 빈 목록이고, 없는 플레이어는 단위를 묻지 않는다', async () => {
        const asked: string[] = []
        const service = createPlayerService({
            db: { findByLoginId: async () => null, findById: async () => ALICE, countScores: async () => 0 },
            listDisplayedDans: async (userId) => {
                asked.push(userId)
                return []
            },
        })

        expect(await service.getProfileByLoginId('nobody')).toBeNull()
        expect((await service.getProfileById(ALICE.id))?.extra).toEqual({ rank: '', dans: [] })
        expect(asked).toEqual([ALICE.id])
    })
})

describe('플레이어 순위의 표시 단위', () => {
    test('순위 행마다 표시 단위가 붙고 조회는 한 번이다', async () => {
        const asked: string[][] = []
        const service = createFeService({
            db: feDb([leaderboardRow(ALICE), leaderboardRow(BOB)]),
            listDisplayedDans: async (userIds) => {
                asked.push(userIds)
                return new Map([[ALICE.id, [STELLA_DAN]]])
            },
        })

        const { items } = await service.playerLeaderboard({ sort: 'cleared', page: 1, limit: 20, include_scratch_auto: false })

        expect(asked).toEqual([[ALICE.id, BOB.id]])
        expect(items.map((item) => [item.id, item.dans])).toEqual([
            ['alice', [STELLA_DAN]],
            ['bob', []],
        ])
    })
})

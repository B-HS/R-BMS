import type { PlayerProfileOutput } from '@server/dto/auth'

export type PlayerRow = {
    id: string
    loginId: string
    name: string
    rank: string
    rankPoints: number
    totalPlays: number
    role: string
}

export type PlayerServiceDb = {
    findByLoginId: (loginId: string) => Promise<PlayerRow | null>
    findById: (id: string) => Promise<PlayerRow | null>
    countScores: (userId: string) => Promise<number>
}

export const toPlayerProfile = (row: PlayerRow, totalPlays: number): PlayerProfileOutput => ({
    id: row.loginId,
    name: row.name,
    total_plays: totalPlays,
    rank_points: row.rankPoints,
    extra: { rank: row.rank },
})

export const createPlayerService = (deps: { db: PlayerServiceDb }) => ({
    getProfileByLoginId: async (loginId: string) => {
        const row = await deps.db.findByLoginId(loginId)
        if (!row) return null
        const totalPlays = await deps.db.countScores(row.id)
        return toPlayerProfile(row, totalPlays)
    },
    getProfileById: async (id: string) => {
        const row = await deps.db.findById(id)
        if (!row) return null
        const totalPlays = await deps.db.countScores(row.id)
        return toPlayerProfile(row, totalPlays)
    },
})

export type PlayerService = ReturnType<typeof createPlayerService>

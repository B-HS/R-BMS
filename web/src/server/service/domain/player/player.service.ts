import type { PlayerProfileOutput } from '@server/dto/auth'
import type { DisplayedDan } from '@server/dto/dan'

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

export type PlayerServiceDeps = {
    db: PlayerServiceDb
    listDisplayedDans: (userId: string) => Promise<DisplayedDan[]>
}

export const createPlayerService = (deps: PlayerServiceDeps) => {
    const toProfileWithDans = async (row: PlayerRow): Promise<PlayerProfileOutput> => {
        const [totalPlays, dans] = await Promise.all([deps.db.countScores(row.id), deps.listDisplayedDans(row.id)])
        const profile = toPlayerProfile(row, totalPlays)
        return { ...profile, extra: { ...profile.extra, dans } }
    }

    return {
        getProfileByLoginId: async (loginId: string) => {
            const row = await deps.db.findByLoginId(loginId)
            return row ? toProfileWithDans(row) : null
        },
        getProfileById: async (id: string) => {
            const row = await deps.db.findById(id)
            return row ? toProfileWithDans(row) : null
        },
    }
}

export type PlayerService = ReturnType<typeof createPlayerService>

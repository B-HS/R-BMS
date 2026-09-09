import { toPlayerProfile, type PlayerRow } from '@server/service/domain/player/player.service'

export type RivalServiceDb = {
    findByLoginId: (loginId: string) => Promise<PlayerRow | null>
    findByLoginIds: (loginIds: string[]) => Promise<PlayerRow[]>
    listRivalRows: (userId: string) => Promise<(PlayerRow & { plays: number })[]>
    replaceRivals: (userId: string, rivalIds: string[]) => Promise<void>
}

export const createRivalService = (deps: { db: RivalServiceDb }) => {
    const listProfiles = async (userId: string) => {
        const rows = await deps.db.listRivalRows(userId)
        return rows.map((row) => toPlayerProfile(row, row.plays))
    }

    return {
        listByLoginId: async (loginId: string) => {
            const owner = await deps.db.findByLoginId(loginId)
            if (!owner) return null
            return listProfiles(owner.id)
        },

        replaceByUserId: async (params: { userId: string; rivalLoginIds: string[] }) => {
            const resolved = await deps.db.findByLoginIds(params.rivalLoginIds)
            const rivalIds = resolved.filter((row) => row.id !== params.userId).map((row) => row.id)
            await deps.db.replaceRivals(params.userId, rivalIds)
            return listProfiles(params.userId)
        },
    }
}

export type RivalService = ReturnType<typeof createRivalService>

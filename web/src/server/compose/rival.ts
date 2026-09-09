import { count, eq, inArray } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { rival, score } from '@server/db/schema'
import { createRivalService } from '@server/service/domain/player/rival.service'
import type { PlayerRow } from '@server/service/domain/player/player.service'

const playerSelection = {
    id: user.id,
    loginId: user.loginId,
    name: user.name,
    rank: user.rank,
    rankPoints: user.rankPoints,
    totalPlays: user.totalPlays,
    role: user.role,
}

export const composeRival = (db: Database) => {
    const countPlaysByUser = async (userIds: string[]) => {
        if (userIds.length === 0) return new Map<string, number>()
        const rows = await db.select({ userId: score.userId, value: count() }).from(score).where(inArray(score.userId, userIds)).groupBy(score.userId)
        return new Map(rows.flatMap((row) => (row.userId ? [[row.userId, row.value] as const] : [])))
    }

    const rivalService = createRivalService({
        db: {
            findByLoginId: async (loginId) => {
                const [row] = await db.select(playerSelection).from(user).where(eq(user.loginId, loginId)).limit(1)
                return row ?? null
            },
            findByLoginIds: async (loginIds) => {
                if (loginIds.length === 0) return []
                return db.select(playerSelection).from(user).where(inArray(user.loginId, loginIds))
            },
            listRivalRows: async (userId) => {
                const rows = await db.select(playerSelection).from(rival).innerJoin(user, eq(user.id, rival.rivalId)).where(eq(rival.userId, userId))
                const plays = await countPlaysByUser(rows.map((row: PlayerRow) => row.id))
                return rows.map((row: PlayerRow) => ({ ...row, plays: plays.get(row.id) ?? 0 }))
            },
            replaceRivals: async (userId, rivalIds) =>
                db.transaction(async (tx) => {
                    await tx.delete(rival).where(eq(rival.userId, userId))
                    if (rivalIds.length === 0) return
                    await tx.insert(rival).values(rivalIds.map((rivalId) => ({ userId, rivalId })))
                }),
        },
    })

    const findUserIdByLoginId = async (loginId: string) => {
        const [row] = await db.select({ id: user.id }).from(user).where(eq(user.loginId, loginId)).limit(1)
        return row?.id ?? null
    }

    return { rivalService, findUserIdByLoginId }
}

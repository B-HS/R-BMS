import { and, desc, eq } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { replay, score } from '@server/db/schema'
import { getEnv } from '@server/lib/env'
import { newId } from '@server/lib/token'
import { createReplayService, type ReplayRow } from '@server/service/domain/replay/replay.service'
import { createMysqlReplayStorage } from '@server/service/shared/storage/replay-storage'

const replaySelection = {
    id: replay.id,
    userId: replay.userId,
    loginId: user.loginId,
    playerName: user.name,
    chartSha256: replay.chartSha256,
    scoreId: replay.scoreId,
    format: replay.format,
    mode: replay.mode,
    random: replay.random,
    randomP2: replay.randomP2,
    seed: replay.seed,
    lntype: replay.lntype,
    offsetMs: replay.offsetMs,
    judgeRate: replay.judgeRate,
    scratchAuto: replay.scratchAuto,
    constant: replay.constant,
    gauge: replay.gauge,
    clientBuildSha256: replay.clientBuildSha256,
    eventCount: replay.eventCount,
    durationUs: replay.durationUs,
    size: replay.size,
    storageKey: replay.storageKey,
    createdAt: replay.createdAt,
}

const toReplayRow = (row: Omit<ReplayRow, 'data' | 'createdAt'> & { createdAt: Date }, data: string | null): ReplayRow => ({
    ...row,
    data,
    createdAt: row.createdAt.getTime(),
})

const createConfiguredReplayStorage = () => {
    const backend = getEnv().REPLAY_STORAGE
    if (backend !== 'db') throw new Error(`REPLAY_STORAGE=${backend} is not implemented; only 'db' is supported`)
    return createMysqlReplayStorage()
}

export const composeReplay = (db: Database) => {
    const storage = createConfiguredReplayStorage()

    const replayService = createReplayService({
        newId,
        storage,
        db: {
            insertReplay: async (row) => {
                await db.insert(replay).values(row)
            },
            findReplayById: async (id) => {
                const [row] = await db
                    .select({ ...replaySelection, data: replay.data })
                    .from(replay)
                    .leftJoin(user, eq(user.id, replay.userId))
                    .where(eq(replay.id, id))
                    .limit(1)
                if (!row) return null
                const { data, ...rest } = row
                return toReplayRow(rest, data)
            },
            listReplaysByChart: async ({ chartSha256, userId, limit }) => {
                const rows = await db
                    .select(replaySelection)
                    .from(replay)
                    .leftJoin(user, eq(user.id, replay.userId))
                    .where(and(eq(replay.chartSha256, chartSha256), userId ? eq(replay.userId, userId) : undefined))
                    .orderBy(desc(replay.createdAt))
                    .limit(limit)
                return rows.map((row) => toReplayRow(row, null))
            },
            linkScoreReplay: async ({ scoreId, userId, replayId }) => {
                await db
                    .update(score)
                    .set({ replayId })
                    .where(and(eq(score.id, scoreId), eq(score.userId, userId)))
            },
        },
    })

    return { replayService, replayStorage: storage }
}

export type ComposedReplay = ReturnType<typeof composeReplay>

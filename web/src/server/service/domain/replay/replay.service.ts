import { gaugeTypeToId } from '@server/dto/common'
import type { ReplayEventInput, ReplayUploadInput } from '@server/dto/replay'
import type { ReplayStorage } from '@server/service/shared/storage/replay-storage'

export type ReplayInsertRow = {
    id: string
    userId: string | null
    chartSha256: string
    scoreId: string | null
    format: string
    mode: string
    random: string
    randomP2: string | null
    seed: number
    lntype: number
    offsetMs: number
    judgeRate: number
    scratchAuto: boolean
    constant: boolean
    gauge: number
    clientBuildSha256: string | null
    eventCount: number
    durationUs: number
    size: number
    storageKey: string | null
    data: string | null
}

export type ReplayRow = ReplayInsertRow & {
    loginId: string | null
    playerName: string | null
    createdAt: number
}

export type ReplayServiceDb = {
    insertReplay: (row: ReplayInsertRow) => Promise<void>
    findReplayById: (id: string) => Promise<ReplayRow | null>
    listReplaysByChart: (params: { chartSha256: string; userId?: string; limit: number }) => Promise<ReplayRow[]>
    linkScoreReplay: (params: { scoreId: string; userId: string; replayId: string }) => Promise<void>
}

export type ReplayServiceDeps = {
    db: ReplayServiceDb
    storage: ReplayStorage
    newId: (prefix: string) => string
}

export const replayUrl = (id: string) => `/api/replays/${id}`

export const measureDurationUs = (events: ReplayEventInput[]) => {
    if (events.length === 0) return 0
    let earliest = events[0].t_us
    let latest = events[0].t_us
    for (const event of events) {
        if (event.t_us < earliest) earliest = event.t_us
        if (event.t_us > latest) latest = event.t_us
    }
    return latest - earliest
}

export const toReplayMeta = (row: ReplayRow) => ({
    id: row.id,
    url: replayUrl(row.id),
    player: { id: row.loginId ?? '' },
    player_name: row.playerName ?? '',
    chart_sha256: row.chartSha256,
    score_id: row.scoreId,
    format: row.format,
    mode: row.mode,
    seed: row.seed,
    lntype: row.lntype,
    event_count: row.eventCount,
    duration_us: row.durationUs,
    size: row.size,
    client_build_sha256: row.clientBuildSha256,
    created_at: row.createdAt,
})

export const toReplayData = (row: ReplayRow, events: ReplayEventInput[]) => ({
    api_version: 1,
    id: row.id,
    format: row.format,
    chart: { md5: '', sha256: row.chartSha256 },
    score_id: row.scoreId,
    mode: row.mode,
    random: row.random,
    random_p2: row.randomP2,
    seed: row.seed,
    lntype: row.lntype,
    offset_ms: row.offsetMs,
    judge_rate: row.judgeRate,
    scratch_auto: row.scratchAuto,
    constant: row.constant,
    client_build_sha256: row.clientBuildSha256,
    events,
    event_count: row.eventCount,
    duration_us: row.durationUs,
    size: row.size,
    extra: {},
})

const parseEvents = (content: string | null): ReplayEventInput[] => {
    if (!content) return []
    const parsed: unknown = JSON.parse(content)
    return Array.isArray(parsed) ? (parsed as ReplayEventInput[]) : []
}

export const createReplayService = (deps: ReplayServiceDeps) => ({
    upload: async (params: { input: ReplayUploadInput; chartSha256: string; userId: string }) => {
        const { input } = params
        const id = deps.newId('rp_')
        const content = JSON.stringify(input.events)
        const payload = await deps.storage.save({ id, content })
        const row: ReplayInsertRow = {
            id,
            userId: params.userId,
            chartSha256: params.chartSha256,
            scoreId: input.score_id ?? null,
            format: input.format,
            mode: input.mode,
            random: input.random ?? 'Off',
            randomP2: input.random_p2 ?? null,
            seed: input.seed ?? 0,
            lntype: input.lntype,
            offsetMs: input.offset_ms,
            judgeRate: input.judge_rate,
            scratchAuto: input.scratch_auto,
            constant: input.constant,
            gauge: input.gauge ? gaugeTypeToId(input.gauge) : 0,
            clientBuildSha256: input.client_build_sha256 ?? null,
            eventCount: input.event_count ?? input.events.length,
            durationUs: input.duration_us ?? measureDurationUs(input.events),
            size: input.size ?? content.length,
            storageKey: payload.storageKey,
            data: payload.inline,
        }
        await deps.db.insertReplay(row)
        if (input.score_id) {
            await deps.db.linkScoreReplay({ scoreId: input.score_id, userId: params.userId, replayId: id })
        }
        return { id, url: replayUrl(id), event_count: row.eventCount }
    },

    getById: async (id: string) => {
        const row = await deps.db.findReplayById(id)
        if (!row) return null
        const content = await deps.storage.load({ storageKey: row.storageKey, inline: row.data })
        return toReplayData(row, parseEvents(content))
    },

    listByChart: async (params: { chartSha256: string; userId?: string; limit: number }) => {
        const rows = await deps.db.listReplaysByChart(params)
        return rows.map(toReplayMeta)
    },
})

export type ReplayService = ReturnType<typeof createReplayService>

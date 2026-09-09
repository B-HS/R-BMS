import { isMd5Hash, isSha256Hash } from '@server/dto/common'
import type { ChartMetaInput } from '@server/dto/chart'

export type ChartRow = {
    sha256: string
    md5: string | null
    title: string
    subtitle: string
    genre: string
    artist: string
    subartist: string
    level: number | null
    total: number | null
    mode: string
    lntype: number
    judge: number
    minbpm: number
    maxbpm: number
    notes: number
    hasLn: boolean
    hasCn: boolean
    hasHcn: boolean
    hasMine: boolean
    hasRandom: boolean
    hasStop: boolean
    url: string | null
    appendurl: string | null
    extra: unknown
}

export type ChartServiceDb = {
    findBySha256: (sha256: string) => Promise<ChartRow | null>
    findByMd5: (md5: string) => Promise<ChartRow | null>
    upsert: (row: Omit<ChartRow, 'extra'> & { extra: Record<string, unknown> }) => Promise<void>
}

export type ChartHashKind = 'md5' | 'sha256' | 'unknown'

export const classifyChartHash = (hash: string): ChartHashKind => {
    if (isMd5Hash(hash)) return 'md5'
    if (isSha256Hash(hash)) return 'sha256'
    return 'unknown'
}

export const toChartMeta = (row: ChartRow): ChartMetaInput => ({
    md5: row.md5 ?? '',
    sha256: row.sha256,
    title: row.title,
    subtitle: row.subtitle,
    genre: row.genre,
    artist: row.artist,
    subartist: row.subartist,
    level: row.level,
    total: row.total,
    mode: row.mode,
    lntype: row.lntype,
    judge: row.judge,
    minbpm: row.minbpm,
    maxbpm: row.maxbpm,
    notes: row.notes,
    has_ln: row.hasLn,
    has_cn: row.hasCn,
    has_hcn: row.hasHcn,
    has_mine: row.hasMine,
    has_random: row.hasRandom,
    has_stop: row.hasStop,
    url: row.url,
    appendurl: row.appendurl,
    extra: (row.extra as Record<string, unknown>) ?? {},
})

export const createChartService = (deps: { db: ChartServiceDb }) => {
    const findByHash = async (hash: string) => {
        const kind = classifyChartHash(hash)
        if (kind === 'md5') return deps.db.findByMd5(hash)
        if (kind === 'sha256') return deps.db.findBySha256(hash)
        return null
    }

    const upsertFromMeta = async (meta: ChartMetaInput) => {
        const existing = meta.sha256 ? await deps.db.findBySha256(meta.sha256) : meta.md5 ? await deps.db.findByMd5(meta.md5) : null
        const sha256 = meta.sha256 || existing?.sha256
        if (!sha256) return null
        await deps.db.upsert({
            sha256,
            md5: meta.md5 || existing?.md5 || null,
            title: meta.title,
            subtitle: meta.subtitle,
            genre: meta.genre,
            artist: meta.artist,
            subartist: meta.subartist,
            level: meta.level ?? null,
            total: meta.total ?? null,
            mode: meta.mode,
            lntype: meta.lntype,
            judge: meta.judge,
            minbpm: meta.minbpm,
            maxbpm: meta.maxbpm,
            notes: meta.notes,
            hasLn: meta.has_ln,
            hasCn: meta.has_cn,
            hasHcn: meta.has_hcn,
            hasMine: meta.has_mine,
            hasRandom: meta.has_random,
            hasStop: meta.has_stop,
            url: meta.url ?? null,
            appendurl: meta.appendurl ?? null,
            extra: meta.extra,
        })
        return deps.db.findBySha256(sha256)
    }

    return {
        findByHash,
        getMetaByHash: async (hash: string) => {
            const row = await findByHash(hash)
            return row ? toChartMeta(row) : null
        },
        upsertFromMeta,
        ensureFromSubmission: async (params: { md5: string; sha256: string; title: string; mode: string; notes: number; lntype: number }) => {
            const existing = params.sha256 ? await deps.db.findBySha256(params.sha256) : params.md5 ? await deps.db.findByMd5(params.md5) : null
            if (existing) {
                if (!existing.md5 && params.md5) {
                    await deps.db.upsert({ ...existing, md5: params.md5, extra: (existing.extra as Record<string, unknown>) ?? {} })
                    return { sha256: existing.sha256, md5: params.md5 }
                }
                return { sha256: existing.sha256, md5: existing.md5 }
            }
            if (!params.sha256) return null
            await deps.db.upsert({
                sha256: params.sha256,
                md5: params.md5 || null,
                title: params.title,
                subtitle: '',
                genre: '',
                artist: '',
                subartist: '',
                level: null,
                total: null,
                mode: params.mode,
                lntype: params.lntype,
                judge: 0,
                minbpm: 0,
                maxbpm: 0,
                notes: params.notes,
                hasLn: false,
                hasCn: false,
                hasHcn: false,
                hasMine: false,
                hasRandom: false,
                hasStop: false,
                url: null,
                appendurl: null,
                extra: {},
            })
            return { sha256: params.sha256, md5: params.md5 || null }
        },
    }
}

export type ChartService = ReturnType<typeof createChartService>

import { createHash } from 'node:crypto'
import type { AiProviderId, AiUsage } from '@server/dto/ai'

export const AI_RESULT_ID_PREFIX = 'air_'

export type AiResultKey = { userId: string; feature: string; subjectKey: string }

export type AiResultRow = {
    id: string
    feature: string
    subjectKey: string
    inputHash: string
    analysisVersion: number
    promptVersion: number
    provider: AiProviderId
    model: string
    output: unknown
    usage: AiUsage
    createdAt: number
}

export type AiResultServiceDb = {
    findByInput: (params: AiResultKey & { inputHash: string }) => Promise<AiResultRow | null>
    findLatest: (params: AiResultKey) => Promise<AiResultRow | null>
    /** Stores one result; a second save for the same user, feature, subject and input hash replaces the first. */
    upsert: (row: AiResultRow & { userId: string }) => Promise<void>
}

type AiResultServiceDeps = {
    db: AiResultServiceDb
    newId: (prefix: string) => string
    now: () => number
}

const canonicalize = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(canonicalize)
    if (value === null || typeof value !== 'object') return value
    return Object.fromEntries(
        Object.entries(value)
            .toSorted(([left], [right]) => (left < right ? -1 : 1))
            .map(([key, entry]) => [key, canonicalize(entry)]),
    )
}

/** SHA-256 (hex) of a value's JSON with object keys sorted, so the same input always gives the same cache key. */
export const hashAiInput = (value: unknown) =>
    createHash('sha256')
        .update(JSON.stringify(canonicalize(value)))
        .digest('hex')

export const createAiResultService = (deps: AiResultServiceDeps) => ({
    findCached: (params: AiResultKey & { inputHash: string }) => deps.db.findByInput(params),

    findLatest: (params: AiResultKey) => deps.db.findLatest(params),

    save: async (params: AiResultKey & Omit<AiResultRow, 'id' | 'feature' | 'subjectKey' | 'createdAt'>) => {
        const row = { ...params, id: deps.newId(AI_RESULT_ID_PREFIX), createdAt: deps.now() }
        await deps.db.upsert(row)
        return row
    },
})

export type AiResultService = ReturnType<typeof createAiResultService>

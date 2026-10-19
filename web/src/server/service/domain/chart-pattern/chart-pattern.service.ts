import { createHash } from 'node:crypto'
import {
    CHART_PATTERN_READ_RECOMPUTE_MAX,
    CHART_PATTERN_UPLOAD_OUTCOME,
    chartPatternDocumentSchema,
    type ChartPatternDocument,
    type ChartPatternRecomputeInput,
} from '@server/dto/chart-pattern'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { computeChartFeatures, type ChartFeatures } from '@server/service/domain/analysis/chart-features'
import { gunzipJson, gzipJsonText } from '@server/service/shared/storage/gzip-json'

export type ChartPatternMeta = { parserVersion: number; contentHash: string; featuresVersion: number }

export type ChartPatternRow = {
    md5: string
    sha256: string
    mode: string
    formatVersion: number
    parserVersion: number
    playableNotes: number
    noteCount: number
    contentHash: string
    byteSize: number
    data: Buffer
    features: ChartFeatures
    featuresVersion: number
    uploadedBy: string
    updatedAt: number
}

export type ChartPatternServiceDb = {
    findMeta: (md5: string) => Promise<ChartPatternMeta | null>
    /** Inserts the row unless one with its md5 is there; false when it was. */
    insertIfAbsent: (row: ChartPatternRow) => Promise<boolean>
    replace: (row: ChartPatternRow) => Promise<void>
    updateFeatures: (update: Pick<ChartPatternRow, 'md5' | 'features' | 'featuresVersion' | 'updatedAt'>) => Promise<void>
    findStored: (md5: string) => Promise<{ data: Buffer; features: unknown; featuresVersion: number } | null>
    /** The stored features of the given charts, without their layout. A chart with no pattern is absent. */
    listFeatures: (md5s: readonly string[]) => Promise<{ md5: string; features: unknown; featuresVersion: number }[]>
    listPresentMd5s: (md5s: readonly string[]) => Promise<string[]>
    countAll: () => Promise<number>
    countFeaturesBelow: (version: number) => Promise<number>
    /** The md5s of charts with features older than `version`, in md5 order, after the md5 `after` (none to start), at most `limit`. */
    listStaleMd5s: (params: { version: number; after: string | null; limit: number }) => Promise<string[]>
}

export type CoverageTable = { id: string; name: string; md5s: readonly string[] }

export type ChartPatternServiceDeps = {
    db: ChartPatternServiceDb
    now: () => number
    listCoverageTables: () => readonly CoverageTable[]
}

const contentHashOf = (text: string) => createHash('sha256').update(text).digest('hex')

const isSameContent = (meta: ChartPatternMeta, document: ChartPatternDocument, contentHash: string) =>
    meta.parserVersion === document.parser_version && meta.contentHash === contentHash

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null

const isCurrentFeatures = (value: unknown): value is ChartFeatures => isRecord(value) && value.analysis_version === ANALYSIS_VERSION

const hasTagShares = (value: unknown): value is ChartFeatures =>
    isRecord(value) && isRecord(value.tags) && Object.values(value.tags).every((share) => isRecord(share) && typeof share.ratio === 'number')

/** Features of an older analysis version, used as they are while the chart waits to be computed again. */
export type ChartFeaturesRead = { features: ChartFeatures; isStale: boolean }

/**
 * Keeps the note layout of each chart (`chart_pattern`) for the server's own analysis. The layout is stored gzip-compressed and is
 * never sent out: no route reads it back, and only the features computed from it reach a screen or an AI digest.
 */
export const createChartPatternService = (deps: ChartPatternServiceDeps) => {
    const toRow = async (document: ChartPatternDocument, text: string, contentHash: string, uploadedBy: string) =>
        ({
            md5: document.md5,
            sha256: document.sha256,
            mode: document.mode,
            formatVersion: document.version,
            parserVersion: document.parser_version,
            playableNotes: document.playable_notes,
            noteCount: document.notes.time_us.length,
            contentHash,
            byteSize: Buffer.byteLength(text, 'utf8'),
            data: await gzipJsonText(text),
            features: computeChartFeatures(document),
            featuresVersion: ANALYSIS_VERSION,
            uploadedBy,
            updatedAt: deps.now(),
        }) satisfies ChartPatternRow

    const keepUnchanged = async (meta: ChartPatternMeta, document: ChartPatternDocument) => {
        const isStale = meta.featuresVersion < ANALYSIS_VERSION
        if (isStale) {
            await deps.db.updateFeatures({
                md5: document.md5,
                features: computeChartFeatures(document),
                featuresVersion: ANALYSIS_VERSION,
                updatedAt: deps.now(),
            })
        }
        return { outcome: CHART_PATTERN_UPLOAD_OUTCOME.UNCHANGED, featuresRecomputed: isStale }
    }

    const store = async (document: ChartPatternDocument, uploadedBy: string) => {
        const text = JSON.stringify(document)
        const contentHash = contentHashOf(text)
        const meta = await deps.db.findMeta(document.md5)
        if (meta && isSameContent(meta, document, contentHash)) return keepUnchanged(meta, document)
        const row = await toRow(document, text, contentHash, uploadedBy)
        if (!meta && (await deps.db.insertIfAbsent(row))) return { outcome: CHART_PATTERN_UPLOAD_OUTCOME.CREATED, featuresRecomputed: true }
        const raced = meta ?? (await deps.db.findMeta(document.md5))
        if (raced && isSameContent(raced, document, contentHash)) return keepUnchanged(raced, document)
        await deps.db.replace(row)
        return { outcome: CHART_PATTERN_UPLOAD_OUTCOME.REPLACED, featuresRecomputed: true }
    }

    const decode = async (data: Buffer) => {
        const parsed = chartPatternDocumentSchema.safeParse(await gunzipJson(data))
        return parsed.success ? parsed.data : null
    }

    const computeAgain = async (md5: string, data: Buffer) => {
        const document = await decode(data)
        if (!document) return null
        const features = computeChartFeatures(document)
        await deps.db.updateFeatures({ md5, features, featuresVersion: ANALYSIS_VERSION, updatedAt: deps.now() })
        return features
    }

    const readPattern = async (md5: string) => {
        const stored = await deps.db.findStored(md5)
        return stored ? decode(stored.data) : null
    }

    const readFeatures = async (md5: string) => {
        const stored = await deps.db.findStored(md5)
        if (!stored) return null
        if (stored.featuresVersion === ANALYSIS_VERSION && isCurrentFeatures(stored.features)) return stored.features
        return computeAgain(md5, stored.data)
    }

    return {
        /**
         * Stores one chart's pattern. The same md5 with the same parser version and content changes nothing (`unchanged`, though
         * features of an older analysis version are computed again); any other difference, a new parser version above all,
         * replaces the stored pattern and its features (`replaced`).
         */
        upload: async (params: { document: ChartPatternDocument; uploadedBy: string }) => {
            const result = await store(params.document, params.uploadedBy)
            return {
                md5: params.document.md5,
                outcome: result.outcome,
                parser_version: params.document.parser_version,
                features_version: ANALYSIS_VERSION,
                features_recomputed: result.featuresRecomputed,
                note_count: params.document.notes.time_us.length,
            }
        },

        /** How many charts of each difficulty table have a pattern, and which md5s are missing. Counts and table md5s only. */
        coverage: async () => {
            const tables = await Promise.all(
                deps.listCoverageTables().map(async (table) => {
                    const present = new Set(await deps.db.listPresentMd5s(table.md5s))
                    return {
                        table_id: table.id,
                        name: table.name,
                        table_md5s: table.md5s.length,
                        covered: present.size,
                        missing: table.md5s.filter((md5) => !present.has(md5)),
                    }
                }),
            )
            const [patterns, staleFeatures] = await Promise.all([deps.db.countAll(), deps.db.countFeaturesBelow(ANALYSIS_VERSION)])
            return { tables, patterns_total: patterns, stale_features: staleFeatures, analysis_version: ANALYSIS_VERSION }
        },

        /** The stored layout, for the server's analysis only. Never hand this to a route. */
        readPattern,

        /** The chart's features at the current analysis version, computed again from the stored layout when they are older. */
        readFeatures,

        /**
         * The features of several charts by md5, for the score AI features (5, 6, 7), without the layout. A read never computes
         * every stale chart: the first CHART_PATTERN_READ_RECOMPUTE_MAX charts (in the order asked) whose features are of an older
         * analysis version are computed again and stored, and the rest are returned as they are with `isStale`. An older feature
         * set that cannot be read as tag shares is left out. `recomputeStale` brings the rest up to date.
         */
        readFeaturesMany: async (md5s: readonly string[]) => {
            const asked = [...new Set(md5s)]
            if (asked.length === 0) return new Map<string, ChartFeaturesRead>()
            const found = new Map((await deps.db.listFeatures(asked)).map((row) => [row.md5, row]))
            const rows = asked.flatMap((md5) => found.get(md5) ?? [])
            const isCurrent = (row: (typeof rows)[number]) => row.featuresVersion === ANALYSIS_VERSION && isCurrentFeatures(row.features)
            const toCompute = new Set(
                rows
                    .filter((row) => !isCurrent(row))
                    .slice(0, CHART_PATTERN_READ_RECOMPUTE_MAX)
                    .map((row) => row.md5),
            )
            const read = async (row: (typeof rows)[number]) => {
                if (isCurrent(row) && isCurrentFeatures(row.features)) return { features: row.features, isStale: false }
                const computed = toCompute.has(row.md5) ? await readFeatures(row.md5) : null
                if (computed) return { features: computed, isStale: false }
                return hasTagShares(row.features) ? { features: row.features, isStale: true } : null
            }
            const entries = await Promise.all(rows.map(async (row) => [row.md5, await read(row)] as const))
            return new Map(entries.flatMap(([md5, value]): [string, ChartFeaturesRead][] => (value ? [[md5, value]] : [])))
        },

        /**
         * Computes again one batch of charts whose features are older than the current analysis version, in md5 order after
         * `after`, one at a time. A chart whose stored layout cannot be read is listed in `skipped` and left as it was.
         * `next_after` is the last md5 examined while the batch was full (pass it as `after` for the next batch) and null on
         * the last batch, so a chart that cannot be computed never makes the walk stop or repeat.
         */
        recomputeStale: async (input: ChartPatternRecomputeInput) => {
            const md5s = await deps.db.listStaleMd5s({ version: ANALYSIS_VERSION, after: input.after, limit: input.limit })
            const skipped: string[] = []
            let recomputed = 0
            for (const md5 of md5s) {
                const stored = await deps.db.findStored(md5)
                const features = stored ? await computeAgain(md5, stored.data).catch(() => null) : null
                if (features) recomputed += 1
                else skipped.push(md5)
            }
            return {
                analysis_version: ANALYSIS_VERSION,
                examined: md5s.length,
                recomputed,
                skipped,
                next_after: md5s.length === input.limit ? (md5s.at(-1) ?? null) : null,
                remaining: await deps.db.countFeaturesBelow(ANALYSIS_VERSION),
            }
        },
    }
}

export type ChartPatternService = ReturnType<typeof createChartPatternService>

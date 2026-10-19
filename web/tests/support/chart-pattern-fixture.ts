import type { ChartPatternDocument, ChartPatternTempoPoint } from '@server/dto/chart-pattern'
import type { ChartPatternMeta, ChartPatternRow, ChartPatternServiceDb } from '@server/service/domain/chart-pattern/chart-pattern.service'

export const FIXTURE_MD5 = '0123456789abcdef0123456789abcdef'
export const FIXTURE_SHA256 = 'f'.repeat(64)
export const FIXTURE_BPM = 150

/** A note as `[time_us, lane, kind]`; kind 0 normal, 1 LN head, 2 LN end, 3 mine. */
export type FixtureNote = readonly [number, number, number]

/** A hand-made pattern document in the exact shape `rbms-cli chart-pattern` writes, with the notes put in time and lane order. */
export const makePatternDocument = (
    notes: readonly FixtureNote[],
    overrides: Partial<Omit<ChartPatternDocument, 'notes'>> & { tempo?: ChartPatternTempoPoint[] } = {},
): ChartPatternDocument => {
    const ordered = notes.toSorted((a, b) => a[0] - b[0] || a[1] - b[1])
    const { tempo = [], ...header } = overrides
    return {
        format: 'rbms-chart-pattern',
        version: 1,
        parser_version: 5,
        md5: FIXTURE_MD5,
        sha256: FIXTURE_SHA256,
        mode: 'BEAT_7K',
        declared_ln: null,
        has_undefined_ln: false,
        uses_control_flow: false,
        init_bpm: FIXTURE_BPM,
        total: 300,
        rank: 2,
        play_level: 12,
        playable_notes: ordered.filter((note) => note[2] !== 3 && note[2] !== 2).length,
        ...header,
        notes: {
            time_us: ordered.map((note) => note[0]),
            lane: ordered.map((note) => note[1]),
            kind: ordered.map((note) => note[2]),
            section: ordered.map((note) => note[0] / 1_600_000),
        },
        tempo,
    }
}

/** An in-memory `chart_pattern` table that behaves like the compose implementation, including the insert race. */
export const createMemoryChartPatternDb = () => {
    const rows = new Map<string, ChartPatternRow>()
    const calls = { insert: 0, replace: 0, updateFeatures: 0 }
    const rawFeatures = new Map<string, unknown>()
    const featuresOf = (row: ChartPatternRow) => (rawFeatures.has(row.md5) ? rawFeatures.get(row.md5) : row.features)
    const hooks: { beforeInsert: ((row: ChartPatternRow) => void) | null } = { beforeInsert: null }
    const toMeta = (row: ChartPatternRow): ChartPatternMeta => ({
        parserVersion: row.parserVersion,
        contentHash: row.contentHash,
        featuresVersion: row.featuresVersion,
    })
    const db: ChartPatternServiceDb = {
        findMeta: async (md5) => {
            const row = rows.get(md5)
            return row ? toMeta(row) : null
        },
        insertIfAbsent: async (row) => {
            hooks.beforeInsert?.(row)
            calls.insert += 1
            if (rows.has(row.md5)) return false
            rows.set(row.md5, row)
            return true
        },
        replace: async (row) => {
            calls.replace += 1
            rows.set(row.md5, { ...row, uploadedBy: row.uploadedBy })
        },
        updateFeatures: async ({ md5, features, featuresVersion, updatedAt }) => {
            calls.updateFeatures += 1
            const row = rows.get(md5)
            rawFeatures.delete(md5)
            if (row) rows.set(md5, { ...row, features, featuresVersion, updatedAt })
        },
        findStored: async (md5) => {
            const row = rows.get(md5)
            return row ? { data: row.data, features: featuresOf(row), featuresVersion: row.featuresVersion } : null
        },
        listFeatures: async (md5s) =>
            md5s.flatMap((md5) => {
                const row = rows.get(md5)
                return row ? [{ md5, features: featuresOf(row), featuresVersion: row.featuresVersion }] : []
            }),
        listPresentMd5s: async (md5s) => md5s.filter((md5) => rows.has(md5)),
        countAll: async () => rows.size,
        countFeaturesBelow: async (version) => [...rows.values()].filter((row) => row.featuresVersion < version).length,
        listStaleMd5s: async ({ version, after, limit }) =>
            [...rows.values()]
                .filter((row) => row.featuresVersion < version && (after === null || row.md5 > after))
                .map((row) => row.md5)
                .toSorted()
                .slice(0, limit),
    }
    return { db, rows, calls, hooks, rawFeatures }
}

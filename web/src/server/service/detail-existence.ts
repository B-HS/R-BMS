import { classifyChartHash } from '@server/service/domain/chart/chart.service'

export const DETAIL_ROUTE_KIND = { chart: 'charts', player: 'players', course: 'courses', replay: 'replays' } as const

export type DetailKind = keyof typeof DETAIL_ROUTE_KIND

export type DetailExistenceDeps = Record<DetailKind, (id: string) => Promise<boolean>>

const DETAIL_PATH = new RegExp(`^/(${Object.values(DETAIL_ROUTE_KIND).join('|')})/([^/]+)/?$`)

const KIND_BY_SEGMENT = Object.fromEntries(Object.entries(DETAIL_ROUTE_KIND).map(([kind, segment]) => [segment, kind])) as Record<
    (typeof DETAIL_ROUTE_KIND)[DetailKind],
    DetailKind
>

export const parseDetailPath = (pathname: string) => {
    const match = DETAIL_PATH.exec(pathname)
    if (!match) return null
    try {
        return { kind: KIND_BY_SEGMENT[match[1] as keyof typeof KIND_BY_SEGMENT], id: decodeURIComponent(match[2]) }
    } catch {
        return null
    }
}

/**
 * Answers whether the page a detail address names has anything behind it: `true` or `false` for a chart, player, course or
 * replay page, and `null` for any other address or when the lookup itself fails, so a broken database never turns a good page
 * into a 404.
 */
export const createDetailExistenceCheck = (resolveDeps: () => DetailExistenceDeps | Promise<DetailExistenceDeps>) => async (pathname: string) => {
    const target = parseDetailPath(pathname)
    if (!target) return null
    if (target.kind === 'chart' && classifyChartHash(target.id) === 'unknown') return false
    try {
        const deps = await resolveDeps()
        return await deps[target.kind](target.id)
    } catch {
        return null
    }
}

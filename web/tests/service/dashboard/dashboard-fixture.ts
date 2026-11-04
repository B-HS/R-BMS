import type { CourseCatalogOutput } from '@server/dto/course-catalog'
import { createDashboardService, type DashboardServiceDb, type DashboardServiceDeps } from '@server/service/domain/dashboard/dashboard.service'

export const MINUTE_MS = 60_000
export const HOUR_MS = 3_600_000
export const DAY_MS = 86_400_000

/** 00:00 KST on Thursday 2026-11-05. The week of that day starts on Monday 2026-11-02. */
export const TODAY_START = Date.UTC(2026, 10, 4, 15, 0, 0)

export const NOON_OFFSET_MS = 12 * HOUR_MS

/** The instant `minutes` after 12:00 KST on the day `daysAgo` days before `TODAY_START`'s day. */
export const noonOf = (daysAgo: number, minutes = 0) => TODAY_START - daysAgo * DAY_MS + NOON_OFFSET_MS + minutes * MINUTE_MS

export const EMPTY_CATALOG: CourseCatalogOutput = { groups: [] }

export const emptyDb = (): DashboardServiceDb => ({
    countChartBests: async () => ({ played: 0, cleared: 0, hardOrBetter: 0, fullComboOrBetter: 0 }),
    listGradeInputs: async () => [],
    summariseScores: async () => ({ total: 0, lastPlayedAt: null }),
    listPlayedAts: async () => ({ playedAts: [], previousPlayedAt: null }),
    listStandings: async () => [],
    listCourseBests: async () => [],
    listRecentCourseScores: async () => [],
    listRivals: async () => [],
    listRecentRivalActivity: async () => [],
    listVersus: async () => [],
    listRecentReplays: async () => [],
})

export const makeDashboardService = (overrides: { db?: Partial<DashboardServiceDb> } & Partial<Omit<DashboardServiceDeps, 'db'>> = {}) => {
    const { db, ...rest } = overrides
    return createDashboardService({
        db: { ...emptyDb(), ...db },
        tableSnapshotService: { getDetail: async () => null, getRecords: async () => null },
        listCatalogGroups: async () => EMPTY_CATALOG,
        ...rest,
    })
}

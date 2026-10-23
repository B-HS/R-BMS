import type { ClearLamp } from '@server/dto/common'
import type { TableRecordOutput } from '@server/dto/table-snapshot'
import { createAiGoalService, type AiGoalRow, type AiGoalServiceDb } from '@server/service/domain/ai/goal-plan.service'
import { createAiPracticeFolderService, type AiPracticeFolderServiceDb } from '@server/service/domain/ai/practice-folder.service'
import { createAiRecommendService, type TableFeatureDeps } from '@server/service/domain/ai/recommend.service'
import type { ChartTagShares, ProgressRecords, ProgressTable } from '@server/service/domain/ai/table-progress'
import { createFolderService } from '@server/service/domain/folder/folder.service'
import { FAKE_USAGE, createFakeAdapter, createMemoryAiServices, testKekEnv, type FakeAdapterOptions } from './ai-memory-db'
import { createMemoryFolderDb } from './folder-memory-db'

export const SCORE_USER = 'user-alice'
export const SCORE_API_KEY = 'sk-ant-api03-score-feature-secret-QRST'
export const SCORE_NOW = Date.UTC(2026, 9, 21, 12)
export const NOTES = 1000

/** A lowercase md5 made from a small number, unique per number. */
export const md5Of = (value: number) => value.toString(16).padStart(32, '0')

/** Levels 1 to 6, `perLevel` charts each, md5 = level × 100 + index. */
export const makeTable = (perLevel = 4): ProgressTable => {
    const levels = ['1', '2', '3', '4', '5', '6']
    return {
        id: 'insane1',
        name: '発狂BMS難易度表',
        symbol: '★',
        levels,
        entries: levels.flatMap((level) =>
            Array.from({ length: perLevel }, (_, index) => ({
                md5: md5Of(Number(level) * 100 + index),
                level,
                title: `L${level}-${index}`,
                artist: 'artist',
            })),
        ),
    }
}

export const recordOf = (lamp: ClearLamp, exRatePercent = 80, playedAt = SCORE_NOW - 86_400_000, minbp = 10): TableRecordOutput => ({
    clear: lamp,
    assist_grade: 0,
    ex_score: Math.round((exRatePercent / 100) * NOTES * 2),
    minbp,
    max_combo: 0,
    total_notes: NOTES,
    played_at: playedAt,
    scratch_auto: false,
})

/** Records for whole levels: every chart of the level gets the lamp. */
export const levelRecords = (table: ProgressTable, lamps: Record<string, ClearLamp>) =>
    Object.fromEntries(table.entries.flatMap((entry) => (lamps[entry.level] ? [[entry.md5, recordOf(lamps[entry.level])]] : [])))

export const createMemoryGoalDb = () => {
    const goals = new Map<string, AiGoalRow & { userId: string; archivedAt: number | null }>()
    const db: AiGoalServiceDb = {
        listActiveGoals: async (userId) => [...goals.values()].filter((goal) => goal.userId === userId && goal.archivedAt === null),
        findActiveGoal: async ({ userId, goalId }) => {
            const goal = goals.get(goalId)
            return goal && goal.userId === userId && goal.archivedAt === null ? goal : null
        },
        insertGoal: async (row) => {
            goals.set(row.id, { ...row, archivedAt: null })
        },
        archiveGoal: async ({ userId, goalId, archivedAt }) => {
            const goal = goals.get(goalId)
            if (!goal || goal.userId !== userId || goal.archivedAt !== null) return false
            goals.set(goalId, { ...goal, archivedAt })
            return true
        },
    }
    return { db, goals }
}

export const createMemoryPracticeDb = () => {
    const links = new Map<string, string>()
    const db: AiPracticeFolderServiceDb = {
        findLinkedFolderId: async ({ userId, previewHash }) => links.get(`${userId}|${previewHash}`) ?? null,
        claimLink: async ({ userId, previewHash, folderId }) => {
            const key = `${userId}|${previewHash}`
            const existing = links.get(key)
            if (existing) return existing
            links.set(key, folderId)
            return folderId
        },
    }
    return { db, links }
}

type ScoreSetupOptions = {
    table?: ProgressTable
    records?: ProgressRecords
    tags?: ChartTagShares
    staleChartMd5s?: readonly string[]
    generate?: FakeAdapterOptions['generate']
    output?: unknown
    now?: () => number
}

/** The table-based AI services (features 5, 6 and 7) on memory databases and a fake provider. */
export const setupScoreFeatures = (options: ScoreSetupOptions = {}) => {
    const table = options.table ?? makeTable()
    const state = { records: options.records ?? {}, tags: options.tags ?? new Map() }
    const now = options.now ?? (() => SCORE_NOW)
    const anthropic = createFakeAdapter('anthropic', {
        validKeys: new Set([SCORE_API_KEY]),
        generate: options.generate ?? (() => ({ data: options.output ?? {}, usage: FAKE_USAGE })),
    })
    const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => testKekEnv('v1'), now })
    const tableDeps: TableFeatureDeps = {
        loadTable: (tableId) => (tableId === undefined || tableId === table.id ? table : null),
        loadRecords: async ({ tableId }) => (tableId === table.id ? state.records : null),
        loadChartTags: async (md5s) => ({
            shares: new Map([...state.tags].filter(([md5]) => md5s.includes(md5))),
            staleMd5s: new Set(options.staleChartMd5s?.filter((md5) => md5s.includes(md5))),
        }),
    }
    const shared = { runner: services.aiFeatureRunner, results: services.aiResultService }
    let counter = 0
    const newId = (prefix: string) => `${prefix}${(++counter).toString(16).padStart(30, '0')}`
    const folderMemory = createMemoryFolderDb()
    const folderService = createFolderService({
        db: folderMemory.db,
        newId,
        now,
        loadTableEntries: async () => null,
        lookupTableText: () => null,
    })
    const goalMemory = createMemoryGoalDb()
    const practiceMemory = createMemoryPracticeDb()
    const recommend = createAiRecommendService({ ...tableDeps, ...shared })
    const goals = createAiGoalService({ ...tableDeps, ...shared, db: goalMemory.db, newId, now })
    const practice = createAiPracticeFolderService({
        ...tableDeps,
        ...shared,
        db: practiceMemory.db,
        recommend,
        goals,
        folders: folderService,
        now,
    })
    return { ...services, anthropic, table, state, recommend, goals, practice, folderService, folderMemory, goalMemory, practiceMemory }
}

export const registerScoreKey = async (context: Pick<ReturnType<typeof setupScoreFeatures>, 'aiCredentialService'>) => {
    await context.aiCredentialService.register({ userId: SCORE_USER, provider: 'anthropic', apiKey: SCORE_API_KEY })
    await context.aiCredentialService.setModel({ userId: SCORE_USER, provider: 'anthropic', model: 'claude-test' })
}

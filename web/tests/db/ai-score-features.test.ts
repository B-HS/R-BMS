import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { eq, inArray, like, sql } from 'drizzle-orm'
import { composeChartPattern } from '@server/compose/chart-pattern'
import { composeFolder } from '@server/compose/folder'
import {
    createAiGoalDb,
    createAiJudgeHabitDb,
    createAiPracticeFolderDb,
    createAiWeeklySummaryDb,
    createChartTagLoader,
} from '@server/compose/ai-score-features'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import { aiGoal, aiPracticeFolder, chart, chartPattern, score, userFolder } from '@server/db/schema'
import { clearLampToId } from '@server/dto/common'
import { createAiGoalService } from '@server/service/domain/ai/goal-plan.service'
import { createAiPracticeFolderService } from '@server/service/domain/ai/practice-folder.service'
import { createAiRecommendService } from '@server/service/domain/ai/recommend.service'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { createTableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { makePatternDocument } from '../support/chart-pattern-fixture'
import { SCORE_NOW, levelRecords, makeTable, md5Of, recordOf } from '../support/ai-score-fixture'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 점수 기반 AI 기능 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'w2sf'
const OWNER = `${PREFIX}_owner`
const OTHER = `${PREFIX}_other`
const DAY = 86_400_000
const MONDAY = Date.UTC(2026, 9, 12)
const NOTES = 1000
const CHARTS = {
    alpha: { sha256: 'd1'.repeat(32), md5: 'd1'.repeat(16), title: 'W2 Alpha' },
    bravo: { sha256: 'd2'.repeat(32), md5: 'd2'.repeat(16), title: 'W2 Bravo' },
}
const PATTERN_MD5 = { current: 'e1'.repeat(16), stale: 'e2'.repeat(16), missing: 'e3'.repeat(16) }

const scoreRow = (id: string, overrides: Partial<typeof score.$inferInsert>) => ({
    id: `${PREFIX}_${id}`,
    userId: OWNER,
    chartSha256: CHARTS.alpha.sha256,
    chartMd5: null,
    mode: 'BEAT_7K',
    notes: NOTES,
    clear: clearLampToId('Easy'),
    exScore: 1500,
    maxExScore: 0,
    playedAt: MONDAY + DAY,
    ...overrides,
})

const SCORES = [
    scoreRow('before', { playedAt: MONDAY - 1, clear: clearLampToId('Hard'), exScore: 1800 }),
    scoreRow('before_sa', { playedAt: MONDAY - DAY, clear: clearLampToId('Normal'), scratchAuto: true }),
    scoreRow('start', { playedAt: MONDAY, fast: 30, slow: 10, pgreat: 500, great: 300, extra: { mean_offset_us: 2500, rule_version: 4 } }),
    scoreRow('bravo', {
        playedAt: MONDAY + 2 * DAY,
        chartSha256: CHARTS.bravo.sha256,
        chartMd5: CHARTS.bravo.md5,
        greenNumber: 500,
        extra: { rule_version: 'x' },
    }),
    scoreRow('auto', { playedAt: MONDAY + 3 * DAY, autoplay: true }),
    scoreRow('end', { playedAt: MONDAY + 7 * DAY }),
    scoreRow('other', { userId: OTHER, playedAt: MONDAY + DAY }),
]

describe.skipIf(!hasDatabase)('점수 기반 AI 기능 DB (임시 MySQL)', () => {
    const cleanup = async () => {
        const db = getDb()
        await db.delete(aiPracticeFolder).where(like(aiPracticeFolder.userId, `${PREFIX}_%`))
        await db.delete(userFolder).where(like(userFolder.userId, `${PREFIX}_%`))
        await db.delete(aiGoal).where(like(aiGoal.userId, `${PREFIX}_%`))
        await db.delete(score).where(like(score.id, `${PREFIX}_%`))
        await db.delete(chart).where(inArray(chart.sha256, [CHARTS.alpha.sha256, CHARTS.bravo.sha256]))
        await db.delete(chartPattern).where(inArray(chartPattern.md5, Object.values(PATTERN_MD5)))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
    }

    beforeAll(async () => {
        await cleanup()
        const db = getDb()
        await db.insert(user).values([OWNER, OTHER].map((id) => ({ id, name: id, email: `${id}@w2sf.invalid`, loginId: id })))
        await db.insert(chart).values(Object.values(CHARTS).map((row) => ({ ...row, notes: NOTES, mode: 'BEAT_7K' })))
        await db.insert(score).values(SCORES)
    })

    afterAll(cleanup)

    test('마이그레이션 0008 이 ai_goal 과 ai_practice_folder 를 만든다(추가만)', async () => {
        const [tables] = await getDb().execute(
            sql`SELECT table_name AS name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name IN ('ai_goal', 'ai_practice_folder')`,
        )
        expect(Array.isArray(tables) ? tables.map((row) => String(Object.values(row)[0])).toSorted() : []).toEqual(['ai_goal', 'ai_practice_folder'])
    })

    test('목표는 종류를 열로, 나머지를 params 로 저장하고 본인의 활성 목표만 읽고 보관한다', async () => {
        const goals = createAiGoalDb(getDb())
        const params = { kind: 'lamp_count_at_level' as const, table: 'insane1', level: '12', lamp: 'Hard' as const, count: 3 }
        await goals.insertGoal({ id: `aig_${'1'.repeat(30)}`, userId: OWNER, params, createdAt: 2 })
        await goals.insertGoal({
            id: `aig_${'2'.repeat(30)}`,
            userId: OWNER,
            params: { kind: 'clear_all_level', table: 'insane1', level: '3' },
            createdAt: 1,
        })
        await getDb()
            .insert(aiGoal)
            .values({ id: `aig_${'3'.repeat(30)}`, userId: OWNER, kind: 'unknown_kind', params: { x: 1 }, createdAt: 3 })
        const [stored] = await getDb()
            .select()
            .from(aiGoal)
            .where(eq(aiGoal.id, `aig_${'1'.repeat(30)}`))
        expect([stored?.kind, stored?.params]).toEqual(['lamp_count_at_level', { table: 'insane1', level: '12', lamp: 'Hard', count: 3 }])
        expect((await goals.listActiveGoals(OWNER)).map((goal) => [goal.id.slice(-1), goal.params.kind])).toEqual([
            ['2', 'clear_all_level'],
            ['1', 'lamp_count_at_level'],
        ])
        expect(await goals.findActiveGoal({ userId: OTHER, goalId: `aig_${'1'.repeat(30)}` })).toBeNull()
        expect(await goals.archiveGoal({ userId: OTHER, goalId: `aig_${'1'.repeat(30)}`, archivedAt: 5 })).toBe(false)
        expect(await goals.archiveGoal({ userId: OWNER, goalId: `aig_${'1'.repeat(30)}`, archivedAt: 5 })).toBe(true)
        expect(await goals.archiveGoal({ userId: OWNER, goalId: `aig_${'1'.repeat(30)}`, archivedAt: 6 })).toBe(false)
        expect((await goals.listActiveGoals(OWNER)).map((goal) => goal.params.kind)).toEqual(['clear_all_level'])
    })

    test('연습 폴더 연결은 같은 미리보기 해시에 하나만 잡히고 먼저 잡은 폴더가 이기며, 폴더를 지우면 연결도 사라진다', async () => {
        const db = getDb()
        const links = createAiPracticeFolderDb(db)
        const folderId = `uf_${'a'.repeat(30)}`
        const laterFolderId = `uf_${'b'.repeat(30)}`
        const hash = 'f'.repeat(64)
        await db.insert(userFolder).values([
            { id: folderId, userId: OWNER, name: 'AI 연습 · W2', updatedAt: 1 },
            { id: laterFolderId, userId: OWNER, name: 'AI 연습 · W2 later', updatedAt: 1 },
        ])
        expect(await links.findLinkedFolderId({ userId: OWNER, previewHash: hash })).toBeNull()
        expect(await links.claimLink({ userId: OWNER, previewHash: hash, folderId, createdAt: 1 })).toBe(folderId)
        expect(await links.claimLink({ userId: OWNER, previewHash: hash, folderId: laterFolderId, createdAt: 2 })).toBe(folderId)
        expect(await links.findLinkedFolderId({ userId: OWNER, previewHash: hash })).toBe(folderId)
        expect(await links.findLinkedFolderId({ userId: OTHER, previewHash: hash })).toBeNull()
        await db.delete(userFolder).where(eq(userFolder.id, folderId))
        expect(await links.findLinkedFolderId({ userId: OWNER, previewHash: hash })).toBeNull()
        await db.delete(userFolder).where(eq(userFolder.id, laterFolderId))
    })

    test('같은 미리보기로 동시에 만들기를 눌러도 실제 폴더 표에는 폴더 하나와 연결 하나만 남는다', async () => {
        const db = getDb()
        const table = makeTable()
        const records = { ...levelRecords(table, { '1': 'Hard', '2': 'Easy' }), [md5Of(300)]: recordOf('Failed', 85) }
        const tableDeps = {
            loadTable: () => table,
            loadRecords: async () => records,
            loadChartTags: async () => ({ shares: new Map(), staleMd5s: new Set<string>() }),
        }
        const unusedRunner = { run: async () => Promise.reject(new Error('the AI is not called by create')) }
        const results = { findLatest: async () => null }
        const shared = { runner: unusedRunner, results }
        const folders = composeFolder(db, {
            tableSnapshotService: createTableSnapshotService({ db: { listBestByMd5: async () => [] }, loadTables: () => new Map() }),
            lookupTableText: () => null,
        }).folderService
        const recommend = createAiRecommendService({ ...tableDeps, ...shared })
        const goals = createAiGoalService({
            ...tableDeps,
            ...shared,
            db: {
                listActiveGoals: async () => [],
                findActiveGoal: async () => null,
                insertGoal: async () => undefined,
                archiveGoal: async () => false,
            },
            newId: (prefix) => `${prefix}x`,
            now: () => SCORE_NOW,
        })
        const practice = createAiPracticeFolderService({
            ...tableDeps,
            ...shared,
            db: createAiPracticeFolderDb(db),
            recommend,
            goals,
            folders,
            now: () => SCORE_NOW,
        })
        const input = {
            source: 'recommend' as const,
            name_suffix: 'W2 race',
            description: 'same preview',
            items: [{ md5: md5Of(300), memo: 'first' }],
        }
        const outcomes = await Promise.all(Array.from({ length: 4 }, () => practice.create({ userId: OWNER, input })))
        expect(outcomes.every((outcome) => outcome.ok)).toBe(true)
        expect(outcomes.map((outcome) => (outcome.ok ? outcome.value.outcome : '')).filter((value) => value === 'created')).toHaveLength(1)
        const stored = await db.select({ id: userFolder.id }).from(userFolder).where(eq(userFolder.userId, OWNER))
        expect(stored).toHaveLength(1)
        const linked = await db.select({ folderId: aiPracticeFolder.folderId }).from(aiPracticeFolder).where(eq(aiPracticeFolder.userId, OWNER))
        expect(linked).toEqual([{ folderId: stored[0]?.id }])
        const created = outcomes.flatMap((outcome) => (outcome.ok ? [outcome.value.folder_id] : []))
        expect(new Set(created)).toEqual(new Set([stored[0]?.id]))
    })

    test('주간 판은 [시작, 끝) 의 본인 판만 오토플레이 없이 읽고 md5 는 차트 표에서 채운다', async () => {
        const weekly = createAiWeeklySummaryDb(getDb())
        const rows = await weekly.listScoresBetween({ userId: OWNER, from: MONDAY, to: MONDAY + 7 * DAY })
        expect(rows.map((row) => [row.playedAt - MONDAY, row.chartMd5, row.title, row.maxExScore])).toEqual([
            [0, CHARTS.alpha.md5, CHARTS.alpha.title, NOTES * 2],
            [2 * DAY, CHARTS.bravo.md5, CHARTS.bravo.title, NOTES * 2],
        ])
        expect(rows[0]).toMatchObject({ fast: 30, slow: 10, judgedNotes: 800, meanOffsetUs: 2500, ruleVersion: 4 })
        expect(rows[1]).toMatchObject({ meanOffsetUs: null, ruleVersion: null })
        const priors = await weekly.listPriorBests({ userId: OWNER, chartSha256s: [CHARTS.alpha.sha256, CHARTS.bravo.sha256], before: MONDAY })
        expect(priors.toSorted((left, right) => Number(left.scratchAuto) - Number(right.scratchAuto))).toEqual([
            { chartSha256: CHARTS.alpha.sha256, scratchAuto: false, clear: clearLampToId('Hard'), exScore: 1800 },
            { chartSha256: CHARTS.alpha.sha256, scratchAuto: true, clear: clearLampToId('Normal'), exScore: 1500 },
        ])
        expect(await weekly.listPriorBests({ userId: OWNER, chartSha256s: [], before: MONDAY })).toEqual([])
    })

    test('판정 습관은 최근 판부터 limit 만큼, 오토플레이 없이 읽는다', async () => {
        const rows = await createAiJudgeHabitDb(getDb(), { analyzedPlaysOfScores: async () => new Map() }).listRecentTimingRows({
            userId: OWNER,
            limit: 3,
        })
        expect(rows.map((row) => row.playedAt - MONDAY)).toEqual([7 * DAY, 2 * DAY, 0])
        expect(rows[1]).toMatchObject({ greenNumber: 500, maxExScore: NOTES * 2, ruleVersion: null })
    })

    test('곡별 태그 비율은 패턴 서비스로 특징만 읽고, 낮은 버전 특징만 다시 계산해 저장한다', async () => {
        const db = getDb()
        const service = composeChartPattern(db, { listCoverageTables: () => [] }).chartPatternService
        const trill = Array.from({ length: 8 }, (_, index) => [index * 100_000, index % 2, 0] as const)
        for (const md5 of [PATTERN_MD5.current, PATTERN_MD5.stale]) {
            expect((await service.upload({ document: makePatternDocument(trill, { md5 }), uploadedBy: OWNER })).outcome).toBe('created')
        }
        await db.update(chartPattern).set({ featuresVersion: 0 }).where(eq(chartPattern.md5, PATTERN_MD5.stale))
        const { shares, staleMd5s } = await createChartTagLoader(service)(Object.values(PATTERN_MD5))
        expect([...shares.keys()].toSorted()).toEqual([PATTERN_MD5.current, PATTERN_MD5.stale])
        expect(shares.get(PATTERN_MD5.current)?.trill).toBeGreaterThan(0)
        expect(shares.get(PATTERN_MD5.stale)).toEqual(shares.get(PATTERN_MD5.current))
        expect(staleMd5s.size).toBe(0)
        const [stale] = await db.select({ version: chartPattern.featuresVersion }).from(chartPattern).where(eq(chartPattern.md5, PATTERN_MD5.stale))
        expect(stale?.version).toBe(ANALYSIS_VERSION)
        expect((await createChartTagLoader(service)([])).shares.size).toBe(0)
    })
})

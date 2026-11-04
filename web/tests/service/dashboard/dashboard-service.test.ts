import { describe, expect, test } from 'bun:test'
import { clearLampToId } from '@server/dto/common'
import type { CourseCatalogOutput } from '@server/dto/course-catalog'
import {
    DASHBOARD_GRADE_GROUPS,
    DASHBOARD_STREAK_WINDOW_DAYS,
    dashboardCalendarOutputSchema,
    dashboardCoursesOutputSchema,
    dashboardOverviewOutputSchema,
    dashboardReplaysOutputSchema,
    dashboardRivalsOutputSchema,
    dashboardTableProgressOutputSchema,
} from '@server/dto/dashboard'
import type { TableRecordOutput, TableRecordsOutput, TableDetailOutput } from '@server/dto/table-snapshot'
import type { ActivityRow } from '@server/service/domain/fe/fe.service'
import { countGradeGroups, toDashboardRank, toDashboardStandings } from '@server/service/domain/dashboard/dashboard.service'
import { DAY_MS, MINUTE_MS, TODAY_START, makeDashboardService, noonOf } from './dashboard-fixture'

const USER_ID = 'u_alice'

const EXPECTED_STEPS_PER_GROUP = { F: 6, E: 3, D: 3, C: 3, B: 3, A: 3, AA: 3, AAA: 2, MAX: 2 } as const

describe('등급 묶음', () => {
    test('27단계 각각이 라벨 접두 기준의 9묶음 가운데 하나에 들어간다', () => {
        const notes = 27
        const inputs = Array.from({ length: 28 }, (_, step) => ({ exScore: 2 * step, notes, count: 1 }))
        const counts = Object.fromEntries(countGradeGroups(inputs).map(({ grade, count }) => [grade, count]))
        expect(counts).toEqual(EXPECTED_STEPS_PER_GROUP)
        expect(countGradeGroups(inputs).map(({ grade }) => grade)).toEqual([...DASHBOARD_GRADE_GROUPS])
    })

    test('경계: 최대 EX 의 전부는 MAX, 한 점 모자라도 MAX- 라 MAX 묶음, 0 점은 F 묶음이다', () => {
        const grades = (exScore: number, notes: number) => countGradeGroups([{ exScore, notes, count: 1 }]).filter(({ count }) => count > 0)
        expect(grades(200, 100)).toEqual([{ grade: 'MAX', count: 1 }])
        expect(grades(199, 100)).toEqual([{ grade: 'MAX', count: 1 }])
        expect(grades(185, 100)).toEqual([{ grade: 'AAA', count: 1 }])
        expect(grades(0, 100)).toEqual([{ grade: 'F', count: 1 }])
    })

    test('같은 EX·노트 수를 가진 최고 기록은 개수로 합산하고 노트 수가 없는 기록은 뺀다', () => {
        const counted = countGradeGroups([
            { exScore: 200, notes: 100, count: 4 },
            { exScore: 200, notes: 100, count: 3 },
            { exScore: 150, notes: 0, count: 9 },
        ])
        expect(counted.find(({ grade }) => grade === 'MAX')?.count).toBe(7)
        expect(counted.reduce((sum, { count }) => sum + count, 0)).toBe(7)
    })

    test('기록이 없어도 9묶음이 0 으로 모두 나온다', () => {
        expect(countGradeGroups([])).toEqual(DASHBOARD_GRADE_GROUPS.map((grade) => ({ grade, count: 0 })))
    })
})

describe('순위', () => {
    const standings = toDashboardStandings([
        { userId: 'u_a', position: 1 },
        { userId: 'u_b', position: 2 },
        { userId: 'u_c', position: 2 },
        { userId: 'u_d', position: 4 },
    ])

    test('공동 순위는 그대로 두고 전체 인원은 순위가 매겨진 사람 수다', () => {
        expect(toDashboardRank(standings, 'u_b')).toEqual({ position: 2, total_players: 4 })
        expect(toDashboardRank(standings, 'u_c')).toEqual({ position: 2, total_players: 4 })
        expect(toDashboardRank(standings, 'u_d')).toEqual({ position: 4, total_players: 4 })
    })

    test('최고 기록이 없는 사람은 순위가 null 이다', () => {
        expect(toDashboardRank(standings, 'u_none')).toEqual({ position: null, total_players: 4 })
        expect(toDashboardRank(toDashboardStandings([]), 'u_none')).toEqual({ position: null, total_players: 0 })
    })

    test('객체 기본 속성 이름의 사용자 id 도 순위로 읽지 않는다', () => {
        expect(toDashboardRank(standings, 'constructor').position).toBeNull()
        expect(toDashboardRank(standings, '__proto__').position).toBeNull()
    })

    test('서비스의 순위 계산은 DB 순위 줄을 사용자 id 순위표로 바꾼다', async () => {
        const service = makeDashboardService({ db: { listStandings: async () => [{ userId: 'u_a', position: 1 }] } })
        expect(await service.leaderboardStandings()).toEqual({ total_players: 1, positions: { u_a: 1 } })
    })
})

describe('overview', () => {
    const OCTOBER_RUN = [10, 11, 12, 13, 14].map((day) => Date.UTC(2026, 9, day, 3, 0, 0))
    const OLD_RUN = [40, 39, 38, 37, 36, 35].map((daysAgo) => noonOf(daysAgo))
    const BEYOND_WINDOW_RUN = Array.from({ length: 8 }, (_, index) => noonOf(400 - index))
    const THIS_WEEK_AND_BEFORE = [noonOf(4), noonOf(2), noonOf(1), noonOf(1, 4), noonOf(0), noonOf(0, 5), noonOf(0, 8)]

    const build = (playedAts: number[]) => {
        const requested: unknown[] = []
        const service = makeDashboardService({
            db: {
                countChartBests: async () => ({ played: 40, cleared: 30, hardOrBetter: 12, fullComboOrBetter: 3 }),
                summariseScores: async () => ({ total: 321, lastPlayedAt: noonOf(0, 8) }),
                listPlayedAts: async (params) => {
                    requested.push(params)
                    return { playedAts, previousPlayedAt: null }
                },
                listGradeInputs: async () => [{ exScore: 200, notes: 100, count: 2 }],
            },
        })
        return { service, requested }
    }

    test('DB 에서 받은 값은 그대로 두고 판 날짜에서 연속·이번 주·30일 값을 계산한다', async () => {
        const { service } = build([...BEYOND_WINDOW_RUN, ...OLD_RUN, ...OCTOBER_RUN, ...THIS_WEEK_AND_BEFORE])
        const { tiles, grades } = await service.overview({ userId: USER_ID, todayStart: TODAY_START })
        expect(tiles).toEqual({
            total_plays: 321,
            played_charts: 40,
            cleared_charts: 30,
            hard_or_better: 12,
            full_combo_or_better: 3,
            play_ms_30d: (5 * 2 + 2 + 2 + (2 + 4) + (2 + 5 + 3)) * MINUTE_MS,
            active_days_30d: 5 + 1 + 1 + 1 + 1,
            streak_days: 3,
            best_streak_days: 6,
            this_week_plays: 1 + 2 + 3,
            last_played_at: noonOf(0, 8),
        })
        expect(grades.find(({ grade }) => grade === 'MAX')?.count).toBe(2)
    })

    test('일요일 밤 판은 이번 주에 넣지 않고 월요일 첫 판부터 센다', async () => {
        const sundayNight = TODAY_START - 4 * DAY_MS + 23 * 3_600_000 + 59 * MINUTE_MS
        const mondayStart = TODAY_START - 3 * DAY_MS
        const { service } = build([sundayNight, mondayStart, mondayStart + MINUTE_MS])
        const { tiles } = await service.overview({ userId: USER_ID, todayStart: TODAY_START })
        expect(tiles.this_week_plays).toBe(2)
        expect(tiles.streak_days).toBe(0)
    })

    test('판 시각이 없으면 연속·플레이 시간·이번 주가 모두 0 이다', async () => {
        const { service } = build([])
        const { tiles } = await service.overview({ userId: USER_ID, todayStart: TODAY_START })
        expect(tiles).toMatchObject({ play_ms_30d: 0, active_days_30d: 0, streak_days: 0, best_streak_days: 0, this_week_plays: 0 })
    })

    test('오늘 판이 아직 없으면 어제까지의 연속을 유지한다', async () => {
        const { service } = build([noonOf(2), noonOf(1)])
        const { tiles } = await service.overview({ userId: USER_ID, todayStart: TODAY_START })
        expect(tiles.streak_days).toBe(2)
    })

    test('DB 에는 오늘을 포함한 371일 창을 묻는다', async () => {
        const { service, requested } = build([])
        await service.overview({ userId: USER_ID, todayStart: TODAY_START })
        expect(requested).toEqual([{ userId: USER_ID, from: TODAY_START - (DASHBOARD_STREAK_WINDOW_DAYS - 1) * DAY_MS, to: TODAY_START + DAY_MS }])
    })

    test('응답 모양이 DTO 를 통과한다(순위는 읽기 함수가 더한다)', async () => {
        const { service } = build(THIS_WEEK_AND_BEFORE)
        const overview = await service.overview({ userId: USER_ID, todayStart: TODAY_START })
        const rank = toDashboardRank(toDashboardStandings([{ userId: USER_ID, position: 1 }]), USER_ID)
        expect(dashboardOverviewOutputSchema.safeParse({ ...overview, rank }).success).toBe(true)
    })
})

describe('calendar', () => {
    test('요청한 일 수만큼 날을 만들고 그 창 안에서 연속·활동일을 센다', async () => {
        const service = makeDashboardService({
            db: { listPlayedAts: async () => ({ playedAts: [noonOf(6), noonOf(5), noonOf(1), noonOf(0), noonOf(0, 3)], previousPlayedAt: null }) },
        })
        const calendar = await service.calendar({ userId: USER_ID, todayStart: TODAY_START, days: 7 })
        expect(calendar.days).toHaveLength(7)
        expect(calendar.days[0].date).toBe('2026-10-30')
        expect(calendar.days[6]).toEqual({ date: '2026-11-05', plays: 2, play_ms: (2 + 3) * MINUTE_MS })
        expect(calendar).toMatchObject({ total_plays: 5, active_days: 4, streak_days: 2, best_streak_days: 2 })
        expect(dashboardCalendarOutputSchema.safeParse(calendar).success).toBe(true)
    })

    test('창 밖의 판은 세지 않는다', async () => {
        const service = makeDashboardService({ db: { listPlayedAts: async () => ({ playedAts: [noonOf(30), noonOf(0)], previousPlayedAt: null }) } })
        const calendar = await service.calendar({ userId: USER_ID, todayStart: TODAY_START, days: 7 })
        expect(calendar).toMatchObject({ total_plays: 1, active_days: 1, streak_days: 1, best_streak_days: 1 })
    })
})

const record = (clear: TableRecordOutput['clear'], assistGrade: TableRecordOutput['assist_grade'] = 0): TableRecordOutput => ({
    clear,
    assist_grade: assistGrade,
    ex_score: 1000,
    minbp: 3,
    max_combo: 500,
    total_notes: 1000,
    played_at: noonOf(1),
    scratch_auto: false,
})

const entry = (md5: string, level: string) => ({ md5, level, title: md5, artist: '', url: '', url_diff: '', name_diff: '' })

const TABLE_DETAIL = {
    table: { id: 'stella-st', name: 'Stella st', symbol: 'st' },
    levels: [
        { level: '1', entries: [entry('a', '1'), entry('b', '1'), entry('c', '1')] },
        { level: '2', entries: [entry('d', '2'), entry('e', '2')] },
    ],
} as unknown as TableDetailOutput

const TABLE_RECORDS: TableRecordsOutput = {
    table_id: 'stella-st',
    records: { a: record('Hard'), b: record('LightAssistEasy', 3), d: record('Failed', 2), e: record('FullCombo') },
}

describe('tableProgress', () => {
    const service = makeDashboardService({
        tableSnapshotService: {
            getDetail: async (tableId) => (tableId === 'stella-st' ? TABLE_DETAIL : null),
            getRecords: async ({ tableId }) => (tableId === 'stella-st' ? TABLE_RECORDS : null),
        },
    })

    test('표 전체와 레벨별로 칸 수·플레이한 칸·클리어·HARD 이상·램프 개수를 센다', async () => {
        const progress = await service.tableProgress({ userId: USER_ID, tableId: 'stella-st' })
        expect(progress).toEqual({
            table_id: 'stella-st',
            name: 'Stella st',
            symbol: 'st',
            total: 5,
            played: 4,
            lamps: [
                { clear: 'FullCombo', assist_grade: 0, count: 1 },
                { clear: 'Hard', assist_grade: 0, count: 1 },
                { clear: 'LightAssistEasy', assist_grade: 3, count: 1 },
                { clear: 'Failed', assist_grade: 0, count: 1 },
                { clear: 'NoPlay', assist_grade: 0, count: 1 },
            ],
            levels: [
                {
                    level: '1',
                    total: 3,
                    played: 2,
                    cleared: 2,
                    hard_or_better: 1,
                    lamps: [
                        { clear: 'Hard', assist_grade: 0, count: 1 },
                        { clear: 'LightAssistEasy', assist_grade: 3, count: 1 },
                        { clear: 'NoPlay', assist_grade: 0, count: 1 },
                    ],
                },
                {
                    level: '2',
                    total: 2,
                    played: 2,
                    cleared: 1,
                    hard_or_better: 1,
                    lamps: [
                        { clear: 'FullCombo', assist_grade: 0, count: 1 },
                        { clear: 'Failed', assist_grade: 0, count: 1 },
                    ],
                },
            ],
        })
        expect(dashboardTableProgressOutputSchema.safeParse(progress).success).toBe(true)
    })

    test('램프 개수의 합은 칸 수와 같다', async () => {
        const progress = await service.tableProgress({ userId: USER_ID, tableId: 'stella-st' })
        expect(progress?.lamps.reduce((sum, lamp) => sum + lamp.count, 0)).toBe(progress?.total)
        for (const level of progress?.levels ?? []) expect(level.lamps.reduce((sum, lamp) => sum + lamp.count, 0)).toBe(level.total)
    })

    test('LIGHT ASSIST 가 아닌 램프에 붙은 어시스트 등급은 무시한다', async () => {
        const progress = await service.tableProgress({ userId: USER_ID, tableId: 'stella-st' })
        expect(progress?.lamps.find((lamp) => lamp.clear === 'Failed')?.assist_grade).toBe(0)
    })

    test('서버에 없는 표는 null 이다', async () => {
        expect(await service.tableProgress({ userId: USER_ID, tableId: 'unknown' })).toBeNull()
    })
})

describe('courses', () => {
    const catalog: CourseCatalogOutput = {
        groups: [
            {
                id: 'g1',
                name: 'GENOSIDE',
                keys: 7,
                source: {
                    id: 's',
                    kind: 'bundled',
                    name: '',
                    maker: '',
                    page_url: '',
                    file_url: '',
                    retrieved_at: 0,
                    file_sha256: '',
                    detail: {},
                },
                courses: [
                    { course_hash: 'c1', name: 'one', stage_count: 4, active: true, dan: null },
                    { course_hash: 'c2', name: 'two', stage_count: 4, active: true, dan: null },
                    { course_hash: 'c3', name: 'three', stage_count: 4, active: true, dan: null },
                    { course_hash: 'c4', name: 'gone', stage_count: 4, active: false, dan: null },
                ],
            },
            {
                id: 'g2',
                name: 'ARCHIVE',
                keys: 7,
                source: {
                    id: 's',
                    kind: 'bundled',
                    name: '',
                    maker: '',
                    page_url: '',
                    file_url: '',
                    retrieved_at: 0,
                    file_sha256: '',
                    detail: {},
                },
                courses: [{ course_hash: 'c4', name: 'gone', stage_count: 4, active: false, dan: null }],
            },
        ],
    }

    test('묶음마다 활성 코스 수·플레이한 코스·클리어한 코스를 세고 활성 코스가 없는 묶음은 뺀다', async () => {
        const service = makeDashboardService({
            listCatalogGroups: async () => catalog,
            db: {
                listCourseBests: async () => [
                    { courseHash: 'c1', clear: clearLampToId('Hard') },
                    { courseHash: 'c2', clear: clearLampToId('Failed') },
                    { courseHash: 'c4', clear: clearLampToId('Hard') },
                ],
                listRecentCourseScores: async () => [
                    { courseHash: 'c1', name: 'one', clear: clearLampToId('Hard'), exScore: 3000, playedAt: noonOf(1) },
                ],
            },
        })
        const courses = await service.courses({ userId: USER_ID })
        expect(courses.groups).toEqual([{ id: 'g1', group: 'GENOSIDE', total: 3, played: 2, cleared: 1 }])
        expect(courses.recent).toEqual([{ course_hash: 'c1', name: 'one', clear: 'Hard', ex_score: 3000, played_at: noonOf(1) }])
        expect(dashboardCoursesOutputSchema.safeParse(courses).success).toBe(true)
    })

    test('최근 코스 기록은 5건만 묻는다', async () => {
        const limits: number[] = []
        const service = makeDashboardService({
            db: {
                listRecentCourseScores: async ({ limit }) => {
                    limits.push(limit)
                    return []
                },
            },
        })
        await service.courses({ userId: USER_ID })
        expect(limits).toEqual([5])
    })
})

const activity = (overrides: Partial<ActivityRow>): ActivityRow => ({
    scoreId: 's1',
    loginId: 'rival_a',
    playerName: 'Rival A',
    clear: clearLampToId('Hard'),
    assistGrade: 0,
    exScore: 1500,
    minbp: 4,
    maxCombo: 800,
    playedAt: noonOf(1),
    scratchAuto: false,
    chartSha256: 'f'.repeat(64),
    chartMd5: 'e'.repeat(32),
    title: 'Song',
    artist: 'Artist',
    level: 12,
    ...overrides,
})

describe('rivals', () => {
    test('라이벌이 없으면 다른 질의를 하지 않고 빈 응답이다', async () => {
        const asked: string[] = []
        const service = makeDashboardService({
            db: {
                listRecentRivalActivity: async () => {
                    asked.push('recent')
                    return []
                },
                listVersus: async () => {
                    asked.push('versus')
                    return []
                },
            },
        })
        expect(await service.rivals({ userId: USER_ID, limit: 10 })).toEqual({ rivals: [], recent: [], versus: [] })
        expect(asked).toEqual([])
    })

    test('라이벌마다 공통 곡·승·패·무를 채우고 겹치는 곡이 없는 라이벌은 0 이다', async () => {
        const seen: unknown[] = []
        const service = makeDashboardService({
            lookupTableText: (md5) => (md5 === 'e'.repeat(32) ? { title: 'From Table', artist: 'Table Artist' } : null),
            db: {
                listRivals: async () => [
                    { userId: 'u_a', loginId: 'rival_a', name: 'Rival A' },
                    { userId: 'u_b', loginId: 'rival_b', name: 'Rival B' },
                ],
                listRecentRivalActivity: async (params) => {
                    seen.push(params)
                    return [activity({ title: '', artist: '' })]
                },
                listVersus: async () => [{ rivalId: 'u_a', common: 7, wins: 3, losses: 2 }],
            },
        })
        const rivals = await service.rivals({ userId: USER_ID, limit: 4 })
        expect(seen).toEqual([{ userIds: ['u_a', 'u_b'], limit: 4 }])
        expect(rivals.rivals).toEqual([
            { login_id: 'rival_a', name: 'Rival A' },
            { login_id: 'rival_b', name: 'Rival B' },
        ])
        expect(rivals.versus).toEqual([
            { login_id: 'rival_a', common: 7, wins: 3, losses: 2, draws: 2 },
            { login_id: 'rival_b', common: 0, wins: 0, losses: 0, draws: 0 },
        ])
        expect(rivals.recent[0]).toMatchObject({
            score_id: 's1',
            player: { id: 'rival_a' },
            clear: 'Hard',
            chart: { title: 'From Table', artist: 'Table Artist' },
        })
        expect(dashboardRivalsOutputSchema.safeParse(rivals).success).toBe(true)
    })
})

describe('replays', () => {
    test('점수가 이어지지 않은 리플레이는 램프와 EX 가 null 이고 차트 글자는 표에서 채운다', async () => {
        const service = makeDashboardService({
            lookupTableText: () => ({ title: 'From Table', artist: 'Table Artist' }),
            db: {
                listRecentReplays: async () => [
                    {
                        id: 'r1',
                        createdAt: noonOf(1),
                        chartSha256: 'a'.repeat(64),
                        chartMd5: 'b'.repeat(32),
                        title: 'Own Title',
                        artist: null,
                        level: 11,
                        clear: clearLampToId('ExHard'),
                        exScore: 1700,
                    },
                    {
                        id: 'r2',
                        createdAt: noonOf(2),
                        chartSha256: 'c'.repeat(64),
                        chartMd5: null,
                        title: null,
                        artist: null,
                        level: null,
                        clear: null,
                        exScore: null,
                    },
                ],
            },
        })
        const replays = await service.replays({ userId: USER_ID, limit: 5 })
        expect(replays.replays).toEqual([
            {
                id: 'r1',
                created_at: noonOf(1),
                chart: { md5: 'b'.repeat(32), sha256: 'a'.repeat(64), title: 'Own Title', level: 11 },
                clear: 'ExHard',
                ex_score: 1700,
            },
            { id: 'r2', created_at: noonOf(2), chart: { md5: '', sha256: 'c'.repeat(64), title: '', level: null }, clear: null, ex_score: null },
        ])
        expect(dashboardReplaysOutputSchema.safeParse(replays).success).toBe(true)
    })
})

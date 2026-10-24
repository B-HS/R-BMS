import { afterAll, beforeAll, describe, expect, mock, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { count, eq, inArray, like } from 'drizzle-orm'
import { composeCourseCatalog } from '@server/compose/course-catalog'
import { composeDan } from '@server/compose/dan'
import { getDb } from '@server/db'
import { user } from '@server/db/auth-schema'
import {
    apiToken,
    course,
    courseBest,
    courseChart,
    courseGroup,
    courseGroupMember,
    courseScore,
    courseSource,
    danCourse,
    userDan,
    userDanDisplay,
} from '@server/db/schema'
import { clearLampToId, gaugeTypeToId } from '@server/dto/common'
import type { StoredCourseStage } from '@server/dto/course'
import { bundledCoursesFileSchema } from '@server/dto/course-catalog'
import { hashApiToken } from '@server/lib/token'
import type { createCourseSubmitRoute } from '@server/route/course'
import { bundledCatalog } from '@server/service/domain/course-catalog/bundled-courses'
import type { CatalogCourse, CatalogDefinition } from '@server/service/domain/course-catalog/course-catalog.type'
import { tableCatalog } from '@server/service/domain/course-catalog/table-courses'
import { DAN_BACKFILL_PAGE } from '@server/service/domain/dan/dan.service'
import { recomputeDans, RECOMPUTE_COMMAND } from '../../scripts/dan-recompute'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 단위 자동 판정 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const PREFIX = 'dnaw'
const PLAYERS = ['alice', 'bob', 'carol', 'dave', 'erin', 'frank', 'grace', 'heidi'] as const
const BULK_COUNT = DAN_BACKFILL_PAGE + 1
const BULK_TIE_BUCKETS = 7
const BASE_PLAYED_AT = 1_760_000_000_000
const PLAY_SPACING_MS = 1_000
const CREATED = 201
const OK = 200
const GAUGE_LEFT = 12.5
const SP_GROUP = 'genoside-2018-sp'
const DP_GROUP = 'genoside-2018-dp'
const GENOSIDE_SOURCE = 'genoside-2018'
const STELLA_GROUP = 'stella-st'
const COURSES_FILE = join(import.meta.dirname, '..', '..', 'data', 'courses', 'genoside-2018', 'courses.json')
const INSERT_CHUNK = 250
const SUMMARY_LINE = /^applied danCourses \d+, scanned \d+, eligible \d+, awarded 0, eligibleAssisted \d+, awardedAssisted 0, assistedLegacy \d+$/
const SP_DISPLAY = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP' } as const
const NO_ASSISTED_TOTALS = { eligibleAssisted: 0, awardedAssisted: 0, assistedLegacy: 0 }

type Player = (typeof PLAYERS)[number]

const loginOf = (player: Player) => `${PREFIX}_${player}`
const tokenOf = (player: Player) => `rbms_${PREFIX}_${player}_token`
const bulkUserOf = (index: number) => `${PREFIX}_bulk_${String(index).padStart(4, '0')}`
const emptyContext = { params: Promise.resolve({}) }

const md5Of = (seed: string) => createHash('md5').update(seed).digest('hex')

const genoside = () => bundledCatalog(bundledCoursesFileSchema.parse(JSON.parse(readFileSync(COURSES_FILE, 'utf8'))))

const stella = () =>
    tableCatalog({
        table: {
            id: STELLA_GROUP,
            expectedName: 'Stella',
            symbol: 'st',
            sourceName: 'Stella',
            maker: 'Stella',
            pageUrl: 'https://example.test/st',
            headerUrl: 'https://example.test/st/header.json',
            allowedHost: 'example.test',
            keys: 7,
            groupPosition: 2,
        },
        headerCourse: [[{ name: 'Stella Skill Simulator 4th st0', md5: [md5Of('dnaw-a'), md5Of('dnaw-b')] }]],
        entries: [],
        headerSha256: 'a'.repeat(64),
        dataSha256: 'b'.repeat(64),
        dataUrl: 'https://example.test/st/score.json',
        fetchedAt: BASE_PLAYED_AT,
    }).definition

const hashesOf = (definition: CatalogDefinition) => definition.groups.flatMap((group) => group.courses.map((entry) => entry.courseHash))

const zeroJudge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

const submissionOf = (player: string, target: CatalogCourse, offset: number, overrides: Record<string, unknown> = {}) => ({
    course_hash: target.courseHash,
    player: { id: player },
    clear: 'Normal',
    ex_score: 0,
    judge: zeroJudge,
    gauge_value: GAUGE_LEFT,
    gauge: 'Class',
    course_gauge: 'LR2',
    charts: target.charts.map((chart) => ({ md5: chart.md5, sha256: chart.sha256 })),
    stages: target.charts.map(() => ({ gauge: 'Class', random: 'Off', lntype: 0 })),
    played_at: BASE_PLAYED_AT + offset * PLAY_SPACING_MS,
    ...overrides,
})

describe.skipIf(!hasDatabase)('단위 자동 판정 (임시 MySQL)', () => {
    let submitCourseRoute: ReturnType<typeof createCourseSubmitRoute>
    const [firstDan] = genoside().groups[0].courses
    const [stellaCourse] = stella().groups[0].courses
    const catalog = () => composeCourseCatalog(getDb()).courseCatalogService
    const dans = () => composeDan(getDb()).danService

    const submit = (player: Player, body: unknown) =>
        submitCourseRoute(
            new Request('http://localhost:3000/api/courses', {
                method: 'POST',
                body: JSON.stringify(body),
                headers: { 'content-type': 'application/json', authorization: `Bearer ${tokenOf(player)}` },
            }),
            emptyContext,
        )

    const rowsOf = (player: Player) =>
        getDb()
            .select()
            .from(userDan)
            .where(eq(userDan.userId, loginOf(player)))

    const countDanRows = async (system: string) => (await getDb().select({ rows: count() }).from(userDan).where(eq(userDan.system, system)))[0].rows

    const cleanUp = async () => {
        const db = getDb()
        const hashes = [...hashesOf(genoside()), ...hashesOf(stella())]
        const groups = [SP_GROUP, DP_GROUP, STELLA_GROUP]
        await db.delete(userDanDisplay).where(like(userDanDisplay.userId, `${PREFIX}_%`))
        await db.delete(userDan).where(like(userDan.userId, `${PREFIX}_%`))
        await db.delete(courseScore).where(like(courseScore.userId, `${PREFIX}_%`))
        await db.delete(courseBest).where(like(courseBest.userId, `${PREFIX}_%`))
        await db.delete(apiToken).where(like(apiToken.userId, `${PREFIX}_%`))
        await db.delete(user).where(like(user.id, `${PREFIX}_%`))
        await db.delete(danCourse).where(inArray(danCourse.courseHash, hashes))
        await db.delete(courseGroupMember).where(inArray(courseGroupMember.groupId, groups))
        await db.delete(courseGroup).where(inArray(courseGroup.id, groups))
        await db.delete(courseSource).where(inArray(courseSource.id, [GENOSIDE_SOURCE, STELLA_GROUP]))
        await db.delete(courseChart).where(inArray(courseChart.courseHash, hashes))
        await db.delete(course).where(inArray(course.courseHash, hashes))
    }

    beforeAll(async () => {
        mock.module('next/cache', () => ({ cacheLife: () => undefined, cacheTag: () => undefined, revalidateTag: () => undefined }))
        submitCourseRoute = (await import('@server/route/course')).createCourseSubmitRoute()
        await cleanUp()
        const db = getDb()
        await db
            .insert(user)
            .values(
                PLAYERS.map((player) => ({ id: loginOf(player), name: player, email: `${loginOf(player)}@dan.invalid`, loginId: loginOf(player) })),
            )
        await db
            .insert(apiToken)
            .values(PLAYERS.map((player) => ({ id: `tk_${loginOf(player)}`, userId: loginOf(player), tokenHash: hashApiToken(tokenOf(player)) })))
        await catalog().applySync(genoside())
    })

    afterAll(cleanUp)

    test('조건을 지킨 코스 제출은 단위를 주고 판의 게이지와 모드와 시각이 기록된다', async () => {
        const response = await submit('alice', submissionOf(loginOf('alice'), firstDan, 1))
        expect(response.status).toBe(CREATED)
        const scoreId = (await response.json()).score_id
        expect(await rowsOf('alice')).toEqual([
            expect.objectContaining({
                system: 'genoside2018_sp',
                label: '初段',
                rank: 1,
                courseHash: firstDan.courseHash,
                courseScoreId: scoreId,
                courseGauge: 'LR2',
                gauge: gaugeTypeToId('Class'),
                acquiredAt: BASE_PLAYED_AT + PLAY_SPACING_MS,
            }),
        ])
    })

    test('같은 판을 다시 보내도 단위는 1행이고 처음 획득이 남는다', async () => {
        const [before] = await rowsOf('alice')
        const again = await submit('alice', submissionOf(loginOf('alice'), firstDan, 1, { course_gauge: 'IIDX' }))
        expect(again.status).toBe(OK)
        const rows = await rowsOf('alice')
        expect(rows).toHaveLength(1)
        expect(rows[0]).toEqual(before)
    })

    test('FAILED·RANDOM 판과 스테이지 사실이 없는 옛 본문은 단위를 주지 않는다', async () => {
        const submissions = [
            ['bob', { clear: 'Failed', gauge_value: 0 }],
            ['dave', { stages: firstDan.charts.map((_, index) => ({ gauge: 'Class', random: index === 0 ? 'Random' : 'Off', lntype: 0 })) }],
            ['erin', { gauge: undefined, course_gauge: undefined, stages: undefined }],
        ] as const
        for (const [player, overrides] of submissions) {
            const response = await submit(player, submissionOf(loginOf(player), firstDan, 2, overrides))
            expect(response.status).toBe(CREATED)
            expect(await rowsOf(player)).toEqual([])
        }
    })

    test('SCRATCH AUTO 판은 A-단위를 주고 같은 단위의 정식 판은 정식 행을 따로 더한다', async () => {
        const id = loginOf('carol')
        const assisted = await submit(
            'carol',
            submissionOf(id, firstDan, 3, { clear: 'LightAssistEasy', scratch_auto: true, assist: ['AUTO_SCRATCH'] }),
        )
        expect(assisted.status).toBe(CREATED)
        expect((await rowsOf('carol')).map((row) => [row.label, row.assisted])).toEqual([['初段', true]])
        expect((await dans().listDisplayed([id])).get(id)).toEqual([{ ...SP_DISPLAY, label: 'A-初段', assisted: true }])

        expect((await submit('carol', submissionOf(id, firstDan, 4))).status).toBe(CREATED)
        const rows = await rowsOf('carol')
        expect(rows.map((row) => [row.label, row.assisted]).toSorted()).toEqual([
            ['初段', false],
            ['初段', true],
        ])
        expect((await dans().listDisplayed([id])).get(id)).toEqual([{ ...SP_DISPLAY, label: '初段', assisted: false }])
        await dans().setDisplay(id, { system: 'genoside2018_sp', mode: 'label', label: '初段', assisted: true })
        expect((await dans().listDisplayed([id])).get(id)).toEqual([{ ...SP_DISPLAY, label: 'A-初段', assisted: true }])
        expect((await dans().listMine(id)).systems[0]).toMatchObject({ display: { mode: 'label', label: '初段', assisted: true } })
    })

    test('넓힌 판정 판은 어시스트만 빼면 순위였다는 사실을 남기고 A-단위를 준다', async () => {
        const id = loginOf('grace')
        const response = await submit('grace', submissionOf(id, firstDan, 5, { clear: 'AssistEasy', assist: ['CUSTOM_JUDGE'] }))
        expect(response.status).toBe(CREATED)
        const [stored] = await getDb().select().from(courseScore).where(eq(courseScore.userId, id))
        expect(stored.ranked).toBe(false)
        expect(stored.extra).toMatchObject({ flags: ['ASSIST', 'UNKNOWN_BUILD'], ranked_without_assist: true })
        expect((await rowsOf('grace')).map((row) => [row.label, row.assisted])).toEqual([['初段', true]])
    })

    test('ranked_without_assist 없이 저장된 넓힌 판정 판은 소급에서 assistedLegacy 로 세고 A-단위를 준다', async () => {
        const id = loginOf('heidi')
        await getDb()
            .insert(courseScore)
            .values({
                id: `cs_${id}`,
                userId: id,
                courseHash: firstDan.courseHash,
                clear: clearLampToId('AssistEasy'),
                ranked: false,
                gaugeValue: GAUGE_LEFT,
                playedAt: BASE_PLAYED_AT,
                gauge: gaugeTypeToId('Class'),
                courseGauge: 'LR2',
                stages: firstDan.charts.map((chart): StoredCourseStage => ({
                    gauge: 'Class',
                    random: 'Off',
                    lntype: 0,
                    md5: chart.md5,
                    sha256: chart.sha256,
                })),
                extra: { flags: ['ASSIST', 'UNKNOWN_BUILD'], scratch_auto: false, assist: ['CUSTOM_JUDGE'] },
            })
        const plan = await dans().awardForCourses({ courseHashes: [firstDan.courseHash], dryRun: true })
        expect(plan.assistedLegacy).toBe(1)
        const applied = await dans().awardForCourses({ courseHashes: [firstDan.courseHash], dryRun: false })
        expect(applied).toMatchObject({ assistedLegacy: 1, awarded: 0, awardedAssisted: 1 })
        expect((await rowsOf('heidi')).map((row) => [row.label, row.assisted, row.courseScoreId])).toEqual([['初段', true, `cs_${id}`]])
    })

    test('표시 단위는 자동이면 획득한 단위이고 숨기면 공개 목록에서 빠진다', async () => {
        const id = loginOf('alice')
        expect((await dans().listDisplayed([id])).get(id)).toEqual([{ ...SP_DISPLAY, label: '初段', assisted: false }])
        await dans().setDisplay(id, { system: 'genoside2018_sp', mode: 'hidden' })
        expect((await dans().listDisplayed([id])).get(id)).toEqual([])
        const mine = await dans().listMine(id)
        expect(mine.systems[0]).toMatchObject({ shown: [], display: { mode: 'hidden' }, acquired: [{ label: '初段', course_name: firstDan.name }] })
        await dans().setDisplay(id, { system: 'genoside2018_sp', mode: 'auto' })
        expect((await dans().listDisplayed([id])).get(id)).toHaveLength(1)
    })

    test('단위 코스로 등록되기 전에 낸 판은 소급으로 단위를 주고 한 쪽 크기를 넘어도 모두 센다', async () => {
        const db = getDb()
        await db.insert(user).values(
            Array.from({ length: BULK_COUNT }, (_, index) => ({
                id: bulkUserOf(index),
                name: bulkUserOf(index),
                email: `${bulkUserOf(index)}@dan.invalid`,
                loginId: bulkUserOf(index),
            })),
        )
        const stages = stellaCourse.charts.map((chart): StoredCourseStage => ({
            gauge: 'Class',
            random: 'Off',
            lntype: 0,
            md5: chart.md5,
            sha256: '',
        }))
        const run = (userId: string, offset: number) => ({
            id: `cs_${userId}`,
            userId,
            courseHash: stellaCourse.courseHash,
            clear: clearLampToId('Normal'),
            gaugeValue: GAUGE_LEFT,
            playedAt: BASE_PLAYED_AT + offset,
            gauge: gaugeTypeToId('Class'),
            courseGauge: 'LR2' as const,
            stages,
        })
        const runs = [run(loginOf('frank'), 0), ...Array.from({ length: BULK_COUNT }, (_, index) => run(bulkUserOf(index), index % BULK_TIE_BUCKETS))]
        for (let start = 0; start < runs.length; start += INSERT_CHUNK) await db.insert(courseScore).values(runs.slice(start, start + INSERT_CHUNK))
        expect(await countDanRows('stella')).toBe(0)

        const plan = await catalog().applySync(stella())
        expect(plan.groups[0].newDanHashes).toEqual([stellaCourse.courseHash])
        const hashes = [stellaCourse.courseHash]

        expect(await dans().awardForCourses({ courseHashes: hashes, dryRun: true })).toEqual({
            scanned: runs.length,
            eligible: runs.length,
            awarded: 0,
            ...NO_ASSISTED_TOTALS,
        })
        expect(await countDanRows('stella')).toBe(0)

        expect(await dans().awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual({
            scanned: runs.length,
            eligible: runs.length,
            awarded: runs.length,
            ...NO_ASSISTED_TOTALS,
        })
        expect(await countDanRows('stella')).toBe(runs.length)
        expect(await rowsOf('frank')).toEqual([
            expect.objectContaining({
                system: 'stella',
                label: 'st0',
                rank: 0,
                courseGauge: 'LR2',
                acquiredAt: BASE_PLAYED_AT,
                courseScoreId: run(loginOf('frank'), 0).id,
            }),
        ])

        expect(await dans().awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual({
            scanned: runs.length,
            eligible: runs.length,
            awarded: 0,
            ...NO_ASSISTED_TOTALS,
        })
    })

    test('모든 단위 코스를 다시 판정하는 스크립트 함수는 이미 받은 단위를 건드리지 않는다', async () => {
        const lines: string[] = []
        const exitCode = await recomputeDans({ command: RECOMPUTE_COMMAND.APPLY, service: dans(), log: (line) => lines.push(line) })
        expect(exitCode).toBe(0)
        expect(lines).toHaveLength(1)
        expect(lines[0]).toMatch(SUMMARY_LINE)
    })
})

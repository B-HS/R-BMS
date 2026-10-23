import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { bundledCoursesFileSchema } from '@server/dto/course-catalog'
import { bundledCatalog } from '@server/service/domain/course-catalog/bundled-courses'
import { createCourseCatalogService } from '@server/service/domain/course-catalog/course-catalog.service'
import type { CatalogSyncPlan } from '@server/service/domain/course-catalog/course-catalog.type'
import { createMemoryCourseCatalogDb } from '../support/course-catalog-memory-db'
import { COURSES_FILE, EXIT_CODE, formatPlan, parseSeedCommand, SEED_COMMAND, seedCourses, USAGE } from '../../scripts/seed-courses'

const SEED_SCRIPT = 'scripts/seed-courses.ts'
const WEB_DIRECTORY = `${import.meta.dirname}/../..`
const HEX_RUN = /[0-9a-f]{12,}/i
const PLAN_FIGURE = '(inserted|changed|unchanged|deactivated|danCourses|newDanCourses) \\d+'
const PLAN_LINE = new RegExp(`^(plan|applied|remaining) [a-z0-9-]+: ${PLAN_FIGURE}(, ${PLAN_FIGURE})*$`)
const COUNTED_GROUP_SIZE = 22

const definition = bundledCatalog(bundledCoursesFileSchema.parse(JSON.parse(readFileSync(COURSES_FILE, 'utf8'))))

const createRun = () => {
    const memory = createMemoryCourseCatalogDb()
    const service = createCourseCatalogService({ db: memory.db })
    const lines: string[] = []
    const run = (command: (typeof SEED_COMMAND)[keyof typeof SEED_COMMAND]) =>
        seedCourses({ command, service, definition, log: (line) => lines.push(line) })
    return { ...memory, lines, run }
}

const runScript = async (args: string[]) => {
    const child = Bun.spawn([process.execPath, '--no-env-file', SEED_SCRIPT, ...args], {
        cwd: WEB_DIRECTORY,
        env: { PATH: process.env.PATH ?? '' },
        stdout: 'pipe',
        stderr: 'pipe',
    })
    const [stdout, stderr, exitCode] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited])
    return { stdout, stderr, exitCode }
}

describe('parseSeedCommand', () => {
    test('count 와 apply 만 알아본다', () => {
        expect(parseSeedCommand(['count'])).toBe(SEED_COMMAND.COUNT)
        expect(parseSeedCommand(['apply'])).toBe(SEED_COMMAND.APPLY)
    })

    test('모르는 인자, 빈 인자, 남는 인자는 알아보지 못한다', () => {
        expect(parseSeedCommand(['seed'])).toBeUndefined()
        expect(parseSeedCommand([])).toBeUndefined()
        expect(parseSeedCommand(['count', 'apply'])).toBeUndefined()
        expect(parseSeedCommand(['COUNT'])).toBeUndefined()
    })
})

describe('seedCourses', () => {
    test('count 는 숫자만 출력하고 행 내용을 출력하지 않는다', async () => {
        const { lines, run } = createRun()
        expect(await run(SEED_COMMAND.COUNT)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual([
            `plan genoside-2018-sp: inserted ${COUNTED_GROUP_SIZE}, changed 0, unchanged 0, deactivated 0, danCourses ${COUNTED_GROUP_SIZE}, newDanCourses ${COUNTED_GROUP_SIZE}`,
            `plan genoside-2018-dp: inserted ${COUNTED_GROUP_SIZE}, changed 0, unchanged 0, deactivated 0, danCourses ${COUNTED_GROUP_SIZE}, newDanCourses ${COUNTED_GROUP_SIZE}`,
        ])
        const courses = definition.groups.flatMap((group) => group.courses)
        const printed = lines.join('\n')
        expect(printed).not.toMatch(HEX_RUN)
        for (const course of courses) expect(printed).not.toContain(course.name)
    })

    test('count 는 아무것도 쓰지 않는다', async () => {
        const { counts, courses, members, run } = createRun()
        await run(SEED_COMMAND.COUNT)
        expect(counts.applyGroup).toBe(0)
        expect(courses.size + members.size).toBe(0)
    })

    test('apply 는 적용하고 다시 세어 바뀔 것이 없으면 성공한다', async () => {
        const { lines, courses, run } = createRun()
        expect(await run(SEED_COMMAND.APPLY)).toBe(EXIT_CODE.OK)
        expect(courses.size).toBe(COUNTED_GROUP_SIZE * 2)
        expect(lines.every((line) => PLAN_LINE.test(line))).toBe(true)
        expect(
            lines
                .filter((line) => line.startsWith('remaining '))
                .every((line) => line.includes('inserted 0, changed 0') && line.includes('deactivated 0')),
        ).toBe(true)
    })

    test('apply 를 두 번 돌려도 같은 결과다', async () => {
        const { counts, courses, members, run } = createRun()
        await run(SEED_COMMAND.APPLY)
        const sizes = [courses.size, members.size]
        expect(await run(SEED_COMMAND.APPLY)).toBe(EXIT_CODE.OK)
        expect([courses.size, members.size]).toEqual(sizes)
        expect(counts.applyGroup).toBe(definition.groups.length * 2)
    })

    test('적용 뒤에도 바뀔 것이 남으면 실패한다', async () => {
        const pending: CatalogSyncPlan = {
            groups: [{ id: 'genoside-2018-sp', inserted: 1, changed: 0, unchanged: 0, deactivated: 0, danCourses: 1, newDanHashes: [] }],
        }
        const lines: string[] = []
        const exitCode = await seedCourses({
            command: SEED_COMMAND.APPLY,
            service: { planSync: async () => pending, applySync: async () => pending },
            definition,
            log: (line) => lines.push(line),
        })
        expect(exitCode).toBe(EXIT_CODE.FAILED)
        expect(lines.at(-1)).toStartWith('remaining genoside-2018-sp: inserted 1')
    })
})

describe('formatPlan', () => {
    test('묶음마다 한 줄이고 새 단위 코스는 해시가 아니라 개수로 낸다', () => {
        const lines = formatPlan({
            groups: [
                {
                    id: 'stella-st',
                    inserted: 1,
                    changed: 2,
                    unchanged: 3,
                    deactivated: 4,
                    danCourses: 5,
                    newDanHashes: ['a'.repeat(64), 'b'.repeat(64)],
                },
            ],
        })
        expect(lines).toEqual(['stella-st: inserted 1, changed 2, unchanged 3, deactivated 4, danCourses 5, newDanCourses 2'])
    })
})

describe('seed-courses.ts 실행', () => {
    test('알 수 없는 인자는 사용법을 내고 종료 코드 2 다', async () => {
        const { stdout, stderr, exitCode } = await runScript(['unknown'])
        expect(exitCode).toBe(EXIT_CODE.USAGE)
        expect(stderr.trim()).toBe(USAGE)
        expect(stdout).toBe('')
    })

    test('데이터베이스가 정해지지 않았으면 접속하지 않고 종료 코드 1 이다', async () => {
        const { stderr, exitCode } = await runScript(['count'])
        expect(exitCode).toBe(EXIT_CODE.FAILED)
        expect(stderr).toContain('is not set')
    })
})

import { describe, expect, test } from 'bun:test'
import { createDanService } from '@server/service/domain/dan/dan.service'
import { createMemoryDanDb } from '../support/dan-memory-db'
import { danRun, FIRST_DAN_COURSE, SECOND_DAN_COURSE } from '../support/dan-fixture'
import { parseRecomputeCommand, RECOMPUTE_COMMAND, recomputeDans, USAGE } from '../../scripts/dan-recompute'
import { EXIT_CODE } from '../../scripts/seed-courses'

const RECOMPUTE_SCRIPT = 'scripts/dan-recompute.ts'
const WEB_DIRECTORY = `${import.meta.dirname}/../..`
const HEX_RUN = /[0-9a-f]{12,}/i
const SUMMARY_LINE = /^(plan|applied) danCourses \d+, scanned \d+, eligible \d+, awarded \d+$/

const createRun = () => {
    const memory = createMemoryDanDb({
        courses: [FIRST_DAN_COURSE, SECOND_DAN_COURSE],
        runs: [
            danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: 'user-1' }),
            danRun(SECOND_DAN_COURSE, { id: 'cs_2', userId: 'user-1' }),
            danRun(SECOND_DAN_COURSE, { id: 'cs_3', userId: 'user-2', clear: 1 }),
        ],
    })
    const service = createDanService({ db: memory.db })
    const lines: string[] = []
    const run = (command: (typeof RECOMPUTE_COMMAND)[keyof typeof RECOMPUTE_COMMAND]) =>
        recomputeDans({ command, service, log: (line) => lines.push(line) })
    return { ...memory, lines, run }
}

const runScript = async (args: string[]) => {
    const child = Bun.spawn([process.execPath, RECOMPUTE_SCRIPT, ...args], {
        cwd: WEB_DIRECTORY,
        env: { PATH: process.env.PATH ?? '' },
        stdout: 'pipe',
        stderr: 'pipe',
    })
    const [stdout, stderr, exitCode] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited])
    return { stdout, stderr, exitCode }
}

describe('parseRecomputeCommand', () => {
    test('count 와 apply 만 알아본다', () => {
        expect(parseRecomputeCommand(['count'])).toBe(RECOMPUTE_COMMAND.COUNT)
        expect(parseRecomputeCommand(['apply'])).toBe(RECOMPUTE_COMMAND.APPLY)
        expect(parseRecomputeCommand(['recompute'])).toBeUndefined()
        expect(parseRecomputeCommand([])).toBeUndefined()
        expect(parseRecomputeCommand(['count', 'apply'])).toBeUndefined()
    })
})

describe('recomputeDans', () => {
    test('count 는 숫자만 출력하고 아무것도 넣지 않는다', async () => {
        const { lines, acquired, run } = createRun()
        expect(await run(RECOMPUTE_COMMAND.COUNT)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual(['plan danCourses 2, scanned 3, eligible 2, awarded 0'])
        expect(acquired.size).toBe(0)
        expect(lines.join('\n')).not.toMatch(HEX_RUN)
    })

    test('apply 는 단위를 넣고, 두 번째 apply 는 새로 넣는 것이 없다', async () => {
        const { lines, acquired, run } = createRun()
        expect(await run(RECOMPUTE_COMMAND.APPLY)).toBe(EXIT_CODE.OK)
        expect(acquired.size).toBe(2)
        expect(await run(RECOMPUTE_COMMAND.APPLY)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual(['applied danCourses 2, scanned 3, eligible 2, awarded 2', 'applied danCourses 2, scanned 3, eligible 2, awarded 0'])
        expect(lines.every((line) => SUMMARY_LINE.test(line))).toBe(true)
    })
})

describe('dan-recompute.ts 실행', () => {
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

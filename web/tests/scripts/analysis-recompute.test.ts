import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { TRACE_QUOTA_LEVEL, TRACE_USER_QUOTA } from '@server/dto/trace'
import type { ChartPatternRecomputeInput } from '@server/dto/chart-pattern'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { CURRENT_ANALYSIS_VERSIONS } from '@server/service/domain/trace/trace-analysis'
import type { TraceService } from '@server/service/domain/trace/trace.service'
import { ANALYSIS_RECOMPUTE_BATCH, ANALYSIS_RECOMPUTE_MAX_MS, recomputeAnalyses, USAGE } from '../../scripts/analysis-recompute'
import { RECOMPUTE_COMMAND, type RecomputeCommand } from '../../scripts/dan-recompute'
import { EXIT_CODE } from '../../scripts/seed-courses'

const RECOMPUTE_SCRIPT = 'scripts/analysis-recompute.ts'
const WEB_DIRECTORY = `${import.meta.dirname}/../..`
const WORKFLOW_FILE = `${WEB_DIRECTORY}/../.github/workflows/web-db.yml`
const MS_PER_MINUTE = 60_000
const SECRET_LOGIN_ID = 'secret-player-login'
const SLOW_BATCH_TICK_MS = 3 * MS_PER_MINUTE

const FIXED_SENTENCES = [
    'plan patternsStale N, analysesMissing N, analysesStale N',
    'applied patterns N, analyses N, skipped N, remaining N',
    'stopped elapsedMs N, remaining N',
]

const sentenceOf = (line: string) => line.replace(/\d+/g, 'N')

const md5Of = (index: number) => index.toString(16).padStart(32, '0')

const traceIdOf = (index: number) => `trace-${String(index).padStart(6, '0')}`

const idsOf = (count: number, idOf: (index: number) => string, from = 0) => Array.from({ length: count }, (_, index) => idOf(from + index))

type Usage = Awaited<ReturnType<TraceService['adminUsage']>>

const usageOf = (analysis: { missing: number; stale: number }): Usage => ({
    limits: { traces: TRACE_USER_QUOTA.COUNT_MAX, bytes: TRACE_USER_QUOTA.TOTAL_BYTES_MAX, near_ratio: TRACE_USER_QUOTA.NEAR_RATIO },
    totals: { users: 2, traces: 9, bytes: 900, stored_bytes: 400 },
    accounts_near: 1,
    accounts_over: 0,
    top: [{ login_id: SECRET_LOGIN_ID, traces: 7, bytes: 700, stored_bytes: 300, level: TRACE_QUOTA_LEVEL.OK }],
    analysis: { analysis_version: CURRENT_ANALYSIS_VERSIONS.play, pattern_analysis_version: CURRENT_ANALYSIS_VERSIONS.pattern, ...analysis },
})

const createFakePatterns = (params: { stale: readonly string[]; unreadable?: readonly string[] }) => {
    const stale = new Set(params.stale)
    const unreadable = new Set(params.unreadable)
    const calls: ChartPatternRecomputeInput[] = []
    const service = {
        coverage: async () => ({ tables: [], patterns_total: stale.size + 5, stale_features: stale.size, analysis_version: ANALYSIS_VERSION }),
        recomputeStale: async (input: ChartPatternRecomputeInput) => {
            calls.push(input)
            const md5s = [...stale]
                .toSorted()
                .filter((md5) => input.after === null || md5 > input.after)
                .slice(0, input.limit)
            const skipped = md5s.filter((md5) => unreadable.has(md5))
            md5s.filter((md5) => !unreadable.has(md5)).forEach((md5) => stale.delete(md5))
            return {
                analysis_version: ANALYSIS_VERSION,
                examined: md5s.length,
                recomputed: md5s.length - skipped.length,
                skipped,
                next_after: md5s.length === input.limit ? (md5s.at(-1) ?? null) : null,
                remaining: stale.size,
            }
        },
    } satisfies Pick<ChartPatternService, 'coverage' | 'recomputeStale'>
    return { service, calls, stale }
}

const createFakeTraces = (params: { missing: readonly string[]; stale: readonly string[]; unreadable?: readonly string[] }) => {
    const missing = new Set(params.missing)
    const stale = new Set(params.stale)
    const unreadable = new Set(params.unreadable)
    const calls: Parameters<TraceService['recomputeStale']>[0][] = []
    const service = {
        adminUsage: async () => usageOf({ missing: missing.size, stale: stale.size }),
        recomputeStale: async (input: Parameters<TraceService['recomputeStale']>[0]) => {
            calls.push(input)
            const ids = [...missing, ...stale]
                .toSorted()
                .filter((id) => input.after === null || id > input.after)
                .slice(0, input.limit)
            ids.forEach((id) => {
                missing.delete(id)
                stale.delete(id)
            })
            const skipped = ids.filter((id) => unreadable.has(id))
            return {
                analysis_version: CURRENT_ANALYSIS_VERSIONS.play,
                pattern_analysis_version: CURRENT_ANALYSIS_VERSIONS.pattern,
                examined: ids.length,
                recomputed: ids.length - skipped.length,
                skipped,
                next_after: ids.at(-1) ?? null,
                remaining: missing.size + stale.size,
            }
        },
    } satisfies Pick<TraceService, 'adminUsage' | 'recomputeStale'>
    return { service, calls, behind: () => missing.size + stale.size }
}

const createTickingNow = (tickMs: number) => {
    let time = 0
    return () => {
        const current = time
        time += tickMs
        return current
    }
}

const createRun = (params: {
    patterns?: Parameters<typeof createFakePatterns>[0]
    traces?: Parameters<typeof createFakeTraces>[0]
    tickMs?: number
}) => {
    const patterns = createFakePatterns(params.patterns ?? { stale: [] })
    const traces = createFakeTraces(params.traces ?? { missing: [], stale: [] })
    const lines: string[] = []
    const run = (command: RecomputeCommand, tickMs = params.tickMs ?? 0) =>
        recomputeAnalyses({
            command,
            patterns: patterns.service,
            traces: traces.service,
            now: createTickingNow(tickMs),
            log: (line) => lines.push(line),
        })
    return { patterns, traces, lines, run }
}

const runScript = async (args: string[]) => {
    const child = Bun.spawn([process.execPath, '--no-env-file', RECOMPUTE_SCRIPT, ...args], {
        cwd: WEB_DIRECTORY,
        env: { PATH: process.env.PATH ?? '' },
        stdout: 'pipe',
        stderr: 'pipe',
    })
    const [stdout, stderr, exitCode] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited])
    return { stdout, stderr, exitCode }
}

describe('recomputeAnalyses count', () => {
    test('낡은 차트 특징과 판 분석 수를 숫자만으로 한 줄 찍고 아무것도 다시 계산하지 않는다', async () => {
        const { patterns, traces, lines, run } = createRun({
            patterns: { stale: idsOf(3, md5Of) },
            traces: { missing: idsOf(2, traceIdOf), stale: idsOf(4, traceIdOf, 2) },
        })
        expect(await run(RECOMPUTE_COMMAND.COUNT)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual(['plan patternsStale 3, analysesMissing 2, analysesStale 4'])
        expect(patterns.calls).toEqual([])
        expect(traces.calls).toEqual([])
        expect(lines.join('\n')).not.toContain(SECRET_LOGIN_ID)
    })

    test('다시 계산할 것이 없으면 모두 0 이다', async () => {
        const { lines, run } = createRun({})
        expect(await run(RECOMPUTE_COMMAND.COUNT)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual(['plan patternsStale 0, analysesMissing 0, analysesStale 0'])
    })
})

describe('recomputeAnalyses apply', () => {
    test('차트 특징을 먼저 다 하고 그 뒤 판 분석을 배치 크기만큼씩 이어 간다', async () => {
        const patternMd5s = idsOf(ANALYSIS_RECOMPUTE_BATCH * 2 + 50, md5Of)
        const traceIds = idsOf(ANALYSIS_RECOMPUTE_BATCH + 30, traceIdOf)
        const { patterns, traces, lines, run } = createRun({
            patterns: { stale: patternMd5s },
            traces: { missing: traceIds.slice(0, 30), stale: traceIds.slice(30) },
        })
        expect(await run(RECOMPUTE_COMMAND.APPLY)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual(['applied patterns 250, analyses 130, skipped 0, remaining 0'])
        expect(patterns.calls).toEqual([
            { limit: ANALYSIS_RECOMPUTE_BATCH, after: null },
            { limit: ANALYSIS_RECOMPUTE_BATCH, after: patternMd5s[ANALYSIS_RECOMPUTE_BATCH - 1] ?? null },
            { limit: ANALYSIS_RECOMPUTE_BATCH, after: patternMd5s[ANALYSIS_RECOMPUTE_BATCH * 2 - 1] ?? null },
        ])
        expect(traces.calls).toEqual([
            { limit: ANALYSIS_RECOMPUTE_BATCH, after: null },
            { limit: ANALYSIS_RECOMPUTE_BATCH, after: traceIds[ANALYSIS_RECOMPUTE_BATCH - 1] ?? null },
            { limit: ANALYSIS_RECOMPUTE_BATCH, after: traceIds.at(-1) ?? null },
        ])
        expect(patterns.stale.size).toBe(0)
        expect(traces.behind()).toBe(0)
    })

    test('배치 크기의 배수만큼 있어도 빈 배치 한 번으로 끝난다', async () => {
        const { patterns, lines, run } = createRun({ patterns: { stale: idsOf(ANALYSIS_RECOMPUTE_BATCH * 2, md5Of) } })
        expect(await run(RECOMPUTE_COMMAND.APPLY)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual(['applied patterns 200, analyses 0, skipped 0, remaining 0'])
        expect(patterns.calls).toHaveLength(3)
    })

    test('읽을 수 없어 건너뛴 것은 건너뛴 수와 남은 수에만 잡히고 id 는 찍히지 않는다', async () => {
        const patternMd5s = idsOf(5, md5Of)
        const traceIds = idsOf(4, traceIdOf)
        const { lines, run } = createRun({
            patterns: { stale: patternMd5s, unreadable: [patternMd5s[1] ?? ''] },
            traces: { missing: traceIds.slice(0, 2), stale: traceIds.slice(2), unreadable: [traceIds[3] ?? ''] },
        })
        expect(await run(RECOMPUTE_COMMAND.APPLY)).toBe(EXIT_CODE.OK)
        expect(lines).toEqual(['applied patterns 4, analyses 3, skipped 2, remaining 1'])
        expect(lines.join('\n')).not.toContain(patternMd5s[1] ?? '')
        expect(lines.join('\n')).not.toContain(traceIds[3] ?? '')
    })

    test('끝난 뒤 다시 apply 하면 새로 계산하는 것이 없다', async () => {
        const { traces, lines, run } = createRun({ patterns: { stale: idsOf(3, md5Of) }, traces: { missing: idsOf(2, traceIdOf), stale: [] } })
        await run(RECOMPUTE_COMMAND.APPLY)
        await run(RECOMPUTE_COMMAND.APPLY)
        expect(lines).toEqual(['applied patterns 3, analyses 2, skipped 0, remaining 0', 'applied patterns 0, analyses 0, skipped 0, remaining 0'])
        expect(traces.behind()).toBe(0)
    })
})

describe('recomputeAnalyses 시간 상한', () => {
    const patternMd5s = idsOf(ANALYSIS_RECOMPUTE_BATCH * 10, md5Of)
    const traceIds = idsOf(ANALYSIS_RECOMPUTE_BATCH + 30, traceIdOf)
    const options = { patterns: { stale: patternMd5s }, traces: { missing: traceIds.slice(0, 30), stale: traceIds.slice(30) } }

    test('호출마다 3분씩 흐르면 세 번째 배치 전에 멈추고 stopped 한 줄을 찍은 뒤 성공 코드로 끝난다', async () => {
        const { patterns, traces, lines, run } = createRun(options)
        expect(await run(RECOMPUTE_COMMAND.APPLY, SLOW_BATCH_TICK_MS)).toBe(EXIT_CODE.OK)
        expect(patterns.calls).toHaveLength(2)
        expect(traces.calls).toEqual([])
        expect(lines).toEqual(['applied patterns 200, analyses 0, skipped 0, remaining 930', 'stopped elapsedMs 540000, remaining 930'])
        expect(lines.filter((line) => line.startsWith('stopped'))).toHaveLength(1)
    })

    test('차트 특징을 마친 뒤 판 분석 중에도 상한에 닿으면 거기서 멈추고 남은 수를 센다', async () => {
        const { patterns, traces, lines, run } = createRun({ ...options, patterns: { stale: idsOf(ANALYSIS_RECOMPUTE_BATCH, md5Of) } })
        expect(await run(RECOMPUTE_COMMAND.APPLY, SLOW_BATCH_TICK_MS)).toBe(EXIT_CODE.OK)
        expect(patterns.calls).toHaveLength(2)
        expect(traces.calls).toEqual([])
        expect(lines).toEqual(['applied patterns 100, analyses 0, skipped 0, remaining 130', 'stopped elapsedMs 540000, remaining 130'])
    })

    test('멈춘 뒤 다시 실행하면 앞서 계산한 것은 건너뛰고 이어서 끝까지 간다', async () => {
        const { patterns, traces, lines, run } = createRun(options)
        await run(RECOMPUTE_COMMAND.APPLY, SLOW_BATCH_TICK_MS)
        await run(RECOMPUTE_COMMAND.COUNT)
        await run(RECOMPUTE_COMMAND.APPLY)
        await run(RECOMPUTE_COMMAND.COUNT)
        expect(lines).toEqual([
            'applied patterns 200, analyses 0, skipped 0, remaining 930',
            'stopped elapsedMs 540000, remaining 930',
            'plan patternsStale 800, analysesMissing 30, analysesStale 100',
            'applied patterns 800, analyses 130, skipped 0, remaining 0',
            'plan patternsStale 0, analysesMissing 0, analysesStale 0',
        ])
        expect(patterns.stale.size).toBe(0)
        expect(traces.behind()).toBe(0)
    })

    test('상한 안에서 끝나면 stopped 줄이 없다', async () => {
        const { lines, run } = createRun({ patterns: { stale: idsOf(ANALYSIS_RECOMPUTE_BATCH, md5Of) } })
        await run(RECOMPUTE_COMMAND.APPLY, ANALYSIS_RECOMPUTE_MAX_MS / 4)
        expect(lines.some((line) => line.startsWith('stopped'))).toBe(false)
    })
})

describe('출력 줄', () => {
    test('숫자를 N 으로 바꾸면 고정 문장 셋 중 하나이고, 로그인 id 와 건너뛴 id 는 찍히지 않는다', async () => {
        const patternMd5s = idsOf(ANALYSIS_RECOMPUTE_BATCH * 3, md5Of)
        const traceIds = idsOf(ANALYSIS_RECOMPUTE_BATCH + 5, traceIdOf)
        const { lines, run } = createRun({
            patterns: { stale: patternMd5s, unreadable: patternMd5s.slice(0, 2) },
            traces: { missing: traceIds.slice(0, 3), stale: traceIds.slice(3), unreadable: traceIds.slice(-1) },
        })
        await run(RECOMPUTE_COMMAND.COUNT)
        await run(RECOMPUTE_COMMAND.APPLY, SLOW_BATCH_TICK_MS)
        await run(RECOMPUTE_COMMAND.APPLY)
        await run(RECOMPUTE_COMMAND.COUNT)
        expect(new Set(lines.map(sentenceOf))).toEqual(new Set(FIXED_SENTENCES))
        expect(lines.every((line) => FIXED_SENTENCES.includes(sentenceOf(line)))).toBe(true)
        expect(lines.join('\n')).not.toContain(SECRET_LOGIN_ID)
        expect(lines.join('\n')).not.toMatch(/[0-9a-f]{12,}|trace-/i)
    })
})

describe('시간 상한과 workflow 작업 제한', () => {
    const workflow = readFileSync(WORKFLOW_FILE, 'utf8')

    test('ANALYSIS_RECOMPUTE_MAX_MS 는 web-db.yml 의 timeout-minutes 보다 작다', () => {
        const minutes = Number(/timeout-minutes:\s*(\d+)/.exec(workflow)?.[1])
        expect(Number.isInteger(minutes)).toBe(true)
        expect(ANALYSIS_RECOMPUTE_MAX_MS).toBeLessThan(minutes * MS_PER_MINUTE)
    })

    test('배치 크기는 100 이다', () => {
        expect(ANALYSIS_RECOMPUTE_BATCH).toBe(100)
    })
})

describe('web-db.yml 의 두 action', () => {
    const workflow = readFileSync(WORKFLOW_FILE, 'utf8')
    const STEP_START = '      - name: '
    const stepsOf = (script: string) =>
        workflow
            .split(`\n${STEP_START}`)
            .slice(1)
            .filter((step) => step.includes(`scripts/${script}.ts`))
    const shapeOf = (step: string, action: string) => ({
        keys: [...step.matchAll(/^ {8}([\w-]+):/gm)].map((match) => match[1]),
        condition: /^ {8}if: (.+)$/m.exec(step)?.[1]?.replaceAll(action, 'ACTION'),
        argument: /run: bun scripts\/[\w-]+\.ts (\w+)$/m.exec(step)?.[1],
    })

    test('입력 choice 목록과 설명에 읽기 action 과 쓰기 action 이 있다', () => {
        const options = /options: \[(.*)\]/.exec(workflow)?.[1]?.split(', ')
        expect(options).toContain('analysis-recompute-count')
        expect(options).toContain('analysis-recompute')
        expect(options?.indexOf('analysis-recompute-count')).toBe((options?.indexOf('analysis-recompute') ?? 0) - 1)
        expect(/description: '.*analysis-recompute-count \/ analysis-recompute: /.test(workflow)).toBe(true)
    })

    test('단계 세 개가 dan-recompute 와 같은 순서와 모양이고 쓰기는 정확히 그 action 일 때만 돈다', () => {
        const dan = stepsOf('dan-recompute').map((step) => shapeOf(step, 'dan-recompute'))
        const analysis = stepsOf('analysis-recompute').map((step) => shapeOf(step, 'analysis-recompute'))
        expect(analysis).toEqual(dan)
        expect(analysis.map((step) => step.argument)).toEqual(['count', 'apply', 'count'])
        expect(analysis.map((step) => step.condition)).toEqual([
            "startsWith(inputs.action, 'ACTION')",
            "inputs.action == 'ACTION'",
            "inputs.action == 'ACTION'",
        ])
    })

    test('단계는 이름, 조건, 명령만 갖고 시크릿은 작업 수준 DATABASE_URL 로만 받는다', () => {
        const steps = stepsOf('analysis-recompute')
        expect(steps).toHaveLength(3)
        steps.forEach((step) => {
            expect([...step.matchAll(/^ {8}([\w-]+):/gm)].map((match) => match[1])).toEqual(['if', 'run'])
            expect(step).not.toMatch(/secrets\.|DATABASE_URL|env:/)
        })
        expect(workflow).toMatch(/\n {4}env:\n {6}DATABASE_URL: \$\{\{ secrets\.WEB_DATABASE_URL \}\}\n/)
    })
})

describe('analysis-recompute.ts 실행', () => {
    test('알 수 없는 인자는 사용법을 내고 종료 코드 2 다', async () => {
        const { stdout, stderr, exitCode } = await runScript(['unknown'])
        expect(exitCode).toBe(EXIT_CODE.USAGE)
        expect(stderr.trim()).toBe(USAGE)
        expect(stdout).toBe('')
    })

    test('데이터베이스가 정해지지 않았으면 접속하지 않고 종료 코드 1 이다', async () => {
        const { stdout, stderr, exitCode } = await runScript(['count'])
        expect(exitCode).toBe(EXIT_CODE.FAILED)
        expect(stderr).toContain('is not set')
        expect(stdout).toBe('')
    })
})

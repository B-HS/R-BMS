import { describe, expect, test } from 'bun:test'
import { CHART_PATTERN_WRITE_RATE_LIMIT } from '@server/dto/chart-pattern'
import { consumeRateLimit } from '@server/lib/with-rate-limit'
import {
    MAX_CONSECUTIVE_RATE_LIMIT_PAUSES,
    RATE_LIMIT_FALLBACK_PAUSE_MS,
    RATE_LIMIT_MAX_PAUSE_MS,
    RATE_LIMIT_MIN_PAUSE_MS,
    UPLOAD_CONCURRENCY,
    UPLOAD_REQUEST_INTERVAL_MS,
    formatSummary,
    uploadChartPatterns,
    type PatternFile,
} from '../../scripts/upload-chart-patterns'
import { makePatternDocument, type FixtureNote } from '../support/chart-pattern-fixture'
import { createVirtualClock } from '../support/virtual-clock'

const SERVER = 'http://127.0.0.1:3999'
const TOKEN = 'rbms_rate_limit_secret_R5'
const ALL_BMS_CHARTS = 7_903
const RETRY_AFTER_SECONDS = 7
const IN_FLIGHT_MS = 500
const MILLIS_PER_SECOND = 1000
const NOTES: FixtureNote[] = Array.from({ length: 8 }, (_, index): FixtureNote => [index * 100_000, index % 8, 0])

const md5Of = (index: number) => index.toString(16).padStart(32, '0')
const filesOf = (count: number): PatternFile[] =>
    Array.from({ length: count }, (_, index) => ({ md5: md5Of(index), path: `/virtual/${md5Of(index)}.json` }))
const readText = async (path: string) => JSON.stringify(makePatternDocument(NOTES, { md5: /([0-9a-f]{32})\.json$/.exec(path)?.[1] }))

const created = () => new Response(JSON.stringify({ success: true, data: { outcome: 'created' } }), { status: 201 })
const limited = (headers: Record<string, string> = {}) =>
    new Response(JSON.stringify({ success: false, error: { code: 'RATE_LIMITED', message: 'slow down' } }), { status: 429, headers })

describe('업로드 스크립트 — 서버 한도 준수', () => {
    test('요청 간격 상수는 서버 한도(분당 1,200)보다 느리다', () => {
        expect(UPLOAD_REQUEST_INTERVAL_MS * CHART_PATTERN_WRITE_RATE_LIMIT.limit).toBeGreaterThan(CHART_PATTERN_WRITE_RATE_LIMIT.windowMs)
    })

    test('7,903곡을 한 번에 올려도 서버의 고정 창 한도에 한 번도 걸리지 않고, 요청 시작은 간격 상수 이상으로 벌어진다', async () => {
        const clock = createVirtualClock()
        const sentAt: number[] = []
        let rejected = 0
        const fetch = async () => {
            sentAt.push(clock.now())
            const { allowed } = consumeRateLimit(
                'chart-pattern-write:admin',
                CHART_PATTERN_WRITE_RATE_LIMIT.limit,
                CHART_PATTERN_WRITE_RATE_LIMIT.windowMs,
                clock.now(),
            )
            if (allowed) return created()
            rejected += 1
            return limited()
        }
        const summary = await uploadChartPatterns({
            files: filesOf(ALL_BMS_CHARTS),
            skipped: 0,
            server: SERVER,
            token: TOKEN,
            dryRun: false,
            fetch,
            readText,
            clock,
        })
        expect(rejected).toBe(0)
        expect(summary).toMatchObject({ files: ALL_BMS_CHARTS, created: ALL_BMS_CHARTS, stoppedByRateLimit: false, rateLimitPauses: 0 })
        expect(summary.failures).toEqual([])
        expect(sentAt).toHaveLength(ALL_BMS_CHARTS)
        const gaps = sentAt.slice(1).map((time, index) => time - sentAt[index])
        expect(Math.min(...gaps)).toBeGreaterThanOrEqual(UPLOAD_REQUEST_INTERVAL_MS)
        const mostInOneWindow = sentAt.reduce((most, _, index) => {
            const first = sentAt.findIndex((time) => time > sentAt[index] - CHART_PATTERN_WRITE_RATE_LIMIT.windowMs)
            return Math.max(most, index - first + 1)
        }, 0)
        expect(mostInOneWindow).toBeLessThanOrEqual(CHART_PATTERN_WRITE_RATE_LIMIT.limit)
    })
})

describe('업로드 스크립트 — 429 를 받으면 멈췄다가 같은 파일을 다시 보낸다', () => {
    test('retry-after 만큼 모든 워커가 멈춘 뒤에 같은 파일이 다시 가고, 실패로 세지 않는다', async () => {
        const clock = createVirtualClock()
        const sent: { md5: string; at: number }[] = []
        let limitedOnce = false
        const fetch = async (url: string) => {
            sent.push({ md5: url.slice(-32), at: clock.now() })
            if (!limitedOnce && sent.length === UPLOAD_CONCURRENCY) {
                limitedOnce = true
                return limited({ 'retry-after': String(RETRY_AFTER_SECONDS) })
            }
            return created()
        }
        const files = filesOf(UPLOAD_CONCURRENCY * 3)
        const summary = await uploadChartPatterns({ files, skipped: 0, server: SERVER, token: TOKEN, dryRun: false, fetch, readText, clock })
        expect(summary).toMatchObject({ created: files.length, rateLimitPauses: 1, stoppedByRateLimit: false })
        expect(summary.failures).toEqual([])
        const limitedAt = sent[UPLOAD_CONCURRENCY - 1].at
        const limitedMd5 = sent[UPLOAD_CONCURRENCY - 1].md5
        const laterRequests = sent.slice(UPLOAD_CONCURRENCY)
        expect(laterRequests.every((request) => request.at >= limitedAt + RETRY_AFTER_SECONDS * MILLIS_PER_SECOND)).toBe(true)
        expect(laterRequests.map((request) => request.md5)).toContain(limitedMd5)
        expect(sent.filter((request) => request.md5 === limitedMd5)).toHaveLength(2)
        expect(new Set(sent.map((request) => request.md5)).size).toBe(files.length)
    })

    test('요청이 날아가는 동안 429 를 받은 워커 여럿은 대기 한 번으로 센다', async () => {
        const clock = createVirtualClock()
        let calls = 0
        const fetch = async () => {
            calls += 1
            const isLimited = calls <= UPLOAD_CONCURRENCY
            await clock.sleep(IN_FLIGHT_MS)
            return isLimited ? limited({ 'retry-after': '2' }) : created()
        }
        const summary = await uploadChartPatterns({
            files: filesOf(UPLOAD_CONCURRENCY),
            skipped: 0,
            server: SERVER,
            token: TOKEN,
            dryRun: false,
            fetch,
            readText,
            clock,
        })
        expect(summary).toMatchObject({ created: UPLOAD_CONCURRENCY, rateLimitPauses: 1 })
        expect(calls).toBe(UPLOAD_CONCURRENCY * 2)
    })

    test('retry-after 가 없으면 서버 창 길이만큼, 너무 길게 알려 오면 상한만큼 기다린다', async () => {
        const longestPauseFor = async (headers: Record<string, string>) => {
            const clock = createVirtualClock()
            const sleeps: number[] = []
            let isFirst = true
            const fetch = async () => {
                const wasFirst = isFirst
                isFirst = false
                return wasFirst ? limited(headers) : created()
            }
            await uploadChartPatterns({
                files: filesOf(1),
                skipped: 0,
                server: SERVER,
                token: TOKEN,
                dryRun: false,
                fetch,
                readText,
                concurrency: 1,
                clock: {
                    now: clock.now,
                    sleep: async (ms) => {
                        sleeps.push(ms)
                        await clock.sleep(ms)
                    },
                },
            })
            return Math.max(...sleeps)
        }
        expect(await longestPauseFor({})).toBe(RATE_LIMIT_FALLBACK_PAUSE_MS)
        expect(await longestPauseFor({ 'retry-after': '86400' })).toBe(RATE_LIMIT_MAX_PAUSE_MS)
        expect(await longestPauseFor({ 'retry-after': '0' })).toBe(RATE_LIMIT_MIN_PAUSE_MS)
        expect(await longestPauseFor({ 'retry-after': 'Thu, 01 Jan 1970 00:00:00 GMT' })).toBe(RATE_LIMIT_MIN_PAUSE_MS)
    })

    test('429 응답의 본문은 다시 보내기 전에 읽어 버린다', async () => {
        const clock = createVirtualClock()
        const responses: Response[] = []
        let isFirst = true
        const fetch = async () => {
            const response = isFirst ? limited({ 'retry-after': '1' }) : created()
            isFirst = false
            responses.push(response)
            return response
        }
        const summary = await uploadChartPatterns({
            files: filesOf(1),
            skipped: 0,
            server: SERVER,
            token: TOKEN,
            dryRun: false,
            fetch,
            readText,
            clock,
            concurrency: 1,
        })
        expect(summary.created).toBe(1)
        expect(responses.map((response) => response.bodyUsed)).toEqual([true, true])
    })

    test('429 가 계속되면 최대 대기 횟수 뒤에 그 파일을 실패로 보고하고 나머지를 보내지 않는다', async () => {
        const clock = createVirtualClock()
        const sent: string[] = []
        const fetch = async (url: string) => {
            sent.push(url)
            return limited()
        }
        const files = filesOf(UPLOAD_CONCURRENCY * 4)
        const summary = await uploadChartPatterns({ files, skipped: 0, server: SERVER, token: TOKEN, dryRun: false, fetch, readText, clock })
        expect(summary.stoppedByRateLimit).toBe(true)
        expect(summary.rateLimitPauses).toBe(MAX_CONSECUTIVE_RATE_LIMIT_PAUSES)
        expect(summary.created).toBe(0)
        expect(summary.failures.length).toBeGreaterThan(0)
        expect(summary.failures.every((failure) => failure.status === 429 && failure.reason.includes('RATE_LIMITED'))).toBe(true)
        expect(new Set(sent).size).toBeLessThan(files.length)
        const text = formatSummary(summary, false)
        expect(text).toContain('속도 제한')
        expect(text).not.toContain(TOKEN)
    })

    test('429 사이에 성공이 있으면 연속 횟수를 다시 센다', async () => {
        const clock = createVirtualClock()
        let calls = 0
        const fetch = async () => {
            calls += 1
            return calls % 2 === 1 ? limited({ 'retry-after': '1' }) : created()
        }
        const files = filesOf((MAX_CONSECUTIVE_RATE_LIMIT_PAUSES + 3) * 2)
        const summary = await uploadChartPatterns({
            files,
            skipped: 0,
            server: SERVER,
            token: TOKEN,
            dryRun: false,
            fetch,
            readText,
            clock,
            concurrency: 1,
        })
        expect(summary).toMatchObject({ created: files.length, stoppedByRateLimit: false })
        expect(summary.rateLimitPauses).toBe(files.length)
    })
})

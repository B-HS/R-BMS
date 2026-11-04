import { createHash } from 'node:crypto'
import { describe, expect, test } from 'bun:test'
import { weaknessProfileViewSchema } from '@server/dto/ai-play'
import { PROFILE_MAX_PLAYS, PROFILE_VERSION, PROFILE_WINDOW_KEY } from '@server/service/domain/ai/play-feature.constants'
import {
    createWeaknessProfileService,
    profileFingerprintOf,
    profileWindowStartOf,
    toProfileDigest,
} from '@server/service/domain/ai/weakness-profile.service'
import { CURRENT_ANALYSIS_VERSIONS } from '@server/service/domain/trace/trace-analysis'
import { countTraceJudgements } from '@server/service/domain/trace/trace-check'
import type { TraceService } from '@server/service/domain/trace/trace.service'
import { createMemoryProfileDb } from '../../support/ai-memory-db'
import { analyzedPlayOf, tallyOf, withMetrics } from '../../support/play-feature-fixture'
import { TRACE_USER_ID, createMemoryTraceDb, createMemoryTraceService, readContractTrace, scoreRowFor } from '../../support/trace-fixture'

const NORMAL = readContractTrace('trace-normal.json')
const DAY_MS = 86_400_000
const ISO_DATE_LENGTH = 10
const NOW = Date.UTC(2026, 10, 4, 15)
const WINDOW_START = Date.UTC(2026, 7, 6)
const INPUTS = { analyses: 12, latestComputedAt: NOW - 1000, latestPlayedAt: NOW - 2000, trainingCharts: 0 }

/** Twelve plays on six charts whose trill notes lose EX against the play: a strong `tag:trill` weakness. */
const weakTrillPlays = () =>
    Array.from({ length: 12 }, (_, index) => ({
        ...withMetrics(analyzedPlayOf(NORMAL, { scoreId: `sc_${index}`, playedAt: NOW - index * DAY_MS }), {
            overall: tallyOf({ pgreat: 320, good: 80 }),
            by_tag: { trill: tallyOf({ pgreat: 50 + (index % 3), good: 50 - (index % 3) }) },
        }),
        chartSha256: String(index % 6).repeat(64),
        isTraining: false,
    }))

type WindowPlays = Awaited<ReturnType<TraceService['analyzedPlaysInWindow']>>

const setup = (options: { stale?: number } = {}) => {
    const memory = createMemoryProfileDb(INPUTS)
    const calls: { sinceMs: number; limit: number }[] = []
    const result: { value: WindowPlays } = { value: { plays: weakTrillPlays(), stale: options.stale ?? 0, failed: 0 } }
    const service = createWeaknessProfileService({
        db: memory.db,
        plays: {
            analyzedPlaysInWindow: async ({ sinceMs, limit }) => {
                calls.push({ sinceMs, limit })
                return result.value
            },
        },
        now: () => NOW,
    })
    return { memory, calls, result, service }
}

describe('약점 프로필 서비스 — 지문 캐시(기획 2.5.1)', () => {
    test('창은 90일 전 UTC 자정부터 최신 300판이고, 계산한 프로필을 지문과 함께 저장한다', async () => {
        const context = setup()
        const view = await context.service.view({ userId: TRACE_USER_ID })
        expect(context.calls).toEqual([{ sinceMs: WINDOW_START, limit: PROFILE_MAX_PLAYS }])
        expect(weaknessProfileViewSchema.safeParse(view).success).toBe(true)
        expect(view).toMatchObject({ profile_version: PROFILE_VERSION, window: { from: WINDOW_START, to: NOW, days: 90, plays: 12, charts: 6 } })
        expect(view.weaknesses.map((item) => item.key)).toEqual(['tag:trill'])
        expect(context.memory.saves).toEqual([TRACE_USER_ID])
        expect(context.memory.profiles.get(`${TRACE_USER_ID}|${PROFILE_WINDOW_KEY}`)).toMatchObject({
            profileVersion: PROFILE_VERSION,
            fingerprint: profileFingerprintOf(INPUTS, WINDOW_START),
            computedAt: NOW,
        })
    })

    test('지문이 같으면 판을 다시 읽지 않고 저장값을 돌려주며, 지문이 바뀌면 다시 계산한다', async () => {
        const context = setup()
        const first = await context.service.view({ userId: TRACE_USER_ID })
        const second = await context.service.current({ userId: TRACE_USER_ID })
        expect(context.calls).toHaveLength(1)
        expect(second).toMatchObject({ cached: true, computedAt: NOW, fingerprint: profileFingerprintOf(INPUTS, WINDOW_START) })
        expect(second.profile).toEqual(first)
        context.memory.state.inputs = { ...INPUTS, analyses: INPUTS.analyses + 1 }
        expect((await context.service.current({ userId: TRACE_USER_ID })).cached).toBe(false)
        expect(context.calls).toHaveLength(2)
        expect(context.memory.saves).toHaveLength(2)
    })

    test('낡은 판이 남은 채 계산한 프로필은 저장하지 않아 다음 읽기에서 다시 계산한다', async () => {
        const context = setup({ stale: 3 })
        expect((await context.service.view({ userId: TRACE_USER_ID })).window.stale).toBe(3)
        await context.service.view({ userId: TRACE_USER_ID })
        expect(context.calls).toHaveLength(2)
        expect(context.memory.saves).toEqual([])
    })

    test('저장값의 버전이 다르거나 모양이 깨졌으면 지문이 같아도 다시 계산한다', async () => {
        const context = setup()
        const key = `${TRACE_USER_ID}|${PROFILE_WINDOW_KEY}`
        const fingerprint = profileFingerprintOf(INPUTS, WINDOW_START)
        context.memory.profiles.set(key, { profileVersion: PROFILE_VERSION - 1, fingerprint, data: {}, computedAt: 1 })
        await context.service.view({ userId: TRACE_USER_ID })
        context.memory.profiles.set(key, { profileVersion: PROFILE_VERSION, fingerprint, data: { cells: 'broken' }, computedAt: 1 })
        await context.service.view({ userId: TRACE_USER_ID })
        expect(context.calls).toHaveLength(2)
    })

    test('지문은 창 시작 날짜와 개수가 바뀔 때 바뀌고 같은 입력이면 같다', () => {
        const base = profileFingerprintOf(INPUTS, WINDOW_START)
        expect(profileFingerprintOf({ ...INPUTS }, WINDOW_START)).toBe(base)
        expect(profileFingerprintOf(INPUTS, WINDOW_START + DAY_MS)).not.toBe(base)
        expect(profileFingerprintOf({ ...INPUTS, trainingCharts: 1 }, WINDOW_START)).not.toBe(base)
        expect(profileFingerprintOf({ ...INPUTS, latestComputedAt: INPUTS.latestComputedAt + 1 }, WINDOW_START)).not.toBe(base)
        expect(base).toMatch(/^[0-9a-f]{64}$/)
        expect(profileWindowStartOf(NOW)).toBe(WINDOW_START)
    })

    test('지문은 프로필 버전을 입력에 넣어, 버전이 오르면 같은 입력도 지문이 달라져 저장된 프로필이 무효가 된다', () => {
        const fingerprintAt = (version: number) =>
            createHash('sha256')
                .update(
                    JSON.stringify([
                        version,
                        CURRENT_ANALYSIS_VERSIONS,
                        new Date(WINDOW_START).toISOString().slice(0, ISO_DATE_LENGTH),
                        INPUTS.analyses,
                        INPUTS.latestComputedAt,
                        INPUTS.latestPlayedAt,
                        INPUTS.trainingCharts,
                    ]),
                )
                .digest('hex')
        expect(profileFingerprintOf(INPUTS, WINDOW_START)).toBe(fingerprintAt(PROFILE_VERSION))
        expect(profileFingerprintOf(INPUTS, WINDOW_START)).not.toBe(fingerprintAt(PROFILE_VERSION - 1))
    })
})

describe('약점 프로필 서비스 — digest 와 측정', () => {
    test('digest 는 low 이상 약점·강점·문맥·손과 그 키만 담고 판정이 갈리지 않은 태그는 이름만 준다', async () => {
        const context = setup()
        const digest = await context.service.digest({ userId: TRACE_USER_ID })
        expect(digest.weaknesses.map((item) => item.key)).toEqual(['tag:trill'])
        expect(digest.weaknesses[0]).toMatchObject({ kind: 'tag', unit: 'ex_residual', evidence: 'strong', sample_level: 'low' })
        expect(digest.window).toEqual({ key: 'profile.window', plays: 12, charts: 6, days: 90 })
        expect(digest.density).toMatchObject({ key: 'density:limit', evidence: 'insufficient' })
        expect(toProfileDigest(await context.service.view({ userId: TRACE_USER_ID }))).toEqual(digest)
    })

    test('측정은 주어진 시각 뒤 판만으로 키마다 값을 주고 캐시를 쓰지 않는다', async () => {
        const context = setup()
        const since = NOW - 5 * DAY_MS
        const measured = await context.service.measureFocus({ userId: TRACE_USER_ID, keys: ['tag:trill', 'lane:3'], sinceMs: since })
        expect(context.calls).toEqual([{ sinceMs: since, limit: PROFILE_MAX_PLAYS }])
        expect(measured['tag:trill']).toMatchObject({ unit: 'ex_residual', evidence: 'strong', charts: 6, plays: 12 })
        expect(measured['lane:3']).toBeNull()
        expect(context.memory.saves).toEqual([])
    })
})

describe('trace 서비스 — 창 안 분석 판(기획 2.5.1)', () => {
    test('창 시작 뒤 판만 최신순으로 주고, 그 사용자의 훈련곡 판에 표시를 단다', async () => {
        const later = { ...NORMAL, score_id: 'sc_later' }
        const memory = createMemoryTraceDb([
            scoreRowFor(NORMAL, { ...countTraceJudgements(NORMAL), playedAt: 1000 }),
            scoreRowFor(later, { ...countTraceJudgements(later), playedAt: 5000 }),
        ])
        memory.trainingCharts.add(`${TRACE_USER_ID}|${NORMAL.chart.sha256}`)
        const { service } = createMemoryTraceService(memory)
        await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        await service.upload({ input: later, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        const window = await service.analyzedPlaysInWindow({ userId: TRACE_USER_ID, sinceMs: 2000, limit: 10 })
        expect(window.plays.map((play) => [play.scoreId, play.isTraining])).toEqual([['sc_later', true]])
        const all = await service.analyzedPlaysInWindow({ userId: TRACE_USER_ID, sinceMs: 0, limit: 10 })
        expect(all.plays.map((play) => play.scoreId)).toEqual(['sc_later', NORMAL.score_id])
        expect(all.plays.every((play) => play.summary.metrics !== null)).toBe(true)
        expect(await service.analyzedPlaysInWindow({ userId: 'usr_nobody', sinceMs: 0, limit: 10 })).toEqual({ plays: [], stale: 0, failed: 0 })
    })

    test('7키가 아닌 판은 분석돼도 창 판에 들지 않는다', async () => {
        const doubles = { ...NORMAL, score_id: 'sc_14k', mode: 'BEAT_14K' }
        const memory = createMemoryTraceDb([scoreRowFor(doubles, countTraceJudgements(doubles))])
        const { service } = createMemoryTraceService(memory)
        await service.upload({ input: doubles, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        expect((await service.analyzedPlaysInWindow({ userId: TRACE_USER_ID, sinceMs: 0, limit: 10 })).plays).toEqual([])
    })
})

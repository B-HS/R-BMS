import { describe, expect, test } from 'bun:test'
import { trainingChartDetailSchema, trainingChartListSchema, trainingRemovalSchema } from '@server/dto/training'
import { FOLDER_LIMIT } from '@server/dto/folder'
import { countTraceJudgements } from '@server/service/domain/trace/trace-check'
import { trainingFolderNoteOf } from '@server/service/domain/training/training.service'
import {
    TRAINING_NOW,
    TRAINING_ORIGIN,
    TRAINING_OTHER_USER,
    TRAINING_USER,
    createTrainingHarness,
    trainingProfileOf,
    trainingWeaknessOf,
} from '../../support/training-fixture'
import { TRACE_USER_ID, createMemoryTraceDb, createMemoryTraceService, readContractTrace, scoreRowFor } from '../../support/trace-fixture'

const PUSH = { focus_key: 'auto', intensity: 'push' } as const
const DAY_MS = 86_400_000

const expectOk = <T>(result: { ok: true; value: T } | { ok: false }) => {
    if (!result.ok) return expect.unreachable(`expected ok, got ${JSON.stringify(result)}`)
    return result.value
}

const createReady = async (options: Parameters<typeof createTrainingHarness>[0] = {}) => {
    const harness = createTrainingHarness(options)
    await harness.registerKey()
    return harness
}

const trainingFolderOf = async (harness: Awaited<ReturnType<typeof createReady>>) => {
    const folderId = await harness.training.db.findFolderLink(TRAINING_USER)
    return folderId ? harness.folderService.get({ userId: TRAINING_USER, folderId }) : null
}

describe('훈련곡 만들기', () => {
    test('약점 프로필로 한 번 부르고 곡을 저장해 훈련곡 폴더 맨 위에 넣고 패턴을 등록한다', async () => {
        const harness = await createReady()
        const detail = expectOk(await harness.create(PUSH))
        expect(trainingChartDetailSchema.safeParse(detail).success).toBe(true)
        expect(harness.anthropic.generateCalls).toHaveLength(1)
        expect(detail.title).toBe('R-BMS Training · 2026-11-04 · PUSH')
        expect(detail.download_url).toBe(`${TRAINING_ORIGIN}/api/training/${detail.id}/chart.zip`)
        expect(detail.focus_keys).toEqual(['tag:trill', 'tag:jack'])
        expect(detail.sections.length).toBeGreaterThanOrEqual(3)
        expect(detail.before_after.map((item) => item.focus_key)).toEqual(['tag:trill', 'tag:jack'])
        expect(detail.before_after[0]).toMatchObject({ before: { value: -0.05, unit: 'ex_residual' }, after: null, delta: null })

        const stored = harness.training.charts.get(detail.id)
        expect(stored?.userId).toBe(TRAINING_USER)
        expect(stored?.request).toMatchObject({ focus_key: 'auto', intensity: 'push' })
        expect(stored?.profileSnapshot.fingerprint).toBe('f'.repeat(64))
        expect(new TextDecoder().decode(stored?.bms)).toContain(`#SUBTITLE [${detail.id.slice(3, 11)}]`)
        expect(harness.uploads.map((document) => document.md5)).toEqual([stored?.md5 ?? ''])

        const folder = await trainingFolderOf(harness)
        expect(folder?.name).toBe('훈련곡')
        expect(folder?.is_public).toBe(false)
        expect(folder?.items).toEqual([
            {
                md5: stored?.md5 ?? '',
                sha256: stored?.sha256 ?? '',
                title: detail.title,
                artist: 'R-BMS',
                note: stored ? trainingFolderNoteOf(stored) : '',
            },
        ])
        expect(folder?.items[0]?.note).toStartWith('tag:trill, tag:jack · ')
        expect(harness.failures).toEqual([])
    })

    test('digest 가 같으면 캐시된 구성안을 다시 쓰고 공급자를 다시 부르지 않지만 곡은 시각이 달라 새 바이트다', async () => {
        const harness = await createReady()
        const first = expectOk(await harness.create(PUSH))
        const firstMd5 = harness.training.charts.get(first.id)?.md5
        expect(await harness.service.remove({ userId: TRAINING_USER, id: first.id })).not.toBeNull()
        harness.state.now += 1
        const second = expectOk(await harness.create(PUSH))
        expect(harness.anthropic.generateCalls).toHaveLength(1)
        expect(harness.training.charts.get(second.id)?.md5).not.toBe(firstMd5)
        expect(second.sections.map((section) => section.focus_key)).toEqual(first.sections.map((section) => section.focus_key))
    })

    test('앞서 만든 곡은 다음 digest 의 previous 에 들어간다', async () => {
        const harness = await createReady()
        expectOk(await harness.create(PUSH))
        harness.state.now += 1
        expectOk(await harness.create(PUSH))
        expect(harness.anthropic.generateCalls).toHaveLength(2)
        const digest = JSON.parse(harness.anthropic.generateCalls[1]?.input ?? '{}')
        expect(digest.previous).toEqual([
            { created_at: TRAINING_NOW, focus_keys: ['tag:trill', 'tag:jack'], kinds: expect.any(Array), result_delta: null },
        ])
        expect(digest.frontier_level).toBe('★12')
    })

    test('20곡을 보관 중이면 공급자를 부르지 않고 chart_limit 이다', async () => {
        const harness = await createReady()
        for (let index = 0; index < 20; index += 1) {
            harness.state.now += 1
            expectOk(await harness.create(PUSH))
        }
        const calls = harness.anthropic.generateCalls.length
        expect(await harness.create(PUSH)).toEqual({ ok: false, reason: 'chart_limit', used: 20, limit: 20 })
        expect(harness.anthropic.generateCalls).toHaveLength(calls)
    })

    test('저장 단계에서 한도 경쟁에 지면 chart_limit 이고 아무것도 남지 않는다', async () => {
        const harness = await createReady()
        harness.training.state.insertResult = 'limit'
        expect(await harness.create(PUSH)).toMatchObject({ ok: false, reason: 'chart_limit' })
        expect(harness.training.charts.size).toBe(0)
        expect(await trainingFolderOf(harness)).toBeNull()
        expect(harness.uploads).toEqual([])
    })

    test('AI 키가 없으면 프로필을 읽기 전에 credential_not_found 이고 공급자 호출이 없다', async () => {
        const harness = createTrainingHarness()
        expect(await harness.create(PUSH)).toEqual({ ok: false, reason: 'credential_not_found' })
        expect(harness.anthropic.generateCalls).toEqual([])
    })

    test('판이 PROFILE_MIN_PLAYS 보다 적으면 traced_plays 가 모자라다고 하고 공급자를 부르지 않는다', async () => {
        const harness = await createReady({ profile: trainingProfileOf({ plays: 9 }) })
        expect(await harness.create(PUSH)).toEqual({ ok: false, reason: 'not_enough_data', need: 'traced_plays' })
        expect(harness.anthropic.generateCalls).toEqual([])
    })

    test('프로필에 없는 키와, 훈련할 종류가 없는 약점뿐인 auto 는 weakness_profile 이 모자라다고 한다', async () => {
        const harness = await createReady()
        expect(await harness.create({ focus_key: 'tag:stair', intensity: 'build' })).toEqual({
            ok: false,
            reason: 'not_enough_data',
            need: 'weakness_profile',
        })
        harness.state.profile = trainingProfileOf({ weaknesses: [trainingWeaknessOf('timing:trill', 'timing', { training_kinds: [] })] })
        expect(await harness.create(PUSH)).toEqual({ ok: false, reason: 'not_enough_data', need: 'weakness_profile' })
        harness.state.profile = trainingProfileOf({ weaknesses: [trainingWeaknessOf('tag:trill', 'tag', { sample_level: 'insufficient' })] })
        expect(await harness.create(PUSH)).toEqual({ ok: false, reason: 'not_enough_data', need: 'weakness_profile' })
        expect(harness.anthropic.generateCalls).toEqual([])
    })

    test('키를 고르면 digest 의 약점과 허용 키가 그 키로 줄어든다', async () => {
        const harness = await createReady()
        const detail = expectOk(await harness.create({ focus_key: 'tag:jack', intensity: 'over' }))
        const digest = JSON.parse(harness.anthropic.generateCalls[0]?.input ?? '{}')
        expect(digest.allowed.keys).toEqual(['tag:jack', 'warmup', 'cooldown'])
        expect(digest.weaknesses.map((item: { key: string }) => item.key)).toEqual(['tag:jack'])
        expect(detail.focus_keys).toEqual(['tag:jack'])
        expect(detail.title).toEndWith('· OVER')
    })

    test('허용 키 구간이 3개 미만인 출력은 output_invalid 이고 저장하지 않는다', async () => {
        const harness = await createReady({
            plan: (digest) => ({
                description: '',
                base_bpm: 150,
                sections: ['tag:stair', 'tag:stair', digest.allowed.keys[0] ?? '', 'lane:1'].map((key) => ({
                    focus_key: key,
                    kind: 'stream',
                    weight: 1,
                    target_nps: 8,
                    lane_hint: 'all',
                    chord_size: 1,
                    scratch_ratio: 0,
                    ln_ratio: 0,
                    bpm_ratio: 1,
                    intent: '',
                })),
                advice: '',
            }),
        })
        expect(await harness.create(PUSH)).toEqual({ ok: false, reason: 'output_invalid', provider: 'anthropic' })
        expect(harness.anthropic.generateCalls).toHaveLength(1)
        expect(harness.training.charts.size).toBe(0)
    })

    test('허용 밖 키 구간은 버리고 맞지 않는 종류는 바꿔 corrections 에 남긴다', async () => {
        const harness = await createReady({
            plan: () => ({
                description: '보정 시험',
                base_bpm: 150,
                sections: [
                    { focus_key: 'warmup', kind: 'stream' },
                    { focus_key: 'tag:stair', kind: 'stair' },
                    { focus_key: 'tag:trill', kind: 'jack' },
                    { focus_key: 'tag:jack', kind: 'jack' },
                    { focus_key: 'cooldown', kind: 'rest' },
                ].map((section) => ({
                    ...section,
                    weight: 2,
                    target_nps: 9,
                    lane_hint: 'all',
                    chord_size: 1,
                    scratch_ratio: 0,
                    ln_ratio: 0,
                    bpm_ratio: 1,
                    intent: '한 줄',
                })),
                advice: '조언',
            }),
        })
        const detail = expectOk(await harness.create(PUSH))
        expect(detail.corrections).toContainEqual({ section: 1, field: 'focus_key', from: 'tag:stair', to: null })
        expect(detail.corrections).toContainEqual({ section: 2, field: 'kind', from: 'jack', to: 'trill' })
        expect(detail.sections.map((section) => section.focus_key)).toEqual(['warmup', 'tag:trill', 'tag:jack', 'cooldown'])
    })
})

describe('훈련곡 폴더', () => {
    test('폴더가 이미 있으면 새로 만들지 않고 새 곡을 맨 위에 넣는다', async () => {
        const harness = await createReady()
        const first = expectOk(await harness.create(PUSH))
        harness.state.now += 1
        const second = expectOk(await harness.create(PUSH))
        expect(await harness.folderService.list(TRAINING_USER)).toHaveLength(1)
        const titles = (await trainingFolderOf(harness))?.items.map((item) => item.md5)
        expect(titles).toEqual([harness.training.charts.get(second.id)?.md5 ?? '', harness.training.charts.get(first.id)?.md5 ?? ''])
    })

    test('같은 이름의 폴더가 있으면 UTC 날짜를 붙여 만들고 화면 언어 이름을 쓴다', async () => {
        const harness = await createReady()
        expectOk(await harness.folderService.create({ userId: TRAINING_USER, input: { name: 'Training', description: '' } }))
        expectOk(await harness.create(PUSH, TRAINING_USER, 'en'))
        expect((await trainingFolderOf(harness))?.name).toBe('Training 2026-11-04')
    })

    test('폴더가 50개라 만들 수 없으면 공급자를 부르지 않고 folder_unavailable 이다', async () => {
        const harness = await createReady()
        for (let index = 0; index < FOLDER_LIMIT.FOLDERS_PER_USER; index += 1) {
            expectOk(await harness.folderService.create({ userId: TRAINING_USER, input: { name: `f${index}`, description: '' } }))
        }
        expect(await harness.create(PUSH)).toEqual({ ok: false, reason: 'folder_unavailable', limit: FOLDER_LIMIT.FOLDERS_PER_USER })
        expect(harness.anthropic.generateCalls).toEqual([])
    })

    test('훈련곡 3곡 뒤 폴더를 지우고 다시 만들면 새 폴더에 4곡이 최신순으로 들어간다', async () => {
        const harness = await createReady()
        const made: string[] = []
        for (let index = 0; index < 3; index += 1) {
            harness.state.now += DAY_MS
            made.push(expectOk(await harness.create(PUSH)).id)
        }
        const folder = await trainingFolderOf(harness)
        expect(await harness.folderService.remove({ userId: TRAINING_USER, folderId: folder?.id ?? '' })).toBe(true)
        harness.state.now += DAY_MS
        made.push(expectOk(await harness.create(PUSH)).id)
        const again = await trainingFolderOf(harness)
        expect(again?.id).not.toBe(folder?.id)
        expect(again?.items.map((item) => item.md5)).toEqual(made.toReversed().map((id) => harness.training.charts.get(id)?.md5 ?? ''))
    })
})

describe('목록·상세·지우기', () => {
    test('목록은 주인 곡만 최신순으로, 사용량과 키 상태를 함께 준다', async () => {
        const harness = await createReady()
        const made = expectOk(await harness.create(PUSH))
        const list = await harness.service.list({ userId: TRAINING_USER, origin: TRAINING_ORIGIN })
        expect(trainingChartListSchema.safeParse(list).success).toBe(true)
        expect(list).toMatchObject({ used: 1, limit: 20, can_create: true, needs_key: false })
        expect(list.charts.map((chart) => chart.id)).toEqual([made.id])
        expect(await harness.service.list({ userId: TRAINING_OTHER_USER, origin: TRAINING_ORIGIN })).toMatchObject({
            charts: [],
            used: 0,
            needs_key: true,
        })
    })

    test('다른 사용자의 id 로는 상세·removal·지우기가 모두 null 이고 곡이 남는다', async () => {
        const harness = await createReady()
        const made = expectOk(await harness.create(PUSH))
        const params = { userId: TRAINING_OTHER_USER, id: made.id }
        expect(await harness.service.detail({ ...params, origin: TRAINING_ORIGIN })).toBeNull()
        expect(await harness.service.removal(params)).toBeNull()
        expect(await harness.service.remove(params)).toBeNull()
        expect(harness.training.charts.has(made.id)).toBe(true)
    })

    test('지우기 전 개수를 세고, 지우면 모든 폴더의 항목이 빠지고 판·패턴은 남는다', async () => {
        const harness = await createReady()
        const made = expectOk(await harness.create(PUSH))
        const stored = harness.training.charts.get(made.id)
        const other = expectOk(await harness.folderService.create({ userId: TRAINING_USER, input: { name: 'mine', description: '' } }))
        await harness.folderService.addItems({
            userId: TRAINING_USER,
            folderId: other.id,
            items: [{ md5: stored?.md5 ?? '', sha256: '', title: '', artist: '', note: '' }],
        })
        harness.training.kept.patterns.add(stored?.md5 ?? '')
        harness.training.kept.traces.set(harness.training.keyOf(TRAINING_USER, stored?.sha256 ?? ''), 2)
        harness.training.kept.replays.set(harness.training.keyOf(TRAINING_USER, stored?.sha256 ?? ''), 1)
        for (const scoreId of ['s1', 's2', 's3']) {
            harness.training.scores.push({
                userId: TRAINING_USER,
                sha256: stored?.sha256 ?? '',
                scoreId,
                playedAt: TRAINING_NOW + 1,
                clear: 4,
                exScore: 100,
                maxExScore: 200,
                minbp: 3,
                random: 'Off',
            })
        }
        const expected = { removes: { training_chart: 1, folder_items: 2 }, keeps: { chart_pattern: 1, scores: 3, traces: 2, replays: 1 } }
        const removal = await harness.service.removal({ userId: TRAINING_USER, id: made.id })
        expect(trainingRemovalSchema.parse(removal)).toEqual(expected)
        expect(await harness.service.remove({ userId: TRAINING_USER, id: made.id })).toEqual({ ...expected, removed: true })
        expect(harness.training.charts.has(made.id)).toBe(false)
        expect(await harness.folderService.listContaining({ userId: TRAINING_USER, md5: stored?.md5 ?? '' })).toEqual([])
        expect(await harness.service.download(made.id)).toBeNull()
    })

    test('상세는 판의 구간 결과와 생성 뒤 판으로 잰 전후 비교를 보인다', async () => {
        const harness = await createReady({
            measured: { 'tag:trill': { value: -0.02, unit: 'ex_residual', notes: 400, charts: 6, plays: 8, evidence: 'strong' } },
            analysed: async ({ scoreIds }) =>
                new Map(
                    scoreIds
                        .filter((scoreId) => scoreId === 's1')
                        .map((scoreId) => [
                            scoreId,
                            {
                                summary: {
                                    metrics: {
                                        training_sections: [
                                            { index: 1, chart_notes: 50, ex_score: 80, bp: 2, fast: 5, slow: 7, moments: { n: 4, sum_us: 6000 } },
                                        ],
                                    },
                                },
                            },
                        ]),
                ),
        })
        const made = expectOk(await harness.create(PUSH))
        const sha256 = harness.training.charts.get(made.id)?.sha256 ?? ''
        const play = { userId: TRAINING_USER, sha256, clear: 4, exScore: 150, maxExScore: 200, minbp: 5, random: 'RANDOM' }
        harness.training.scores.push({ ...play, scoreId: 's1', playedAt: TRAINING_NOW + 2 }, { ...play, scoreId: 's2', playedAt: TRAINING_NOW + 1 })
        const detail = await harness.service.detail({ userId: TRAINING_USER, id: made.id, origin: TRAINING_ORIGIN })
        expect(trainingChartDetailSchema.safeParse(detail).success).toBe(true)
        expect(detail?.play_count).toBe(2)
        expect(detail?.best_ex_ratio).toBe(0.75)
        expect(detail?.plays.map((item) => [item.score_id, item.ex_ratio, item.random])).toEqual([
            ['s1', 0.75, 'RANDOM'],
            ['s2', 0.75, 'RANDOM'],
        ])
        expect(detail?.plays[0]?.sections).toEqual([{ index: 1, ex_ratio: 0.8, bp: 2, mean_err_ms: 1.5, fast: 5, slow: 7 }])
        expect(detail?.plays[1]?.sections).toBeNull()
        expect(detail?.before_after[0]).toMatchObject({ focus_key: 'tag:trill', after: { value: -0.02, plays: 8 }, delta: 0.03 })
        expect(detail?.before_after[1]).toMatchObject({ focus_key: 'tag:jack', after: null, delta: null })
    })

    test('zip 은 id 만으로 받고 파일 이름은 id 앞 8자다', async () => {
        const harness = await createReady()
        const made = expectOk(await harness.create(PUSH))
        const zip = await harness.service.download(made.id)
        expect(zip?.fileName).toBe(`rbms-training-${made.id.slice(3, 11)}.zip`)
        expect(
            Buffer.from(zip?.bytes ?? [])
                .subarray(0, 4)
                .readUInt32LE(0),
        ).toBe(0x04034b50)
        expect(await harness.service.download(`tc_${'f'.repeat(32)}`)).toBeNull()
    })
})

describe('훈련곡 판의 구간 결과 (trace 분석)', () => {
    test('trace 의 곡 sha256 으로 훈련곡 구간을 찾아 요약의 training_sections 를 채운다', async () => {
        const trace = readContractTrace('trace-normal.json')
        const memory = createMemoryTraceDb([scoreRowFor(trace, countTraceJudgements(trace))])
        const asked: string[] = []
        const { service } = createMemoryTraceService(memory, {
            trainingSectionsOf: async (sha256) => {
                asked.push(sha256)
                return [{ index: 0, kind: 'stream', focus_key: 'tag:stream', from_us: 0, to_us: 3_600_000_000 }]
            },
        })
        await service.upload({ input: trace, pathHash: trace.chart.md5, userId: TRACE_USER_ID })
        expect(asked).toEqual([trace.chart.sha256])
        const [analysis] = [...memory.analyses.values()]
        const sections = (analysis?.summary.metrics as { training_sections: { index: number; notes: number }[] } | undefined)?.training_sections
        expect(sections?.map((section) => section.index)).toEqual([0])
        expect(sections?.[0]?.notes).toBeGreaterThan(0)
    })

    test('훈련곡이 아니면 training_sections 가 null 이다', async () => {
        const trace = readContractTrace('trace-normal.json')
        const memory = createMemoryTraceDb([scoreRowFor(trace, countTraceJudgements(trace))])
        const { service } = createMemoryTraceService(memory, { trainingSectionsOf: async () => null })
        await service.upload({ input: trace, pathHash: trace.chart.md5, userId: TRACE_USER_ID })
        const [analysis] = [...memory.analyses.values()]
        expect((analysis?.summary.metrics as { training_sections: unknown } | undefined)?.training_sections).toBeNull()
    })
})

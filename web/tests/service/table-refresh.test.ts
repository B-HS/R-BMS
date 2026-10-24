import { describe, expect, test } from 'bun:test'
import {
    createTableRefreshService,
    type TableCourseSync,
    type TableCourseSyncInput,
    type TableDanAward,
    type TableSnapshotStoreDb,
} from '@server/service/domain/table-refresh/table-refresh.service'
import { TABLE_REFRESH_REASON } from '@server/service/domain/table-refresh/table-refresh.validate'
import {
    TABLE_DATA_MAX_BYTES,
    TABLE_FETCH_TIMEOUT_MS,
    TABLE_FETCH_USER_AGENT,
    TABLE_HEADER_MAX_BYTES,
    type TableSourceDefinition,
} from '@server/service/domain/table-refresh/table-sources'
import type { FetchBytes } from '@server/service/shared/http/fetch-limited'
import {
    DAY_MS,
    FIRST_NOW,
    SATELLITE,
    SATELLITE_DATA_URL,
    STELLA,
    STELLA_DATA_URL,
    createMemoryStore,
    createStubFetch,
    failure,
    ok,
    okBytes,
    planOf,
    remoteEntries,
    rowOf,
    satelliteHeader,
    stellaHeader,
} from '../support/remote-table-fixture'

const SMALL_TABLE_SIZE = 600
const BIG_TABLE_SIZE = 1000
const SHRUNK_TABLE_SIZE = 700
const SECOND_RUN_NOW = FIRST_NOW + DAY_MS
const DAN_HASH = 'a'.repeat(64)

const logged = () => {
    const lines = { info: [] as unknown[][], error: [] as unknown[][] }
    return {
        lines,
        logger: {
            info: (...args: unknown[]) => void lines.info.push(args),
            error: (...args: unknown[]) => void lines.error.push(args),
        },
    }
}

const createCourses = (plan = planOf(STELLA.id)) => {
    const calls = { plan: [] as TableCourseSyncInput[], apply: [] as TableCourseSyncInput[] }
    const courses: TableCourseSync = {
        planSync: async (input) => {
            calls.plan.push(input)
            return plan
        },
        applySync: async (input) => {
            calls.apply.push(input)
            return plan
        },
    }
    return { courses, calls }
}

const createDan = (awarded = 3, awardedAssisted = 0) => {
    const calls: Parameters<TableDanAward['awardForCourses']>[0][] = []
    const dan: TableDanAward = {
        awardForCourses: async (params) => {
            calls.push(params)
            return { scanned: awarded + awardedAssisted, awarded, awardedAssisted }
        },
    }
    return { dan, calls }
}

type HarnessOptions = {
    tables?: readonly TableSourceDefinition[]
    initial?: ReturnType<typeof rowOf>[]
    answers?: Parameters<typeof createStubFetch>[0]
    courses?: TableCourseSync | null
    dan?: TableDanAward | null
    wrapStore?: (store: TableSnapshotStoreDb) => TableSnapshotStoreDb
    fetchBytes?: FetchBytes
}

const stellaAnswers = (entries = remoteEntries(SMALL_TABLE_SIZE), header: unknown = stellaHeader()) => ({
    [STELLA.headerUrl]: ok(header),
    [STELLA_DATA_URL]: ok(entries),
})

const satelliteAnswers = (entries = remoteEntries(SMALL_TABLE_SIZE)) => ({
    [SATELLITE.headerUrl]: ok(satelliteHeader()),
    [SATELLITE_DATA_URL]: ok(entries),
})

const createHarness = (options: HarnessOptions = {}) => {
    const memory = createMemoryStore(options.initial)
    const stub = createStubFetch(options.answers ?? stellaAnswers())
    const log = logged()
    let now = SECOND_RUN_NOW
    const service = createTableRefreshService({
        tables: options.tables ?? [STELLA],
        fetchBytes: options.fetchBytes ?? stub.fetchBytes,
        store: options.wrapStore ? options.wrapStore(memory.store) : memory.store,
        courses: options.courses === undefined ? null : options.courses,
        dan: options.dan === undefined ? null : options.dan,
        now: () => now,
        logger: log.logger,
    })
    const refresh = (overrides: Partial<Parameters<typeof service.refreshAll>[0]> = {}) =>
        service.refreshAll({ allowLargeChange: false, dryRun: false, ...overrides })
    return { ...memory, ...stub, ...log, refresh, tick: (to: number) => (now = to) }
}

const storedStella = (entries = remoteEntries(SMALL_TABLE_SIZE), stamps?: Parameters<typeof rowOf>[3]) =>
    rowOf(STELLA, stellaHeader(), entries, stamps)

describe('표 매일 갱신 — 저장', () => {
    test('처음 받은 표를 저장하고 코스를 등록한다', async () => {
        const { courses, calls } = createCourses(planOf(STELLA.id, { inserted: 26, danCourses: 13 }))
        const harness = createHarness({ courses })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ tableId: 'stella-st', outcome: 'saved', reason: null, entries: SMALL_TABLE_SIZE, added: SMALL_TABLE_SIZE })
        const row = harness.rows.get('stella-st')
        expect(row).toMatchObject({
            headerUrl: STELLA.headerUrl,
            dataUrl: STELLA_DATA_URL,
            entryCount: SMALL_TABLE_SIZE,
            fetchedAt: SECOND_RUN_NOW,
            lastOutcome: 'saved',
        })
        expect(calls.plan).toHaveLength(1)
        expect(calls.apply).toHaveLength(1)
        expect(calls.apply[0]).toMatchObject({ headerCourse: stellaHeader().course, dataUrl: STELLA_DATA_URL, fetchedAt: SECOND_RUN_NOW })
        expect(calls.apply[0].entries).toHaveLength(SMALL_TABLE_SIZE)
    })

    test('저장하는 바이트는 받은 바이트 그대로이다', async () => {
        const harness = createHarness()
        await harness.refresh()
        const row = harness.rows.get('stella-st')
        expect(new TextDecoder().decode(row?.headerBytes)).toBe(JSON.stringify(stellaHeader()))
        expect(row?.levelCount).toBe(13)
    })

    test('내용이 같으면 확인 시각만 바꾸고 코스 계획에는 처음 받은 시각을 쓴다', async () => {
        const { courses, calls } = createCourses()
        const harness = createHarness({ initial: [storedStella()], courses })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'unchanged', reason: null, added: 0, removed: 0, releveled: 0 })
        expect(harness.calls.saveSnapshot).toBe(0)
        expect(harness.rows.get('stella-st')).toMatchObject({ fetchedAt: FIRST_NOW, checkedAt: SECOND_RUN_NOW, lastOutcome: 'unchanged' })
        expect(calls.plan[0].fetchedAt).toBe(FIRST_NOW)
        expect(calls.apply).toHaveLength(0)
    })

    test('머리만 같고 data 가 바뀌면 저장하고, 코스 계획에 할 일이 없으면 코스는 다시 등록하지 않는다', async () => {
        const { courses, calls } = createCourses()
        const grown = remoteEntries(SMALL_TABLE_SIZE + 5)
        const harness = createHarness({ initial: [storedStella()], courses, answers: stellaAnswers(grown) })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'saved', entries: SMALL_TABLE_SIZE + 5, added: 5, removed: 0 })
        expect(harness.rows.get('stella-st')).toMatchObject({ entryCount: SMALL_TABLE_SIZE + 5, fetchedAt: SECOND_RUN_NOW })
        expect(calls.plan).toHaveLength(1)
        expect(calls.apply).toHaveLength(0)
    })

    test('머리 해시만 달라도 새 내용으로 저장한다', async () => {
        const harness = createHarness({
            initial: [storedStella()],
            answers: stellaAnswers(remoteEntries(SMALL_TABLE_SIZE), stellaHeader({ last_update: '2026-10-23' })),
        })
        const [report] = await harness.refresh()
        expect(report.outcome).toBe('saved')
        expect(new TextDecoder().decode(harness.rows.get('stella-st')?.headerBytes)).toContain('2026-10-23')
    })

    test('코스 계획에 할 일이 있으면 등록하고, 새 단위 코스가 생기면 소급 판정을 부른다', async () => {
        const { courses, calls } = createCourses(planOf(STELLA.id, { changed: 2, newDanHashes: [DAN_HASH] }))
        const { dan, calls: danCalls } = createDan(4)
        const harness = createHarness({ initial: [storedStella()], courses, dan })
        const [report] = await harness.refresh()
        expect(report.outcome).toBe('unchanged')
        expect(calls.apply).toHaveLength(1)
        expect(danCalls).toEqual([{ courseHashes: [DAN_HASH], dryRun: false }])
        expect(report.awarded).toBe(4)
        expect(report.courses?.changed).toBe(2)
    })

    test('소급이 준 A-단위도 awarded 에 더해 캐시 무효화 기준이 된다', async () => {
        const { courses } = createCourses(planOf(STELLA.id, { changed: 1, newDanHashes: [DAN_HASH] }))
        const assistedOnly = await createHarness({ initial: [storedStella()], courses, dan: createDan(0, 2).dan }).refresh()
        expect(assistedOnly[0].awarded).toBe(2)
        const both = await createHarness({ initial: [storedStella()], courses, dan: createDan(1, 2).dan }).refresh()
        expect(both[0].awarded).toBe(3)
    })

    test('새 단위 코스가 없으면 소급 판정을 부르지 않는다', async () => {
        const { courses } = createCourses(planOf(STELLA.id, { inserted: 1 }))
        const { dan, calls: danCalls } = createDan()
        const [report] = await createHarness({ courses, dan }).refresh()
        expect(danCalls).toHaveLength(0)
        expect(report.awarded).toBeNull()
    })

    test('코스 연결이 없으면 표만 저장하고 코스 결과는 비어 있다', async () => {
        const [report] = await createHarness().refresh()
        expect(report).toMatchObject({ outcome: 'saved', courses: null, awarded: null })
    })

    test('dryRun 은 쓰지 않고 코스는 계획만 본다', async () => {
        const { courses, calls } = createCourses(planOf(STELLA.id, { inserted: 26, newDanHashes: [DAN_HASH] }))
        const { dan, calls: danCalls } = createDan()
        const harness = createHarness({ initial: [storedStella(remoteEntries(SMALL_TABLE_SIZE - 10))], courses, dan })
        const [report] = await harness.refresh({ dryRun: true })
        expect(report).toMatchObject({ outcome: 'saved', added: 10 })
        expect(harness.calls.saveSnapshot).toBe(0)
        expect(harness.calls.markChecked).toBe(0)
        expect(calls.apply).toHaveLength(0)
        expect(danCalls).toHaveLength(0)
        expect(report.courses?.inserted).toBe(26)
    })

    test('dryRun 은 거절과 실패도 표시에 쓰지 않는다', async () => {
        const harness = createHarness({ initial: [storedStella()], answers: { [STELLA.headerUrl]: failure('network') } })
        const [report] = await harness.refresh({ dryRun: true })
        expect(report.outcome).toBe('failed')
        expect(harness.calls.markChecked).toBe(0)
        expect(harness.rows.get('stella-st')?.lastOutcome).toBe('saved')
    })

    test('다른 실행이 먼저 쓰면 덮지 않고 코스도 만지지 않는다', async () => {
        const { courses, calls } = createCourses(planOf(STELLA.id, { inserted: 1 }))
        const harness = createHarness({ courses, wrapStore: (store) => ({ ...store, saveSnapshot: async () => false }) })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'unchanged', reason: null })
        expect(calls.plan).toHaveLength(0)
        expect(harness.rows.size).toBe(0)
    })

    test('저장은 이전 data 해시를 조건으로 건다', async () => {
        const saved: (string | null)[] = []
        const previous = storedStella(remoteEntries(SMALL_TABLE_SIZE - 1))
        const harness = createHarness({
            initial: [previous],
            wrapStore: (store) => ({
                ...store,
                saveSnapshot: async (row, previousDataSha256) => {
                    saved.push(previousDataSha256)
                    return store.saveSnapshot(row, previousDataSha256)
                },
            }),
        })
        await harness.refresh()
        expect(saved).toEqual([previous.dataSha256])
        const first = createHarness({
            wrapStore: (store) => ({
                ...store,
                saveSnapshot: async (row, previousDataSha256) => {
                    saved.push(previousDataSha256)
                    return store.saveSnapshot(row, previousDataSha256)
                },
            }),
        })
        await first.refresh()
        expect(saved.at(-1)).toBeNull()
    })
})

describe('표 매일 갱신 — 거절과 실패', () => {
    test('받기 실패는 이전 값을 남기고 failed 로 표시한다', async () => {
        const previous = storedStella()
        const harness = createHarness({ initial: [previous], answers: { [STELLA.headerUrl]: failure('network') } })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'failed', reason: TABLE_REFRESH_REASON.FETCH_FAILED, entries: null })
        const row = harness.rows.get('stella-st')
        expect(row?.dataSha256).toBe(previous.dataSha256)
        expect(row).toMatchObject({ checkedAt: SECOND_RUN_NOW, lastOutcome: 'failed:fetch_failed' })
    })

    test('리다이렉트 응답은 따르지 않고 실패로 본다', async () => {
        const harness = createHarness({ answers: { [STELLA.headerUrl]: failure('redirect') } })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'failed', reason: TABLE_REFRESH_REASON.FETCH_FAILED })
        expect(harness.asked).toEqual([STELLA.headerUrl])
    })

    test('크기 상한을 넘는 응답은 too_large 이고, 요청은 상한·제한 시간·User-Agent 를 단다', async () => {
        const limits: Parameters<FetchBytes>[1][] = []
        const fetchBytes: FetchBytes = async (url, requested) => {
            limits.push(requested)
            return url === STELLA.headerUrl ? ok(stellaHeader()) : failure('too_large')
        }
        const [report] = await createHarness({ fetchBytes }).refresh()
        expect(report).toMatchObject({ outcome: 'failed', reason: TABLE_REFRESH_REASON.TOO_LARGE })
        expect(limits).toEqual([
            { maxBytes: TABLE_HEADER_MAX_BYTES, timeoutMs: TABLE_FETCH_TIMEOUT_MS, userAgent: TABLE_FETCH_USER_AGENT },
            { maxBytes: TABLE_DATA_MAX_BYTES, timeoutMs: TABLE_FETCH_TIMEOUT_MS, userAgent: TABLE_FETCH_USER_AGENT },
        ])
    })

    test('200 이 아닌 응답은 http_status', async () => {
        const [report] = await createHarness({ answers: { [STELLA.headerUrl]: okBytes(new Uint8Array(), 503) } }).refresh()
        expect(report).toMatchObject({ outcome: 'failed', reason: TABLE_REFRESH_REASON.HTTP_STATUS })
    })

    test('JSON 이 아닌 본문은 not_json 으로 거절한다', async () => {
        const html = okBytes(new TextEncoder().encode('<html>blocked</html>'))
        const [report] = await createHarness({ answers: { [STELLA.headerUrl]: html } }).refresh()
        expect(report).toMatchObject({ outcome: 'rejected', reason: TABLE_REFRESH_REASON.NOT_JSON })
    })

    test('이름이 다른 표는 거절하고 저장된 값에는 거절 사유만 남는다', async () => {
        const harness = createHarness({
            initial: [storedStella()],
            answers: stellaAnswers(remoteEntries(SMALL_TABLE_SIZE), stellaHeader({ name: 'Other' })),
        })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'rejected', reason: TABLE_REFRESH_REASON.NAME_MISMATCH })
        expect(harness.calls.saveSnapshot).toBe(0)
        expect(harness.rows.get('stella-st')?.lastOutcome).toBe('rejected:name_mismatch')
    })

    test('data_url 이 다른 호스트면 그 주소를 요청하지 않는다', async () => {
        const harness = createHarness({ answers: { [STELLA.headerUrl]: ok(stellaHeader({ data_url: 'https://evil.example/score.json' })) } })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'rejected', reason: TABLE_REFRESH_REASON.DATA_URL_OFF_SITE })
        expect(harness.asked).toEqual([STELLA.headerUrl])
    })

    test('10% 넘게 줄어든 표는 거절하고 이전 값을 남긴다', async () => {
        const previous = storedStella(remoteEntries(BIG_TABLE_SIZE))
        const harness = createHarness({ initial: [previous], answers: stellaAnswers(remoteEntries(SHRUNK_TABLE_SIZE)) })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({
            outcome: 'rejected',
            reason: TABLE_REFRESH_REASON.SHRANK,
            entries: SHRUNK_TABLE_SIZE,
            removed: BIG_TABLE_SIZE - SHRUNK_TABLE_SIZE,
        })
        expect(harness.rows.get('stella-st')?.dataSha256).toBe(previous.dataSha256)
        expect(harness.rows.get('stella-st')?.lastOutcome).toBe('rejected:shrank')
    })

    test('레벨만 25% 넘게 옮겨진 표도 거절한다', async () => {
        const previous = storedStella(remoteEntries(BIG_TABLE_SIZE))
        const moved = remoteEntries(BIG_TABLE_SIZE, { levelOf: (n) => (n % 2 === 0 ? 'moved' : String(n % 13)) })
        const [report] = await createHarness({ initial: [previous], answers: stellaAnswers(moved) }).refresh()
        expect(report).toMatchObject({ outcome: 'rejected', reason: TABLE_REFRESH_REASON.CHANGED_TOO_MUCH })
    })

    test('강제 반영은 급감을 받아들이지만 형식 오류는 여전히 거절한다', async () => {
        const previous = storedStella(remoteEntries(BIG_TABLE_SIZE))
        const forced = createHarness({ initial: [previous], answers: stellaAnswers(remoteEntries(SHRUNK_TABLE_SIZE)) })
        expect((await forced.refresh({ allowLargeChange: true }))[0]).toMatchObject({ outcome: 'saved', entries: SHRUNK_TABLE_SIZE })
        const broken = createHarness({
            initial: [previous],
            answers: { [STELLA.headerUrl]: ok(stellaHeader()), [STELLA_DATA_URL]: ok([{ md5: 'bad' }]) },
        })
        expect((await broken.refresh({ allowLargeChange: true }))[0]).toMatchObject({
            outcome: 'rejected',
            reason: TABLE_REFRESH_REASON.ENTRIES_INVALID,
        })
    })

    test('첫 저장이 500곡 미만이면 거절하고 행을 만들지 않는다', async () => {
        const harness = createHarness({ answers: stellaAnswers(remoteEntries(10)) })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'rejected', reason: TABLE_REFRESH_REASON.TOO_FEW_ENTRIES })
        expect(harness.rows.size).toBe(0)
        expect(harness.calls.markChecked).toBe(0)
    })

    test('저장된 내용을 읽을 수 없으면 첫 저장 규칙으로 판단하고 해시 조건으로 갈아 쓴다', async () => {
        const corrupt = { ...storedStella(), dataBytes: new TextEncoder().encode('not json') }
        const harness = createHarness({ initial: [corrupt], answers: stellaAnswers(remoteEntries(SMALL_TABLE_SIZE + 1)) })
        const [report] = await harness.refresh()
        expect(report.outcome).toBe('saved')
        expect(harness.rows.get('stella-st')?.entryCount).toBe(SMALL_TABLE_SIZE + 1)
    })
})

describe('표 매일 갱신 — 여러 표와 오류', () => {
    test('표는 차례로 받고 한 표가 실패해도 다음 표는 갱신된다', async () => {
        const harness = createHarness({
            tables: [STELLA, SATELLITE],
            answers: { [STELLA.headerUrl]: failure('timeout'), ...satelliteAnswers() },
        })
        const reports = await harness.refresh()
        expect(reports.map((report) => [report.tableId, report.outcome])).toEqual([
            ['stella-st', 'failed'],
            ['stella-sl', 'saved'],
        ])
        expect(harness.asked).toEqual([STELLA.headerUrl, SATELLITE.headerUrl, SATELLITE_DATA_URL])
        expect([...harness.rows.keys()]).toEqual(['stella-sl'])
    })

    test('코스 동기화가 던져도 표는 저장된 채 남고 사유만 표시한다', async () => {
        const courses: TableCourseSync = {
            planSync: async () => Promise.reject(new Error('secret query text')),
            applySync: async () => planOf(STELLA.id),
        }
        const harness = createHarness({ courses })
        const [report] = await harness.refresh()
        expect(report).toMatchObject({ outcome: 'saved', reason: TABLE_REFRESH_REASON.COURSE_SYNC_FAILED })
        expect(harness.rows.has('stella-st')).toBe(true)
        expect(JSON.stringify(harness.lines.error)).not.toContain('secret query text')
    })

    test('예상 못한 오류는 그 표만 failed 로 두고 다음 표를 계속한다', async () => {
        const harness = createHarness({
            tables: [STELLA, SATELLITE],
            answers: { ...stellaAnswers(), ...satelliteAnswers() },
            wrapStore: (store) => ({
                ...store,
                saveSnapshot: async (row, previous) =>
                    row.tableId === 'stella-st' ? Promise.reject(new Error('boom with query')) : store.saveSnapshot(row, previous),
            }),
        })
        const reports = await harness.refresh()
        expect(reports.map((report) => [report.outcome, report.reason])).toEqual([
            ['failed', TABLE_REFRESH_REASON.UNEXPECTED_ERROR],
            ['saved', null],
        ])
        expect(JSON.stringify(harness.lines.error)).not.toContain('boom with query')
    })

    test('로그에는 표 id, 고정 코드, 개수만 남고 제목·주소는 남지 않는다', async () => {
        const harness = createHarness({
            initial: [storedStella(remoteEntries(BIG_TABLE_SIZE))],
            answers: stellaAnswers(remoteEntries(SHRUNK_TABLE_SIZE)),
        })
        await harness.refresh()
        const text = JSON.stringify([harness.lines.info, harness.lines.error])
        expect(text).toContain('stella-st')
        expect(text).toContain('shrank')
        expect(text).not.toMatch(/title-|https:|example/)
    })

    test('성공은 info, 거절·실패는 error 로 남긴다', async () => {
        const saved = createHarness()
        await saved.refresh()
        expect([saved.lines.info.length, saved.lines.error.length]).toEqual([1, 0])
        const failed = createHarness({ answers: { [STELLA.headerUrl]: failure('network') } })
        await failed.refresh()
        expect([failed.lines.info.length, failed.lines.error.length]).toEqual([0, 1])
    })
})

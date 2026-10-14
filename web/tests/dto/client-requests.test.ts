import { describe, expect, test } from 'bun:test'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { isMd5Hash, isSha256Hash } from '@server/dto/common'
import { courseSubmissionSchema } from '@server/dto/course'
import { replayUploadSchema } from '@server/dto/replay'
import { API_VERSION, scoreSubmissionSchema } from '@server/dto/score'
import { SETTING_MAX_BYTES, settingPutSchema } from '@server/dto/setting'
import { parseEnv } from '@server/lib/env'
import { createCourseService } from '@server/service/domain/course/course.service'
import { createReplayService, type ReplayInsertRow } from '@server/service/domain/replay/replay.service'
import { BUILD_TRUST } from '@server/service/domain/score/ranked-policy'
import {
    createScoreService,
    GUEST_PLAYER_ID,
    isJudgeCountConsistent,
    isPlayedAtPlausible,
    type ScoreInsertRow,
} from '@server/service/domain/score/score.service'
import { createSettingService } from '@server/service/domain/setting/setting.service'
import { createMysqlReplayStorage } from '@server/service/shared/storage/replay-storage'

const CONTRACT_DIR = join(import.meta.dirname, '..', '..', '..', 'contract', 'client-requests')
const CONTRACT_KINDS = ['score', 'course', 'replay', 'settings'] as const
const REQUEST_SCHEMA = {
    score: scoreSubmissionSchema,
    course: courseSubmissionSchema,
    replay: replayUploadSchema,
    settings: settingPutSchema,
} as const
const FLOAT_FIELDS: Record<ContractKind, ReadonlySet<string>> = {
    score: new Set(['gauge_value', 'options.hispeed', 'options.lift', 'options.lane_cover', 'options.total_override', 'options.green_number']),
    course: new Set(['gauge_value']),
    replay: new Set(),
    settings: new Set(),
}
const NULLISH_FIELDS: Record<ContractKind, ReadonlySet<string>> = {
    score: new Set(['replay_id', 'client_build_sha256', 'client_platform', 'options.random_p2']),
    course: new Set(['trophy']),
    replay: new Set(['chart', 'score_id', 'random', 'random_p2', 'seed', 'gauge', 'client_build_sha256', 'event_count', 'duration_us', 'size']),
    settings: new Set(['name', 'updated_at', 'base_updated_at']),
}
const DROPPED_FIELDS: Record<ContractKind, ReadonlySet<string>> = {
    score: new Set(),
    course: new Set(),
    replay: new Set(['id']),
    settings: new Set(),
}
const BUILDLESS_SCORE_FILES = ['score-sync-resend.json']
const REPLAY_EVENTS_THAT_MUST_FIT = 100_000
const CONTRACT_USER_ID = 'usr_contract'
const SERVER_NOW = 1_791_970_074_354

type ContractKind = (typeof CONTRACT_KINDS)[number]
type Leaf = { path: string; value: unknown }

const isContractKind = (value: string): value is ContractKind => CONTRACT_KINDS.some((kind) => kind === value)

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)

const kindOf = (file: string) => {
    const prefix = file.split('-')[0]
    return isContractKind(prefix) ? prefix : null
}

const readContract = (file: string): unknown => JSON.parse(readFileSync(join(CONTRACT_DIR, file), 'utf8'))

const childPath = (path: string, key: string) => (path === '' ? key : `${path}.${key}`)

const leavesOf = (value: unknown, path = ''): Leaf[] => {
    if (Array.isArray(value)) return value.flatMap((item) => leavesOf(item, `${path}[]`))
    if (isRecord(value)) return Object.entries(value).flatMap(([key, item]) => leavesOf(item, childPath(path, key)))
    return [{ path, value }]
}

const changedPaths = (input: unknown, parsed: unknown, dropped: ReadonlySet<string>, path = ''): string[] => {
    if (Array.isArray(input)) {
        if (!Array.isArray(parsed) || parsed.length !== input.length) return [path]
        return input.flatMap((item, index) => changedPaths(item, parsed[index], dropped, `${path}[]`))
    }
    if (isRecord(input)) {
        if (!isRecord(parsed)) return [path]
        return Object.entries(input).flatMap(([key, item]) => {
            const at = childPath(path, key)
            if (!(key in parsed)) return dropped.has(at) ? [] : [`${at} (서버가 버림)`]
            return changedPaths(item, parsed[key], dropped, at)
        })
    }
    return Object.is(input, parsed) ? [] : [`${path}: ${JSON.stringify(input)} -> ${JSON.stringify(parsed)}`]
}

const sentSeed = (raw: unknown) => (isRecord(raw) && typeof raw.seed === 'number' ? raw.seed : null)

const byteLength = (value: unknown) => new TextEncoder().encode(JSON.stringify(value)).length

const contractFiles = readdirSync(CONTRACT_DIR)
    .filter((file) => file.endsWith('.json'))
    .toSorted()

const filesOf = (kind: ContractKind) => contractFiles.filter((file) => kindOf(file) === kind)

const createScoreStub = () => {
    const inserted: ScoreInsertRow[] = []
    const service = createScoreService({
        db: {
            findScoreIdByIdempotency: async () => null,
            insertScore: async (row) => {
                inserted.push(row)
            },
            getChartBest: async () => null,
            upsertChartBest: async () => undefined,
            countBetterBests: async () => 0,
            getRanking: async () => [],
            getBestByLoginId: async () => null,
            getScoreById: async () => null,
            getRivalUserIds: async () => [],
            insertAudit: async () => undefined,
        },
        newId: (prefix) => `${prefix}contract`,
        now: () => SERVER_NOW,
    })
    return { inserted, service }
}

const createCourseStub = (charts: { md5: string; sha256: string }[]) =>
    createCourseService({
        newId: (prefix) => `${prefix}contract`,
        db: {
            findCourse: async () => null,
            findCourseCharts: async () => charts.map((chart, position) => ({ position, chartSha256: chart.sha256, md5: chart.md5 })),
            upsertCourse: async () => undefined,
            replaceCourseCharts: async () => undefined,
            findCourseScoreIdByIdempotency: async () => null,
            insertCourseScore: async () => undefined,
            getCourseBest: async () => null,
            upsertCourseBest: async () => undefined,
            countBetterCourseBests: async () => 0,
            getCourseRanking: async () => [],
            getCourseBestByLoginId: async () => null,
            getRivalUserIds: async () => [],
        },
    })

const createReplayStub = () => {
    const stored: ReplayInsertRow[] = []
    const service = createReplayService({
        db: {
            insertReplay: async (row: ReplayInsertRow) => {
                stored.push(row)
            },
            findReplayById: async () => null,
            listReplaysByChart: async () => [],
            linkScoreReplay: async () => undefined,
            findLinkedReplay: async () => null,
        },
        storage: createMysqlReplayStorage(),
        newId: (prefix) => `${prefix}contract`,
    })
    return { stored, service }
}

const createSettingStub = () =>
    createSettingService({
        now: () => SERVER_NOW,
        db: {
            findSetting: async () => null,
            upsertSetting: async () => undefined,
            listSettings: async () => [],
        },
    })

describe('클라이언트 요청 계약 파일', () => {
    test('네 종류의 요청이 모두 하나 이상 있다', () => {
        expect(CONTRACT_KINDS.filter((kind) => filesOf(kind).length === 0)).toEqual([])
    })

    test('모든 파일 이름이 알려진 요청 종류로 시작한다', () => {
        expect(contractFiles.filter((file) => kindOf(file) === null)).toEqual([])
    })
})

const contracts = contractFiles.flatMap((file) => {
    const kind = kindOf(file)
    return kind ? [{ file, kind }] : []
})

for (const { file, kind } of contracts)
    describe(file, () => {
        const input = readContract(file)
        const leaves = leavesOf(input)

        test('서버가 쓰는 스키마를 통과한다', () => {
            const result = REQUEST_SCHEMA[kind].safeParse(input)
            expect(result.success ? [] : result.error.issues.map((issue) => `${issue.path.join('.')}: ${issue.message}`)).toEqual([])
        })

        test('정수 칸은 안전한 정수, 실수 칸은 유한한 수다', () => {
            const numbers = leaves.filter((leaf) => typeof leaf.value === 'number')
            const outOfRange = numbers.filter(({ path, value }) =>
                FLOAT_FIELDS[kind].has(path) ? !Number.isFinite(value) : !Number.isSafeInteger(value),
            )
            expect(outOfRange).toEqual([])
        })

        test('null 은 서버가 nullish 로 받거나 읽지 않는 칸에만 있다', () => {
            const accepted = (path: string) => NULLISH_FIELDS[kind].has(path) || DROPPED_FIELDS[kind].has(path)
            expect(leaves.filter((leaf) => leaf.value === null && !accepted(leaf.path)).map((leaf) => leaf.path)).toEqual([])
        })

        test('파싱 결과가 입력과 같다 — 서버가 조용히 바꾸거나 버리는 값이 없다', () => {
            const result = REQUEST_SCHEMA[kind].safeParse(input)
            expect(result.success).toBe(true)
            expect(result.success ? changedPaths(input, result.data, DROPPED_FIELDS[kind]) : []).toEqual([])
        })
    })

describe.each(filesOf('score'))('%s — 점수 제출 라우트와 서비스', (file) => {
    const raw = readContract(file)

    test('라우트의 검사를 모두 통과한다', () => {
        const input = scoreSubmissionSchema.parse(raw)
        expect(input.api_version).toBe(API_VERSION)
        expect(input.player.id).not.toBe('')
        expect(input.player.id).not.toBe(GUEST_PLAYER_ID)
        expect(isPlayedAtPlausible(input.played_at, Date.now())).toBe(true)
        expect(isSha256Hash(input.chart.sha256)).toBe(true)
        expect(isMd5Hash(input.chart.md5)).toBe(true)
        expect(isJudgeCountConsistent(input.judge, input.total_notes, input.mode)).toBe(true)
    })

    test('시드가 0 으로 바뀌지 않고 그대로 저장된다', async () => {
        const input = scoreSubmissionSchema.parse(raw)
        const { inserted, service } = createScoreStub()
        const result = await service.submit({
            input,
            chartSha256: input.chart.sha256,
            chartMd5: input.chart.md5,
            user: { id: CONTRACT_USER_ID, loginId: input.player.id, name: input.player.id },
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.response.accepted).toBe(true)
        expect(inserted).toHaveLength(1)
        expect<number | null>(inserted[0].seed).toBe(sentSeed(raw))
        expect(inserted[0].exScore).toBe(input.ex_score)
    })
})

describe('점수 제출 — 빌드 해시', () => {
    test('빌드 해시가 없는 본문은 기록 DB 행에서 다시 만든 본문뿐이다 (REQUIRE_BUILD_HASH 가 켜진 서버는 이것만 거절한다)', () => {
        const buildless = filesOf('score').filter((file) => !scoreSubmissionSchema.parse(readContract(file)).client_build_sha256)
        expect(buildless).toEqual(BUILDLESS_SCORE_FILES)
    })
})

describe.each(filesOf('course'))('%s — 코스 제출 라우트와 서비스', (file) => {
    const input = courseSubmissionSchema.parse(readContract(file))

    test('라우트의 검사를 통과하고 코스와 곡을 해시로 가리킨다', () => {
        expect(input.api_version).toBe(API_VERSION)
        expect(input.player.id).not.toBe('')
        expect(isPlayedAtPlausible(input.played_at, Date.now())).toBe(true)
        expect(isSha256Hash(input.course_hash)).toBe(true)
        expect(input.charts.filter((chart) => !isMd5Hash(chart.md5) || !isSha256Hash(chart.sha256))).toEqual([])
    })

    test('서비스가 거절 없이 받는다', async () => {
        const result = await createCourseStub(input.charts).submit({ input, user: { id: CONTRACT_USER_ID, loginId: input.player.id } })
        expect(result.response.accepted).toBe(true)
    })
})

describe.each(filesOf('replay'))('%s — 리플레이 업로드 라우트와 서비스', (file) => {
    const raw = readContract(file)
    const input = replayUploadSchema.parse(raw)

    test('점수에 연결되고, 서버가 처음 보는 차트도 본문의 SHA-256 으로 찾는다', () => {
        expect(input.score_id).not.toBeNull()
        expect(isSha256Hash(input.chart?.sha256 ?? '')).toBe(true)
        expect(input.seed).toBe(sentSeed(raw))
    })

    test('이벤트 수와 길이가 이벤트와 맞는다', () => {
        const times = input.events.map((event) => event.t_us)
        expect(input.event_count).toBe(input.events.length)
        expect(input.duration_us).toBe(Math.max(...times) - Math.min(...times))
    })

    test(`본문이 업로드 한도 안이고, 같은 모양이면 이벤트 ${REPLAY_EVENTS_THAT_MUST_FIT}개도 들어간다`, () => {
        const maxBytes = parseEnv({}).REPLAY_MAX_BYTES
        const bytes = byteLength(raw)
        const bytesPerEvent = byteLength(input.events) / input.events.length
        expect(bytes).toBeLessThanOrEqual(maxBytes)
        expect(Math.floor((maxBytes - (bytes - byteLength(input.events))) / bytesPerEvent)).toBeGreaterThanOrEqual(REPLAY_EVENTS_THAT_MUST_FIT)
    })

    test('서비스가 이벤트를 모두 저장한다', async () => {
        const { stored, service } = createReplayStub()
        const uploaded = await service.upload({ input, chartSha256: input.chart?.sha256 ?? '', userId: CONTRACT_USER_ID })
        expect(uploaded.created).toBe(true)
        expect(stored).toHaveLength(1)
        expect(stored[0].eventCount).toBe(input.events.length)
    })
})

describe.each(filesOf('settings'))('%s — 설정 업로드 라우트와 서비스', (file) => {
    const raw = readContract(file)
    const input = settingPutSchema.parse(raw)

    test('본문과 내용이 설정 한도 안이다', () => {
        expect(byteLength(raw)).toBeLessThanOrEqual(SETTING_MAX_BYTES)
        expect(new TextEncoder().encode(input.content).length).toBeLessThanOrEqual(SETTING_MAX_BYTES)
    })

    test('서비스가 충돌 없이 저장한다', async () => {
        const result = await createSettingStub().put({ userId: CONTRACT_USER_ID, name: input.name ?? '', input })
        expect(result.conflict).toBe(false)
    })
})

import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { toJudgeBreakdown } from '@server/compose/judge-row'
import { chartUpsertSchema } from '@server/dto/chart'
import { accountSchema, MAX_DISPLAY_NAME_LENGTH, MAX_EMAIL_LENGTH } from '@server/dto/auth'
import { FLOAT_COLUMN_MAX, INT_COLUMN_MAX, INT_COLUMN_MIN, JSON_COLUMN_DEPTH_MAX, JSON_KEY_MAX_BYTES, judgeBreakdownSchema } from '@server/dto/common'
import { COURSE_NAME_MAX_LENGTH, courseMetaUpsertSchema, courseSubmissionSchema } from '@server/dto/course'
import { replayUploadSchema } from '@server/dto/replay'
import { scoreSubmissionSchema } from '@server/dto/score'
import {
    folderIdFor,
    TABLE_FOLDER_ID_MAX_LENGTH,
    TABLE_FOLDER_NAME_MAX_LENGTH,
    TABLE_NAME_MAX_LENGTH,
    TABLE_URL_MAX_LENGTH,
    tableUpsertSchema,
} from '@server/dto/table'
import { findCourseIntegrityFailure } from '@server/service/domain/course/course.service'
import { buildScoreRow, findSubmissionIntegrityFailure } from '@server/service/domain/score/score.service'

const CONTRACT_DIR = join(import.meta.dirname, '..', '..', '..', 'contract', 'client-requests')
const PAST_INT = INT_COLUMN_MAX + 1
const HALF_PAST_INT = 1_500_000_000
const PAST_FLOAT = 1e300
const PAST_JSON_DEPTH = JSON_COLUMN_DEPTH_MAX + 1

const contract = (file: string): Record<string, unknown> => JSON.parse(readFileSync(join(CONTRACT_DIR, file), 'utf8'))
const scoreContract = contract('score-normal.json')
const courseContract = contract('course-normal.json')
const scoreOptions = scoreContract.options as Record<string, unknown>

const zeroJudge = { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }

const scoreWith = (overrides: Record<string, unknown>, options: Record<string, unknown> = {}) => ({
    ...scoreContract,
    ...overrides,
    options: { ...scoreOptions, ...options },
})

const nestedArray = (depth: number): unknown => (depth <= 1 ? [] : [nestedArray(depth - 1)])

const parsedScore = (overrides: Record<string, unknown>) => scoreSubmissionSchema.parse({ ...scoreContract, ...overrides })

describe('점수 본문의 수는 DB 열이 담는 범위 안이어야 한다', () => {
    test('INT 열 하나라도 넘으면 스키마가 거절한다', () => {
        const bodies = [
            scoreWith({ total_notes: PAST_INT }),
            scoreWith({ ex_score: PAST_INT }),
            scoreWith({ max_ex_score: PAST_INT }),
            scoreWith({ max_combo: PAST_INT }),
            scoreWith({ passnotes: PAST_INT }),
            scoreWith({ minbp: PAST_INT }),
            scoreWith({ judge: { ...zeroJudge, combobreak: PAST_INT } }),
            scoreWith({ judge: { ...zeroJudge, epg: PAST_INT } }),
            scoreWith({ judge: { ...zeroJudge, empty_poor: PAST_INT } }),
            scoreWith({}, { lntype: PAST_INT }),
            scoreWith({}, { judge_rate: PAST_INT }),
            scoreWith({}, { offset_ms: PAST_INT }),
            scoreWith({}, { offset_ms: INT_COLUMN_MIN - 1 }),
        ]
        for (const body of bodies) expect(scoreSubmissionSchema.safeParse(body).success).toBe(false)
    })

    test('FLOAT 열을 넘는 게이지, 리프트, 레인 커버를 거절한다', () => {
        for (const body of [
            scoreWith({ gauge_value: PAST_FLOAT }),
            scoreWith({}, { lift: PAST_FLOAT }),
            scoreWith({}, { lane_cover: -PAST_FLOAT }),
        ]) {
            expect(scoreSubmissionSchema.safeParse(body).success).toBe(false)
        }
    })

    test('열 끝의 값은 받는다', () => {
        const body = scoreWith(
            { max_combo: INT_COLUMN_MAX, gauge_value: FLOAT_COLUMN_MAX, judge: { ...zeroJudge, fast: INT_COLUMN_MAX } },
            { offset_ms: INT_COLUMN_MIN, judge_rate: INT_COLUMN_MAX },
        )
        expect(scoreSubmissionSchema.safeParse(body).success).toBe(true)
    })

    test('extra 가 DB 의 JSON 깊이를 넘으면 거절하고, 그 깊이까지는 받는다', () => {
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { deep: nestedArray(PAST_JSON_DEPTH - 1) } })).success).toBe(false)
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { deep: nestedArray(JSON_COLUMN_DEPTH_MAX - 1) } })).success).toBe(true)
    })
})

describe('점수 본문의 글자는 열 길이로 자르고, 해시와 id 는 형식을 본다', () => {
    test('모드, 입력 장치, 판정 방식, 규칙, 스킨, 클라이언트, 플랫폼은 열 길이로 잘린다', () => {
        const parsed = scoreSubmissionSchema.parse(
            scoreWith(
                {
                    mode: 'M'.repeat(17),
                    judge_algorithm: 'J'.repeat(17),
                    rule: 'R'.repeat(17),
                    skin: '스'.repeat(65),
                    client: 'C'.repeat(65),
                    client_platform: 'P'.repeat(33),
                },
                { input_device: 'I'.repeat(33) },
            ),
        )
        expect(parsed.mode).toBe('M'.repeat(16))
        expect(parsed.judge_algorithm).toBe('J'.repeat(16))
        expect(parsed.rule).toBe('R'.repeat(16))
        expect(parsed.skin).toBe('스'.repeat(64))
        expect(parsed.client).toBe('C'.repeat(64))
        expect(parsed.client_platform).toBe('P'.repeat(32))
        expect(parsed.options.input_device).toBe('I'.repeat(32))
    })

    test('빌드 해시가 64자를 넘거나 16진수가 아니면, 리플레이 id 가 36자를 넘으면 거절한다', () => {
        for (const body of [
            scoreWith({ client_build_sha256: 'a'.repeat(65) }),
            scoreWith({ client_build_sha256: 'Z'.repeat(64) }),
            scoreWith({ replay_id: 'r'.repeat(37) }),
        ]) {
            expect(scoreSubmissionSchema.safeParse(body).success).toBe(false)
        }
        expect(scoreSubmissionSchema.parse(scoreWith({ client_build_sha256: null })).client_build_sha256).toBeNull()
    })
})

describe('서버가 셈해 저장하는 EX 와 BP 도 INT 열 안이어야 한다', () => {
    test('필드마다 열 안이어도 PGREAT 두 배가 넘치면 판정 필드로 거절한다', () => {
        const body = parsedScore({
            ex_score: 0,
            max_ex_score: 0,
            minbp: 0,
            clear: 'Normal',
            total_notes: HALF_PAST_INT,
            judge: { ...zeroJudge, pgreat: HALF_PAST_INT },
        })
        expect(findSubmissionIntegrityFailure(body)).toMatchObject({ field: 'judge' })
    })

    test("pop'n 의 BAD 와 空POOR 합이 넘쳐도 판정 필드로 거절한다", () => {
        const body = parsedScore({
            mode: 'POPN_9K',
            ex_score: 0,
            max_ex_score: 0,
            minbp: 0,
            clear: 'Normal',
            judge: { ...zeroJudge, bad: INT_COLUMN_MAX, miss: INT_COLUMN_MAX },
        })
        expect(findSubmissionIntegrityFailure(body)).toMatchObject({ field: 'judge' })
    })

    test('코스도 BP 0 이라 서버가 셈할 BP 가 넘치면 거절한다', () => {
        const body = courseSubmissionSchema.parse({
            ...courseContract,
            ex_score: 0,
            minbp: 0,
            judge: { ...zeroJudge, bad: INT_COLUMN_MAX, miss: INT_COLUMN_MAX },
        })
        expect(findCourseIntegrityFailure(body)).toMatchObject({ field: 'judge' })
    })

    test('계약 본문은 그대로 통과한다', () => {
        expect(findSubmissionIntegrityFailure(scoreSubmissionSchema.parse(scoreContract))).toBeNull()
        expect(findCourseIntegrityFailure(courseSubmissionSchema.parse(courseContract))).toBeNull()
    })
})

describe('코스와 차트 본문의 수', () => {
    test('코스 판정, EX, 게이지가 열을 넘으면 거절한다', () => {
        for (const body of [
            { ...courseContract, judge: { ...zeroJudge, pgreat: PAST_INT } },
            { ...courseContract, max_ex_score: PAST_INT },
            { ...courseContract, gauge_value: PAST_FLOAT },
            { ...courseContract, client_build_sha256: 'a'.repeat(65) },
        ]) {
            expect(courseSubmissionSchema.safeParse(body).success).toBe(false)
        }
    })

    test('차트 메타는 INT 열 아래쪽도 막는다', () => {
        expect(chartUpsertSchema.safeParse({ chart: { sha256: 'a'.repeat(64), lntype: INT_COLUMN_MIN - 1 } }).success).toBe(false)
        expect(chartUpsertSchema.safeParse({ chart: { sha256: 'a'.repeat(64), lntype: INT_COLUMN_MIN } }).success).toBe(true)
    })
})

describe('녹색 숫자', () => {
    test('반올림한 값이 INT 열을 넘으면 저장하지 않는다', () => {
        const row = (greenNumber: number) =>
            buildScoreRow({
                id: 'sc_x',
                userId: null,
                guestName: 'guest',
                chartSha256: 'a'.repeat(64),
                chartMd5: null,
                input: scoreSubmissionSchema.parse(scoreWith({}, { green_number: greenNumber })),
                exScore: 0,
                minbp: 0,
                ranked: false,
                scratchAuto: false,
                flags: [],
            }).greenNumber
        expect(row(1e12)).toBeNull()
        expect(row(800.4)).toBe(800)
    })
})

describe('응답의 판정 합계는 검사가 읽은 값이다', () => {
    test('분리 합과 다른 합계는 분리 합으로 돌려준다', () => {
        const stored = { ...judgeBreakdownSchema.parse({ ...zeroJudge, epg: 8, egr: 1, epr: 1, pgreat: 10 }), emptyPoor: 0 }
        expect(toJudgeBreakdown(stored)).toMatchObject({ pgreat: 8, great: 1, poor: 1, epg: 8, egr: 1, epr: 1 })
    })

    test('분리가 없는 합계는 그대로 돌려준다', () => {
        const stored = { ...judgeBreakdownSchema.parse({ ...zeroJudge, pgreat: 56, fast: 56 }), emptyPoor: 0 }
        expect(toJudgeBreakdown(stored)).toMatchObject({ pgreat: 56, great: 0, poor: 0, fast: 56 })
    })
})

const replayContract = contract('replay-long.json')
const replayWith = (overrides: Record<string, unknown>) => ({ ...replayContract, ...overrides })
const LONE_HIGH_SURROGATE = String.fromCharCode(0xd800)
const LONE_LOW_SURROGATE = String.fromCharCode(0xdc00)
const SUPPLEMENTARY_CHARACTER = String.fromCodePoint(0x20000)

describe('리플레이 본문의 수와 글자도 DB 열 안이어야 한다', () => {
    test('INT 열을 넘는 롱노트 방식, 오프셋, 판정 폭, 개수, 크기와 정확히 담지 못하는 시간을 거절한다', () => {
        for (const body of [
            replayWith({ lntype: PAST_INT }),
            replayWith({ offset_ms: INT_COLUMN_MIN - 1 }),
            replayWith({ judge_rate: PAST_INT }),
            replayWith({ event_count: PAST_INT }),
            replayWith({ size: PAST_INT }),
            replayWith({ duration_us: Number.MAX_SAFE_INTEGER + 2 }),
            replayWith({ events: [{ t_us: PAST_FLOAT, lane: 0, press: true }] }),
            replayWith({ score_id: 's'.repeat(37) }),
        ]) {
            expect(replayUploadSchema.safeParse(body).success).toBe(false)
        }
    })

    test('형식과 모드는 열 길이로 자르고, 계약 본문은 그대로 받는다', () => {
        const parsed = replayUploadSchema.parse(replayWith({ format: 'F'.repeat(40), mode: 'M'.repeat(20) }))
        expect(parsed.format).toBe('F'.repeat(32))
        expect(parsed.mode).toBe('M'.repeat(16))
        expect(replayUploadSchema.safeParse(replayContract).success).toBe(true)
    })
})

describe('JSON 열에 들어가는 글자는 짝 없는 서로게이트가 없어야 한다', () => {
    test('extra 의 값과 키, 코스의 보조 표시, 제약, 트로피 이름에 짝 없는 서로게이트가 있으면 거절한다', () => {
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { note: [`a${LONE_HIGH_SURROGATE}`] } })).success).toBe(false)
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { [LONE_LOW_SURROGATE]: 1 } })).success).toBe(false)
        expect(replayUploadSchema.safeParse(replayWith({ extra: { note: LONE_LOW_SURROGATE } })).success).toBe(false)
        expect(courseSubmissionSchema.safeParse({ ...courseContract, assist: [LONE_HIGH_SURROGATE] }).success).toBe(false)
        const meta = { course_hash: 'c', constraint: [], trophy: [] }
        expect(courseMetaUpsertSchema.safeParse({ ...meta, constraint: [LONE_HIGH_SURROGATE] }).success).toBe(false)
        expect(courseMetaUpsertSchema.safeParse({ ...meta, trophy: [{ name: LONE_LOW_SURROGATE }] }).success).toBe(false)
    })

    test('짝을 이룬 서로게이트는 받는다', () => {
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { [SUPPLEMENTARY_CHARACTER]: SUPPLEMENTARY_CHARACTER } })).success).toBe(true)
        expect(courseMetaUpsertSchema.safeParse({ course_hash: 'c', constraint: [SUPPLEMENTARY_CHARACTER] }).success).toBe(true)
    })
})

const THREE_BYTE_CHARACTER = '가'
const THREE_BYTE_KEY_MAX = JSON_KEY_MAX_BYTES / 3

describe('extra 의 키는 DB 의 JSON 키 길이(UTF-8 바이트) 안이어야 한다', () => {
    test('ASCII 키는 JSON_KEY_MAX_BYTES 바이트까지 받고 한 바이트 넘으면 거절한다', () => {
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { ['k'.repeat(JSON_KEY_MAX_BYTES)]: 1 } })).success).toBe(true)
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { ['k'.repeat(JSON_KEY_MAX_BYTES + 1)]: 1 } })).success).toBe(false)
    })

    test('여러 바이트 글자의 키는 글자 수가 아니라 바이트로 센다', () => {
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { [THREE_BYTE_CHARACTER.repeat(THREE_BYTE_KEY_MAX)]: 1 } })).success).toBe(true)
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { [THREE_BYTE_CHARACTER.repeat(THREE_BYTE_KEY_MAX + 1)]: 1 } })).success).toBe(
            false,
        )
    })

    test('중첩 객체의 키와 코스·리플레이의 extra 도 같이 본다', () => {
        const longKey = 'k'.repeat(JSON_KEY_MAX_BYTES + 1)
        expect(scoreSubmissionSchema.safeParse(scoreWith({ extra: { outer: [{ [longKey]: 1 }] } })).success).toBe(false)
        expect(courseSubmissionSchema.safeParse({ ...courseContract, extra: { [longKey]: 1 } }).success).toBe(false)
        expect(replayUploadSchema.safeParse(replayWith({ extra: { [longKey]: 1 } })).success).toBe(false)
    })
})

describe('관리자 코스 메타는 열을 넘으면 거절한다', () => {
    const meta = { course_hash: 'c' }

    test('이름은 열 길이까지, lntype 은 INT 열 안까지 받는다', () => {
        expect(courseMetaUpsertSchema.safeParse({ ...meta, name: 'N'.repeat(COURSE_NAME_MAX_LENGTH), lntype: INT_COLUMN_MAX }).success).toBe(true)
        expect(courseMetaUpsertSchema.safeParse({ ...meta, lntype: INT_COLUMN_MIN }).success).toBe(true)
    })

    test('이름이 열 길이를 넘거나 lntype 이 INT 열 밖이면 거절한다', () => {
        for (const body of [
            { ...meta, name: 'N'.repeat(COURSE_NAME_MAX_LENGTH + 1) },
            { ...meta, lntype: PAST_INT },
            { ...meta, lntype: INT_COLUMN_MIN - 1 },
        ]) {
            expect(courseMetaUpsertSchema.safeParse(body).success).toBe(false)
        }
    })

    test('난이도표에 든 코스 메타도 같은 상한을 쓴다', () => {
        expect(tableUpsertSchema.safeParse({ id: 't', courses: [{ ...meta, name: 'N'.repeat(COURSE_NAME_MAX_LENGTH + 1) }] }).success).toBe(false)
    })
})

describe('관리자 난이도표는 열을 넘으면 거절한다', () => {
    const FOLDER_ID_SEPARATOR_AND_DIGIT = folderIdFor('', 0).length
    const longestTableIdWithOneFolder = 't'.repeat(TABLE_FOLDER_ID_MAX_LENGTH - FOLDER_ID_SEPARATOR_AND_DIGIT)

    test('이름, 주소, 폴더 이름은 열 길이까지 받는다', () => {
        const body = {
            id: 't',
            name: 'N'.repeat(TABLE_NAME_MAX_LENGTH),
            url: 'u'.repeat(TABLE_URL_MAX_LENGTH),
            folders: [{ name: 'F'.repeat(TABLE_FOLDER_NAME_MAX_LENGTH) }],
        }
        expect(tableUpsertSchema.safeParse(body).success).toBe(true)
    })

    test('이름, 주소, 폴더 이름이 열 길이를 넘으면 거절한다', () => {
        for (const body of [
            { id: 't', name: 'N'.repeat(TABLE_NAME_MAX_LENGTH + 1) },
            { id: 't', url: 'u'.repeat(TABLE_URL_MAX_LENGTH + 1) },
            { id: 't', folders: [{ name: 'F'.repeat(TABLE_FOLDER_NAME_MAX_LENGTH + 1) }] },
        ]) {
            expect(tableUpsertSchema.safeParse(body).success).toBe(false)
        }
    })

    test('표 id 와 폴더 위치로 만드는 폴더 id 가 열 길이를 넘으면 거절한다', () => {
        expect(tableUpsertSchema.safeParse({ id: longestTableIdWithOneFolder, folders: [{}] }).success).toBe(true)
        expect(tableUpsertSchema.safeParse({ id: `${longestTableIdWithOneFolder}t`, folders: [{}] }).success).toBe(false)
        expect(tableUpsertSchema.safeParse({ id: longestTableIdWithOneFolder, folders: Array.from({ length: 11 }, () => ({})) }).success).toBe(false)
        expect(tableUpsertSchema.safeParse({ id: 't'.repeat(TABLE_FOLDER_ID_MAX_LENGTH), folders: [] }).success).toBe(true)
    })
})

describe('계정 가입의 이메일과 표시 이름', () => {
    const account = { id: 'player_1', password: 'password-1234' }
    const emailOfLength = (length: number) => `${'a'.repeat(length - '@example.com'.length)}@example.com`

    test('이메일은 열 길이까지, 표시 이름은 상한까지 받는다', () => {
        expect(
            accountSchema.safeParse({ ...account, email: emailOfLength(MAX_EMAIL_LENGTH), name: 'N'.repeat(MAX_DISPLAY_NAME_LENGTH) }).success,
        ).toBe(true)
    })

    test('넘으면 거절한다', () => {
        expect(accountSchema.safeParse({ ...account, email: emailOfLength(MAX_EMAIL_LENGTH + 1) }).success).toBe(false)
        expect(accountSchema.safeParse({ ...account, email: 'a@example.com', name: 'N'.repeat(MAX_DISPLAY_NAME_LENGTH + 1) }).success).toBe(false)
    })
})

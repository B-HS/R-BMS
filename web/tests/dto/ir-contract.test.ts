import { describe, expect, test } from 'bun:test'
import { chartIdSchema, isMd5Hash, isSha256Hash, judgeBreakdownSchema, playOptionsSchema, settingsBlobSchema } from '@server/dto/common'
import { scoreRecordSchema, scoreSubmissionSchema, submitResponseSchema } from '@server/dto/score'
import { accountSchema, authResponseSchema, loginSchema, playerProfileSchema } from '@server/dto/auth'
import { serverInfoSchema } from '@server/dto/system'

describe('judgeBreakdownSchema (rbms-ir dto.rs 픽스처)', () => {
    test('기본 9개 필드만 있으면 슈퍼셋 필드는 0 으로 채운다', () => {
        const parsed = judgeBreakdownSchema.parse({
            pgreat: 5,
            great: 4,
            good: 3,
            bad: 2,
            poor: 1,
            miss: 0,
            fast: 9,
            slow: 8,
            combobreak: 7,
        })
        expect(parsed.pgreat).toBe(5)
        expect(parsed.fast).toBe(9)
        expect(parsed.epg).toBe(0)
        expect(parsed.lms).toBe(0)
        expect(parsed.avgjudge).toBe(0)
        expect(parsed.empty_poor).toBe(0)
    })

    test('기본 필드가 빠지면 실패한다', () => {
        const result = judgeBreakdownSchema.safeParse({ great: 4, good: 3, bad: 2, poor: 1, miss: 0, fast: 9, slow: 8, combobreak: 7 })
        expect(result.success).toBe(false)
    })

    test('음수 avgjudge 를 그대로 보존한다', () => {
        const parsed = judgeBreakdownSchema.parse({
            pgreat: 0,
            great: 0,
            good: 0,
            bad: 0,
            poor: 0,
            miss: 0,
            fast: 0,
            slow: 0,
            combobreak: 0,
            avgjudge: -123456,
        })
        expect(parsed.avgjudge).toBe(-123456)
    })
})

describe('playOptionsSchema (rbms-ir dto.rs 픽스처)', () => {
    test('필수 7개 필드만 있으면 슈퍼셋 필드는 기본값이다', () => {
        const parsed = playOptionsSchema.parse({
            gauge: 'Hard',
            random: 'Mirror',
            random_p2: null,
            scratch_auto: true,
            lntype: 2,
            input_device: 'midi',
            assist: ['A', 'B'],
        })
        expect(parsed.gauge).toBe('Hard')
        expect(parsed.random_p2).toBeNull()
        expect(parsed.lntype).toBe(2)
        expect(parsed.assist).toEqual(['A', 'B'])
        expect(parsed.option).toBe(0)
        expect(parsed.judge_rate).toBe(0)
        expect(parsed.hispeed).toBe(0)
        expect(parsed.autoplay).toBe(false)
        expect(parsed.green_number).toBe(0)
    })

    test('2P random 과 큰 option 비트마스크를 보존한다', () => {
        const parsed = playOptionsSchema.parse({
            gauge: 'Easy',
            random: 'Random',
            random_p2: 'SRandom',
            scratch_auto: false,
            lntype: 1,
            input_device: 'bm',
            assist: [],
            option: 1234567890123,
            offset_ms: -42,
            hispeed: 4.25,
        })
        expect(parsed.random_p2).toBe('SRandom')
        expect(parsed.option).toBe(1234567890123)
        expect(parsed.offset_ms).toBe(-42)
        expect(parsed.hispeed).toBe(4.25)
    })

    test('gauge 가 없으면 실패한다', () => {
        const result = playOptionsSchema.safeParse({
            random: 'Off',
            random_p2: null,
            scratch_auto: false,
            lntype: 0,
            input_device: 'kb',
            assist: [],
        })
        expect(result.success).toBe(false)
    })

    test('알 수 없는 enum variant 는 거부한다', () => {
        expect(
            playOptionsSchema.safeParse({
                gauge: 'NotAGauge',
                random: 'Off',
                random_p2: null,
                scratch_auto: false,
                lntype: 0,
                input_device: 'kb',
                assist: [],
            }).success,
        ).toBe(false)
    })
})

describe('scoreSubmissionSchema (http.rs submission() 픽스처)', () => {
    const submission = {
        api_version: 1,
        chart: { md5: 'abc123', sha256: 'def456' },
        player: { id: 'p1' },
        mode: 'BEAT_7K',
        clear: 'Hard',
        ex_score: 1488,
        max_ex_score: 1624,
        judge: { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 },
        max_combo: 540,
        total_notes: 812,
        minbp: 7,
        gauge_value: 86.0,
        options: {
            gauge: 'Hard',
            random: 'Random',
            random_p2: null,
            scratch_auto: false,
            lntype: 1,
            input_device: 'keyboard',
            assist: [],
            option: 0,
            judge_rate: 100,
            offset_ms: 0,
            constant: false,
            hispeed: 3.0,
            lift: 0.0,
            lane_cover: 0.0,
            total_override: 0.0,
            autoplay: false,
            auto_offset: false,
            scratch_left: false,
            green_number: 0.0,
        },
        played_at: 1700000000000,
        client: 'rbms/0.1',
        replay_id: null,
        seed: 0,
        judge_algorithm: '',
        rule: '',
        skin: '',
        client_build_sha256: null,
        client_platform: null,
        extra: {},
    }

    test('클라이언트가 실제로 보내는 본문을 그대로 파싱한다', () => {
        const parsed = scoreSubmissionSchema.parse(submission)
        expect(parsed.chart.sha256).toBe('def456')
        expect(parsed.ex_score).toBe(1488)
        expect(parsed.options.lntype).toBe(1)
        expect(parsed.played_at).toBe(1700000000000)
        expect(parsed.passnotes).toBe(0)
    })

    test('clear 는 대소문자를 구분한다', () => {
        expect(scoreSubmissionSchema.safeParse({ ...submission, clear: 'hard' }).success).toBe(false)
    })
})

describe('scoreRecordSchema (랭킹 raw 배열 행)', () => {
    test('기본 랭킹 payload 는 슈퍼셋 필드를 기본값으로 채운다', () => {
        const parsed = scoreRecordSchema.parse({
            player: { id: 'b' },
            player_name: 'B',
            clear: 'Normal',
            ex_score: 1200,
            max_combo: 300,
            minbp: 9,
            rank: 2,
            played_at: 20,
        })
        expect(parsed.lntype).toBe(0)
        expect(parsed.option).toBe(0)
        expect(parsed.total_notes).toBe(0)
        expect(parsed.judge).toBeNull()
        expect(parsed.extra).toEqual({})
    })

    test('HCN 슈퍼셋 행을 보존한다', () => {
        const parsed = scoreRecordSchema.parse({
            player: { id: 'a' },
            player_name: 'A',
            clear: 'Hard',
            ex_score: 1500,
            max_combo: 500,
            minbp: 3,
            rank: 1,
            played_at: 10,
            lntype: 2,
            total_notes: 812,
        })
        expect(parsed.lntype).toBe(2)
        expect(parsed.total_notes).toBe(812)
    })

    test('rank 가 null 이어도 파싱한다', () => {
        const parsed = scoreRecordSchema.parse({
            player: { id: 'p' },
            player_name: 'X',
            clear: 'Easy',
            ex_score: 10,
            max_combo: 5,
            minbp: 2,
            rank: null,
            played_at: 7,
        })
        expect(parsed.rank).toBeNull()
    })

    test('player_name 이 없으면 실패한다', () => {
        expect(
            scoreRecordSchema.safeParse({
                player: { id: 'p' },
                clear: 'Easy',
                ex_score: 10,
                max_combo: 5,
                minbp: 2,
                rank: null,
                played_at: 7,
            }).success,
        ).toBe(false)
    })
})

describe('submitResponseSchema / authResponseSchema / playerProfileSchema', () => {
    test('SubmitResponse 는 전부 null 이어도 파싱한다', () => {
        const parsed = submitResponseSchema.parse({ accepted: false, rank: null, previous_best: null, message: null })
        expect(parsed.accepted).toBe(false)
        expect(parsed.rank).toBeNull()
    })

    test('AuthResponse 는 token/player/name 3개를 요구한다', () => {
        const parsed = authResponseSchema.parse({ token: 'tok', player: { id: 'p1' }, name: 'P1' })
        expect(parsed.player.id).toBe('p1')
        expect(authResponseSchema.safeParse({ token: 'tok', player: { id: 'p1' } }).success).toBe(false)
    })

    test('PlayerProfile 은 extra 없이도 파싱한다', () => {
        const parsed = playerProfileSchema.parse({ id: 'p1', name: 'P1', total_plays: 3, rank_points: 12.5 })
        expect(parsed.total_plays).toBe(3)
        expect(parsed.extra).toEqual({})
    })
})

describe('accountSchema / loginSchema', () => {
    test('register 는 이메일이 필수다 (OQ2 결정)', () => {
        expect(accountSchema.safeParse({ id: 'alice', password: 'password1' }).success).toBe(false)
        expect(accountSchema.parse({ id: 'alice', password: 'password1', email: 'a@e.com' }).email).toBe('a@e.com')
    })

    test('login 은 id/password 만으로 파싱한다', () => {
        const parsed = loginSchema.parse({ id: 'p1', password: 'pw' })
        expect(parsed.id).toBe('p1')
        expect(parsed.email).toBeNull()
        expect(parsed.name).toBeNull()
    })

    test('login 은 password 가 없으면 실패한다', () => {
        expect(loginSchema.safeParse({ id: 'p1' }).success).toBe(false)
    })
})

describe('serverInfoSchema / settingsBlobSchema', () => {
    test('capabilities 가 없으면 전부 false 로 채운다', () => {
        const parsed = serverInfoSchema.parse({ name: 'n', version: 'v', ir_compat: 'c', capabilities: {} })
        expect(parsed.capabilities.ranking).toBe(false)
        expect(parsed.capabilities.tables).toBe(false)
    })

    test('SettingsBlob 의 updated_at 은 기본 0 이다', () => {
        const parsed = settingsBlobSchema.parse({ name: 'tables', content: '[]' })
        expect(parsed.updated_at).toBe(0)
    })

    test('SettingsBlob content 는 원문 그대로 보존한다', () => {
        const raw = '{"a":1,"nested":{"b":[1,2,3]},"unicode":"日本語"}'
        expect(settingsBlobSchema.parse({ name: 'settings', content: raw }).content).toBe(raw)
    })
})

describe('chartIdSchema 해시 검증', () => {
    test('길이·문자셋을 넘는 해시는 거부한다', () => {
        expect(chartIdSchema.safeParse({ md5: 'a'.repeat(33), sha256: 'b'.repeat(64) }).success).toBe(false)
        expect(chartIdSchema.safeParse({ md5: 'a'.repeat(32), sha256: 'c'.repeat(65) }).success).toBe(false)
        expect(chartIdSchema.safeParse({ md5: 'zz', sha256: '' }).success).toBe(false)
    })

    test('빈 문자열과 소문자 16진수는 통과한다', () => {
        expect(chartIdSchema.parse({}).sha256).toBe('')
        expect(chartIdSchema.safeParse({ md5: 'a'.repeat(32), sha256: 'b'.repeat(64) }).success).toBe(true)
    })

    test('isMd5Hash / isSha256Hash 는 정확한 길이의 소문자 16진수만 인정한다', () => {
        expect(isMd5Hash('a'.repeat(32))).toBe(true)
        expect(isMd5Hash('A'.repeat(32))).toBe(false)
        expect(isMd5Hash('a'.repeat(31))).toBe(false)
        expect(isSha256Hash('0'.repeat(64))).toBe(true)
        expect(isSha256Hash('0'.repeat(63))).toBe(false)
    })
})

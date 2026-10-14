import { describe, expect, test } from 'bun:test'
import { playOptionsSchema, type PlayOptionsInput } from '@server/dto/common'
import { BUILD_TRUST, assistFlags, isScratchAutoPlay, rankedPolicy, submissionMessage } from '@server/service/domain/score/ranked-policy'

const cleanOptions = (overrides: Partial<PlayOptionsInput> = {}) =>
    playOptionsSchema.parse({
        gauge: 'Hard',
        random: 'Random',
        random_p2: null,
        scratch_auto: false,
        lntype: 1,
        input_device: 'keyboard',
        assist: [],
        judge_rate: 100,
        ...overrides,
    })

describe('rankedPolicy', () => {
    test('정상 플레이 + 신뢰 빌드면 ranked 이고 flags 가 비어 있다', () => {
        const result = rankedPolicy({ options: cleanOptions(), buildTrust: BUILD_TRUST.TRUSTED, isGuest: false, requireTrustedBuild: false })
        expect(result.ranked).toBe(true)
        expect(result.flags).toEqual([])
    })

    test('autoplay 는 AUTOPLAY 로 unranked 다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ autoplay: true }),
            buildTrust: BUILD_TRUST.TRUSTED,
            isGuest: false,
            requireTrustedBuild: false,
        })
        expect(result.ranked).toBe(false)
        expect(result.flags).toEqual(['AUTOPLAY'])
    })

    test('SCRATCH AUTO 판은 SCRATCH_AUTO 표지를 달고 ranked 로 남는다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ scratch_auto: true, assist: ['AUTO_SCRATCH'] }),
            buildTrust: BUILD_TRUST.TRUSTED,
            isGuest: false,
            requireTrustedBuild: false,
        })
        expect(result.flags).toEqual(['SCRATCH_AUTO'])
        expect(result.ranked).toBe(true)
        expect(result.scratchAuto).toBe(true)
        expect(submissionMessage(result.ranked, result.flags)).toBe('saved (scratch auto)')
    })

    test('assist 태그 없이 scratch_auto 만 보낸 옛 클라이언트 판도 SCRATCH AUTO 판으로 ranked 다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ scratch_auto: true }),
            buildTrust: BUILD_TRUST.TRUSTED,
            isGuest: false,
            requireTrustedBuild: false,
        })
        expect(result).toEqual({ ranked: true, flags: ['SCRATCH_AUTO'], scratchAuto: true })
    })

    test('AUTO_SCRATCH 태그만 와도 SCRATCH AUTO 판으로 본다', () => {
        expect(isScratchAutoPlay({ scratch_auto: false, assist: ['AUTO_SCRATCH'] })).toBe(true)
        expect(assistFlags({ scratch_auto: false, assist: ['AUTO_SCRATCH'] })).toEqual(['SCRATCH_AUTO'])
    })

    test('AUTO_SCRATCH 밖의 어시스트 태그가 있으면 ASSIST 로 unranked 다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ assist: ['CUSTOM_JUDGE'], judge_rate: 120 }),
            buildTrust: BUILD_TRUST.TRUSTED,
            isGuest: false,
            requireTrustedBuild: false,
        })
        expect(result.flags).toEqual(['ASSIST', 'JUDGE_WIDTH'])
        expect(result.ranked).toBe(false)
    })

    test('SCRATCH AUTO 와 넓힌 판정을 함께 쓴 판은 넓힌 판정 때문에 unranked 이고 사유에 scratch_auto 를 넣지 않는다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ scratch_auto: true, assist: ['AUTO_SCRATCH', 'CUSTOM_JUDGE'], judge_rate: 120 }),
            buildTrust: BUILD_TRUST.TRUSTED,
            isGuest: false,
            requireTrustedBuild: false,
        })
        expect(result.flags).toEqual(['SCRATCH_AUTO', 'ASSIST', 'JUDGE_WIDTH'])
        expect(result.ranked).toBe(false)
        expect(submissionMessage(result.ranked, result.flags)).toBe('recorded (unranked: assist,judge_width)')
    })

    test('judge_rate 가 100 을 넘으면 JUDGE_WIDTH 로 unranked 다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ judge_rate: 120 }),
            buildTrust: BUILD_TRUST.TRUSTED,
            isGuest: false,
            requireTrustedBuild: false,
        })
        expect(result.flags).toEqual(['JUDGE_WIDTH'])
    })

    test('judge_rate 가 100 이하면 판정폭 flag 를 붙이지 않는다', () => {
        expect(
            rankedPolicy({ options: cleanOptions({ judge_rate: 100 }), buildTrust: BUILD_TRUST.TRUSTED, isGuest: false, requireTrustedBuild: false })
                .flags,
        ).toEqual([])
        expect(
            rankedPolicy({ options: cleanOptions({ judge_rate: 50 }), buildTrust: BUILD_TRUST.TRUSTED, isGuest: false, requireTrustedBuild: false })
                .flags,
        ).toEqual([])
        expect(
            rankedPolicy({ options: cleanOptions({ judge_rate: 0 }), buildTrust: BUILD_TRUST.TRUSTED, isGuest: false, requireTrustedBuild: false })
                .flags,
        ).toEqual([])
    })

    test('total_override 가 0 보다 크면 TOTAL_OVERRIDE 로 unranked 다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ total_override: 300 }),
            buildTrust: BUILD_TRUST.TRUSTED,
            isGuest: false,
            requireTrustedBuild: false,
        })
        expect(result.flags).toEqual(['TOTAL_OVERRIDE'])
    })

    test('미등록 빌드는 UNKNOWN_BUILD flag 를 남기지만 기본 설정에서는 ranked 를 막지 않는다', () => {
        const result = rankedPolicy({ options: cleanOptions(), buildTrust: BUILD_TRUST.UNKNOWN, isGuest: false, requireTrustedBuild: false })
        expect(result.flags).toEqual(['UNKNOWN_BUILD'])
        expect(result.ranked).toBe(true)
    })

    test('REQUIRE_BUILD_HASH 가 켜지면 미등록 빌드는 unranked 다', () => {
        const result = rankedPolicy({ options: cleanOptions(), buildTrust: BUILD_TRUST.UNKNOWN, isGuest: false, requireTrustedBuild: true })
        expect(result.flags).toEqual(['UNKNOWN_BUILD'])
        expect(result.ranked).toBe(false)
    })

    test('신뢰하지 않는 빌드는 설정과 무관하게 unranked 다', () => {
        const result = rankedPolicy({ options: cleanOptions(), buildTrust: BUILD_TRUST.UNTRUSTED, isGuest: false, requireTrustedBuild: false })
        expect(result.flags).toEqual(['UNKNOWN_BUILD'])
        expect(result.ranked).toBe(false)
    })

    test('guest 제출은 GUEST 로 unranked 다', () => {
        const result = rankedPolicy({ options: cleanOptions(), buildTrust: BUILD_TRUST.TRUSTED, isGuest: true, requireTrustedBuild: false })
        expect(result.ranked).toBe(false)
        expect(result.flags).toEqual(['GUEST'])
    })

    test('여러 위반은 선언 순서대로 누적된다', () => {
        const result = rankedPolicy({
            options: cleanOptions({ autoplay: true, scratch_auto: true, assist: ['A'], judge_rate: 130, total_override: 260 }),
            buildTrust: BUILD_TRUST.UNKNOWN,
            isGuest: true,
            requireTrustedBuild: false,
        })
        expect(result.flags).toEqual(['AUTOPLAY', 'SCRATCH_AUTO', 'ASSIST', 'JUDGE_WIDTH', 'TOTAL_OVERRIDE', 'UNKNOWN_BUILD', 'GUEST'])
        expect(result.ranked).toBe(false)
    })
})

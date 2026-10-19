import { describe, expect, test } from 'bun:test'
import { includeScratchAutoSchema } from '@server/dto/common'
import { chartLeaderboardQuerySchema, playerLeaderboardQuerySchema } from '@server/dto/fe'
import { rankingQuerySchema } from '@server/dto/score'
import { chartLeaderboardQueryOptions } from '../../src/entities/chart.query'
import { DEFAULT_INCLUDE_SCRATCH_AUTO, INCLUDE_SCRATCH_AUTO_PARAM } from '../../src/shared/constants/scratch-auto'
import { readIncludeScratchAutoParam, writeIncludeScratchAutoParam } from '../../src/shared/lib/scratch-auto-param'
import { serializeSearchParams } from '../../src/shared/lib/search-params'

describe('SCRATCH AUTO 포함 주소 파라미터', () => {
    test('파라미터가 없으면 켠 상태이고 false 일 때만 끈다', () => {
        expect(DEFAULT_INCLUDE_SCRATCH_AUTO).toBe(true)
        expect(readIncludeScratchAutoParam({})).toBe(true)
        expect(readIncludeScratchAutoParam({ [INCLUDE_SCRATCH_AUTO_PARAM]: 'false' })).toBe(false)
        expect(readIncludeScratchAutoParam({ [INCLUDE_SCRATCH_AUTO_PARAM]: 'true' })).toBe(true)
        expect(readIncludeScratchAutoParam({ [INCLUDE_SCRATCH_AUTO_PARAM]: 'garbage' })).toBe(true)
    })

    test('끄면 파라미터를 false 로 쓰고 켜면 파라미터를 지운다', () => {
        expect(serializeSearchParams({ [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(false) })).toBe('include_scratch_auto=false')
        expect(serializeSearchParams({ [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(true) })).toBe('')
    })

    test('쓴 주소를 다시 읽으면 같은 상태로 돌아온다', () => {
        for (const include of [true, false]) {
            const query = new URLSearchParams(serializeSearchParams({ [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(include) }))
            expect(readIncludeScratchAutoParam(Object.fromEntries(query))).toBe(include)
        }
    })

    test('서버 프리페치와 화면 기본 조회가 같은 캐시 키를 가리킨다', () => {
        const prefetched = chartLeaderboardQueryOptions('h', { include_scratch_auto: readIncludeScratchAutoParam({}) }).queryKey
        const displayed = chartLeaderboardQueryOptions('h', { include_scratch_auto: DEFAULT_INCLUDE_SCRATCH_AUTO }).queryKey
        expect(prefetched).toEqual(displayed)
    })
})

describe('서버 API 의 SCRATCH AUTO 포함 기본값', () => {
    test('파라미터를 싣지 않는 Rust 클라이언트를 위해 API 기본값은 끔으로 둔다', () => {
        expect(includeScratchAutoSchema.parse(undefined)).toBe(false)
        expect(chartLeaderboardQuerySchema.parse({}).include_scratch_auto).toBe(false)
        expect(playerLeaderboardQuerySchema.parse({}).include_scratch_auto).toBe(false)
        expect(rankingQuerySchema.parse({}).include_scratch_auto).toBe(false)
    })
})

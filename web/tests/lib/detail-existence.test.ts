import { describe, expect, test } from 'bun:test'
import { createDetailExistenceCheck, parseDetailPath, type DetailKind } from '@server/service/detail-existence'

const MD5 = 'a'.repeat(32)
const SHA256 = 'b'.repeat(64)

const createCheck = (known: Partial<Record<DetailKind, string[]>>) => {
    const looked: string[] = []
    const lookup = (kind: DetailKind) => async (id: string) => {
        looked.push(`${kind}:${id}`)
        return (known[kind] ?? []).includes(id)
    }
    const check = createDetailExistenceCheck(() => ({
        chart: lookup('chart'),
        player: lookup('player'),
        course: lookup('course'),
        replay: lookup('replay'),
    }))
    return { check, looked }
}

describe('parseDetailPath', () => {
    test('차트, 플레이어, 코스, 리플레이 상세 주소만 종류와 id 로 나눈다', () => {
        expect(parseDetailPath(`/charts/${MD5}`)).toEqual({ kind: 'chart', id: MD5 })
        expect(parseDetailPath('/players/kuro_neko/')).toEqual({ kind: 'player', id: 'kuro_neko' })
        expect(parseDetailPath('/courses/ch%201')).toEqual({ kind: 'course', id: 'ch 1' })
        expect(parseDetailPath('/replays/rp_1')).toEqual({ kind: 'replay', id: 'rp_1' })
    })

    test('목록, 하위 경로, 다른 페이지는 상세 주소가 아니다', () => {
        for (const pathname of ['/charts', '/charts/', `/charts/${MD5}/replays`, '/tables/insane1', '/settings', '/']) {
            expect(parseDetailPath(pathname)).toBeNull()
        }
    })

    test('깨진 퍼센트 인코딩은 판단하지 않는다', () => {
        expect(parseDetailPath('/players/%E0%A4%A')).toBeNull()
    })
})

describe('createDetailExistenceCheck', () => {
    test('있는 대상은 true, 없는 대상은 false 다', async () => {
        const { check } = createCheck({ chart: [SHA256], player: ['kuro_neko'], course: ['c1'], replay: ['rp_1'] })
        expect(await check(`/charts/${SHA256}`)).toBe(true)
        expect(await check('/players/kuro_neko')).toBe(true)
        expect(await check('/players/nobody')).toBe(false)
        expect(await check('/courses/c1')).toBe(true)
        expect(await check('/courses/missing')).toBe(false)
        expect(await check('/replays/rp_1')).toBe(true)
        expect(await check('/replays/rp_missing')).toBe(false)
    })

    test('해시 모양이 아닌 차트 주소는 조회 없이 false 다', async () => {
        const { check, looked } = createCheck({})
        expect(await check('/charts/not-a-hash')).toBe(false)
        expect(looked).toEqual([])
    })

    test('상세 주소가 아니면 null 이고 조회하지 않는다', async () => {
        const { check, looked } = createCheck({})
        expect(await check('/settings')).toBeNull()
        expect(looked).toEqual([])
    })

    test('조회가 실패하면 null 이라 멀쩡한 페이지를 404 로 만들지 않는다', async () => {
        const failing = createDetailExistenceCheck(() => ({
            chart: async () => {
                throw new Error('connection lost')
            },
            player: async () => true,
            course: async () => true,
            replay: async () => true,
        }))
        expect(await failing(`/charts/${SHA256}`)).toBeNull()
        const unresolved = createDetailExistenceCheck(() => {
            throw new Error('no database configuration')
        })
        expect(await unresolved('/players/kuro_neko')).toBeNull()
    })
})

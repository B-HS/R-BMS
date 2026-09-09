import { describe, expect, test } from 'bun:test'
import type { z } from 'zod'
import { chartMetaSchema } from '../../src/server/dto/chart'
import { playerProfileSchema } from '../../src/server/dto/auth'
import { settingBlobResponseSchema } from '../../src/server/dto/setting'
import { tableDataSchema } from '../../src/server/dto/table'
import { courseMetaSchema } from '../../src/server/dto/course'
import { scoreRecordSchema } from '../../src/server/dto/score'
import { toSettingKey } from '../../src/server/service/domain/setting/setting.service'
import { toClientBuild } from '../../src/server/service/domain/admin/build.service'
import type { ChartMeta } from '../../src/entities/chart.type'
import type { PlayerProfile } from '../../src/entities/player.type'
import type { SettingBlob, SettingBlobKey } from '../../src/entities/setting.type'
import type { TableData } from '../../src/entities/table.type'
import type { CourseMeta } from '../../src/entities/course.type'
import type { ScoreRecord } from '../../src/entities/score.type'
import type { ClientBuild } from '../../src/entities/build.type'

const CHART_SHA256 = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
const CHART_MD5 = 'b1946ac92492d2347c6235b4d2611184'

type Equals<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

const assertSameShape = <A, B>(...proof: Equals<A, B> extends true ? [] : ['서버 스키마와 FE 타입의 필드가 어긋났습니다']) => proof.length === 0

describe('entities 타입이 서버 응답 계약과 일치한다', () => {
    test('ChartMeta 는 chartMetaSchema 출력과 필드가 완전히 같다', () => {
        const parsed: ChartMeta = chartMetaSchema.parse({ sha256: CHART_SHA256, md5: CHART_MD5, title: 'conflict', notes: 2456, minbpm: 220 })
        expect(parsed.title).toBe('conflict')
        expect(parsed.notes).toBe(2456)
        expect(assertSameShape<ChartMeta, z.infer<typeof chartMetaSchema>>()).toBe(true)
    })

    test('PlayerProfile 은 playerProfileSchema 출력과 필드가 완전히 같다', () => {
        const parsed: PlayerProfile = playerProfileSchema.parse({ id: 'alice', name: 'alice', total_plays: 12, rank_points: 8421.5 })
        expect(parsed.rank_points).toBe(8421.5)
        expect(assertSameShape<PlayerProfile, z.infer<typeof playerProfileSchema>>()).toBe(true)
    })

    test('ScoreRecord 는 judge 가 없는 응답도 받는다', () => {
        const parsed: ScoreRecord = scoreRecordSchema.parse({
            player: { id: 'alice' },
            player_name: 'alice',
            clear: 'ExHard',
            ex_score: 4390,
            max_combo: 2456,
            minbp: 3,
            rank: 1,
            played_at: 1_757_000_000_000,
        })
        expect(parsed.judge).toBeNull()
        expect(parsed.rank).toBe(1)
        expect(assertSameShape<ScoreRecord, z.infer<typeof scoreRecordSchema>>()).toBe(true)
    })

    test('SettingBlob 은 settingBlobResponseSchema 출력과 같은 필드를 갖는다', () => {
        const parsed: SettingBlob = settingBlobResponseSchema.parse({ name: 'config', format: 'ron', content: '(a: 1)', updated_at: 10 })
        expect(parsed.content).toBe('(a: 1)')
        expect(assertSameShape<SettingBlob, z.infer<typeof settingBlobResponseSchema>>()).toBe(true)
    })

    test('SettingBlobKey 는 toSettingKey 결과와 필드가 완전히 같다', () => {
        const key: SettingBlobKey = toSettingKey({ name: 'config', format: 'ron', size: 6, updatedAt: 10 })
        expect(key.key).toBe('config')
        expect(key.size).toBe(6)
        expect(assertSameShape<SettingBlobKey, ReturnType<typeof toSettingKey>>()).toBe(true)
    })

    test('TableData 는 courses 를 CourseMeta 배열로 갖는다', () => {
        const parsed: TableData = tableDataSchema.parse({
            id: 'satellite',
            name: 'Satellite',
            folders: [{ name: 'sl0', charts: [{ md5: CHART_MD5, sha256: CHART_SHA256 }] }],
            courses: [{ course_hash: 'ch1', name: 'sl0 course' }],
        })
        expect(parsed.folders[0].charts[0].sha256).toBe(CHART_SHA256)
        expect(parsed.courses[0].course_hash).toBe('ch1')
        expect(assertSameShape<TableData, z.infer<typeof tableDataSchema>>()).toBe(true)
    })

    test('CourseMeta 의 charts 는 해시 쌍만 갖는다', () => {
        const parsed: CourseMeta = courseMetaSchema.parse({ course_hash: 'ch1', charts: [{ md5: CHART_MD5, sha256: CHART_SHA256 }] })
        expect(Object.keys(parsed.charts[0]).sort()).toEqual(['md5', 'sha256'])
        expect(assertSameShape<CourseMeta, z.infer<typeof courseMetaSchema>>()).toBe(true)
    })

    test('ClientBuild 는 toClientBuild 결과와 필드가 완전히 같다', () => {
        const build: ClientBuild = toClientBuild({
            sha256: CHART_SHA256.padEnd(64, '0'),
            version: '0.1.0',
            platform: 'macos-arm64',
            channel: 'stable',
            releasedAt: 1_757_000_000_000,
            trusted: true,
            note: '',
        })
        expect(build.released_at).toBe(1_757_000_000_000)
        expect(assertSameShape<ClientBuild, ReturnType<typeof toClientBuild>>()).toBe(true)
    })
})

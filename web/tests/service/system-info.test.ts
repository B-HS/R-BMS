import { describe, expect, test } from 'bun:test'
import { serverInfoSchema } from '@server/dto/system'
import { parseEnv } from '@server/lib/env'
import { getServerInfo, getVersionInfo } from '@server/service/domain/system/capabilities'

describe('getServerInfo', () => {
    test('ServerInfo 형태와 식별자를 반환한다', () => {
        const info = getServerInfo()
        expect(serverInfoSchema.safeParse(info).success).toBe(true)
        expect(info.name).toBe('rbms-ir')
        expect(info.ir_compat).toBe('superset-1')
    })

    test('capabilities 가 실제 구현된 엔드포인트 범위와 일치한다', () => {
        expect(getServerInfo().capabilities).toEqual({
            ranking: true,
            player_best: true,
            rivals: true,
            courses: true,
            replays: true,
            tables: true,
            settings_sync: true,
            folders: true,
            accounts: true,
            lr2ir_compat: false,
            traces: true,
        })
    })
})

describe('getVersionInfo', () => {
    test('api_version 1 과 서버 버전 문자열을 반환한다', () => {
        const version = getVersionInfo()
        expect(version.api_version).toBe(1)
        expect(typeof version.server).toBe('string')
    })
})

describe('getVersionInfo 의 commit', () => {
    test('VERCEL_GIT_COMMIT_SHA 의 앞 7자를 돌려준다', () => {
        expect(getVersionInfo(parseEnv({ VERCEL_GIT_COMMIT_SHA: '0123456789abcdef0123456789abcdef01234567' })).commit).toBe('0123456')
    })

    test('SERVER_COMMIT 이 있으면 그것이 우선한다', () => {
        expect(getVersionInfo(parseEnv({ SERVER_COMMIT: 'manual', VERCEL_GIT_COMMIT_SHA: '0123456789abcdef' })).commit).toBe('manual')
    })

    test('둘 다 없으면 null 이다', () => {
        expect(getVersionInfo(parseEnv({})).commit).toBeNull()
    })
})

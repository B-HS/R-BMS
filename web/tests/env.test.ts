import { describe, expect, test } from 'bun:test'
import { parseEnv, resolveDatabaseUrl } from '@server/lib/env'

describe('parseEnv', () => {
    test('필수 값이 없어도 기본값으로 파싱한다', () => {
        const env = parseEnv({})
        expect(env.NODE_ENV).toBe('development')
        expect(env.ALLOW_GUEST).toBe(false)
        expect(env.DB_PORT).toBe(3306)
        expect(env.REPLAY_MAX_BYTES).toBe(4 * 1024 * 1024)
    })

    test('ALLOW_GUEST 문자열 false 를 boolean false 로 변환한다', () => {
        expect(parseEnv({ ALLOW_GUEST: 'false' }).ALLOW_GUEST).toBe(false)
        expect(parseEnv({ ALLOW_GUEST: 'true' }).ALLOW_GUEST).toBe(true)
    })

    test('NODE_ENV 가 허용 값이 아니면 실패한다', () => {
        expect(() => parseEnv({ NODE_ENV: 'staging' })).toThrow()
    })

    test('REPLAY_MAX_BYTES 가 음수면 실패한다', () => {
        expect(() => parseEnv({ REPLAY_MAX_BYTES: '-1' })).toThrow()
    })
})

describe('resolveDatabaseUrl', () => {
    test('DATABASE_URL 이 있으면 그대로 쓴다', () => {
        const env = parseEnv({ DATABASE_URL: 'mysql://u:p@h:3306/d' })
        expect(resolveDatabaseUrl(env)).toBe('mysql://u:p@h:3306/d')
    })

    test('분리 변수로 URL 을 조립한다', () => {
        const env = parseEnv({ DB_HOST: 'db.local', DB_PORT: '3307', DB_USER: 'rbms', DB_PASSWORD: 'p@ss', DB_NAME: 'rbms' })
        expect(resolveDatabaseUrl(env)).toBe('mysql://rbms:p%40ss@db.local:3307/rbms')
    })

    test('접속 정보가 부족하면 null 이다', () => {
        expect(resolveDatabaseUrl(parseEnv({}))).toBeNull()
    })
})

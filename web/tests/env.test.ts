import { describe, expect, test } from 'bun:test'
import {
    CRON_SECRET_MIN_LENGTH,
    assertProductionAuthEnv,
    findMissingProductionAuthEnv,
    parseEnv,
    readAiCredentialKeyRing,
    resolveDatabaseUrl,
} from '@server/lib/env'

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

    test('CRON_SECRET 은 16자 미만이면 환경 파싱이 실패한다', () => {
        expect(() => parseEnv({ CRON_SECRET: 'x'.repeat(CRON_SECRET_MIN_LENGTH - 1) })).toThrow()
        expect(parseEnv({ CRON_SECRET: 'x'.repeat(CRON_SECRET_MIN_LENGTH) }).CRON_SECRET).toHaveLength(CRON_SECRET_MIN_LENGTH)
    })

    test('CRON_SECRET 은 없어도 된다', () => {
        expect(parseEnv({}).CRON_SECRET).toBeUndefined()
        expect(parseEnv({ CRON_SECRET: '' }).CRON_SECRET).toBeUndefined()
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

describe('운영 필수 인증 환경 변수', () => {
    const complete = {
        NODE_ENV: 'production',
        BETTER_AUTH_SECRET: 'x'.repeat(32),
        BETTER_AUTH_URL: 'https://bms.example',
        NEXT_PUBLIC_APP_URL: 'https://bms.example',
    }

    test('환경 변수가 하나도 없어도 파싱은 성공해 빌드가 막히지 않는다', () => {
        expect(() => parseEnv({ NODE_ENV: 'production' })).not.toThrow()
    })

    test('production 에서 비어 있거나 빠진 키를 모두 모아 알려준다', () => {
        expect(findMissingProductionAuthEnv({ NODE_ENV: 'production' })).toEqual(['BETTER_AUTH_SECRET', 'BETTER_AUTH_URL', 'NEXT_PUBLIC_APP_URL'])
        expect(findMissingProductionAuthEnv({ ...complete, BETTER_AUTH_SECRET: '', BETTER_AUTH_URL: undefined })).toEqual([
            'BETTER_AUTH_SECRET',
            'BETTER_AUTH_URL',
        ])
        expect(() => assertProductionAuthEnv({ NODE_ENV: 'production', NEXT_PUBLIC_APP_URL: 'https://bms.example' })).toThrow(
            'Missing required production environment variables: BETTER_AUTH_SECRET, BETTER_AUTH_URL',
        )
    })

    test('모두 있으면 통과하고 development 와 test 는 요구하지 않는다', () => {
        expect(() => assertProductionAuthEnv(complete)).not.toThrow()
        expect(findMissingProductionAuthEnv({ NODE_ENV: 'development' })).toEqual([])
        expect(findMissingProductionAuthEnv({ NODE_ENV: 'test' })).toEqual([])
        expect(findMissingProductionAuthEnv({})).toEqual([])
    })
})

describe('AI 키 암호화 환경 변수', () => {
    test('없어도 파싱은 성공하고 값이 이상해도 앱 전체 환경 파싱을 막지 않는다', () => {
        expect(parseEnv({ NODE_ENV: 'production' }).AI_CREDENTIAL_KEKS).toBeUndefined()
        expect(() => parseEnv({ NODE_ENV: 'production', AI_CREDENTIAL_KEKS: 'not json', AI_CREDENTIAL_KEK_CURRENT: 'v1' })).not.toThrow()
    })

    test('사용 시점 검사는 빠진 값과 틀린 값을 사유로만 구분한다', () => {
        expect(readAiCredentialKeyRing(parseEnv({}))).toEqual({ ok: false, reason: 'ai_credential_kek_missing' })
        expect(readAiCredentialKeyRing(parseEnv({ AI_CREDENTIAL_KEKS: 'not json', AI_CREDENTIAL_KEK_CURRENT: 'v1' }))).toEqual({
            ok: false,
            reason: 'ai_credential_kek_invalid',
        })
    })
})

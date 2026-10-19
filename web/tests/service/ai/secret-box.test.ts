import { describe, expect, test } from 'bun:test'
import { readAiCredentialKeyRing } from '@server/lib/env'
import { isAppError } from '@server/lib/error'
import { createSecretBox, loadSecretBox, type SealedSecret } from '@server/service/shared/secret-box/secret-box'
import { TEST_KEKS, testKekEnv } from '../../support/ai-memory-db'

const SECRET = 'sk-test-secret-value-0123456789'
const ALICE = { userId: 'user-alice', provider: 'anthropic' }

const boxFor = (current: keyof typeof TEST_KEKS, versions: (keyof typeof TEST_KEKS)[] = [current]) => loadSecretBox(testKekEnv(current, versions))

const flipFirstByte = (base64: string) => {
    const bytes = Buffer.from(base64, 'base64')
    bytes[0] ^= 1
    return bytes.toString('base64')
}

const thrownAppError = (run: () => unknown) => {
    try {
        run()
    } catch (error) {
        if (isAppError(error)) return error
    }
    return null
}

describe('봉투 암호화 왕복', () => {
    test('봉인한 값을 같은 사용자·공급자로 열면 원문이 나오고 재포장은 필요 없다', () => {
        const box = boxFor('v1')
        const sealed = box.seal(SECRET, ALICE)
        expect(box.open(sealed, ALICE)).toEqual({ ok: true, plaintext: SECRET, rewrapped: null })
    })

    test('봉인 결과 어디에도 원문이 없고 행마다 IV 와 데이터 키가 다르다', () => {
        const box = boxFor('v1')
        const first = box.seal(SECRET, ALICE)
        const second = box.seal(SECRET, ALICE)
        expect(JSON.stringify(first)).not.toContain(SECRET)
        expect(JSON.stringify(first)).not.toContain(Buffer.from(SECRET).toString('base64'))
        expect(first.iv).not.toBe(second.iv)
        expect(first.wrappedDek).not.toBe(second.wrappedDek)
        expect(first.ciphertext).not.toBe(second.ciphertext)
        expect(first.kekVersion).toBe('v1')
    })
})

describe('AAD 와 위조', () => {
    const box = boxFor('v1')
    const sealed = box.seal(SECRET, ALICE)

    test('다른 사용자 행으로 옮긴 암호문은 열리지 않는다', () => {
        expect(box.open(sealed, { ...ALICE, userId: 'user-bob' })).toEqual({ ok: false, reason: 'tampered' })
    })

    test('다른 공급자 행으로 옮긴 암호문은 열리지 않는다', () => {
        expect(box.open(sealed, { ...ALICE, provider: 'openai' })).toEqual({ ok: false, reason: 'tampered' })
    })

    test('암호문·태그·IV·감싼 데이터 키 중 하나라도 바뀌면 열리지 않는다', () => {
        const fields: (keyof Omit<SealedSecret, 'kekVersion'>)[] = ['ciphertext', 'authTag', 'iv', 'wrappedDek', 'dekTag', 'dekIv']
        for (const field of fields) {
            expect(box.open({ ...sealed, [field]: flipFirstByte(sealed[field]) }, ALICE)).toEqual({ ok: false, reason: 'tampered' })
        }
    })

    test('잘린 태그나 다른 길이의 IV 는 위조로 본다', () => {
        expect(box.open({ ...sealed, authTag: sealed.authTag.slice(0, 8) }, ALICE)).toEqual({ ok: false, reason: 'tampered' })
        expect(box.open({ ...sealed, iv: Buffer.alloc(16).toString('base64') }, ALICE)).toEqual({ ok: false, reason: 'tampered' })
    })

    test('키 버전 이름을 바꿔 다른 KEK 로 열게 해도 열리지 않는다', () => {
        const both = boxFor('v1', ['v1', 'v2'])
        expect(both.open({ ...both.seal(SECRET, ALICE), kekVersion: 'v2' }, ALICE)).toEqual({ ok: false, reason: 'tampered' })
    })

    test('환경에 없는 KEK 버전은 unknown_kek 이다', () => {
        expect(boxFor('v2').open(sealed, ALICE)).toEqual({ ok: false, reason: 'unknown_kek' })
    })
})

describe('KEK 회전과 지연 재포장', () => {
    test('옛 버전 행을 새 CURRENT 로 열면 원문과 함께 새 버전으로 다시 감싼 값을 준다', () => {
        const sealedV1 = boxFor('v1').seal(SECRET, ALICE)
        const rotated = boxFor('v2', ['v1', 'v2'])
        const opened = rotated.open(sealedV1, ALICE)
        expect(opened).toMatchObject({ ok: true, plaintext: SECRET })
        const rewrapped = opened.ok ? opened.rewrapped : null
        expect(rewrapped?.kekVersion).toBe('v2')
        expect(rewrapped?.ciphertext).toBe(sealedV1.ciphertext)
        expect(rewrapped?.wrappedDek).not.toBe(sealedV1.wrappedDek)
        expect(rewrapped && boxFor('v2').open(rewrapped, ALICE)).toEqual({ ok: true, plaintext: SECRET, rewrapped: null })
    })

    test('새로 봉인하는 값은 CURRENT 버전을 쓴다', () => {
        expect(boxFor('v2', ['v1', 'v2']).seal(SECRET, ALICE).kekVersion).toBe('v2')
    })
})

describe('KEK 환경 변수', () => {
    test('둘 다 없으면 SERVICE_NOT_CONFIGURED(503) 이고 값을 담지 않는다', () => {
        const error = thrownAppError(() => loadSecretBox({}))
        expect(error?.code).toBe('SERVICE_NOT_CONFIGURED')
        expect(error?.statusCode).toBe(503)
        expect(error?.details).toEqual({ reason: 'ai_credential_kek_missing' })
    })

    test('형식이 틀리면 invalid 사유로 503 이고 키 값은 오류에 없다', () => {
        const badEnvs = [
            { AI_CREDENTIAL_KEKS: 'not json', AI_CREDENTIAL_KEK_CURRENT: 'v1' },
            { AI_CREDENTIAL_KEKS: JSON.stringify({ v1: Buffer.alloc(16, 1).toString('base64') }), AI_CREDENTIAL_KEK_CURRENT: 'v1' },
            { AI_CREDENTIAL_KEKS: JSON.stringify({ v1: TEST_KEKS.v1 }), AI_CREDENTIAL_KEK_CURRENT: 'v2' },
            { AI_CREDENTIAL_KEKS: JSON.stringify({ 'v 1': TEST_KEKS.v1 }), AI_CREDENTIAL_KEK_CURRENT: 'v 1' },
            { AI_CREDENTIAL_KEKS: JSON.stringify({}), AI_CREDENTIAL_KEK_CURRENT: 'v1' },
        ]
        for (const env of badEnvs) {
            expect(readAiCredentialKeyRing(env)).toEqual({ ok: false, reason: 'ai_credential_kek_invalid' })
            const error = thrownAppError(() => loadSecretBox(env))
            expect(error?.statusCode).toBe(503)
            expect(JSON.stringify(error)).not.toContain(TEST_KEKS.v1)
        }
    })

    test('올바르면 버전별 32 바이트 키와 CURRENT 를 돌려준다', () => {
        const result = readAiCredentialKeyRing(testKekEnv('v2', ['v1', 'v2']))
        const ring = result.ok ? result.ring : null
        expect(ring?.current).toBe('v2')
        expect([...(ring?.keys.keys() ?? [])]).toEqual(['v1', 'v2'])
        expect(ring?.keys.get('v1')?.length).toBe(32)
        expect(ring && createSecretBox(ring).seal(SECRET, ALICE).kekVersion).toBe('v2')
    })
})

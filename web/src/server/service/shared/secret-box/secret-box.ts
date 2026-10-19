import { createCipheriv, createDecipheriv, randomBytes } from 'node:crypto'
import { readAiCredentialKeyRing, type Env } from '@server/lib/env'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'

const ALGORITHM = 'aes-256-gcm'
const DEK_BYTES = 32
const IV_BYTES = 12
const TAG_BYTES = 16
const ENCODING = 'base64'
const SECRET_AAD_PREFIX = 'rbms-ai-credential|1'
const DEK_AAD_PREFIX = 'rbms-ai-dek'

export type SecretBinding = { userId: string; provider: string }

export type SealedSecret = {
    ciphertext: string
    iv: string
    authTag: string
    wrappedDek: string
    dekIv: string
    dekTag: string
    kekVersion: string
}

export type SecretKeyRing = Extract<ReturnType<typeof readAiCredentialKeyRing>, { ok: true }>['ring']

const secretAad = ({ userId, provider }: SecretBinding) => `${SECRET_AAD_PREFIX}|${userId}|${provider}`

const dekAad = (kekVersion: string, { userId, provider }: SecretBinding) => `${DEK_AAD_PREFIX}|${kekVersion}|${userId}|${provider}`

const encrypt = (key: Buffer, plaintext: Buffer, aad: string) => {
    const iv = randomBytes(IV_BYTES)
    const cipher = createCipheriv(ALGORITHM, key, iv, { authTagLength: TAG_BYTES })
    cipher.setAAD(Buffer.from(aad, 'utf8'))
    const data = Buffer.concat([cipher.update(plaintext), cipher.final()])
    return { data: data.toString(ENCODING), iv: iv.toString(ENCODING), tag: cipher.getAuthTag().toString(ENCODING) }
}

const decrypt = (key: Buffer, sealed: { data: string; iv: string; tag: string }, aad: string) => {
    const iv = Buffer.from(sealed.iv, ENCODING)
    const tag = Buffer.from(sealed.tag, ENCODING)
    if (iv.length !== IV_BYTES || tag.length !== TAG_BYTES) return null
    try {
        const decipher = createDecipheriv(ALGORITHM, key, iv, { authTagLength: TAG_BYTES })
        decipher.setAAD(Buffer.from(aad, 'utf8'))
        decipher.setAuthTag(tag)
        return Buffer.concat([decipher.update(Buffer.from(sealed.data, ENCODING)), decipher.final()])
    } catch {
        return null
    }
}

const wrapDek = (ring: SecretKeyRing, dek: Buffer, binding: SecretBinding) => {
    const kek = ring.keys.get(ring.current)
    if (!kek) throw createAppError(ERROR_CODE.SERVICE_NOT_CONFIGURED, { reason: 'ai_credential_kek_invalid' })
    const wrapped = encrypt(kek, dek, dekAad(ring.current, binding))
    return { wrappedDek: wrapped.data, dekIv: wrapped.iv, dekTag: wrapped.tag, kekVersion: ring.current }
}

/**
 * Envelope encryption for one stored secret per row: a fresh 256-bit data key encrypts the secret with AES-256-GCM, and the
 * current key-encryption key wraps that data key. Both layers bind the owner and provider as additional authenticated data, so
 * a sealed value copied to another user's or provider's row does not open.
 */
export const createSecretBox = (ring: SecretKeyRing) => ({
    seal: (plaintext: string, binding: SecretBinding) => {
        const dek = randomBytes(DEK_BYTES)
        try {
            const sealed = encrypt(dek, Buffer.from(plaintext, 'utf8'), secretAad(binding))
            return { ciphertext: sealed.data, iv: sealed.iv, authTag: sealed.tag, ...wrapDek(ring, dek, binding) }
        } finally {
            dek.fill(0)
        }
    },

    /**
     * Opens a sealed secret. When it was wrapped under an older key version, `rewrapped` carries the same secret with its data key
     * wrapped under the current version, for the caller to store (lazy rewrap); otherwise it is null.
     */
    open: (sealed: SealedSecret, binding: SecretBinding) => {
        const kek = ring.keys.get(sealed.kekVersion)
        if (!kek) return { ok: false as const, reason: 'unknown_kek' as const }
        const dek = decrypt(kek, { data: sealed.wrappedDek, iv: sealed.dekIv, tag: sealed.dekTag }, dekAad(sealed.kekVersion, binding))
        if (!dek || dek.length !== DEK_BYTES) return { ok: false as const, reason: 'tampered' as const }
        try {
            const plaintext = decrypt(dek, { data: sealed.ciphertext, iv: sealed.iv, tag: sealed.authTag }, secretAad(binding))
            if (!plaintext) return { ok: false as const, reason: 'tampered' as const }
            const rewrapped = sealed.kekVersion === ring.current ? null : { ...sealed, ...wrapDek(ring, dek, binding) }
            const text = plaintext.toString('utf8')
            plaintext.fill(0)
            return { ok: true as const, plaintext: text, rewrapped }
        } finally {
            dek.fill(0)
        }
    },
})

export type SecretBox = ReturnType<typeof createSecretBox>

/** Builds the secret box from the environment at the moment it is needed; throws `SERVICE_NOT_CONFIGURED` (503) without the keys. */
export const loadSecretBox = (env: Pick<Env, 'AI_CREDENTIAL_KEKS' | 'AI_CREDENTIAL_KEK_CURRENT'>) => {
    const result = readAiCredentialKeyRing(env)
    if (!result.ok) throw createAppError(ERROR_CODE.SERVICE_NOT_CONFIGURED, { reason: result.reason })
    return createSecretBox(result.ring)
}

import { AI_FAILURE_KIND, AI_LIMIT, type AiFailureKind, type AiProviderId } from '@server/dto/ai'
import type { AiProviderResolver } from '@server/service/domain/ai/ai.type'
import type { SealedSecret, SecretBox } from '@server/service/shared/secret-box/secret-box'

/** How long one key check or model listing may wait for the provider. */
export const AI_KEY_CHECK_TIMEOUT_MS = 15_000

export type AiCredentialRow = SealedSecret & {
    provider: AiProviderId
    keyHint: string
    model: string | null
    createdAt: number
    lastValidatedAt: number | null
    lastErrorKind: AiFailureKind | null
}

type CredentialKey = { userId: string; provider: AiProviderId }

export type AiCredentialServiceDb = {
    listCredentials: (userId: string) => Promise<AiCredentialRow[]>
    findCredential: (params: CredentialKey) => Promise<AiCredentialRow | null>
    /** Inserts or replaces the key of one provider; a replaced key forgets its model, and `errorKind` is the check's warning or null. */
    upsertCredential: (
        params: CredentialKey & { sealed: SealedSecret; keyHint: string; validatedAt: number; errorKind: AiFailureKind | null; updatedAt: number },
    ) => Promise<void>
    updateModel: (params: CredentialKey & { model: string; updatedAt: number }) => Promise<boolean>
    recordCheck: (params: CredentialKey & { validatedAt: number | null; errorKind: AiFailureKind | null; updatedAt: number }) => Promise<void>
    /** Stores a data key rewrapped under the current key version, only while the row still holds the wrapped data key it was read with. */
    rewrapCredential: (
        params: CredentialKey & { previous: Pick<SealedSecret, 'kekVersion' | 'wrappedDek' | 'dekIv'>; sealed: SealedSecret },
    ) => Promise<void>
    deleteCredential: (params: CredentialKey) => Promise<boolean>
}

type AiCredentialServiceDeps = {
    db: AiCredentialServiceDb
    getProvider: AiProviderResolver
    getSecretBox: () => SecretBox
    now: () => number
}

const toCredentialOutput = (row: Pick<AiCredentialRow, 'provider' | 'keyHint' | 'model' | 'createdAt' | 'lastValidatedAt' | 'lastErrorKind'>) => ({
    provider: row.provider,
    key_hint: row.keyHint,
    model: row.model,
    created_at: row.createdAt,
    last_validated_at: row.lastValidatedAt,
    last_error_kind: row.lastErrorKind,
})

const keyCheckSignal = () => AbortSignal.timeout(AI_KEY_CHECK_TIMEOUT_MS)

const toSealed = (row: AiCredentialRow) => ({
    ciphertext: row.ciphertext,
    iv: row.iv,
    authTag: row.authTag,
    wrappedDek: row.wrappedDek,
    dekIv: row.dekIv,
    dekTag: row.dekTag,
    kekVersion: row.kekVersion,
})

/** A 403 means the provider took the key but refused this resource, so the key itself is authenticated and only warned about. */
const isAuthenticated = (errorKind: AiFailureKind | null) => errorKind === null || errorKind === AI_FAILURE_KIND.FORBIDDEN

export const createAiCredentialService = (deps: AiCredentialServiceDeps) => {
    const withApiKey = async <T>(key: CredentialKey, run: (apiKey: string, row: AiCredentialRow) => Promise<T>) => {
        const box = deps.getSecretBox()
        const row = await deps.db.findCredential(key)
        if (!row) return { ok: false, reason: 'credential_not_found' } as const
        const opened = box.open(toSealed(row), key)
        if (!opened.ok) return { ok: false, reason: 'credential_undecryptable', cause: opened.reason } as const
        if (opened.rewrapped) {
            const previous = { kekVersion: row.kekVersion, wrappedDek: row.wrappedDek, dekIv: row.dekIv }
            await Promise.allSettled([deps.db.rewrapCredential({ ...key, previous, sealed: opened.rewrapped })])
        }
        return { ok: true, value: await run(opened.plaintext, row) } as const
    }

    const recordCheck = async (key: CredentialKey, errorKind: AiFailureKind | null) => {
        const now = deps.now()
        await deps.db.recordCheck({ ...key, validatedAt: isAuthenticated(errorKind) ? now : null, errorKind, updatedAt: now })
    }

    return {
        list: async (userId: string) => (await deps.db.listCredentials(userId)).map(toCredentialOutput),

        /**
         * Checks the key with its provider and stores it encrypted when the provider authenticates it. A 403 still stores the key,
         * with `forbidden` as its warning, because the model list or a model may be what is refused. The result shows the last characters only.
         */
        register: async ({ userId, provider, apiKey }: CredentialKey & { apiKey: string }) => {
            const box = deps.getSecretBox()
            const check = await deps.getProvider(provider).validateKey({ apiKey, signal: keyCheckSignal() })
            const errorKind = check.ok ? null : check.kind
            if (!check.ok && !isAuthenticated(errorKind)) return { ok: false, reason: 'provider', provider, detail: check } as const
            const now = deps.now()
            const keyHint = apiKey.slice(-AI_LIMIT.KEY_HINT_LENGTH)
            await deps.db.upsertCredential({
                userId,
                provider,
                sealed: box.seal(apiKey, { userId, provider }),
                keyHint,
                validatedAt: now,
                errorKind,
                updatedAt: now,
            })
            const stored = await deps.db.findCredential({ userId, provider })
            return {
                ok: true,
                value: toCredentialOutput(
                    stored ?? { provider, keyHint, model: null, createdAt: now, lastValidatedAt: now, lastErrorKind: errorKind },
                ),
            } as const
        },

        setModel: async ({ userId, provider, model }: CredentialKey & { model: string }) => {
            if (!(await deps.db.updateModel({ userId, provider, model, updatedAt: deps.now() }))) return null
            const row = await deps.db.findCredential({ userId, provider })
            return row ? toCredentialOutput(row) : null
        },

        remove: (key: CredentialKey) => deps.db.deleteCredential(key),

        /** Checks a stored key again. A 403 is kept as a warning on the card and the check still succeeds. */
        revalidate: async (key: CredentialKey) => {
            const checked = await withApiKey(key, (apiKey) => deps.getProvider(key.provider).validateKey({ apiKey, signal: keyCheckSignal() }))
            if (!checked.ok) return checked
            const errorKind = checked.value.ok ? null : checked.value.kind
            await recordCheck(key, errorKind)
            if (!checked.value.ok && !isAuthenticated(errorKind))
                return { ok: false, reason: 'provider', provider: key.provider, detail: checked.value } as const
            const row = await deps.db.findCredential(key)
            if (!row) return { ok: false, reason: 'credential_not_found' } as const
            return { ok: true, value: toCredentialOutput(row) } as const
        },

        listModels: async (key: CredentialKey) => {
            const listed = await withApiKey(key, (apiKey) => deps.getProvider(key.provider).listModels({ apiKey, signal: keyCheckSignal() }))
            if (!listed.ok) return listed
            if (!listed.value.ok) {
                if (listed.value.kind === AI_FAILURE_KIND.INVALID_KEY) await recordCheck(key, listed.value.kind)
                return { ok: false, reason: 'provider', provider: key.provider, detail: listed.value } as const
            }
            return { ok: true, value: listed.value.models } as const
        },

        /** Marks a stored key as rejected by its provider (401 only), so the settings card shows it needs registering again. */
        markRejected: (key: CredentialKey) => recordCheck(key, AI_FAILURE_KIND.INVALID_KEY),

        /**
         * Runs `run` with the decrypted key. The plain key exists only as the callback's argument: it is never returned, stored or
         * logged. A key wrapped under an older key version is rewrapped under the current one on the way (lazy rewrap); that write is
         * best effort, so a failed or lost rewrap leaves the old wrapping in place and the call goes on.
         */
        withApiKey,
    }
}

export type AiCredentialService = ReturnType<typeof createAiCredentialService>

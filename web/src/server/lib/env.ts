import { z } from 'zod'

const DEFAULT_DB_PORT = 3306
const DEFAULT_REPLAY_MAX_BYTES = 4 * 1024 * 1024

const envSchema = z.object({
    DATABASE_URL: z.string().min(1).optional(),
    DB_HOST: z.string().min(1).optional(),
    DB_PORT: z.coerce.number().int().positive().default(DEFAULT_DB_PORT),
    DB_USER: z.string().min(1).optional(),
    DB_PASSWORD: z.string().optional(),
    DB_NAME: z.string().min(1).optional(),
    BETTER_AUTH_SECRET: z.string().min(1).optional(),
    BETTER_AUTH_URL: z.string().min(1).default('http://localhost:3000'),
    NEXT_PUBLIC_APP_URL: z.string().min(1).default('http://localhost:3000'),
    ALLOW_GUEST: z.stringbool({ truthy: ['true', '1'], falsy: ['false', '0'] }).default(false),
    REPLAY_MAX_BYTES: z.coerce.number().int().positive().default(DEFAULT_REPLAY_MAX_BYTES),
    REQUIRE_BUILD_HASH: z.stringbool({ truthy: ['true', '1'], falsy: ['false', '0'] }).default(false),
    REPLAY_STORAGE: z.enum(['db', 'blob']).default('db'),
    BLOB_READ_WRITE_TOKEN: z.string().min(1).optional(),
    SERVER_VERSION: z.string().min(1).optional(),
    SERVER_COMMIT: z.string().min(1).optional(),
    VERCEL_GIT_COMMIT_SHA: z.string().min(1).optional(),
    AI_CREDENTIAL_KEKS: z.string().min(1).optional(),
    AI_CREDENTIAL_KEK_CURRENT: z.string().min(1).optional(),
    NODE_ENV: z.enum(['development', 'production', 'test']).default('development'),
})

export type Env = z.infer<typeof envSchema>

const withoutBlanks = (source: Record<string, string | undefined>) =>
    Object.fromEntries(Object.entries(source).filter(([, value]) => value !== undefined && value !== ''))

export const parseEnv = (source: Record<string, string | undefined>) => {
    const result = envSchema.safeParse(withoutBlanks(source))
    if (!result.success) {
        const issues = result.error.issues.map((issue) => `${issue.path.join('.')}: ${issue.message}`).join(', ')
        throw new Error(`Invalid environment variables: ${issues}`)
    }
    return result.data
}

const PRODUCTION_AUTH_ENV_KEYS = ['BETTER_AUTH_SECRET', 'BETTER_AUTH_URL', 'NEXT_PUBLIC_APP_URL'] as const

/**
 * The auth settings a production deployment must set itself. The schema keeps them optional or defaulted so a build with no
 * environment still succeeds; this is checked when auth is first used instead.
 */
export const findMissingProductionAuthEnv = (source: Record<string, string | undefined>) => {
    if (source.NODE_ENV !== 'production') return []
    const present = withoutBlanks(source)
    return PRODUCTION_AUTH_ENV_KEYS.filter((key) => !(key in present))
}

export const assertProductionAuthEnv = (source: Record<string, string | undefined> = process.env) => {
    const missing = findMissingProductionAuthEnv(source)
    if (missing.length > 0) throw new Error(`Missing required production environment variables: ${missing.join(', ')}`)
}

const KEK_BYTES = 32
const KEK_BASE64_PATTERN = /^[A-Za-z0-9+/]{43}=$/
const KEK_VERSION_PATTERN = /^[A-Za-z0-9_-]{1,32}$/

const kekMapSchema = z.record(z.string().regex(KEK_VERSION_PATTERN), z.string().regex(KEK_BASE64_PATTERN))

const parseJsonOrNull = (text: string): unknown => {
    try {
        return JSON.parse(text)
    } catch {
        return null
    }
}

/**
 * Reads the key-encryption keys for stored AI credentials from `AI_CREDENTIAL_KEKS` (a JSON object of version to base64 of 32 bytes)
 * and `AI_CREDENTIAL_KEK_CURRENT`. Both stay optional in the schema so a build with no environment still succeeds; callers check
 * the result when a credential is first sealed or opened. A failure names only the reason, never a key value.
 */
export const readAiCredentialKeyRing = (env: Pick<Env, 'AI_CREDENTIAL_KEKS' | 'AI_CREDENTIAL_KEK_CURRENT'>) => {
    if (!env.AI_CREDENTIAL_KEKS || !env.AI_CREDENTIAL_KEK_CURRENT) return { ok: false as const, reason: 'ai_credential_kek_missing' as const }
    const parsed = kekMapSchema.safeParse(parseJsonOrNull(env.AI_CREDENTIAL_KEKS))
    const entries = parsed.success ? Object.entries(parsed.data).map(([version, value]) => [version, Buffer.from(value, 'base64')] as const) : []
    const keys = new Map(entries)
    const isValid = entries.length > 0 && entries.every(([, key]) => key.length === KEK_BYTES) && keys.has(env.AI_CREDENTIAL_KEK_CURRENT)
    if (!isValid) return { ok: false as const, reason: 'ai_credential_kek_invalid' as const }
    return { ok: true as const, ring: { current: env.AI_CREDENTIAL_KEK_CURRENT, keys } }
}

export const resolveDatabaseUrl = (env: Env) => {
    if (env.DATABASE_URL) return env.DATABASE_URL
    if (!env.DB_HOST || !env.DB_USER || !env.DB_NAME) return null
    const password = env.DB_PASSWORD ? `:${encodeURIComponent(env.DB_PASSWORD)}` : ''
    return `mysql://${encodeURIComponent(env.DB_USER)}${password}@${env.DB_HOST}:${env.DB_PORT}/${env.DB_NAME}`
}

let cachedEnv: Env | null = null

export const getEnv = () => {
    if (cachedEnv) return cachedEnv
    cachedEnv = parseEnv(process.env)
    return cachedEnv
}

export const getRequiredDatabaseUrl = () => {
    const url = resolveDatabaseUrl(getEnv())
    if (!url) throw new Error('Missing database configuration: set DATABASE_URL or DB_HOST/DB_USER/DB_NAME')
    return url
}

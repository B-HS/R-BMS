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

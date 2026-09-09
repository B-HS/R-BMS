import 'server-only'
import { betterAuth } from 'better-auth'
import { drizzleAdapter } from 'better-auth/adapters/drizzle'
import { getDb } from '@server/db'
import * as authSchema from '@server/db/auth-schema'
import { getEnv } from '@server/lib/env'

const SESSION_EXPIRES_SECONDS = 60 * 60 * 24 * 30
const SESSION_UPDATE_AGE_SECONDS = 60 * 60 * 24
const MIN_PASSWORD_LENGTH = 8

const createAuth = () => {
    const env = getEnv()
    return betterAuth({
        appName: 'rbms',
        baseURL: env.BETTER_AUTH_URL,
        secret: env.BETTER_AUTH_SECRET,
        basePath: '/api/auth',
        database: drizzleAdapter(getDb(), { provider: 'mysql', schema: authSchema }),
        emailAndPassword: { enabled: true, autoSignIn: true, minPasswordLength: MIN_PASSWORD_LENGTH },
        session: { expiresIn: SESSION_EXPIRES_SECONDS, updateAge: SESSION_UPDATE_AGE_SECONDS },
        user: {
            additionalFields: {
                loginId: { type: 'string', required: true, input: true },
                rank: { type: 'string', required: false, input: false, defaultValue: '' },
                rankPoints: { type: 'number', required: false, input: false, defaultValue: 0 },
                totalPlays: { type: 'number', required: false, input: false, defaultValue: 0 },
                role: { type: 'string', required: false, input: false, defaultValue: 'user' },
                allowGuestMerge: { type: 'boolean', required: false, input: false, defaultValue: false },
            },
        },
    })
}

let authInstance: ReturnType<typeof createAuth> | null = null

export const getAuth = () => {
    if (authInstance) return authInstance
    authInstance = createAuth()
    return authInstance
}

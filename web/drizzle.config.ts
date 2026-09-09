import { defineConfig } from 'drizzle-kit'
import { parseEnv, resolveDatabaseUrl } from './src/server/lib/env'

export default defineConfig({
    dialect: 'mysql',
    schema: ['./src/server/db/schema.ts', './src/server/db/auth-schema.ts'],
    out: './drizzle',
    dbCredentials: { url: resolveDatabaseUrl(parseEnv(process.env)) ?? '' },
})

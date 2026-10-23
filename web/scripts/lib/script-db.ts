import { drizzle } from 'drizzle-orm/mysql2'
import mysql from 'mysql2'
import * as authSchema from '../../src/server/db/auth-schema'
import * as domainSchema from '../../src/server/db/schema'
import { parseEnv, resolveDatabaseUrl } from '../../src/server/lib/env'

const SCRIPT_CONNECTION_LIMIT = 2
const CONNECT_TIMEOUT_MS = 10_000

/** The lock wait the web server sets on its connections (`LOCK_WAIT_TIMEOUT_SECONDS` of `src/server/db`, which a script can not import: it needs `server-only`). */
const LOCK_WAIT_TIMEOUT_SECONDS = 5

const schema = { ...authSchema, ...domainSchema }

export const DATABASE_NOT_SET_MESSAGE = 'DATABASE_URL (or DB_HOST, DB_USER, DB_NAME) is not set'

/**
 * The database for an operations script, built the way the web server builds its own (`src/server/db`) without importing it,
 * or null when the environment names no database. The connection settings come from the environment only; `close` ends the
 * pool so the script can exit.
 */
export const createScriptDb = (source: Record<string, string | undefined> = process.env) => {
    const uri = resolveDatabaseUrl(parseEnv(source))
    if (!uri) return null
    const pool = mysql.createPool({ uri, connectionLimit: SCRIPT_CONNECTION_LIMIT, connectTimeout: CONNECT_TIMEOUT_MS })
    pool.on('connection', (connection) => {
        connection.query('SET SESSION innodb_lock_wait_timeout = ?', [LOCK_WAIT_TIMEOUT_SECONDS], (error) => {
            if (error) console.error('could not set the lock wait timeout', error.code)
        })
    })
    return { db: drizzle(pool, { schema, mode: 'default' }), close: () => pool.promise().end() }
}

import 'server-only'
import { drizzle } from 'drizzle-orm/mysql2'
import mysql from 'mysql2'
import { getRequiredDatabaseUrl } from '@server/lib/env'
import * as authSchema from '@server/db/auth-schema'
import * as domainSchema from '@server/db/schema'

const CONNECTION_LIMIT = 5

/**
 * How long, in seconds, one statement waits for a row lock before MySQL gives up with 1205. The server default (50) times
 * the lock retries would let one request wait past the function's time limit.
 */
export const LOCK_WAIT_TIMEOUT_SECONDS = 5

export const schema = { ...authSchema, ...domainSchema }

const createPool = () => {
    const pool = mysql.createPool({ uri: getRequiredDatabaseUrl(), connectionLimit: CONNECTION_LIMIT, enableKeepAlive: true })
    pool.on('connection', (connection) => {
        connection.query('SET SESSION innodb_lock_wait_timeout = ?', [LOCK_WAIT_TIMEOUT_SECONDS], (error) => {
            if (error) console.error(error)
        })
    })
    return pool
}

const createDb = () => drizzle(createPool(), { schema, mode: 'default' })

let dbInstance: ReturnType<typeof createDb> | null = null

export const getDb = () => {
    if (dbInstance) return dbInstance
    dbInstance = createDb()
    return dbInstance
}

export type Database = ReturnType<typeof getDb>

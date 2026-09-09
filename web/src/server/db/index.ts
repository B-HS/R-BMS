import 'server-only'
import { drizzle } from 'drizzle-orm/mysql2'
import mysql from 'mysql2/promise'
import { getRequiredDatabaseUrl } from '@server/lib/env'
import * as authSchema from '@server/db/auth-schema'
import * as domainSchema from '@server/db/schema'

const CONNECTION_LIMIT = 5

export const schema = { ...authSchema, ...domainSchema }

const createDb = () =>
    drizzle(mysql.createPool({ uri: getRequiredDatabaseUrl(), connectionLimit: CONNECTION_LIMIT, enableKeepAlive: true }), {
        schema,
        mode: 'default',
    })

let dbInstance: ReturnType<typeof createDb> | null = null

export const getDb = () => {
    if (dbInstance) return dbInstance
    dbInstance = createDb()
    return dbInstance
}

export type Database = ReturnType<typeof getDb>

import { readFile } from 'node:fs/promises'
import { isAbsolute, join, relative, resolve } from 'node:path'
import mysql, { type RowDataPacket } from 'mysql2/promise'
import { parseEnv, resolveDatabaseUrl } from '../src/server/lib/env'

const SCRIPTS_DIR = import.meta.dirname
const CONNECT_TIMEOUT_MS = 10_000
const EXIT_CODE = { USAGE: 2, FAILED: 1 } as const

const usage = () => {
    console.error('usage: bun scripts/run-sql.ts <file under web/scripts/ ending in .sql>')
    process.exit(EXIT_CODE.USAGE)
}

const scriptPath = (argument: string | undefined) => {
    if (!argument || !argument.endsWith('.sql')) return usage()
    const path = resolve(SCRIPTS_DIR, argument)
    const inside = relative(SCRIPTS_DIR, path)
    if (inside.startsWith('..') || isAbsolute(inside)) return usage()
    return path
}

const affectedRows = (result: unknown) =>
    typeof result === 'object' && result !== null && 'affectedRows' in result && typeof result.affectedRows === 'number' ? result.affectedRows : 0

const isRowSet = (result: unknown): result is RowDataPacket[] => Array.isArray(result)

const report = (result: unknown, index: number) => {
    if (isRowSet(result)) {
        console.log(`-- result ${index + 1}: ${result.length} row(s)`)
        if (result.length > 0) console.table(result)
        return
    }
    console.log(`-- statement ${index + 1}: ${affectedRows(result)} row(s) affected`)
}

const main = async () => {
    const path = scriptPath(process.argv[2])
    const url = resolveDatabaseUrl(parseEnv(process.env))
    if (!url) {
        console.error('DATABASE_URL (or DB_HOST, DB_USER, DB_NAME) is not set')
        process.exit(EXIT_CODE.FAILED)
    }
    const sql = await readFile(path, 'utf8')
    console.log(`running ${join('web/scripts', relative(SCRIPTS_DIR, path))}`)
    const connection = await mysql.createConnection({ uri: url, multipleStatements: true, connectTimeout: CONNECT_TIMEOUT_MS })
    try {
        const [results] = await connection.query(sql)
        const list =
            Array.isArray(results) && results.some((item) => isRowSet(item) || (typeof item === 'object' && item !== null && 'affectedRows' in item))
                ? results
                : [results]
        list.forEach((result, index) => report(result, index))
    } finally {
        await connection.end()
    }
}

main().catch((error: unknown) => {
    console.error(error instanceof Error ? `failed: ${error.message}` : 'failed')
    process.exit(EXIT_CODE.FAILED)
})

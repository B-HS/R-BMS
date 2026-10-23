import { composeDan } from '../src/server/compose/dan'
import type { DanService } from '../src/server/service/domain/dan/dan.service'
import { createScriptDb, DATABASE_NOT_SET_MESSAGE } from './lib/script-db'
import { EXIT_CODE } from './seed-courses'

export const RECOMPUTE_COMMAND = { COUNT: 'count', APPLY: 'apply' } as const

export type RecomputeCommand = (typeof RECOMPUTE_COMMAND)[keyof typeof RECOMPUTE_COMMAND]

export const USAGE = 'usage: bun scripts/dan-recompute.ts <count|apply>'

export const parseRecomputeCommand = (args: readonly string[]) => {
    const [command, ...rest] = args
    return rest.length === 0 ? Object.values(RECOMPUTE_COMMAND).find((candidate) => candidate === command) : undefined
}

/**
 * Judges every stored run of every dan course again. `count` only counts what `apply` would do; `apply` gives the dans the runs
 * earn. A player keeps the dan of the first run that earned it, so running `apply` twice gives nothing the second time. Only
 * numbers are printed: the Actions log is public.
 */
export const recomputeDans = async (params: {
    command: RecomputeCommand
    service: Pick<DanService, 'listDanCourseHashes' | 'awardForCourses'>
    log: (line: string) => void
}) => {
    const { command, service, log } = params
    const courseHashes = await service.listDanCourseHashes()
    const totals = await service.awardForCourses({ courseHashes, dryRun: command === RECOMPUTE_COMMAND.COUNT })
    log(
        `${command === RECOMPUTE_COMMAND.COUNT ? 'plan' : 'applied'} danCourses ${courseHashes.length}, scanned ${totals.scanned}, eligible ${totals.eligible}, awarded ${totals.awarded}`,
    )
    return EXIT_CODE.OK
}

/** What is printed for a failure: only the kind and code of the error, since a database error can name the server it could not reach. */
const describeFailure = (error: unknown) => {
    const code = typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : 'UNKNOWN'
    return `${error instanceof Error ? error.name : typeof error} ${code}`
}

const main = async () => {
    const command = parseRecomputeCommand(process.argv.slice(2))
    if (!command) {
        console.error(USAGE)
        return EXIT_CODE.USAGE
    }
    const scriptDb = createScriptDb()
    if (!scriptDb) {
        console.error(DATABASE_NOT_SET_MESSAGE)
        return EXIT_CODE.FAILED
    }
    const { db, close } = scriptDb
    try {
        return await recomputeDans({ command, service: composeDan(db).danService, log: console.log })
    } finally {
        await close()
    }
}

if (import.meta.main) {
    main()
        .then((code) => {
            process.exitCode = code
        })
        .catch((error: unknown) => {
            console.error(`failed: ${describeFailure(error)}`)
            process.exitCode = EXIT_CODE.FAILED
        })
}

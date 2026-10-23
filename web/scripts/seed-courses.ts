import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { composeCourseCatalog } from '../src/server/compose/course-catalog'
import { bundledCoursesFileSchema } from '../src/server/dto/course-catalog'
import { isAppError } from '../src/server/lib/error'
import { bundledCatalog } from '../src/server/service/domain/course-catalog/bundled-courses'
import type { CatalogDefinition, CatalogSyncPlan } from '../src/server/service/domain/course-catalog/course-catalog.type'
import type { CourseCatalogService } from '../src/server/service/domain/course-catalog/course-catalog.service'
import { createScriptDb, DATABASE_NOT_SET_MESSAGE } from './lib/script-db'

export const COURSES_FILE = join(import.meta.dirname, '..', 'data', 'courses', 'genoside-2018', 'courses.json')

export const SEED_COMMAND = { COUNT: 'count', APPLY: 'apply' } as const

export type SeedCommand = (typeof SEED_COMMAND)[keyof typeof SEED_COMMAND]

export const EXIT_CODE = { OK: 0, FAILED: 1, USAGE: 2 } as const

export const USAGE = 'usage: bun scripts/seed-courses.ts <count|apply>'

export const parseSeedCommand = (args: readonly string[]) => {
    const [command, ...rest] = args
    return rest.length === 0 ? Object.values(SEED_COMMAND).find((candidate) => candidate === command) : undefined
}

const hasPendingChange = (plan: CatalogSyncPlan) => plan.groups.some((group) => group.inserted + group.changed + group.deactivated > 0)

/** The plan as printed: one line per group with numbers only. The Actions log is public, so no course name, hash or row content is printed. */
export const formatPlan = (plan: CatalogSyncPlan) =>
    plan.groups.map(
        (group) =>
            `${group.id}: inserted ${group.inserted}, changed ${group.changed}, unchanged ${group.unchanged}, deactivated ${group.deactivated}, danCourses ${group.danCourses}, newDanCourses ${group.newDanHashes.length}`,
    )

/**
 * `count` only reads and prints what `apply` would do. `apply` prints that plan, registers the definition, then plans again
 * and succeeds only when nothing is left to change, so running it twice is the same as running it once.
 */
export const seedCourses = async (params: {
    command: SeedCommand
    service: Pick<CourseCatalogService, 'planSync' | 'applySync'>
    definition: CatalogDefinition
    log: (line: string) => void
}) => {
    const { command, service, definition, log } = params
    if (command === SEED_COMMAND.COUNT) {
        formatPlan(await service.planSync(definition)).forEach((line) => log(`plan ${line}`))
        return EXIT_CODE.OK
    }
    formatPlan(await service.applySync(definition)).forEach((line) => log(`applied ${line}`))
    const remaining = await service.planSync(definition)
    formatPlan(remaining).forEach((line) => log(`remaining ${line}`))
    return hasPendingChange(remaining) ? EXIT_CODE.FAILED : EXIT_CODE.OK
}

const ZOD_ERROR_NAME = 'ZodError'

/** What is printed for a failure: the reason of a refused data file, else only the kind and code of the error, since a database error can name the server it could not reach. */
const describeFailure = (error: unknown) => {
    if (isAppError(error)) return `${error.code} ${JSON.stringify(error.details ?? {})}`
    if (error instanceof Error && error.name === ZOD_ERROR_NAME) return error.message
    const code = typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : 'UNKNOWN'
    return `${error instanceof Error ? error.name : typeof error} ${code}`
}

const main = async () => {
    const command = parseSeedCommand(process.argv.slice(2))
    if (!command) {
        console.error(USAGE)
        return EXIT_CODE.USAGE
    }
    const definition = bundledCatalog(bundledCoursesFileSchema.parse(JSON.parse(await readFile(COURSES_FILE, 'utf8'))))
    const scriptDb = createScriptDb()
    if (!scriptDb) {
        console.error(DATABASE_NOT_SET_MESSAGE)
        return EXIT_CODE.FAILED
    }
    const { db, close } = scriptDb
    try {
        const { courseCatalogService } = composeCourseCatalog(db)
        return await seedCourses({ command, service: courseCatalogService, definition, log: console.log })
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

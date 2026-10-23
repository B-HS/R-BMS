import { NextResponse } from 'next/server'
import { successResponse } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, withRateLimit } from '@server/lib/with-rate-limit'
import { readCourseCatalog, readCourseCatalogEntry } from '@server/service/public-reads'

export type CourseCatalogRouteDeps = {
    readCatalog: typeof readCourseCatalog
    readEntry: typeof readCourseCatalogEntry
}

type CourseCatalogContext = { params: Promise<{ courseHash: string }> }

const resolveCourseCatalogRouteDeps = (): CourseCatalogRouteDeps => ({ readCatalog: readCourseCatalog, readEntry: readCourseCatalogEntry })

const withPublicRead = <TContext>(handler: (request: Request, context: TContext) => Promise<Response>) =>
    withErrorHandling(withRateLimit('fe-read', RATE_LIMIT.PUBLIC_READ)(handler))

/** `GET /api/fe/courses`: every course group with where its courses come from and the dan each course gives. */
export const createCourseCatalogListRoute = (resolveDeps: () => CourseCatalogRouteDeps = resolveCourseCatalogRouteDeps) =>
    withPublicRead(async () => NextResponse.json(successResponse(await resolveDeps().readCatalog())))

/** `GET /api/fe/courses/{courseHash}/catalog`: one course's group, source, dan and chart titles; 404 when the course is no part of the catalog. */
export const createCourseCatalogEntryRoute = (resolveDeps: () => CourseCatalogRouteDeps = resolveCourseCatalogRouteDeps) =>
    withPublicRead(async (_request: Request, context: CourseCatalogContext) => {
        const { courseHash } = await context.params
        const entry = await resolveDeps().readEntry(courseHash)
        if (!entry) throw createAppError(ERROR_CODE.IR_COURSE_NOT_FOUND, { courseHash })
        return NextResponse.json(successResponse(entry))
    })

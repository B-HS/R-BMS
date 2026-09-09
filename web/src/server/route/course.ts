import { compose } from '@server/compose'
import { courseMetaUpsertSchema, courseSubmissionSchema } from '@server/dto/course'
import { bestQuerySchema, rankingQuerySchema } from '@server/dto/score'
import { irRaw } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { parseBearerToken } from '@server/lib/token'
import { withApiToken } from '@server/lib/with-api-token'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { isPlayedAtPlausible } from '@server/service/domain/score/score.service'
import {
    getCourseBestCached,
    getCourseMetaCached,
    getCourseRankingCached,
    revalidateOnCourseSubmit,
    revalidateOnCourseUpsert,
} from '@server/service/read-cache'

type CourseContext = { params: Promise<{ courseHash: string }> }

type OptionalCourseContext = { params?: Promise<{ courseHash?: string }> }

const CREATED_STATUS = 201
const SUPPORTED_API_VERSION = 1

export const resolvePathCourseHash = async (context: OptionalCourseContext) => (context?.params ? ((await context.params).courseHash ?? null) : null)

const submitCourse = () =>
    withErrorHandling(async (request: Request, context: OptionalCourseContext) => {
        if (!parseBearerToken(request)) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        const { authService, courseService } = compose()
        return withApiToken({ resolveApiTokenUser: authService.resolveApiTokenUser })(
            async (authedRequest, authedContext: OptionalCourseContext, user) => {
                const input = await parseJsonBody(authedRequest, courseSubmissionSchema)
                if (input.api_version !== SUPPORTED_API_VERSION)
                    throw createAppError(ERROR_CODE.IR_API_VERSION_UNSUPPORTED, { received: input.api_version })
                if (input.player.id !== user.loginId) throw createAppError(ERROR_CODE.FORBIDDEN, { player: input.player.id })
                const pathCourseHash = await resolvePathCourseHash(authedContext)
                if (pathCourseHash && pathCourseHash !== input.course_hash)
                    throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'course_hash', path: pathCourseHash })
                if (!isPlayedAtPlausible(input.played_at, Date.now())) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'played_at' })
                const result = await courseService.submit({ input, user: { id: user.id, loginId: user.loginId } })
                revalidateOnCourseSubmit({ courseHash: input.course_hash, loginId: user.loginId })
                return irRaw(result.response, result.duplicated ? 200 : CREATED_STATUS)
            },
        )(request, context)
    })

export const createCourseSubmitRoute = submitCourse

export const createCourseScoreSubmitRoute = submitCourse

export const createCourseMetaUpsertRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        if (!parseBearerToken(request)) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        const { authService, courseService } = compose()
        return withApiToken({ resolveApiTokenUser: authService.resolveApiTokenUser })(async (authedRequest) => {
            const input = await parseJsonBody(authedRequest, courseMetaUpsertSchema)
            const meta = await courseService.upsertMeta(input)
            if (!meta) throw createAppError(ERROR_CODE.IR_COURSE_NOT_FOUND, { courseHash: input.course_hash })
            revalidateOnCourseUpsert(input.course_hash)
            return irRaw(meta)
        })(request, context)
    })

export const createCourseDetailRoute = () =>
    withErrorHandling(async (_request: Request, context: CourseContext) => {
        const { courseHash } = await context.params
        const meta = await getCourseMetaCached(courseHash)
        if (!meta) throw createAppError(ERROR_CODE.IR_COURSE_NOT_FOUND, { courseHash })
        return irRaw(meta)
    })

export const createCourseRankingRoute = () =>
    withErrorHandling(async (request: Request, context: CourseContext) => {
        const query = parseSearchParams(request, rankingQuerySchema)
        const { courseHash } = await context.params
        const rows = await getCourseRankingCached({ courseHash, limit: query.limit, page: query.page, rivalOf: query.rival_of })
        return irRaw(rows)
    })

export const createCourseBestRoute = () =>
    withErrorHandling(async (request: Request, context: CourseContext) => {
        const query = parseSearchParams(request, bestQuerySchema)
        const { courseHash } = await context.params
        return irRaw(await getCourseBestCached({ courseHash, loginId: query.player }))
    })

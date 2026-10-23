import { compose, type Composed } from '@server/compose'
import { COURSE_SUBMISSION_MAX_BYTES, courseMetaUpsertSchema, courseSubmissionSchema } from '@server/dto/course'
import { bestQuerySchema, rankingQuerySchema } from '@server/dto/score'
import { irRaw } from '@server/lib/api-response'
import { getEnv } from '@server/lib/env'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { summarizeError } from '@server/lib/error-summary'
import { parseJsonBody, parseLimitedJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { parseBearerToken } from '@server/lib/token'
import { withApiToken } from '@server/lib/with-api-token'
import { withAdmin } from '@server/lib/with-session'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit, getClientIp } from '@server/lib/with-rate-limit'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import { findCourseIntegrityFailure, type CourseService } from '@server/service/domain/course/course.service'
import type { DanService } from '@server/service/domain/dan/dan.service'
import { isPlayedAtPlausible, type ScoreService } from '@server/service/domain/score/score.service'
import { readCourseMeta, readCourseRanking } from '@server/service/public-reads'
import { getCourseBestCached, revalidateOnCourseSubmit, revalidateOnCourseUpsert, revalidateOnDanAward } from '@server/service/read-cache'

export type CourseSubmitRouteDeps = {
    authService: Pick<AuthService, 'resolveApiTokenUser'>
    courseService: Pick<CourseService, 'submit'>
    scoreService: Pick<ScoreService, 'recordRejection' | 'recordAcceptance'>
    resolveBuildTrust: Composed['resolveBuildTrust']
    danService: Pick<DanService, 'awardFromRun'>
    revalidate: { onCourseSubmit: typeof revalidateOnCourseSubmit; onDanAward: typeof revalidateOnDanAward }
}

type CourseContext = { params: Promise<{ courseHash: string }> }

type OptionalCourseContext = { params?: Promise<{ courseHash?: string }> }

const CREATED_STATUS = 201
const SUPPORTED_API_VERSION = 1

export const resolvePathCourseHash = async (context: OptionalCourseContext) => (context?.params ? ((await context.params).courseHash ?? null) : null)

const resolveCourseSubmitDeps = (): CourseSubmitRouteDeps => {
    const { authService, courseService, scoreService, resolveBuildTrust, danService } = compose()
    return {
        authService,
        courseService,
        scoreService,
        resolveBuildTrust,
        danService,
        revalidate: { onCourseSubmit: revalidateOnCourseSubmit, onDanAward: revalidateOnDanAward },
    }
}

const submitCourse = (resolveDeps: () => CourseSubmitRouteDeps = resolveCourseSubmitDeps) =>
    withErrorHandling(async (request: Request, context: OptionalCourseContext) => {
        if (!parseBearerToken(request)) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        const { authService, courseService, scoreService, resolveBuildTrust, danService, revalidate } = resolveDeps()
        return withApiToken({ resolveApiTokenUser: authService.resolveApiTokenUser })(
            async (authedRequest, authedContext: OptionalCourseContext, user) => {
                enforceUserRateLimit('course-submit', user.id, RATE_LIMIT.SCORE_SUBMIT)
                const env = getEnv()
                const auditRejection = () =>
                    scoreService
                        .recordRejection({
                            userId: user.id,
                            clientBuildSha256: null,
                            clientPlatform: null,
                            ip: getClientIp(request),
                            userAgent: request.headers.get('user-agent'),
                            flags: [],
                        })
                        .catch(() => undefined)
                const reject = async (error: ReturnType<typeof createAppError>) => {
                    await auditRejection()
                    throw error
                }

                const input = await parseLimitedJsonBody(authedRequest, courseSubmissionSchema, COURSE_SUBMISSION_MAX_BYTES).catch(
                    async (error: unknown) => {
                        await auditRejection()
                        throw error
                    },
                )
                if (input.api_version !== SUPPORTED_API_VERSION)
                    await reject(createAppError(ERROR_CODE.IR_API_VERSION_UNSUPPORTED, { received: input.api_version }))
                if (input.player.id !== user.loginId) await reject(createAppError(ERROR_CODE.FORBIDDEN, { player: input.player.id }))
                const pathCourseHash = await resolvePathCourseHash(authedContext)
                if (pathCourseHash && pathCourseHash !== input.course_hash)
                    await reject(createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'course_hash', path: pathCourseHash }))
                if (!isPlayedAtPlausible(input.played_at, Date.now()))
                    await reject(createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'played_at' }))
                if (env.REQUIRE_BUILD_HASH && !input.client_build_sha256)
                    await reject(createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'client_build_sha256' }))
                const integrityFailure = findCourseIntegrityFailure(input)
                if (integrityFailure) await reject(createAppError(ERROR_CODE.VALIDATION_ERROR, integrityFailure))

                const result = await courseService.submit({
                    input,
                    user: { id: user.id, loginId: user.loginId },
                    buildTrust: await resolveBuildTrust(input.client_build_sha256 ?? null),
                    requireTrustedBuild: env.REQUIRE_BUILD_HASH,
                })
                if (!result.duplicated) {
                    await scoreService
                        .recordAcceptance({
                            scoreId: result.courseScoreId,
                            userId: user.id,
                            clientBuildSha256: input.client_build_sha256 ?? null,
                            clientPlatform: input.client_platform ?? null,
                            ip: getClientIp(request),
                            userAgent: request.headers.get('user-agent'),
                            ranked: result.ranked,
                            flags: result.flags,
                        })
                        .catch(() => undefined)
                }
                try {
                    const { awarded } = await danService.awardFromRun(result.scoreRow)
                    if (awarded) revalidate.onDanAward({ loginId: user.loginId })
                } catch (error) {
                    console.error('dan award failed', summarizeError(error))
                }
                revalidate.onCourseSubmit({ courseHash: input.course_hash, loginId: user.loginId })
                return irRaw(result.response, result.duplicated ? 200 : CREATED_STATUS)
            },
        )(request, context)
    })

export const createCourseSubmitRoute = submitCourse

export const createCourseScoreSubmitRoute = submitCourse

export const createCourseMetaUpsertRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService, courseService } = compose()
        return withAdmin({ resolveSessionUser: authService.resolveSessionUser })(async (authedRequest) => {
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
        return irRaw(await readCourseMeta(courseHash))
    })

export const createCourseRankingRoute = () =>
    withErrorHandling(async (request: Request, context: CourseContext) => {
        const query = parseSearchParams(request, rankingQuerySchema)
        const { courseHash } = await context.params
        return irRaw(await readCourseRanking(courseHash, query))
    })

export const createCourseBestRoute = () =>
    withErrorHandling(async (request: Request, context: CourseContext) => {
        const query = parseSearchParams(request, bestQuerySchema)
        const { courseHash } = await context.params
        return irRaw(await getCourseBestCached({ courseHash, loginId: query.player, includeScratchAuto: query.include_scratch_auto }))
    })

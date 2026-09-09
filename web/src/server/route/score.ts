import { compose } from '@server/compose'
import { isSha256Hash } from '@server/dto/common'
import { scoreSubmissionSchema } from '@server/dto/score'
import { irRaw } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { getEnv } from '@server/lib/env'
import { guardBodySize, parseLimitedJsonBody } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { resolveOptionalApiTokenUser } from '@server/lib/with-api-token'
import { parseBearerToken } from '@server/lib/token'
import { RATE_LIMIT, consumeRateLimit, getClientIp } from '@server/lib/with-rate-limit'
import { GUEST_PLAYER_ID, isJudgeCountConsistent, isPlayedAtPlausible } from '@server/service/domain/score/score.service'
import { getScoreCached, revalidateOnScoreSubmit } from '@server/service/read-cache'

const SUBMISSION_MAX_BYTES = 256 * 1024
const CREATED_STATUS = 201
const SUPPORTED_API_VERSION = 1

export const createSubmitScoreRoute = () =>
    withErrorHandling(async (request: Request) => {
        guardBodySize(request, SUBMISSION_MAX_BYTES)
        const env = getEnv()
        const bearer = parseBearerToken(request)
        if (!bearer && !env.ALLOW_GUEST) throw createAppError(ERROR_CODE.UNAUTHORIZED)

        const { authService, chartService, scoreService, resolveBuildTrust } = compose()
        const user = await resolveOptionalApiTokenUser(request, authService.resolveApiTokenUser)

        const rateKey = user ? `user:${user.id}` : `ip:${getClientIp(request)}`
        const { allowed } = consumeRateLimit(`score-submit:${rateKey}`, RATE_LIMIT.SCORE_SUBMIT.limit, RATE_LIMIT.SCORE_SUBMIT.windowMs)
        if (!allowed) throw createAppError(ERROR_CODE.RATE_LIMITED)

        const auditRejection = (reason: string) =>
            scoreService
                .recordRejection({
                    userId: user?.id ?? null,
                    clientBuildSha256: null,
                    clientPlatform: null,
                    ip: getClientIp(request),
                    userAgent: request.headers.get('user-agent'),
                    flags: [],
                })
                .catch(() => undefined)
                .then(() => reason)

        const input = await parseLimitedJsonBody(request, scoreSubmissionSchema, SUBMISSION_MAX_BYTES).catch(async (error: unknown) => {
            await auditRejection('validation')
            throw error
        })
        if (input.api_version !== SUPPORTED_API_VERSION) {
            await auditRejection('api-version')
            throw createAppError(ERROR_CODE.IR_API_VERSION_UNSUPPORTED, { received: input.api_version })
        }
        if (user && input.player.id !== user.loginId) {
            await auditRejection('player-mismatch')
            throw createAppError(ERROR_CODE.FORBIDDEN, { player: input.player.id })
        }
        if (!user && input.player.id !== GUEST_PLAYER_ID) {
            await auditRejection('guest-id')
            throw createAppError(ERROR_CODE.UNAUTHORIZED)
        }
        if (!isPlayedAtPlausible(input.played_at, Date.now())) {
            await auditRejection('played-at')
            throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'played_at' })
        }
        if (!isSha256Hash(input.chart.sha256)) {
            await auditRejection('chart-hash')
            throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'chart.sha256' })
        }
        if (!isJudgeCountConsistent(input.judge, input.total_notes)) {
            await auditRejection('judge-total')
            throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'judge', reason: 'judge count exceeds total_notes' })
        }
        if (env.REQUIRE_BUILD_HASH && !input.client_build_sha256) {
            await auditRejection('build-hash')
            throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'client_build_sha256' })
        }

        const resolved = await chartService.ensureFromSubmission({
            md5: input.chart.md5,
            sha256: input.chart.sha256,
            title: '',
            mode: input.mode,
            notes: input.total_notes,
            lntype: input.options.lntype,
        })
        if (!resolved) {
            await auditRejection('chart-unknown')
            throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND)
        }

        const buildTrust = await resolveBuildTrust(input.client_build_sha256 ?? null)
        const result = await scoreService.submit({
            input,
            chartSha256: resolved.sha256,
            chartMd5: resolved.md5,
            user: user ? { id: user.id, loginId: user.loginId, name: user.name } : null,
            buildTrust,
            requireTrustedBuild: env.REQUIRE_BUILD_HASH,
            ip: getClientIp(request),
            userAgent: request.headers.get('user-agent'),
        })

        revalidateOnScoreSubmit({ sha256: resolved.sha256, md5: resolved.md5, loginId: user?.loginId ?? null })

        return irRaw(result.response, result.duplicated ? 200 : CREATED_STATUS)
    })

export const createScoreDetailRoute = () =>
    withErrorHandling(async (_request: Request, context: { params: Promise<{ scoreId: string }> }) => {
        const { scoreId } = await context.params
        const record = await getScoreCached(scoreId)
        if (!record) throw createAppError(ERROR_CODE.IR_SCORE_NOT_FOUND)
        return irRaw(record)
    })

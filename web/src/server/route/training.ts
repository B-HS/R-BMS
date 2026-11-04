import { connection } from 'next/server'
import { compose } from '@server/compose'
import { trainingChartIdSchema } from '@server/dto/training'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, withRateLimit } from '@server/lib/with-rate-limit'
import type { TrainingChartService } from '@server/service/domain/training/training.service'

export type TrainingDownloadRouteDeps = { trainingChartService: Pick<TrainingChartService, 'download'> }

type DownloadContext = { params: Promise<{ id: string }> }

const ZIP_HEADERS = { 'Content-Type': 'application/zip', 'Cache-Control': 'private, no-store' }

/**
 * `GET /api/training/[id]/chart.zip` (plan 2026-11-04 5.1): the zip of a training chart to anyone holding its id, which cannot be
 * guessed. An id not of the id's form is a 404 without a database read, and a deleted chart is a 404 at once (`no-store`). The
 * limit is per IP. `waitForRequest` keeps the handler out of build-time prerendering; tests pass a no-op.
 */
export const createTrainingDownloadRoute = (
    resolveDeps: () => TrainingDownloadRouteDeps = compose,
    waitForRequest: () => Promise<void> = connection,
) => ({
    download: withErrorHandling(
        withRateLimit(
            'training-download',
            RATE_LIMIT.TRAINING_DOWNLOAD,
        )(async (_request: Request, context: DownloadContext) => {
            await waitForRequest()
            const parsed = trainingChartIdSchema.safeParse((await context.params).id)
            if (!parsed.success) throw createAppError(ERROR_CODE.TRAINING_CHART_NOT_FOUND)
            const zip = await resolveDeps().trainingChartService.download(parsed.data)
            if (!zip) throw createAppError(ERROR_CODE.TRAINING_CHART_NOT_FOUND)
            return new Response(zip.bytes, {
                status: 200,
                headers: {
                    ...ZIP_HEADERS,
                    'Content-Disposition': `attachment; filename="${zip.fileName}"`,
                    'Content-Length': String(zip.bytes.byteLength),
                },
            })
        }),
    ),
})

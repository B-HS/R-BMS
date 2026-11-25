import { z } from 'zod'
import { isMd5Hash, isSha256Hash } from '@server/dto/common'

export const CHART_BLOCKLIST_ID_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/
export const CHART_BLOCKLIST_ID_MAX = 64
export const CHART_BLOCKLIST_LABEL_MAX = 200
export const CHART_BLOCKLIST_PATH_MAX = 512
/** The most charts one `PUT /api/admin/chart-blocklist` carries; the script sends a bigger folder in batches. */
export const CHART_BLOCKLIST_BATCH_MAX = 1000
export const CHART_BLOCKLIST_WRITE_MAX_BYTES = 2 * 1024 * 1024
export const CHART_BLOCKLIST_WRITE_RATE_LIMIT = { limit: 120, windowMs: 60_000 } as const

export const chartBlocklistEntrySchema = z.object({
    md5: z.string().refine(isMd5Hash, { message: 'md5 must be 32 lowercase hex characters' }),
    sha256: z.string().refine(isSha256Hash, { message: 'sha256 must be 64 lowercase hex characters' }),
    path: z.string().min(1).max(CHART_BLOCKLIST_PATH_MAX),
})

/** The body of `PUT /api/admin/chart-blocklist`: charts of one source (a game version, a pack) the server will never rank. */
export const chartBlocklistRegisterSchema = z.object({
    source: z.object({
        id: z.string().max(CHART_BLOCKLIST_ID_MAX).regex(CHART_BLOCKLIST_ID_PATTERN),
        label: z.string().trim().min(1).max(CHART_BLOCKLIST_LABEL_MAX),
    }),
    charts: z.array(chartBlocklistEntrySchema).min(1).max(CHART_BLOCKLIST_BATCH_MAX),
})

export type ChartBlocklistEntry = z.infer<typeof chartBlocklistEntrySchema>
export type ChartBlocklistRegisterInput = z.infer<typeof chartBlocklistRegisterSchema>

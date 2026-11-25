import { z } from 'zod'
import { isMd5Hash, isSha256Hash } from '@server/dto/common'

export const CHART_BLOCKLIST_ID_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/

export const chartBlocklistEntrySchema = z.object({
    md5: z.string().refine(isMd5Hash, { message: 'md5 must be 32 lowercase hex characters' }),
    sha256: z.string().refine(isSha256Hash, { message: 'sha256 must be 64 lowercase hex characters' }),
    path: z.string().min(1),
})

/** One committed file of `web/data/chart-blocklist/`: the charts of one source (a game version, a pack) the server never ranks. */
export const chartBlocklistSourceSchema = z.object({
    id: z.string().regex(CHART_BLOCKLIST_ID_PATTERN),
    label: z.string().min(1),
    charts: z.array(chartBlocklistEntrySchema).min(1),
})

export type ChartBlocklistEntry = z.infer<typeof chartBlocklistEntrySchema>
export type ChartBlocklistSource = z.infer<typeof chartBlocklistSourceSchema>

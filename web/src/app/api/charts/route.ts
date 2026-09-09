import { createChartListRoute, createChartUpsertRoute } from '@server/route/chart'

export const GET = createChartListRoute()

export const POST = createChartUpsertRoute()

import { createChartReplayListRoute, createReplayUploadRoute } from '@server/route/replay'

export const GET = createChartReplayListRoute()
export const POST = createReplayUploadRoute()

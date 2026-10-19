import { createTraceRoutes } from '@server/route/trace'

export const maxDuration = 300

export const POST = createTraceRoutes().adminRecompute

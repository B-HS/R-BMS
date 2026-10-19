import { createTraceRoutes } from '@server/route/trace'

const routes = createTraceRoutes()

export const GET = routes.sharingView
export const PUT = routes.sharingPut

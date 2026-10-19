import { createAiRoutes } from '@server/route/ai'

const routes = createAiRoutes()

export const GET = routes.getLimits

export const PUT = routes.putLimits

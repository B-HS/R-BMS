import { createAiRoutes } from '@server/route/ai'

const routes = createAiRoutes()

export const GET = routes.getSettings

export const PUT = routes.putSettings

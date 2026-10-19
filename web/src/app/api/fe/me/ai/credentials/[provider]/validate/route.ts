import { createAiRoutes } from '@server/route/ai'

const routes = createAiRoutes()

export const POST = routes.validateCredential

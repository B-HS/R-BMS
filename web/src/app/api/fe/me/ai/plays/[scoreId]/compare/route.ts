import { createAiPlayFeatureRoutes } from '@server/route/ai'

export const maxDuration = 300

const routes = createAiPlayFeatureRoutes()

export const GET = routes.compareView
export const POST = routes.compareInterpret

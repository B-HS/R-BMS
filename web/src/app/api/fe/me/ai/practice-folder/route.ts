import { createAiScoreFeatureRoutes } from '@server/route/ai'

const routes = createAiScoreFeatureRoutes()

export const GET = routes.practiceView
export const POST = routes.practiceCreate

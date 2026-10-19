import { createAiScoreFeatureRoutes } from '@server/route/ai'

const routes = createAiScoreFeatureRoutes()

export const GET = routes.goalList
export const POST = routes.goalCreate

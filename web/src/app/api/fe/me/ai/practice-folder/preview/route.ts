import { createAiScoreFeatureRoutes } from '@server/route/ai'

export const maxDuration = 300

const routes = createAiScoreFeatureRoutes()

export const POST = routes.practicePreview

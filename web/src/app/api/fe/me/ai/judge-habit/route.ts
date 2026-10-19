import { createAiScoreFeatureRoutes } from '@server/route/ai'

export const maxDuration = 300

const routes = createAiScoreFeatureRoutes()

export const GET = routes.judgeHabitView
export const POST = routes.judgeHabitInterpret

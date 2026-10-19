import { createAiPlayFeatureRoutes } from '@server/route/ai'

export const maxDuration = 300

const routes = createAiPlayFeatureRoutes()

export const GET = routes.gaugeAdviceView
export const POST = routes.gaugeAdviceInterpret

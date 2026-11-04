import { createAiTrainingChartRoutes } from '@server/route/training-charts'

const routes = createAiTrainingChartRoutes()

export const GET = routes.detail
export const DELETE = routes.remove

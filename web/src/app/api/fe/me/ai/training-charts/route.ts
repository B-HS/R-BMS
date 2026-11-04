import { createAiTrainingChartRoutes } from '@server/route/training-charts'

export const maxDuration = 300

const routes = createAiTrainingChartRoutes()

export const GET = routes.list
export const POST = routes.create

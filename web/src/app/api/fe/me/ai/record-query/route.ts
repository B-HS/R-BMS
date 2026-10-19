import { createAiRecordQueryRoutes } from '@server/route/ai'

export const maxDuration = 300

const routes = createAiRecordQueryRoutes()

export const POST = routes.ask

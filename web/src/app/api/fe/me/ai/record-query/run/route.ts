import { createAiRecordQueryRoutes } from '@server/route/ai'

const routes = createAiRecordQueryRoutes()

export const POST = routes.run

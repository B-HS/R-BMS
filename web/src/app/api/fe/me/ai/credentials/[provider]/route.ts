import { createAiRoutes } from '@server/route/ai'

const routes = createAiRoutes()

export const PUT = routes.putCredential

export const PATCH = routes.patchCredential

export const DELETE = routes.deleteCredential

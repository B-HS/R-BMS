import { createFeTokenCreateRoute, createFeTokenListRoute } from '@server/route/fe'

export const GET = createFeTokenListRoute()
export const POST = createFeTokenCreateRoute()

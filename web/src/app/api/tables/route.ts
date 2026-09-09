import { createTableListRoute, createTableUpsertRoute } from '@server/route/table'

export const GET = createTableListRoute()
export const POST = createTableUpsertRoute()

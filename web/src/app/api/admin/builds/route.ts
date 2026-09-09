import { createAdminBuildListRoute, createAdminBuildUpsertRoute } from '@server/route/admin'

export const GET = createAdminBuildListRoute()
export const POST = createAdminBuildUpsertRoute()

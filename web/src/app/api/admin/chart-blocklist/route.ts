import { createChartBlocklistRoutes } from '@server/route/chart-blocklist'

const routes = createChartBlocklistRoutes()

export const PUT = routes.put
export const GET = routes.list

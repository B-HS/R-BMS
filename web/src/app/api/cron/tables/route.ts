import { createTableRefreshCronRoute } from '@server/route/cron'

export const maxDuration = 60

export const GET = createTableRefreshCronRoute()

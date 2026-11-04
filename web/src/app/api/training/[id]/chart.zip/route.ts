import { createTrainingDownloadRoute } from '@server/route/training'

const routes = createTrainingDownloadRoute()

export const GET = routes.download

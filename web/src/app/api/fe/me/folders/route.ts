import { createFolderRoutes } from '@server/route/folder'

const routes = createFolderRoutes()

export const GET = routes.list

export const POST = routes.create

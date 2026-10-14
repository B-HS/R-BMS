import { createFolderRoutes } from '@server/route/folder'

const routes = createFolderRoutes()

export const POST = routes.addItems

export const PUT = routes.replaceItems

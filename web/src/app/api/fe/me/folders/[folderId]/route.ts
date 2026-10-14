import { createFolderRoutes } from '@server/route/folder'

const routes = createFolderRoutes()

export const GET = routes.detail

export const PATCH = routes.update

export const DELETE = routes.remove

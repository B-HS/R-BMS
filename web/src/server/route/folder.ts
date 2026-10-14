import { NextResponse } from 'next/server'
import { compose } from '@server/compose'
import {
    FOLDER_LIMIT,
    folderContainingParamSchema,
    folderCreateSchema,
    folderIdSchema,
    folderImportSchema,
    folderItemNoteSchema,
    folderItemsAddSchema,
    folderItemsOrderSchema,
    folderItemsRemoveSchema,
    folderItemsReplaceSchema,
    folderOrderSchema,
    folderUpdateSchema,
} from '@server/dto/folder'
import { successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseLimitedJsonBody } from '@server/lib/parse-request'
import { withBearerOrSession } from '@server/lib/with-api-token'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit } from '@server/lib/with-rate-limit'
import { withSession } from '@server/lib/with-session'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import type { FolderFailure, FolderResult, FolderService } from '@server/service/domain/folder/folder.service'

export type FolderRouteDeps = {
    authService: Pick<AuthService, 'resolveSessionUser' | 'resolveApiTokenUser'>
    folderService: FolderService
}

type FolderContext = { params: Promise<{ folderId: string }> }
type ItemContext = { params: Promise<{ folderId: string; md5: string }> }
type ContainingContext = { params: Promise<{ md5: string }> }
type PlayerFolderContext = { params: Promise<{ playerId: string; folderId?: string }> }

const CREATED_STATUS = 201
const NO_CONTENT_STATUS = 204
const PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }

const FAILURE_CODE = {
    not_found: ERROR_CODE.IR_FOLDER_NOT_FOUND,
    name_taken: ERROR_CODE.IR_FOLDER_NAME_TAKEN,
    limit_exceeded: ERROR_CODE.IR_FOLDER_LIMIT_EXCEEDED,
    set_mismatch: ERROR_CODE.VALIDATION_ERROR,
    conflict: ERROR_CODE.IR_FOLDER_CONFLICT,
    table_not_found: ERROR_CODE.IR_TABLE_NOT_FOUND,
} as const satisfies Record<FolderFailure['reason'], string>

const privateJson = (body: unknown, status = 200) => NextResponse.json(body, { status, headers: PRIVATE_HEADERS })

const envelope = <T>(data: T, status = 200) => privateJson(successResponse(data), status)

export const unwrapFolderResult = <T>(result: FolderResult<T>) => {
    if (result.ok) return result.value
    const details = 'details' in result ? result.details : { reason: result.reason }
    throw createAppError(FAILURE_CODE[result.reason], details)
}

const parseFolderId = async (context: { params: Promise<{ folderId?: string }> }) => {
    const parsed = folderIdSchema.safeParse((await context.params).folderId)
    if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'folderId' })
    return parsed.data
}

const requireFolder = <T>(value: T | null) => {
    if (value === null) throw createAppError(ERROR_CODE.IR_FOLDER_NOT_FOUND)
    return value
}

export const createFolderRoutes = (resolveDeps: () => FolderRouteDeps = compose) => {
    const withUser = <TContext>(handler: (request: Request, context: TContext, user: AuthUser, deps: FolderRouteDeps) => Promise<Response>) =>
        withErrorHandling(async (request: Request, context: TContext) => {
            const deps = resolveDeps()
            return withSession({ resolveSessionUser: deps.authService.resolveSessionUser })((authedRequest, authedContext: TContext, user) =>
                handler(authedRequest, authedContext, user, deps),
            )(request, context)
        })

    const withWrite = <TContext>(handler: (request: Request, context: TContext, user: AuthUser, deps: FolderRouteDeps) => Promise<Response>) =>
        withUser<TContext>(async (request, context, user, deps) => {
            enforceUserRateLimit('folder-write', user.id, RATE_LIMIT.FOLDER_WRITE)
            return handler(request, context, user, deps)
        })

    const withRead = <TContext>(handler: (request: Request, context: TContext, user: AuthUser, deps: FolderRouteDeps) => Promise<Response>) =>
        withUser<TContext>(async (request, context, user, deps) => {
            enforceUserRateLimit('folder-read', user.id, RATE_LIMIT.FOLDER_READ)
            return handler(request, context, user, deps)
        })

    const withOwnerRead = <TContext extends PlayerFolderContext>(
        handler: (request: Request, context: TContext, user: AuthUser, deps: FolderRouteDeps) => Promise<Response>,
    ) =>
        withErrorHandling(async (request: Request, context: TContext) => {
            const deps = resolveDeps()
            return withBearerOrSession({
                resolveApiTokenUser: deps.authService.resolveApiTokenUser,
                resolveSessionUser: deps.authService.resolveSessionUser,
            })(async (authedRequest, authedContext: TContext, user) => {
                const { playerId } = await authedContext.params
                if (user.loginId !== playerId) throw createAppError(ERROR_CODE.FORBIDDEN, { playerId })
                enforceUserRateLimit('folder-read', user.id, RATE_LIMIT.FOLDER_READ)
                return handler(authedRequest, authedContext, user, deps)
            })(request, context)
        })

    return {
        list: withRead<unknown>(async (_request, _context, user, deps) => envelope(await deps.folderService.list(user.id))),

        create: withWrite<unknown>(async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, folderCreateSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(unwrapFolderResult(await deps.folderService.create({ userId: user.id, input })), CREATED_STATUS)
        }),

        reorder: withWrite<unknown>(async (request, _context, user, deps) => {
            const input = await parseLimitedJsonBody(request, folderOrderSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(unwrapFolderResult(await deps.folderService.reorderFolders({ userId: user.id, folderIds: input.folder_ids })))
        }),

        detail: withRead<FolderContext>(async (_request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            return envelope(requireFolder(await deps.folderService.get({ userId: user.id, folderId })))
        }),

        update: withWrite<FolderContext>(async (request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            const input = await parseLimitedJsonBody(request, folderUpdateSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(unwrapFolderResult(await deps.folderService.update({ userId: user.id, folderId, input })))
        }),

        remove: withWrite<FolderContext>(async (_request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            if (!(await deps.folderService.remove({ userId: user.id, folderId }))) throw createAppError(ERROR_CODE.IR_FOLDER_NOT_FOUND)
            return new NextResponse(null, { status: NO_CONTENT_STATUS, headers: PRIVATE_HEADERS })
        }),

        addItems: withWrite<FolderContext>(async (request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            const input = await parseLimitedJsonBody(request, folderItemsAddSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(unwrapFolderResult(await deps.folderService.addItems({ userId: user.id, folderId, items: input.items })))
        }),

        removeItems: withWrite<FolderContext>(async (request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            const input = await parseLimitedJsonBody(request, folderItemsRemoveSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(unwrapFolderResult(await deps.folderService.removeItems({ userId: user.id, folderId, md5s: input.md5s })))
        }),

        reorderItems: withWrite<FolderContext>(async (request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            const input = await parseLimitedJsonBody(request, folderItemsOrderSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(unwrapFolderResult(await deps.folderService.reorderItems({ userId: user.id, folderId, md5s: input.md5s })))
        }),

        replaceItems: withWrite<FolderContext>(async (request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            const input = await parseLimitedJsonBody(request, folderItemsReplaceSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(unwrapFolderResult(await deps.folderService.replaceItems({ userId: user.id, folderId, input })))
        }),

        updateItemNote: withWrite<ItemContext>(async (request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            const md5 = folderContainingParamSchema.safeParse({ md5: (await context.params).md5 })
            if (!md5.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'md5' })
            const input = await parseLimitedJsonBody(request, folderItemNoteSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(
                unwrapFolderResult(await deps.folderService.updateItemNote({ userId: user.id, folderId, md5: md5.data.md5, note: input.note })),
            )
        }),

        importFromTable: withUser<FolderContext>(async (request, context, user, deps) => {
            enforceUserRateLimit('folder-import', user.id, RATE_LIMIT.FOLDER_IMPORT)
            const folderId = await parseFolderId(context)
            const input = await parseLimitedJsonBody(request, folderImportSchema, FOLDER_LIMIT.WRITE_MAX_BYTES)
            return envelope(
                unwrapFolderResult(
                    await deps.folderService.importFromTable({ userId: user.id, folderId, tableId: input.table_id, levels: input.levels }),
                ),
            )
        }),

        containing: withRead<ContainingContext>(async (_request, context, user, deps) => {
            const parsed = folderContainingParamSchema.safeParse({ md5: (await context.params).md5 })
            if (!parsed.success) throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'md5' })
            return envelope(await deps.folderService.listContaining({ userId: user.id, md5: parsed.data.md5 }))
        }),

        readIndex: withOwnerRead<PlayerFolderContext>(async (_request, _context, user, deps) =>
            privateJson({ folders: await deps.folderService.list(user.id) }),
        ),

        readFolder: withOwnerRead<PlayerFolderContext>(async (_request, context, user, deps) => {
            const folderId = await parseFolderId(context)
            return privateJson(requireFolder(await deps.folderService.get({ userId: user.id, folderId })))
        }),
    }
}

import { isServer, MutationCache, QueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { ApiError, HTTP_ERROR_CODE } from '@shared/lib/fetch'

const STALE_TIME_MS = 60_000
const GC_TIME_MS = 600_000

type MutationErrorMessages = { http: (status: number) => string; unknown: string }

const boundMessages: { current: MutationErrorMessages | null } = { current: null }

/** Hands the toast of a failed mutation the sentences of the language on screen; the returned function unbinds them. */
export const bindMutationErrorMessages = (messages: MutationErrorMessages) => {
    boundMessages.current = messages
    return () => {
        if (boundMessages.current === messages) boundMessages.current = null
    }
}

const toMessage = (error: unknown) => {
    const messages = boundMessages.current
    if (error instanceof ApiError) return error.code === HTTP_ERROR_CODE && messages ? messages.http(error.status) : error.message
    if (error instanceof Error) return error.message
    return messages?.unknown ?? String(error)
}

export const createQueryClient = () =>
    new QueryClient({
        defaultOptions: {
            queries: { staleTime: STALE_TIME_MS, gcTime: GC_TIME_MS, retry: 1, refetchOnWindowFocus: false },
        },
        mutationCache: new MutationCache({
            onError: (error, _variables, _context, mutation) => {
                if (mutation.meta?.suppressErrorToast) return
                toast.error(toMessage(error))
            },
        }),
    })

let browserQueryClient: QueryClient | null = null

export const getQueryClient = () => {
    if (isServer) return createQueryClient()
    if (!browserQueryClient) browserQueryClient = createQueryClient()
    return browserQueryClient
}

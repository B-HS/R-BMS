import { isServer, MutationCache, QueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { ApiError } from '@shared/lib/fetch'

const STALE_TIME_MS = 60_000
const GC_TIME_MS = 600_000

const toMessage = (error: unknown) => {
    if (error instanceof ApiError) return error.message
    if (error instanceof Error) return error.message
    return '요청을 처리하지 못했습니다.'
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

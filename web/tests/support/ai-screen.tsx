import type { ReactNode } from 'react'
import { QueryClientProvider, type QueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { createQueryClient } from '../../src/shared/utils/get-query-client'
import { mount, unmountAll } from './dom-harness'

export type FetchCall = { method: string; path: string; body: unknown }
type FetchHandler = (call: FetchCall) => Response | Promise<Response>

const JSON_HEADERS = { 'content-type': 'application/json' }

const clients: QueryClient[] = []

export const jsonResponse = (data: unknown, status = 200) => new Response(JSON.stringify({ success: true, data }), { status, headers: JSON_HEADERS })

export const errorResponse = (status: number, code: string, message: string, details?: Record<string, unknown>) =>
    new Response(JSON.stringify({ success: false, error: { code, message, ...(details ? { details } : {}) } }), { status, headers: JSON_HEADERS })

export const noContentResponse = () => new Response(null, { status: 204 })

export const installFetchRoutes = (routes: Record<string, FetchHandler>) => {
    const calls: FetchCall[] = []
    const originalFetch = globalThis.fetch
    globalThis.fetch = Object.assign(async (input: RequestInfo | URL, init?: RequestInit) => {
        const method = init?.method ?? 'GET'
        const path = String(input).split('?')[0]
        const call = { method, path, body: typeof init?.body === 'string' ? (JSON.parse(init.body) as unknown) : null }
        calls.push(call)
        const handler = routes[`${method} ${path}`]
        return handler ? handler(call) : errorResponse(404, 'NOT_FOUND', `unrouted ${method} ${path}`)
    }, originalFetch)
    return { calls, restore: () => void (globalThis.fetch = originalFetch) }
}

export const renderWithQueryClient = async (ui: ReactNode) => {
    const queryClient = createQueryClient()
    const defaults = queryClient.getDefaultOptions()
    queryClient.setDefaultOptions({ ...defaults, queries: { ...defaults.queries, retry: false } })
    clients.push(queryClient)
    await mount(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>)
    return { queryClient }
}

export const cleanupScreens = async () => {
    await unmountAll()
    for (const queryClient of clients.splice(0)) queryClient.clear()
}

export const toastTexts = () => toast.getHistory().flatMap((item) => ('title' in item ? [String(item.title)] : []))

'use client'

import { useMutation, useQueryClient, type QueryClient } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useIsHydrated } from '@shared/hooks/use-is-hydrated'
import { authClient } from '@shared/lib/auth-client'
import { ApiError, buildApiUrl, fetchEnvelope } from '@shared/lib/fetch'
import { clientSessionUserSchema, type RegisterInput, type SessionLoginInput, type SessionAuthResult } from '@entities/auth.type'

const jsonInit = (body: unknown) => ({ method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })

const UNAUTHORIZED_STATUS = 401

const notifySessionChanged = () => authClient.$store.notify('$sessionSignal')

const USER_SCOPED_QUERY_KEYS = [
    QUERY_KEY.AUTH.ALL,
    QUERY_KEY.FOLDER.ALL,
    QUERY_KEY.TABLE_SNAPSHOT.RECORDS_ALL,
    QUERY_KEY.TOKEN.ALL,
    QUERY_KEY.SETTING.ALL,
    QUERY_KEY.AI.ALL,
    QUERY_KEY.ADMIN.AI_LIMITS,
]

/** Drops cached queries that belong to the signed-in user so the next account never sees them. */
export const removeUserScopedQueries = (queryClient: QueryClient) => {
    for (const queryKey of USER_SCOPED_QUERY_KEYS) queryClient.removeQueries({ queryKey })
}

/** Signs in with the player id on the session-only web route, so the web never mints a native API token. */
export const useSessionLogin = () => {
    const queryClient = useQueryClient()
    const t = useTranslations('pages.auth')
    return useMutation({
        mutationFn: async (input: SessionLoginInput) => {
            try {
                const result = await fetchEnvelope<SessionAuthResult>(buildApiUrl('/fe/auth/login'), jsonInit(input))
                removeUserScopedQueries(queryClient)
                notifySessionChanged()
                return result.user
            } catch (error) {
                if (error instanceof ApiError && error.status === UNAUTHORIZED_STATUS) throw new ApiError(error.code, t('signInFailed'), error.status)
                throw error
            }
        },
        meta: { suppressErrorToast: true },
    })
}

export const useSessionLogout = () => {
    const queryClient = useQueryClient()
    const t = useTranslations('pages.auth')
    return useMutation({
        mutationFn: async () => {
            const result = await authClient.signOut()
            if (result.error) throw new Error(t('signOutFailed'))
        },
        onSuccess: () => removeUserScopedQueries(queryClient),
    })
}

export const useRegister = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: async (input: RegisterInput) => {
            const result = await fetchEnvelope<SessionAuthResult>(buildApiUrl('/fe/auth/register'), jsonInit(input))
            removeUserScopedQueries(queryClient)
            notifySessionChanged()
            return result.user
        },
        meta: { suppressErrorToast: true },
    })
}

/**
 * Reads the signed-in user from the better-auth session store and narrows it to the shell's user shape.
 * Reports a pending session until hydration ends, because the store hands its live value to the server snapshot as well.
 */
export const useSessionUser = () => {
    const { data, isPending } = authClient.useSession()
    const isHydrated = useIsHydrated()
    if (!isHydrated) return { user: null, isPending: true }
    const parsed = data?.user ? clientSessionUserSchema.safeParse(data.user) : null
    return { user: parsed?.success ? parsed.data : null, isPending }
}

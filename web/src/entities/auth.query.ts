'use client'

import { useMutation } from '@tanstack/react-query'
import { authClient } from '@shared/lib/auth-client'
import { ApiError, buildApiUrl, fetchEnvelope } from '@shared/lib/fetch'
import { clientSessionUserSchema, type RegisterInput, type SessionLoginInput, type SessionAuthResult } from '@entities/auth.type'

const jsonInit = (body: unknown) => ({ method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })

const SIGN_IN_FAILED_MESSAGE = '아이디 또는 비밀번호가 올바르지 않습니다.'
const UNAUTHORIZED_STATUS = 401

const notifySessionChanged = () => authClient.$store.notify('$sessionSignal')

/** Signs in with the player id on the session-only web route, so the web never mints a native API token. */
export const useSessionLogin = () =>
    useMutation({
        mutationFn: async (input: SessionLoginInput) => {
            try {
                const result = await fetchEnvelope<SessionAuthResult>(buildApiUrl('/fe/auth/login'), jsonInit(input))
                notifySessionChanged()
                return result.user
            } catch (error) {
                if (error instanceof ApiError && error.status === UNAUTHORIZED_STATUS)
                    throw new ApiError(error.code, SIGN_IN_FAILED_MESSAGE, error.status)
                throw error
            }
        },
        meta: { suppressErrorToast: true },
    })

export const useSessionLogout = () =>
    useMutation({
        mutationFn: async () => {
            const result = await authClient.signOut()
            if (result.error) throw new Error('로그아웃하지 못했습니다. 잠시 후 다시 시도하세요.')
        },
    })

export const useRegister = () =>
    useMutation({
        mutationFn: async (input: RegisterInput) => {
            const result = await fetchEnvelope<SessionAuthResult>(buildApiUrl('/fe/auth/register'), jsonInit(input))
            notifySessionChanged()
            return result.user
        },
        meta: { suppressErrorToast: true },
    })

/** Reads the signed-in user from the better-auth session store and narrows it to the shell's user shape. */
export const useSessionUser = () => {
    const { data, isPending } = authClient.useSession()
    const parsed = data?.user ? clientSessionUserSchema.safeParse(data.user) : null
    return { user: parsed?.success ? parsed.data : null, isPending }
}

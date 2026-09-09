import { useMutation } from '@tanstack/react-query'
import { buildApiUrl, fetchRaw } from '@shared/lib/fetch'
import type { AuthResponse, LoginInput, RegisterInput } from '@entities/auth.type'

const jsonInit = (body: unknown) => ({ method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })

export const useLogin = () => useMutation({ mutationFn: (input: LoginInput) => fetchRaw<AuthResponse>(buildApiUrl('/auth/login'), jsonInit(input)) })

export const useRegister = () =>
    useMutation({ mutationFn: (input: RegisterInput) => fetchRaw<AuthResponse>(buildApiUrl('/auth/register'), jsonInit(input)) })

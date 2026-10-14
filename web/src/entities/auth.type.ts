import { z } from 'zod'

export type SessionUser = {
    id: string
    login_id: string
    name: string
    email: string
    role: string
}

const DEFAULT_ROLE = 'user'

/** Narrows the user object better-auth returns into the fields the web shell reads. */
export const clientSessionUserSchema = z
    .object({
        id: z.string(),
        name: z.string().nullish(),
        email: z.string(),
        loginId: z.string().nullish(),
        role: z.string().nullish(),
    })
    .transform((user) => ({
        id: user.id,
        login_id: user.loginId ?? user.id,
        name: user.name ?? user.loginId ?? user.email,
        email: user.email,
        role: user.role ?? DEFAULT_ROLE,
    }))

export type SessionLoginInput = {
    id: string
    password: string
}

export type SessionAuthResult = {
    user: Pick<SessionUser, 'id' | 'login_id' | 'name' | 'role'>
}

export type RegisterInput = {
    id: string
    password: string
    email: string
    name: string
}

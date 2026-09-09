export type AuthUser = {
    id: string
    loginId: string
    name: string
    role: string
}

export type ResolveApiTokenUser = (plainToken: string) => Promise<AuthUser | null>

export type ResolveSessionUser = (request: Request) => Promise<AuthUser | null>

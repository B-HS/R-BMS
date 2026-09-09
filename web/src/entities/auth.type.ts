export type SessionUser = {
    id: string
    login_id: string
    name: string
    email: string
    role: string
}

export type AuthResponse = {
    token: string
    player: { id: string }
    name: string
}

export type LoginInput = {
    id: string
    password: string
}

export type RegisterInput = LoginInput & {
    email: string
    name: string
}

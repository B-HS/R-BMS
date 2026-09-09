export type ApiTokenRow = {
    id: string
    label: string
    created_at: number
    last_used_at: number | null
}

export type IssuedApiToken = {
    token: string
    created_at: number
}

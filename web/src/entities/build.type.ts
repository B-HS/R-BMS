export type ClientBuild = {
    sha256: string
    version: string
    platform: string
    channel: string
    released_at: number
    trusted: boolean
    note: string
}

export type ClientBuildInput = Omit<ClientBuild, 'released_at' | 'note'> & Partial<Pick<ClientBuild, 'released_at' | 'note'>>

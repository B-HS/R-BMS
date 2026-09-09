export type ReplayPayload = {
    storageKey: string | null
    inline: string | null
}

export type ReplayStorage = {
    kind: string
    save: (params: { id: string; content: string }) => Promise<ReplayPayload>
    load: (payload: ReplayPayload) => Promise<string | null>
}

export const createMysqlReplayStorage = (): ReplayStorage => ({
    kind: 'db',
    save: async ({ content }) => ({ storageKey: null, inline: content }),
    load: async (payload) => payload.inline,
})
